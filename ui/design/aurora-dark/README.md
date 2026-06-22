# Handoff: Cloacina UI — Aurora Dark Redesign

## Overview
A full visual redesign of the Cloacina web UI (the tenant-scoped control plane, `@cloacina/ui`). It replaces the default light Mantine skeleton with a dark, dense, "cold Nordic" identity — a slate base with aurora accents (ice-blue / teal / violet) and IBM Plex type — and adds first-class **status-colored task-graph (DAG) visualizations** as the product's signature motif.

It covers every existing route — Overview, Executions (+ detail), Workflows, Triggers, Graphs (+ topology), Operations, API Keys, Settings — plus a Connect gate, and **mocks three not-yet-supported features** the team wants to scope: adding an agent, viewing a task's source, and pausing a run/graph.

The visual language is intentionally shared with the "Skadi" design system the team already liked: same token set, same Plex Sans/Mono pairing, same card/drawer/event-log patterns.

## About the Design Files
The file in this bundle (`Cloacina.dc.html`) is a **design reference created in HTML** — an interactive, high-fidelity prototype showing the intended look and behavior. **It is not production code to copy directly.**

Your target environment already exists: the **`@cloacina/ui`** app — a **React 18 + Vite + TypeScript SPA** using **Mantine 7**, **TanStack Query** over `@cloacina/client`, and **React Router 7**. The task is to **recreate this design in that app**, using its established patterns:

- Replace the minimal `createTheme(...)` in `src/theme.ts` with the tokens below (dark color scheme, Plex fonts, the accent/semantic palette). Drive it through Mantine's theming (`colors`, `primaryColor`, `defaultRadius`, `fontFamily`, `MantineProvider` with `defaultColorScheme="dark"`).
- Build each screen as the existing route component (`src/routes/*`), keeping the data layer intact: `useExecutions`, `useExecution`, `useExecutionTasks`, `useExecutionEvents`, `useWorkflows`, `useWorkflow`, `useTriggers`, `useGraphs`, `useGraph`, `useLiveOpsMetrics`, etc.
- The prototype uses **hardcoded sample data and a demo "tick" timer** purely for demonstration. In the real app, all values come from the hooks above.

## Fidelity
**High-fidelity.** Colors, typography, spacing, radii, and interactions are all specified below and visible in `screenshots/`. Recreate the UI to match. Where the prototype gives pixel values, treat them as the spec. Validate each screen against the matching screenshot as you build.

---

## Design Tokens

Add to `theme.ts` (`:root`-equivalent via Mantine theme + a small CSS reset for the body background and scrollbars).

### Colors — surfaces
| Token | Hex | Use |
|---|---|---|
| `--bg` | `#0e1116` | App / main background |
| `--sidebar` | `#12161c` | Sidebar, drawers, modals, connect/gate chrome |
| `--panel` | `#161a21` | Cards, inputs, raised rows, active list rows |
| `--panel-2` | `#13171e` | Secondary cards (health strip, rollups, list items) |
| `--inset` | `#0a0c10` | Event-log / code / DAG-canvas insets |
| `--control` | `#1b2129` | Buttons, progress/pip track, pill backgrounds |
| `--border` | `#232a34` | Default card border |
| `--border-soft` | `#1d232c` | Section hairlines, subtle borders |
| `--border-fainter` | `#15191f` | List-row separators |
| `--border-control` | `#2a3340` | Input / button borders, node outlines |

### Colors — text
| Token | Hex | Use |
|---|---|---|
| `--fg` | `#e6e9ee` | Primary text |
| `--fg-bright` | `#f1f4f8` | Large headings (`#eef1f5` also used) |
| `--fg-2` | `#c3cbd5` | Secondary (`#d7dde4`, `#dce2e9`, `#cdd4dc` also used) |
| `--muted` | `#8b95a3` | Muted labels / meta |
| `--faint` | `#5b6573` | Captions, placeholders, mono meta |
| `--fainter` | `#4a525e` | Log timestamps, gutters |

### Colors — accents & semantic (status)
| Token | Hex | Use |
|---|---|---|
| `--ice` | `#7fb2ff` | **Primary accent** — running, active nav, primary buttons, accumulators, progress |
| `--brand-stroke` | `#8fbcff` | Logo stroke |
| `--teal` | `#5fd0c5` | Secondary accent, graph "strategy" badge, aurora mid |
| `--violet` | `#9d8cff` | Reactors, "scheduled" status, version badge, "criteria" badge |
| `--gold` | `#d8a657` | "cancelled" / "paused" / "warming", warnings |
| `--ok` | `#4bd07f` | completed / live / healthy |
| `--bad` | `#f06464` | failed / unreachable |

