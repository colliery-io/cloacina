---
id: packaged-workflow-authoring-dx
level: initiative
title: "Packaged-workflow authoring DX — scaffold, one-command pack (Rust+Python), author-time validation, canonical format"
short_code: "CLOACI-I-0119"
created_at: 2026-06-14T15:06:03.467892+00:00
updated_at: 2026-06-14T15:06:03.467892+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: packaged-workflow-authoring-dx
---

# Packaged-workflow authoring DX — scaffold, one-command pack (Rust+Python), author-time validation, canonical format Initiative

## Context **[REQUIRED]**

Packaged workflows (`.cloacina`) are *the* deployment unit, but authoring one is a
sharp edge — there is **no scaffold, no Python `pack`, and no author-time
validation**, so users hand-roll everything and hit silent/late failures. The
whole gap was hit first-hand building the CLOACI-I-0117 UI demo (it spun out
T-0663/0665/0666/0669). Surveyed state (2026-06-14):

- `cloacinactl package` has `build / pack / publish / upload / list / inspect /
  delete` — **no `new`/`init`/`validate`**.
- `package pack` is **Rust-only** (`build.rs` hard-requires `Cargo.toml`, runs
  cargo, delegates to fidius-pack). **Python = hand-tarred bzip2** (T-0665, no
  code yet).
- `package.toml [metadata]` is a closed schema (`CloacinaMetadata`,
  `#[serde(deny_unknown_fields)]`,
  `cloacina-workflow-plugin/src/types.rs:295`): accepts `workflow_name`,
  `graph_name`, `language`(req), `description`, `author`, `requires_python`,
  `entry_module`, `reaction_mode`, `input_strategy`, `accumulators`. **Rejects**
  `package_type` and `[[metadata.triggers]]` (which still appear in old
  docs/examples).
- Layouts you must just-know: Rust = `Cargo.toml`+`package.toml`+`src/lib.rs`
  (+`build.rs`); Python = `package.toml` + module under **`workflow/<mod>/`** or
  load fails with `Missing workflow source directory`
  (`cloacina-python/src/package_loader.rs:179`).
- **Docs ≠ examples ≠ server** for Python: the published Python packaging how-to
  uses a top-level module + `manifest.json` → server **rejects**; the committed
  `examples/.../python-packaged-graph` has the broken top-level layout.
- Failures surface **late** (upload/load), not at author time.

Net: too much tribal knowledge; the packaged-workflow story is hard to adopt.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- From nothing to a valid, buildable, uploadable package in **one command**, for
  both Rust and Python, without knowing internal layout rules.
- `package pack` works for **both** languages (subsumes T-0665).
- **Author-time** validation catches the known footguns with actionable messages.
- **One canonical format**: server ↔ examples ↔ docs agree; a "create your first
  package" how-to that just uses `package new`.

**Non-Goals:**
- No change to the runtime package format / FFI ABI itself — this is tooling +
  docs over the *existing* accepted format.
- Not a GUI/UI authoring experience (CLI-first; the web UI already covers
  upload/inspect).
- Not a registry/marketplace for sharing packages.

## Use Cases

### UC-1: Bootstrap a new package
- **Actor**: a developer new to packaged workflows.
- **Scenario**: `cloacinactl package new my_wf --lang rust --kind workflow` →
  `cd my_wf` → edit the generated task → `cloacinactl package publish .`.
- **Outcome**: builds, uploads, registers, runs — no layout/manifest knowledge.

### UC-2: Author a Python workflow
- **Actor**: a Python user.
- **Scenario**: `cloacinactl package new etl --lang python` → scaffold puts the
  module under `workflow/etl/` with a valid `package.toml` → `package pack`
  produces a server-accepted archive (no hand-tar).
- **Outcome**: the package the docs tell you to make is the one the server accepts.

### UC-3: Catch mistakes before upload
- **Actor**: anyone editing a package by hand.
- **Scenario**: `cloacinactl package validate .` flags wrong module location,
  bogus `[metadata]` keys (`package_type`), a cron trigger listed in
  `#[workflow(triggers=[])]`, missing `entry_module`, etc.
- **Outcome**: actionable author-time error, not a late upload/load failure.

## Detailed Design **[REQUIRED]**

**`package new <name> --lang rust|python [--kind workflow|graph|cron]`** — emit a
correct, buildable skeleton from embedded templates:
- Rust: `Cargo.toml` (depending on **published** cloacina crate versions, NOT the
  fixtures' `__WORKSPACE__` path-deps), `build.rs` (`cloacina_build::configure()`),
  `package.toml` (valid `[metadata]`), `src/lib.rs` with a minimal commented
  `#[workflow]` / `#[computation_graph]` (split reactor form) / `#[trigger(cron)]`.
- Python: `package.toml` (`language="python"`, `entry_module="<name>.tasks"`),
  `workflow/<name>/__init__.py` + `tasks.py` with `@cloaca.task` examples.

**`package pack` for both languages** (T-0665): branch on `[metadata].language` —
Rust keeps build+fidius-pack; Python validates layout (`workflow/<entry_module>/`
exists, `package.toml` parses) and archives the source tree (bzip2 tar) the way
the server accepts. Reconciler's `package.toml + workflow/` is canonical.

**`package validate`** — a lint reusing the real parsers: parse `package.toml`
through `CloacinaMetadata` (bogus keys fail exactly as the server would), check
the language-specific layout, and static-check the footguns
(cron-trigger-in-`#[workflow(triggers)]`, missing `entry_module` for Python,
missing `graph_name` for a CG, `__WORKSPACE__` left unrewritten). `pack` runs it.

**Format reconciliation + docs** — server ↔ examples ↔ docs agree: rewrite the
Python packaging how-to to `workflow/<mod>/` + `package.toml`, fix the broken
`python-packaged-graph` example, add a "create your first package" how-to on
`package new`.

## Alternatives Considered **[REQUIRED]**

- **`angreal init` templates** as the primary path — rejected: authoring shouldn't
  need a second tool; the verb belongs on `cloacinactl` next to `pack`/`publish`
  (could reuse angreal's template engine under the hood).
- **Leave packing to docs/copy-paste** — status quo; rejected (produces
  server-rejected packages).
- **Loosen the runtime format** (accept top-level Python modules / ignore unknown
  `[metadata]` keys) — rejected: `deny_unknown_fields` catches real typos; the fix
  is tooling + one documented format, not a laxer server.

## Implementation Plan **[REQUIRED]**

Four tasks (build in order; T2 can land first as the highest-value standalone slice):

1. **T1 — `package new` scaffold**: embedded templates for rust|python ×
   workflow|graph|cron; published-crate deps; `new → publish` verified e2e.
2. **T2 — `package pack` for Python** = **CLOACI-T-0665** (reparented): branch on
   language; canonical `workflow/` layout; no hand-tar.
3. **T3 — `package validate` footgun lint**: author-time checks via the real
   parsers; `pack` invokes it.
4. **T4 — format reconciliation + docs**: one canonical format; fix the Python
   how-to + the broken example; "first package" how-to on `package new`.

Closeout: **CLOACI-T-0670** (umbrella DX task) is superseded by this initiative —
archived. **CLOACI-T-0666** (compiler read `[package].language`, already fixed) is
closed under T4.

## Exit Criteria

- `package new` → `package publish` works e2e for: Rust workflow, Python workflow,
  computation graph, cron trigger.
- `package pack` handles Python; no hand-tarring anywhere.
- `package validate` rejects each known footgun with an actionable message.
- Docs ↔ examples ↔ server agree on one format; the "first package" how-to uses
  `package new`.
