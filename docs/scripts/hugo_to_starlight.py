#!/usr/bin/env python3
"""Move the Hugo (geekdoc) pages of the Cloacina docs into Starlight.

    python3 docs/scripts/hugo_to_starlight.py <hugo content dir> <starlight docs dir>

Reads the Hugo pages (`docs/content` at the commit before CLOACINA-T-0951) and
writes the Starlight pages (`docs/src/content/docs`), with:

- front matter: `title`, `description`; `weight` -> `sidebar.order`;
  `aliases` -> the redirect map; other keys kept;
- each page at its Diataxis route (ROUTES below);
- the shortcodes converted (ref, relref, hint, tabs/tab, toc-tree, mermaid,
  button, columns, api-link) and site links made relative;
- `src/redirects.json`: each old address (page and alias) -> its new route.

The API pages (`api-reference/`) are not converted: plissken writes them into
`reference/api/`. Run `plissken render` first, so the links to them resolve.

A one-time tool of the move; it can be deleted later.
"""

import json
import os
import posixpath
import re
import sys
from collections import Counter
from pathlib import Path

COUNTS = Counter()
LEFT = []  # shortcodes not converted: (page, text)
FIXED = []  # links that were broken in Hugo and now resolve: (page, old, new)

# --------------------------------------------------------------------------
# Routes: old Hugo key -> new Starlight key. A key is the content path with no
# extension; a section page (_index.md) has the key of its folder.

PREFIX_ROUTES = [
    # (old prefix, new prefix), first match wins; matched on whole segments.
    ("start/install", "how-to/install"),
    ("embed/quick-start", "tutorials/embed/quick-start"),
    ("embed/tutorials", "tutorials/embed"),
    ("embed/how-to", "how-to/embed"),
    ("embed/explanation", "explanation/embed"),
    ("service/quick-start", "tutorials/service/quick-start"),
    ("service/embedded-ui", "how-to/service/embedded-ui"),
    ("service/tutorials", "tutorials/service"),
    ("service/how-to", "how-to/service"),
    ("service/explanation", "explanation/service"),
    ("engine/explanation", "explanation/engine"),
    ("engine/workflows/how-to", "how-to/workflows"),
    ("engine/computation-graphs/how-to", "how-to/computation-graphs"),
    ("engine/constructors/author-a-provider", "how-to/constructors/author-a-provider"),
    ("engine/constructors/consume-a-provider", "how-to/constructors/consume-a-provider"),
    ("engine/workflows", "explanation/primitives/workflows"),
    ("engine/computation-graphs", "explanation/primitives/computation-graphs"),
    ("engine/scheduling", "explanation/primitives/scheduling"),
    ("engine/packaging", "explanation/primitives/packaging"),
    ("engine/constructors", "explanation/primitives/constructors"),
    ("start/what-is-cloacina", "explanation/what-is-cloacina"),
    ("start/is-cloacina-for-you", "explanation/is-cloacina-for-you"),
    ("start/concepts", "explanation/concepts"),
    ("start/features", "explanation/features"),
    ("api-reference", "reference/api"),
]
# Kept where they are: "" (home), start, embed, service, engine (the four
# door pages), reference/**, contributing/**.


def route(old):
    for a, b in PREFIX_ROUTES:
        if old == a or old.startswith(a + "/"):
            return b + old[len(a):]
    return old


# --------------------------------------------------------------------------
# The API pages that plissken wrote.

API_PAGES = set()
ANCHORS = {}  # anchor id -> the API pages that have it


def load_api(docs):
    api = docs / "reference" / "api"
    for p in api.rglob("*.md"):
        rel = p.relative_to(docs).with_suffix("").as_posix()
        if rel.endswith("/index"):
            rel = rel[: -len("/index")]
        API_PAGES.add(rel)
        for a in re.findall(r'id="([^"]+)"', p.read_text()):
            ANCHORS.setdefault(a, []).append(rel)
    API_PAGES.add("reference/api")  # reference/api.md, the index page


def api_route(old):
    """An old api-reference key -> the nearest API page plissken wrote."""
    rest = old[len("api-reference"):].lstrip("/")
    if rest.startswith("cloaca/python"):
        rest = "cloacina-python" + rest[len("cloaca/python"):]
    key = posixpath.join("reference/api", rest) if rest else "reference/api"
    while key not in API_PAGES:
        key = posixpath.dirname(key)
    return key


