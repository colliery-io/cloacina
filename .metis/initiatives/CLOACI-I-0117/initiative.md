---
id: cloacina-web-ui-tenant-scoped
level: initiative
title: "Cloacina web UI — tenant-scoped control plane (React SPA on @cloacina/client)"
short_code: "CLOACI-I-0117"
created_at: 2026-06-10T18:33:09.675518+00:00
updated_at: 2026-06-11T02:28:56.958866+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: L
initiative_id: cloacina-web-ui-tenant-scoped
---

# Cloacina web UI — tenant-scoped control plane (React SPA on @cloacina/client) Initiative

## Context **[REQUIRED]**

Cloacina is consumable three ways today: embedded (`cloacina`/`cloaca`), terminal (`cloacinactl`), and — as of CLOACI-I-0113 — as a service via first-party SDKs (`@cloacina/client` for TS, plus Rust/Python). There is no graphical way to operate a running `cloacina-server`. Every observe-or-act loop (what's executing, why did this run fail, upload a new package, fire a workflow) goes through the CLI.

I-0113 deliberately built the data layer this initiative needs:
- **`@cloacina/client`** (npm) — a typed REST client + WebSocket execution-event stream, explicitly scoped to work in modern browsers, with pagination iterators, a typed `CloacinaApiError` (status + machine-readable `code`), and a delivery-WS wrapper that handles ticket minting, dedup, ack, and reconnect.
- **OpenAPI 3.1 contract** at `/openapi.json` + a documented WS protocol — so the UI's data shapes are generated, not hand-guessed, and drift is CI-gated.
- **Configurable CORS** on the server (off by default) and the **API-key-in-browser auth decision** (REQ-010 of I-0113): API key + tenant is the accepted v1 browser auth story for a first-party UI; browser-grade auth (sessions/OIDC) is explicitly deferred and owned *here*.

This initiative is that follow-on UI. It is a **tenant-scoped control plane**: a single human, holding one tenant's API key, manages and observes *their* workflows, executions, triggers, and keys. It is **not** the server-operator/admin console (cross-tenant visibility, tenant lifecycle, fleet management) — that is a structurally different audience and a separate future initiative.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- A browser SPA that gives a tenant-scoped user full CRUD + live observability over everything their key is authorized to touch: **workflows** (upload, list, inspect, delete), **executions** (list, filter, detail, **live event follow**), **triggers/schedules** (list, detail), and **tenant-scoped API keys** (create, list, revoke).
- Consume **`@cloacina/client`** as the only data layer — no bespoke fetch/WS code in the UI. If the UI needs something the SDK can't express, that's an SDK gap to fix upstream, not to route around. The UI is also the SDK's first real browser consumer (the forcing function I-0113 anticipated).
- A real-time execution experience: the executions detail view streams events over the delivery WS as a run progresses, with the SDK's reconnect/dedup handling the transport.
- Ship as a **standalone container** deployable alongside `cloacina-server` (compose + Helm), pointed at a server URL — exercising the CORS opt-in I-0113 shipped.
- Production-grade fundamentals: every view has explicit loading / empty / error states; `CloacinaApiError` surfaces as actionable UI (auth → re-enter key, not-found → 404 view, validation → inline); keyboard-navigable; desktop-first responsive.
- **Automated UAT**: a seed/demo harness (a workload generator that uploads fixtures and drives executions) powers both an automated Playwright acceptance pass *and* a manual "stand it up and watch it run" demo — so the live-streaming centerpiece is actually exercised, not just unit-mocked.

**Non-Goals:**
- **Server/operator administration.** No tenant lifecycle (create/remove/list tenants), no cross-tenant views, no fleet/agent or god-mode surfaces. A tenant-scoped key cannot perform these; an operator-admin console is a separate future initiative.
- **The OAuth/OIDC *server* capability.** Owned by **CLOACI-I-0118** (cloacina-server OIDC auth). I-0117 *consumes* it: the UI gains "Login with <provider>" once I-0118 ships the `/auth/*` contract, and owns only the browser side — redirect handling, storing the minted short-TTL key in `sessionStorage`, and calling `/auth/refresh` silently. The API-key `/connect` gate **remains** as the fallback / dev / headless path. Full BFF session cookies (httpOnly, no browser-held credential) stay out of scope (see I-0118 non-goals).
- **Authoring/building workflows in the browser.** The UI uploads compiled `.cloacina` packages; it does not compile, edit, or generate them. (Compilation is the compiler service's job.)
- **Computation-graph topology editing / visual DAG authoring.** Read-only graph *health* is in scope; a visual graph editor is not.
- **Mobile-first / native apps.** Desktop-first web; usable but not optimized on phones.
- **Embedding in `cloacina-server`.** Chosen deploy model is a separate container (see Architecture); same-origin embedding is an explicitly-considered-and-rejected alternative.

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

### User Requirements
- **User Characteristics**: technical — a developer or operator who already holds a tenant API key and understands workflows/executions conceptually. Comfortable with a control-plane dashboard; not a first-time end user needing hand-holding.
- **System Functionality**: see the live and historical state of their tenant's executions; trigger workflows and watch them run; upload/inspect/remove workflow packages; review schedules; manage their own API keys.
- **User Interfaces**: a left-nav SPA (see Information Architecture) — connection/auth gate → overview → per-noun list/detail views, with a persistent live-execution affordance.

### System Requirements

**Functional:**
- REQ-001: Connection + auth gate. Two ways in: (a) **"Login with <provider>"** (OIDC/GitHub via I-0118 — redirect to `/auth/login`, receive a minted short-TTL key on callback, silent `/auth/refresh`); (b) **manual** server URL + API key + tenant (fallback/dev/headless), validated via `/health` + a scoped read. Either way the credential is a bearer key in `sessionStorage` (NFR-005). No app surface is reachable unauthenticated.
- REQ-002: Overview — at-a-glance tenant state: recent executions (with status), counts, and computation-graph health for graphs visible to the key.
- REQ-003: Workflows — list (paged), detail (incl. `build_status`/`build_error`, tasks, version), multipart package upload with progress + error surfacing, and delete (with confirm).
- REQ-004: Executions — list with `status`/`workflow` filters + pagination; detail view with the full event log; **live follow** of an in-flight execution over the delivery WS, rendering events as they arrive.
- REQ-005: Triggers — list (paged) of cron + trigger schedules; detail with recent executions.
- REQ-006: Keys — create (one-time plaintext shown once, copy-to-clipboard, never re-displayed), list (no plaintext), revoke (with confirm). Tenant-scoped only.
- REQ-007: Every server error renders as a typed, actionable state — 401/403 → re-auth prompt; 404 → not-found view; 400/422 → inline validation message carrying the server's `code`/`error`; 5xx → retry affordance.
- REQ-008: All data access goes through `@cloacina/client`; the UI pins the SDK version matching the target server (lockstep, per I-0113 REQ-008).

**Non-Functional:**
- NFR-001: Every async view has explicit loading, empty, and error states — no indefinite spinners, no blank screens on error.
- NFR-002: Live execution view survives transient disconnects (SDK reconnect) and dedups redelivered events (SDK dedup) without visible duplication or gaps.
- NFR-003: Keyboard-navigable and screen-reader-labeled for primary flows (WCAG 2.1 AA as the target, pragmatically).
- NFR-004: Deterministic, reproducible build; the container image is built in CI and version-locked to the server release.
- NFR-005: **Security — API key handling (decided).** Store the key in **`sessionStorage`** — cleared when the tab closes, smaller exposure window than `localStorage`, user re-enters per session. Never log it; never put it in a URL except the SDK's single-use WS ticket. (Acknowledged residual risk: any XSS in-session can read sessionStorage — mitigated by a strict CSP and the small first-party surface; revisit if/when browser-grade auth lands.)
- NFR-006: Generated API types regenerate from the committed `openapi.json`; CI fails on drift (mirrors the SDK drift gate).

## Use Cases **[CONDITIONAL: User-Facing Initiative]**

### Use Case 1: Trigger a workflow and watch it run
- **Actor**: tenant-scoped user
- **Scenario**: From the workflow detail (or list) view, click "Execute", optionally provide JSON context, confirm. The UI calls `executeWorkflow`, navigates to the new execution's detail view, and immediately opens the live event stream — events render in order as the run progresses to a terminal state.
- **Expected Outcome**: the user sees the run start, stream, and finish without manually refreshing; a failure surfaces the failing task/event inline.

### Use Case 2: Debug a failed execution
- **Actor**: tenant-scoped user
- **Scenario**: From the executions list, filter by `status=Failed`, open a failed run, read its event log (with event types, data, timestamps), and identify the failing step.
- **Expected Outcome**: the failure is diagnosable from the event log without dropping to the CLI or the database.

### Use Case 3: Publish a new workflow package
- **Actor**: tenant-scoped user
- **Scenario**: From the workflows view, upload a compiled `.cloacina` package. The UI shows upload progress, then the resulting package's build status; a rejected/garbage package surfaces the server's error `code` inline.
- **Expected Outcome**: the user confirms the package registered (or sees exactly why it didn't) and can immediately execute it.

### Use Case 4: Rotate a tenant API key
- **Actor**: tenant-scoped user with an admin-role key
- **Scenario**: Create a new tenant-scoped key (plaintext shown once, copied), verify it appears in the list, then revoke the old one.
- **Expected Outcome**: key rotated; plaintext was capturable exactly once and is never recoverable from the UI afterward.

## Architecture **[CONDITIONAL: Technically Complex Initiative]**

### Overview
A client-rendered React + TypeScript SPA built with Vite, consuming `@cloacina/client` as its sole data layer. Server state (queries, caching, invalidation, retries) is managed by a data-fetching library (TanStack Query is the leading candidate) wrapping the SDK; live execution events come from the SDK's `followExecutionEvents` async iterator bridged into React state. The app ships as a static bundle inside an nginx (or similar) container image, configured at runtime with the target server URL; it talks to `cloacina-server` cross-origin, requiring the server's CORS opt-in (`--cors-allowed-origins`).

### Component Diagrams
```
┌─────────────────────── browser ───────────────────────┐
│  React SPA (Vite build, served by nginx container)     │
│                                                         │
│  routes/views ── hooks (TanStack Query) ── @cloacina/   │
│       │                │                     client     │
│   live-exec view ──────┴── followExecutionEvents() ─────┼──► WS  /v1/ws/delivery/*
│                                                         │
│            CloacinaClient (REST) ──────────────────────┼──► HTTP /v1/* (CORS)
└─────────────────────────────────────────────────────────┘
                                  │
                          cloacina-server
```

### Deployment Diagrams
- New container image (e.g. `cloacina-ui`) — multi-stage: Vite build → static assets served by nginx.
- Compose: a `ui` service alongside `server`; server configured with `CLOACINA_CORS_ALLOWED_ORIGINS` for the UI origin.
- Helm: an optional `ui` deployment + service (+ ingress), gated like the existing optional components.
- Runtime config: the target server URL is injected at container start (env → a generated `config.js` or nginx-templated), so one image works against any server — the bundle itself is server-agnostic.

## Detailed Design **[REQUIRED]**

### Stack (decided)
- **React + Vite + TypeScript** (per discovery decision). Routing via React Router; server-state via TanStack Query over `@cloacina/client`; minimal client state (connection/auth context) via React context or a small store.
- **Separate container** deploy (per discovery decision) → cross-origin → depends on the server's CORS opt-in.

### Data layer
- `@cloacina/client` is the only thing that talks to the server. A thin app-side layer wraps it in query hooks (`useWorkflows`, `useExecution`, …) with cache keys per tenant + resource, and maps `CloacinaApiError` to typed UI states.
- Live events: a hook bridges `followExecutionEvents(execId)` into component state, appending events and managing the iterator lifecycle (start on mount of a live execution, cancel on unmount). Dedup + reconnect are the SDK's job; the hook handles "is this execution terminal yet" and stream-closed UX.
- Auth context holds `{ serverUrl, apiKey, tenant }`, constructs the `CloacinaClient`, and gates the router.

### Build / drift
- Generated types: the UI either depends on the published `@cloacina/client` (which carries its own generated types) or regenerates from the committed `docs/static/openapi.json`. Prefer depending on the published package so there's one generation, version-pinned to the target server.

## UI/UX Design **[CONDITIONAL: Frontend Initiative]**

### Information Architecture (the page map)
```
(unauthenticated)
  /connect            — "Login with <provider>" (OIDC/GitHub, via I-0118) OR manual server URL + API key + tenant; validates, then enters the app

(authenticated shell: left nav + tenant/connection indicator + "disconnect")
  /                   — Overview: recent executions, status rollup, graph health
  /workflows          — Workflows list (paged)
  /workflows/:name    — Workflow detail (build status, tasks, version) + Execute + Delete
  /workflows/upload   — Upload a .cloacina package (progress, result)
  /executions         — Executions list (status/workflow filters, pagination)
  /executions/:id     — Execution detail + event log + LIVE follow toggle
  /triggers           — Triggers/schedules list (paged)
  /triggers/:name     — Trigger detail + recent executions
  /keys               — API keys list + Create (one-time plaintext) + Revoke
  /settings           — Connection details, SDK/server version, API-key handling notes
```
Graph health (accumulators + graphs) appears on the Overview; whether it earns its own top-level view in v1 is an open question (depends on how central computation graphs are to the target user).

### User Flows
- **Execute → follow** (UC-1): `/workflows/:name` → Execute (context modal) → redirect `/executions/:id` with live stream auto-started.
- **Debug** (UC-2): `/executions?status=Failed` → `/executions/:id` → event log.
- **Upload** (UC-3): `/workflows/upload` → drag/select file → progress → result (success → link to detail; failure → inline `code`/`error`).
- **Key rotation** (UC-4): `/keys` → Create → copy plaintext (shown once) → confirm in list → Revoke old (confirm dialog).
- **Connect/disconnect** (REQ-001): `/connect` validates and persists; "disconnect" clears stored credentials and returns to `/connect`.

### Design System Integration (decided)
No existing web design language in the repo (the docs site is Hugo-themed; not reusable here). **Decision: adopt an off-the-shelf, accessible React component library** (shadcn/ui, Mantine, or Radix-based — final pick in design phase) so NFR-003 accessibility comes mostly for free and the team builds product, not primitives. A distinctive, non-generic visual treatment is layered on top via the `frontend-design` skill rather than fighting the library's defaults.

## Testing Strategy **[CONDITIONAL: Separate Testing Initiative]**

### Unit Testing
- **Strategy**: component + hook tests (Vitest + Testing Library); mock `@cloacina/client` at the hook boundary so view logic (loading/empty/error/data states) is tested without a server.
- **Tools**: Vitest, @testing-library/react.

### Automated UAT — seed/demo harness + browser E2E
The UI's centerpiece (live execution streaming) cannot be tested or demoed without *something producing executions*. So UAT is built on a **seed + demo harness**: a small service that, against a target server, ensures a tenant, uploads fixture `.cloacina` packages, and drives executions. It has two modes:
- **Seed mode (deterministic)** — establishes a known state (a few completed runs, one *failed* run, one in-flight) so the E2E assertions are stable.
- **Loop mode (continuous workload generator)** — fires executions on an interval, mixing fast, slow-enough-to-watch, and intentionally-failing runs, so the UI always has live activity. This powers the manual "stand it up and watch it run" experience.

The same harness serves both automated UAT and a manual demo:
- **Automated UAT**: a Playwright `ui-e2e` lane builds + serves the SPA, runs the harness in seed mode against a `cloacina-server` booted on a fresh DB (reusing the I-0113 `sdk-contract` server-boot harness), then asserts the acceptance scenarios end-to-end: connect → list → **follow a live run to completion** → open the failed run and read its error → upload (success + rejected paths) → key create/revoke. CORS enabled.
- **Manual demo**: a `docker compose` *demo profile* wires postgres + server + UI + the harness in loop mode, so `compose up` yields a UI with continuous live activity to watch.
- **Fixtures**: requires at least a **slow-streaming** workflow (emits a visible event sequence over ~10–30s — long enough to watch stream) and a **failing** workflow (to exercise the debug/failed-state UI), in addition to reusing existing `examples/fixtures/*` packages where they fit.

### Drift
The UI's generated types are checked against the committed spec in CI (mirrors the SDK gate); SDK + UI versions move in lockstep with the server.

## Alternatives Considered **[REQUIRED]**

- **Served by `cloacina-server` (same-origin), no CORS.** Simpler ops and sidesteps CORS entirely, but couples UI release to the server binary and bloats it with static assets. Rejected per discovery decision in favor of an independently-deployable container; the trade-off is depending on the CORS opt-in (already shipped).
- **Operator/admin console (cross-tenant) as v1.** A different, broader audience (tenant lifecycle, fleet, god-mode). Deferred to a separate initiative; v1 is deliberately tenant-scoped to ship something coherent and useful first.
- **SvelteKit / Vue.** Viable, but React + Vite chosen for ecosystem depth, admin-component availability, and being the conventional choice for this kind of dashboard.
- **Hand-rolled fetch/WS instead of `@cloacina/client`.** Rejected — it would duplicate (and drift from) the SDK the platform just invested in; the UI consuming the SDK is itself a goal (dogfooding the browser path).
- **Browser-grade auth (OIDC/sessions) in v1.** Rejected for v1 scope; API-key auth is the sanctioned starting point (I-0113 REQ-010). Sequenced as a follow-on once the surface is proven.

## Implementation Plan **[REQUIRED]**

Candidate phases (each a task batch on decomposition). Phase 1 is the walking skeleton that makes everything after it cheap.

1. **App skeleton + connection/auth gate + SDK wiring.** Vite/React/TS project under `ui/`, TanStack Query + SDK, auth context (key in `sessionStorage`), `/connect` flow, the authenticated shell (nav + routing), and the chosen off-the-shelf component library. Exit: can connect to a live server and land on an (empty) overview; loading/empty/error primitives exist.
2. **Read surfaces.** Overview, workflows list/detail, executions list/detail (event log, non-live), triggers list/detail, graph health. Exit: every documented read endpoint is rendered with proper states.
3. **Live execution stream.** The WS-backed live-follow on execution detail (the real-time centerpiece). Exit: execute-from-CLI, watch it stream in the UI; reconnect/dedup verified.
4. **Write surfaces.** Workflow upload (multipart + progress), execute-with-context, workflow delete, key create/list/revoke — all with confirms and typed error surfacing. Exit: full tenant-scoped CRUD.
5. **Package, deploy, test, document.** Container image, compose + optional Helm, Playwright `ui-e2e` lane in CI, type-drift gate, Diataxis docs (tutorial + how-to), version lockstep. Exit: image published in lockstep with the server release; E2E green.

Sequencing note: Phase 1 unblocks 2–4, which can parallelize by noun once the skeleton + component library are settled. Phase 3 depends on Phase 2's execution-detail view existing. Phase 5 aggregates.

**Cross-initiative dependency:** I-0117 is *not* blocked by I-0118 — Phase 1's auth gate ships with the manual API-key `/connect` path first. Only the **"Login with <provider>"** feature depends on I-0118 reaching ~Phase 3 (mint + `/auth/refresh`) with a stable `/auth/*` contract. Slot the UI OIDC-login work as a late phase (or a fast-follow) gated on that contract.

## Open Questions

**Resolved in discovery (2026-06-10):**
- **OQ-1 — Component/design system** → off-the-shelf accessible library + `frontend-design` visual layer (see Design System Integration).
- **OQ-2 — API-key storage** → `sessionStorage` (see NFR-005).
- **OQ-3 — Repo location** → top-level `ui/`.

**Deferred to design phase (do not block decomposition):**
- **OQ-4 — Graph health prominence**: Overview-only, or its own top-level view in v1? Depends on how central computation graphs are to the tenant-scoped user. Settle with a prototype.
- **OQ-5 — Runtime server-URL config**: env-injected `config.js` vs. nginx template vs. server-discovery convention — how "one image, any server" actually works. A Phase 5 (packaging) concern.
- **OQ-6 — Execution history vs. live merge**: how much historical event-log depth shows by default, and whether the live stream backfills from the REST events endpoint on open or only tails forward (the delivery WS resyncs unacked rows; historical events come from REST — define the merge). A Phase 3 (live stream) concern.
