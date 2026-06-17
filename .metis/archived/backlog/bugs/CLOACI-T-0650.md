---
id: fix-cloacinactl-release-binary
level: task
title: "Fix cloacinactl release-binary matrix (aarch64-linux + x86_64-darwin)"
short_code: "CLOACI-T-0650"
created_at: 2026-06-10T03:31:33.102846+00:00
updated_at: 2026-06-17T11:51:26.584723+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Fix cloacinactl release-binary matrix (aarch64-linux + x86_64-darwin)

## Objective

The `build-release-binaries` job in `unified_release.yml` cross-compiles `cloacinactl`
for four targets. On the v0.7.0 release, 2 of 4 failed (the GitHub Release shipped with
only `x86_64-unknown-linux-gnu` + `aarch64-apple-darwin`). Make all four targets build +
upload so `scripts/install.sh` works on every supported platform. Lands in v0.7.1.

Follow-up to [[cut-v0-7-0-release-fleet-default]] (CLOACI-T-0641).

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (release convenience binaries; the crate/image/chart all shipped fine)

### Impact Assessment
- **Affected Users**: Users on ARM Linux (`aarch64-unknown-linux-gnu`) and Intel macOS
  (`x86_64-apple-darwin`) who install `cloacinactl` via the prebuilt tarball / `install.sh`.
  Workaround: `cargo install` from crates.io (0.7.0 is published).
- **Reproduction**: Tag a release → `build-release-binaries` matrix → those two legs fail.
- **Expected vs Actual**: All four tarballs attach to the GitHub Release; actual = 2/4.

### Root cause (from v0.7.0 run 27246082223)
`crates/cloacinactl/Cargo.toml` declares `[features] default = ["postgres","sqlite","kafka"]`.
The release build runs `cargo build --release --locked --target <t> --bin cloacinactl`
with default features on, so every binary links **librdkafka** (kafka) and pulls **pyo3**
transitively:
- `aarch64-unknown-linux-gnu` (cross): `pyo3-build-config` → `no Python 3.x interpreter
  found` — the `cross` container has no Python.
- `x86_64-apple-darwin` (x86 target on an arm64 macOS runner): `ld: symbol(s) not found
  for architecture x86_64` (`_rd_kafka_*`) — no x86_64 librdkafka present.
The two native-arch legs tolerate it; the cross/x-arch legs don't.

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

## Acceptance Criteria

## Acceptance Criteria

- [ ] All 4 `build-release-binaries` targets build + upload on a tagged release: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`
- [ ] `cloacinactl` is decided: does the distributed CLI binary need `kafka`/`pyo3` at all? If not, build it with `--no-default-features --features postgres,sqlite` (or trim the crate's default features) so it stops linking librdkafka + pulling pyo3
- [ ] If kafka/pyo3 ARE required in the binary: provide Python in the `cross` image (aarch64-linux) and x86_64 `librdkafka` on the macOS runner (x86_64-darwin)
- [ ] `scripts/install.sh` verified to resolve + run the tarball on each platform
- [ ] Lands in v0.7.1 (next tagged release)

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

*To be added during implementation*

## Status Updates
- 2026-06-17: **Fixed + verified locally.** Decided the distributed CLI does NOT
  need `kafka` (a runtime accumulator backend) — that feature is what linked
  librdkafka and pulled pyo3 transitively, breaking the cross (aarch64-linux, no
  Python) and x-arch (x86_64-darwin, no x86_64 librdkafka) legs. Changed both the
  native and cross build steps in `.github/workflows/unified_release.yml` to
  `-p cloacinactl --no-default-features --features postgres,sqlite --bin cloacinactl`.
  Verified locally: `cargo build -p cloacinactl --no-default-features --features
  postgres,sqlite` finishes clean, the binary runs (`cloacinactl 0.7.0`), and
  `otool -L` shows **no librdkafka / no libpython** linkage — exactly the two
  things that broke the failing legs. The 4-target cross-build + `install.sh`
  per-platform checks will confirm on the next tagged release (v0.7.1).