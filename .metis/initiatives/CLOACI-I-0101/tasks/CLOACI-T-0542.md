---
id: t-04-tutorials-how-to-and-breaking
level: task
title: "T-04: Tutorials, how-to, and breaking-change release notes for CG split"
short_code: "CLOACI-T-0542"
created_at: 2026-04-24T15:08:24.727807+00:00
updated_at: 2026-04-24T15:08:24.727807+00:00
parent: CLOACI-I-0101
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0101
---

# T-04: Tutorials, how-to, and breaking-change release notes for CG split

## Parent Initiative

[[CLOACI-I-0101]]

## Objective

Bring the user-facing docs, tutorials, and release notes in line with the post-I-0101 declaration surface. All tutorials that touch computation graphs use the split form; a new how-to guide covers "wrap a computation graph as a workflow task node"; release notes call out the breaking change with a clear before/after migration example. No user-facing reference to the bundled form survives.

## Acceptance Criteria

- [ ] All Rust computation-graph tutorials under `docs/content/computation-graphs/tutorials/` use the split declaration (`#[reactor]` + `#[computation_graph(trigger = reactor("name"))]` for standalone CGs).
- [ ] All Python computation-graph tutorials under `docs/content/python/tutorials/computation-graphs/` use the split decorator form (`@cloaca.computation_graph(trigger=cloaca.reactor("name"))`).
- [ ] New how-to guide: `docs/content/platform/how-to-guides/computation-graph-in-workflow.md` (or equivalent location), covering the `invokes = computation_graph("name")` / `@task(invokes=...)` pattern, the context ↔ graph-type adapter, and when to prefer embedded-CG vs. standalone-CG.
- [ ] Any reference doc (`docs/content/api-reference/**`, tutorials' `_index.md`, glossary) that mentioned the bundled form is rewritten.
- [ ] Release notes entry (wherever the project tracks them — `CHANGELOG.md` or an entry under `.github/release_notes/` or the docs' release section) that:
  - Announces the breaking change.
  - Shows a before-and-after migration snippet for Rust.
  - Shows the same for Python.
  - Links to CLOACI-I-0101 and CLOACI-S-0011.
- [ ] `docs/content/_index.md` and any entry-point material reflects the new mental model (reactor as standalone trigger; CG optionally declares its trigger upstream).
- [ ] Full-text grep across `docs/content/` for the old bundled form turns up nothing (or only release-notes historical references).
- [ ] `angreal docs build` green.

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
