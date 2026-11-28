#!/usr/bin/env python3
"""The release gate (CLOACINA-T-1089).

Compares the head with the last release tag and lists each change that the
compatibility promise names:

- the HTTP API (`docs/public/openapi.json`): a path, method, parameter,
  response status, field or enum value removed; a request field made
  required; a type changed;
- the chart values (`charts/*/values.yaml`): a key removed or renamed, a
  default changed; and a render of the head chart with the values file of
  the release;
- the metrics catalog: a metric name or label removed, a kind changed; and
  the code against the catalog (head only);
- the version constants: plugin interface, agent protocol, delivery
  protocol, package description document, constructor interfaces;
- the migration folders: added (an upgrade note); removed, renamed or
  changed (always fails);
- the flags and environment variables of `cloacina-server`, and the
  generated reference page of them.

An intended break is marked by naming it in backticks in `## [Unreleased]` of
`CHANGELOG.md`, under `### Changed (breaking)`; a migration under
`### Upgrade notes`. When the
workspace version differs from the release tag (a bump pull request, or the
release itself), the section of the new version is read instead, the
Unreleased section above it must be empty, and in a patch release every
listed change fails, marked or not.

    python3 scripts/check_release_compat.py                 # against the last v* tag
    python3 scripts/check_release_compat.py --base v0.12.0
    python3 scripts/check_release_compat.py --release v0.13.0   # at release
    python3 scripts/check_release_compat.py --warn-only
    python3 scripts/check_release_compat.py flags --write   # regenerate the flags page
    python3 scripts/check_release_compat.py flags --check

Standard library, plus PyYAML for the chart values. `helm` for the render.
Exit code 0: nothing fails the gate (or `--warn-only`). 1: something fails.
2: the script could not run (no tag, no git).
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field

OPENAPI = "docs/public/openapi.json"
CHARTS = ("cloacina-server", "cloacina-agent")
# Values set on top of the old values file so the render gets past the
# chart's own "configure a database / a server" checks.
RENDER_SETS = {
    "cloacina-server": ["database.url=postgres://u:p@h:5432/d"],
    "cloacina-agent": ["server.url=http://cloacina-server:8080", "apiKey=dev-key"],
}
CATALOG = "docs/src/content/docs/reference/metrics-catalog.md"
CODE_ROOT = "crates"
MIGRATIONS = "crates/cloacina/src/database/migrations"
BACKENDS = ("postgres", "sqlite")
SERVER_MAIN = "crates/cloacina-server/src/main.rs"
SERVER_SRC = "crates/cloacina-server/src"
FLAGS_PAGE = "docs/src/content/docs/reference/server-flags.md"
CHANGELOG = "CHANGELOG.md"

# (file, regex with one group for the name and one for the number)
CONSTANTS = (
    ("crates/cloacina-workflow-plugin/src/lib.rs",
     r"(plugin_interface)\s*\(\s*version\s*=\s*(\d+)"),
    ("crates/cloacina/src/fleet/protocol.rs",
     r"\b(AGENT_PROTOCOL_VERSION)\s*:\s*u32\s*=\s*(\d+)"),
    ("crates/cloacina-api-types/src/delivery.rs",
     r"\b(DELIVERY_PROTOCOL_VERSION)\s*:\s*u32\s*=\s*(\d+)"),
    ("crates/cloacina/src/registry/package_description.rs",
     r"\b(PACKAGE_DESCRIPTION_VERSION)\s*:\s*u32\s*=\s*(\d+)"),
    ("crates/cloacina-constructor-contract/src/lib.rs",
     r"\b(\w+_INTERFACE_VERSION)\s*:\s*u32\s*=\s*(\d+)"),
)

BREAKING = "Changed (breaking)"
UPGRADE_NOTES = "Upgrade notes"

BREAK = "break"          # needs an entry; fails in a patch
UPGRADE = "upgrade"      # needs an entry; fails in a patch
NOTE = "note"            # reported; fails in a patch
ERROR = "error"          # always fails; no entry fixes it


@dataclass
class Finding:
    kind: str
    file: str
    change: str
    names: list = field(default_factory=list)

    @property
    def section(self) -> str:
        return UPGRADE_NOTES if self.kind in (UPGRADE, NOTE) else BREAKING


# ---------------------------------------------------------------------------
# Sources: the files at a tag, or in a folder
# ---------------------------------------------------------------------------

class TreeSource:
    """The files of a folder (the working tree, or a test fixture)."""

    def __init__(self, root: str, label: str = "head"):
        self.root = root
        self.label = label

    def read(self, path: str):
        full = os.path.join(self.root, path)
        if not os.path.isfile(full):
            return None
        with open(full, encoding="utf-8") as fh:
            return fh.read()

    def list_dir(self, path: str):
        full = os.path.join(self.root, path)
        if not os.path.isdir(full):
            return []
        return sorted(os.listdir(full))

    def files(self, prefix: str, suffix: str):
        out = []
        base = os.path.join(self.root, prefix)
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames[:] = [d for d in dirnames if d not in ("target", "node_modules", ".git")]
            for name in filenames:
                if name.endswith(suffix):
                    out.append(os.path.relpath(os.path.join(dirpath, name), self.root))
        return sorted(p.replace(os.sep, "/") for p in out)


class GitSource:
    """The files at a git revision, read with `git show <rev>:<path>`."""

    def __init__(self, root: str, rev: str):
        self.root = root
        self.rev = rev
        self.label = rev

    def _git(self, *args):
        return subprocess.run(["git", "-C", self.root, *args],
                              capture_output=True, text=True, check=False)

    def read(self, path: str):
        proc = self._git("show", f"{self.rev}:{path}")
        return proc.stdout if proc.returncode == 0 else None

    def list_dir(self, path: str):
        proc = self._git("ls-tree", "--name-only", f"{self.rev}:{path}")
        if proc.returncode != 0:
            return []
        return sorted(line for line in proc.stdout.splitlines() if line)

    def files(self, prefix: str, suffix: str):
        proc = self._git("ls-tree", "-r", "--name-only", self.rev, "--", prefix)
        return sorted(p for p in proc.stdout.splitlines() if p.endswith(suffix))


# ---------------------------------------------------------------------------
# Rust source helpers
# ---------------------------------------------------------------------------

def strip_rust_comments(text: str) -> str:
    """Blank out `//` and `/* */` comments, keep string literals and lines."""
    out = []
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            out.append(text[i:j + 1])
            i = j + 1
        elif text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            i = j
        elif text.startswith("/*", i):
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append("\n" * text.count("\n", i, j))
            i = j
        else:
            out.append(c)
            i += 1
    return "".join(out)


def balanced(text: str, start: int, open_ch: str = "(", close_ch: str = ")"):
    """The text between `text[start]` (an open bracket) and its match."""
    depth = 0
    i = start
    n = len(text)
    while i < n:
        c = text[i]
        if c == '"':
            i += 1
            while i < n and text[i] != '"':
                i += 2 if text[i] == "\\" else 1
        elif c == open_ch:
            depth += 1
        elif c == close_ch:
            depth -= 1
            if depth == 0:
                return text[start + 1:i]
        i += 1
    return text[start + 1:]


def split_top(text: str):
    """Split on commas that are not inside brackets or strings."""
    parts, depth, cur, i = [], 0, [], 0
    while i < len(text):
        c = text[i]
        if c == '"':
            j = i + 1
            while j < len(text) and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            cur.append(text[i:j + 1])
            i = j + 1
            continue
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        if c == "," and depth == 0:
            parts.append("".join(cur).strip())
            cur = []
        else:
            cur.append(c)
        i += 1
    tail = "".join(cur).strip()
    if tail:
        parts.append(tail)
    return parts


def workspace_version(src) -> str | None:
    text = src.read("Cargo.toml") or ""
    m = re.search(r"\[workspace\.package\][^\[]*?\bversion\s*=\s*\"([^\"]+)\"", text, re.DOTALL)
    return m.group(1) if m else None


def parse_version(v: str):
    m = re.match(r"^v?(\d+)\.(\d+)\.(\d+)", v or "")
    return tuple(int(x) for x in m.groups()) if m else None


# ---------------------------------------------------------------------------
# HTTP API
# ---------------------------------------------------------------------------

@dataclass
class Node:
    owner: str | None
    prop: str | None
    types: set = field(default_factory=set)
    nullable: bool = False
    required: bool = False
    enum: set | None = None


def _parent(path: str) -> str:
    if path.endswith("[]") or path.endswith("{}"):
        return path[:-2]
    return path.rsplit(".", 1)[0] if "." in path else ""


def _join(path: str, prop: str) -> str:
    return f"{path}.{prop}" if path else prop


class Flattener:
    """Flattens the schemas of an operation into field paths."""

    def __init__(self, spec: dict):
        self.comps = spec.get("components", {}).get("schemas", {})

    def flatten(self, schema) -> dict:
        acc: dict = {}
        self._walk(schema, "", None, None, True, frozenset(), acc, None)
        return acc

    def _walk(self, s, path, owner, prop, required, seen, acc, cur):
        if not isinstance(s, dict):
            return
        if "$ref" in s:
            name = s["$ref"].rsplit("/", 1)[-1]
            node = acc.setdefault(path, Node(owner, prop))
            node.required = node.required or required
            if name in seen:
                return
            self._walk(self.comps.get(name, {}), path, owner, prop, required,
                       seen | {name}, acc, name)
            return
        node = acc.setdefault(path, Node(owner, prop))
        node.required = node.required or required
        t = s.get("type")
        types = set(t) if isinstance(t, list) else ({t} if isinstance(t, str) else set())
        if "null" in types or s.get("nullable") is True:
            node.nullable = True
        node.types |= types - {"null"}
        values = None
        if "enum" in s:
            values = {json.dumps(v) for v in s["enum"] if v is not None}
            if None in s["enum"]:
                node.nullable = True
        elif "const" in s:
            values = {json.dumps(s["const"])}
        if values is not None:
            node.enum = (node.enum or set()) | values
        for sub in s.get("allOf", []):
            self._walk(sub, path, owner, prop, required, seen, acc, cur)
        for key in ("oneOf", "anyOf"):
            for sub in s.get(key, []):
                if isinstance(sub, dict) and sub.get("type") == "null" and len(
                        [k for k in sub if k != "description"]) == 1:
                    node.nullable = True
                else:
                    self._walk(sub, path, owner, prop, required, seen, acc, cur)
        req = set(s.get("required", []))
        for p, ps in (s.get("properties") or {}).items():
            self._walk(ps, _join(path, p), cur, p, p in req, seen, acc, cur)
        if isinstance(s.get("items"), dict):
            self._walk(s["items"], path + "[]", owner, prop, True, seen, acc, cur)
        if isinstance(s.get("additionalProperties"), dict):
            self._walk(s["additionalProperties"], path + "{}", owner, prop, True, seen, acc, cur)


def _operations(spec: dict):
    flat = Flattener(spec)
    params_comp = spec.get("components", {}).get("parameters", {})
    ops = {}
    for path, item in (spec.get("paths") or {}).items():
        common = item.get("parameters", [])
        for method, op in item.items():
            if method not in ("get", "put", "post", "delete", "patch", "head", "options"):
                continue
            key = f"{method.upper()} {path}"
            params = {}
            for p in list(common) + list(op.get("parameters", [])):
                if "$ref" in p:
                    p = params_comp.get(p["$ref"].rsplit("/", 1)[-1], {})
                params[(p.get("in"), p.get("name"))] = bool(p.get("required"))
            request = {}
            body = op.get("requestBody") or {}
            for media in (body.get("content") or {}).values():
                for k, v in flat.flatten(media.get("schema", {})).items():
                    request.setdefault(k, v)
            if body and not body.get("required", False) and "" in request:
                request[""].required = False
            response = {}
            for status, resp in (op.get("responses") or {}).items():
                if not str(status).startswith("2"):
                    continue
                for media in (resp.get("content") or {}).values():
                    for k, v in flat.flatten(media.get("schema", {})).items():
                        response.setdefault(k, v)
            ops[key] = {
                "params": params,
                "statuses": set(str(s) for s in (op.get("responses") or {})),
                "request": request,
                "response": response,
            }
    return ops


def _field_name(op, side, path, node):
    if node.owner and node.prop:
        return f"{node.owner}.{node.prop}"
    return f"{op} {side} {path or 'body'}"


def _enum_text(values):
    return ", ".join(sorted(v.strip('"') for v in values))


def compare_fields(op, side, old, new, emit):
    for path, o in old.items():
        n = new.get(path)
        name = _field_name(op, side, path, o)
        where = f"{side} field `{name}`" if path else f"{side} body"
        if n is None:
            parent = _parent(path)
            if path and parent in old and parent not in new:
                continue
            emit(BREAK, f"{where} removed", name, op)
            continue
        if o.types and n.types and o.types != n.types:
            emit(BREAK, f"{where} type changed from {'/'.join(sorted(o.types))} "
                        f"to {'/'.join(sorted(n.types))}", name, op)
        if o.enum is not None and n.enum is not None:
            gone = o.enum - n.enum
            if gone:
                emit(BREAK, f"{where}: enum value {_enum_text(gone)} removed", name, op)
            added = n.enum - o.enum
            if added and side == "response":
                emit(NOTE, f"{where}: enum value {_enum_text(added)} added", name, op)
        elif o.enum is None and n.enum is not None and side == "request":
            emit(BREAK, f"{where} now takes only {_enum_text(n.enum)}", name, op)
        if side == "response" and n.nullable and not o.nullable and path:
            emit(BREAK, f"{where} may now be null", name, op)
        if side == "request" and o.nullable and not n.nullable:
            emit(BREAK, f"{where} may no longer be null", name, op)
        if side == "request" and n.required and not o.required:
            emit(BREAK, f"{where} is now required", name, op)
    if side == "request":
        for path, n in new.items():
            if path in old or not n.required or not path:
                continue
            parent = _parent(path)
            if parent in old:
                name = _field_name(op, side, path, n)
                emit(BREAK, f"new required request field `{name}`", name, op)


def compare_openapi(old_text, new_text):
    findings = []
    if old_text is None:
        return findings
    if new_text is None:
        return [Finding(BREAK, OPENAPI, "the OpenAPI document was removed", [OPENAPI])]
    old_ops = _operations(json.loads(old_text))
    new_ops = _operations(json.loads(new_text))
    grouped: dict = {}

    def emit(kind, change, name, op):
        key = (kind, change)
        entry = grouped.setdefault(key, (name, []))
        if op not in entry[1]:
            entry[1].append(op)

    for op, o in old_ops.items():
        n = new_ops.get(op)
        if n is None:
            findings.append(Finding(BREAK, OPENAPI, f"operation `{op}` removed", [op]))
            continue
        for (where, pname), req in o["params"].items():
            if (where, pname) not in n["params"]:
                findings.append(Finding(BREAK, OPENAPI,
                                        f"`{op}`: {where} parameter `{pname}` removed", [op]))
        for (where, pname), req in n["params"].items():
            if req and not o["params"].get((where, pname), False) and where != "path":
                findings.append(Finding(BREAK, OPENAPI,
                                        f"`{op}`: {where} parameter `{pname}` is now required", [op]))
        for status in sorted(o["statuses"] - n["statuses"]):
            findings.append(Finding(BREAK, OPENAPI,
                                    f"`{op}`: response status {status} no longer listed", [op]))
        compare_fields(op, "request", o["request"], n["request"], emit)
        compare_fields(op, "response", o["response"], n["response"], emit)
    for (kind, change), (name, ops) in grouped.items():
        names = [name] + ops
        more = f" (in `{ops[0]}`" + (f" and {len(ops) - 1} more operations)" if len(ops) > 1 else ")")
        findings.append(Finding(kind, OPENAPI, change + more, names))
    return findings


# ---------------------------------------------------------------------------
# Chart values
# ---------------------------------------------------------------------------

def _load_yaml(text):
    try:
        import yaml  # type: ignore
    except ImportError:
        raise SystemExit("check_release_compat: PyYAML is needed for the chart values "
                         "(pip install pyyaml)")
    return yaml.safe_load(text) or {}


def flatten_values(data, prefix=""):
    out = {}
    if isinstance(data, dict) and data:
        for k, v in data.items():
            path = f"{prefix}.{k}" if prefix else str(k)
            if isinstance(v, dict) and v:
                out.update(flatten_values(v, path))
            else:
                out[path] = v
    return out


def compare_values(chart, old_text, new_text):
    path = f"charts/{chart}/values.yaml"
    if old_text is None:
        return []
    if new_text is None:
        return [Finding(BREAK, path, "the values file was removed", [path])]
    old = flatten_values(_load_yaml(old_text))
    new = flatten_values(_load_yaml(new_text))
    findings = []
    for key, value in old.items():
        names = [key, f"{chart}.{key}"]
        if key in new:
            if new[key] != value:
                findings.append(Finding(NOTE, path, f"default of `{key}` changed from "
                                        f"{json.dumps(value)} to {json.dumps(new[key])}", names))
            continue
        if any(k.startswith(key + ".") for k in new):
            if value == {} or value is None:
                continue
            findings.append(Finding(BREAK, path, f"value `{key}` changed from a value to a map", names))
            continue
        findings.append(Finding(BREAK, path, f"value `{key}` removed or renamed", names))
    return findings


def render_with_old_values(root, chart, old_text):
    """`helm template` of the head chart with the release's values file."""
    path = f"charts/{chart}/values.yaml"
    if old_text is None or not os.path.isdir(os.path.join(root, "charts", chart)):
        return []
    if shutil.which("helm") is None:
        return [Finding(ERROR, path, "helm is not on PATH: the render with the old values "
                        "file did not run (install helm, or pass --skip-render)", [])]
    with tempfile.NamedTemporaryFile("w", suffix=".yaml", delete=False) as fh:
        fh.write(old_text)
        tmp = fh.name
    try:
        cmd = ["helm", "template", "release-gate", os.path.join(root, "charts", chart), "-f", tmp]
        for item in RENDER_SETS.get(chart, []):
            cmd += ["--set", item]
        proc = subprocess.run(cmd, capture_output=True, text=True, check=False)
    finally:
        os.unlink(tmp)
    if proc.returncode == 0:
        return []
    err = proc.stderr.strip().splitlines()
    msg = err[-1] if err else "no output"
    if "missing in charts/ directory" in proc.stderr:
        return [Finding(ERROR, path, f"render needs the chart dependencies: run "
                        f"`helm dependency update charts/{chart}` ({msg})", [])]
    if "execution error at" in proc.stderr:
        return [Finding(NOTE, path, f"the head chart refuses the old values file with its own "
                        f"message: {msg}", [path])]
    return [Finding(BREAK, path, f"the head chart does not render with the old values file: {msg}",
                    [path])]


