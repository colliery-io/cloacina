---
id: t-01-install-script-github
level: task
title: "T-01: Install script + GitHub Releases binaries (cloacinactl + daemon)"
short_code: "CLOACI-T-0603"
created_at: 2026-05-14T22:45:15.030610+00:00
updated_at: 2026-05-14T23:28:39.981025+00:00
parent: CLOACI-I-0111
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0111
---

# T-01: Install script + GitHub Releases binaries (cloacinactl + daemon)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0111]]

## Objective **[REQUIRED]**

Ship `cloacinactl` and `cloacina-daemon` as pre-built binaries via GitHub Releases plus a `curl | bash` install script.

### Deliverables

1. **Cross-compile job** in `.github/workflows/unified_release.yml` (or new `release-binaries.yml`):
   - Targets: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`
   - Linux: `cross` (Docker-backed cross-compile) on `ubuntu-latest`; macOS: native build on `macos-latest`
   - Each artifact: `cloacinactl-<version>-<target>.tar.gz` (stripped binary) + `.sha256` sidecar; same matrix for `cloacina-daemon`
   - Upload to the GitHub Release on the tag

2. **Install script** at `scripts/install.sh`:
   - Detect OS+arch, map to target triple
   - `--version vX.Y.Z` to pin, otherwise resolve `latest` via GitHub API
   - Download both tarballs + sha256, verify, extract to `${PREFIX:-$HOME/.cloacina}/bin`
   - `--prefix /usr/local` for system-wide (escalates with `sudo` if needed)
   - PATH-add hint when install dir not on `$PATH`
   - Idempotent — re-running upgrades in place

3. **Docs**: `docs/content/quick-start/install.md` with the curl-bash one-liner, `--version` pin example, and uninstall instructions.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Release tag produces 8 tarballs (4 targets × 2 binaries) + 8 sha256 sidecars on the GitHub Release page
- [ ] `bash scripts/install.sh` on macOS arm64 installs both binaries; `cloacinactl --version` prints the tag
- [ ] Same on Linux x86_64 (smoke in a docker container)
- [ ] `--version v0.6.0` pins to that release
- [ ] Sha256 mismatch aborts with a clear error
- [ ] Quick-start doc renders

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

*To be added during implementation*
