"""The checker's rules and its bookkeeping (CLOACINA-T-1108).

A rule is a function from a snapshot to a verdict. The ledger keeps, for each
rule, whether it holds, since when, and for how long it has held. Nothing in
this module talks to a server: it is given snapshots and times.

Verdicts. A rule answers one of three things:

- holds: there is evidence, and it agrees with the promise;
- does not hold: there is evidence, and it disagrees;
- cannot judge: there is no evidence yet, or a source was away.

"Cannot judge" is not "holds". It does not break a streak, and its time does
not count as time held.

Named gaps. A rule with an owner task (`gap="CLOACINA-T-1084"`, or
`"unfiled"`) is a promise known not to hold. When it does not hold it shows as
a gap with its task and does not make the report red. When it holds, the
report says so: the gap may be closed, so check and remove the tag.

Disturbance windows. A window is a time in which something was broken on
purpose. A verdict of "does not hold" inside a window whose kind the rule
tolerates is shown as "disturbed" and does not break the streak. Inside any
other window it is a break; the report shows the window next to it.
"""

from __future__ import annotations

import json
import os
import re
from datetime import datetime, timezone

from . import promises

# The checker runs on the system Python, which can be 3.9: no `datetime.UTC`.
UTC = timezone.utc  # noqa: UP017

HOLDS = "holds"
BROKEN = "does not hold"
UNKNOWN = "cannot judge"

# What the report shows for a rule.
SHOW_HOLDS = "holds"
SHOW_BROKEN = "broken"
SHOW_GAP = "gap"
SHOW_GAP_HOLDS = "gap holds"
SHOW_DISTURBED = "disturbed"
SHOW_UNKNOWN = "cannot judge"

TASK_CODE = re.compile(r"^[A-Z][A-Z0-9]*-[A-Z]-\d+$")
MAX_TRANSITIONS = 20


class Missing(Exception):
    """A rule asked for a source that is not in the snapshot, or that failed.
    The engine turns it into "cannot judge"."""


class Verdict:
    def __init__(self, state, evidence, details=None):
        self.state = state
        self.evidence = evidence
        self.details = list(details or [])

    def as_dict(self):
        return {"state": self.state, "evidence": self.evidence, "details": self.details}


def holds(evidence, details=None):
    return Verdict(HOLDS, evidence, details)


def broken(evidence, details=None):
    return Verdict(BROKEN, evidence, details)


def unknown(evidence, details=None):
    return Verdict(UNKNOWN, evidence, details)


def parse_time(value):
    """A time as seconds since the epoch. Takes a number, or an RFC 3339
    string, or Postgres's `2026-11-27 16:23:00.123+00`. A time with no zone is
    read as UTC. Returns None for anything else."""
    if value is None or isinstance(value, bool):
        return None
    if isinstance(value, (int, float)):
        return float(value)
    text = str(value).strip()
    if not text:
        return None
    text = text.replace(" ", "T", 1)
    if text.endswith("Z"):
        text = text[:-1] + "+00:00"
    # `+00` -> `+00:00`
    if "T" in text and re.search(r"[+-]\d{2}$", text):
        text = text + ":00"
    # Python 3.9 reads fractions of exactly three or six digits only.
    match = re.match(r"^(.*T\d{2}:\d{2}:\d{2})\.(\d+)(.*)$", text)
    if match:
        text = f"{match.group(1)}.{(match.group(2) + '000000')[:6]}{match.group(3)}"
    try:
        parsed = datetime.fromisoformat(text)
    except ValueError:
        return None
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=UTC)
    return parsed.timestamp()


def format_time(epoch):
    if epoch is None:
        return None
    return datetime.fromtimestamp(float(epoch), tz=UTC).strftime("%Y-%m-%dT%H:%M:%SZ")


class Window:
    """One disturbance: something broken on purpose, on record."""

    def __init__(self, kind, start, end=None, settle_until=None, target="", ident=""):
        self.kind = kind
        self.start = start
        self.end = end
        self.settle_until = settle_until
        self.target = target
        self.ident = ident

    @classmethod
    def from_dict(cls, data):
        start = parse_time(data.get("start"))
        if not data.get("kind") or start is None:
            return None
        return cls(str(data["kind"]), start, parse_time(data.get("end")),
                   parse_time(data.get("settle_until")), str(data.get("target") or ""),
                   str(data.get("id") or ""))

    @property
    def closes(self):
        """The time the window stops excusing anything. None while it is open."""
        if self.end is None:
            return None
        return max(self.end, self.settle_until or self.end)

    def active_at(self, at):
        closes = self.closes
        return self.start <= at and (closes is None or at <= closes)

    def overlaps(self, begin, finish):
        closes = self.closes
        return self.start <= finish and (closes is None or closes >= begin)

    def as_dict(self):
        return {"id": self.ident, "kind": self.kind, "target": self.target,
                "start": format_time(self.start), "end": format_time(self.end),
                "settle_until": format_time(self.settle_until)}

    def label(self):
        target = f" of {self.target}" if self.target else ""
        end = format_time(self.closes) or "still open"
        return f"{self.kind}{target}, {format_time(self.start)} to {end}"


