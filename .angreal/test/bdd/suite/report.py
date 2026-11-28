"""The checker's report: one JSON file and one page (CLOACINA-T-1108).

The report lists every promise of the table, row by row: whether it holds,
since when and for how long, the evidence of the last pass, and the rules
that judged it. A promise with no rule yet is in the list too, with the task
of its chapter. A promise that does not hold and has an owner task shows as a
gap with that task.

`build` makes the dictionary, `write` puts `report.json` and `index.html` in
a folder, `serve` serves that folder on localhost.
"""

from __future__ import annotations

import functools
import html
import http.server
import json
import os
import threading

from . import engine, promises, workload

NO_RULE = "no rule yet"

# The worst state of a promise's rules is the state of the promise.
SEVERITY = {
    engine.SHOW_BROKEN: 6,
    engine.SHOW_GAP: 5,
    engine.SHOW_DISTURBED: 4,
    engine.SHOW_UNKNOWN: 3,
    engine.SHOW_GAP_HOLDS: 2,
    engine.SHOW_HOLDS: 1,
}

LABEL = {
    engine.SHOW_HOLDS: "holds",
    engine.SHOW_BROKEN: "does not hold",
    engine.SHOW_GAP: "known gap",
    engine.SHOW_GAP_HOLDS: "known gap, holds now",
    engine.SHOW_DISTURBED: "disturbed",
    engine.SHOW_UNKNOWN: "cannot judge",
    NO_RULE: "no rule yet",
}


def duration(seconds):
    """A length of time in words: `3 d 4 h`, `12 min`."""
    seconds = int(seconds or 0)
    if seconds < 60:
        return f"{seconds} s"
    minutes, _ = divmod(seconds, 60)
    hours, minutes = divmod(minutes, 60)
    days, hours = divmod(hours, 24)
    if days:
        return f"{days} d {hours} h"
    if hours:
        return f"{hours} h {minutes} min"
    return f"{minutes} min"


def _rule_view(rule, entry, ledger, at):
    held = ledger.held_seconds(rule.id, at)
    return {
        "id": rule.id,
        "about": rule.about,
        "chapter": rule.chapter,
        "status": engine.shown(rule, entry),
        "gap": rule.gap,
        "tolerates": list(rule.tolerates),
        "since": engine.format_time(entry["since"]),
        "held_since": engine.format_time(entry["held_since"]),
        "held_for_seconds": int(held),
        "held_for": duration(held) if entry["held_since"] is not None else None,
        "longest_held": duration(entry["longest_held_seconds"]),
        "breaks": entry["breaks"],
        "last_break_at": engine.format_time(entry["last_break_at"]),
        "last_recovered_at": engine.format_time(entry["last_recovered_at"]),
        "passes": entry["passes"],
        "evidence": entry["evidence"],
        "details": entry["details"],
        "window": entry["window"],
        "rule_error": entry["rule_error"],
        "transitions": [
            {"at": engine.format_time(t["at"]), "from": t["from"], "to": t["to"],
             "evidence": t["evidence"], "window": t.get("window")}
            for t in entry["transitions"][-5:]
        ],
    }


def _disturbance_view(window, at, states):
    """One disturbance, with the promises it tests and their state now."""
    view = dict(window.as_dict(), active=window.active_at(at))
    view["tests"] = [
        {"id": promise_id, "sentence": promises.sentence_of(promise_id),
         "label": states.get(promise_id, LABEL[NO_RULE])}
        for promise_id in promises.DISTURBANCE_TESTS.get(window.kind, ())
    ]
    for field in ("mode", "outcome"):
        if window.extra.get(field):
            view[field] = str(window.extra[field])
    return view


