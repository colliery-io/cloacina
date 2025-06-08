#!/usr/bin/env python3
"""
Cloacina Python Tutorial 03: Error Handling and Recovery

This example demonstrates building resilient workflows with comprehensive
error handling strategies, retry mechanisms, fallback patterns,
and debugging techniques.

Learning objectives:
- Implement robust error handling in tasks
- Configure retry policies for transient failures
- Design fallback strategies and recovery patterns
- Handle validation errors and debugging
- Build resilient workflows with graceful degradation
- Monitor and respond to workflow failures

Usage:
    python python_tutorial_03_error_handling.py

Prerequisites:
    pip install cloaca[sqlite]
"""

import sys
import cloaca
import random
import time
from datetime import datetime

# Simulate external service that can fail
class UnreliableExternalService:
    """Simulates an external service with configurable failure rates."""

    def __init__(self, failure_rate=0.3):
        self.failure_rate = failure_rate
        self.call_count = 0

    def fetch_data(self, data_id):
        """Fetch data with potential for failure."""
        self.call_count += 1

        if random.random() < self.failure_rate:
            if random.random() < 0.5:
                raise ConnectionError(f"Network timeout fetching data {data_id}")
            else:
                raise ValueError(f"Invalid data ID: {data_id}")

        # Simulate processing time
        time.sleep(0.1)

        return {
            "id": data_id,
            "data": f"important_data_{data_id}",
            "timestamp": datetime.now().isoformat(),
            "fetch_attempt": self.call_count
        }

# Create service instance
external_service = UnreliableExternalService(failure_rate=0.4)

# Tasks demonstrating different error handling patterns
@cloaca.task(id="fetch_external_data")
def fetch_external_data(context):
    """Fetch data from external service with basic error handling."""
    print("Fetching data from external service...")

    data_ids = context.get("data_ids", ["data_001", "data_002", "data_003"])
    fetched_data = []
    errors = []

    for data_id in data_ids:
        try:
            # Attempt to fetch data
            result = external_service.fetch_data(data_id)
            fetched_data.append(result)
            print(f"✓ Successfully fetched {data_id}")

        except ConnectionError as e:
            # Network-related errors (might be transient)
            error_info = {
                "data_id": data_id,
                "error_type": "connection_error",
                "error_message": str(e),
                "timestamp": datetime.now().isoformat(),
                "retryable": True
            }
            errors.append(error_info)
            print(f"✗ Connection error for {data_id}: {e}")

        except ValueError as e:
            # Validation errors (not retryable)
            error_info = {
                "data_id": data_id,
                "error_type": "validation_error",
                "error_message": str(e),
                "timestamp": datetime.now().isoformat(),
                "retryable": False
            }
            errors.append(error_info)
            print(f"✗ Validation error for {data_id}: {e}")

        except Exception as e:
            # Unexpected errors
            error_info = {
                "data_id": data_id,
                "error_type": "unexpected_error",
                "error_message": str(e),
                "timestamp": datetime.now().isoformat(),
                "retryable": False
            }
            errors.append(error_info)
            print(f"✗ Unexpected error for {data_id}: {e}")

    # Store results and errors in context
    context.set("fetched_data", fetched_data)
    context.set("fetch_errors", errors)
    context.set("fetch_summary", {
        "total_requested": len(data_ids),
        "successful": len(fetched_data),
        "failed": len(errors),
        "success_rate": len(fetched_data) / len(data_ids) if data_ids else 0
    })

    print(f"Fetch completed: {len(fetched_data)} successful, {len(errors)} failed")

    # Decide whether to fail the task based on success rate
    success_rate = len(fetched_data) / len(data_ids) if data_ids else 0
    if success_rate < 0.5:  # Fail if less than 50% success rate
        raise RuntimeError(f"Insufficient data fetched. Success rate: {success_rate:.1%}")

    return context

