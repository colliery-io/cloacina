---
id: doc-c-platform-how-to-tutorials
level: task
title: "DOC-C: Platform how-to + tutorials refresh — compiler, server, multi-tenant, signatures, profiles"
short_code: "CLOACI-T-0613"
created_at: 2026-05-18T18:19:22.776560+00:00
updated_at: 2026-05-18T18:19:22.776560+00:00
parent: CLOACI-I-0112
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0112
---

# DOC-C: Platform how-to + tutorials refresh — compiler, server, multi-tenant, signatures, profiles

## Parent Initiative

[[CLOACI-I-0112]]

## Objective

Refresh every platform-side how-to and tutorial against the May 2026 release: compiler hardening (I-0104), `cloacinactl server start` verb model (I-0098/T-0538), signature enforcement (I-0103), multi-tenant `remove_tenant` orchestration (I-0106/T-0581), Helm chart with embedded Postgres subchart (T-0610), and the `--log-retention-days` family (I-0109). Write three new how-tos (`decommission-a-tenant.md`, `require-signed-packages.md`, `use-cli-profiles.md`) and rewrite the security how-to pair (`local-development.md`, `package-signing.md`) for the I-0103 server-side enforcement model.

## Scope

### Files in cluster (~17)

| File | Effort | Headline change |
|---|---|---|
| `platform/tutorials/_index.md` | S | List proposed follow-up tutorials as "coming next" stubs |
| `platform/tutorials/01-deploy-a-server.md` | S | Note `--require-signatures` is off by default; verify CLI profile setup against `crates/cloacinactl/src/commands/config.rs`; replace `<package_id>` placeholder with deterministic value |
| `platform/how-to-guides/_index.md` | (S — DOC-A already touches index for IA-P-01) | Cross-link new how-tos after they land |
| `platform/how-to-guides/configure-multi-tenant-deployment.md` | **L** | Rewrite. `TenantDatabaseCache never evicts` (lines 183-189) is STALE post-T-0581. `Workflow execution scheduling is NOT tenant-scoped` (168-180) — verify; `TenantRunnerCache` may invalidate. Add `--require-signatures` cross-link, `--tenant-deletion-drain-timeout-s` knob, non-admin `cloacinactl key create --tenant` pattern. |
| `platform/how-to-guides/deploying-the-api-server.md` | **L** | Rewrite. `cloacinactl serve` → `cloacinactl server start` (handled in DOC-A but verify). `/auth/keys` → `/v1/auth/keys`. Reconcile PG version claim (16+ vs 14+). Add I-0103 + I-0106 + I-0109 flags. Docker example → `ghcr.io/colliery-software/cloacina-server`. Update startup banner sample. |
| `platform/how-to-guides/deploying-to-kubernetes.md` | M | T-0610: replace "Bitnami `postgresql` subchart" with "local embedded subchart at `charts/cloacina-server/charts/postgresql/`". Verify chart version vs `appVersion`. Note compiler-not-in-chart (deferred). Add missing values (`tenantDeletionDrainTimeoutS`, `logRetentionDays`, `reconcileIntervalS` if exposed). |
| `platform/how-to-guides/performance-tuning.md` | M | Remove `RecoveryManager` reference (T-0502). Replace `cloacina_pipelines_total` (old) with `cloacina_workflows_total`. Defer "Key metrics to watch" table to metrics-catalog cross-link. Add I-0099/I-0108 reactor/accumulator/WS metric callouts. |
| `platform/how-to-guides/production-deployment.md` | M | NOM: `cloacinactl serve` (lines 8, 79, 84, 114). Docker compose `command: ["serve", ...]` → `["server", "start", ...]` or remove (image entrypoint is server binary). Add `--require-signatures`, `--log-retention-days`, `--tenant-runner-cache-size` cross-links. Link to Helm/Docker-image how-tos. |
| `platform/how-to-guides/running-the-compiler.md` | S | Add `--log-retention-days`, `/metrics` endpoint, `cloacina_compiler_*` cross-link to metrics-catalog. |
| `platform/how-to-guides/running-the-daemon.md` | S | Verify `cloacinactl daemon` bare form parses (clap default). Add `--log-retention-days`. Cross-flag `cron_lost_threshold_min` not-wired-to-`_minutes` caveat. |
| `platform/how-to-guides/running-the-server-image.md` | S | DOC-A handles VER-P-04 (`0.6.0` Docker tag). Add `rdkafka` build-deps note (T-0609). Add I-0103/I-0106/I-0109 env-var pointers. Verify tag-scheme table against CI publishing workflow; remove invented columns. |
| `platform/how-to-guides/safely-unload-a-package.md` | S | Update "Caveat: restarting the server resets the `TenantDatabaseCache`" (lines 213-217) — T-0581 added eviction; restart-only path applies if the DELETE route was bypassed. |
| `platform/how-to-guides/security/local-development.md` | M | Rewrite. Verify Rust API surface against `crates/cloacina/src/security/`. GH Actions `cloacina sign` is fictional — use `cloacinactl package pack --sign <key>` (currently fail-hard per `crates/cloacinactl/src/nouns/package/pack.rs:21-32`; document the fail-hard). Add I-0103 server-side enforcement cross-link. |
| `platform/how-to-guides/security/package-signing.md` | **L** | Rewrite + split. Hybrid how-to/reference; split off API surface to a new `platform/reference/security-api.md` (deferred to a follow-up if scope creeps; otherwise leave inline with clearer headings). Add I-0103 flags (`--require-signatures`, `--verification-org-id`) prominently. Document the fail-hard CLI stub. Verify audit-log event types. |
| `platform/how-to-guides/use-cloacina-compiler-locally.md` | S | Line 107-112: `--sign` "no-op" → "fail-hard" with I-0103 reference. After DOC-H lands, update compiler-deployment cross-link from `docs/operations/compiler-deployment.md` GitHub URL to `platform/how-to-guides/compiler-deployment-runbook.md`. |
| **new**: `platform/how-to-guides/compiler-deployment-runbook.md` | (created by DOC-H) | Cross-linked from `running-the-compiler.md` here |
| **new**: `platform/how-to-guides/decommission-a-tenant.md` | **M** | New. 4-step teardown order (revoke keys → evict runner → evict DB → drop schema); `--tenant-deletion-drain-timeout-s`; hard-eviction semantics; per-step audit events; recovery if half-completes; smoke-test verification (404 on deleted tenant, not stale-pool). |
| **new**: `platform/how-to-guides/require-signed-packages.md` | **M** | New. Choose `verification_org_id`; produce first trusted public key; `--require-signatures` + `--verification-org-id` server flags (fail-fast); package upload 403 path; audit log entries; current limitation (`--sign` is fail-hard CLI stub); lockout recovery. |
| **new**: `platform/how-to-guides/use-cli-profiles.md` | S | New. `config profile set`; `config profile use`; resolution precedence (flag > `--profile` > `default_profile`); switch local daemon ↔ remote server; API key schemes (`raw`, `env:`, `file:`, `keyring:` reserved); secret rotation. |