def build(registry, ledger, results, snapshot, interval_s=60):
    """The report of one pass, as a dictionary."""
    at = snapshot.now
    by_promise = {}
    for rule, entry in results:
        by_promise.setdefault(rule.promise, []).append(_rule_view(rule, entry, ledger, at))

    counts = {label: 0 for label in list(SEVERITY) + [NO_RULE]}
    rows = []
    for row in promises.ROWS:
        listed = []
        for promise_id, (promise_row, sentence) in promises.PROMISES.items():
            if promise_row != row:
                continue
            rules = by_promise.get(promise_id, [])
            if rules:
                status = max((r["status"] for r in rules), key=lambda s: SEVERITY[s])
            else:
                status = NO_RULE
            counts[status] += 1
            holding = rules and all(r["held_since"] for r in rules) and status in (
                engine.SHOW_HOLDS, engine.SHOW_GAP_HOLDS)
            held_for = min(r["held_for_seconds"] for r in rules) if holding else 0
            listed.append({
                "id": promise_id,
                "sentence": sentence,
                "status": status,
                "label": LABEL[status],
                "held_since": max(r["held_since"] for r in rules) if holding else None,
                "held_for_seconds": held_for,
                "held_for": duration(held_for) if holding else None,
                "gap_tasks": sorted({r["gap"] for r in rules if r["gap"]}),
                "evidence_from": workload.EVIDENCE.get(promise_id, []),
                "without_evidence": workload.WITHOUT_EVIDENCE.get(promise_id),
                "rules": rules,
            })
        rows.append({"row": row, "chapter_task": promises.CHAPTER_TASK.get(row), "promises": listed})

    windows = snapshot.windows()
    recent = [w for w in windows if w.overlaps(at - 86400, at)]
    states = {item["id"]: item["label"] for row in rows for item in row["promises"]}
    return {
        "title": "Cloacina behaviour suite",
        "generated_at": engine.format_time(at),
        "generated_at_epoch": at,
        "interval_seconds": interval_s,
        "tenants": snapshot.tenants,
        "red": engine.is_red(results),
        "summary": counts,
        "disturbances": [_disturbance_view(w, at, states) for w in recent],
        "rows": rows,
    }


# --- the page -----------------------------------------------------------------

CSS = """
:root { color-scheme: light dark; --bg:#fbfbf9; --fg:#1d1d1b; --muted:#66665f; --line:#dcdcd4;
  --card:#ffffff; --ok:#1d6b3a; --bad:#a3241b; --gap:#8a5a00; --wait:#4d5a73; }
@media (prefers-color-scheme: dark) { :root { --bg:#161614; --fg:#ecece6; --muted:#a3a39a;
  --line:#35352f; --card:#1e1e1b; --ok:#6fcf8e; --bad:#ff8a7e; --gap:#e6b655; --wait:#9fb0d0; } }
* { box-sizing: border-box; }
body { margin:0; background:var(--bg); color:var(--fg);
  font:15px/1.5 system-ui,-apple-system,"Segoe UI",sans-serif; }
main { max-width:1100px; margin:0 auto; padding:24px 20px 60px; }
h1 { font-size:22px; margin:0 0 4px; }
h2 { font-size:16px; margin:32px 0 8px; display:flex; gap:12px; align-items:baseline; }
h2 small, .muted { color:var(--muted); font-weight:400; font-size:13px; }
.bar { display:flex; flex-wrap:wrap; gap:8px 18px; margin:12px 0 0; padding:12px 14px;
  background:var(--card); border:1px solid var(--line); border-radius:8px; }
.bar b { font-variant-numeric:tabular-nums; }
#stale { display:none; margin-top:12px; padding:10px 14px; border:1px solid var(--bad);
  border-radius:8px; color:var(--bad); }
.promise { background:var(--card); border:1px solid var(--line); border-radius:8px;
  padding:12px 14px; margin:8px 0; }
.promise header { display:flex; flex-wrap:wrap; gap:6px 14px; align-items:baseline; }
.sentence { font-weight:600; flex:1 1 380px; }
.state { font-weight:600; white-space:nowrap; }
.state::before { content:"\\25CF"; margin-right:6px; }
.s-holds { color:var(--ok); } .s-broken { color:var(--bad); }
.s-gap, .s-gap-holds, .s-disturbed { color:var(--gap); }
.s-cannot-judge, .s-no-rule-yet { color:var(--wait); }
.held { font-variant-numeric:tabular-nums; white-space:nowrap; }
.rule { margin:8px 0 0; padding:8px 0 0; border-top:1px solid var(--line); font-size:14px; }
.rule .about { color:var(--muted); }
.evidence { overflow-wrap:anywhere; }
details { margin-top:4px; } summary { cursor:pointer; color:var(--muted); }
ul { margin:4px 0 0; padding-left:18px; } li { overflow-wrap:anywhere; }
code { font:13px ui-monospace,SFMono-Regular,Menlo,monospace; }
.loud { margin-top:6px; padding:6px 10px; border:1px solid var(--gap); border-radius:6px; }
table { border-collapse:collapse; width:100%; font-size:14px; }
td, th { text-align:left; padding:4px 10px 4px 0; border-bottom:1px solid var(--line); }
"""

