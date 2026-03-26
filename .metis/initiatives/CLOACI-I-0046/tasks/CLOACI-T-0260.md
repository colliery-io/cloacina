---
id: server-mode-http-scenarios-against
level: task
title: "Server mode — HTTP scenarios against cloacinactl serve"
short_code: "CLOACI-T-0260"
created_at: 2026-03-26T02:36:48.036680+00:00
updated_at: 2026-03-26T03:23:20.669085+00:00
parent: CLOACI-I-0046
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0046
---

# Server mode — HTTP scenarios against cloacinactl serve

## Parent Initiative

[[CLOACI-I-0046]]

## Objective

Update angreal performance commands to invoke the new Python bench script, and update CI workflows.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `.angreal/performance.py` rewritten: remove old Rust-based `smoke`/`trigger`/`continuous`/`execution`/`hybrid` commands
- [ ] New `angreal performance daemon` command invokes `scheduler_bench.py --mode daemon`
- [ ] New `angreal performance server` command invokes `scheduler_bench.py --mode server`
- [ ] `angreal performance smoke` runs a quick daemon bench as sanity check
- [ ] Nightly CI workflow updated if it references old scheduler-bench commands
- [ ] `angreal performance --help` shows accurate descriptions

## Implementation Notes

### Files to modify
- `.angreal/performance.py` — rewrite scheduler-bench commands
- `.github/workflows/nightly.yml` — update if it references old commands

### Dependencies
- T-0258 (daemon mode script)
- T-0259 (server mode added to script)

## Status Updates

- 2026-03-26: Rewrote `.angreal/performance.py` — replaced smoke/trigger/continuous/execution/hybrid commands with daemon/server/smoke commands that invoke Python bench script. Updated `.github/workflows/nightly.yml` scheduler-bench job to use Python script instead of Rust binary.
