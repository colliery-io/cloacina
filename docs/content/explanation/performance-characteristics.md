---
title: "Performance Characteristics"
description: "Understanding Cloacina's performance test suite and how to run benchmarks"
weight: 55
reviewer: "dstorey"
review_date: "2025-01-17"
---

This article describes Cloacina's performance test suite and how to run benchmarks to understand system performance characteristics.

{{< hint type=warning title="Performance Measurement Methodology" >}}
The performance tests measure throughput in terms of **workflows per second**, but for a more accurate assessment of system performance, **tasks per second** would be a better metric. This is because:

- **Workflows/second** measures complete workflow completion rates
- **Tasks/second** would better reflect the actual processing capacity of the system
- Different workflow types (simple, parallel, complex DAG) have varying numbers of tasks per workflow
- Task-level metrics provide more granular insight into system performance characteristics

The current measurements focus on workflow completion rates, which is useful for understanding end-to-end performance but may not fully represent the system's task processing capabilities.
{{< /hint >}}

## Performance Test Suite

Cloacina includes a comprehensive performance test suite located in the `examples/` directory:

- **`performance-simple/`**: Single-task workflow throughput testing
- **`performance-parallel/`**: Multi-task parallel execution testing
- **`performance-pipeline/`**: Complex DAG workflow testing

These tests measure different aspects of system performance and help identify bottlenecks and optimization opportunities.

## What the Tests Measure

### Simple Performance Test

The simple performance test measures throughput of single-task workflows, which represents the baseline performance of Cloacina's execution engine.

**Test Configuration:**
```rust
#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value = "150")]
    iterations: usize,

    #[arg(short, long, default_value = "32")]
    concurrency: usize,
}
```

**What it tests:**
- Basic workflow execution throughput
- Database I/O performance
- Context serialization overhead
- Concurrency scaling characteristics

### Parallel Performance Test

The parallel performance test measures execution of workflows with multiple independent tasks that can run concurrently.

**What it tests:**
- Parallel task execution efficiency
- Dependency resolution performance
- Context merging overhead
- Database contention under concurrent load

### Pipeline Performance Test

The pipeline performance test measures complex DAG workflows with dependencies to test the system's ability to handle sophisticated execution patterns.

**What it tests:**
- DAG complexity impact on execution time
- Dependency resolution scaling
- Workflow completion time vs task count
- Memory usage patterns with complex workflows

## Running the Performance Tests

The performance tests can be run through the angreal performance command:

```bash
angreal performance
```

This command will:
- Build and run all performance test variants
- Generate performance metrics and graphs
- Provide analysis of the results

## Test Results

Performance test results and graphs are generated when running the tests. The following visualization shows comprehensive performance results for the **pipeline performance test** with SQLite backend across various configuration settings:

![SQLite Pipeline Runner Performance Graph](/cloacina/pipeline-performance.png)

*Performance characteristics of the pipeline performance test with SQLite backend across different concurrency levels and workflow complexities*

### Key Insights

- **Near-linear scaling up to 8 workers with 89.4% efficiency**
- **16.6x throughput improvement achieved with 32x concurrency increase**
- **Diminishing returns observed beyond 16 workers (efficiency drops to 51.9%)**
- **Optimal efficiency range appears to be 4-8 workers for this workload**

The visualization shows performance characteristics including concurrency levels, workflow complexity, and execution patterns for the pipeline performance test.

### Parallel Performance Test Results

The following visualization shows performance results for the **parallel performance test** with SQLite backend:

![SQLite Parallel Performance Graph](/cloacina/parallel-performance.png)

*Performance characteristics of the parallel performance test with SQLite backend*

The parallel performance test measures execution of workflows with multiple independent tasks that can run concurrently, testing the system's ability to handle parallel task execution efficiently.

#### Key Insights

- **Peak performance at 32 workers: 44.60 workflows/s**
- **Performance degradation at 64 workers (37.05 vs 44.60 workflows/s)**
- **22.5x speedup achieved with 32x concurrency (70.4% efficiency)**
- **Near-linear scaling up to 8 workers (95.4% efficiency)**

## Related Resources

- [Explanation: Task Execution Sequence]({{< ref "/explanation/task-execution-sequence/" >}})
- [Explanation: Database Backends]({{< ref "/explanation/database-backends/" >}})
- [Examples: Performance Tests](https://github.com/colliery-io/cloacina/tree/main/examples#performance-examples)
