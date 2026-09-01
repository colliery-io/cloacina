---
id: leptos-web-ui-migrate-the-control
level: initiative
title: "Leptos web UI — migrate the control plane off React onto Rust/WASM with Aurora Dark"
short_code: "CLOACI-I-0141"
created_at: 2026-08-30T11:09:03.898425+00:00
updated_at: 2026-09-01T17:56:15.727388+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: leptos-web-ui-migrate-the-control
---

# Leptos web UI — migrate the control plane off React onto Rust/WASM with Aurora Dark Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context **[REQUIRED]**

Maintainer decision (2026-08-30, pre-0.11.0): migrate the web UI to Leptos
before releasing. The 0.11.0 release train (PR #264, version lockstep +
changelog written) is PARKED until this ships; the openapi.json version
touchpoint fix rides the same train.

**Current state (surveyed):**
- `ui/` is a React 18 SPA: 69 TS files / ~10.2k LOC, Mantine 7, TanStack
  react-query, `@xyflow/react` + dagre for DAG views, Vite build, embedded
  into `cloacina-server` via rust-embed (I-0130, `embedded-ui` feature),
  same-origin API via the generated `@cloacina/client` TS SDK.
- **18 routes:** Connect, Overview, Workflows, WorkflowDetail, WorkflowUpload,
  Executions, ExecutionDetail, Graphs, GraphDetail, Triggers, TriggerDetail,
  Operations, Fleet, Secrets, Keys, Accounts, Settings, Placeholder.
- **26 components**, the heavy end being graph/timeline rendering: Dag,
  FullDag, MiniDag, WorkflowGraph, TaskGantt, TaskRuntimeChart, RunHeatmap,
  CombinedTimeline, EventLog — all currently xyflow/dagre/JS-chart territory.
- **Aurora Dark** (`@colliery-io/aurora-dark`, colliery-io GitHub, consumed
  by every control-plane UI): two layers. (1) A **framework-agnostic token
  layer** — CSS custom properties (`--bg --panel --ice --teal --ok --bad`,
  aurora gradients, IBM Plex fonts, scrollbars, `cl-pulse`) directly
  consumable from Leptos. (2) ~20 **React/Mantine presentational
  primitives** (Pill, Panel, PageHeader, Chip, StatusBadge, GraphHealth,
  RunCircles, Loading/Empty/ErrorState, BrandMark…) plus semantic helpers
  (statusColor, healthState, formatAgo, classifyError) — these need Leptos
  twins; the helpers are trivial ports, the primitives are small.
- **`cloacina-client` (Rust SDK) is wasm-feasible**: reqwest 0.12
  rustls/default-features-off (wasm-supported); only the WebSocket transport
  (tokio-tungstenite, native-only) needs a wasm-gated swap (web-sys/gloo).
  Version-locked to the server, contract-tested live (sdk-contract-rust lane).
- **Acceptance harness already exists**: Playwright e2e + the visual harness
  (`ui/harness`, `test:visual`) run against the served UI over HTTP — they
  are implementation-agnostic and become the parity gate for the port.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- The control plane UI is a Leptos (Rust/WASM) app styled by Aurora Dark
  tokens, served exactly as today (static assets, rust-embed, same-origin).
- `cloacina-client` becomes the single API client for CLI, SDK consumers,
  AND the UI (wasm feature) — the generated TS client stops being a UI
  dependency.
- Feature parity with the 18 routes, gated by the existing e2e + visual
  harness rather than by eyeballing.
- One less toolchain in the release path (node stays only for docs/SDK-TS).

**Non-Goals:**
- Redesigning flows or adding features during the port (parity first; any
  redesign is post-migration work).
- SSR/hydration (leptos_axum) — the UI stays a CSR SPA embedded in the
  server binary; SSR is a possible future initiative, not this one.
- Porting the Aurora REACT primitives for other consumers — other UIs keep
  the React layer; we add a Leptos layer alongside it.

## Detailed Design **[REQUIRED]**

**Maintainer decisions (2026-08-30):** Aurora Leptos layer lives in the
aurora-dark repo (it already does — see below); DAG/timeline rendering is
pure-Rust SVG (no JS interop); cutover is REPLACE IN PLACE (delete the React
tree up front, no parallel maintenance); scope is STRICT PARITY with the 18
routes.

**The design pack is further along than assumed.** colliery-io/aurora-dark
has pivoted to Leptos: crate `colliery-io-aurora` (lib `aurora_leptos`,
leptos 0.8, renderer-agnostic, CSR selected by the consuming binary) ships
IMPLEMENTED: all 27 Mantine-equivalent primitives the React UI uses (audited
against `cloacina/ui/src` with usage counts in INVENTORY.md), the layer-2
widgets (Pill/StatusBadge/Panel/PageHeader/Loading/Empty/ErrorState/Meter/
HealthPill/InputTable/NodeReadiness/...), `tokens.rs` (semantic palette,
`status_color`, `ApiError` classification), **and `graph.rs`** — a generic
node/edge model + dependency-free layered (Sugiyama-lite) layout + an
Aurora-styled SVG `Graph` component, with feeding positions from
`layout-rs`/`rust-sugiyama` as the documented escape hatch for
crossing-heavy graphs. Stylesheet ships in-crate (`AuroraStyles` runtime
injection, or `write_css`/`aurora-css` from a build hook for no-flash).
PATTERNS.md is the composition guide. **Contract: the pack renders; the app
supplies meaning** (state vocab, colors-by-status, brand mark are props).

