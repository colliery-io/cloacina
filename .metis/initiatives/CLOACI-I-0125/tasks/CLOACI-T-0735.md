---
id: package-toml-minimization-default
level: task
title: "package.toml minimization — default the constant fields, infer language/entry_module"
short_code: "CLOACI-T-0735"
created_at: 2026-06-17T05:33:09.700128+00:00
updated_at: 2026-07-06T00:54:29.038025+00:00
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

## Investigation (2026-06-17, before impl)
- `[metadata]` is the `CloacinaMetadata` struct
  (`crates/cloacina-workflow-plugin/src/types.rs:315`). **Most fields are already
  `#[serde(default)]` Optional** — `workflow_name`, `graph_name`, `description`,
  `author`, `requires_python`, `entry_module`, `reaction_mode`, `input_strategy`,
  `accumulators`. The **only required field is `language: String`** (line 326).
- So the `[metadata]` minimization win reduces to: **infer `language` from layout**
  so it too becomes optional (this is the T-0666 drift field). The
  cloacinactl validators already classify layout
  (`crates/cloacinactl/src/nouns/package/manifest.rs:84-149`,
  `validate_rust_layout`/`validate_python_layout`), so the inference logic exists
  to reuse. `entry_module` default-to-convention is the other `[metadata]` item.
- The constant triple `interface`/`interface_version`/`extension` is **NOT** in
  `CloacinaMetadata`; it's scaffolded by `new.rs` into a separate manifest
  section consumed by the fidius/plugin layer — needs locating before defaulting
  (no `struct PackageManifest` found; parser is elsewhere).
- **Next steps when resumed:** (1) add `#[serde(default)]` + layout-inference for
  `language` (reuse the validators' layout classification) — guarded against the
  T-0666 drift; (2) default `entry_module`; (3) locate + default the
  interface/version/extension triple; (4) verify a minimal `package.toml`
  (name+version+workflow_name) packs for Rust + Python via `angreal` packaging /
  the demo fixtures, and `package validate` is clean.

## Status Updates
- 2026-06-17: Filed from the T-0720 decomposition. Not started.
- 2026-06-17: **BLOCKED — deferred pending fidius wasm traits.** fidius is
  introducing a wasm implementation of traits that may significantly reshape the
  authoring/packaging story (the cdylib + FFI + manifest model this task targets).
  Per the user, hold off on the metadata/packaging work so we don't build
  something the wasm direction would rework. Unblock = fidius wasm-traits
  direction settles. Investigation above is preserved for resumption. See
  [[project_fidius_wasm_authoring_shift]].