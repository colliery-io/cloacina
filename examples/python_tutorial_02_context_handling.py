#!/usr/bin/env python3
"""
Cloacina Python Tutorial 02: Context Handling

This example demonstrates advanced data flow and context management
in Python workflows, including data transformation pipelines, validation,
and complex data structures.

Learning objectives:
- Understand the Context data model
- Work with different data types in Context
- Implement data transformation pipelines
- Handle context serialization and type safety
- Apply best practices for data flow

Usage:
    python python_tutorial_02_context_handling.py

Prerequisites:
    pip install cloaca[sqlite]
"""

import cloaca
from datetime import datetime

# Data transformation pipeline
@cloaca.task(id="extract_data")
def extract_data(context):
    """Extract raw data from a simulated source."""
    print("Extracting data...")

    # Simulate raw data extraction
    raw_data = {
        "users": [
            {"id": 1, "name": "Alice", "email": "alice@example.com", "score": 85},
            {"id": 2, "name": "Bob", "email": "bob@example.com", "score": 92},
            {"id": 3, "name": "Charlie", "email": "charlie@example.com", "score": 78},
            {"id": 4, "name": "Diana", "email": "diana@example.com", "score": 95}
        ],
        "extracted_at": datetime.now().isoformat(),
        "source": "user_database"
    }

    context.set("raw_data", raw_data)
    context.set("extraction_complete", True)

    print(f"Extracted {len(raw_data['users'])} users")
    return context

@cloaca.task(id="transform_data", dependencies=["extract_data"])
def transform_data(context):
    """Transform the raw data into a processed format."""
    print("Transforming data...")

    # Get raw data from previous task
    raw_data = context.get("raw_data")

    if not raw_data:
        raise ValueError("No raw data found in context")

    # Transform data
    transformed_users = []
    total_score = 0

    for user in raw_data["users"]:
        # Calculate grade based on score
        score = user["score"]
        if score >= 90:
            grade = "A"
        elif score >= 80:
            grade = "B"
        elif score >= 70:
            grade = "C"
        else:
            grade = "F"

        transformed_user = {
            "user_id": user["id"],
            "display_name": user["name"].upper(),
            "email_domain": user["email"].split("@")[1],
            "score": score,
            "grade": grade,
            "performance": "high" if score >= 85 else "standard"
        }

        transformed_users.append(transformed_user)
        total_score += score

    # Create summary statistics
    transformation_result = {
        "users": transformed_users,
        "summary": {
            "total_users": len(transformed_users),
            "average_score": total_score / len(transformed_users),
            "high_performers": len([u for u in transformed_users if u["performance"] == "high"]),
            "grade_distribution": {
                "A": len([u for u in transformed_users if u["grade"] == "A"]),
                "B": len([u for u in transformed_users if u["grade"] == "B"]),
                "C": len([u for u in transformed_users if u["grade"] == "C"]),
                "F": len([u for u in transformed_users if u["grade"] == "F"])
            }
        },
        "transformed_at": datetime.now().isoformat()
    }

    context.set("transformed_data", transformation_result)
    context.set("transformation_complete", True)

    print(f"Transformed {len(transformed_users)} users")
    print(f"Average score: {transformation_result['summary']['average_score']:.1f}")

    return context

@cloaca.task(id="validate_data", dependencies=["transform_data"])
def validate_data(context):
    """Validate the transformed data meets quality standards."""
    print("Validating data...")

    transformed_data = context.get("transformed_data")

    if not transformed_data:
        raise ValueError("No transformed data found in context")

    validation_results = {
        "total_records": len(transformed_data["users"]),
        "validation_checks": {},
        "errors": [],
        "warnings": []
    }

    # Validation checks
    users = transformed_data["users"]

    # Check 1: All users have required fields
    required_fields = ["user_id", "display_name", "email_domain", "score", "grade"]
    for user in users:
        for field in required_fields:
            if field not in user:
                validation_results["errors"].append(
                    f"User {user.get('user_id', 'unknown')} missing field: {field}"
                )

    # Check 2: Score ranges are valid
    for user in users:
        score = user.get("score", 0)
        if not (0 <= score <= 100):
            validation_results["errors"].append(
                f"User {user['user_id']} has invalid score: {score}"
            )

    # Check 3: Grade consistency
    for user in users:
        score = user.get("score", 0)
        grade = user.get("grade", "")
        expected_grade = "A" if score >= 90 else "B" if score >= 80 else "C" if score >= 70 else "F"
        if grade != expected_grade:
            validation_results["errors"].append(
                f"User {user['user_id']} grade mismatch: expected {expected_grade}, got {grade}"
            )

    # Check 4: Data quality warnings
    summary = transformed_data["summary"]
    if summary["average_score"] < 75:
        validation_results["warnings"].append("Average score is below 75")

    validation_results["validation_checks"] = {
        "required_fields": "passed" if not any("missing field" in error for error in validation_results["errors"]) else "failed",
        "score_ranges": "passed" if not any("invalid score" in error for error in validation_results["errors"]) else "failed",
        "grade_consistency": "passed" if not any("grade mismatch" in error for error in validation_results["errors"]) else "failed"
    }

    # Overall validation status
    validation_status = "passed" if not validation_results["errors"] else "failed"
    validation_results["status"] = validation_status

    context.set("validation_results", validation_results)
    context.set("validation_complete", True)
    context.set("data_valid", validation_status == "passed")

    print(f"Validation {validation_status}")
    if validation_results["errors"]:
        print(f"Errors found: {len(validation_results['errors'])}")
    if validation_results["warnings"]:
        print(f"Warnings: {len(validation_results['warnings'])}")

    return context