**Execution & task status → color** (this is the core mapping, used on dots, pills, pips, DAG nodes/edges):
`running → #7fb2ff` · `completed → #4bd07f` · `failed → #f06464` · `scheduled → #9d8cff` · `pending → #8b95a3` · `cancelled → #d8a657` · `paused → #8b95a3` · `skipped → #5b6573`

**Graph health → color:**
`live → #4bd07f` · `running → #4bd07f` · `warming → #d8a657` · `connecting → #7fb2ff` · `socket_only → #8b95a3` · `stopped → #8b95a3` · `paused → #d8a657`

**Graph node kind → color:** `accumulator → #7fb2ff (ice)` · `reactor → #9d8cff (violet)` · `compute node → #8b95a3/#5b6573 (muted)`

**Pills/badges** are the status color at full strength on a tinted background = the same hex with `1c` alpha suffix (e.g. `background:#7fb2ff1c`), radius 10px, 10.5px Plex Mono.

**Gradients:** running progress / aurora = `linear-gradient(90deg,#5fd0c5,#7fb2ff)`; 3-stop aurora = `linear-gradient(90deg,#5fd0c5,#7fb2ff,#9d8cff)`.

### Typography
Load **IBM Plex Sans** (400/500/600/700) and **IBM Plex Mono** (400/500/600) (Google Fonts or self-host).
- **Plex Sans** — all UI text, headings, buttons, labels.
- **Plex Mono** — run ids, task names, code, timestamps, durations, counts, paths, env vars, cron, throughput. Use `font-variant-numeric: tabular-nums` for live numbers.

| Role | Size / weight | Notes |
|---|---|---|
| Page title (h2) | 22px / 600 Sans | `#e6e9ee` |
| Drawer title (h3) | 19px / 600 Sans | `#f1f4f8` |
| Section header | 13px / 600 Sans | bottom hairline `--border-soft` |
| Metric number | 30px / 600 Sans | line-height 1, tabular-nums, colored by metric |
| Card / row title | 13–14px / 600 Sans | |
| Body | 12.5–13.5px / 400–500 Sans | |
| Mono label (uppercase) | 10–10.5px / 500 Mono | letter-spacing .07–.1em, `--muted`/`--faint` |
| Mono meta / id / path | 10.5–12px / 400 Mono | |
| Code | 12px / 400 Mono | line-height 1.65 |

### Radius / shadow / layout
- Radius: pills 10–20px; cards/inputs 9–11px; modals 13–14px; buttons 6–9px; DAG nodes 8px; pips/tags 2–4px; switch track 11–12px.
- Shadows: drawer `-20px 0 50px rgba(0,0,0,.5)`; modal/toast `0 24px 60px rgba(0,0,0,.5)` / `0 12px 32px rgba(0,0,0,.5)`.
- **Sidebar:** 232px fixed (`flex:none`), `--sidebar` bg, right border `--border-soft`, padding `18px 14px`, flex column. **Main:** `flex:1; min-width:0; overflow:hidden; position:relative` (anchors absolute drawers/modals/toast).
- Standard page padding: `22px 28px`.
- Scrollbar: 9px, thumb `#283039`, transparent track.

---

## Screens / Views
Numbers map to files in `screenshots/`.

### App shell — persistent sidebar (in every screenshot)
- **Brand:** a "confluence" mark (two short strokes flowing down into a node — an SVG of 3 lines + 3 dots, ice/teal/violet) + "Cloacina" 16px/600. Below: `● server ready · v0.8.0` (green dot + Mono 11px). Maps to the existing `HealthBadge`/`ServerHealthDot`.
- **Primary button:** `▸ Run workflow` — full-width, `--ice` bg, `#0b0d10` text, 600/13px, radius 9. (Navigates to Workflows in the mock.)
- **Nav** (existing `NAV` array in `Shell.tsx`): ungrouped **Overview**, **Executions** (with a running-count in ice); group `ORCHESTRATION` → **Workflows** (ice square + count), **Triggers** (violet square + count), **Graphs** (teal square + count); group `SYSTEM` → **Operations**, **API Keys**, **Settings**. Group labels are 10px Mono, letter-spacing .1em, `--faint`.
- **Active nav item:** bg `rgba(127,178,255,.13)`, text `--fg`, `box-shadow: inset 2px 0 0 #7fb2ff`. Inactive: `--muted`, transparent.
- **Sidebar footer** (`margin-top:auto`, top hairline): `CONNECTION` label, green dot + tenant, the server URL (Mono, ellipsis), and a `Disconnect ↗` link (→ Connect gate). Replaces the header tenant badge in the current build.