@cloaca.task(id="retry_failed_fetches", dependencies=["fetch_external_data"])
def retry_failed_fetches(context):
    """Retry failed fetches with exponential backoff."""
    print("Retrying failed fetches...")

    fetch_errors = context.get("fetch_errors", [])
    retryable_errors = [e for e in fetch_errors if e.get("retryable", False)]

    if not retryable_errors:
        print("No retryable errors found")
        context.set("retry_results", {"attempted": 0, "successful": 0, "still_failed": 0})
        return context

    print(f"Attempting to retry {len(retryable_errors)} failed fetches...")

    retry_successful = []
    still_failed = []

    for error_info in retryable_errors:
        data_id = error_info["data_id"]
        max_retries = 3

        for attempt in range(max_retries):
            try:
                # Exponential backoff: wait 2^attempt seconds
                if attempt > 0:
                    wait_time = 2 ** attempt
                    print(f"  Waiting {wait_time}s before retry {attempt + 1} for {data_id}")
                    time.sleep(wait_time)

                print(f"  Retry attempt {attempt + 1}/{max_retries} for {data_id}")
                result = external_service.fetch_data(data_id)

                retry_successful.append({
                    "data_id": data_id,
                    "result": result,
                    "retry_attempt": attempt + 1,
                    "original_error": error_info["error_message"]
                })
                print(f"  ✓ Retry successful for {data_id} on attempt {attempt + 1}")
                break

            except Exception as e:
                print(f"  ✗ Retry {attempt + 1} failed for {data_id}: {e}")
                if attempt == max_retries - 1:  # Last attempt
                    still_failed.append({
                        "data_id": data_id,
                        "final_error": str(e),
                        "retry_attempts": max_retries,
                        "original_error": error_info["error_message"]
                    })

    # Merge successful retries with original fetched data
    original_fetched = context.get("fetched_data", [])
    retry_data = [r["result"] for r in retry_successful]
    all_fetched_data = original_fetched + retry_data

    context.set("all_fetched_data", all_fetched_data)
    context.set("retry_results", {
        "attempted": len(retryable_errors),
        "successful": len(retry_successful),
        "still_failed": len(still_failed),
        "successful_retries": retry_successful,
        "final_failures": still_failed
    })

    print(f"Retry completed: {len(retry_successful)} recovered, {len(still_failed)} still failed")
    return context

@cloaca.task(id="validate_and_process", dependencies=["retry_failed_fetches"])
def validate_and_process(context):
    """Validate fetched data and process with error handling."""
    print("Validating and processing data...")

    all_data = context.get("all_fetched_data", [])

    if not all_data:
        print("No data available for processing")
        context.set("processing_result", {
            "status": "no_data",
            "processed_count": 0,
            "validation_errors": [],
            "processing_errors": []
        })
        return context

    validation_errors = []
    processing_errors = []
    processed_items = []

    for item in all_data:
        try:
            # Validate data structure
            if not isinstance(item, dict):
                raise ValueError(f"Invalid data format: expected dict, got {type(item)}")

            required_fields = ["id", "data", "timestamp"]
            for field in required_fields:
                if field not in item:
                    raise ValueError(f"Missing required field: {field}")

            # Validate data content
            if not item["data"].startswith("important_data_"):
                raise ValueError(f"Invalid data format: {item['data']}")

            # Process the validated data
            processed_item = {
                "original_id": item["id"],
                "processed_data": item["data"].upper(),
                "data_length": len(item["data"]),
                "processed_at": datetime.now().isoformat(),
                "source_timestamp": item["timestamp"],
                "fetch_attempt": item.get("fetch_attempt", "unknown")
            }

            # Simulate processing that might fail
            if random.random() < 0.1:  # 10% chance of processing failure
                raise RuntimeError(f"Processing failed for item {item['id']}")

            processed_items.append(processed_item)

        except ValueError as e:
            validation_errors.append({
                "item_id": item.get("id", "unknown"),
                "error": str(e),
                "error_type": "validation"
            })
            print(f"  ✗ Validation error for {item.get('id', 'unknown')}: {e}")

        except RuntimeError as e:
            processing_errors.append({
                "item_id": item.get("id", "unknown"),
                "error": str(e),
                "error_type": "processing"
            })
            print(f"  ✗ Processing error for {item.get('id', 'unknown')}: {e}")

        except Exception as e:
            processing_errors.append({
                "item_id": item.get("id", "unknown"),
                "error": str(e),
                "error_type": "unexpected"
            })
            print(f"  ✗ Unexpected error for {item.get('id', 'unknown')}: {e}")

    processing_result = {
        "status": "completed",
        "total_input": len(all_data),
        "processed_count": len(processed_items),
        "validation_errors": validation_errors,
        "processing_errors": processing_errors,
        "error_rate": (len(validation_errors) + len(processing_errors)) / len(all_data) if all_data else 0
    }

    context.set("processed_items", processed_items)
    context.set("processing_result", processing_result)

    print(f"Processing completed: {len(processed_items)} items processed")
    print(f"Errors: {len(validation_errors)} validation, {len(processing_errors)} processing")

    return context

