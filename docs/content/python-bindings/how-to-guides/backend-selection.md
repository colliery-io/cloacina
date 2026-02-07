---
title: "Backend Selection"
description: "How to choose between SQLite and PostgreSQL backends for your Cloaca deployment"
weight: 20
---

# Backend Selection

Cloaca supports two database backends: SQLite for development and single-tenant deployments, and PostgreSQL for production and multi-tenant deployments. This guide helps you choose the right backend for your needs.

## Overview

| Feature | SQLite | PostgreSQL |
|---------|--------|------------|
| **Setup Complexity** | Minimal | Moderate |
| **Multi-tenancy** | Not supported | Full support |
| **Concurrent Access** | Limited | Excellent |
| **Production Ready** | Development only | Yes |
| **Admin API** | Not available | Available |
| **Performance** | Good for single user | Excellent for concurrent |

## SQLite Backend

### When to Use SQLite

- **Development and testing**
- **Single-tenant applications**
- **Desktop applications**
- **Proof of concepts**
- **Local development**

### Installation

```bash
pip install cloaca
```

### Basic Usage

```python
import cloaca

# SQLite with file
runner = cloaca.DefaultRunner("sqlite:///workflows.db")

# SQLite in-memory (testing)
runner = cloaca.DefaultRunner("sqlite:///:memory:")
```

### Configuration Options

```python
# File-based SQLite with custom path
runner = cloaca.DefaultRunner("sqlite:///./data/workflows.db")

# SQLite with WAL mode (better concurrency)
runner = cloaca.DefaultRunner("sqlite:///workflows.db?mode=rwc&_journal_mode=WAL")

# SQLite with custom timeout
runner = cloaca.DefaultRunner("sqlite:///workflows.db?_busy_timeout=5000")
```

### Limitations

- **No multi-tenancy support**
- **Limited concurrent access**
- **Not recommended for production**
- **No Admin API**

## PostgreSQL Backend

### When to Use PostgreSQL

- **Production deployments**
- **Multi-tenant applications**
- **High concurrency requirements**
- **SaaS applications**
- **Team development**

### Installation

```bash
pip install cloaca
```

### Basic Usage

```python
import cloaca

# Basic PostgreSQL connection
runner = cloaca.DefaultRunner("postgresql://user:password@localhost:5432/cloacina")

# PostgreSQL with schema isolation
runner = cloaca.DefaultRunner.with_schema(
    "postgresql://user:password@localhost:5432/cloacina",
    "tenant_schema"
)
```

### Multi-Tenant Setup

```python
# Admin setup for tenant provisioning
admin = cloaca.DatabaseAdmin("postgresql://admin:admin@localhost:5432/cloacina")

# Create tenant
config = cloaca.TenantConfig(
    schema_name="tenant_acme",
    username="acme_user",
    password=""  # Auto-generate
)

credentials = admin.create_tenant(config)

# Use tenant-specific runner
tenant_runner = cloaca.DefaultRunner(credentials.connection_string)
```

### Advanced Configuration

```python
# Connection pooling
runner = cloaca.DefaultRunner(
    "postgresql://user:password@localhost:5432/cloacina?pool_min_size=5&pool_max_size=20"
)

# SSL configuration
runner = cloaca.DefaultRunner(
    "postgresql://user:password@host:5432/db?sslmode=require"
)

# Connection timeout
runner = cloaca.DefaultRunner(
    "postgresql://user:password@host:5432/db?connect_timeout=10"
)
```

## Decision Matrix

### For Development

```python
# Simple development setup
if environment == "development":
    runner = cloaca.DefaultRunner("sqlite:///dev_workflows.db")
```

**Pros:**
- Zero configuration
- No external dependencies
- Fast setup
- Good for testing

**Cons:**
- Not representative of production
- Limited testing scenarios

### For Testing

```python
# In-memory for unit tests
@pytest.fixture
def test_runner():
    runner = cloaca.DefaultRunner("sqlite:///:memory:")
    yield runner
    runner.shutdown()

# PostgreSQL for integration tests
@pytest.fixture
def integration_runner():
    runner = cloaca.DefaultRunner("postgresql://test:test@localhost:5432/test_db")
    yield runner
    runner.shutdown()
```

### For Production

