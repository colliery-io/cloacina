---
id: t8-shell-completions-bash-zsh-fish
level: task
title: "T8: Shell completions (bash, zsh, fish, powershell)"
short_code: "CLOACI-T-0517"
created_at: 2026-04-17T17:00:00+00:00
updated_at: 2026-04-18T01:40:12.974652+00:00
parent: CLOACI-I-0098
blocked_by: [CLOACI-T-0514, CLOACI-T-0515, CLOACI-T-0516]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0098
---

# T8: Shell completions (bash, zsh, fish, powershell)

## Parent Initiative

CLOACI-I-0098 — cloacinactl CLI redesign

## Objective

Emit shell completion scripts for the four supported shells via `clap_complete`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `cloacinactl completions <shell>` emits a completion script for `bash | zsh | fish | powershell` on stdout.
- [ ] Completions include top-level nouns, nested verbs, and all flag names.
- [ ] Value hints wired up for path-valued flags (`--home`, `--out`, `--context`, `--sign`) and enum-valued flags (`-o`, `--role`).
- [ ] Manual verification: source each script into a fresh shell, confirm tab completion shows expected values.
- [ ] README / `docs/cli-completions.md` snippet showing install for each shell.

## Implementation Notes

### `clap_complete` dependency

Add `clap_complete = "4"` to cloacinactl deps. Use `generate(shell, &mut cmd, bin_name, &mut stdout)`.

### Dynamic completion deferred

Completing tenant names, package IDs, workflow names etc. from the server is **not** in scope for v1. Documented in the spec's Open Items.

### Shell-install ergonomics

Not auto-installing into the user's shell config. Just emit; user integrates.

## Status Updates

*To be added during implementation*
