"""The chapters of the behaviour suite, found by their files (CLOACINA-T-1107).

A chapter is a row of the suite's promises: publishing, schedules, tenants and
so on. It owns three things, and edits no file of another chapter:

- a steps file here, `<chapter>_steps.py`, with `CHAPTER`, `FEATURES` (its
  feature files, from the root of the repository) and `steps` (a
  `gherkin.StepRegistry`);
- its feature files, under `features/<chapter>/`;
- a rules file for the checker, `suite/rules/<chapter>.py`.

Nothing lists the chapters. This module finds the steps files by their name,
and the checker finds the rules files the same way.

`dry_run` is what `angreal test bdd standing --dry-run` prints: every feature
parsed, every step resolved to exactly one definition, the rules loaded, the
workload description checked. It needs no cluster and does not import angreal.
"""

from __future__ import annotations

import importlib
from pathlib import Path

from . import gherkin
from .suite import promises, rules, workload

HERE = Path(__file__).resolve().parent
PROJECT_ROOT = HERE.parent.parent.parent


class Chapter:
    def __init__(self, name, features, steps, steps_file):
        self.name = name
        self.features = tuple(features)
        self.steps = steps
        self.steps_file = steps_file

    @property
    def has_rules(self):
        return self.name in rules.chapter_modules()


def discover():
    """Every chapter, in the order of its steps file's name. A steps file
    with a part missing raises: that is a fault to fix, not to skip."""
    found = []
    for path in sorted(HERE.glob("*_steps.py")):
        module = importlib.import_module(f".{path.stem}", package=__package__)
        missing = [name for name in ("CHAPTER", "FEATURES", "steps") if not hasattr(module, name)]
        if missing:
            raise gherkin.GherkinError(f"{path.name} has no {', '.join(missing)}")
        found.append(Chapter(module.CHAPTER, module.FEATURES, module.steps, path.name))
    return found


def find(name):
    for chapter in discover():
        if chapter.name == name:
            return chapter
    known = ", ".join(chapter.name for chapter in discover())
    raise gherkin.GherkinError(f"no chapter is named {name!r}; the chapters are: {known}")


def load_features(chapter, root=PROJECT_ROOT):
    """The parsed features of a chapter."""
    parsed = []
    for rel in chapter.features:
        path = Path(root) / rel
        parsed.append(gherkin.parse(path.read_text(), path=rel))
    return parsed


def dry_run(root=PROJECT_ROOT):
    """Check everything that can be checked with no cluster.

    Returns (lines to print, problems). An empty list of problems is a pass.
    """
    root = Path(root)
    lines = []
    problems = []
    claimed = set()
    names = set()

    try:
        chapters = discover()
    except gherkin.GherkinError as exc:
        return lines, [str(exc)]

    for chapter in chapters:
        if chapter.name in names:
            problems.append(f"two steps files are for the chapter {chapter.name!r}")
        names.add(chapter.name)
        rules_note = "rules file: yes" if chapter.has_rules else "rules file: none yet"
        lines.append(f"chapter {chapter.name}  ({chapter.steps_file}; {rules_note})")
        if not chapter.features:
            problems.append(f"{chapter.steps_file}: FEATURES is empty")
        used = set()
        for rel in chapter.features:
            claimed.add(rel)
            path = root / rel
            if not path.exists():
                problems.append(f"{chapter.steps_file}: feature file not found: {rel}")
                continue
            try:
                feature = gherkin.parse(path.read_text(), path=rel)
            except gherkin.GherkinError as exc:
                problems.append(str(exc))
                continue
            found, _notes = gherkin.check(feature, chapter.steps)
            problems.extend(found)
            total = len(feature.background) + sum(len(s.steps) for s in feature.scenarios)
            lines.append(f"  feature {rel}: {len(feature.scenarios)} scenarios, {total} steps")
            for number, scenario in enumerate(feature.scenarios, start=1):
                mark = f"  [gap {', '.join(scenario.gap_tasks)}]" if scenario.is_gap else ""
                lines.append(f"    {number}. {scenario.name}{mark}")
            for step in list(feature.background) + [s for sc in feature.scenarios for s in sc.steps]:
                for fn, _groups in chapter.steps.find(step):
                    used.add(fn)
        for _kind, pattern, fn in chapter.steps.definitions:
            if fn not in used:
                lines.append(f"  note: step definition used by no feature of the chapter: "
                             f"{fn.__name__} ({pattern.pattern})")

    features_dir = root / "features"
    if features_dir.exists():
        for path in sorted(features_dir.rglob("*.feature")):
            rel = path.relative_to(root).as_posix()
            if rel not in claimed:
                problems.append(f"{rel}: no steps file names this feature in FEATURES")

    # The checker's side: the promises table, the workload, the rules.
    problems.extend(promises.problems())
    problems.extend(workload.problems())
    try:
        registry = rules.load()
        problems.extend(registry.problems())
    except Exception as exc:
        registry = None
        problems.append(f"the checker's rules do not load: {exc.__class__.__name__}: {exc}")
    for module in rules.chapter_modules():
        if module not in names:
            problems.append(f"suite/rules/{module}.py has no steps file `{module}_steps.py`")
    for package in workload.PACKAGES:
        if not (root / "examples" / "fixtures" / package.fixture / "package.toml").exists():
            problems.append(f"the workload names a fixture that is not there: {package.fixture}")

    if registry is not None:
        judged = {}
        for rule in registry.rules:
            judged.setdefault(rule.promise, []).append(rule)
        lines.append("")
        lines.append("promises (rules of the checker / work that gives evidence):")
        for row in promises.ROWS:
            lines.append(f"  {row}  (chapter task {promises.CHAPTER_TASK[row]})")
            for promise_id, (promise_row, sentence) in promises.PROMISES.items():
                if promise_row != row:
                    continue
                count = len(judged.get(promise_id, []))
                gaps = sorted({r.gap for r in judged.get(promise_id, []) if r.gap})
                rules_text = f"{count} rule{'s' if count != 1 else ''}" if count else "no rule yet"
                if gaps:
                    rules_text += f", gap {', '.join(gaps)}"
                work = ", ".join(workload.EVIDENCE.get(promise_id) or []) or \
                    workload.WITHOUT_EVIDENCE.get(promise_id, "none")
                lines.append(f"    {sentence}\n      {rules_text}; evidence: {work}")
    return lines, problems