@cloaca.task(id="generate_report", dependencies=["validate_data"])
def generate_report(context):
    """Generate a final report combining all data."""
    print("Generating report...")

    # Gather all data from context
    raw_data = context.get("raw_data")
    transformed_data = context.get("transformed_data")
    validation_results = context.get("validation_results")

    # Create comprehensive report
    report = {
        "report_metadata": {
            "generated_at": datetime.now().isoformat(),
            "workflow_id": context.get("tutorial", "02"),
            "report_type": "user_data_processing"
        },
        "data_summary": {
            "source_info": {
                "source": raw_data["source"],
                "extracted_at": raw_data["extracted_at"],
                "total_records": len(raw_data["users"])
            },
            "processing_summary": transformed_data["summary"],
            "validation_summary": {
                "status": validation_results["status"],
                "checks_performed": len(validation_results["validation_checks"]),
                "errors": len(validation_results["errors"]),
                "warnings": len(validation_results["warnings"])
            }
        },
        "processed_users": transformed_data["users"],
        "quality_metrics": validation_results
    }

    context.set("final_report", report)
    context.set("report_complete", True)

    print(f"Report generated with {len(report['processed_users'])} user records")
    return context

# Create the workflow
def create_data_pipeline_workflow():
    """Build the complete data transformation pipeline."""
    builder = cloaca.WorkflowBuilder("data_pipeline")
    builder.description("Complete data extraction, transformation, validation, and reporting pipeline")

    # Add tasks in dependency order
    builder.add_task("extract_data")
    builder.add_task("transform_data")
    builder.add_task("validate_data")
    builder.add_task("generate_report")

    return builder.build()

# Register the workflow
cloaca.register_workflow_constructor("data_pipeline", create_data_pipeline_workflow)

# Main execution
if __name__ == "__main__":
    print("=== Cloacina Python Tutorial 02: Context Handling ===")
    print()
    print("This tutorial demonstrates:")
    print("- Complex data transformation pipelines")
    print("- Context data flow between tasks")
    print("- Data validation and quality checks")
    print("- Report generation from accumulated context")
    print("- Best practices for context management")
    print()

    # Create runner
    runner = cloaca.DefaultRunner("sqlite:///python_tutorial_02.db")

    # Create initial context with metadata
    context = cloaca.Context({
        "tutorial": "02",
        "pipeline_name": "user_data_processing",
        "started_by": "tutorial_user"
    })

    # Execute the workflow
    print("Executing data pipeline workflow...")
    result = runner.execute("data_pipeline", context)

    # Display results
    print(f"\nWorkflow Status: {result.status}")

    if result.status == "Completed":
        print("Success! Data pipeline completed.")

        # Access specific results from context
        final_context = result.final_context
        report = final_context.get("final_report")

        if report:
            print("\n=== Pipeline Results ===")
            print(f"Total users processed: {report['data_summary']['source_info']['total_records']}")
            print(f"Average score: {report['data_summary']['processing_summary']['average_score']:.1f}")
            print(f"High performers: {report['data_summary']['processing_summary']['high_performers']}")
            print(f"Validation status: {report['data_summary']['validation_summary']['status']}")

            # Show grade distribution
            grade_dist = report['data_summary']['processing_summary']['grade_distribution']
            print(f"Grade distribution: A={grade_dist['A']}, B={grade_dist['B']}, C={grade_dist['C']}, F={grade_dist['F']}")

            # Show sample processed user
            if report['processed_users']:
                print("\nSample processed user:")
                sample_user = report['processed_users'][0]
                print(f"  {sample_user['display_name']} ({sample_user['email_domain']})")
                print(f"  Score: {sample_user['score']}, Grade: {sample_user['grade']}")
                print(f"  Performance: {sample_user['performance']}")

    else:
        print(f"Pipeline failed with status: {result.status}")
        if hasattr(result, 'error'):
            print(f"Error: {result.error}")

    # Cleanup
    print("\nCleaning up...")
    runner.shutdown()
    print("Tutorial 02 completed!")
    print()
    print("Key concepts demonstrated:")
    print("- Complex data structures in context")
    print("- Multi-stage data transformation")
    print("- Data validation patterns")
    print("- Report aggregation from context")
    print()
    print("Next steps:")
    print("- Try python_tutorial_03_error_handling.py for resilient workflows")
    print("- Experiment with different validation rules")
    print("- Add your own transformation steps")
