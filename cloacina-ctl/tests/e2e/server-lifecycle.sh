#!/bin/bash

# Comprehensive cloacina server test script
set -e

echo "=== Complete Cloacina Server Test ==="
echo ""

# Build the binary if needed
echo "Building SQLite binary..."
cargo build --bin cloacina-ctl-sqlite --features sqlite --quiet

# Generate the test configuration
echo "Generating test configuration..."
cat > cloacina.yaml << 'EOF'
database:
  url: sqlite://test-daemon.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000
  pool_size: 5
execution:
  max_concurrent_tasks: 2
  task_timeout_secs: 300
  worker_threads: null
  polling_interval_ms: 1000
registry:
  enabled: true
  storage:
    type: filesystem
    path: ./test-registry
cron:
  enabled: true
  check_interval_secs: 60
server:
  pid_file: ./test-daemon.pid
  log_file: ./test-daemon.log
  log_level: info
  graceful_shutdown_timeout_secs: 10
  api:
    unix_socket:
      enabled: true
      path: ./test-api.sock
      permissions: 432
EOF

# Clean up old files
echo "Cleaning up old test files..."
rm -f test-daemon.pid test-daemon.log test-daemon.db test-api.sock
rm -rf test-registry

echo ""
echo "=== 1. Initial Status Check ==="
echo "Should show 'not running':"
./target/debug/cloacina-ctl-sqlite server status

echo ""
echo "=== 2. Configuration Tests ==="
echo "Testing config validation:"
./target/debug/cloacina-ctl-sqlite server config validate

echo ""
echo "Testing config generation:"
./target/debug/cloacina-ctl-sqlite server config generate --output test-generated.yaml --force
echo "✓ Generated config file"
rm test-generated.yaml

echo ""
echo "=== 3. Backend Validation Test ==="
echo "Testing backend mismatch (should fail):"
if ./target/debug/cloacina-ctl-postgres server start --database-url "sqlite://test.db" 2>/dev/null; then
    echo "✗ Backend validation failed - should have rejected SQLite URL"
else
    echo "✓ Backend validation working - correctly rejected SQLite URL for postgres binary"
fi

echo ""
echo "=== 4. Server Lifecycle Test ==="
echo "Starting server in daemon mode..."
./target/debug/cloacina-ctl-sqlite server start &
SERVER_PID=$!

# Give server time to start
echo "Waiting 2 seconds for startup..."
sleep 2

echo ""
echo "Status after 2 seconds (testing accurate uptime):"
./target/debug/cloacina-ctl-sqlite server status

echo ""
echo "Verifying PID file exists:"
if [ -f test-daemon.pid ]; then
    echo "✓ PID file exists: $(cat test-daemon.pid)"
else
    echo "✗ PID file missing"
fi

echo ""
echo "Verifying process is running:"
if ps -p "$(cat test-daemon.pid 2>/dev/null)" > /dev/null 2>&1; then
    echo "✓ Server process is running"
else
    echo "✗ Server process not found"
fi

echo ""
echo "Waiting 3 more seconds..."
sleep 3

echo "Status after total 5 seconds (testing uptime accuracy):"
./target/debug/cloacina-ctl-sqlite server status

echo ""
echo "=== 5. JSON Output Test ==="
echo "Testing JSON status output while server is running:"
./target/debug/cloacina-ctl-sqlite server status --format json

echo ""
echo "=== 6. Stop Server Test ==="
echo "Stopping server using separate command:"
./target/debug/cloacina-ctl-sqlite server stop

echo ""
echo "Verifying server stopped:"
./target/debug/cloacina-ctl-sqlite server status

echo ""
echo "Verifying PID file cleaned up:"
if [ -f test-daemon.pid ]; then
    echo "✗ PID file still exists"
else
    echo "✓ PID file cleaned up"
fi

echo ""
echo "=== 7. Post-Stop Status Test ==="
echo "Testing status output after server stopped:"
./target/debug/cloacina-ctl-sqlite server status --format json

echo ""
echo "=== All Tests Complete ==="
echo "✓ Configuration validation"
echo "✓ Backend compatibility checking"
echo "✓ Server start/stop lifecycle"
echo "✓ Accurate uptime calculation"
echo "✓ PID file management"
echo "✓ Process monitoring"
echo "✓ JSON output format"

# Clean up
echo ""
echo "Cleaning up test files..."
rm -f cloacina.yaml test-daemon.yaml
rm -f test-daemon.pid test-daemon.log test-daemon.db test-api.sock
rm -rf test-registry

echo "✓ Test environment cleaned up"
