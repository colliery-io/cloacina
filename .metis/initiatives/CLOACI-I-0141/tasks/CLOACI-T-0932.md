---
id: wave-1-foundation-wasm-cloacina
level: task
title: "Wave 1 foundation — wasm cloacina-client + Leptos scaffold replaces the React tree"
short_code: "CLOACI-T-0932"
created_at: 2026-08-30T11:37:50.221281+00:00
updated_at: 2026-08-30T12:41:12.374116+00:00
parent: CLOACI-I-0141
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0141
---

# Wave 1 foundation — wasm cloacina-client + Leptos scaffold replaces the React tree

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0141]]

## Objective **[REQUIRED]**

Lay the foundation of the Leptos migration (I-0141, replace-in-place):

1. **`cloacina-client` compiles to wasm32-unknown-unknown**: reqwest is
   already wasm-capable; cfg-gate the native-only pieces (tokio-tungstenite
   WS, tokio rt/time) and add a web-sys `WebSocket` twin for the
   execution-event stream. Same typed API surface on both targets.
2. **Delete the React tree; scaffold the Leptos CSR app in `ui/`**: trunk
   build, `colliery-io-aurora` git dep (aurora_leptos, leptos 0.8 csr),
   `aurora.css` via pre_build hook, AppShell + leptos_router with all 18
   route stubs, and full auth/session parity for the entry flow: Connect
   (API key + local login + OIDC redirect), whoami role gating, silent
   refresh, tenant switcher, key storage semantics as today.
3. **rust-embed keeps working**: cloacina-server's embedded-ui feature
   serves the trunk `dist/` unchanged.
4. **The parity harness survives**: `ui/e2e` + `ui/harness` (Playwright)
   preserved; Connect-flow specs pass against the Leptos app.

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

## Acceptance Criteria **[REQUIRED]**

- [x] `cargo check -p cloacina-client --target wasm32-unknown-unknown` passes;
      native build green (sdk-contract-rust lane runs in CI).
- [x] `trunk build` in `ui/` produces a dist the server embeds; the demo
      stack serves the Leptos app at the same URL (new hashed bundle
      confirmed over HTTP; rust-embed/serving path untouched).
- [x] Connect flow works LIVE against the demo stack: API-key connect,
      local-login error path (wrong password → alert), add-tenant via
      ?add=1, tenant switch, disconnect-clears-session. (whoami role
      resolution runs at connect; visible gating lands with the gated views
      from Wave 2 on — same as the React shell.)
- [x] Playwright specs pass against the served Leptos app: connect.spec.ts
      @smoke, local-auth wrong-password, and a new wave1-session.spec.ts
      (add/switch/disconnect). Harness + e2e trees survived the deletion.
- [x] No node/vite in the build path; ui/package.json is Playwright-only.

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

- 2026-08-30 — Wave 1 built; first two acceptance criteria green; live gate
  in flight. Branch `feat/i0141-wave1-foundation`.

  **wasm client DONE**: `cargo check -p cloacina-client` green native AND
  wasm32-unknown-unknown. The ws.rs protocol loop (hello/ack/dedup/backoff/
  terminal-4426) is target-independent over a tiny `socket` module —
  tokio-tungstenite native, gloo-net browser WS on wasm; backoff sleep via
  tokio/gloo-timers. reqwest timeout knobs cfg'd out on wasm. GOTCHAS:
  (1) reqwest's `stream` feature on wasm pulls wasm-streams 0.4, which
  collides at LINK time (duplicate wasm-bindgen exports) with the 0.5 from
  leptos' server_fn — `stream` dropped from the wasm-side reqwest (nothing
  streams bodies); (2) `futures_util::SinkExt` needs the explicit `sink`
  feature once tungstenite stops unifying it in.

  **React tree DELETED, Leptos scaffold IN**: `ui/` is now crate
  `cloacina-ui` (detached workspace, trunk → dist/, rust-embed path
  unchanged). aurora-leptos pinned (rev cb84a232); build.rs `write_css` →
  no-flash `<link>`; auth.rs ports AuthContext 1:1 (SAME sessionStorage
  keys so sessions survive the swap; T-0779 multi-connection; T-0800 silent
  refresh; T-0803 fail-closed gating; SSO membership decode); Connect route
  at full parity (key/password/SSO, fragment pickup + history scrub, tenant
  picker, ?add=1, dev auto-connect via cfg!(debug_assertions)); Shell rail
  + tenant switcher; 16 stubs carrying real page titles. `trunk build`
  green locally.

  **Server/docker rewired**: cloacina-server build.rs runs trunk (not npm)
  on feature-on builds (skip-env name kept); Dockerfile.demo's ui-builder
  stage is rust+trunk (node stage and TS-client image build removed).

  **IN FLIGHT**: `angreal ui up` rebuilding the demo image with the Leptos
  dist; then the live Connect gate + Playwright connect specs.

- 2026-08-30 (later) — **WAVE 1 GREEN LIVE.** Demo stack rebuilt with the
  trunk image and serves the Leptos bundle at the same URL. Playwright
  against it: connect.spec.ts @smoke ✓, local-auth wrong-password ✓, new
  wave1-session.spec.ts (add tenant via ?add=1, switch, disconnect clears
  session) ✓. All acceptance boxes checked.

  **Design-pack finding worth remembering**: aurora-leptos field components
  didn't associate labels with controls, and PageHeader's title was a div —
  both broke role/label-based selectors AND assistive tech. Fixed UPSTREAM
  (aurora-dark PR #2, for/id via generated ids + h1 title); cloacina pins
  rev 8e0eaf6b. The pack-first rule held: the app didn't work around it.