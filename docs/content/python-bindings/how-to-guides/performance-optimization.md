---
title: "Performance Optimization"
description: "Optimize Cloaca workflow performance for production workloads"
weight: 30
---

# Performance Optimization

Learn how to optimize Cloaca workflows for production workloads, including database tuning, workflow design patterns, and monitoring strategies.

## Overview

Performance optimization in Cloaca involves several areas:

- **Database optimization** - Connection pooling, indexing, query optimization
- **Workflow design** - Task granularity, dependency management, parallel execution
- **Resource management** - Memory usage, connection limits, concurrent execution
- **Monitoring** - Performance metrics, bottleneck identification

## Database Optimization

### Connection Pooling

```python
import cloaca

# Optimized PostgreSQL connection with pooling
runner = cloaca.DefaultRunner(
    "postgresql://user:pass@host:5432/db?"
    "pool_min_size=5&"           # Minimum connections
    "pool_max_size=20&"          # Maximum connections
    "pool_timeout=30&"           # Connection timeout
    "pool_recycle=3600"          # Recycle connections hourly
)
```

### Database Configuration

```python
# PostgreSQL connection with performance tuning
database_url = (
    "postgresql://user:pass@host:5432/cloacina?"
    "pool_min_size=10&"
    "pool_max_size=50&"
    "pool_timeout=30&"
    "pool_recycle=7200&"
    "connect_timeout=10&"
    "application_name=cloacina_app"
)

runner = cloaca.DefaultRunner(database_url)
```

### Multi-Tenant Connection Management

```python
class OptimizedTenantManager:
    def __init__(self, base_url: str):
        self.base_url = base_url
        self.runners = {}
        self.connection_limits = {
            "max_per_tenant": 10,
            "total_max": 100
        }

    def get_runner(self, tenant_id: str):
        if tenant_id not in self.runners:
            # Create optimized connection per tenant
            tenant_url = f"{self.base_url}?pool_max_size={self.connection_limits['max_per_tenant']}"
            self.runners[tenant_id] = cloaca.DefaultRunner.with_schema(
                tenant_url,
                tenant_id
            )

        return self.runners[tenant_id]
```

## Workflow Design Optimization

### Task Granularity

```python
import cloaca
from typing import List

# Inefficient: Too many small tasks
@cloaca.task(id="process_item")
def process_single_item(context):
    item = context.get("item")
    result = expensive_operation(item)
    context.set("result", result)
    return context

# Efficient: Batch processing
@cloaca.task(id="process_batch")
def process_item_batch(context):
    items = context.get("items", [])
    batch_size = context.get("batch_size", 100)

    results = []
    for i in range(0, len(items), batch_size):
        batch = items[i:i + batch_size]
        batch_results = process_batch_efficiently(batch)
        results.extend(batch_results)

    context.set("results", results)
    return context
```

### Parallel Task Design

```python
# Design workflows for parallel execution
def create_parallel_workflow():
    builder = cloaca.WorkflowBuilder("parallel_processing")

    # Independent tasks that can run in parallel
    builder.add_task("fetch_data_source_a")
    builder.add_task("fetch_data_source_b")
    builder.add_task("fetch_data_source_c")

    # Aggregation task that depends on all data sources
    builder.add_task("aggregate_results", dependencies=[
        "fetch_data_source_a",
        "fetch_data_source_b",
        "fetch_data_source_c"
    ])

    return builder.build()
```

### Context Size Management

```python
@cloaca.task(id="memory_efficient_task")
def memory_efficient_processing(context):
    # Avoid storing large data in context
    large_dataset = context.get("dataset_reference")  # Reference, not data

    # Process in chunks to manage memory
    chunk_size = 1000
    results_summary = {
        "processed_count": 0,
        "success_count": 0,
        "error_count": 0
    }

    for chunk in process_in_chunks(large_dataset, chunk_size):
        try:
            process_chunk(chunk)
            results_summary["success_count"] += len(chunk)
        except Exception as e:
            results_summary["error_count"] += len(chunk)
            # Log error but continue processing

        results_summary["processed_count"] += len(chunk)

    # Store summary, not full data
    context.set("processing_summary", results_summary)
    return context
```

## Async Task Optimization

### Efficient Async Processing

```python
import asyncio
import aiohttp

@cloaca.task(id="optimized_api_calls")
async def optimized_api_calls(context):
    urls = context.get("urls", [])
    max_concurrent = context.get("max_concurrent_requests", 10)

    # Use semaphore to limit concurrent requests
    semaphore = asyncio.Semaphore(max_concurrent)

    async def fetch_url(session, url):
        async with semaphore:
            try:
                async with session.get(url) as response:
                    return await response.json()
            except Exception as e:
                return {"error": str(e), "url": url}

    # Process all URLs concurrently with limit
    async with aiohttp.ClientSession() as session:
        tasks = [fetch_url(session, url) for url in urls]
        results = await asyncio.gather(*tasks)

    context.set("api_results", results)
    return context
```

