#!/usr/bin/env python3
"""Test just DefaultRunner creation with async DAL."""
import sys
import os

print("Testing DefaultRunner creation...")

# Enable Rust logging
os.environ['RUST_LOG'] = 'debug,cloacina=trace'
os.environ['RUST_BACKTRACE'] = '1'

try:
    print("1. Importing cloaca...")
    import cloaca
    print("   Backend: sqlite")
    
    print("2. Creating runner with SQLite WAL mode...")
    runner = cloaca.DefaultRunner("sqlite://test_runner.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000")
    print("   Runner created successfully!")
    
    print("3. Checking runner properties...")
    print(f"   Runner type: {type(runner)}")
    print("   Runner has execute method:", hasattr(runner, 'execute'))
    
    print("4. Success! DefaultRunner works with async DAL")
    
    # Clean up
    if os.path.exists("test_runner.db"):
        os.remove("test_runner.db")
    
except Exception as e:
    print(f"ERROR: {e}")
    import traceback
    traceback.print_exc()
    sys.exit(1)