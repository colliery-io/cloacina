#!/usr/bin/env python3
"""Test just runner creation."""
import sys
import os
import tempfile

test_env = "/Users/dstorey/Desktop/colliery/cloacina/test-env-sqlite/lib/python3.12/site-packages"
if os.path.exists(test_env):
    sys.path.insert(0, test_env)

print("1. Importing cloaca...")
import cloaca

print("2. Creating database file...")
with tempfile.NamedTemporaryFile(suffix='.db', delete=False) as tmp:
    db_path = tmp.name
print(f"   Database: {db_path}")

print("3. About to create DefaultRunner...")
print("   This might hang during initialization...")
sys.stdout.flush()

runner = cloaca.DefaultRunner(f"sqlite://{db_path}")

print("4. SUCCESS: Runner created!")
print(f"   Runner: {runner}")

print("5. Testing a simple query...")
# Just see if we can create a context
context = cloaca.Context()
print("   Context created")

print("\nRunner creation test completed successfully!")