@cloaca.task(id="generate_error_report", dependencies=["validate_and_process"])
def generate_error_report(context):
    """Generate comprehensive error report and recovery recommendations."""
    print("Generating error report...")

    # Collect all error information from the workflow
    fetch_errors = context.get("fetch_errors", [])
    retry_results = context.get("retry_results", {})
    processing_result = context.get("processing_result", {})
    fetch_summary = context.get("fetch_summary", {})

    # Calculate overall success metrics
    original_data_count = fetch_summary.get("total_requested", 0)
    final_processed_count = processing_result.get("processed_count", 0)
    overall_success_rate = final_processed_count / original_data_count if original_data_count > 0 else 0

    # Categorize errors by type and severity
    error_analysis = {
        "fetch_errors": {
            "connection_errors": len([e for e in fetch_errors if e.get("error_type") == "connection_error"]),
            "validation_errors": len([e for e in fetch_errors if e.get("error_type") == "validation_error"]),
            "unexpected_errors": len([e for e in fetch_errors if e.get("error_type") == "unexpected_error"])
        },
        "retry_effectiveness": {
            "attempted": retry_results.get("attempted", 0),
            "recovered": retry_results.get("successful", 0),
            "recovery_rate": retry_results.get("successful", 0) / retry_results.get("attempted", 1)
        },
        "processing_errors": {
            "validation_errors": len(processing_result.get("validation_errors", [])),
            "processing_errors": len(processing_result.get("processing_errors", []))
        }
    }

    # Generate recommendations
    recommendations = []

    if error_analysis["fetch_errors"]["connection_errors"] > 2:
        recommendations.append("Consider increasing retry attempts for connection errors")

    if error_analysis["retry_effectiveness"]["recovery_rate"] < 0.5:
        recommendations.append("Review retry strategy - current approach may be insufficient")

    if processing_result.get("error_rate", 0) > 0.2:
        recommendations.append("High processing error rate detected - review data validation logic")

    if overall_success_rate < 0.7:
        recommendations.append("Overall success rate is low - consider workflow redesign")

    # Create comprehensive error report
    error_report = {
        "report_metadata": {
            "generated_at": datetime.now().isoformat(),
            "workflow_execution": context.get("tutorial", "03")
        },
        "success_metrics": {
            "original_data_requested": original_data_count,
            "final_processed_count": final_processed_count,
            "overall_success_rate": overall_success_rate,
            "fetch_success_rate": fetch_summary.get("success_rate", 0),
            "retry_recovery_rate": error_analysis["retry_effectiveness"]["recovery_rate"]
        },
        "error_analysis": error_analysis,
        "detailed_errors": {
            "fetch_errors": fetch_errors,
            "retry_failures": retry_results.get("final_failures", []),
            "validation_errors": processing_result.get("validation_errors", []),
            "processing_errors": processing_result.get("processing_errors", [])
        },
        "recommendations": recommendations,
        "workflow_resilience": "high" if overall_success_rate > 0.8 else "medium" if overall_success_rate > 0.5 else "low"
    }

    context.set("error_report", error_report)
    context.set("report_complete", True)

    print(f"Error report generated - Overall success rate: {overall_success_rate:.1%}")
    print(f"Workflow resilience: {error_report['workflow_resilience']}")

    return context

