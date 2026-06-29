---
id: constructor-distribution-as-fidius
level: task
title: "Constructor distribution as fidius provider packages + registry load"
short_code: "CLOACI-T-0827"
created_at: 2026-06-28T23:57:48.544299+00:00
updated_at: 2026-06-29T11:44:42.503204+00:00
parent: CLOACI-I-0132
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Constructor distribution as fidius provider packages + registry load

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Constructors distributed as **fidius provider packages** (signed, versioned) + the registry **load path** (install/resolve/load an constructor package).

**AC:** an constructor is packaged (`.fid`/`.cloacina`), signed, installed, and loaded by a server/runtime from the registry; versioning + signature verification enforced; a "provider" = a package of constructors. Blocked by CLOACI-T-0823 (the loader); can follow CLOACI-T-0825.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement  
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates **[REQUIRED]**

### 2026-06-29 — Provider packaging + signed load-from-package landed (branch `i0132-t0827-packaging`, NOT committed)

**Implemented (mirrors workflow packaging, reuses fidius rather than a parallel format):**

1. **Provider package format + packaging step** — `crates/cloacina/src/packaging/constructor_provider.rs` (new, gated by new default-OFF `constructor-packaging` feature). `package_constructor_provider()` builds the crate to `wasm32-wasip2`, runs its manifest-emitter bin (`emit_manifest`) for the constructor manifest JSON, stages a fidius `runtime = "wasm"` package dir (`package.toml` + the `.wasm` component + `constructor.json` sidecar + a new N-capable `provider.json` index), optionally Ed25519-signs it (reusing fidius's `package_digest` + `package.sig` scheme, byte-compatible with `fidius_host::verify_package`), then packs via `fidius_core::package::pack_package` → `<name>-<version>.cloacina`. `ProviderManifest { constructors: Vec<…> }` is structured for N constructors; single is wired end-to-end (a fidius `[wasm]` package binds one component; N-per-archive is the noted follow-on).

2. **Command** — `cloacinactl constructor package <crate-dir> [--out] [--sign-key K] [--manifest-bin emit_manifest] [--debug]` (`crates/cloacinactl/src/nouns/constructor/mod.rs`, wired in `nouns/mod.rs` + `main.rs`). cloacinactl enables `cloacina/constructor-packaging` (wasmtime-free).

3. **Loader resolves the packaged form** — `constructor_loader.rs`: new `unpack_provider_archive()` (unpack + optional Ed25519 signature verify; fails closed on hostile/tampered/unsigned-when-required archive) and `load_task_constructor_from_package()` (unpack/verify → delegate to existing loose-dir `load_task_constructor`, since fidius resolves a package by its `[package].name` regardless of dir name). Loose-dir path unchanged.

**Gating (intact):** new `constructor-packaging` feature pulls only the serde-only contract crate — NOT `fidius-host/wasm`. `constructors-wasm` now implies `constructor-packaging`. `cargo tree -p cloacina -i wasmtime` ABSENT under both default and `constructor-packaging`; only the wasm *loader* pulls wasmtime.

**Validation (all run, all green):**
- `cargo check -p cloacina` (default) — Finished clean.
- `cargo check -p cloacina --features constructor-packaging` — clean; `cargo tree … -i wasmtime` → "did not match any packages" (absent).
- `cargo check -p cloacinactl` — clean.
- `cargo test -p cloacina --features constructors-wasm --test constructor_provider_package_wasm` — **4 passed**: signed package → load-from-package → runs as Task → `result == "hello, world"`; wrong-key fails closed; tampered (repacked) package fails signature verification; missing `constructor.json` fails closed.
- Regression `--test constructor_macro_wasm` — 4 passed (loose-dir path intact).
- Packaging lib unit tests — 2 passed.
- CLI smoke: `cloacinactl constructor package <fixture>` → archive `prefix-0.1.0/{package.toml,constructor.json,provider.json,task_constructor_macro_fixture.wasm}`.
- `cargo fmt --all -- --check` exit 0.

**Deferred (noted, not built):** remote registry/publish endpoint (push/pull providers); signing-key management; multi-constructor-per-archive packing (data model is already N-shaped); Python authoring/consumer surface; trigger/accumulator/reactor packaging (only `task` codegen exists today).

**NOT committed — left for review.**