STALE_SCRIPT = """
(function () {
  var made = %(made)f, every = %(every)d;
  function check() {
    var age = Date.now() / 1000 - made;
    if (age > Math.max(3 * every, 180)) {
      var box = document.getElementById('stale');
      box.style.display = 'block';
      box.textContent = 'This report is stale: the last pass of the checker was '
        + Math.round(age / 60) + ' minutes ago. The states below are from that pass.';
    }
  }
  check(); setInterval(check, 30000);
})();
"""


def _class(status):
    return "s-" + status.replace(" ", "-")


def _esc(value):
    return html.escape(str(value), quote=True)


def _rule_html(rule):
    parts = ['<div class="rule">']
    parts.append(f'<div><span class="state {_class(rule["status"])}">{_esc(LABEL[rule["status"]])}</span> '
                 f'<code>{_esc(rule["id"])}</code> <span class="about">{_esc(rule["about"])}</span></div>')
    parts.append(f'<div class="evidence">{_esc(rule["evidence"])}</div>')
    facts = [f'in this state since {_esc(rule["since"])}']
    if rule["held_for"]:
        facts.append(f'held for {_esc(rule["held_for"])}')
    facts.append(f'longest {_esc(rule["longest_held"])}')
    facts.append(f'breaks {rule["breaks"]}')
    if rule["last_break_at"]:
        facts.append(f'last break {_esc(rule["last_break_at"])}')
    if rule["last_recovered_at"]:
        facts.append(f'last recovery {_esc(rule["last_recovered_at"])}')
    parts.append(f'<div class="muted">{"; ".join(facts)}</div>')
    if rule["window"]:
        parts.append(f'<div class="muted">disturbance window: {_esc(rule["window"])}</div>')
    if rule["gap"] and rule["status"] == engine.SHOW_GAP:
        parts.append(f'<div class="muted">owner task: {_esc(rule["gap"])}</div>')
    if rule["status"] == engine.SHOW_GAP_HOLDS:
        parts.append(f'<div class="loud">This rule is marked as a known gap ({_esc(rule["gap"])}) '
                     'and it holds. The gap may be closed: check, then remove the mark.</div>')
    if rule["rule_error"]:
        parts.append('<div class="loud">The rule itself failed on this pass. Fix the rule.</div>')
    if rule["details"]:
        items = "".join(f"<li>{_esc(item)}</li>" for item in rule["details"])
        parts.append(f'<details><summary>{len(rule["details"])} details</summary><ul>{items}</ul></details>')
    if rule["transitions"]:
        items = "".join(
            f'<li>{_esc(t["at"])}: {_esc(t["from"] or "first pass")} to {_esc(t["to"])}. '
            f'{_esc(t["evidence"])}'
            + (f' (window: {_esc(t["window"])})' if t.get("window") else "") + "</li>"
            for t in reversed(rule["transitions"]))
        parts.append(f"<details><summary>last changes</summary><ul>{items}</ul></details>")
    parts.append("</div>")
    return "".join(parts)


def _promise_html(promise):
    parts = ['<section class="promise"><header>']
    parts.append(f'<span class="sentence">{_esc(promise["sentence"])}</span>')
    parts.append(f'<span class="state {_class(promise["status"])}">{_esc(promise["label"])}</span>')
    if promise["held_for"]:
        parts.append(f'<span class="held">for {_esc(promise["held_for"])}, '
                     f'since {_esc(promise["held_since"])}</span>')
    if promise["gap_tasks"] and promise["status"] in (engine.SHOW_GAP, engine.SHOW_GAP_HOLDS):
        parts.append(f'<span class="muted">task {_esc(", ".join(promise["gap_tasks"]))}</span>')
    parts.append("</header>")
    if promise["evidence_from"]:
        parts.append('<div class="muted">work that gives evidence: '
                     + ", ".join(f"<code>{_esc(w)}</code>" for w in promise["evidence_from"]) + "</div>")
    elif promise["without_evidence"]:
        parts.append(f'<div class="muted">{_esc(promise["without_evidence"])}</div>')
    for rule in promise["rules"]:
        parts.append(_rule_html(rule))
    parts.append("</section>")
    return "".join(parts)