### 01 / 02 — Overview (`/`)
Operational dashboard; scrolls vertically; padding `22px 28px`.
- **Header:** "Overview" + sub `tenant {name} · last sync {n}s ago` (Mono `--faint`). Right: a clickable search pill (300px, `--panel`, border `--border`, radius 9) `⌕ Find a workflow, run, or task…` → Executions.
- **Metrics row** (4-col grid, gap 13): cards `--panel`, border `--border`, radius 10, padding `15px 16px` — uppercase Mono label, 30/600 number **colored** (Workflows `#e6e9ee`, Running `#7fb2ff`, Completed `#4bd07f`, Failed `#f06464`), Mono sub. Values derived from the hooks (counts of workflows / running execs / completed-24h / failed-24h).
- **Health strip** (6-col grid, gap 9): cards `--panel-2`, border `--border-soft`, radius 9 — 8px status dot + name (12/500) + Mono detail. Drive from `useLiveOpsMetrics` (Server, Compiler, Reconciler, Scheduler, Database, Agents).
- **Two columns** (`grid-template-columns: 1.5fr 1fr`, gap 18):
  - **Left — Active executions:** header (title + "{n} in flight" → Executions). One card per running/paused exec (`--panel`, radius 10): workflow name + status pill + **Pause/Resume** button; a **mini task-DAG** (see "DAG rendering"); Mono meta `N/M tasks · {elapsed} · on {currentTask}`. Empty state = dashed border. **Below, a `Computation graphs` sub-section** (header + "{n} active"): one card per graph — health dot + name + health label + throughput (right), a **mini topology DAG**, and `Pause/Resume` + `▸ Fire` buttons. (See 02.)
  - **Right — Recently completed:** list rows (separator `--border-fainter`): status dot (with soft glow ring) + workflow + Mono trigger + right-aligned duration (colored for failed/cancelled) + faint "ago". Click → execution drawer.

### 03 — Executions (`/executions`)
- Header "Executions" + Mono status line `{n} runs · {n} running · {n} failed`.
- **Filter bar** (bottom hairline): chips `All / Running / Completed / Failed / Scheduled` with live counts (active chip = `--ice` bg + `#0b0d10`; inactive = `--panel`, border, `--fg-2`, radius 20) + a text filter input (240px) over workflow name + run id. URL-reflect like the current `Executions.tsx`.
- **Rows** (`--panel`, radius 10): status dot + workflow (13.5/600) & run id (Mono `--faint`) + a **task-pip strip** (130px: one 6px segment per task colored by state) with `N/M tasks` caption + status pill + duration (Mono) + "ago" (Mono). Click → execution drawer.

### 04 — Execution detail (right drawer, 640px)
Opened from any exec row/card. Scrim `rgba(6,8,11,.55)` + panel `--sidebar`, left border, drawer shadow.
- **Header:** status pill + `● live` (animated dot) when running + **Pause/Resume** + **↻ Re-run** + `✕`. Title = workflow (19/600), run id (Mono). Then a 4-up meta row: `STARTED`, `ELAPSED`/`DURATION`, `TRIGGER`, `TASKS` (Mono labels + values).
- **Task graph:** legend (running/done/failed) + an `--inset` canvas holding the full **DAG** (nodes are 128×38 rounded boxes: status dot + Mono task name; edges are status-colored curves; running nodes pulse; pending/skipped dimmed). **Clicking a node opens the task-code modal (05).** Caption: "Click a task to view its source."
- **Timeline:** per-task row = task name (118px Mono) + a 14px track (`--inset`) with a status-colored bar positioned by topological column + a right-aligned duration. (Maps to `TaskGantt`.)
- **Event log:** `--inset` card, rows = Mono timestamp (`--fainter`) + message colored by kind (started/snatched → ice, completed/imported → green, failed → red, scheduled/upgrade → violet, retry → gold). (Maps to `EventLog`.)

### 05 — Task code modal *(MOCK — not supported today)*
Centered modal (620px) over the drawer. Header: status pill + task name (Mono 15/600) + duration + `✕`. Sub-row: `depends on {upstream tasks}`. Body = `--inset` code viewer with a `{task}.rs · source · read-only` bar, then numbered lines (18px faint gutter + 12px Mono code). Minimal coloring: `#[task(...)]` attribute & `async fn`/`Ok(...)` in ice, `//` comments in `--faint`, rest `#cdd4dc`. **Note for impl:** task source isn't shipped in compiled `.cloacina` packages — this view is a target; back it with whatever source/metadata the registry can expose (or hide if unavailable).

