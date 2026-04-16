---
id: investigate-python-execution
level: task
title: "Investigate Python execution sandbox — status and gaps vs. original hardening intent"
short_code: "CLOACI-T-0503"
created_at: 2026-04-16T17:26:59.912489+00:00
updated_at: 2026-04-16T17:26:59.912489+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Investigate Python execution sandbox — status and gaps vs. original hardening intent

## Objective

I-0051 listed "Python sandbox" as a security goal alongside auth, path traversal, and rate limits. Auth/path/rate-limit work landed through I-0083, I-0085, I-0087. It's unclear whether Python sandboxing was explicitly addressed, implicitly covered by tenant/auth isolation, or still open. This is a timeboxed investigation to produce a clear answer and, if gaps exist, concrete follow-up tasks.

## Type
- [x] Tech Debt (investigation)

## Priority
- [x] P2 — If sandboxing is actually missing, a tenant running arbitrary Python could read host filesystem, exfiltrate credentials, or escalate within the server. Needs a clear answer before 1.0.

## Technical Debt Impact

- **Current problems**: Unknown. The original `archive/cloacina-server-week1` branch claimed Python sandboxing but it's not obvious what landed on main.
- **Benefits of fixing**: Clarifies the security posture for multi-tenant server deployments; either confirms coverage or produces an actionable backlog.

## Acceptance Criteria

- [ ] Document the current isolation model for Python task/CG execution in server mode
- [ ] Identify what (if anything) restricts filesystem, network, subprocess, and env var access from tenant Python code
- [ ] Compare against the original I-0051 intent and the `archive/cloacina-server-week1` implementation
- [ ] Produce either: (a) a "confirmed covered" writeup citing the mechanism, or (b) concrete follow-up tasks for the specific gaps

## Implementation Notes

Starting points:
- Python execution happens via `cloaca` FFI bindings — check how tasks are dispatched in server mode
- Reference: `archive/cloacina-server-week1` commit `eeebd80` (original "Python sandbox" claim)
- Consider whether tenant schema isolation (T-0485) + credential protection (T-0451) is sufficient for the threat model, or whether process-level sandboxing is still wanted

## Status Updates

*To be added during implementation*