def read_windows(lines):
    """Windows from the lines of a disturbance record (one JSON object each).
    A line that cannot be read is skipped: the record is written by another
    program and may be half written."""
    found = []
    for line in lines:
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        try:
            window = Window.from_dict(json.loads(line))
        except (ValueError, AttributeError):
            window = None
        if window is not None:
            found.append(window)
    return found


class Snapshot:
    """What one pass saw. Rules read it through `api`, `db` and `windows`."""

    def __init__(self, data):
        self.data = data
        self.now = parse_time(data.get("taken_at")) or 0.0
        self.tenants = list((data.get("tenants") or {}).keys())
        self._windows = [w for w in (Window.from_dict(d) for d in data.get("disturbances") or [])
                         if w is not None]

    def _entry(self, scope, kind, name):
        if scope is None:
            holder = self.data.get("admin") or {}
        else:
            holder = (self.data.get("tenants") or {}).get(scope)
            if holder is None:
                raise Missing(f"tenant {scope} is not in the snapshot")
        entry = (holder.get(kind) or {}).get(name)
        where = scope or "the deployment"
        if entry is None:
            raise Missing(f"{kind} source {name!r} of {where} was not collected")
        if entry.get("error"):
            raise Missing(f"{kind} source {name!r} of {where} is away: {entry['error']}")
        return entry

    def api(self, tenant, name):
        """(status, body) of an API source of a tenant."""
        entry = self._entry(tenant, "api", name)
        return entry.get("status"), entry.get("body")

    def api_items(self, tenant, name):
        """The `items` of an API source that answered 200."""
        status, body = self.api(tenant, name)
        if status != 200 or not isinstance(body, dict):
            raise Missing(f"api source {name!r} of {tenant} answered {status}")
        return body.get("items") or []

    def db(self, tenant, name):
        """The rows of a database source of a tenant; `tenant=None` for a
        source of the whole deployment."""
        return self._entry(tenant, "db", name).get("rows") or []

    def windows(self, kind=None):
        return [w for w in self._windows if kind is None or w.kind == kind]

    def active_windows(self):
        return [w for w in self._windows if w.active_at(self.now)]


class Rule:
    def __init__(self, ident, promise, chapter, fn, gap=None, tolerates=(), about=""):
        self.id = ident
        self.promise = promise
        self.chapter = chapter
        self.fn = fn
        self.gap = gap
        self.tolerates = tuple(tolerates)
        self.about = about or (fn.__doc__ or "").strip().split("\n")[0]

    def judge(self, snapshot):
        """Run the rule. A source that is away, or a fault of the rule itself,
        is "cannot judge": the checker never stops for one rule."""
        try:
            verdict = self.fn(snapshot)
        except Missing as exc:
            return unknown(str(exc))
        except Exception as exc:
            verdict = unknown(f"the rule itself failed: {exc.__class__.__name__}: {exc}")
            verdict.rule_error = True
            return verdict
        if not isinstance(verdict, Verdict):
            verdict = unknown(f"the rule returned {type(verdict).__name__}, not a verdict")
            verdict.rule_error = True
        return verdict


class Registry:
    """The rules, and the sources they need. Each chapter's rules file adds
    its own; nothing here lists the chapters."""

    def __init__(self):
        self.rules = []
        self.api_needs = {}  # name -> path template, for each tenant
        self.db_needs = {}  # name -> SQL template, for each tenant
        self.admin_db_needs = {}  # name -> SQL, once for the deployment

    def rule(self, ident, promise, chapter, gap=None, tolerates=(), about=""):
        def decorate(fn):
            self.rules.append(Rule(ident, promise, chapter, fn, gap=gap,
                                   tolerates=tolerates, about=about))
            return fn

        return decorate

    def _need(self, table, name, value):
        if name in table and table[name] != value:
            raise ValueError(f"two different sources are named {name!r}")
        table[name] = value

    def need_api(self, name, path):
        """An API GET for each tenant. `{tenant}` in the path is replaced."""
        self._need(self.api_needs, name, path)

    def need_db(self, name, sql):
        """A query for each tenant. `{schema}` in the SQL is the tenant's schema."""
        self._need(self.db_needs, name, sql)

    def need_admin_db(self, name, sql):
        """A query that runs once, outside any tenant's schema."""
        self._need(self.admin_db_needs, name, sql)

    def problems(self):
        """What is wrong with the registered rules. Empty when sound."""
        found = []
        seen = set()
        for rule in self.rules:
            if rule.id in seen:
                found.append(f"two rules have the id {rule.id}")
            seen.add(rule.id)
            if rule.promise not in promises.PROMISES:
                found.append(f"rule {rule.id} judges an unknown promise: {rule.promise}")
            if rule.gap is not None and rule.gap != "unfiled" and not TASK_CODE.match(rule.gap):
                found.append(f"rule {rule.id}: gap {rule.gap!r} is not a task code or 'unfiled'")
            if not rule.about:
                found.append(f"rule {rule.id} does not say what it checks")
        return found


