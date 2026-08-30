---
id: wave-1-foundation-wasm-cloacina
level: task
title: "Wave 1 foundation — wasm cloacina-client + Leptos scaffold replaces the React tree"
short_code: "CLOACI-T-0932"
created_at: 2026-08-30T11:37:50.221281+00:00
updated_at: 2026-08-30T11:40:12.075642+00:00
parent: CLOACI-I-0141
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


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

## Acceptance Criteria **[REQUIRED]**

- [ ] `cargo check -p cloacina-client --target wasm32-unknown-unknown` passes;
      native build and the sdk-contract-rust lane unchanged/green.
- [ ] `trunk build` in `ui/` produces a dist the server embeds; the demo
      stack serves the Leptos app at the same URL with no serving-path change.
- [ ] Connect flow works LIVE against the demo stack: API-key connect,
      local-account login, whoami-driven nav gating, tenant switch, logout.
- [ ] Playwright Connect/auth specs pass against the served Leptos app;
      the harness and e2e trees survived the React deletion.
- [ ] No node_modules/vite anywhere in `ui/`'s build path (package.json
      remains only for the Playwright harness tooling if needed).

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