# --------------------------------------------------------------------------
# Hand fixes: links that were broken in Hugo too. (page key -> [(old, new)])

FIXUPS = {
    # The heading is "Failure Scenarios and Recovery".
    "service/explanation/horizontal-scaling": [("(#failure-scenarios)", "(#failure-scenarios-and-recovery)")],
}


# --------------------------------------------------------------------------
# Hugo pages


class Page:
    def __init__(self, path, root):
        self.path = path
        rel = path.relative_to(root).as_posix()
        self.is_section = path.name == "_index.md"
        if self.is_section:
            self.key = posixpath.dirname(rel) if "/" in rel else ""
        else:
            self.key = rel[: -len(".md")]
        self.parent = posixpath.dirname(self.key) if self.key else None
        text = path.read_text()
        self.fm, self.body = split_front_matter(text)
        for a, b in FIXUPS.get(self.key, []):
            assert a in self.body, (self.key, a)
            self.body = self.body.replace(a, b)
            FIXED.append((self.key, a, b))
            COUNTS["hand fix"] += 1
        self.title = self.fm.get("title", "")
        self.weight = self.fm.get("weight")
        self.new = route(self.key)
        self.mdx = False

    @property
    def url(self):
        return self.key + "/" if self.key else ""

    @property
    def new_url(self):
        return self.new + "/" if self.new else ""

    @property
    def dir_key(self):
        """The folder that a relative ref of this page starts from."""
        return self.key if self.is_section else posixpath.dirname(self.key)


def unquote(v):
    v = v.strip()
    if len(v) >= 2 and v[0] == v[-1] and v[0] in "\"'":
        inner = v[1:-1]
        if v[0] == '"':
            inner = inner.replace('\\"', '"').replace("\\\\", "\\")
        return inner
    return v


def split_front_matter(text):
    """YAML front matter as an ordered dict of raw values; aliases as a list."""
    if not text.startswith("---\n"):
        return {}, text
    end = text.index("\n---", 4)
    block, body = text[4:end], text[end + 4:]
    body = body[1:] if body.startswith("\n") else body
    fm, key = {}, None
    for line in block.splitlines():
        m = re.match(r"^([A-Za-z_]+):\s*(.*)$", line)
        if m:
            key, val = m.group(1), m.group(2)
            fm[key] = [] if key == "aliases" and not val else (
                int(val) if key == "weight" else (val if key not in ("title", "description") else unquote(val))
            )
        elif re.match(r"^\s+-\s+", line) and key == "aliases":
            fm[key].append(unquote(re.sub(r"^\s+-\s+", "", line)))
        elif line.strip():
            raise SystemExit(f"front matter line not read: {line!r}")
    return fm, body


def yaml_str(s):
    return json.dumps(s, ensure_ascii=False)


# --------------------------------------------------------------------------
# Links


class Site:
    def __init__(self, pages):
        self.pages = pages
        self.by_key = {p.key: p for p in pages}

    def lookup(self, key):
        """A Hugo key (also of an api-reference page) -> new route, or None."""
        if key == "api-reference" or key.startswith("api-reference/"):
            return api_route(key)
        if key in self.by_key:
            return self.by_key[key].new
        return None

    def resolve_ref(self, target, page):
        """A Hugo ref/relref target -> new route and anchor, or None."""
        path, _, anchor = target.partition("#")
        if not path:
            return page.new, anchor
        path = path.strip()
        path = re.sub(r"\.md$", "", path).rstrip("/")
        path = re.sub(r"(^|/)_index$", "", path)
        if path.startswith("/"):
            key = posixpath.normpath(path[1:]) if path != "/" else ""
            key = "" if key == "." else key
            found = self.lookup(key)
        else:
            key = posixpath.normpath(posixpath.join(page.dir_key, path))
            found = self.lookup(key)
            if found is None:
                hits = [p for p in self.pages if p.key == path or p.key.endswith("/" + path)]
                if len(hits) == 1:
                    found = hits[0].new
        if found is None:
            return None
        return found, anchor


