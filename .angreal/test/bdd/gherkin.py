"""A small Gherkin parser and runner for the behaviour specs driven from angreal.

The angreal tasks run in angreal's own Python and have no dependency file, so
`behave` and `pytest-bdd` are not available here. This module reads the subset
of Gherkin the features use and runs it against step definitions.

Supported: `Feature:`, free description text, `Background:`, `Scenario:`,
tags (`@name`), comments (`#`), and the step keywords `Given`, `When`, `Then`,
`And`, `But`. Not supported, and refused with the line number: `Scenario
Outline`, `Examples`, data tables, doc strings, `Rule`.

Known gaps. A scenario tagged `@gap` plus a task code (`@CLOACINA-T-1022`)
still runs. Its failure is reported as "known gap" and does not fail the run.
Its pass is reported as "unexpected pass": the gap may be closed.

Hard and soft steps. A failed `Given` or `When` stops the scenario: there is
nothing to observe after it. A failed `Then` is recorded and the scenario goes
on, so one run shows every expectation that does not hold.

This file needs the standard library only and no cluster.
"""

from __future__ import annotations

import re
import traceback

PRIMARY = ("Given", "When", "Then")
KEYWORDS = PRIMARY + ("And", "But")
UNSUPPORTED = ("Scenario Outline:", "Scenario Template:", "Examples:", "Rule:", '"""', "```", "|")
TASK_CODE = re.compile(r"^@[A-Z][A-Z0-9]*-[A-Z]-\d+$")

PASS = "pass"
KNOWN_GAP = "known gap"
NEW_FAILURE = "new failure"
UNEXPECTED_PASS = "unexpected pass"


class GherkinError(Exception):
    pass


class Step:
    def __init__(self, keyword, kind, text, line):
        self.keyword = keyword  # as written: Given, When, Then, And, But
        self.kind = kind  # Given, When or Then: And and But take the kind before them
        self.text = text
        self.line = line

    def __str__(self):
        return f"{self.keyword} {self.text}"


class Scenario:
    def __init__(self, name, tags, line):
        self.name = name
        self.tags = tags
        self.line = line
        self.steps = []

    @property
    def is_gap(self):
        return "@gap" in self.tags

    @property
    def gap_tasks(self):
        return [t[1:] for t in self.tags if TASK_CODE.match(t)]


class Feature:
    def __init__(self, name, tags, path):
        self.name = name
        self.tags = tags
        self.path = path
        self.description = []
        self.background = []
        self.scenarios = []