def render(report):
    """The page of a report, as one HTML document with no outside files."""
    summary = report["summary"]
    order = [engine.SHOW_HOLDS, engine.SHOW_BROKEN, engine.SHOW_GAP, engine.SHOW_GAP_HOLDS,
             engine.SHOW_DISTURBED, engine.SHOW_UNKNOWN, NO_RULE]
    bar = "".join(
        f'<span class="state {_class(status)}">{_esc(LABEL[status])} <b>{summary.get(status, 0)}</b></span>'
        for status in order)
    body = [f"<h1>{_esc(report['title'])}</h1>"]
    body.append(f'<div class="muted">Promises of the system, judged against a standing workload. '
                f'Last pass {_esc(report["generated_at"])}, one pass every '
                f'{report["interval_seconds"]} s. Tenants: {_esc(", ".join(report["tenants"]) or "none")}.</div>')
    body.append(f'<div class="bar">{bar}</div>')
    body.append('<div id="stale" role="alert"></div>')
    if report["disturbances"]:
        lines = "".join(
            f'<tr><td>{_esc(w["kind"])}{_esc(" (" + w["mode"] + ")") if w.get("mode") else ""}</td>'
            f'<td>{_esc(w["target"])}</td><td>{_esc(w["start"])}</td>'
            f'<td>{_esc(w["end"] or "open")}</td><td>{"active" if w["active"] else "over"}'
            f'{_esc(", " + w["outcome"]) if w.get("outcome") else ""}</td>'
            "<td>" + "<br>".join(f'{_esc(t["sentence"])} <span class="muted">{_esc(t["label"])}</span>'
                                 for t in w.get("tests", [])) + "</td></tr>"
            for w in reversed(report["disturbances"]))
        body.append("<h2>Disturbances <small>the last 24 hours</small></h2>"
                    "<table><tr><th>kind</th><th>target</th><th>start</th><th>end</th><th></th>"
                    "<th>promises it tests</th></tr>"
                    f"{lines}</table>")
    for row in report["rows"]:
        if not row["promises"]:
            continue
        body.append(f'<h2>{_esc(row["row"])} <small>chapter task {_esc(row["chapter_task"])}</small></h2>')
        for promise in row["promises"]:
            body.append(_promise_html(promise))
    script = STALE_SCRIPT % {"made": report["generated_at_epoch"], "every": report["interval_seconds"]}
    refresh = max(30, int(report["interval_seconds"]))
    return (
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">"
        f'<meta http-equiv="refresh" content="{refresh}">'
        '<meta name="viewport" content="width=device-width, initial-scale=1">'
        f"<title>{_esc(report['title'])}</title><style>{CSS}</style></head>"
        f"<body><main>{''.join(body)}</main><script>{script}</script></body></html>\n"
    )


def write(folder, report):
    """Write `report.json` and `index.html` into `folder`, each in one move."""
    os.makedirs(folder, exist_ok=True)
    for name, text in (("report.json", json.dumps(report, indent=1, sort_keys=True) + "\n"),
                       ("index.html", render(report))):
        temporary = os.path.join(folder, name + ".tmp")
        with open(temporary, "w", encoding="utf-8") as handle:
            handle.write(text)
        os.replace(temporary, os.path.join(folder, name))


class _Quiet(http.server.SimpleHTTPRequestHandler):
    def log_message(self, format, *args):
        pass


def serve(folder, port, host="127.0.0.1"):
    """Serve `folder` on localhost in a background thread. Returns the server."""
    handler = functools.partial(_Quiet, directory=folder)
    server = http.server.ThreadingHTTPServer((host, port), handler)
    thread = threading.Thread(target=server.serve_forever, name="report-server", daemon=True)
    thread.start()
    return server
