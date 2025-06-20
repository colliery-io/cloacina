"""Performance testing commands for Cloacina examples."""

import os
import subprocess
import angreal

# Define command group
performance = angreal.command_group(name="performance", about="run performance tests for Cloacina")


@performance()
@angreal.command(
    name="simple",
    about="run simple performance test",
    when_to_use=["performance benchmarking", "regression testing", "measuring baseline performance"],
    when_not_to_use=["functional testing", "development debugging", "small test runs"]
)
@angreal.argument(name="iterations", python_type="int", long="iterations", short="i", takes_value=True, required=False, help="number of workflow iterations to execute")
@angreal.argument(name="concurrency", python_type="int", long="concurrency", short="c", takes_value=True, required=False, help="maximum number of concurrent tasks")
def performance_simple(iterations: int=150, concurrency: int=32):
    """Run the simple performance test example."""
    print(f"Running simple performance test ({iterations} iterations, {concurrency} concurrency)")

    example_dir = os.path.join("examples", "performance-simple")
    if not os.path.exists(example_dir):
        print(f"ERROR: Performance simple example not found at {example_dir}")
        return 1

    try:
        print("Building and running performance test (this may take a moment)...")
        result = subprocess.run(
            ["cargo", "run", "--", "--iterations", str(iterations), "--concurrency", str(concurrency)],
            cwd=example_dir,
            stderr=subprocess.DEVNULL  # Suppress debug output
        )

        return result.returncode
    except Exception as e:
        print(f"ERROR: Error running simple performance test: {e}")
        return 1


@performance()
@angreal.command(
    name="pipeline",
    about="run pipeline performance test",
    when_to_use=["testing pipeline performance", "benchmarking complex workflows", "measuring sequential task overhead"],
    when_not_to_use=["testing parallel execution", "simple workflow testing", "debugging functionality"]
)
@angreal.argument(name="iterations", python_type="int", long="iterations", short="i", takes_value=True, required=False, help="number of workflow iterations to execute")
@angreal.argument(name="concurrency", python_type="int", long="concurrency", short="c", takes_value=True, required=False, help="maximum number of concurrent tasks")
def performance_pipeline(iterations: int=150, concurrency: int=32):
    """Run the pipeline performance test example."""
    print(f"Running pipeline performance test ({iterations} iterations, {concurrency} concurrency)")

    example_dir = os.path.join("examples", "performance-pipeline")
    if not os.path.exists(example_dir):
        print(f"ERROR: Performance pipeline example not found at {example_dir}")
        return 1

    try:
        print("Building and running performance test (this may take a moment)...")
        result = subprocess.run(
            ["cargo", "run", "--", "--iterations", str(iterations), "--concurrency", str(concurrency)],
            cwd=example_dir,
            stderr=subprocess.DEVNULL  # Suppress debug output
        )

        return result.returncode
    except Exception as e:
        print(f"ERROR: Error running pipeline performance test: {e}")
        return 1


@performance()
@angreal.command(
    name="parallel",
    about="run parallel performance test",
    when_to_use=["testing parallel execution", "benchmarking concurrency", "measuring parallelization benefits"],
    when_not_to_use=["testing sequential workflows", "debugging task order", "simple performance testing"]
)
@angreal.argument(name="iterations", python_type="int", long="iterations", short="i", takes_value=True, required=False, help="number of workflow iterations to execute")
@angreal.argument(name="concurrency", python_type="int", long="concurrency", short="c", takes_value=True, required=False, help="maximum number of concurrent tasks")
def performance_parallel(iterations: int=150, concurrency: int=32):
    """Run the parallel performance test example."""
    print(f"Running parallel performance test ({iterations} iterations, {concurrency} concurrency)")

    example_dir = os.path.join("examples", "performance-parallel")
    if not os.path.exists(example_dir):
        print(f"ERROR: Performance parallel example not found at {example_dir}")
        return 1

    try:
        print("Building and running performance test (this may take a moment)...")
        result = subprocess.run(
            ["cargo", "run", "--", "--iterations", str(iterations), "--concurrency", str(concurrency)],
            cwd=example_dir,
            stderr=subprocess.DEVNULL  # Suppress debug output
        )

        return result.returncode
    except Exception as e:
        print(f"ERROR: Error running parallel performance test: {e}")
        return 1



@performance()
@angreal.command(
    name="all",
    about="run all performance tests",
    when_to_use=["comprehensive performance testing", "release validation", "comparing all test types"],
    when_not_to_use=["quick feedback", "development testing", "resource-constrained environments"]
)
def performance_all():
    """Run all performance tests."""
    print("Running all performance tests")

    tests = [
        ("Simple Performance Test", performance_simple),
        ("Pipeline Performance Test", performance_pipeline),
        ("Parallel Performance Test", performance_parallel),
    ]

    results = []
    for test_name, test_func in tests:
        print(f"\n{'='*60}")
        print(f"Running {test_name}")
        print(f"{'='*60}")

        result = test_func()
        results.append((test_name, result))

        if result != 0:
            print(f"ERROR: {test_name} failed with return code {result}")
        else:
            print(f"SUCCESS: {test_name} completed successfully")

    # Summary
    print(f"\n{'='*60}")
    print("Performance Test Summary")
    print(f"{'='*60}")

    for test_name, result in results:
        status = "PASS" if result == 0 else "FAIL"
        print(f"{test_name}: {status}")

    failed_tests = [name for name, result in results if result != 0]
    if failed_tests:
        print(f"ERROR: Failed tests: {', '.join(failed_tests)}")
        return 1
    else:
        print("SUCCESS: All performance tests passed")
        return 0


@performance()
@angreal.command(
    name="quick",
    about="run quick performance tests",
    when_to_use=["rapid feedback", "development testing", "sanity checks"],
    when_not_to_use=["production benchmarking", "accurate performance metrics", "release validation"]
)
def performance_quick():
    """Run quick performance tests with reduced iterations."""
    print("Running quick performance tests")

    example_configs = [
        ("performance-simple", ["--iterations", "25", "--concurrency", "2"]),
        ("performance-pipeline", ["--iterations", "25", "--concurrency", "2"]),
        ("performance-parallel", ["--iterations", "20", "--concurrency", "4"]),
    ]

    results = []
    for example_name, args in example_configs:
        example_dir = os.path.join("examples", example_name)

        if not os.path.exists(example_dir):
            print(f"ERROR: Example not found at {example_dir}")
            results.append((example_name, 1))
            continue

        print(f"Running quick test for {example_name}")
        print(f"\n{'='*50}")
        print(f"Quick Test: {example_name}")
        print(f"{'='*50}")

        try:
            result = subprocess.run(
                ["cargo", "run", "--"] + args,
                cwd=example_dir,
                capture_output=True,
                text=True
            )

            print(result.stdout)
            if result.stderr:
                print("STDERR:", result.stderr)

            results.append((example_name, result.returncode))
        except Exception as e:
            print(f"ERROR: Error running {example_name}: {e}")
            results.append((example_name, 1))

    # Summary
    print(f"\n{'='*50}")
    print("Quick Performance Test Summary")
    print(f"{'='*50}")

    for test_name, result in results:
        status = "PASS" if result == 0 else "FAIL"
        print(f"{test_name}: {status}")

    failed_tests = [name for name, result in results if result != 0]
    if failed_tests:
        print(f"ERROR: Failed tests: {', '.join(failed_tests)}")
        return 1
    else:
        print("SUCCESS: All quick performance tests passed")
        return 0
