---
id: design-cloacinactl-cli-interface
level: task
title: "Design cloacinactl CLI interface"
short_code: "CLOACI-T-0496"
created_at: 2026-04-16T12:38:21.811404+00:00
updated_at: 2026-04-16T12:38:21.811404+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Design cloacinactl CLI interface

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Objective

Audit and design the full `cloacinactl` CLI command surface. Today the CLI has grown organically (`serve`, `package`, `init`, etc.) without a coherent design for subcommand structure, flag conventions, output formats, or discoverability. This task is a design pass — document what exists, identify gaps, and propose a consistent interface.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (nice to have)

### Business Justification
- **User Value**: Consistent, discoverable CLI makes the platform easier to adopt. Missing commands force users into raw API calls or manual steps.
- **Effort Estimate**: M (design pass, not full implementation)

## Acceptance Criteria

## Acceptance Criteria

- [ ] Audit of all existing `cloacinactl` subcommands and flags
- [ ] Identify missing commands (e.g., graph management, instance status, package inspection)
- [ ] Propose consistent subcommand hierarchy and naming conventions
- [ ] Define output format strategy (human-readable default, `--json` flag for scripting)
- [ ] Document the proposed CLI surface in a spec or ADR

## Implementation Notes

### Areas to cover
- Server lifecycle: `serve`, `init`, config management
- Package management: `package build`, `upload`, `list`, `inspect`, `delete`
- Graph operations: `graph list`, `graph status`, `graph pause/resume`
- Tenant/key management: `tenant create`, `key create/revoke`
- Diagnostics: `health`, `metrics`, instance status
- Developer workflow: `package build --watch`, local dev mode

## Status Updates

*To be added during implementation*