```python
# Production PostgreSQL setup
def create_production_runner():
    database_url = os.getenv("DATABASE_URL")
    if not database_url:
        raise ValueError("DATABASE_URL environment variable required")

    return cloaca.DefaultRunner(database_url)
```

## Migration from SQLite to PostgreSQL

### Data Migration

```python
# Export from SQLite (conceptual - not implemented)
sqlite_runner = cloaca.DefaultRunner("sqlite:///old_workflows.db")
# Note: Direct data migration tools are not provided
# Recommend re-running workflows in new environment

# Set up new PostgreSQL environment
pg_runner = cloaca.DefaultRunner("postgresql://user:pass@host/db")
```

### Code Changes Required

```python
# Before (SQLite)
runner = cloaca.DefaultRunner("sqlite:///workflows.db")

# After (PostgreSQL)
runner = cloaca.DefaultRunner("postgresql://user:pass@host:5432/db")

# Multi-tenant version
runner = cloaca.DefaultRunner.with_schema(
    "postgresql://user:pass@host:5432/db",
    "tenant_schema"
)
```

## Performance Considerations

### SQLite Performance

```python
# Optimized SQLite configuration for development
sqlite_url = (
    "sqlite:///workflows.db?"
    "mode=rwc&"
    "_journal_mode=WAL&"
    "_synchronous=NORMAL&"
    "_busy_timeout=5000"
)
runner = cloaca.DefaultRunner(sqlite_url)
```

### PostgreSQL Performance

```python
# Optimized PostgreSQL configuration
pg_url = (
    "postgresql://user:pass@host:5432/cloacina?"
    "pool_min_size=5&"
    "pool_max_size=20&"
    "pool_timeout=30"
)
runner = cloaca.DefaultRunner(pg_url)
```

## Environment-Based Configuration

```python
import os

def create_runner_for_environment():
    """Create appropriate runner based on environment."""

    env = os.getenv("ENVIRONMENT", "development")

    if env == "development":
        # SQLite for local development
        return cloaca.DefaultRunner("sqlite:///dev_workflows.db")

    elif env == "testing":
        # In-memory SQLite for tests
        return cloaca.DefaultRunner("sqlite:///:memory:")

    elif env in ["staging", "production"]:
        # PostgreSQL for deployed environments
        database_url = os.getenv("DATABASE_URL")
        if not database_url:
            raise ValueError(f"DATABASE_URL required for {env} environment")
        return cloaca.DefaultRunner(database_url)

    else:
        raise ValueError(f"Unknown environment: {env}")

# Usage
runner = create_runner_for_environment()
```

## Troubleshooting

### SQLite Issues

```python
# Handle locked database
try:
    runner = cloaca.DefaultRunner("sqlite:///workflows.db")
except Exception as e:
    if "database is locked" in str(e):
        print("Database locked - check for other processes")
        # Try with WAL mode
        runner = cloaca.DefaultRunner("sqlite:///workflows.db?_journal_mode=WAL")
```

### PostgreSQL Issues

```python
# Handle connection issues
try:
    runner = cloaca.DefaultRunner("postgresql://user:pass@host:5432/db")
except Exception as e:
    if "connection refused" in str(e):
        print("PostgreSQL server not running")
    elif "authentication failed" in str(e):
        print("Check username/password")
    elif "database does not exist" in str(e):
        print("Database needs to be created")
```

## Best Practices

### Development Workflow

1. **Start with SQLite** for initial development
2. **Test with PostgreSQL** before deploying
3. **Use environment variables** for configuration
4. **Test multi-tenancy scenarios** if applicable

### Production Deployment

1. **Always use PostgreSQL** in production
2. **Use connection pooling** for better performance
3. **Configure SSL** for security
4. **Monitor connection counts** and performance
5. **Set up proper backup** and recovery procedures

## See Also

- [Quick Start Guide]({{< ref "/python-bindings/quick-start/" >}}) - Getting started with either backend
- [Multi-Tenancy Tutorial]({{< ref "/python-bindings/tutorials/06-multi-tenancy/" >}}) - PostgreSQL multi-tenant setup
- [Performance Optimization]({{< ref "/python-bindings/how-to-guides/performance-optimization/" >}}) - Optimize your chosen backend
- [Configuration Reference]({{< ref "/python-bindings/api-reference/configuration/" >}}) - Complete configuration options
