---
id: aurora-dark-ui-redesign-recreate
level: initiative
title: "Aurora Dark UI redesign — recreate the design handoff in @cloacina/ui"
short_code: "CLOACI-I-0129"
created_at: 2026-06-21T02:30:48.625025+00:00
updated_at: 2026-07-04T03:39:30.967083+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: aurora-dark-ui-redesign-recreate
---

# Aurora Dark UI redesign — recreate the design handoff in @cloacina/ui

## Context

A designer delivered a full visual redesign — **"Aurora Dark"** — of the
tenant-scoped control plane (`@cloacina/ui`) as a high-fidelity HTML prototype +
per-screen specs + 14 screenshots (the `design_handoff_cloacina_ui` bundle,
vendored at `ui/design/aurora-dark/`). It replaces the default light Mantine
skeleton with a dark, dense **"cold Nordic"** identity — slate base with aurora
accents (ice-blue / teal / violet), IBM Plex type — and makes **status-colored
task-graph (DAG) visualizations** the product's signature motif. The design
language is intentionally shared with the team's "Skadi" system.

The frontend was deliberately **frozen** during the I-0126 / I-0128 backend work
precisely so this redesign could land cleanly. It is now unfrozen.

Crucially, **two of the handoff's three "not-yet-supported" mocks are now backed
by shipped work** (PR #138):
- **View a task's source** → backed by [[CLOACI-I-0126]] T-0750 (source files on
  `WorkflowDetail`).
- **Pause / resume a run or graph** → backed by T-0749 (pause) + T-0751 (reactor
  manual fire).
- **Typed execute config** → backed by [[CLOACI-I-0128]] (`declared_params` on
  `WorkflowDetail` + the `…/interface` discovery endpoints).
- Only **agent enrollment from the UI** remains a true mock (no token endpoint yet).

## Goals & Non-Goals

**Goals:**
- Recreate the Aurora Dark design inside the existing `@cloacina/ui` app
  (React 18 + Vite + TypeScript + Mantine 7 + TanStack Query over
  `@cloacina/client` + React Router 7), matching the screenshots at high fidelity.
- Replace `src/theme.ts` with the token set (dark scheme, Plex fonts,
  accent/semantic palette) driven through Mantine theming.
- Add the shared **status-colored DAG component** (full + mini) as the signature
  element, via the existing `@xyflow/react` + `@dagrejs/dagre` deps.
- Redesign every existing route + the Connect gate against **real hooks/data**
  (the prototype's mock data + 1.5s "tick" are demo-only — do not replicate).
- Wire the now-backed features to real endpoints: task source, pause/resume,
  reactor fire, and a typed Run form from `declared_params`.

**Non-Goals:**
- New backend capabilities beyond what's shipped — **agent enrollment from the
  UI stays mocked/hidden** until a one-time-token endpoint lands.
- Changing API contracts or the data layer (consume existing hooks:
  `useExecutions`, `useExecution`, `useWorkflows`, `useGraphs`,
  `useLiveOpsMetrics`, `useLiveExecutionEvents`, …).
- A light theme (token it for later; "Light = soon" in Settings).

## Source of truth

Vendored at `ui/design/aurora-dark/`:
- **`README.md`** — per-screen spec + the full design-token table (authoritative).
- **`screenshots/01..14`** — rendered captures in flow order; the visual source
  of truth. Validate each screen against its matching screenshot.
- **`Cloacina.dc.html`** (+ `support.js`) — interactive prototype; a *reference*,
  not production code to copy.

## Detailed Design / Scope

- **Theme foundation:** surface/text/accent/semantic token set; IBM Plex Sans
  (UI) + Plex Mono (ids/code/numbers, tabular-nums); radii/shadows/layout;
  custom scrollbars. The core **status→color**, **graph-health→color**, and
  **node-kind→color** maps drive dots, pills, pips, and DAG nodes/edges.
- **App shell:** 232px sidebar — confluence brand mark + `server ready · v0.8.0`
  badge, full-width `▸ Run workflow` primary, grouped nav (Overview, Executions
  w/ running count; ORCHESTRATION: Workflows/Triggers/Graphs w/ counts; SYSTEM:
  Operations/API Keys/Settings), active-item inset accent, connection footer with
  `Disconnect ↗` → Connect gate.
- **Signature DAG component:** shared layered-DAG renderer (full 128×38 nodes +
  mini 9×9) — bezier edges, status/kind coloring, running-node pulse — reused in
  the execution task-graph (04), overview mini-DAG (01), and graph topology
  (09/02). `@xyflow/react` + Dagre, or inline SVG for the mini variants.
- **Screens:** Overview (01/02), Executions (03) + execution-detail drawer (04)
  + task-code modal (05), Workflows (06), Triggers (07), Graphs (08) + topology
  drawer (09), Operations (10) + add-agent modal (11), API Keys (12),
  Settings (13), Connect gate (14).
- **Now-backed features to wire (not mocks anymore):** task-source viewer →
  T-0750; pause/resume → T-0749; reactor fire → T-0751; typed Run form →
  I-0128 `declared_params`. Agent enrollment (11) stays a mock/hidden until a
  backend token endpoint exists.
- **Behavior:** live recolor off the delivery-WS tail (`useLiveExecutionEvents`)
  + WS-pushed Operations (`useLiveOpsMetrics`); URL-reflected client-side
  filters; bottom-center toasts; subtle hover lifts; scroll-x DAG canvases.

## UI/UX Design

Mockups + flows are the 14 screenshots and the `Cloacina.dc.html` prototype;
the per-screen specs in `ui/design/aurora-dark/README.md` give pixel-level
values to treat as the spec. Design-system integration: same token set / Plex
pairing / card-drawer-event-log patterns as "Skadi".

## Alternatives Considered

- **Ship the prototype HTML directly** — rejected: it's a high-fidelity
  reference, not production React; the app's hooks, router, and Mantine patterns
  must be preserved.
- **Build a new UI app** — rejected: `@cloacina/ui` already has the data layer,
  routing, and the DAG deps (`@xyflow/react`, `@dagrejs/dagre`, Tabler icons).
  This is a re-skin + component build, not a rewrite.

## Implementation Plan

Discovery → design with the maintainer on the two design-bearing calls (token
mapping into Mantine theming; the shared DAG component approach), then decompose.
Land as one initiative across a few PRs — **foundation first** (theme + shell +
DAG component), then screens in flow order, then live-behavior polish.

**Proposed decomposition (pending `decompose`):**
1. Theme foundation + Plex fonts + app shell (tokens, sidebar, Connect chrome).
2. Shared status/kind-colored DAG component (full + mini).
3. Overview (01/02) — metrics, health strip, active execs + graphs, recents.
4. Executions list (03) + detail drawer (04) + task-code modal (05) wired to
   the T-0750 source endpoint.
5. Workflows (06) + typed Run form (I-0128 `declared_params`) + pause/resume.
6. Triggers (07) + Graphs (08) + topology drawer (09) + reactor fire (T-0751).
7. Operations (10) + API Keys (12) + Settings (13); add-agent modal (11)
   mock/hidden pending an enrollment endpoint.
8. Live behavior, toasts, hover/scroll polish + per-screen screenshot validation.
