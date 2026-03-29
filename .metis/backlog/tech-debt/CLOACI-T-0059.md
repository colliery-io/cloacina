---
id: review-cli-nomenclature-and
level: task
title: "Review CLI nomenclature and subcommand organization"
short_code: "CLOACI-T-0059"
created_at: 2026-01-28T14:14:05.539285+00:00
updated_at: 2026-03-29T12:13:54.840234+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Review CLI nomenclature and subcommand organization

## Objective

Audit and standardize the Cloacina CLI structure for consistency, discoverability, and alignment with common CLI patterns.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P3 - Low (when time permits)

### Technical Debt Impact
- **Current Problems**: CLI is growing organically with package signing, key management, server commands. Risk of inconsistent naming.
- **Benefits of Fixing**: Better developer experience, easier to learn, easier to document.
- **Risk Assessment**: Low risk of not addressing immediately - can review after more CLI features exist.

## Areas to Review

- Subcommand naming conventions (verbs vs nouns)
- Flag naming consistency (`--key` vs `--key-id` vs `--key-file`)
- Output formats (JSON, table, plain text)
- Help text quality and examples
- Discoverability of related commands
- Alignment with common CLI patterns (kubectl, gh, docker)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Document current CLI structure
- [ ] Identify inconsistencies
- [ ] Propose standardized naming conventions
- [ ] Update CLI to match conventions
- [ ] Update documentation

## Status Updates

**2026-03-29**: Complete. Binary renamed, CLI restructured kubectl-style, config get/set/list added.

### Final CLI structure:
```
cloacinactl [--home <PATH>] [-v]
├── daemon --watch-dir ... --poll-interval N
├── config get <key>
├── config set <key> <value>
├── config list
└── admin cleanup-events [--database-url <URL>] --older-than 90d [--dry-run]
```