# ---------------------------------------------------------------------------
# Metrics
# ---------------------------------------------------------------------------

def parse_catalog(text):
    """name -> (kind, set of labels), from the tables of the catalog."""
    out = {}
    if text is None:
        return None
    kind = None
    for line in text.splitlines():
        if line.startswith("#"):
            h = re.match(r"^###\s+(Counters|Histograms|Gauges)\s*$", line)
            kind = h.group(1).lower()[:-1] if h else None
            continue
        m = re.match(r"^\|\s*`(cloacina_[a-z0-9_]+)`\s*\|([^|]*)\|", line)
        if m and kind:
            labels = set(re.findall(r"`([a-z_][a-z0-9_]*)`", m.group(2)))
            out[m.group(1)] = (kind, labels)
    return out


def compare_catalog(old_text, new_text):
    old = parse_catalog(old_text)
    new = parse_catalog(new_text)
    if old is None:
        return []
    if new is None:
        return [Finding(BREAK, CATALOG, "the metrics catalog was removed", [CATALOG])]
    findings = []
    for name, (kind, labels) in old.items():
        if name not in new:
            findings.append(Finding(BREAK, CATALOG, f"metric `{name}` removed or renamed", [name]))
            continue
        nkind, nlabels = new[name]
        if nkind != kind:
            findings.append(Finding(BREAK, CATALOG, f"metric `{name}` changed from a {kind} "
                                    f"to a {nkind}", [name]))
        for label in sorted(labels - nlabels):
            findings.append(Finding(BREAK, CATALOG, f"label `{label}` of metric `{name}` removed",
                                    [name, f"{name}{{{label}}}"]))
    return findings