def rel_link(from_url, to_route, anchor=""):
    """A relative link from a page at from_url (".../") to a route."""
    to_url = to_route + "/" if to_route else ""
    rel = "" if to_url == from_url else posixpath.relpath("/" + to_url, "/" + from_url) + "/"
    if rel == "./":
        rel = ""
    if anchor:
        return rel + "#" + anchor
    return rel or "./"


def rel_file(from_url, path):
    """A relative link from a page at from_url to a file of the site."""
    return posixpath.relpath("/" + path, "/" + from_url)


STATIC = set()


def rewrite_site_link(target, page, site):
    """A plain link of a Hugo page -> its new form, or None to keep it."""
    if re.match(r"^[a-z][a-z0-9+.-]*:", target, re.I) or target.startswith(("#", "//", "{{")):
        return None
    path, _, anchor = target.partition("#")
    if not path:
        return None
    if path.startswith("/cloacina/"):  # a link written with the base path
        path = path[len("/cloacina"):]
    if path.endswith(".md") and not path.startswith("/"):
        # A link to a source file: Hugo left it broken. Read it as a ref.
        r = site.resolve_ref(path, page)
        if r:
            new = rel_link(page.new_url, r[0], anchor)
            FIXED.append((page.key, target, new))
            COUNTS["link .md -> page"] += 1
            return new
    absolute = posixpath.normpath(posixpath.join("/" + page.url, path))
    key = absolute.strip("/")
    found = site.lookup(key)
    if found is not None:
        COUNTS["plain link -> page"] += 1
        return rel_link(page.new_url, found, anchor)
    if key in STATIC:
        COUNTS["plain link -> file"] += 1
        return rel_file(page.new_url, key) + ("#" + anchor if anchor else "")
    for alt in static_alternatives(key):
        FIXED.append((page.key, target, alt))
        COUNTS["plain link -> file (name fixed)"] += 1
        return rel_file(page.new_url, alt)
    LEFT.append((page.key, f"link not resolved: {target}"))
    return None


# Two images whose links had the right name but the files a misspelled one.
STATIC_ALIASES = {
    "pipeline-performance.png": "pipeline-performnace.png",
    "parallel-performance.png": "parallel-performnance.png",
}


def static_alternatives(key):
    if key in STATIC_ALIASES and STATIC_ALIASES[key] in STATIC:
        yield STATIC_ALIASES[key]


# --------------------------------------------------------------------------
# Body conversion

FENCE = re.compile(r"^(\s*)(`{3,}|~{3,})")


def segments(body):
    """Split into (is_code, text) parts on fenced code blocks."""
    out, buf, code, fence = [], [], False, None
    for line in body.splitlines(keepends=True):
        m = FENCE.match(line)
        if not code and m:
            if buf:
                out.append((False, "".join(buf)))
            buf, code, fence = [line], True, m.group(2)
        elif code and m and m.group(2)[0] == fence[0] and len(m.group(2)) >= len(fence) and line.strip() == m.group(2):
            buf.append(line)
            out.append((True, "".join(buf)))
            buf, code, fence = [], False, None
        else:
            buf.append(line)
    if buf:
        out.append((code, "".join(buf)))
    return out


ATTR = re.compile(r'(\w+)=(?:"((?:[^"\\]|\\.)*)"|(\S+))')

HINT_KIND = {
    "note": "note", "info": "note",
    "tip": "tip", "ok": "tip",
    "important": "caution", "warning": "caution",
    "caution": "danger", "danger": "danger", "error": "danger",
}


def sc_args(s):
    """Named and positional arguments of a shortcode."""
    named = {m.group(1): (m.group(2).replace('\\"', '"') if m.group(2) is not None else m.group(3)) for m in ATTR.finditer(s)}
    rest = ATTR.sub("", s)
    positional = [unquote(x) for x in re.findall(r'"(?:[^"\\]|\\.)*"|\S+', rest)]
    return named, positional


def convert_hint(m):
    named, pos = sc_args(m.group(1))
    # geekdoc reads only the named "type": "{{< hint info >}}" showed a "Note".
    typ = named.get("type") or "note"
    if pos:
        COUNTS["hint with a positional type (shown as Note, as Hugo did)"] += 1
    title = named.get("title") or typ.title()
    COUNTS["hint"] += 1
    return f":::{HINT_KIND.get(typ.lower(), 'note')}[{title}]"