### 06 — Workflows (`/workflows`)
- Header "Workflows" + `↑ Upload package` (ice). Mono sub. Rows (`--panel`, radius 11): ice square + package name (14/600) + version badge (violet tint, `v{x}`) + `{n} tasks` + a **run-history strip** (8 small squares colored by recent run status) + `▸ Run` button. Second line: description + "updated {ago}". Click row → Executions filtered to that package.

### 07 — Triggers (`/triggers`)
- Header + Mono sub. A 7-col grid table: **Workflow** · **Type** (cron = violet tint / poll = teal tint badge) · **Schedule** (humanized cron via `cronstrue`, Mono) · **State** (enabled green / disabled faint) · **Next run** · **Last run** · a **toggle switch** (34×20 track; on = `--ice`, off = `--border-control`; 16px white knob). Maps to `Triggers.tsx` + `humanizeCron`.

### 08 — Graphs (`/graphs`)
- Header + Mono sub. Rows (`--panel`, radius 11): health dot + name (14/600) + health label (colored) + throughput (right, Mono) + `Pause/Resume` + `▸ Fire`. Second line: accumulators (Mono) `→` reactor badge (violet tint) `+` mode/strategy (`{reaction_mode} · {input_strategy}`, humanized via `explainToken`). Click → topology drawer.

### 09 — Graph topology (right drawer, 600px)
- Header: health dot + label + **mode** badge (violet) + **strategy** badge (teal) + `▸ Fire` + `Pause/Resume` + `✕`. Title = graph name, throughput sub.
- **Topology:** legend (accumulator / reactor / node) + `--inset` canvas with the augmented CG graph — **accumulators (ice) → reactor (violet) → compute nodes (muted)**, curved edges. (Mirrors `buildCgGraph` in `GraphDetail.tsx`.) **Clicking a node** shows an inline detail panel: title + kind tag + rows (accumulator → role/feeds; reactor → criteria/input-strategy/accumulators; node → role/upstream). Default hint: "Click a node to inspect its role and routing."

### 10 / 11 — Operations (`/operations`)
- Header "Operations" + `● live` pill + Mono sub. **4 metric cards** (Server / Compiler / Reconciler / Fleet) each: title + state badge + 3 Mono stat rows (label left `--faint`, value right colored). Drive from `useLiveOpsMetrics`.
- **Execution agents** section: header with a **`+ Add agent`** button + a 5-col table (Agent w/ health dot · Target triple · Capacity · Heartbeat (stale = gold/red) · Tenant).
- **11 — Add agent modal *(MOCK — not supported today)*:** centered (520px). "Register an agent" + sub. Inputs: NAME, MAX CONCURRENCY, TARGET TRIPLE. An `ENROLLMENT` block = `--inset` with a `cloacinactl agent join --server … --token … --tenant …` command (ice Mono, line-continued) + "Token {id} · expires in 15m · single use". Footer: `Cancel` + `Issue token` (ice). **Note for impl:** model as issuing a one-time enrollment token; the agent self-registers and then appears in the fleet table via the existing ops metrics.

### 12 — API Keys (`/keys`)
- Header + `+ Create key` (ice). Rows (`--panel-2`, radius 10): key glyph + name (13/600) & `{prefix} · {scopes}` (Mono) + right "last used"/"created" (Mono) + `Revoke` button (red text).

### 13 — Settings (`/settings`)
- Sections (hairline headers): **Connection** (Tenant, Server URL read-only Mono cards) · **Server** (CLOACINA_BIND_ADDR, DATABASE_URL, SECRET_KEY [green "set · encrypted"], SCHEDULER) · **Appearance** (Aurora dark = active, Light = "soon").

### 14 — Connect gate (`/connect`)
Full-screen, `radial-gradient(120% 90% at 50% -10%, #131922, #0e1116)`. Centered: brand mark + "Cloacina"; a `--sidebar` card (430px, radius 14): "Connect to a server" + sub, fields **SERVER URL / API KEY (password) / TENANT** (Mono inputs), `Connect` button (ice, full-width). Footer line `cloacina v0.8.0 · tenant-scoped control plane`. Maps to `Connect.tsx`.

---

## DAG rendering (the signature element)
Three places use the same approach — execution task-graph (full, in 04), execution mini-DAG (in 01), and graph topology (full in 09, mini in 02).