### Resource Management

```python
from contextlib import asynccontextmanager

@cloaca.task(id="resource_managed_task")
async def resource_managed_task(context):
    # Properly manage resources
    @asynccontextmanager
    async def managed_resource():
        resource = await acquire_expensive_resource()
        try:
            yield resource
        finally:
            await resource.cleanup()

    async with managed_resource() as resource:
        # Use resource efficiently
        results = await resource.process_data(context.get("data"))
        context.set("processed_results", results)

    return context
```

## Caching Strategies

### Result Caching

```python
import functools
import hashlib
import json

class ResultCache:
    def __init__(self):
        self.cache = {}

    def cache_key(self, context, task_id):
        # Create cache key from relevant context data
        relevant_data = {
            "task_id": task_id,
            "input_data": context.get("cache_key_data")
        }
        key_string = json.dumps(relevant_data, sort_keys=True)
        return hashlib.md5(key_string.encode()).hexdigest()

    def get(self, key):
        return self.cache.get(key)

    def set(self, key, value):
        self.cache[key] = value

# Global cache instance
result_cache = ResultCache()

@cloaca.task(id="cached_expensive_task")
def cached_expensive_task(context):
    cache_key = result_cache.cache_key(context, "cached_expensive_task")

    # Check cache first
    cached_result = result_cache.get(cache_key)
    if cached_result is not None:
        context.set("result", cached_result)
        context.set("cache_hit", True)
        return context

    # Perform expensive operation
    result = expensive_computation(context.get("input_data"))

    # Cache result
    result_cache.set(cache_key, result)
    context.set("result", result)
    context.set("cache_hit", False)

    return context
```

### External Caching

```python
import redis

# Redis-based caching for distributed systems
redis_client = redis.Redis(host='localhost', port=6379, db=0)

@cloaca.task(id="redis_cached_task")
def redis_cached_task(context):
    cache_key = f"task_result:{context.get('input_hash')}"
    ttl = 3600  # 1 hour cache

    # Check Redis cache
    cached_result = redis_client.get(cache_key)
    if cached_result:
        context.set("result", json.loads(cached_result))
        context.set("cache_hit", True)
        return context

    # Compute result
    result = compute_result(context.get("input_data"))

    # Cache in Redis
    redis_client.setex(cache_key, ttl, json.dumps(result))
    context.set("result", result)
    context.set("cache_hit", False)

    return context
```

## Monitoring and Profiling

### Performance Metrics

```python
import time
import psutil
from datetime import datetime

@cloaca.task(id="monitored_task")
def monitored_task(context):
    # Start monitoring
    start_time = time.time()
    start_memory = psutil.Process().memory_info().rss

    try:
        # Your task logic here
        result = perform_task_work(context)

        # Record success metrics
        execution_time = time.time() - start_time
        memory_used = psutil.Process().memory_info().rss - start_memory

        context.set("performance_metrics", {
            "execution_time": execution_time,
            "memory_used": memory_used,
            "status": "success",
            "timestamp": datetime.now().isoformat()
        })

        context.set("result", result)

    except Exception as e:
        # Record error metrics
        execution_time = time.time() - start_time
        context.set("performance_metrics", {
            "execution_time": execution_time,
            "status": "error",
            "error": str(e),
            "timestamp": datetime.now().isoformat()
        })
        raise

    return context
```

### Workflow Performance Tracking

```python
class WorkflowPerformanceTracker:
    def __init__(self):
        self.metrics = {}

    def track_execution(self, workflow_name: str, execution_time: float, success: bool):
        if workflow_name not in self.metrics:
            self.metrics[workflow_name] = {
                "total_executions": 0,
                "total_time": 0.0,
                "success_count": 0,
                "error_count": 0,
                "avg_time": 0.0
            }

        metrics = self.metrics[workflow_name]
        metrics["total_executions"] += 1
        metrics["total_time"] += execution_time

        if success:
            metrics["success_count"] += 1
        else:
            metrics["error_count"] += 1

        metrics["avg_time"] = metrics["total_time"] / metrics["total_executions"]

    def get_performance_report(self):
        return self.metrics

# Usage
tracker = WorkflowPerformanceTracker()

def execute_with_tracking(runner, workflow_name, context):
    start_time = time.time()
    success = False

    try:
        result = runner.execute(workflow_name, context)
        success = result.status == "Completed"
        return result
    finally:
        execution_time = time.time() - start_time
        tracker.track_execution(workflow_name, execution_time, success)
```

## Batch Processing Optimization

### Efficient Batch Design