_METRIC_MACRO = re.compile(
    r"\b(describe_counter|describe_gauge|describe_histogram|counter|gauge|histogram)!\s*\(")


def _cut_tests(text):
    m = re.search(r"#\[cfg\(test\)\]\s*(pub\s+)?mod\s+\w+\s*\{", text)
    return text[:m.start()] if m else text


def scan_metrics(src, files):
    """name -> {"labels": set or None (unknown), "where": [file:line]}; plus errors."""
    found, errors = {}, []
    for path in files:
        parts = path.split("/")
        if "tests" in parts or parts[-1] in ("tests.rs", "test.rs") or "benches" in parts:
            continue
        raw = src.read(path)
        if raw is None or "!(" not in raw:
            continue
        text = _cut_tests(strip_rust_comments(raw))
        consts = dict(re.findall(r"const\s+([A-Z0-9_]+)\s*:\s*&(?:'static\s+)?str\s*=\s*\"([^\"]+)\"", text))
        for m in _METRIC_MACRO.finditer(text):
            line = text.count("\n", 0, m.start()) + 1
            args = split_top(balanced(text, m.end() - 1))
            if not args:
                continue
            first = args[0]
            lit = re.fullmatch(r"\"([^\"]+)\"", first)
            if lit:
                name = lit.group(1)
            else:
                ident = re.fullmatch(r"(?:[\w:]+::)?([A-Z0-9_]+)", first)
                name = consts.get(ident.group(1)) if ident else None
                if name is None:
                    errors.append(Finding(ERROR, f"{path}:{line}",
                                          f"metric name `{first}` is not a literal or a const "
                                          "of the same file: the gate cannot check it", []))
                    continue
            entry = found.setdefault(name, {"labels": set(), "where": []})
            entry["where"].append(f"{path}:{line}")
            if m.group(1).startswith("describe_"):
                continue
            for arg in args[1:]:
                pair = re.match(r"^\"([^\"]+)\"\s*=>", arg)
                if pair:
                    if entry["labels"] is not None:
                        entry["labels"].add(pair.group(1))
                else:
                    entry["labels"] = None
    return found, errors