class Ledger:
    """For each rule: its state, since when, and for how long it has held.

    The ledger is a plain dictionary that is written to a file after each
    pass and read again at start, so "held since" survives a restart of the
    checker. Time the checker was not running is not counted as time held.
    """

    def __init__(self, path=None, max_gap_s=900):
        self.path = path
        self.max_gap_s = max_gap_s
        self.entries = {}
        if path and os.path.exists(path):
            with open(path, "r", encoding="utf-8") as handle:
                self.entries = json.load(handle).get("rules", {})

    def save(self):
        if not self.path:
            return
        folder = os.path.dirname(self.path)
        if folder:
            os.makedirs(folder, exist_ok=True)
        temporary = self.path + ".tmp"
        with open(temporary, "w", encoding="utf-8") as handle:
            json.dump({"rules": self.entries}, handle, indent=1, sort_keys=True)
        os.replace(temporary, self.path)

    @staticmethod
    def _new_entry(at):
        return {
            "state": None, "since": at, "first_judged_at": at, "last_judged_at": None,
            "held_since": None, "unjudged_in_streak": 0.0, "breaks": 0,
            "last_break_at": None, "last_recovered_at": None, "longest_held_seconds": 0.0,
            "passes": 0, "evidence": "", "details": [], "window": None, "rule_error": False,
            "transitions": [],
        }

    def record(self, rule, verdict, at, windows=()):
        """Record one verdict of one rule at time `at`. Returns the entry."""
        entry = self.entries.get(rule.id) or self._new_entry(at)
        self.entries[rule.id] = entry
        active = [w for w in windows if w.active_at(at)]
        tolerated = [w for w in active if w.kind in rule.tolerates]

        if verdict.state == HOLDS:
            state = SHOW_HOLDS
        elif verdict.state == BROKEN:
            state = SHOW_DISTURBED if tolerated else SHOW_BROKEN
        else:
            state = SHOW_UNKNOWN

        previous = entry["state"]
        last = entry["last_judged_at"]
        if last is not None and entry["held_since"] is not None:
            delta = max(0.0, at - last)
            # Time that was not judged as holding is not time held: the time
            # after a pass that could not judge, and the time the checker was
            # not running at all.
            if previous in (SHOW_UNKNOWN, SHOW_DISTURBED) or delta > self.max_gap_s:
                entry["unjudged_in_streak"] += delta

        if state == SHOW_HOLDS:
            if entry["held_since"] is None:
                entry["held_since"] = at
                entry["unjudged_in_streak"] = 0.0
            if previous == SHOW_BROKEN:
                entry["last_recovered_at"] = at
        elif state == SHOW_BROKEN:
            if entry["held_since"] is not None:
                held = max(0.0, at - entry["held_since"] - entry["unjudged_in_streak"])
                entry["longest_held_seconds"] = max(entry["longest_held_seconds"], held)
                entry["held_since"] = None
                entry["unjudged_in_streak"] = 0.0
            if previous != SHOW_BROKEN:
                entry["breaks"] += 1
                entry["last_break_at"] = at

        if state != previous:
            entry["transitions"].append({
                "at": at, "from": previous, "to": state, "evidence": verdict.evidence,
                "window": active[0].label() if active else None,
            })
            del entry["transitions"][:-MAX_TRANSITIONS]
            entry["since"] = at

        entry["state"] = state
        entry["last_judged_at"] = at
        entry["passes"] += 1
        entry["evidence"] = verdict.evidence
        entry["details"] = verdict.details[:50]
        entry["rule_error"] = bool(getattr(verdict, "rule_error", False))
        entry["window"] = (tolerated or active)[0].label() if (tolerated or active) else None
        held_now = self.held_seconds(rule.id, at)
        entry["longest_held_seconds"] = max(entry["longest_held_seconds"], held_now)
        return entry

    def held_seconds(self, rule_id, at):
        """How long the rule has held in its current streak, without the time
        it could not be judged. Zero when it does not hold now."""
        entry = self.entries.get(rule_id)
        if not entry or entry["held_since"] is None or entry["state"] == SHOW_BROKEN:
            return 0.0
        return max(0.0, at - entry["held_since"] - entry["unjudged_in_streak"])


def shown(rule, entry):
    """What the report shows for a rule: its state, with the gap rule on top."""
    state = entry["state"]
    if rule.gap is not None:
        if state == SHOW_BROKEN:
            return SHOW_GAP
        if state == SHOW_HOLDS:
            return SHOW_GAP_HOLDS
    return state


def judge_all(registry, ledger, snapshot):
    """One pass: every rule against one snapshot. Returns [(rule, entry)]."""
    results = []
    windows = snapshot.windows()
    for rule in registry.rules:
        verdict = rule.judge(snapshot)
        results.append((rule, ledger.record(rule, verdict, snapshot.now, windows)))
    return results


def is_red(results):
    """True when a promise with no owner task does not hold."""
    return any(shown(rule, entry) == SHOW_BROKEN for rule, entry in results)