**What cloacina builds downstream:**

1. **`ui/` becomes a Leptos CSR crate** (trunk-built; NOT a cargo workspace
   member — like the React app it builds separately, keeping leptos/web deps
   out of the server dependency tree). Same-origin API, same routes/URLs.
   `aurora.css` emitted via trunk pre_build hook (no flash). rust-embed in
   cloacina-server points at the trunk `dist/` — the embedded-ui feature and
   serving path are untouched.
2. **`cloacina-client` gains a `wasm` build**: reqwest 0.12 already supports
   wasm32-unknown-unknown; the WebSocket transport (tokio-tungstenite,
   native-only) gets a wasm twin on web-sys `WebSocket` behind cfg. The typed
   API surface — the thing the contract lane locks — is identical on both
   targets. The UI consumes THIS, not the generated TS client.
3. **State/data layer**: leptos resources + polling intervals replacing
   react-query semantics (stale-while-revalidate where the UI relied on it),
   WS execution-event streams driving the live views (EventLog, ActiveRunCard,
   StatusStrip).
4. **App-specific views** on pack primitives: the 18 routes, the DAG views on
   `aurora_leptos::graph` (workflow DAGs are small; if GraphDetail's CG view
   crosses badly, switch that one view to rust-sugiyama positions), and the
   bespoke SVG timeline/chart widgets (TaskGantt, RunHeatmap,
   TaskRuntimeChart, CombinedTimeline) — candidates to upstream into the pack
   once stable.
5. **Auth/session parity**: Connect flow (API key + local login + OIDC
   redirect), whoami-driven role gating, silent refresh, tenant switcher,
   key storage semantics identical to the React app.
6. **Parity gate**: `ui/e2e` + `ui/harness` (Playwright, HTTP-level,
   implementation-agnostic) SURVIVE the React deletion and gate the port.
   Visual specs re-baseline only where pixel output legitimately differs;
   flows/selectors must pass unmodified wherever possible (selectors move to
   role/text where they were React-class-coupled).
7. **Toolchain/release**: ui CI lanes swap node/vite for rust/trunk; npm
   leaves the release path (TS client + docs keep node where they need it);
   version-lockstep touchpoints for `ui/package.json`/harness drop out of
   `release bump` + the drift guard; 0.11.0 changelog gains the migration.

## Alternatives Considered **[REQUIRED]**

- **Parallel build + gate swap** (build ui-leptos/ alongside, flip rust-embed
  when green): rejected by maintainer — no parallel maintenance; the e2e/visual
  harness provides the safety instead.
- **SSR (leptos_axum)**: rejected for this initiative — restructures
  cloacina-server around hydration for no control-plane win; CSR keeps the
  embed/serving contract identical.
- **JS interop for graph layout** (dagre via wasm-bindgen): rejected — the
  pack's layered layout covers typical DAGs, rust-sugiyama is the escape
  hatch, and keeping ANY node_modules in the UI defeats the toolchain win.
- **Keep the generated TS client for the UI**: rejected — cloacina-client on
  wasm makes the UI a first-class consumer of the same contract-tested SDK,
  and drift between TS/Rust clients stops mattering to the UI.

## Implementation Plan **[REQUIRED]**

Waves, each PR-able; strict parity throughout; initiative = one PR per repo
convention is WAIVED here (maintainer precedent: large initiatives land in
reviewable waves — as I-0138 did across #259/#261):

1. **Foundation** — wasm `cloacina-client`; `ui/` React tree deleted and
   replaced by the Leptos scaffold (trunk, aurora-leptos git dep, AppShell,
   router, Connect + auth/session + tenant switcher); rust-embed wired;
   harness/e2e preserved and Connect-flow specs green.
2. **Core routes** — Overview, Workflows, WorkflowDetail, WorkflowUpload,
   Executions, ExecutionDetail (+ data layer, WS events, run modals).
3. **Graph + operate routes** — Graphs, GraphDetail (DAG on pack graph.rs),
   Triggers, TriggerDetail, Operations (fire/inject modals, typed-slot forms).
4. **Admin routes + charts** — Fleet, Secrets, Keys, Accounts, Settings;
   TaskGantt/RunHeatmap/TaskRuntimeChart/CombinedTimeline in SVG.
5. **Gate + release** — full e2e/visual parity run against the demo stack,
   CI lane swap (trunk), lockstep touchpoint update, docs, 0.11.0 changelog
   entry (+ the parked openapi.json version fix), release train resumes.