def check_code_against_catalog(src):
    catalog = parse_catalog(src.read(CATALOG))
    if catalog is None:
        return []
    found, findings = scan_metrics(src, src.files(CODE_ROOT, ".rs"))
    for name, entry in sorted(found.items()):
        if name not in catalog:
            findings.append(Finding(ERROR, CATALOG, f"metric `{name}` is in the code "
                                    f"({entry['where'][0]}) and not in the catalog: add a row", []))
            continue
        if entry["labels"]:
            for label in sorted(entry["labels"] - catalog[name][1]):
                findings.append(Finding(ERROR, CATALOG, f"label `{label}` of metric `{name}` is in "
                                        f"the code ({entry['where'][0]}) and not in the catalog", []))
    for name in sorted(set(catalog) - set(found)):
        findings.append(Finding(ERROR, CATALOG, f"metric `{name}` is in the catalog and no code "
                                "registers or records it", []))
    return findings


# ---------------------------------------------------------------------------
# Version constants and migrations
# ---------------------------------------------------------------------------

def read_constants(src):
    out = {}
    for path, pattern in CONSTANTS:
        text = src.read(path)
        if text is None:
            continue
        for name, value in re.findall(pattern, strip_rust_comments(text)):
            out[name] = (path, int(value))
    return out


def compare_constants(base, head):
    old, new = read_constants(base), read_constants(head)
    findings = []
    for name, (path, value) in sorted(old.items()):
        if name not in new:
            findings.append(Finding(BREAK, path, f"version constant `{name}` removed", [name]))
        elif new[name][1] != value:
            findings.append(Finding(BREAK, new[name][0], f"`{name}` changed from {value} to "
                                    f"{new[name][1]}", [name]))
    return findings


