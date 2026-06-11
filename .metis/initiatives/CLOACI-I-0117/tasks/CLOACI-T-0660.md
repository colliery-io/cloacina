---
id: ui-seed-demo-harness-workload
level: task
title: "UI seed + demo harness — workload generator (seed + loop modes) + demo compose profile"
short_code: "CLOACI-T-0660"
created_at: 2026-06-11T02:19:03.526145+00:00
updated_at: 2026-06-11T13:28:04.784988+00:00
parent: CLOACI-I-0117
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI seed + demo harness — workload generator (seed + loop modes) + demo compose profile

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

The environment generator that makes the UI's live-streaming centerpiece testable *and* demoable. A small harness that, against a target `cloacina-server`, ensures a tenant, uploads fixture `.cloacina` packages, and drives executions — in a deterministic **seed mode** (for automated UAT) and a continuous **loop mode** (for "stand it up and watch it run"). Plus the fixtures it needs and a `docker compose` demo profile. Server-side tooling — independent of the UI build, so it can land early and in parallel.

## Acceptance Criteria **[REQUIRED]**

- [x] Harness = a small **Node driver sharing `@cloacina/client`** (`ui/harness/src/main.mjs`) — the same SDK the UI ships on. Given server URL + key + package dir: waits for health, ensures the tenant (creates non-`public`), uploads the `.cloacina` packages, drives executions. **Verified end-to-end** against a live server+compiler.
- [x] **Seed mode** — deterministic: one **completed** (slow workflow, `step_seconds=0`), one **failed** (fail fixture), one **in-flight** (slow workflow left running) run, then exits. Verified: API showed `demo_slow_workflow` Complete + `demo_fail_workflow` Failed + `demo_slow_workflow` Running.
- [x] **Loop mode** — fires on a configurable interval (`HARNESS_INTERVAL_MS`), rotating quick / watchable-slow (`HARNESS_STEP_SECONDS`) / failing runs. Verified with a ~6-tick smoke run.
- [x] **Fixtures** (new, compile-verified): `demo-slow-rust` (`demo_slow_workflow`, 5 chained steps each sleeping `step_seconds` → ~20s visible event sequence) + `demo-fail-rust` (`demo_fail_workflow`, prepare→boom `TaskError`). Reuse `examples/fixtures/*` conventions. Packed via `angreal ui build-fixtures` → `examples/fixtures/dist/*.cloacina`.
- [x] **Demo compose profile** `docker/docker-compose.demo.yml`: postgres + server (CORS) + **compiler** + one-shot **fixtures packer** (`docker/Dockerfile.fixtures`, packs paths→`/workspace` so the compiler can build them) + harness (loop) + UI image. `compose config` validates. *(Full `compose up` not run this session — the compiler/fixtures images compile the workspace and are heavy; the equivalent local stack is fully verified.)*
- [x] Documented invocation (seed vs loop, angreal / node / compose) in `ui/harness/README.md`.

> **Key finding (drives the design):** `cloacina-server` does **not** build uploaded packages — a separate `cloacina-compiler` polls the DB (`pending → success`) before the workflow registers in the runner. So `angreal ui up` now also starts a compiler (previously UI uploads would hang at `pending` forever — a real T-0651 gap), and the demo compose includes a compiler service. Execute is keyed by **workflow name** (`demo_slow_workflow`), not package name — the known platform naming-drift gap.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Prefer reusing the I-0113 `sdk-contract` server-boot harness for the server side. The driver itself can lean on `@cloacina/client` (Node) or the existing Python tooling. The slow-streaming fixture is the important new artifact — it must emit events gradually so the live view has something to animate; the failing fixture exercises the debug/failed-state UI.

### Dependencies
Independent of the UI feature tasks (it only needs a server). The **demo compose profile** depends on CLOACI-T-0659 (UI image). Consumed by CLOACI-T-0661 (automated UAT).

### Risk Considerations
The slow-streaming fixture must be slow enough to observe but not so slow it makes CI crawl — make the pacing configurable. Use fresh-DB isolation (per CLOACI-T-0649, the server ignores the dbname, so isolate at the DB-create level as the contract harness does).

## Status Updates **[REQUIRED]**

**2026-06-11 — Implemented + verified the local flow end-to-end.**
- **Fixtures:** `examples/fixtures/demo-slow-rust` + `examples/fixtures/demo-fail-rust` (Rust cdylib, mirror `fleet-slow-rust`). Both compile-verified by staging (`__WORKSPACE__`→repo) + `cargo build`, and pack-verified into `.cloacina` via cloacinactl.
- **Driver:** `ui/harness/` — `package.json` (`file:` dep on the SDK) + `src/main.mjs` (seed/loop, retry-until-registered, terminal-status polling) + `Dockerfile` (multi-stage, SDK+driver) + `README.md`.
- **Angreal:** `ui build-fixtures` (stage+pack demo fixtures → `examples/fixtures/dist/`) and `ui seed [--loop] [--server/--key/--tenant]` (run the driver; auto-builds fixtures + installs deps). `ui up` now also builds+starts `cloacina-compiler` after the server is healthy (fixes the upload-never-builds gap) and prints the seed hint.
- **Demo compose:** `docker/docker-compose.demo.yml` + `docker/Dockerfile.fixtures` + `docker/pack-demo-fixtures.sh` (one-shot packer → shared volume; compiler builds; harness loops; UI renders).
- **Live verification:** booted postgres + `cloacina-server` (:8085) + `cloacina-compiler` (:9001, `--cargo-flag build --lib`, shared target). Hit two real wrinkles and fixed both: (1) the compiler's default `--frozen` fails without a committed `Cargo.lock` → override with `build --lib`; (2) server+compiler racing migrations on a fresh DB → start the compiler *after* the server migrates (and `restart: on-failure` in compose). Seed produced Complete/Failed/Running exactly; loop fired a fast/slow/fail mix. All test processes + tmp cleaned up.
- **Gitignore:** compiled `.cloacina` + `node_modules` already covered by existing rules; nothing committed from `dist/`.