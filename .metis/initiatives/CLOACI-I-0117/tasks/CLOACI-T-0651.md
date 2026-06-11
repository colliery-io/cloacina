---
id: ui-app-skeleton-vite-react-ts
level: task
title: "UI app skeleton — Vite/React/TS under ui/, component library, routing, auth gate, SDK + TanStack Query wiring"
short_code: "CLOACI-T-0651"
created_at: 2026-06-11T02:18:51.335029+00:00
updated_at: 2026-06-11T10:29:59.793786+00:00
parent: CLOACI-I-0117
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI app skeleton — Vite/React/TS under ui/, component library, routing, auth gate, SDK + TanStack Query wiring

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

The walking skeleton that makes every later task cheap. A Vite + React + TypeScript app under `ui/`, an off-the-shelf accessible component library, routing, the authenticated shell (left nav + connection indicator), TanStack Query wrapping `@cloacina/client`, the API-key `/connect` gate (credential in `sessionStorage`), and the shared loading/empty/error primitives every view reuses. Phase-1 foundation — nothing else in I-0117 starts until this lands.

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] `ui/` project: Vite + React + TS, ESLint (flat config) + Prettier, scripts (`dev`/`build`/`test`/`typecheck`/`lint`); depends on `@cloacina/client` via `file:../clients/typescript` (published-npm dep once the SDK ships; see status note).
- [x] Component library chosen + wired: **Mantine** (recorded below); base theme in `src/theme.ts`. Distinctive visual pass via `frontend-design` is a deliberate follow-up.
- [x] Routing (React Router) with the full IA route map; authenticated `Shell` (left nav, tenant/connection badge, disconnect) wraps all in-app routes; feature routes are wired placeholders.
- [x] Auth context (`AuthContext.tsx`): holds `{ serverUrl, apiKey, tenant }`, constructs `CloacinaClient`, persists to `sessionStorage`, gates the router via `RequireAuth`.
- [x] `/connect` manual path: validates via `client.health()` + `client.listWorkflows()` (scoped read); success enters the app, failure renders the typed error. (OIDC button = T-0662.)
- [x] TanStack Query provider + hook convention (`api/hooks.ts` query-key factory + `useClient()`); `CloacinaApiError` → typed via `classifyError` with a retry policy in `queryClient.ts`.
- [x] `Loading` / `Empty` / `ErrorState` primitives (NFR-001) + typed error-to-UI mapper (401/403 → auth, 404 → not-found, 400/422 → validation+code, 5xx/network → retry) per REQ-007.
- [x] `npm run build` + `typecheck` green; empty overview renders against a live server — **verified via `angreal ui up`** (Vite ready, server healthy, the `/connect` scoped read returns the list envelope, CORS preflight from the UI origin passes). Neither flagged risk spot (Mantine polymorphic NavLink, CSS side-effect import) tripped the build.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Standard Vite React-TS scaffold. Auth context is a small React context (not a heavy store) holding the connection + the constructed client; a route guard redirects to `/connect` when absent. Query hooks key on `[tenant, resource, ...params]`. The error mapper centralizes `CloacinaApiError` handling so views stay declarative.

### Dependencies
None — first task. Unblocks T-0652/0653/0654/0655/0658 (and transitively the rest).

### Risk Considerations
Component-library lock-in — pick one with good a11y defaults (NFR-003) and escape hatches. Keep the SDK as the *only* data path from day one (initiative goal); resist hand-fetch shortcuts that would later drift.

## Status Updates **[REQUIRED]**

**2026-06-11** — Skeleton built on branch `i0117-web-ui` (under `ui/`):
- **Component library decision: Mantine 7.** Picked over shadcn/ui (CLI-driven, needs Tailwind setup — friction for a hand-written scaffold) and bare Radix (more wiring). Mantine has strong a11y defaults (NFR-003), a full component set, and wires with one provider. Easily swapped if it grates.
- **SDK link: `file:../clients/typescript`** until `@cloacina/client` is published to npm (rides the next release). README documents "build the SDK first" (`npm run build` in clients/typescript) since the file dep resolves to its `dist/`.
- **Files:** Vite/TS config (project-refs tsconfig), PostCSS (Mantine preset), ESLint flat config, Prettier; `src/` — `main.tsx` (Mantine + Query + Auth + Router providers), `App.tsx` (route map), `auth/AuthContext` (connection + client + sessionStorage + `useClient`), `api/{errors,queryClient,hooks}`, `components/{Shell,RequireAuth,states/States}`, `routes/{Connect,Overview,Placeholder}`, a smoke test (`App.test.tsx`: unauth → connect redirect), and README.
- **Per the user's external-build preference, I did NOT run install/build in-tool.** Verification handed off: `cd clients/typescript && npm i && npm run build` then `cd ui && npm i && npm run typecheck && npm run dev`.
- **Risk spots to watch on first build:** (1) Mantine polymorphic `NavLink component={RouterNavLink}` typing for `to`/`end` props; (2) the `*.css` side-effect import under `noUncheckedSideEffectImports` (covered by `vite/client` ambient decls). Both expected-fine but the most likely tweak points.
- **Deferred within scope:** distinctive visual identity via `frontend-design` (skeleton uses a clean base theme).
