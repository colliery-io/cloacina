#!/usr/bin/env python3
"""Run the data-pipeline workflow end-to-end.

This script builds and executes the workflow using the Cloacina runtime,
then validates that every task produced the expected outputs.

Exit codes:
    0 — all assertions passed
    1 — workflow failed or assertions failed
"""

import sys
import cloaca

# Register tasks inside a WorkflowBuilder context so the @task decorators
# bind to the correct namespace.
with cloaca.WorkflowBuilder("data_pipeline") as builder:
    builder.description("Data pipeline example: fetch → validate → aggregate → report")

    # Importing inside the context triggers decorator registration.
    import data_pipeline.tasks  # noqa: F401

# Execute the workflow with an in-memory SQLite backend.
runner = cloaca.DefaultRunner("sqlite://:memory:")
context = cloaca.Context({"pipeline_name": "functional-test"})

result = runner.execute("data_pipeline", context)

# ---------- Assertions ----------

ok = True


def check(condition: bool, msg: str) -> None:
    global ok
    if not condition:
        print(f"FAIL: {msg}")
        ok = False
    else:
        print(f"  ok: {msg}")


print(f"Workflow status: {result.status}")
check(result.status == "Completed", "workflow completed")

ctx = result.final_context

# fetch-data
raw = ctx.get("raw_data")
check(raw is not None, "raw_data present")
check(len(raw) == 3, f"raw_data has 3 records (got {len(raw) if raw else 0})")

# validate-data
validated = ctx.get("validated_records")
check(validated is not None, "validated_records present")
check(len(validated) == 3, f"all records valid (got {len(validated) if validated else 0})")

errors = ctx.get("validation_errors")
check(errors is not None, "validation_errors present")
check(len(errors) == 0, f"no validation errors (got {len(errors)})")

# aggregate-data
agg = ctx.get("aggregations")
check(agg is not None, "aggregations present")
check(agg["count"] == 3, f"count == 3 (got {agg.get('count')})")
expected_sum = 10.5 + 20.3 + 30.1
check(abs(agg["sum"] - expected_sum) < 0.01, f"sum ≈ {expected_sum}")
check(abs(agg["avg"] - expected_sum / 3) < 0.01, "avg correct")
check(agg["min"] == 10.5, "min == 10.5")
check(agg["max"] == 30.1, "max == 30.1")

# generate-report
report = ctx.get("report")
check(report is not None, "report present")
check("Records processed: 3" in report, "report contains record count")
check("Range:" in report, "report contains range")

# Cleanup
runner.shutdown()

if ok:
    print("\nAll checks passed.")
    sys.exit(0)
else:
    print("\nSome checks FAILED.")
    sys.exit(1)
