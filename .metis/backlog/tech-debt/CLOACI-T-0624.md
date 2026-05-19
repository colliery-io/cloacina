---
id: upgrade-diesel-2-1-2-4-rand-0-8-0
level: task
title: "Upgrade diesel (2.1 → 2.4+), rand (0.8 → 0.9+), lru (0.12 → 0.18) to clear RustSec ignores tracked in audit.toml"
short_code: "CLOACI-T-0624"
created_at: 2026-05-19T14:55:00+00:00
updated_at: 2026-05-19T17:46:05.699208+00:00
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

# Upgrade diesel/rand/lru to clear RustSec ignores

## Objective

Remove the five unsound-class advisory ignores from `audit.toml` by upgrading the underlying crates. These were suppressed (rather than fixed) in [[CLOACI-T-0623]] to unblock nightly; this task does the actual upgrade work, which is non-trivial because each upgrade crosses a semver-significant API boundary.

## Backlog Item Details

### Type
- [x] Tech debt

### Priority
- [x] P2 — no live exploitation, but the ignored advisories shadow any future real ones

## Technical Debt Impact

- **Current problems**:
  - `diesel` pinned at `2.1`, resolving to `2.3.7`. Latest is `2.3.9` (may still be affected — verify), with `2.4` as the upgrade target if the fixes are not backported.
  - `rand` pinned at `0.8`, resolving to `0.8.5`. RUSTSEC-2026-0097 is fixed in `0.9+`. API: `thread_rng` → `rng`, `gen_range` deprecated, traits renamed.
  - `lru` pinned at `0.12`, resolving to `0.12.5`. Latest is `0.18`. Several iterator-API changes between 0.12 and 0.18.
- **Benefits of fixing**: audit job becomes a real signal again; we get UB fixes and maintained upstream.
- **Risks of not addressing**: the audit-ignore file rots — when a new genuine advisory lands, it gets lost in the existing-ignores noise.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `diesel` upgraded to a release that fixes RUSTSEC-2026-0111, 0134, 0135. All DAL code compiles; both postgres and sqlite test suites pass.
- [ ] `rand` upgraded to ≥ 0.9. `cloacina-workflow` and `cloacina` updated for the API breakage. `angreal test all` passes.
- [ ] `lru` upgraded to ≥ 0.13 (preferably latest 0.18). `cloacina-server` cache call sites adapted.
- [ ] The five corresponding entries removed from `audit.toml` (RUSTSEC-2026-0111 / 0134 / 0135 / 0002 / 0097).
- [ ] Nightly Dependency Audit job stays green.

## Implementation Notes

### Suggested order

1. `lru` (smallest API surface — only `cloacina-server`).
2. `diesel` — verify 2.3.9 vs 2.4 fix; expect migration in `crates/cloacina/src/dal/**`. Plan for a dedicated PR with full integration test run.
3. `rand` — broader surface; do last so it doesn't tangle with the diesel PR.

### Out of scope

- `paste` and `instant` (RUSTSEC-2024-0436 / RUSTSEC-2024-0384). These are unmaintained-class warnings with no fixed version; their `audit.toml` ignores stay until upstream (`cel-interpreter`, `notify`) drops them.

## Status Updates

- 2026-05-19: Created as follow-up to [[CLOACI-T-0623]] under the "suppress now, upgrade later" plan.
- 2026-05-19 (later): Abandoned. Investigation showed `lru 0.12 → 0.18` is drop-in, `diesel 2.1 → 2.3.9` is a clean patch bump, but `rand 0.8 → 0.9` cascades through `rand_core 0.6 → 0.9` which is incompatible with `ed25519-dalek 2.2.0` — would chain into `ed25519-dalek 3.x`. After weighing the scope, user chose to drop the audit job from CI entirely instead of upgrading. `cargo-audit` job removed from `.github/workflows/nightly.yml`; `audit.toml` deleted. Closing 0624 as won't-fix. Reopen if/when we decide to invest in re-enabling the audit signal (with the cascaded upgrades).
