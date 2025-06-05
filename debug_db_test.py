#!/usr/bin/env python3
"""Debug script with persistent database for inspection."""
import sys
import os

print("Starting database debug test...")

# Enable Rust logging
os.environ['RUST_LOG'] = 'debug,cloacina=trace'
os.environ['RUST_BACKTRACE'] = '1'

try:
    print("Importing cloaca...")
    import cloaca
    print(f"Backend: {cloaca.get_backend()}")
    
    # Use a persistent database file for inspection
    db_path = "debug_test.db"
    if os.path.exists(db_path):
        os.remove(db_path)
    
    print(f"Creating runner with database: {db_path}")
    runner = cloaca.DefaultRunner(f"sqlite://{db_path}")
    print("Runner created - database should now exist")
    
    # Check if database file was created
    if os.path.exists(db_path):
        print(f"✓ Database file created: {db_path} ({os.path.getsize(db_path)} bytes)")
    else:
        print("✗ Database file not created")
        sys.exit(1)
    
    print("Stopping here to inspect database...")
    
except Exception as e:
    print(f"ERROR: {e}")
    import traceback
    traceback.print_exc()