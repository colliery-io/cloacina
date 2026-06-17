---
id: package-toml-minimization-default
level: task
title: "package.toml minimization — default the constant fields, infer language/entry_module"
short_code: "CLOACI-T-0735"
created_at: 2026-06-17T05:33:09.700128+00:00
updated_at: 2026-06-17T05:33:09.700128+00:00
parent: CLOACI-I-0125
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0125
---

# package.toml minimization — default the constant fields, infer language/entry_module

## Parent Initiative

[[CLOACI-I-0125]] — acts on theme **T4** of the [[CLOACI-T-0720]] sweep. One of
the two highest-ROI, near-pure loader wins.

## Objective

Stop `package.toml` from carrying constants-for-everyone and restating things the
layout already implies. Default the constant fields in the manifest loader and
infer `language`/`entry_module`, so a minimal Python `package.toml` can shrink to
`name` + `version` + `workflow_name`.

## Type / Priority
- Tech Debt (DX) — mostly additive (fields become optional with defaults/inference;
  explicit values still honored). P2.

## Background (verified — T-0720)
- Constant triple `interface="cloacina-workflow-plugin"`, `interface_version=1`,
  `extension="cloacina"` is identical in every fixture + template
  (`crates/cloacinactl/src/nouns/package/new.rs:167-170,354-357`).
- `language` is inferable from layout (`Cargo.toml`+`src/lib.rs` vs `workflow/`);
  validators already key on exactly that (`crates/cloacinactl/.../package/manifest.rs:84-149`).
  This field already caused a costly drift bug ([[CLOACI-T-0666]]).
- `entry_module` is conventionally `<module>.tasks`/`.graph` (`new.rs:176,217`).
- `requires_python` is unused at build (`crates/cloacina-compiler/src/build.rs:217-223`).

## Acceptance Criteria
- [ ] Omitting `interface`/`interface_version`/`extension` loads with the correct
      defaults; explicit overrides still win.
- [ ] `language` is inferred from layout when absent; explicit value overrides.
- [ ] `entry_module` defaults to the convention; `requires_python` optional.
- [ ] A minimal `package.toml` (`name`+`version`+`workflow_name`) packs and runs
      for both Rust and Python; kept as a regression guard.
- [ ] `package validate` accepts the minimal form without warnings.

## Implementation Notes
Manifest loader + `package new` template (`cloacinactl/src/nouns/package/`).
Coordinate inference with the validators that already classify layout. Note:
deriving `workflow_name`/`description` from code is the separate, larger
[[CLOACI-T-0736]] (this task only defaults/infer the manifest-local fields).

## Status Updates
- 2026-06-17: Filed from the T-0720 decomposition. Not started.