def compare_migrations(base, head):
    findings = []
    added: dict = {}
    for backend in BACKENDS:
        folder = f"{MIGRATIONS}/{backend}"
        old = set(base.list_dir(folder))
        new = set(head.list_dir(folder))
        for name in sorted(old - new):
            findings.append(Finding(ERROR, f"{folder}/{name}", f"released migration `{name}` "
                                    "removed or renamed: a migration never changes after its release", []))
        for name in sorted(old & new):
            up = f"{folder}/{name}/up.sql"
            if base.read(up) != head.read(up):
                findings.append(Finding(ERROR, up, f"released migration `{name}` changed: add a new "
                                        "migration instead", []))
        for name in sorted(new - old):
            if not re.match(r"^\d", name):
                continue
            number = name.split("_", 1)[0]
            added.setdefault(number, {}).setdefault(name, []).append(backend)
    for number, names in sorted(added.items()):
        text = ", ".join(f"`{n}` ({'/'.join(b)})" for n, b in sorted(names.items()))
        findings.append(Finding(UPGRADE, MIGRATIONS, f"migration {text} added",
                                [number] + sorted(names)))
    return findings


# ---------------------------------------------------------------------------
# Server flags and environment variables
# ---------------------------------------------------------------------------

def _kebab(name):
    return name.replace("_", "-")


