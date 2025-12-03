---
id: update-ci-workflows-for-unified
level: task
title: "Update CI workflows for unified build testing"
short_code: "CLOACI-T-0010"
created_at: 2025-11-30T02:05:40.850583+00:00
updated_at: 2025-12-03T23:36:08.495360+00:00
parent: CLOACI-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0001
---

# Update CI workflows for unified build testing

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0001]]

## Objective

Update GitHub Actions CI workflows to build and test a single unified binary against both PostgreSQL and SQLite backends, replacing the current separate build matrices.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Single build job produces dual-backend binary
- [ ] Tests run against both PostgreSQL and SQLite in same workflow
- [ ] PostgreSQL service container configured for integration tests
- [ ] SQLite in-memory/file tests work in CI
- [ ] Build time comparable or improved vs. separate builds
- [ ] Test coverage reported for both backends
- [ ] Python bindings tested with both backends
- [ ] Release artifacts include dual-backend binary

## Implementation Notes

### Technical Approach

1. **Current CI Structure Analysis**

   Review existing workflow files:
   - `.github/workflows/ci.yml` (or similar)
   - Identify separate postgres/sqlite build jobs
   - Note test matrix configuration

2. **Unified Build Job**

   ```yaml
   jobs:
     build:
       runs-on: ubuntu-latest
       steps:
         - uses: actions/checkout@v4

         - name: Install Rust
           uses: dtolnay/rust-action@stable

         - name: Build (dual backend)
           run: cargo build --release
           # Default features now include both backends

         - name: Run unit tests
           run: cargo test --lib
   ```

3. **Integration Test Job**

   ```yaml
     integration-tests:
       runs-on: ubuntu-latest
       needs: build

       services:
         postgres:
           image: postgres:15
           env:
             POSTGRES_PASSWORD: postgres
             POSTGRES_DB: cloacina_test
           ports:
             - 5432:5432
           options: >-
             --health-cmd pg_isready
             --health-interval 10s
             --health-timeout 5s
             --health-retries 5

       steps:
         - uses: actions/checkout@v4

         - name: Run PostgreSQL integration tests
           env:
             DATABASE_URL: postgres://postgres:postgres@localhost:5432/cloacina_test
           run: cargo test --test '*' -- --test-threads=1

         - name: Run SQLite integration tests
           env:
             DATABASE_URL: sqlite://./test.db
           run: cargo test --test '*' -- --test-threads=1
   ```

4. **Test Parameterization**

   Ensure integration tests can be parameterized by `DATABASE_URL` environment variable, running the same test suite against both backends.

### Files to Modify

- `.github/workflows/ci.yml` (or equivalent)
- `.github/workflows/release.yml` (if exists)
- Test configuration files if needed

### Dependencies

- Requires CLOACI-T-0008 (Feature flags refactored)
- Requires CLOACI-T-0009 (Cleanup complete)

### Risk Considerations

- CI time may increase if running sequential backend tests
- Consider parallel test jobs for each backend to maintain speed
- Ensure test isolation between backend runs

## Status Updates

*To be added during implementation*
