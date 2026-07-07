---
id: fleet-secret-resolution-per
level: task
title: "Fleet secret resolution — per-execution HPKE envelope wrap to agent ephemeral key"
short_code: "CLOACI-T-0861"
created_at: 2026-07-07T11:52:26.213065+00:00
updated_at: 2026-07-07T11:52:26.213065+00:00
parent: CLOACI-I-0133
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0133
---

# Fleet secret resolution — per-execution HPKE envelope wrap to agent ephemeral key

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0133]]

## Objective **[REQUIRED]**

The fleet execution path (D-2/D-5/D-6) — the riskiest task. A packaged task running on a remote agent resolves secrets via **per-execution HPKE envelope wrap**: the agent generates a fresh ephemeral X25519 keypair per task claim and sends the public key with the claim; the server resolves the at-rest secret and HPKE-wraps it to that public key; the agent unwraps with its ephemeral private key into the `Secrets` accessor, memory-only, never persisted. On-wire ciphertext is bound to one agent + one execution.

**Dependencies:** T-0858 (accessor + leak guarantee it reuses), T-0860 (grant check). Highest risk in the initiative.

**Design refs:** [[CLOACI-I-0133]] D-2/D-5/D-6, NFR-003. Fleet path: `crates/cloacina-server/src/fleet_graph_executor.rs` + the agent claim protocol; lease model [[CLOACI-A-0008]]. HPKE = RFC 9180 (Rust `hpke` crate).

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

- [ ] **Spike first (first AC):** a standalone HPKE wrap→unwrap round-trip (server wraps to an ephemeral pubkey, agent unwraps) proving the crate + scheme before wiring into the protocol.
- [ ] Agent generates an ephemeral X25519 keypair per task claim and includes the public key in the claim request.
- [ ] Server resolves the at-rest secret (grant-checked via T-0860), HPKE-wraps to the claim's pubkey; the wrapped blob is what crosses the wire.
- [ ] Agent unwraps into the `Secrets` accessor (T-0858); plaintext is memory-only for the run and never persisted to disk/DB by the agent.
- [ ] Replay/isolation: a wrapped blob for agent A / execution 1 cannot be unwrapped by a different keypair (test).
- [ ] NFR-001 leak test extended to the fleet path: no plaintext in agent logs, the dispatch record, or the wire capture (only ciphertext).
- [ ] Embedded/in-process path is untouched (no envelope there — confirmed by test).

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