def toc_tree(page, site, depth=0):
    children = [p for p in site.pages if p.parent == page.key and p is not page and p.key != "" and not p.key.startswith("api-reference")]
    if page.key == "" or page.key.startswith("api-reference"):
        return ""
    children.sort(key=lambda p: (p.weight if p.weight is not None else 0, p.title))
    lines = []
    for c in children:
        lines.append("  " * depth + f"- [{c.title}]({{TOC:{c.new}}})")
        if c.is_section:
            sub = toc_tree(c, site, depth + 1)
            if sub:
                lines.append(sub)
    return "\n".join(lines)


def api_link(m, page):
    named, _ = sc_args(m.group(1))
    path = named["path"]
    typ = named.get("type", "struct")
    display = named.get("display", path)
    parts = path.split("::")
    crate = parts[0].replace("_", "-")
    if typ == "module":
        mod, anchor = parts, ""
    else:
        mod, item = parts[:-1], parts[-1]
        anchor = {"fn": f"fn.{item}", "enum": f"enum.{item}", "struct": f"struct.{item}"}.get(typ, "")
        if typ == "fn" and len(mod) >= 2 and mod[-1][:1].isupper():
            # A method: Type::method, on the page of the module of Type.
            anchor = f"{mod[-1]}.{item}"
            mod = mod[:-1]
    key = posixpath.join("reference/api/rust", crate, *mod[1:])
    while key not in API_PAGES:
        key = posixpath.dirname(key)
    if anchor and key not in ANCHORS.get(anchor, []):
        # A re-export: the item is on the page of the module that defines it.
        # Take the page in the same crate (then any crate) with the shortest path.
        pages = ANCHORS.get(anchor, [])
        same = [k for k in pages if k.startswith(f"reference/api/rust/{crate}/")]
        found = sorted(same or pages, key=len)
        if found:
            key = found[0]
            COUNTS["api-link item found on another page"] += 1
        else:
            anchor = ""
    COUNTS["api-link"] += 1
    return f"[`{display}`]({rel_link(page.new_url, key, anchor)})"


