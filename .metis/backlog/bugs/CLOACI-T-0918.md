---
id: installer-and-packaging-leftovers
level: task
title: "Installer and packaging leftovers — root install.sh 404s Intel macs, stale sandbox comments, dead code"
short_code: "CLOACI-T-0918"
created_at: 2026-08-02T16:33:41.487632+00:00
updated_at: 2026-08-04T09:08:42.829774+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Installer and packaging leftovers — root install.sh 404s Intel macs, stale sandbox comments, dead code

## Objective

Sweep the small concrete leftovers the deep dive found in installers and the packaging pipeline (companions to the already-merged T-0912 fixes).

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (nice to have)

## Findings

1. TWO DIVERGENT INSTALLERS: repo-root install.sh and scripts/install.sh differ (org default, checksum optional vs mandatory) — and the ROOT one still maps x86_64-apple-darwin to a release target that was dropped from the build matrix, so Intel-mac users get a 404 mid-install. scripts/install.sh got the org fix in T-0912 (#226); the root one did not. Fix: converge on ONE installer (docs/start/install.md points at scripts/install.sh as the get.cloacina.dev artifact), delete or alias the other, and either restore the Intel-mac target or fail with a clear unsupported-platform message.
2. STALE SANDBOX NARRATIVE IN CODE: the I-0105 bwrap/landlock compiler sandbox was excised 2026-07-11, but comments still narrate the old model (e.g. crates/cloacina-compiler build.rs-handling around :756-761 claims confinement that no longer exists; the audited sandbox field honestly records "none"). Fix comments to the tenant-blast-radius model so the next reader does not trust an excised guarantee.
3. DEAD CODE: crates/cloacina/src/packaging/manifest_schema.rs is a test-only "manifest.json" schema for a format that does not exist in shipped packages — remove or move under tests to stop it being documented/trusted.
4. FFI SHELL HARDCODES input_strategy:"latest" in emitted metadata regardless of manifest (cloacina-workflow-plugin package! expansion) — honor the manifest value or emit nothing.
5. Unload rejection (bound-subscriber guard) drops the scheduler tracking state for the rejected package, and a partial load can orphan cron schedules — both leave operator-visible ghosts. Add cleanup on the rejection path and a partial-load rollback for schedules.
6. COMPILER AMBIENT ENV LEAK (DEEPDIVE register #11): post-sandbox-excision, compile-phase build.rs/proc-macros inherit the compiler's FULL environment including DATABASE_URL — the cargo spawn does no env_clear. Fix: env_clear + an explicit allowlist (PATH, CARGO_HOME, RUSTUP_HOME, TMPDIR, the injected build wiring vars) on the cargo Command. Cheap, and removes the sharpest edge of the accepted no-sandbox posture.
7. --cargo-flag REPLACES the entire default flag list including --frozen --offline (a single override silently re-enables network during the compile phase). Fix: make --cargo-flag additive; keep a separate explicit --cargo-flags-replace escape hatch for the rare full-override case.

## Acceptance Criteria

## Acceptance Criteria

- [x] One installer (root install.sh deleted; scripts/install.sh canonical per unified_release.yml); target allowlist matches the release build matrix; Intel macOS gets an explicit cargo-install pointer instead of a 404; 4 doc pages converged
- [x] No comment claims the excised sandbox (run_cargo_fetch doc, log line, config.rs dev-workspace comment); dead landlock dep removed
- [x] manifest_schema.rs deleted (654 lines) — parse_duration_str RESCUED to cloacina::packaging (it had 2 production consumers, contra the ticket's "test-only" assumption) with unit coverage preserved
- [x] input_strategy honored: FFI wire default documented as unknowable-at-emit (wire compat), host now applies manifest [metadata] input_strategy — which it NEVER did before, silently ignoring sequential; test proves manifest wins
- [x] Rejected unload retains residual tracking (retry next cycle) + partial-load rollback unregisters every created schedule row across all 3 language branches; tests for both
- [x] Compile-phase cargo env-cleared behind allowlist; tests assert DATABASE_URL absent + allowlist passthrough (phase-1 fetch deliberately inherits)
- [x] --cargo-flag additive; --cargo-flags-replace is the explicit hatch; all replace-reliant call sites migrated (demo compose, chart, angreal) + docs

## Status Updates

- 2026-08-02: Filed from the architecture deep dive (packaging-pipeline + quality-release reports; DEEPDIVE.md register medium tier). Verified against main @ 5216e632.
- 2026-08-04: DONE — merged to main in PR #232 (squash). Two findings were WORSE/different than filed and are the most valuable outcomes: (a) manifest_schema.rs was not fully dead — parse_duration_str had production consumers in packaging_bridge + python trigger; deleting blind would have broken the build, so it moved to cloacina::packaging; (b) input_strategy was ignored END-TO-END — the shell's hardcoded "latest" was only half the story; the host never read the manifest value, so packaged sequential graphs silently ran latest. Both now fixed with tests. Also excised the dead landlock dependency (pure I-0105 leftover).