def parse_server_flags(text):
    """`--flag` -> {"long", "short", "env", "default", "type"}, from `struct Cli`."""
    if text is None:
        return None
    code = strip_rust_comments(text)
    m = re.search(r"struct\s+Cli\s*\{", code)
    if not m:
        return {}
    body = balanced(code, m.end() - 1, "{", "}")
    flags = {}
    i = 0
    pending = []
    while i < len(body):
        a = re.compile(r"#\[\s*arg\s*\(").match(body, i)
        if a:
            inner = balanced(body, a.end() - 1)
            pending.append(inner)
            i = a.end() - 1 + len(inner) + 2
            continue
        other = re.compile(r"#\[").match(body, i)
        if other:
            inner = balanced(body, other.end() - 1, "[", "]")
            if "subcommand" in inner:
                pending = [None]
            i = other.end() - 1 + len(inner) + 2
            continue
        f = re.compile(r"(?:pub\s+)?([a-z_][a-z0-9_]*)\s*:\s*").match(body, i)
        if f:
            j = f.end()
            depth = 0
            while j < len(body):
                ch = body[j]
                if ch in "<([":
                    depth += 1
                elif ch in ">)]":
                    depth -= 1
                elif ch == "," and depth == 0:
                    break
                j += 1
            ftype = " ".join(body[f.end():j].split())
            attrs = [p for p in pending if p is not None]
            if attrs and None not in pending:
                info = {"long": None, "short": None, "env": None, "default": None, "type": ftype}
                for attr in attrs:
                    for part in split_top(attr):
                        kv = re.match(r"^(\w+)\s*(?:=\s*(.+))?$", part, re.DOTALL)
                        if not kv:
                            continue
                        key, val = kv.group(1), (kv.group(2) or "").strip()
                        if key == "long":
                            info["long"] = val.strip('"') if val else _kebab(f.group(1))
                        elif key == "short":
                            info["short"] = val.strip("'") if val else f.group(1)[0]
                        elif key == "env":
                            info["env"] = val.strip('"')
                        elif key in ("default_value", "default_values"):
                            info["default"] = val
                        elif key in ("default_value_t", "default_value_os_t"):
                            info["default"] = " ".join(val.split()) if val else "Default::default()"
                if info["long"]:
                    flags["--" + info["long"]] = info
            pending = []
            i = j + 1
            continue
        i += 1
    return flags


def server_env_only(src, flags):
    names = set()
    for path in src.files(SERVER_SRC, ".rs"):
        parts = path.split("/")
        if "tests" in parts or parts[-1] in ("tests.rs", "test.rs"):
            continue
        text = _cut_tests(strip_rust_comments(src.read(path) or ""))
        names |= set(re.findall(r"env::var(?:_os)?\(\s*\"([A-Z][A-Z0-9_]*)\"\s*\)", text))
        names |= set(re.findall(r"\"(CLOACINA_[A-Z0-9_]+)\"", text))
    flag_envs = {f["env"] for f in (flags or {}).values() if f["env"]}
    return sorted(names - flag_envs)


def render_flags_page(flags, env_only):
    lines = [
        "---",
        'title: "Server Flags"',
        'description: "Every flag and environment variable of cloacina-server, generated from its command."',
        "sidebar:",
        "  order: 16",
        "---",
        "",
        "This page is generated from the `cloacina-server` command",
        f"(`{SERVER_MAIN}`) by `python3 scripts/check_release_compat.py flags --write`.",
        "Do not edit it by hand: the release gate fails when it differs from the",
        "code, and compares the flags and environment variables with the last",
        "release (see [the release gate](../../contributing/release-gate/)).",
        "",
        "A default is the text of the code: a literal, or the name of a constant",
        "of the code. For the meaning of each setting, see",
        "[Configuration](../configuration/) and",
        "[Environment variables](../environment-variables/).",
        "",
        "## Flags",
        "",
        "| Flag | Environment variable | Default |",
        "|------|----------------------|---------|",
    ]
    for name in sorted(flags):
        f = flags[name]
        flag = f"`{name}`" + (f", `-{f['short']}`" if f["short"] else "")
        env = f"`{f['env']}`" if f["env"] else "—"
        default = f"`{f['default']}`" if f["default"] else "—"
        lines.append(f"| {flag} | {env} | {default} |")
    lines += ["", "## Environment variables with no flag", "",
              "The environment variable names in the server code (tests left out) that",
              "no flag reads: a name read with `env::var`, or a `CLOACINA_*` string. Most",
              "are settings the server reads; some are variables it sets on the agent and",
              "compiler pods it starts.", ""]
    if env_only:
        lines += [f"- `{name}`" for name in env_only]
    else:
        lines.append("None.")
    return "\n".join(lines) + "\n"


