---
id: distribution-pre-built-binaries
level: task
title: "Distribution — pre-built binaries, install script, Homebrew formula"
short_code: "CLOACI-T-0285"
created_at: 2026-03-28T15:45:22.274406+00:00
updated_at: 2026-04-03T01:33:26.802499+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Distribution — pre-built binaries, install script, Homebrew formula

## Parent Initiative

[[CLOACI-I-0057]]

## Objective

Make `cloacinactl` installable without building from source. Users should be able to download a pre-built binary, run an install script, or use Homebrew. The binary is self-contained (bundled SQLite, no external dependencies).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] GitHub Actions release workflow: on tag push, builds `cloacinactl` for linux-x86_64, linux-arm64, macos-arm64, macos-x86_64
- [ ] Binaries uploaded to GitHub Releases as `.tar.gz` archives with checksums
- [ ] Install script (`install.sh`): detects platform, downloads correct binary, places in `~/.local/bin` or `/usr/local/bin`
- [ ] Homebrew formula in `colliery-io/homebrew-tap` repo: `brew install colliery-io/tap/cloacinactl`
- [ ] `cargo install cloacinactl` works as fallback (publish to crates.io)
- [ ] Binary is self-contained — no runtime dependencies beyond libc
- [ ] `cloacinactl --version` shows version and build info

## Implementation Notes

### CI workflow
- Use `cross` or `cargo-zigbuild` for cross-compilation
- Strip binaries for size
- Generate SHA256 checksums alongside each archive
- Consider `cargo-dist` for automating the release process

### Homebrew
- Create `colliery-io/homebrew-tap` repo (or add to existing)
- Formula downloads binary from GitHub Releases
- Auto-update formula on new releases via GitHub Actions

### Depends on
- T-0278 (daemon subcommand must exist to distribute)

## Status Updates

*To be added during implementation*
