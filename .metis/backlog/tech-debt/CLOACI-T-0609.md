---
id: move-cloacina-kafka-out-of-default
level: task
title: "Add rdkafka build deps to cloacina-server Dockerfile (keep kafka as a default capability)"
short_code: "CLOACI-T-0609"
created_at: 2026-05-16T00:53:30.076692+00:00
updated_at: 2026-05-16T03:17:44.987742+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Add rdkafka build deps to cloacina-server Dockerfile (keep kafka as a default capability)

## Type
Tech Debt

## Priority
P2 — Medium. Helm e2e is green via a workaround that scopes feature resolution; the right fix is making the image self-sufficient so operators don't need to re-roll the image to use kafka stream accumulators.

## Course correction
Original task framed this as "drop kafka from cloacina defaults". That's the **wrong** direction — `#[stream_accumulator(type = "kafka", ...)]` is a first-class accumulator type in our macro surface, and operators reasonably expect `cloacina-server` to handle workflows that use kafka without re-rolling the image. Same posture as the "python is core" rule: fix the deps, not the feature.

## Current state
Batch 2026-05 Helm e2e iteration hit:
```
error: failed to run custom build command for rdkafka-sys v4.10.0
  command 'c++ --version' failed
```
Workaround in `Dockerfile` (commit `b2ddd29d`): scope cargo build with
`-p cloacina-server --bin cloacina-server` so workspace feature
unification stops activating cloacina's `kafka` default feature.

The workaround ships a server image with NO kafka support. Any
workflow that registers a kafka stream accumulator gets a runtime
"feature not enabled" error.

## Acceptance criteria
- [x] Add to Dockerfile builder stage: `build-essential` (for c++),
      `cmake`, `libsasl2-dev`, `libssl-dev`. rdkafka-sys clones +
      compiles librdkafka from source; these are its prereqs.
- [x] Add to Dockerfile runtime stage: `libsasl2-2`, `libssl3` so
      librdkafka can dyn-link SASL + SSL at connect time.
- [x] Revert the `-p cloacina-server` scope in the cargo build step.
      Use `cargo build --release --locked --bin cloacina-server` so
      kafka (and any future default features) come along.
- [x] Leave `cloacina/Cargo.toml` defaults at
      `["macros", "postgres", "sqlite", "kafka"]`.
- [ ] Verify `docker build` succeeds locally.
- [ ] Verify `angreal helm test` (kind e2e) passes with the new image.
- [ ] Confirm CI helm-chart-e2e green.

## Out of scope
- Splitting kafka out into a separate `cloacina-server-minimal` image.
  If that's wanted, file a separate task.
- Changing cloacina's library-level default features. Library consumers
  who don't want kafka can keep using `default-features = false`.

## Notes
- librdkafka built by rdkafka-sys statically bundles into the binary,
  but its SSL/SASL paths still dyn-link the system libs. That's why
  runtime gets `libsasl2-2` + `libssl3`.
- Image size delta: ~25 MB unpacked in builder (cmake + headers), ~3 MB
  in runtime (libsasl2-2 + libssl3). Net runtime image stays ~200 MB
  compressed.

## Related
- Workaround commit being reverted: `b2ddd29d` (Dockerfile `-p cloacina-server` scope)
- [[CLOACI-T-0610]] (Helm chart tech-debt from same iteration)

## Status Updates

**2026-05-16** — Flipped task direction after feedback: kafka is a core
capability via `#[stream_accumulator(type = "kafka", ...)]`, so the
right fix is build-deps in the image, not opt-in features. Applied the
Dockerfile changes locally. Pending verification + CI.
