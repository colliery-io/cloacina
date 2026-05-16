---
id: t-02-unify-rest-error-envelope-fix
level: task
title: "T-02: Unify REST error envelope + fix /v1/health and /v1/ws router nest"
short_code: "CLOACI-T-0595"
created_at: 2026-05-14T17:23:12.481127+00:00
updated_at: 2026-05-14T17:37:06.769409+00:00
parent: CLOACI-I-0107
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0107
---

# T-02: Unify REST error envelope + fix /v1/health and /v1/ws router nest

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0107]]

## Objective **[REQUIRED]**

Two seam-correctness fixes that pair naturally.

### API-06 — Unify REST error envelope

The server emits **three different** error envelope shapes across its routes:
- `ApiError { code, message, request_id }` (the canonical `T-0448` shape)
- `{ error: String }` (legacy)
- Bare 4xx with a string body (legacy)

CLI's `extract_message` tries each in order; "works most of the time" but means a stale endpoint can silently send a different field name and the CLI quietly renders `error: <unparseable>`.

**Fix:** every server route returns the canonical `ApiError` shape from T-0448. Remove the legacy fallbacks. CLI's `extract_message` reads `body.message` (with `body.code` for the operator-facing classifier) and errors if the field is absent.

### API-08 — Router nest invariant

`/v1/health/*` and `/v1/ws/*` are mounted as sibling routes alongside `/v1/*` rather than under the `/v1` nest. Result: the `/v1` nest's middleware stack (request-id, tracing, OTLP, etc.) bypasses health + ws. Symptom: WS upgrade requests don't show up in tracing, health probes don't get the request-id header.

**Fix:** route `/v1/health/*` and `/v1/ws/*` through the same axum `nest("/v1", ...)` call that hosts the rest of v1. One Router::nest, not three siblings.

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

- [ ] All server error responses use `ApiError` (code + message + request_id). No `{error: ...}` or bare-string variants survive.
- [ ] CLI's `extract_message` reads `body.message` only; legacy fallbacks deleted.
- [ ] `/v1/health/*` and `/v1/ws/*` live under the `/v1` axum nest. The existing request-id middleware tags health responses with `x-request-id`.
- [ ] WebSocket upgrade requests show up in the tracing span tree under the `/v1` nest, same as other v1 routes.
- [ ] T-0597 (harness) adds integration tests that assert: (a) every 4xx response has `code` + `message`, (b) `/v1/health` response has `x-request-id`.

## Implementation Notes

- API-06: grep `IntoResponse for` and any handler that returns `(StatusCode, Json<serde_json::Value>)` for legacy variants. Convert each to `ApiError`. Pre-existing infrastructure: `crates/cloacina-server/src/routes/error.rs::ApiError` from T-0448.
- API-08: rewrite the top-level router assembly so the v1 base is `Router::new().merge(api_v1_routes())` where `api_v1_routes()` returns a single `Router` with everything mounted under `/v1`. Walk the `build_router()` callers — there's usually just one.

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

### 2026-05-14 — implemented

- **API-06**: legacy `{error: ...}` envelopes replaced with canonical `ApiError`:
  - `fallback_404` → `ApiError::not_found("not_found", ...)`.
  - `auth::validate_token` return type changed from `Result<_, (StatusCode, Json)>` to `Result<_, ApiError>`. Two Err arms (unknown/revoked key, internal validation error) now construct `ApiError::unauthorized` / `ApiError::internal`.
  - `auth::require_auth`'s caller already used `.into_response()` so the call site is unchanged; ws.rs's `map_err(|_| ApiError::unauthorized(...))` keeps working.
  - Removed unused `StatusCode` + `Json` imports from `auth.rs`.
  - CLI's `extract_message` now reads `body.message` only — the legacy `body.error.message` / `body.error` / bare-string fallbacks that silently masked schema drift are gone. Unparseable bodies render the raw JSON so an operator sees the unexpected shape.
- **API-08**: router restructure. `graph_health_routes` + `ws_routes` paths switched from absolute `/v1/...` to relative (`/health/...`, `/ws/...`). All three are merged into a single `v1` Router that's then `.nest("/v1", v1)`'d at the top level. Net effect: every `/v1/*` route lives under one nest and shares the same middleware contract; request-id + tracing + api_request_metrics layers were already at the outer Router so behavior was unchanged at runtime — this is the structural cleanup that the AC asked for.

Sequenced next: T-0596 (pagination + --sign/--follow fail-hard).
