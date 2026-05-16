---
id: t-04-tutorials-how-to-and-breaking
level: task
title: "T-04: Tutorials, how-to, and breaking-change release notes for CG split"
short_code: "CLOACI-T-0542"
created_at: 2026-04-24T15:08:24.727807+00:00
updated_at: 2026-05-12T19:07:52.815517+00:00
parent: CLOACI-I-0101
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0101
---

# T-04: Tutorials, how-to, and breaking-change release notes for CG split

## Parent Initiative

[[CLOACI-I-0101]]

## Objective

Bring the user-facing docs, tutorials, and release notes in line with the post-I-0101 declaration surface. All tutorials that touch computation graphs use the split form; a new how-to guide covers "wrap a computation graph as a workflow task node"; release notes call out the breaking change with a clear before/after migration example. No user-facing reference to the bundled form survives.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] All Rust computation-graph tutorials under `docs/content/computation-graphs/tutorials/` use the split declaration (`#[reactor]` + `#[computation_graph(trigger = reactor("name"))]`).
- [x] All Python computation-graph tutorials under `docs/content/python/tutorials/computation-graphs/` use the split decorator form (`@cloaca.reactor(...)` + `ComputationGraphBuilder(..., reactor=ClassObject)`).
- [x] New how-to guide authored at `docs/content/computation-graphs/how-to-guides/computation-graph-in-workflow.md` covering `invokes = computation_graph("name")` / `@task(invokes=...)`, context adapter, post-invocation hook, and embedded-vs-standalone guidance.
- [x] Reference docs rewritten (CG reference, reactor-lifecycle, trigger-less-graphs, ffi-vtable, troubleshooting). Two auto-generated API-ref pages carry HTML TODOs for the generator pass.
- [x] CHANGELOG `[Unreleased]` → `Changed (breaking)` entry added with Rust + Python before/after snippets, linking to CLOACI-S-0011 and the new how-to.
- [x] How-to index updated with the new guide.
- [x] Grep of `docs/content/` for the bundled `react =` form: no surviving references outside historical CHANGELOG context.
- [ ] `angreal docs build` — run externally (per user convention, builds not run in-tool).

## Status Updates

**2026-05-12** — Completed.

Docs sweep done across 18 files via subagent + direct authoring:

- **Rust tutorial library 07–10** and **service 09–10**: bundled `react = when_any(...)` rewritten to split `#[reactor(...)]` + `#[computation_graph(trigger = reactor("name"))]`, matching example sources verbatim. Cross-package-binding also had stale `accumulators = ["..."]` quoting and paren-less `criteria = when_any` — corrected to bare-ident + parenthesized form per the macro parser.
- **CG reference**: full macro section rewrite, new `#[reactor]` companion section, criteria table updated.
- **Explanation docs**: `reactor-lifecycle.md` declaration-model rewrite + dropped bundled-form back-compat section; `trigger-less-graphs.md` stray bundled mention cleaned; `ffi-vtable.md` "synthesized reactor" line rewritten — synthesized reactors no longer exist.
- **Python tutorials 09/10/11**: confirmed verbatim against example sources; no edits needed (already current).
- **CHANGELOG**: `[Unreleased]` entry with Rust + Python before/after migration snippets, S-0011 link, how-to link.
- **New how-to**: `computation-graph-in-workflow.md` with worked end-to-end example + Python parity + linked from how-to index.

### Outstanding (not blocking close)

- `docs/content/workflows/how-to-guides/sequential-strategy.md` — HTML TODO marks unresolved macro-level sequential strategy syntax. Reactor macro has no `input_strategy =` clause; sequential strategy is set on `Reactor::new` at runtime, which the doc shows lower down. Re-review when a macro clause exists.
- Two auto-generated API-ref pages have TODOs for the next generator pass.
- `angreal docs build` needs to run externally to verify hugo green.

## Implementation Notes

### Technical Approach

1. Audit `docs/content/computation-graphs/**` for any code examples that use the bundled macro form. Rewrite each using the split form. Verify that the narrative still flows — if the existing tutorial's pedagogy assumed "declare reactor + accumulators + graph all in one block," it may need a small reordering so the reactor and graph are introduced as distinct steps.
2. Same pass on `docs/content/python/tutorials/computation-graphs/**`.
3. Write the new how-to guide. Recommended structure:
   - Problem statement ("I have a multi-step workflow and one step is a computation graph").
   - Declaration shape (trigger-less CG + task with `invokes`).
   - Context adapter notes (what gets serialized, what terminal outputs look like in the downstream task's context).
   - When to use this vs. standalone-CG-with-reactor (the two quantum models from S-0011).
4. Draft the release notes entry. Before/after snippets should be copy-pasteable. Add a one-liner pointing at the how-to for the embedded-CG case.
5. Grep + cleanup pass for stray references to the old form.

### Dependencies

- **T-01b** (bundled form removed — docs should show the final state).
- **T-03** (Python parity exists — tutorials and how-to cover both languages).

### Risk Considerations

- Docs are often written in a way that conflates "the reactor" and "the graph." With the split, every existing sentence that said "the reactor" or "the computation graph" needs to be read carefully to make sure it still names the right primitive per CLOACI-S-0011 R1–R3.
- Release-notes tone: the breaking change needs to be clearly marked and easy to migrate from. A short, direct entry beats a comprehensive one here.

## Status Updates

*To be added during implementation.*
