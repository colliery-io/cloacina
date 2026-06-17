---
id: resolve-rustsec-advisory-drift
level: task
title: "Resolve RustSec advisory drift breaking nightly cargo-audit (diesel 2.3.7, rand 0.8.5, lru 0.12.5, paste, instant)"
short_code: "CLOACI-T-0623"
created_at: 2026-05-19T14:26:04+00:00
updated_at: 2026-05-19T17:45:39.294203+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Resolve RustSec advisory drift breaking nightly cargo-audit

## Objective

Get the nightly Dependency Audit job green by resolving (upgrade, patch, or `cargo audit` ignore-with-justification) the RustSec advisories below. The job has been failing recently as new advisories landed against pinned transitive deps.

## Backlog Item Details

### Type
- [x] Tech debt

### Priority
- [x] P2 — does not break product, but masks any future real advisory because the job is permanently red

### Advisories tripping nightly (run 26080699054)

| Crate | Version | Advisory | Class | Path |
|---|---|---|---|---|
| `instant` | 0.1.13 | RUSTSEC (unmaintained) | warning | `notify 7.0.0` → `cloacinactl` |
| `paste` | 1.0.15 | RUSTSEC-2024-0436 (unmaintained) | warning | `cel-interpreter 0.10.0` → `cloacina` |
| `diesel` | 2.3.7 | RUSTSEC-2026-0111 (UTF-8 corruption, SQLite) | unsound | direct |
| `diesel` | 2.3.7 | RUSTSEC-2026-0134 (padding-byte UB, MySQL) | unsound | direct (MySQL backend unused) |
| `diesel` | 2.3.7 | RUSTSEC-2026-0135 (transmute UB, SQLite debug print) | unsound | direct |
| `lru` | 0.12.5 | RUSTSEC-2026-0002 (`IterMut` stacked-borrows UB) | unsound | `cloacina-server` |
| `rand` | 0.8.5 | RUSTSEC-2026-0097 (custom logger unsoundness) | unsound | `cloacina-workflow`, `cloacina` |

### Technical Debt Impact

- **Current problems**: nightly audit job is permanently red, so it provides zero signal — a new critical advisory would not be noticed.
- **Benefits**: restore audit-job signal; quietly upgrades several transitive deps to maintained versions.
- **Risk of not addressing**: real advisories slip past unchecked.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

User chose "suppress now, upgrade later" — actual crate upgrades are tracked in [[CLOACI-T-0624]].

- [x] `audit.toml` created at workspace root listing all seven advisories with per-advisory justification comments pointing back here. `cargo audit` picks up `audit.toml` from the workspace root by default.
- [x] Follow-up tracking task created ([[CLOACI-T-0624]]) covering the diesel/rand/lru upgrades that will let us delete the unsound-class ignores.
- [ ] Nightly Dependency Audit job exits 0 for at least one consecutive run. (Pending next nightly. Will close on confirmation.)

## Implementation Notes

### Plan

1. Audit each direct-vs-transitive boundary — diesel/rand/lru are easy direct upgrades; instant/paste are transitive and only fixable upstream.
2. Add `audit.toml` (workspace root) with documented ignores for the warning-class unmaintained advisories.
3. Verify `angreal test all` + `angreal lint all` pass after upgrades.

## Status Updates

- 2026-05-19: Filed from nightly run 26080699054 triage.
- 2026-05-19: Decided with user: suppress now, upgrade later. Created `audit.toml` at workspace root ignoring all 7 advisories with per-line justification comments. Filed follow-up [[CLOACI-T-0624]] to track the actual diesel/rand/lru upgrades.
- 2026-05-19 (later): Attempted the actual upgrades. `lru 0.12 → 0.18` is drop-in. `rand 0.8 → 0.9` cascades into rand_core 0.6 → 0.9, which is incompatible with ed25519-dalek 2.2.0 (uses rand_core 0.6 traits in `SigningKey::generate`); cleanly bumping rand would chain into ed25519-dalek 3.x. After scope grew, user said: "lets just revert to our last thing and drop the compliance check". Reverted all upgrade work, deleted `audit.toml`, removed the `cargo-audit` job from `.github/workflows/nightly.yml` (and its three downstream `needs:` references). Closing 0623 and 0624 as resolved-by-removal: the audit signal will return when we choose to invest in the actual upgrades, not before.