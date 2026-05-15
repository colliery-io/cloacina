---
id: t-02-cloacina-server-docker-image
level: task
title: "T-02: cloacina-server Docker image + ghcr.io publish"
short_code: "CLOACI-T-0604"
created_at: 2026-05-14T22:45:16.385322+00:00
updated_at: 2026-05-14T23:35:39.601242+00:00
parent: CLOACI-I-0111
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0111
---

# T-02: cloacina-server Docker image + ghcr.io publish

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0111]]

## Objective **[REQUIRED]**

Ship `cloacina-server` as a multi-stage Docker image published to `ghcr.io/colliery-software/cloacina-server`.

### Deliverables

1. **`Dockerfile`** at repo root, multi-stage:
   - Stage 1 (builder): `rust:1.85-slim`, install Postgres dev headers + libpq, `cargo build --release -p cloacina-server`
   - Stage 2 (runtime): `gcr.io/distroless/cc-debian12`, copy binary, `EXPOSE 8080`, healthcheck via the binary's own probe
   - No Rust toolchain in runtime — CG compilation routes through the compiler service (I-0097)
   - `LABEL org.opencontainers.image.source=https://github.com/colliery-software/cloacina` so ghcr links the image to the repo

2. **`.dockerignore`** scoping out `target/`, `.metis/`, docs, tests, examples.

3. **Release workflow job** in `unified_release.yml`:
   - On tag: `docker buildx build` for `linux/amd64` + `linux/arm64`, push tags `<semver>`, `<major>.<minor>`, `latest`
   - On nightly schedule: push `nightly` tag
   - `docker/login-action` with `GITHUB_TOKEN`

4. **Smoke test** in CI:
   - `docker run --rm <image> --version` exits 0 with the tag
   - Optional: spin up server, hit `/v1/health`, expect 200

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `docker build -t cloacina-server:dev .` succeeds locally
- [ ] Compressed image < 200 MB
- [ ] Release tag pushes multi-arch image to ghcr.io
- [ ] `docker pull ghcr.io/colliery-software/cloacina-server:<tag>` works post-release
- [ ] Healthcheck reports healthy with `DATABASE_URL` wired
- [ ] Image runs as non-root

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