def parse(text, path="<feature>"):
    """Parse the text of one feature file."""
    feature = None
    target = None  # the list that takes the next step
    scenario = None
    pending_tags = []
    last_kind = None

    for number, raw in enumerate(text.splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        for marker in UNSUPPORTED:
            if line.startswith(marker):
                raise GherkinError(f"{path}:{number}: '{marker}' is not supported by this runner")
        if line.startswith("@"):
            pending_tags.extend(line.split())
            continue
        if line.startswith("Feature:"):
            if feature is not None:
                raise GherkinError(f"{path}:{number}: a second Feature in one file")
            feature = Feature(line[len("Feature:"):].strip(), pending_tags, path)
            pending_tags = []
            continue
        if feature is None:
            raise GherkinError(f"{path}:{number}: text before the Feature line")
        if line.startswith("Background:"):
            if feature.scenarios:
                raise GherkinError(f"{path}:{number}: Background must come before the scenarios")
            target = feature.background
            scenario = None
            last_kind = None
            continue
        if line.startswith("Scenario:") or line.startswith("Example:"):
            name = line.split(":", 1)[1].strip()
            scenario = Scenario(name, feature.tags + pending_tags, number)
            pending_tags = []
            feature.scenarios.append(scenario)
            target = scenario.steps
            last_kind = None
            continue
        keyword = line.split(" ", 1)[0]
        if keyword in KEYWORDS:
            if target is None:
                raise GherkinError(f"{path}:{number}: a step outside a Background or Scenario")
            step_text = line[len(keyword):].strip()
            if keyword in PRIMARY:
                last_kind = keyword
            elif last_kind is None:
                raise GherkinError(f"{path}:{number}: '{keyword}' with no Given, When or Then before it")
            target.append(Step(keyword, last_kind, step_text, number))
            continue
        if target is None:
            feature.description.append(line)
            continue
        raise GherkinError(f"{path}:{number}: not a step: {line!r}")

    if feature is None:
        raise GherkinError(f"{path}: no Feature line")
    if pending_tags:
        raise GherkinError(f"{path}: tags {pending_tags} with no scenario after them")
    return feature


class StepRegistry:
    """Step definitions: a pattern (a regular expression that must match the
    whole step text) and the function to call with the world and the groups."""

    def __init__(self):
        self.definitions = []  # (kind, compiled pattern, function)

    def _add(self, kind, pattern):
        def decorate(fn):
            self.definitions.append((kind, re.compile(pattern), fn))
            return fn

        return decorate

    def given(self, pattern):
        return self._add("Given", pattern)

    def when(self, pattern):
        return self._add("When", pattern)

    def then(self, pattern):
        return self._add("Then", pattern)

    def find(self, step):
        """Every definition that matches the step: (function, groups)."""
        found = []
        for kind, pattern, fn in self.definitions:
            if kind != step.kind:
                continue
            match = pattern.fullmatch(step.text)
            if match:
                found.append((fn, match.groups()))
        return found


def check(feature, registry):
    """The dry run: what is wrong with the feature and its step definitions.

    Returns (problems, notes). A problem fails the dry run. Needs no cluster.
    """
    problems = []
    notes = []
    used = set()
    names = set()

    def check_steps(steps, where):
        for step in steps:
            found = registry.find(step)
            if not found:
                problems.append(f"{feature.path}:{step.line}: no step definition for: {step} ({where})")
            elif len(found) > 1:
                names_ = ", ".join(fn.__name__ for fn, _ in found)
                problems.append(f"{feature.path}:{step.line}: {len(found)} definitions match: {step} ({names_})")
            else:
                used.add(found[0][0])

    check_steps(feature.background, "Background")
    for scenario in feature.scenarios:
        if scenario.name in names:
            problems.append(f"{feature.path}:{scenario.line}: two scenarios are named {scenario.name!r}")
        names.add(scenario.name)
        if not scenario.steps:
            problems.append(f"{feature.path}:{scenario.line}: scenario with no steps: {scenario.name}")
        if not any(step.kind == "Then" for step in scenario.steps):
            problems.append(f"{feature.path}:{scenario.line}: scenario with no Then: {scenario.name}")
        if scenario.is_gap and not scenario.gap_tasks:
            problems.append(
                f"{feature.path}:{scenario.line}: @gap with no task code (for example @CLOACINA-T-1022): {scenario.name}"
            )
        if scenario.gap_tasks and not scenario.is_gap:
            problems.append(f"{feature.path}:{scenario.line}: a task code tag with no @gap: {scenario.name}")
        check_steps(scenario.steps, scenario.name)

    for _kind, pattern, fn in registry.definitions:
        if fn not in used:
            notes.append(f"step definition not used by this feature: {fn.__name__} ({pattern.pattern})")
    return problems, notes


class ScenarioResult:
    def __init__(self, number, scenario):
        self.number = number
        self.scenario = scenario
        self.failures = []  # (step, message)
        self.facts = []  # observed facts, in the order the steps noted them
        self.reached_then = False
        self.skipped = []  # steps not run after a hard failure
        self.outcome = None

    def settle(self):
        failed = bool(self.failures)
        if self.scenario.is_gap:
            self.outcome = KNOWN_GAP if failed else UNEXPECTED_PASS
        else:
            self.outcome = NEW_FAILURE if failed else PASS
        return self.outcome


def _message(exc):
    text = str(exc).strip() or exc.__class__.__name__
    if not isinstance(exc, AssertionError):
        text = f"{exc.__class__.__name__}: {text}"
    return text


def run(feature, registry, world, only=None, log=print):
    """Run each scenario against `world`. Returns the list of ScenarioResult.

    `world` gets `begin_scenario(result)` and `end_scenario(result)` if it has
    them; `end_scenario` runs always, also after a failure. `only` is a set of
    scenario numbers (1-based) to run.
    """
    results = []
    for number, scenario in enumerate(feature.scenarios, start=1):
        if only and number not in only:
            continue
        result = ScenarioResult(number, scenario)
        results.append(result)
        tags = " ".join(scenario.tags)
        log(f"\n--- scenario {number}: {scenario.name} {tags}".rstrip())
        if hasattr(world, "begin_scenario"):
            world.begin_scenario(result)
        stopped = False
        try:
            for step in list(feature.background) + list(scenario.steps):
                if stopped:
                    result.skipped.append(step)
                    continue
                if step.kind == "Then":
                    result.reached_then = True
                found = registry.find(step)
                if len(found) != 1:
                    result.failures.append((step, f"{len(found)} step definitions match (run the dry run)"))
                    stopped = True
                    continue
                fn, groups = found[0]
                try:
                    fn(world, *groups)
                    log(f"    ok    {step}")
                except KeyboardInterrupt:
                    raise
                except Exception as exc:  # noqa: BLE001 - any failure of a step is a result
                    result.failures.append((step, _message(exc)))
                    log(f"    FAIL  {step}\n          {_message(exc)}")
                    if not isinstance(exc, AssertionError):
                        log("          " + traceback.format_exc().strip().replace("\n", "\n          "))
                    if step.kind != "Then":
                        stopped = True
        finally:
            if hasattr(world, "end_scenario"):
                try:
                    world.end_scenario(result)
                except Exception as exc:  # noqa: BLE001
                    result.failures.append((None, f"cleanup after the scenario failed: {_message(exc)}"))
        log(f"    => {result.settle()}")
    return results


def report(feature, results):
    """The list of rough spots, as text: one block for each scenario."""
    lines = [
        f"Rough spots: {feature.name}",
        f"Feature file: {feature.path}",
        "",
    ]
    counts = {PASS: 0, KNOWN_GAP: 0, NEW_FAILURE: 0, UNEXPECTED_PASS: 0}
    for result in results:
        counts[result.outcome] += 1
        scenario = result.scenario
        owner = f" ({', '.join(scenario.gap_tasks)})" if scenario.gap_tasks else ""
        lines.append(f"{result.number}. {scenario.name}")
        lines.append(f"   result: {result.outcome.upper()}{owner}")
        if result.outcome == UNEXPECTED_PASS:
            lines.append(
                f"   !!! This scenario is marked @gap{owner} and it PASSED. The gap may be closed: "
                "check, then remove the tags."
            )
        if result.failures and not result.reached_then:
            lines.append(
                "   note: the scenario stopped before its first Then. The expectation itself was not "
                "observed; read the failure as a problem of setup or of an earlier step."
            )
        for step, message in result.failures:
            where = f"{step} (line {step.line})" if step is not None else "after the scenario"
            lines.append(f"   failed: {where}")
            for part in message.splitlines():
                lines.append(f"           {part}")
        for step in result.skipped:
            lines.append(f"   not run: {step}")
        for fact in result.facts:
            lines.append(f"   observed: {fact}")
        lines.append("")
    lines.append(
        "Summary: "
        f"{counts[PASS]} pass, {counts[KNOWN_GAP]} known gap, "
        f"{counts[NEW_FAILURE]} new failure, {counts[UNEXPECTED_PASS]} unexpected pass "
        f"(of {len(results)} run, {len(feature.scenarios)} in the feature)"
    )
    if counts[UNEXPECTED_PASS]:
        lines.append("!!! A scenario marked as a known gap passed. See above.")
    return "\n".join(lines) + "\n"


def run_failed(results):
    """True when the run must fail: a scenario with no @gap tag failed."""
    return any(result.outcome == NEW_FAILURE for result in results)
