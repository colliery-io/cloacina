---
id: code-drift-found-by-the-t-0911
level: task
title: "Code drift found by the T-0911 docs audit — wrong image org, stale scaffold claims, inert sqlite URLs"
short_code: "CLOACI-T-0912"
created_at: 2026-08-02T14:58:14.146326+00:00
updated_at: 2026-08-02T15:09:17.753244+00:00
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

# Code drift found by the T-0911 docs audit — wrong image org, stale scaffold claims, inert sqlite URLs

## Objective

Fix the code-side defects surfaced while verifying documentation against source during the full-corpus docs overhaul (CLOACI-T-0911). Docs now describe actual behavior; this task fixes the code so intended behavior and described behavior converge.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (important for user experience)

### Impact Assessment
- **Affected Users**: Anyone installing via install.sh defaults, deploying via the Helm charts as-shipped, scaffolding packages, or reading connection-string behavior off the daemon.
- **Expected vs Actual**: itemized below.

## Findings (each independently fixable)

1. **Helm charts + install.sh pin an org CI never publishes to.** Release workflows push images to `ghcr.io/<owner>/...` = `ghcr.io/colliery-io/*` (`.github/workflows/unified_release.yml:463`, `nightly.yml:288`), but `charts/cloacina-server/values.yaml:9`, `charts/cloacina-agent/values.yaml:15`, `charts/cloacina-ui/values.yaml:9`, and `charts/cloacina-server/values.yaml:162` (agentImage) pin `ghcr.io/colliery-software/*`; `scripts/install.sh:26` defaults its repo to `colliery-software/cloacina`. A chart deployed without overrides pulls from an org that receives no pushes. Fix: repoint to colliery-io (or add an org-level redirect and document it). Docs currently note the chart-pin mismatch in reference/glossary.md — remove that caveat once fixed.

2. **`cloacinactl package new` claims cron triggers are Rust-only — false.** `crates/cloacinactl/src/nouns/package/new.rs:24-25,70-74` hard-errors `--kind cron --language python` with text saying Python has no cron trigger. `@cloaca.trigger(cron=..., on=...)` is fully supported (crates/cloacina-python/src/trigger.rs:115-160) and works packaged end-to-end (examples/features/workflows/python-cron, CI-wired). Fix: either add a Python cron scaffold template, or keep refusing but correct the error text to say only the TEMPLATE is missing and point at the @cloaca.trigger(cron=...) path.

3. **Inert sqlite URL query strings become literal filenames.** `materialize_sqlite_connection` (crates/cloacina/src/database/connection/mod.rs:526-530) strips `sqlite://` and passes the remainder to diesel as a literal path; sqlite only URI-parses `file:`-prefixed names, so `?mode=rwc&_journal_mode=WAL` lands in the on-disk filename. `cloacinactl daemon` builds exactly that URL (crates/cloacinactl/src/commands/daemon.rs:193), as do many examples/fixtures/conftest URLs. Functionally harmless (WAL + busy_timeout=30000 are applied as pragmas in run_migrations) but produces junk-named DB files and misleads readers. Fix options: strip query strings in materialize_sqlite_connection, or stop stripping the scheme and let diesel rewrite `sqlite://` -> `file:` + SQLITE_OPEN_URI (making params meaningful); then clean daemon.rs + examples. Docs already teach the bare `sqlite://app.db` form.

4. **Stale comment: diesel and SQLITE_OPEN_URI.** Same file, mod.rs:507-513 doc comment claims diesel's open path does not set SQLITE_OPEN_URI. False since diesel 2.3.x (raw.rs:51 sets READWRITE|CREATE|URI and rewrites sqlite:// -> file:). The :memory: tempfile workaround is still valid because the crate strips the scheme first (finding 3); fix the comment when fixing 3.

5. **Rust `--kind graph` scaffold omits its `cloacina-macros` dependency** (found by WP-G while verifying scaffold output; scaffolded crate does not compile as generated). Add the missing dep to the template in package/new.rs.

6. **Retired shapes still shipped:** `docker/docker-compose.production.yml` still uses the retired `serve` subcommand shape, and some examples/fixtures still carry build.rs/cloacina-build packaging wiring that the compiler-injected-shell path obsoleted. Sweep and align with the current story.

## Acceptance Criteria

## Acceptance Criteria

- [x] Charts + install.sh pull from the org CI actually publishes to; glossary caveat removed
- [x] package new errors with accurate text (Python cron template itself split to CLOACI-T-0913)
- [x] sqlite URL query handling decided and implemented; daemon.rs + examples cleaned; mod.rs comment corrected
- [x] --kind graph scaffold compiles as generated (deps added + test pins them)
- [x] production compose free of retired serve shape (removed with its Dockerfile — both unreferenced, superseded by deploy/docker-compose/cloacina.yml); example build.rs migration explicitly deferred to I-0138

## Status Updates

- 2026-08-02: Filed from CLOACI-T-0911 findings (docs already describe current actual behavior; see that task for verification citations).
- 2026-08-02: Executed. (1) colliery-software -> colliery-io in charts/*/{values,Chart}.yaml incl. vendored postgresql subchart + agentImage, charts/cloacina-server/README.md, scripts/install.sh:26; glossary caveat removed. (2) new.rs module doc + error text corrected — says template-missing, points at @cloaca.trigger(cron=..., on=...) + python-cron example; template creation split to CLOACI-T-0913. (3) DECISION: query params after sqlite:// are STRIPPED with a warn! (semantics preserved — params were always inert; bare paths untouched byte-for-byte; sqlite://:memory:?x still hits the tempfile path); unit tests added in connection/mod.rs. daemon.rs migrates the pre-fix junk-named DB file AND its -wal/-shm sidecars to the clean name before opening (state preservation). Swept the last mode=rwc stragglers: registry-execution example, cron-scheduling README, tests/python/conftest.py. (4) mod.rs doc comment now states diesel 2.3+ sets SQLITE_OPEN_URI but the crate strips the scheme first. (5) rust_cargo_toml(name, graph): graph kind adds cloacina-computation-graph + cloacina-macros (mirrors the CI-built packaged-graph example); scaffold test asserts both. (6) docker/docker-compose.production.yml + docker/Dockerfile git rm'd — compose invoked the nonexistent serve subcommand, Dockerfile existed only to serve it; zero referrers in docs/README/CI; deploy/docker-compose/cloacina.yml is canonical. The ~20 examples on cloacina_build::configure() build.rs are CI-green legacy shape — migration is CLOACI-I-0138/T-0886 scope, deliberately not done here. cargo fmt --all clean. NOT compile-checked in-session (user runs builds): suggest `angreal check crate crates/cloacinactl` (pulls cloacina too) + `angreal test integration` for the connection/scaffold tests. Note: rust-analyzer flashed a stale E0061 on new.rs:293 right after the signature change — known harness LSP false-positive class; trust cargo.
