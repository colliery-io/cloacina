---
id: move-cloacina-kafka-out-of-default
level: task
title: "Move cloacina `kafka` out of default features (Docker / minimal-deps consumers)"
short_code: "CLOACI-T-0609"
created_at: 2026-05-16T00:53:30.076692+00:00
updated_at: 2026-05-16T00:53:30.076692+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Move `kafka` out of cloacina's default features

## Type
Tech Debt

## Priority
P2 — Medium. We have a working Dockerfile workaround (`cargo build -p cloacina-server`), but the surprise factor is high and the rdkafka build cost is real.

## Current problem
`crates/cloacina/Cargo.toml`:
```toml
default = ["macros", "postgres", "sqlite", "kafka"]
```

`kafka = ["rdkafka"]`. rdkafka clones + builds librdkafka from source via a build script, which requires `cmake`, `c++`, `libsasl2-dev`, and `libssl-dev` in the builder image. Every consumer that uses cloacina without `default-features = false` drags this in, even if they never touch the stream backend.

What we hit while landing batch 2026-05:
- Dockerfile said `cargo build --release --locked --bin cloacina-server`. Cargo's workspace feature unification flipped `kafka` on for cloacina-server's dep tree, pulling rdkafka-sys, which failed because `c++` wasn't in the bookworm-slim builder.
- Workaround: change to `-p cloacina-server --bin cloacina-server` so feature resolution is scoped (commit `b2ddd29d`). Works, but masks the real footgun for anyone copying our Dockerfile pattern.

## Acceptance criteria
- [ ] Drop `kafka` from `cloacina`'s `default` features. New default:
      `default = ["macros", "postgres", "sqlite"]`.
- [ ] Anywhere internal that depended on the kafka feature being on by
      default, opt in explicitly (test crates, integration tests).
      Grep for `#[cfg(feature = "kafka")]` paths and any callers.
- [ ] Add a regression note in CHANGELOG and a one-liner in the
      cloacina-server Dockerfile comment block (since we can relax
      the `-p` workaround if we want).
- [ ] Verify `angreal check all-crates` still green.
- [ ] Verify `angreal test integration` still green (kafka tests
      should opt into the feature where needed).

## Notes
- Only internal usage of rdkafka: `cloacina::computation_graph::stream_backend` and `cloacina::computation_graph::packaging_bridge`. Both already gated behind `#[cfg(feature = "kafka")]`.
- Downstream consumers that want kafka opt in: `cloacina = { version = "0.7", features = ["kafka"] }`.

## Related
- Workaround commit: `b2ddd29d` (Dockerfile `-p cloacina-server` scope)
- [[CLOACI-T-0610]] (Helm chart tech-debt from same iteration)

## Status Updates

*To be added during implementation*