# Create the error handling workflow
def create_error_handling_workflow():
    """Build the error handling and recovery workflow."""
    builder = cloaca.WorkflowBuilder("error_handling_workflow")
    builder.description("Demonstrates comprehensive error handling, retry logic, and recovery patterns")

    # Add tasks with dependencies
    builder.add_task("fetch_external_data")
    builder.add_task("retry_failed_fetches")
    builder.add_task("validate_and_process")
    builder.add_task("generate_error_report")

    return builder.build()

# Register the workflow
cloaca.register_workflow_constructor("error_handling_workflow", create_error_handling_workflow)

# Main execution
if __name__ == "__main__":
    print("=== Cloacina Python Tutorial 03: Error Handling and Recovery ===")
    print()
    print("This tutorial demonstrates:")
    print("- Robust error handling in tasks")
    print("- Retry mechanisms with exponential backoff")
    print("- Error categorization and analysis")
    print("- Recovery strategies and fallback patterns")
    print("- Comprehensive error reporting")
    print()

    # Create runner
    runner = cloaca.DefaultRunner("sqlite://:memory:")

    # Create initial context
    context = cloaca.Context({
        "tutorial": "03",
        "data_ids": ["data_001", "data_002", "data_003", "data_004", "data_005"],
        "error_simulation": True
    })

    print("Executing error handling workflow...")
    print("Note: This workflow intentionally includes failures to demonstrate error handling")
    print()

    # Execute the workflow
    result = runner.execute("error_handling_workflow", context)

    # Display results
    print(f"\nWorkflow Status: {result.status}")

    if result.status == "Completed":
        print("Success! Error handling workflow completed.")

        # Access the error report
        final_context = result.final_context
        error_report = final_context.get("error_report")

        if error_report:
            print("\n=== Error Handling Results ===")
            metrics = error_report["success_metrics"]
            print(f"Data requested: {metrics['original_data_requested']}")
            print(f"Successfully processed: {metrics['final_processed_count']}")
            print(f"Overall success rate: {metrics['overall_success_rate']:.1%}")
            print(f"Fetch success rate: {metrics['fetch_success_rate']:.1%}")
            print(f"Retry recovery rate: {metrics['retry_recovery_rate']:.1%}")
            print(f"Workflow resilience: {error_report['workflow_resilience']}")

            if error_report["recommendations"]:
                print("\nRecommendations:")
                for rec in error_report["recommendations"]:
                    print(f"  • {rec}")

            # Show error breakdown
            analysis = error_report["error_analysis"]
            print("\nError breakdown:")
            print(f"  Fetch errors: {sum(analysis['fetch_errors'].values())}")
            print(f"  Processing errors: {sum(analysis['processing_errors'].values())}")
            print(f"  Retry recovery: {analysis['retry_effectiveness']['recovered']}/{analysis['retry_effectiveness']['attempted']}")

    else:
        print(f"Workflow failed with status: {result.status}")
        if hasattr(result, 'error'):
            print(f"Error: {result.error}")

        # Clean up before exiting
        print("\nCleaning up...")
        runner.shutdown()
        sys.exit(1)

    # Cleanup
    print("\nCleaning up...")
    runner.shutdown()
    print("Tutorial 03 completed!")
    print()
    print("Key concepts demonstrated:")
    print("- Exception handling within tasks")
    print("- Retry logic with exponential backoff")
    print("- Error categorization and analysis")
    print("- Graceful degradation patterns")
    print("- Comprehensive error reporting")
    print()
    print("Next steps:")
    print("- Try python_tutorial_04_complex_workflows.py for advanced patterns")
    print("- Experiment with different error rates and retry strategies")
    print("- Implement your own fallback mechanisms")
