---
id: ui-oidc-login-integration-login
level: task
title: "UI OIDC login integration — \"Login with provider\" browser flow (gated on I-0118)"
short_code: "CLOACI-T-0662"
created_at: 2026-06-11T02:19:06.404306+00:00
updated_at: 2026-06-11T18:00:00.000000+00:00
parent: CLOACI-I-0118
blocked_by: ["CLOACI-T-0651"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# UI OIDC login integration — "Login with provider" browser flow (gated on I-0118)

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

The browser side of OIDC/GitHub login (REQ-001 path a): "Login with <provider>" buttons on `/connect`, the redirect/callback handling, storing the server-minted short-TTL key in `sessionStorage`, and silent `/auth/refresh` for all-day sessions. The server capability is owned by **CLOACI-I-0118**; this task consumes its `/auth/*` contract. Cross-initiative dependency — **gated on I-0118 reaching ~Phase 3 (mint + refresh) with a stable contract**; not on the v1 critical path.

## Acceptance Criteria **[REQUIRED]**

- [ ] `/connect` gains "Login with <provider>" alongside the manual API-key path; clicking redirects to the server's `/auth/login`.
- [ ] Callback handling: receive the minted short-TTL key, store it in `sessionStorage` exactly like the manual key, and enter the app (the rest of the UI is unchanged — it's still a bearer key).
- [ ] **Silent refresh**: before the short-TTL key expires, call `/auth/refresh` to re-mint without a full re-login; on refresh failure, fall back to the login screen.
- [ ] **Logout** calls the server's `/auth/logout` (revoke + forget) in addition to clearing local credentials.
- [ ] Provider list is server-driven (the UI shows whatever providers I-0118 advertises), not hard-coded.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Because the hybrid model (I-0118) mints a bearer key that lands in `sessionStorage` exactly like the manual key, this is **additive** to the T-0651 auth context — not a rearchitecture. The new surface is the redirect/callback round-trip + a refresh timer. Keep the manual `/connect` path as the fallback.

### Dependencies
Blocked by CLOACI-T-0651 (auth context/`/connect` to extend). **Hard cross-initiative gate: CLOACI-I-0118 Phase 3** (`/auth/login`, `/auth/callback`, `/auth/refresh`, `/auth/logout`) must be live and contract-stable. Not blocking the rest of I-0117 — schedule as a fast-follow once I-0118 lands.

### Risk Considerations
Contract coupling to I-0118 — co-design the `/auth/*` shapes with that initiative (the UI is its forcing-function consumer, mirroring how this UI dogfoods the SDK). Refresh-timing edge cases (tab asleep, clock skew) — refresh on a margin before expiry and on 401.

## Status Updates **[REQUIRED]**

*To be added during implementation*
