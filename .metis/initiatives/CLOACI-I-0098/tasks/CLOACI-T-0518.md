---
id: t9-end-to-end-cli-integration
level: task
title: "T9: End-to-end CLI integration tests against a running server fixture"
short_code: "CLOACI-T-0518"
created_at: 2026-04-17T17:00:00+00:00
updated_at: 2026-04-18T01:40:13.448556+00:00
parent: CLOACI-I-0098
blocked_by: [CLOACI-T-0517]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0098
---

# T9: End-to-end CLI integration tests against a running server fixture

## Parent Initiative

CLOACI-I-0098 — cloacinactl CLI redesign

## Objective

Lock the CLI contract with real end-to-end tests. Each test spawns a `cloacina-server` fixture, runs `cloacinactl` as a subprocess, and asserts against stdout/stderr/exit-code. This is the regression harness for everything the spec defines.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Test harness spawns `cloacina-server` against a test SQLite or Postgres (mirroring existing `server-soak` fixture).
- [ ] `cloacinactl` is invoked as a subprocess in each test. No in-process shortcuts.
- [ ] Happy-path coverage for every noun-verb combination defined in the spec: package, workflow, graph, execution, trigger, tenant, key (~35 verbs total).
- [ ] Error-path coverage: unreachable server (exit 2), invalid key (exit 4), not-found resource (exit 3), invalid flags (exit 1).
- [ ] Tenant-resolution coverage: admin key requires `--tenant`, tenant key implicit, mismatched `--tenant` rejected.
- [ ] Output-format coverage: each command invoked with `-o table` (default) and `-o json`; JSON parses with `serde_json::from_str`. `-o id` tested on list commands.
- [ ] Wired into `angreal cloacina` or a new `angreal cloacinactl e2e` task.
- [ ] Runs in CI as part of the standard test matrix.

## Implementation Notes

### Harness shape

```rust
struct CliFixture {
    _server: ServerProcess,
    home: TempDir,
    profile: &'static str,
    api_key: String,
}

impl CliFixture {
    fn cmd(&self, args: &[&str]) -> std::process::Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_cloacinactl"));
        cmd.arg("--home").arg(self.home.path());
        cmd.arg("--profile").arg(self.profile);
        cmd.args(args);
        cmd
    }
}
```

`CARGO_BIN_EXE_cloacinactl` is the cargo-provided binary path.

### Server fixture

Reuse / adapt the existing `server-soak` fixture — it already handles Postgres + bootstrap key.

### Parallelism

Each test gets its own tempdir and server port (`0.0.0.0:0` + read bound port). `tempfile::TempDir` auto-cleans.

### Coverage scoreboard

Maintain a mapping table in the harness so a failing test for `workflow disable` lands on the right line of the spec.

## Status Updates

*To be added during implementation*
