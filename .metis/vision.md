---
id: cloacina-embedded-workflow
level: vision
title: "Cloacina: Workflow Orchestration Platform"
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

Cloacina is a workflow orchestration platform that offers two deployment models:

1. **Embedded Library**: Integrate directly into Rust and Python applications for zero-dependency workflow orchestration
2. **Deployable Service**: Run as centralized infrastructure for teams to author, distribute, execute, and operate workflows

Both models share the same core engine, providing enterprise-grade task pipeline capabilities with automatic retry, state persistence, dependency resolution, and multi-tenant isolation. Teams can start embedded and graduate to service deployment as their needs grow, or run both models simultaneously.

## Product/Solution Overview

**Target Audiences**:

- **Application Developers**: Building data processing applications, background job systems, ETL pipelines, or integration workflows in Rust or Python (embedded mode)
- **Platform Teams**: Providing workflow infrastructure for multiple teams or organizations (service mode)
- **Implementation Partners**: Running managed workflow services for clients (service mode with multi-tenancy)

**Key Benefits**:

- Zero external dependencies for embedded use (Postgres optional for resilience)
- Compile-time type safety for workflow definitions (Rust)
- Guaranteed execution with two-phase commit and automatic recovery
- Multi-tenant isolation through database schema separation
- Cross-language support with shared execution engine
- Horizontal scaling via stateless workers and pipeline-claiming schedulers
- Local authoring and testing with server deployment for production

## Current State

**Embedded Library (Production Ready)**:
- Core Rust implementation with procedural macros for task/workflow definition
- PostgreSQL and SQLite backend support
- Python bindings (Cloaca) via PyO3
- Cron scheduling with guaranteed execution architecture
- Multi-tenancy via PostgreSQL schema isolation
- Content-based workflow versioning
- Event-based triggers for workflow activation
- Comprehensive documentation and tutorials

The library is published on crates.io and functional for production use cases.

**Deployable Service (Not Yet Implemented)**:
- Architecture designed: modular monolith with API/scheduler/worker modes
- Scaling model defined: stateless workers, pipeline-claiming schedulers
- No server binary, API surface, or deployment artifacts exist yet

## Future State

A mature workflow orchestration platform offering both embedded and service deployment:

**Library**:
- The go-to choice for embedded workflow orchestration in Rust applications
- Fully hardened with proper error handling, security controls, and performance optimizations
- Well-documented with clear migration paths between versions

**Service**:
- Easy-to-deploy server binary with modular architecture (all-in-one or scaled components)
- REST/gRPC API for workflow submission, status queries, and management
- Multi-tenant by default with schema-based isolation
- Horizontally scalable workers and schedulers
- Production-ready deployment artifacts (Docker, Helm)
- Observability built-in (metrics, tracing, structured logging)

**Ecosystem**:
- CLI tooling for local authoring, testing, compilation, and deployment
- Actively maintained with a growing community of contributors

## Major Features

**Core Engine** (shared by library and service):
- **Resilient Execution**: Two-phase commit pattern ensures no scheduled executions are lost, with automatic recovery for failed handoffs
- **Type-Safe Workflows**: Procedural macros provide compile-time validation of task dependencies and data flow
- **Database-Backed State**: PostgreSQL or SQLite for reliable state management with cross-database compatibility
- **Multi-Tenant Ready**: PostgreSQL schema-based isolation provides complete tenant separation with zero query overhead
- **Async-First**: Built on tokio for high-performance concurrent execution
- **Content-Versioned**: Automatic workflow versioning based on task code and structure enables safe workflow evolution
- **Cross-Language**: Python bindings share the same core engine, allowing mixed-language deployments

**Embedded Library**:
- **Zero Dependencies**: Integrates as a library with no external services required
- **Flexible Backends**: SQLite for simple deployments, PostgreSQL for scale

**Deployable Service**:
- **Modular Monolith**: Single binary runs as all-in-one or in dedicated modes (api, scheduler, worker)
- **Stateless Workers**: Scale horizontally by adding worker instances
- **Pipeline-Claiming Schedulers**: Multiple schedulers share work via database-level coordination
- **API Surface**: REST and/or gRPC for workflow submission and management
- **Local-to-Server Workflow**: Author and test locally, compile and upload to server for execution

## Principles

- **Flexibility in Deployment**: Support both embedded library and service deployment - let users choose
- **Reliability Over Speed**: Guaranteed execution matters more than raw throughput
- **Safety First**: Rust's type system and ownership model should prevent entire classes of bugs
- **Database as Coordination**: All state and coordination through the database - no external services required
- **Stateless Components**: Workers and schedulers hold no persistent state, enabling horizontal scaling
- **Explicit Over Implicit**: Workflow definitions should be clear and auditable
- **Local Development First**: Author and test workflows locally before deploying to servers

## Constraints

- Rust stable toolchain compatibility required
- Must support both PostgreSQL and SQLite backends (service mode requires PostgreSQL for multi-tenancy)
- Python bindings must maintain API compatibility with Rust equivalents
- No runtime dependencies beyond the database connection (no Redis, no message queues, no coordination services)
- Library mode must remain embeddable with no background processes required
- Service mode must be deployable as a single binary with optional horizontal scaling