```python
@cloaca.task(id="optimized_batch_processor")
def optimized_batch_processor(context):
    items = context.get("items", [])
    batch_size = context.get("batch_size", 100)
    max_workers = context.get("max_workers", 4)

    # Process in parallel batches
    from concurrent.futures import ThreadPoolExecutor

    def process_batch(batch):
        # Process a batch of items
        results = []
        for item in batch:
            try:
                result = process_single_item(item)
                results.append({"item": item, "result": result, "status": "success"})
            except Exception as e:
                results.append({"item": item, "error": str(e), "status": "error"})
        return results

    # Create batches
    batches = [items[i:i + batch_size] for i in range(0, len(items), batch_size)]

    # Process batches in parallel
    all_results = []
    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        batch_results = executor.map(process_batch, batches)
        for batch_result in batch_results:
            all_results.extend(batch_result)

    # Summarize results
    success_count = sum(1 for r in all_results if r["status"] == "success")
    error_count = len(all_results) - success_count

    context.set("batch_results", {
        "total_processed": len(all_results),
        "success_count": success_count,
        "error_count": error_count,
        "results": all_results
    })

    return context
```

## Production Deployment Optimization

### Environment Configuration

```python
import os

class ProductionOptimizedRunner:
    def __init__(self):
        self.database_url = self._build_optimized_database_url()
        self.runner = cloaca.DefaultRunner(self.database_url)

    def _build_optimized_database_url(self):
        base_url = os.getenv("DATABASE_URL")
        if not base_url:
            raise ValueError("DATABASE_URL environment variable required")

        # Add production optimizations
        params = {
            "pool_min_size": os.getenv("DB_POOL_MIN_SIZE", "10"),
            "pool_max_size": os.getenv("DB_POOL_MAX_SIZE", "50"),
            "pool_timeout": os.getenv("DB_POOL_TIMEOUT", "30"),
            "pool_recycle": os.getenv("DB_POOL_RECYCLE", "7200"),
            "connect_timeout": os.getenv("DB_CONNECT_TIMEOUT", "10"),
            "application_name": os.getenv("APP_NAME", "cloacina_prod")
        }

        param_string = "&".join(f"{k}={v}" for k, v in params.items())
        separator = "&" if "?" in base_url else "?"

        return f"{base_url}{separator}{param_string}"
```

### Health Checks

```python
def workflow_health_check(runner):
    """Check if workflow system is healthy"""
    try:
        # Simple health check workflow
        @cloaca.task(id="health_check")
        def health_check_task(context):
            context.set("health_status", "healthy")
            context.set("timestamp", datetime.now().isoformat())
            return context

        def create_health_workflow():
            builder = cloaca.WorkflowBuilder("health_check")
            builder.add_task("health_check")
            return builder.build()

        cloaca.register_workflow_constructor("health_check", create_health_workflow)

        start_time = time.time()
        result = runner.execute("health_check", cloaca.Context({}))
        execution_time = time.time() - start_time

        return {
            "healthy": result.status == "Completed",
            "execution_time": execution_time,
            "timestamp": datetime.now().isoformat()
        }

    except Exception as e:
        return {
            "healthy": False,
            "error": str(e),
            "timestamp": datetime.now().isoformat()
        }
```

## Best Practices Summary

### Database Performance

1. **Use connection pooling** with appropriate limits
2. **Configure timeouts** to prevent hanging connections
3. **Monitor connection usage** across tenants
4. **Use PostgreSQL** for production workloads

### Workflow Design

1. **Batch small operations** to reduce overhead
2. **Design for parallelism** when possible
3. **Manage context size** to control memory usage
4. **Cache expensive computations**

### Resource Management

1. **Use async/await** for I/O-bound operations
2. **Implement proper cleanup** for resources
3. **Monitor memory usage** in long-running tasks
4. **Set reasonable timeouts** for external calls

### Monitoring

1. **Track execution times** and success rates
2. **Monitor resource usage** (CPU, memory, connections)
3. **Set up alerts** for performance degradation
4. **Regular performance testing** in staging environment

## Troubleshooting Performance Issues

### Common Issues

```python
# Too many database connections
# Solution: Reduce pool sizes, implement connection sharing

# Slow workflow execution
# Solution: Profile individual tasks, optimize bottlenecks

# Memory leaks
# Solution: Proper resource cleanup, context size management

# Deadlocks in multi-tenant scenarios
# Solution: Consistent ordering of operations, timeout configuration
```

## See Also

- [Backend Selection]({{< ref "/python-bindings/how-to-guides/backend-selection/" >}}) - Choose the right database backend
- [Testing Workflows]({{< ref "/python-bindings/how-to-guides/testing-workflows/" >}}) - Performance testing strategies
- [Multi-Tenancy Tutorial]({{< ref "/python-bindings/tutorials/06-multi-tenancy/" >}}) - Multi-tenant performance considerations
- [API Reference]({{< ref "/python-bindings/api-reference/" >}}) - Configuration options for performance
