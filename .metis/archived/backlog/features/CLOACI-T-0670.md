---
id: close-the-package-authoring-gap
level: task
title: "Close the package-authoring gap — scaffold/bootstrap + one-command pack for Rust & Python (.cloacina DX)"
short_code: "CLOACI-T-0670"
created_at: 2026-06-13T12:12:36.952036+00:00
updated_at: 2026-06-13T12:12:36.952036+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Close the package-authoring gap — scaffold/bootstrap + one-command pack for Rust & Python (.cloacina DX)

## Objective **[REQUIRED]**

Authoring a `.cloacina` package is a sharp edge: there is **no scaffold and no
reliable one-command pack**, so users hand-roll everything and hit
silent/late-binding failures. A user should be able to go from nothing to a
valid, buildable, uploadable package in **one command**, for both Rust and
Python, without knowing the internal layout rules.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P1 - High (important for user experience)

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: Packaged workflows are *the* deployment unit. If authoring one
  is error-prone, the whole packaged-workflow story is hard to adopt. A scaffold
  + one-command pack turns "read the source to figure out the layout" into
  "`cloacinactl package new`".
- **Likely an initiative, not a single task** — see decomposition below; flag
  for promotion.

## The sharp edges (all hit first-hand while building the UI demo, CLOACI-I-0117)

Authoring packages by hand this session required knowing, undocumented:
- The exact dir layout: Rust = `Cargo.toml` + `package.toml` + `src/lib.rs`
  (+ `build.rs`); **Python = `package.toml` + module tree under `workflow/`** —
  a top-level module silently fails to load (`Missing workflow source directory`).
- The `__WORKSPACE__` placeholder must be rewritten to an absolute path before
  packing (host repo vs `/workspace` in-container).
- `package.toml` `[metadata]` is a closed schema: `package_type` and
  `[[metadata.triggers]]` are **rejected** (parse error) — but appear in
  examples/soak fixtures.
- `language` must be `[metadata].language` (the server/upload), even though the
  compiler historically read `[package].language` (CLOACI-T-0666, fixed).
- `cloacinactl package pack` is **Rust-only** (requires Cargo.toml + cargo
  build); Python is hand-tarred bzip2 (CLOACI-T-0665).
- Trigger wiring is subtle: a **cron** trigger binds via `on` and must NOT be
  listed in `#[workflow(triggers=[...])]` (that's poll-trigger subscriptions);
  getting it wrong fails the load every reconcile tick (CLOACI-T-0669).
- The published docs' Python packaging procedure (`manifest.json` + top-level
  module) produces an archive the **server rejects** (CLOACI-T-0665).

Net: too much tribal knowledge, failures surface late (at upload/load, not at
author time), and the docs/examples disagree with the server.

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] `cloacinactl package new <name> --lang rust|python [--kind workflow|graph|cron]`
      scaffolds a **correct, buildable** package skeleton (right layout, valid
      `package.toml`, a minimal working workflow/graph/cron example, comments).
- [ ] `cloacinactl package pack` works for **both** languages (Rust: build+pack;
      Python: validate layout + pack — no hand-tarring). (Subsumes CLOACI-T-0665.)
- [ ] Author-time validation: `pack` (and ideally `new`) reject the known
      footguns early with actionable messages — wrong module location, bogus
      `[metadata]` keys, cron trigger listed in `#[workflow(triggers)]`, missing
      `entry_module`, etc. — instead of a late upload/load failure.
- [ ] `new` → `pack` → `upload` → builds + registers, verified end-to-end for a
      Rust workflow, a Python workflow, a computation graph, and a cron trigger.
- [ ] One canonical documented format (reconcile docs ↔ examples ↔ server) and a
      "create your first package" how-to that just uses `package new`.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
- New `cloacinactl package new`/`init` verb generating templates (consider
  `angreal init` templates or embedded skeletons). Mirror the *correct* layouts
  the reconciler accepts (Rust __WORKSPACE__-templated; Python `workflow/<mod>/`).
- Extend `package pack` to branch on `[metadata].language` (Python = archive the
  module tree, no cargo) — CLOACI-T-0665.
- Add a `package validate`/lint pass for the footguns above.

### Dependencies / related (the bugs this gap produced)
- CLOACI-T-0665 — `pack` can't do Python + 3 inconsistent formats (P1).
- CLOACI-T-0666 — compiler read `[package].language` (fixed).
- CLOACI-T-0663 — `tasks`/`symbols` metadata persists empty after build.
- CLOACI-T-0669 — packaged cron triggers (target propagation + idempotency).

### Risk Considerations
Scaffolding must track the (currently under-documented) accepted layouts — pin
them down (and fix the docs/example) as part of this, or the scaffold drifts too.

## Status Updates **[REQUIRED]**

**2026-06-13 — Filed** off the back of the CLOACI-I-0117 UI-demo work. Building
demo fixtures by hand surfaced the whole sharp edge (see "sharp edges" above) and
spun out T-0663/0665/0666/0669. Recommend promoting to an initiative
("Packaged-workflow authoring DX") and decomposing: (1) `package new` scaffold,
(2) `package pack` for Python, (3) `package validate` footgun lint, (4) format
reconciliation + docs.

**2026-06-14 — Superseded by CLOACI-I-0119; archiving.** This umbrella was
promoted to initiative [[CLOACI-I-0119]] and decomposed exactly as recommended:
- (4) format reconciliation + docs → **CLOACI-T-0677** (done)
- (2) `package pack` for Python → **CLOACI-T-0665** (done)
- (1) `package new` scaffold → **CLOACI-T-0678** (done)
- (3) `package validate` footgun lint → **CLOACI-T-0679** (done)
All acceptance criteria here are satisfied across those tasks, and the
`new → edit → validate → pack → upload` loop is regression-locked by the
`angreal test e2e cli` authoring scenario. No remaining work — archived.