def compare_flags(base, head):
    old = parse_server_flags(base.read(SERVER_MAIN))
    new = parse_server_flags(head.read(SERVER_MAIN))
    if old is None:
        return []
    if new is None:
        return [Finding(BREAK, SERVER_MAIN, "the server command was not found", [SERVER_MAIN])]
    findings = []
    for name, info in sorted(old.items()):
        names = [name] + ([info["env"]] if info["env"] else [])
        n = new.get(name)
        if n is None:
            findings.append(Finding(BREAK, SERVER_MAIN, f"flag `{name}` removed or renamed", names))
            continue
        if info["env"] and n["env"] != info["env"]:
            findings.append(Finding(BREAK, SERVER_MAIN, f"environment variable `{info['env']}` of "
                                    f"`{name}` removed or renamed", names))
        if (info["default"] or "") != (n["default"] or ""):
            findings.append(Finding(NOTE, SERVER_MAIN, f"default of `{name}` changed from "
                                    f"`{info['default'] or 'none'}` to `{n['default'] or 'none'}`",
                                    names))
    old_env = set(server_env_only(base, old))
    new_env = set(server_env_only(head, new)) | {f["env"] for f in new.values() if f["env"]}
    for env in sorted(old_env - new_env):
        findings.append(Finding(BREAK, SERVER_SRC, f"environment variable `{env}` is no longer read",
                                [env]))
    return findings


def check_flags_page(head):
    flags = parse_server_flags(head.read(SERVER_MAIN))
    if flags is None:
        return []
    want = render_flags_page(flags, server_env_only(head, flags))
    if head.read(FLAGS_PAGE) != want:
        return [Finding(ERROR, FLAGS_PAGE, "the flags page differs from the server command: run "
                        "`python3 scripts/check_release_compat.py flags --write`", [])]
    return []


# ---------------------------------------------------------------------------
# The changelog
# ---------------------------------------------------------------------------

def changelog_sections(text):
    """[(heading inside the brackets, body)] in file order."""
    heads = list(re.finditer(r"^##\s*\[([^\]]+)\][^\n]*$", text or "", re.MULTILINE))
    out = []
    for i, h in enumerate(heads):
        end = heads[i + 1].start() if i + 1 < len(heads) else len(text)
        out.append((h.group(1), text[h.end():end]))
    return out


def subsections(body):
    heads = list(re.finditer(r"^###\s+(.+?)\s*$", body, re.MULTILINE))
    out = {}
    for i, h in enumerate(heads):
        end = heads[i + 1].start() if i + 1 < len(heads) else len(body)
        out[h.group(1)] = body[h.end():end]
    return out


def marked_names(body):
    """section title -> the backticked names under it, for the two sections."""
    names = {BREAKING: set(), UPGRADE_NOTES: set()}
    for title, text in subsections(body).items():
        for section in names:
            if title.lower().startswith(section.lower()):
                names[section] |= set(re.findall(r"`([^`\n]+)`", text))
    return names


def has_entries(body):
    return bool(re.search(r"^\s*(?:[-*+]|\d+\.)\s+\S", body, re.MULTILINE))


# ---------------------------------------------------------------------------
# The gate
# ---------------------------------------------------------------------------

@dataclass
class Result:
    failures: list
    passed: list
    notes: list
    header: list


def run_gate(base, head, root=None, render=True):
    """Compare `base` (the release) with `head`; return a Result."""
    base_version = workspace_version(base) or ""
    head_version = workspace_version(head) or ""
    bv, hv = parse_version(base_version), parse_version(head_version)
    release = hv is not None and bv is not None and hv != bv
    patch = release and hv[:2] == bv[:2]
    findings = []
    findings += compare_openapi(base.read(OPENAPI), head.read(OPENAPI))
    for chart in CHARTS:
        old_values = base.read(f"charts/{chart}/values.yaml")
        findings += compare_values(chart, old_values, head.read(f"charts/{chart}/values.yaml"))
        if render and root is not None:
            findings += render_with_old_values(root, chart, old_values)
    findings += compare_catalog(base.read(CATALOG), head.read(CATALOG))
    findings += check_code_against_catalog(head)
    findings += compare_constants(base, head)
    findings += compare_migrations(base, head)
    findings += compare_flags(base, head)
    findings += check_flags_page(head)

    changelog = head.read(CHANGELOG) or ""
    sections = changelog_sections(changelog)
    keys = [k for k, _ in sections]
    target = "Unreleased"
    if release:
        target = head_version if head_version in keys else "Unreleased"
    section_body = dict(sections).get(target)
    header = [f"release gate: {head.label} against {base.label} "
              f"({base_version} -> {head_version}"
              + (", patch release: every listed change fails" if patch else
                 ", release" if release else "") + ")",
              f"entries are read from `## [{target}]` of {CHANGELOG}"]
    if section_body is None:
        findings.append(Finding(ERROR, CHANGELOG, f"no `## [{target}]` section", []))
        section_body = ""
    if release:
        if head_version not in keys:
            findings.append(Finding(ERROR, CHANGELOG, f"no `## [{head_version}]` section: run "
                                    f"`angreal release bump {head_version}`", []))
        else:
            idx = keys.index(head_version)
            if "Unreleased" not in keys[:idx]:
                findings.append(Finding(ERROR, CHANGELOG, "no `## [Unreleased]` above "
                                        f"`## [{head_version}]`", []))
            elif has_entries(dict(sections)["Unreleased"]):
                findings.append(Finding(ERROR, CHANGELOG, "`## [Unreleased]` has entries that are "
                                        f"not under `## [{head_version}]`", []))
            if patch:
                body = subsections(dict(sections)[head_version])
                for title, text in body.items():
                    if title.lower().startswith(BREAKING.lower()) and has_entries(text):
                        findings.append(Finding(ERROR, CHANGELOG, f"patch release {head_version} has "
                                                f"entries under `### {BREAKING}`: cut a minor", []))

    marks = marked_names(section_body)
    failures, passed, notes = [], [], []
    for f in findings:
        if f.kind == ERROR:
            failures.append(f)
        elif patch:
            failures.append(f)
        elif f.kind == NOTE:
            notes.append(f)
        elif any(n in marks[f.section] for n in f.names):
            passed.append(f)
        else:
            failures.append(f)
    return Result(failures, passed, notes, header)


