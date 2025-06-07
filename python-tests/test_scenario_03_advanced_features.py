"""
Scenario 3: Advanced Features Tests

This test file verifies advanced workflow features including function-based DAG topology,
complex dependency chains, trigger rules, workflow versioning, and registry management.

Uses clean_runner fixture to ensure clean state between tests.
"""

# Test scenarios to implement:
# 1. Function-based DAG topology - using function references instead of string IDs
# 2. Complex dependency chains - diamond patterns, fan-out/fan-in, multi-level chains  
# 3. Trigger rules - different execution triggers based on dependency states
# 4. Workflow versioning - content-based hashing for workflow versions
# 5. Registry management - task and workflow registry isolation between tests
# 6. Error handling - circular dependencies, invalid references, missing attributes