def convert_text(text, page, site):
    """Shortcodes and links of one part of a page that is not code."""
    # Plain links first, while the ref shortcodes are still shortcodes.
    def plain(m):
        new = rewrite_site_link(m.group(2), page, site)
        return m.group(0) if new is None else f"{m.group(1)}{new}{m.group(3)}"

    # An image of public/: an <img> with a relative src. (Astro reads a
    # Markdown image as a file next to the page, and does not add the base
    # path to an absolute one.)
    def image(m):
        new = rewrite_site_link(m.group(2), page, site)
        if new is None:
            return m.group(0)
        COUNTS["image -> <img>"] += 1
        alt = " ".join(m.group(1).split()).replace('"', "&quot;")
        return f'<img src="{new}" alt="{alt}" />'

    text = re.sub(r'((?:href|src)=")([^"]+)(")', plain, text)
    text = re.sub(r"!\[([^\]]*)\]\(\s*([^)\s]+)\s*\)", image, text)
    text = re.sub(r"(\]\(\s*)([^)\s]+)((?:\s+\"[^\"]*\")?\s*\))", plain, text)
    text = re.sub(r"^(\[[^\]]+\]:\s*)(\S+)()", plain, text, flags=re.M)

    def ref(m):
        r = site.resolve_ref(m.group(2), page)
        COUNTS[m.group(1)] += 1
        if r is None:
            LEFT.append((page.key, m.group(0)))
            return m.group(0)
        return rel_link(page.new_url, *r)

    text = re.sub(r'\{\{<\s*(ref|relref)\s+"([^"]*)"\s*>\}\}', ref, text)

    text = re.sub(r"\{\{<\s*hint\s*(.*?)\s*>\}\}", convert_hint, text)
    text = re.sub(r"\{\{<\s*/hint\s*>\}\}", lambda m: ":::", text)
    # A Hugo "notice" written in escaped form (shown as text): the aside it meant.
    if re.search(r"\{\{%/\*\s*notice", text):
        text = re.sub(r"\{\{%/\*\s*notice\s+(\w+)\s*\*/%\}\}", lambda m: f":::{HINT_KIND.get(m.group(1), 'note')}[{m.group(1).title()}]", text)
        text = re.sub(r"\{\{%/\*\s*/notice\s*\*/%\}\}", ":::", text)
        COUNTS["notice (escaped) -> aside"] += 1

    def tabs(m):
        COUNTS["tabs"] += 1
        page.mdx = True
        return "<Tabs>"

    text = re.sub(r'\{\{<\s*tabs\s*(?:"[^"]*")?\s*>\}\}', tabs, text)
    text = re.sub(r"\{\{<\s*/tabs\s*>\}\}", "</Tabs>", text)

    def tab(m):
        COUNTS["tab"] += 1
        return f"<TabItem label={yaml_str(m.group(1))}>"

    text = re.sub(r'\{\{<\s*tab\s+"([^"]*)"\s*>\}\}', tab, text)
    text = re.sub(r"\{\{<\s*/tab\s*>\}\}", "</TabItem>", text)

    def toc(m):
        COUNTS["toc-tree"] += 1
        return toc_tree(page, site)

    text = re.sub(r"\{\{<\s*toc-tree\s*>\}\}", toc, text)
    text = re.sub(r"\{TOC:([^}]*)\}", lambda m: rel_link(page.new_url, m.group(1)), text)

    def mermaid(m):
        COUNTS["mermaid"] += 1
        return "```mermaid\n" + m.group(1).strip("\n") + "\n```"

    text = re.sub(r"\{\{<\s*mermaid\s*>\}\}\n?(.*?)\{\{<\s*/mermaid\s*>\}\}", mermaid, text, flags=re.S)

    def button(m):
        named, _ = sc_args(m.group(1))
        target = named.get("relref") or named.get("ref")
        if target:
            r = site.resolve_ref(target, page)
            href = rel_link(page.new_url, *r)
        else:
            href = named.get("href")
        COUNTS["button"] += 1
        page.mdx = True
        return f'<LinkButton href={yaml_str(href)}>{m.group(2).strip()}</LinkButton>'

    text = re.sub(r"\{\{<\s*button\s+(.*?)\s*>\}\}(.*?)\{\{<\s*/button\s*>\}\}", button, text, flags=re.S)

    def columns(m):
        COUNTS["columns"] += 1
        return ""

    text = re.sub(r"\{\{<\s*columns\s*>\}\}\n?", columns, text)
    text = re.sub(r"\{\{<\s*/columns\s*>\}\}\n?", "", text)
    text = re.sub(r"^<--->\n", "", text, flags=re.M)

    text = re.sub(r"\{\{<\s*api-link\s+(.*?)\s*>\}\}", lambda m: api_link(m, page), text)

    for left in re.findall(r"\{\{[<%](?!/\*).*?[>%]\}\}", text):
        LEFT.append((page.key, left))
    # Escaped shortcodes are shown as text: unescape them.
    n = len(re.findall(r"\{\{<\s*/\*", text))
    COUNTS["escaped shortcode shown as text"] += n
    text = re.sub(r"\{\{<\s*/\*\s*(.*?)\s*\*/\s*>\}\}", r"{{< \1 >}}", text)
    return text


def convert_code(text):
    """Code blocks: show escaped shortcodes as Hugo showed them. A rustdoc
    fence ("rust,ignore") gets the language "rust"."""
    text = re.sub(r"^(\s*(?:`{3,}|~{3,}))rust,[\w,]+", r"\1rust", text, count=1)
    return re.sub(r"\{\{<\s*/\*\s*(.*?)\s*\*/\s*>\}\}", r"{{< \1 >}}", text)


# --------------------------------------------------------------------------
# MDX

HTML_TAGS = "details|summary|br|img|a|div|span|sup|sub|kbd|table|thead|tbody|tr|td|th|p|em|strong|code|pre|b|i|ul|ol|li|hr"
JSX = re.compile(r"^</?(Tabs|TabItem|LinkButton)\b")