### Cross-cluster dependencies

- **Blocked by**: DOC-A (drift sweep), DOC-B (platform reference must be correct first), DOC-H (compiler-deployment-runbook.md + metrics-catalog.md must exist for cross-links)
- **Blocks**: nothing directly; DOC-I cross-links into this cluster's how-tos but writes last

## Acceptance Criteria

- [ ] Every how-to in scope has been re-verified against current `crates/` code.
- [ ] All three new how-tos exist with concrete steps + verification step (no `{placeholder}` prose).
- [ ] `cloacinactl serve` does not appear in any how-to (DOC-A handles the sweep; this cluster verifies no regressions).
- [ ] Multi-tenant docs reflect post-T-0581 cache-eviction behavior (no stale "restart the server" guidance).
- [ ] Security how-tos reference both the library API (`SecurityConfig`) AND the server-side enforcement (`--require-signatures`, `--verification-org-id`).
- [ ] Helm how-to references the local embedded Postgres subchart (T-0610), not Bitnami.
- [ ] Every cross-link to `platform/reference/cli.md`, `http-api.md`, `metrics-catalog.md` resolves under `angreal docs:build`.
- [ ] `angreal docs:build` passes.

## Implementation Notes

### Sources

- **Audit file**: `.metis/initiatives/CLOACI-I-0112/audit-platform.md` (see `### platform/how-to-guides/*` and `### platform/tutorials/*` entries)
- **Code paths**:
  - CLI verb truth: `crates/cloacinactl/src/nouns/server/`, `daemon/`, `compiler/`, `package/`, `tenant/`, `key/`
  - Tenant teardown: `crates/cloacina-server/src/routes/tenants.rs:112-220`, `crates/cloacina-server/src/main.rs:74-83` (`--tenant-deletion-drain-timeout-s`)
  - Tenant caches: `crates/cloacina-server/src/tenant_runner_cache.rs`, `crates/cloacina-server/src/lib.rs:44-100` (`TenantDatabaseCache`)
  - search_path enforcement: `crates/cloacina/src/database/connection/mod.rs:113-160` (fail-closed per I-0106)
  - Signature verification: `crates/cloacina-server/src/lib.rs:152-200`, `crates/cloacina-server/src/main.rs:51-59`
  - Profile model: `crates/cloacinactl/src/commands/config.rs`
  - Compiler hardening: `crates/cloacina-compiler/src/config.rs:32-107`, `crates/cloacina-compiler/src/main.rs`
  - Helm chart: `charts/cloacina-server/Chart.yaml`, `values.yaml`, `charts/cloacina-server/charts/postgresql/`
