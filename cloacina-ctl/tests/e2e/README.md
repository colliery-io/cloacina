# End-to-End Tests

This directory contains shell script-based end-to-end tests for cloacina-ctl.

## Test Scripts

### `server-lifecycle.sh`
Comprehensive test of the server management commands including:
- Configuration generation and validation
- Backend compatibility checking
- Server start/stop lifecycle
- Process monitoring and status reporting
- PID file management
- Memory/CPU monitoring accuracy
- JSON output format validation

## Running Tests

From the repository root:

```bash
# Run all e2e tests
./cloacina-ctl/tests/e2e/server-lifecycle.sh

# Run specific test
./cloacina-ctl/tests/e2e/server-lifecycle.sh
```

## Test Structure

These tests are designed to:
- Be self-contained (generate their own configs)
- Test actual binary functionality (not mocked)
- Verify complete workflows from CLI to process management
- Clean up after themselves
- Provide clear pass/fail indicators

## Requirements

- `cargo` and Rust toolchain
- SQLite (for database tests)
- Unix-like environment (for process management)