def escape_mdx(text):
    """Escape a part of a page that is not code, for MDX."""
    out = []
    for line in text.splitlines(keepends=True):
        stripped = line.strip()
        if JSX.match(stripped) or stripped.startswith("<LinkButton"):
            out.append(line)
            continue
        parts = re.split(r"(`+[^`\n]*?`+)", line)
        for i, p in enumerate(parts):
            if i % 2 == 1:
                continue
            p = re.sub(r"<!--(.*?)-->", r"{/*\1*/}", p)
            p = re.sub(r"<(https?://[^>\s]+)>", r"[\1](\1)", p)
            p = re.sub(r"\{(?!/\*)", r"\\{", p)
            p = re.sub(r"(?<!\*/)\}", r"\\}", p)
            p = re.sub(rf"<(?!/?(?:{HTML_TAGS})\b)", "&lt;", p)
            p = re.sub(r"<br\s*>", "<br />", p)
            parts[i] = p
        out.append("".join(parts))
    return "".join(out)


# --------------------------------------------------------------------------


def front_matter(page, title, label):
    lines = ["---", f"title: {yaml_str(title)}"]
    if "description" in page.fm:
        lines.append(f"description: {yaml_str(page.fm['description'])}")
    side = []
    if page.weight is not None:
        side.append(f"  order: {page.weight}")
    if label:
        side.append(f"  label: {yaml_str(label)}")
    if side:
        lines += ["sidebar:"] + side
    for k, v in page.fm.items():
        if k in ("title", "description", "weight", "aliases"):
            continue
        lines.append(f"{k}: {v}")
    lines.append("---")
    return "\n".join(lines) + "\n"


def convert_page(page, site):
    body = page.body
    title, label = page.title, None
    # Starlight shows the title as the heading: drop a first "# heading".
    m = re.match(r"^\s*# (.+?)\s*\n", body)
    if m:
        body = body[m.end():]
        if m.group(1) != page.title:
            title, label = m.group(1), page.title
        COUNTS["first heading -> title"] += 1
    parts = []
    for is_code, text in segments(body):
        parts.append((is_code, convert_code(text) if is_code else convert_text(text, page, site)))
    if page.mdx:
        parts = [(c, t if c else escape_mdx(t)) for c, t in parts]
    out = "".join(t for _, t in parts).lstrip("\n")
    imports = []
    if "<Tabs>" in out:
        imports.append("Tabs, TabItem")
    if "<LinkButton" in out:
        imports.append("LinkButton")
    head = front_matter(page, title, label)
    if imports:
        head += "\nimport { " + ", ".join(imports) + " } from '@astrojs/starlight/components';\n"
    return head + "\n" + out


def main(src, dst):
    src, dst = Path(src), Path(dst)
    for p in (dst.parent.parent.parent / "public").rglob("*"):
        if p.is_file():
            STATIC.add(p.relative_to(dst.parent.parent.parent / "public").as_posix())
    load_api(dst)
    pages = [Page(p, src) for p in sorted(src.rglob("*.md"))]
    site = Site(pages)
    redirects = {}
    written = {}
    for page in pages:
        if page.key == "api-reference" or page.key.startswith("api-reference/"):
            new = api_route(page.key)
            redirects[page.url] = new + "/"
            continue
        text = convert_page(page, site)
        ext = ".mdx" if page.mdx else ".md"
        if page.key == "":
            out = dst / ("index" + ext)
        elif page.is_section:
            out = dst / page.new / ("index" + ext)
        else:
            out = dst / (page.new + ext)
        if page.new in written:
            raise SystemExit(f"two pages at {page.new}: {written[page.new]} and {page.key}")
        written[page.new] = page.key
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(text)
        COUNTS["pages written"] += 1
        COUNTS["pages .mdx" if page.mdx else "pages .md"] += 1
        if page.url != page.new_url:
            redirects[page.url] = page.new_url
        for alias in page.fm.get("aliases", []):
            a = alias.strip("/") + "/"
            redirects[a] = page.new_url
            COUNTS["aliases"] += 1
    (dst.parent.parent / "redirects.json").write_text(json.dumps(dict(sorted(redirects.items())), indent=1) + "\n")
    COUNTS["redirects"] = len(redirects)
    print(json.dumps(dict(sorted(COUNTS.items())), indent=1))
    for k, old, new in FIXED:
        print(f"fixed  {k}: {old} -> {new}")
    for k, t in LEFT:
        print(f"LEFT   {k}: {t}")
    return 1 if LEFT else 0


if __name__ == "__main__":
    sys.exit(main(*sys.argv[1:]))