- **Archived initiatives**: I-0097 (compiler service), I-0098 (CLI redesign), I-0103 (signature verification), I-0104 (compiler hardening), I-0106 (multi-tenant), I-0107 (CLI/server contract), I-0109 (metrics + log retention), I-0111 (distribution), T-0567, T-0581, T-0610

### Approach

This cluster is L-effort overall, driven by the multi-tenant + deploy-server rewrites. Suggest sequencing within the cluster:

1. **Reference verification first**: pull the current CLI noun-verb tree, server flag set, and tenants route shape into a scratch notes file. Use this as the source of truth for every doc edit.
2. **Existing-doc edits next**: rewrite `configure-multi-tenant-deployment.md` and `deploying-the-api-server.md` (the two L docs). The multi-tenant rewrite especially feeds the new `decommission-a-tenant.md`.
3. **New how-tos**: write `decommission-a-tenant.md`, `require-signed-packages.md`, `use-cli-profiles.md` once their cited references are in place.
4. **Security pair**: rewrite `local-development.md` and `package-signing.md` last, after `require-signed-packages.md` lands (so the cross-link target exists).
5. **Tutorial 01**: small edits at the end.
6. Add the three new how-tos to `platform/how-to-guides/_index.md` (DOC-A may have touched it for IA-P-01; verify).

### Risk considerations

- The "TenantDatabaseCache never evicts" claim is repeated in `http-api.md` and `configure-multi-tenant-deployment.md`. Coordinate with DOC-B writer to use identical post-T-0581 wording. Suggest a shared snippet or strong cross-link.
- `package-signing.md` rewrite is L because the doc is currently a hybrid; if scope-creep threatens, split the reference content to a new `platform/reference/security-api.md` and surface the split as a Phase 3b follow-up.
- The CLI `--sign` flag is currently fail-hard. Make sure docs say "raises an error today; sign manually until I-0103 wire-up completes" rather than implying signing works.
- Helm chart values may be edited by infra work between Phase 2 and execution; re-grep `values.yaml` at start-of-work.

## Status Updates

*To be added during implementation.*
