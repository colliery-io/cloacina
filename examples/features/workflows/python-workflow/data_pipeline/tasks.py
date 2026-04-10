"""Example data-pipeline tasks for Cloacina.

Each function decorated with ``@task`` is discovered by ``cloaca build``
and registered in the package manifest.
"""

from __future__ import annotations

import cloaca


@cloaca.task(id="fetch-data", dependencies=[])
def fetch_data(context):
    """Simulate fetching data from an external source."""
    context.set("raw_data", [
        {"id": 1, "name": "alpha", "value": 10.5},
        {"id": 2, "name": "beta", "value": 20.3},
        {"id": 3, "name": "gamma", "value": 30.1},
    ])
    return context


@cloaca.task(id="validate-data", dependencies=["fetch-data"])
def validate_data(context):
    """Validate raw data and filter bad records."""
    raw = context.get("raw_data")
    validated = []
    errors = []

    for i, item in enumerate(raw):
        if not isinstance(item.get("value"), (int, float)):
            errors.append(f"Record {i}: invalid value")
        else:
            validated.append(item)

    context.set("validated_records", validated)
    context.set("validation_errors", errors)
    return context


@cloaca.task(id="aggregate-data", dependencies=["validate-data"])
def aggregate_data(context):
    """Compute summary statistics on validated records."""
    records = context.get("validated_records")
    if not records:
        context.set("aggregations", {"count": 0, "sum": 0.0, "avg": 0.0})
        return context

    values = [r["value"] for r in records]
    context.set("aggregations", {
        "count": len(values),
        "sum": sum(values),
        "avg": sum(values) / len(values),
        "min": min(values),
        "max": max(values),
    })
    return context


@cloaca.task(id="generate-report", dependencies=["aggregate-data"])
def generate_report(context):
    """Produce a human-readable summary report."""
    agg = context.get("aggregations")
    errors = context.get("validation_errors")
    if errors is None:
        errors = []

    lines = [
        "=== Data Pipeline Report ===",
        f"Records processed: {agg['count']}",
        f"Sum: {agg['sum']:.2f}",
        f"Average: {agg['avg']:.2f}",
    ]
    if "min" in agg:
        lines.append(f"Range: {agg['min']:.2f} - {agg['max']:.2f}")

    if errors:
        lines.append(f"\nValidation errors: {len(errors)}")
        for err in errors[:5]:
            lines.append(f"  - {err}")

    context.set("report", "\n".join(lines))
    return context
