---
title: "Database Backends"
description: "Understanding the differences between PostgreSQL and SQLite backends in Cloacina"
weight: 25
---

# Database Backends

Cloacina supports two database backends: PostgreSQL and SQLite. This guide explains the differences between them, when to use each, and important considerations for your deployment.

## Overview

Cloacina uses a compile-time feature flag system to select between PostgreSQL and SQLite backends. This approach ensures:

- Zero runtime overhead for backend selection
- Optimal performance for each database type
- Type safety across different backends
- Consistent API regardless of backend choice

## Backend Selection

### PostgreSQL (Default)

PostgreSQL is the default backend, designed for scalable deployments:

```toml
[dependencies]
cloacina = "0.0.0-alpha.2"
# or explicitly:
cloacina = { version = "0.0.0-alpha.2", features = ["postgres"] }
```

### SQLite

SQLite backend is ideal for development and embedded deployments:

```toml
[dependencies]
cloacina = { version = "0.0.0-alpha.2", default-features = false, features = ["sqlite", "macros"] }
```

## Key Differences

### Connection Strings

**PostgreSQL:**
```rust
let executor = DefaultRunner::new("postgres://user:pass@localhost:5432/mydb").await?;
```

**SQLite:**
```rust
// File-based database with optimizations
let executor = DefaultRunner::new("myapp.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000").await?;

// In-memory database (for testing)
let executor = DefaultRunner::new(":memory:").await?;
```

### Concurrency Characteristics

| Aspect | PostgreSQL | SQLite |
|--------|------------|---------|
| Concurrent Readers | Unlimited | Unlimited (with WAL) |
| Concurrent Writers | Multiple | Single |
| Connection Pool Size | 10 (default) | 1 (recommended) |
| Lock Contention | Minimal | Can experience "database is locked" |
| Ideal For | Scalable systems, High concurrency | Local development, Embedded, Single-user |

### Feature Comparison

| Feature | PostgreSQL | SQLite |
|---------|------------|---------|
| ACID Compliance | Full | Full |
| Triggers | Yes | No (handled in application) |
| Default Timestamps | Yes | No (handled in application) |
| UUID Generation | Native | Application-generated |
| JSON Validation | Native | Via CHECK constraints |
| Performance | High for concurrent loads | High for single-user loads |
| Deployment | Requires server | Embedded, no server needed |

## SQLite Configuration

SQLite requires specific configuration for optimal performance. Here's the recommended setup:

```rust
// Recommended connection string with optimizations
let executor = DefaultRunner::new("myapp.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000").await?;

// Configuration details:
// - WAL mode: Enables concurrent readers while writing
// - Synchronous=NORMAL: Balances durability and performance
// - Busy timeout: 5s wait for locks instead of immediate failure
// - Connection pool: Use single connection (default)
```

## Implementation Details

### Timestamp Handling

**PostgreSQL** uses database-generated timestamps:
```sql
created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
```

**SQLite** requires application-generated timestamps:
```rust
let now = current_timestamp();
diesel::insert_into(table)
    .values((
        created_at.eq(&now),
        updated_at.eq(&now),
        // ... other fields
    ))
```

### UUID Handling

**PostgreSQL** can generate UUIDs:
```sql
id UUID PRIMARY KEY DEFAULT uuid_generate_v4()
```

**SQLite** requires application-generated UUIDs:
```rust
let id = UniversalUuid::new_v4();
```

## When to Use Each Backend

### Use PostgreSQL When:
- Building scalable systems
- Requiring high concurrency
- Multiple applications share the database

### Use SQLite When:
- Developing locally
- Building embedded applications
- Single-user or low-concurrency scenarios
- Simplicity is more important than scalability
- Want zero-dependency deployment


{{< hint warning >}}
**Migration Between Backends**

Cloacina does not officially support or provide tools for migrating between PostgreSQL and SQLite backends. While it may be technically possible to migrate data between backends, this process is not supported and may lead to data inconsistencies or loss.

If you need to switch backends, we recommend:
1. Starting fresh with the new backend
2. Re-implementing your data model
3. Migrating your data through your application layer

{{< /hint >}}


## Performance Considerations

### PostgreSQL Performance
- Excels at concurrent workloads
- Benefits from connection pooling
- Can utilize multiple CPU cores
- Scales horizontally

### SQLite Performance
- Extremely fast for single-threaded access
- Minimal memory overhead
- No network latency
- Entire database can fit in memory

## Conclusion

Both PostgreSQL and SQLite are first-class citizens in Cloacina. Choose based on your specific needs:

- **PostgreSQL**: Systems requiring scalability and high concurrency
- **SQLite**: Local development, testing, and embedded deployments

The compile-time backend selection ensures you get optimal performance and behavior for your chosen database, while maintaining a consistent API across both backends.
