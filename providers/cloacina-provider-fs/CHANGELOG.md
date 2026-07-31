# Changelog — cloacina-provider-fs

All notable changes to this provider. Providers version independently of
cloacina core (ADR A-0010); config-schema changes are breaking changes.

## [0.1.0] - UNRELEASED

### Added

- `read_file` / `write_file` — a WASM task suite for sandboxed file I/O.
  Capability-gated by the consumer's `grants = { fs = [...] }` at the
  `constructor!` call site; default-closed (no grant → the operation fails and
  the node fails closed). Config: `path`; `write_file` takes `contents` as a
  required param (CLOACI-T-0834/T-0837).
