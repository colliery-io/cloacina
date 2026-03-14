---
id: code-example-validation-cli
level: task
title: "Code Example Validation — CLI Commands & Config/Manifest Examples"
short_code: "CLOACI-T-0101"
created_at: 2026-03-13T14:30:10.179404+00:00
updated_at: 2026-03-14T01:31:13.416920+00:00
parent: CLOACI-I-0028
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0028
---

# Code Example Validation — CLI Commands & Config/Manifest Examples

**Phase:** 3 — Code Example Validation (Pass 2)
**Parent Initiative:** [[CLOACI-I-0028]]

## Objective

Validate all CLI command examples and configuration/manifest examples across the entire documentation. Verify command syntax, flags, and output against current `cloacinactl` and `cloaca` CLIs. Verify JSON/TOML config examples parse with current schemas.

## Scope

All docs containing CLI commands or config/manifest JSON/TOML examples.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Every `cloacinactl` command example tested for valid syntax (correct subcommands, flags, arguments)
- [ ] Every `cloaca` CLI command example tested (build, etc.)
- [ ] Every `cargo` command example verified (correct flags, features, targets)
- [ ] Every `manifest.json` example validates against ManifestV2 schema
- [ ] Every `pyproject.toml` `[tool.cloaca]` example validates against current parser
- [ ] Every `Cargo.toml` example has correct dependency names and compatible versions
- [ ] No references to old `cloacina-cli` binary name (should all be `cloacinactl`)
- [ ] All broken examples fixed in-place

## Implementation Notes

### Validation Approach
1. Grep all docs for bash/shell code blocks
2. Extract CLI commands: `cloacinactl`, `cloaca`, `cargo`, `tar`, `uv`
3. For `cloacinactl`: run `cloacinactl --help` and subcommand help to verify syntax
4. For `cloaca`: verify against current Python CLI entry point
5. For manifest examples: parse with `serde_json` against ManifestV2 struct
6. For pyproject.toml examples: verify `[tool.cloaca]` section fields

### Known Risk Areas
- Old docs may reference `cloacina-cli` (renamed to `cloacinactl` in CLOACI-T-0059)
- `cloacinactl package` subcommand syntax may have changed
- Key management CLI (`cloacinactl key`) examples may be out of date

## Status Updates

### Session 1 (2026-03-13)

**Fixes applied:**
1. **package-format.md**: Replaced all 10 occurrences of `cloacina-ctl` → `cloacinactl` (CLI binary name and source path references)
2. **package-format.md**: Fixed `cloacinactl package .` commands to `cloacinactl package build` (correct subcommand); removed non-existent `--profile` flag; added `--dry-run` which actually exists
3. **package-format.md**: Rewrote `inspect` section — was documenting package inspection that doesn't exist; actual `cloacinactl package inspect` inspects detached signature files
4. **ffi-system.md**: Fixed source path reference `cloacina-ctl/src/` → `cloacinactl/src/`

**Already fixed in prior tasks:**
- Tutorial 07 `cloacina-ctl` → `cloacinactl` (T-0098)
- security/local-development.md `cloacina sign` → `cloacinactl package sign` (T-0100)

**Verification:**
- `cloacinactl --help` confirmed: subcommands are `package`, `key`, `admin`
- `cloacinactl package --help` confirmed: subcommands are `build`, `sign`, `verify`, `inspect`
- `cloacinactl package inspect --help` confirmed: inspects signature files, not packages
- Zero `cloacina-ctl` references remain in `docs/content/`
- Hugo docs build passes