1. **Layout** is a simple layered DAG: each node has a `col` (depth) and `row` (lane). Position `x = pad + col*colGap`, `y = pad + row*rowGap`. Full graph: node 128×38, colGap 148, rowGap 62, pad 20. Mini: node 9×9 square, colGap ~50–58, rowGap ~20–22, pad 6.
2. **Edges** are cubic Béziers from a node's right-center to the target's left-center: `M x1 y1 C x1+k y1, x2-k y2, x2 y2`. Stroke: target running → ice; both endpoints completed → `rgba(75,208,127,.35)`; else `#283039`.
3. **Nodes** are colored by task status (full DAG: border = status color at ~`7a` alpha over `--panel`; mini: filled square). Running pulses (`@keyframes` opacity .45↔1); pending/skipped are dimmed.
4. For **computation graphs**, build the node set as `accumulators (col 0) → reactor (col 1) → compute nodes (col 2+)`, colored by **kind** not status.

In the React app, render this with **`@xyflow/react`** (already a dependency, used by `WorkflowGraph`/`Dag`) using a Dagre layout (`@dagrejs/dagre`, also present) and custom node/edge styles matching the tokens — or a lightweight inline SVG for the mini versions.

## Interactions & Behavior
- **Navigation:** sidebar swaps views (already `react-router`). Search pill, rollups, rows → their targets. Drawers/modals are absolute overlays inside `<main>`.
- **Live:** running executions show a pulsing `live` badge and an elapsing duration; the real app already tails the delivery WS (`useLiveExecutionEvents`) and polls task rows — recolor DAG/timeline on each transition. Operations is WS-pushed (`useLiveOpsMetrics`). The prototype's 1.5s "tick" is **demo-only**; do not replicate it.
- **Filters:** chip + text filter, client-side, URL-reflected (status/workflow/offset), as in the current routes.
- **Toasts:** bottom-center, `--control` bg, green dot + message, auto-dismiss ~2.6s. Fired on fire/pause/resume/re-run/revoke/create/upload/register.
- **Hover:** rows/cards lift subtly (border/background ~.12s). DAG/topology canvases scroll-x when wider than their container.

## State Management
Real state lives in the API, surfaced via `@cloacina/client` + TanStack Query (the prototype's local `state` is a stand-in). UI-local state (signals/useState): current route (router), exec filter + text, selected execution / selected graph / selected graph-node / selected task, paused sets (mock), add-agent modal open (mock), toast. Counts/metrics/rollups are **derived client-side** from the lists.

## Mocked (not-yet-supported) features — flagged for scoping
These are intentionally **mockups of behavior the product does not support today** — wire them to real endpoints when those land, or hide until then:
1. **Add an agent** (11) — agent enrollment/token issuance from the UI.
2. **View a task's source** (05) — source isn't in compiled packages; needs a registry/source path.
3. **Pause / resume** a running execution and a computation graph (controls in 01, 04, 08, 09).

Everything else maps onto existing routes, hooks, and data.

## Assets
- **Logo:** inline SVG "confluence" mark (3 strokes + 3 dots, ice/teal/violet on `#8fbcff` stroke) — recreate as an SVG component, no raster needed.
- **Icons:** the prototype uses a few unicode glyphs (`⌕ ✕ ▸ ↻ ↑ ⏸ → ⚿ ·`). Swap for `@tabler/icons-react` (already a dependency) — the current `Shell.tsx` already imports Tabler icons for nav.
- **Fonts:** IBM Plex Sans + IBM Plex Mono (Google Fonts / self-host).

## Files
- `Cloacina.dc.html` — the full interactive prototype (all screens + drawers + modals + toast). Primary reference. (`support.js` is included so it renders if opened directly in a browser.)
- `support.js` — runtime needed only to render the `.dc.html` prototype locally.
- `screenshots/` — rendered captures, numbered in flow order: `01-overview`, `02-overview-graphs`, `03-executions`, `04-execution-detail`, `05-task-code`, `06-workflows`, `07-triggers`, `08-graphs`, `09-graph-topology`, `10-operations`, `11-add-agent`, `12-api-keys`, `13-settings`, `14-connect`. Use these as the visual source of truth alongside the per-view specs above.

### How to view the prototype
Open `Cloacina.dc.html` in a browser (it loads `support.js` from the same folder). Click the sidebar to switch views; click an execution to open its drawer; click a task node for its code; open a graph for its topology; Operations → "+ Add agent"; the sidebar "Disconnect ↗" shows the Connect gate.