def describe(f: Finding, base_label: str, target: str, patch: bool) -> str:
    if f.kind == ERROR:
        return f"{f.file}: {f.change}"
    if patch:
        return f"{f.file}: {f.change} since {base_label}; a patch release may not change this"
    name = f.names[0] if f.names else f.file
    return (f"{f.file}: {f.change} since {base_label}; name `{name}` under "
            f"`## [{target}]` / `### {f.section}`")


def print_result(result: Result, base_label: str, warn_only: bool) -> int:
    gha = os.environ.get("GITHUB_ACTIONS") == "true"
    target = re.search(r"## \[([^\]]+)\]", result.header[1]).group(1)
    patch = "patch release" in result.header[0]
    for line in result.header:
        print(line)
    level = "warning" if warn_only else "error"
    if result.failures:
        print(f"\n{len(result.failures)} change(s) fail the gate:")
        for f in result.failures:
            text = describe(f, base_label, target, patch)
            print(f"  FAIL {text}")
            if gha:
                print(f"::{level} file={f.file.split(':')[0]}::{text}")
    if result.passed:
        print(f"\n{len(result.passed)} change(s) named in the changelog:")
        for f in result.passed:
            print(f"  ok   {f.file}: {f.change}")
    if result.notes:
        print(f"\n{len(result.notes)} note(s), no entry needed in a minor:")
        for f in result.notes:
            print(f"  note {f.file}: {f.change}")
    if not result.failures:
        print("\nrelease gate: pass")
        return 0
    if warn_only:
        print(f"\nrelease gate: warn-only, {len(result.failures)} failure(s) not enforced")
        return 0
    print(f"\nrelease gate: {len(result.failures)} failure(s)")
    return 1


def _git(root, *args):
    return subprocess.run(["git", "-C", root, *args], capture_output=True, text=True, check=False)


def last_tag(root):
    proc = _git(root, "describe", "--tags", "--abbrev=0", "--match", "v*")
    return proc.stdout.strip() if proc.returncode == 0 else None


def previous_tag(root, tag):
    want = parse_version(tag)
    proc = _git(root, "tag", "--list", "v*", "--merged", tag)
    tags = [t for t in proc.stdout.split() if parse_version(t) and parse_version(t) < want]
    return max(tags, key=parse_version) if tags else None


def main(argv=None) -> int:
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    argv = list(sys.argv[1:] if argv is None else argv)
    if argv and argv[0] == "flags":
        p = argparse.ArgumentParser(prog="check_release_compat.py flags")
        g = p.add_mutually_exclusive_group(required=True)
        g.add_argument("--write", action="store_true")
        g.add_argument("--check", action="store_true")
        args = p.parse_args(argv[1:])
        head = TreeSource(root)
        flags = parse_server_flags(head.read(SERVER_MAIN)) or {}
        page = render_flags_page(flags, server_env_only(head, flags))
        if args.write:
            with open(os.path.join(root, FLAGS_PAGE), "w", encoding="utf-8") as fh:
                fh.write(page)
            print(f"wrote {FLAGS_PAGE}: {len(flags)} flags")
            return 0
        bad = check_flags_page(head)
        for f in bad:
            print(f"{f.file}: {f.change}", file=sys.stderr)
        return 1 if bad else 0
    if argv and argv[0] == "check":
        argv = argv[1:]
    p = argparse.ArgumentParser(description="the release gate (CLOACINA-T-1089)")
    p.add_argument("--base", help="the release tag to compare with (default: the last v* tag)")
    p.add_argument("--release", help="the tag being released; the base is the tag before it")
    p.add_argument("--warn-only", action="store_true", help="report, and exit 0")
    p.add_argument("--skip-render", action="store_true", help="do not run helm template")
    args = p.parse_args(argv)
    if args.release:
        base_tag = args.base or previous_tag(root, args.release)
        head_version = workspace_version(TreeSource(root))
        if parse_version(args.release) != parse_version(head_version or ""):
            print(f"release gate: tag {args.release} is not the workspace version {head_version}",
                  file=sys.stderr)
            return 1
    else:
        base_tag = args.base or last_tag(root)
    if not base_tag:
        print("release gate: no v* tag found (fetch the tags: fetch-depth 0)", file=sys.stderr)
        return 2
    if _git(root, "rev-parse", "--verify", "--quiet", f"{base_tag}^{{commit}}").returncode != 0:
        print(f"release gate: tag {base_tag} not found", file=sys.stderr)
        return 2
    result = run_gate(GitSource(root, base_tag), TreeSource(root), root=root,
                      render=not args.skip_render)
    return print_result(result, base_tag, args.warn_only)


if __name__ == "__main__":
    sys.exit(main())
