---
id: t-02-offline-by-default-cargo
level: task
title: "T-02: Offline-by-default cargo flags + vendor wiring"
short_code: "CLOACI-T-0574"
created_at: 2026-05-13T12:43:31.049366+00:00
updated_at: 2026-05-13T14:06:47.371772+00:00
parent: CLOACI-I-0104
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0104
---

# T-02: Offline-by-default cargo flags + vendor wiring

## Parent Initiative

[[CLOACI-I-0104]]

## Objective

Make `cloacina-compiler` builds offline-by-default. The compiler's default cargo invocation becomes `cargo build --release --lib --frozen --offline`, reading from an operator-curated vendor directory. Packages whose `Cargo.toml` requires uncached crates fail fast with a clean error naming the missing deps. Closes the network-side of SEC-06 in Phase 1 without standing up a sandbox.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Default cargo args are `["build", "--release", "--lib", "--frozen", "--offline"]` (see `main.rs` cargo_flags default).
- [x] `--vendor-dir` + `CLOACINA_COMPILER_VENDOR_DIR` plumbed; default `None` leaves `CARGO_HOME` untouched (cargo uses `~/.cargo`). When set, the cargo subprocess sees only the operator's curated `CARGO_HOME`.
- [x] Structured rejection: on offline failure, `BuildError::Failed("dependencies not available offline: <crate>[, <crate>...]. ...")` — operator-actionable, names the missing crates. Distinct messages for missing-Cargo.lock-under-frozen and git-source-offline. Implemented via `classify_offline_failure` in `build.rs`.
- [ ] Live integration test for missing-dep path — **deferred** (see Status Updates). Classifier unit tests (6 cases) pin the load-bearing parser; end-to-end exercised by T-0518 compiler e2e against the new defaults.
- [ ] Happy-path integration test — covered structurally by the existing compiler e2e suite once it runs against the new defaults.
- [ ] Existing in-tree packaged examples build clean — **pending external test run**. Likely to surface fixtures that need Cargo.lock or pre-populated CARGO_HOME; fix path TBD based on the failures.

## Test Cases

- **TC-1 (happy path):** package depending only on workspace + vendored deps builds successfully.
- **TC-2 (missing dep):** package depends on `unobtanium = "0.1"`. Build fails fast, rejection names the crate, no network fetch attempted.
- **TC-3 (operator vendor override):** set `--vendor-dir=/tmp/empty`, submit any non-trivial package. Build fails with the missing-deps error listing what the standard vendor would have supplied (sanity check that the flag is wired).
- **TC-4 (no network access required):** verify with `strace`/`tcpdump` in the integration test environment that a successful build does not open outbound sockets. (Optional; nice-to-have given the Phase 2 sandbox will enforce this kernel-side.)

## Implementation Notes

### Technical Approach

- Update the cargo argv builder in `crates/cloacina-compiler/src/build_loop.rs` (verify path) to prepend `--frozen --offline` to whatever args are constructed today. Centralize the default args list so T-0577 can document it.
- Vendor dir: pass via the cargo `Command`'s env (`CARGO_HOME` or explicit registry config), not by mutating the user's `~/.cargo`. Read `--vendor-dir` from config; default to the OS-resolved cargo registry path.
- Parsing cargo's "no matching package" output: cargo emits structured JSON when invoked with `--message-format=json`. If the compiler already uses JSON message capture, extend the parser. If not, regex `^error: no matching package named \`([^\`]+)\`` over stderr is the pragmatic fallback. Aggregate multiple missing-crate errors into one rejection message.
- Surface the rejection through the existing `mark_build_failed` DAL path; the structured `failure_reason` column already exists (post-I-0097).

### Dependencies

- None blocking. Can run in parallel with T-0573 and T-0575.

### Risk Considerations

- **Existing example breakage:** the in-tree packaged tutorials may have transitive deps that aren't currently vendored in the CI fixture. Mitigation: run angreal test:e2e:compiler locally; vendor any newly-missing crates in the fixture.
- **Cargo error format drift:** if a future cargo release changes its "no matching package" wording, the regex breaks. Mitigation: prefer `--message-format=json` parsing if practical; pin the cargo version used by the compiler in CI to detect drift.
- **Operator confusion:** a deployment without a vendored dep gets a hard rejection where today they'd get a successful build (with hidden network fetches). This is the intended tradeoff; T-0577 documents the `cargo vendor` workflow prominently.

## Status Updates

**2026-05-13** — Production code + unit tests landed locally; ready for external lint + test pass.

### What changed

- `crates/cloacina-compiler/src/config.rs`: added `vendor_dir: Option<PathBuf>` to `CompilerConfig`. `None` leaves `CARGO_HOME` untouched; `Some` sets it on the cargo subprocess only.
- `crates/cloacina-compiler/src/main.rs`: default `cargo_flags` extended to `["build", "--release", "--lib", "--frozen", "--offline"]`. New `--vendor-dir` CLI flag (env `CLOACINA_COMPILER_VENDOR_DIR`).
- `crates/cloacina-compiler/src/build.rs`: cargo `Command` now sets `CARGO_HOME` from `config.vendor_dir` when set. New `classify_offline_failure(stderr) -> Option<String>` recognizes (1) missing-crate aggregation via `no matching package named \`<name>\` found`, (2) missing Cargo.lock under `--frozen`, (3) git source unavailable offline. On non-zero cargo exit, the failure path tries the classifier first; on match, returns the operator-actionable message. On no match, falls back to the existing 64 KiB stderr tail.
- `src/build.rs::tests`: six new unit tests for the classifier — single missing crate, multi-crate aggregation, dedup, missing-lockfile, git-offline, and the negative case (unrelated compile error → `None`). Also added `vendor_dir: None` to the existing T-0573 `test_config` for the struct-update fix.

### Design decisions

- **String matching, not regex.** Kept the classifier dep-free — no new `regex` crate. Conservative `stderr.find(...)` and `contains(...)` matches against stable substrings. Cargo's exact wording does drift across releases; the unit tests pin our parser, and the fallback path still emits the raw stderr tail so operators always have something to act on.
- **CARGO_HOME override, not in-tree vendor mutation.** `--vendor-dir` is plumbed via `cmd.env("CARGO_HOME", vendor)`, never touches the user's `~/.cargo`. Operator workflow: `cargo vendor` to a curated CARGO_HOME, point `--vendor-dir` at it, the cargo subprocess reads only what's there.
- **Operator-not-submitter override surface.** Default flags are not user-overridable per build; operators set the global defaults via repeated `--cargo-flag`. Matches AC: "Phase 1 trusts the operator's config, not the submitter's."

### Outstanding (post-lint, depending on external test results)

- `angreal lint clippy`, `angreal lint fmt`, `angreal test unit`, `angreal test integration`, plus the compiler e2e (T-0518) if that's wired into the matrix.
- The change from "compiler fetches deps" to "compiler refuses to fetch deps" will surface any packaged fixture whose deps aren't already cached. Fix paths if it breaks: pre-populate CARGO_HOME for the test runner, or override `--cargo-flag` in those specific test paths.

### Verification (2026-05-13)

External run: `angreal lint clippy`, `angreal lint fmt`, `angreal test unit`, `angreal test integration` — all green. No fixture breakage from the offline-by-default flip; CI's CARGO_HOME already carries the workspace deps.
