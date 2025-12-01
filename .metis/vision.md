---
id: cloacina-embedded-workflow
level: vision
title: "Cloacina: Embedded Workflow Orchestration"
short_code: "CLOACI-V-0001"
created_at: 2025-10-15T20:20:16.636881+00:00
updated_at: 2025-11-30T02:01:24.012272+00:00
archived: false

tags:
  - "#vision"
  - "#phase/published"


exit_criteria_met: false
strategy_id: NULL
initiative_id: NULL
---

# Cloacina Vision

## Purpose

Cloacina exists to provide developers with an embedded, resilient workflow orchestration framework that integrates directly into Rust and Python applications. Unlike standalone orchestration services that add operational complexity and network dependencies, Cloacina delivers enterprise-grade task pipeline capabilities as a library, enabling complex multi-step workflows with automatic retry, state persistence, and dependency resolution without external infrastructure.

## Product/Solution Overview

**Target Audience**: Backend developers building data processing applications, background job systems, ETL pipelines, or complex integration workflows in Rust or Python.

**Key Benefits**:

- Zero external dependencies (unless you choose to use postgres resilience)
- Compile-time type safety for workflow definitions (Rust)
- Guaranteed execution with two-phase commit and automatic recovery
- Multi-tenant isolation through database schema separation
- Cross-language support with shared execution engine

## Current State

Cloacina is a functional workflow orchestration library with:

- Core Rust implementation with procedural macros for task/workflow definition
- PostgreSQL and SQLite backend support
- Python bindings (Cloaca) via PyO3
- Cron scheduling with guaranteed execution architecture
- Multi-tenancy via PostgreSQL schema isolation
- Content-based workflow versioning
- Comprehensive documentation and tutorials

The library is published on crates.io and functional for production use cases, though several hardening initiatives remain in progress.

## Future State

A mature, production-hardened workflow orchestration library that is:

- The go-to choice for embedded workflow orchestration in Rust applications
- Fully hardened with proper error handling, security controls, and performance optimizations
- Well-documented with clear migration paths between versions
- Actively maintained with a growing community of contributors

## Major Features

- **Embedded Framework**: Integrates as a library rather than a separate service, reducing operational complexity
- **Resilient Execution**: Two-phase commit pattern ensures no scheduled executions are lost, with automatic recovery for failed handoffs
- **Type-Safe Workflows**: Procedural macros provide compile-time validation of task dependencies and data flow
- **Database-Backed State**: PostgreSQL or SQLite for reliable state management with cross-database compatibility
- **Multi-Tenant Ready**: PostgreSQL schema-based isolation provides complete tenant separation with zero query overhead
- **Async-First**: Built on tokio for high-performance concurrent execution
- **Content-Versioned**: Automatic workflow versioning based on task code and structure enables safe workflow evolution
- **Cross-Language**: Python bindings share the same core engine, allowing mixed-language deployments

## Principles

- **Embedded Over External**: Prefer library integration over service deployment
- **Reliability Over Speed**: Guaranteed execution matters more than raw throughput
- **Safety First**: Rust's type system and ownership model should prevent entire classes of bugs
- **Database Portability**: Support both PostgreSQL and SQLite without code changes
- **Explicit Over Implicit**: Workflow definitions should be clear and auditable

## Constraints

- Rust stable toolchain compatibility required
- Must support both PostgreSQL and SQLite backends
- Python bindings must maintain API compatibility with Rust equivalents
- No runtime dependencies beyond the database connection
- Must remain embeddable (no background processes or external services required)
