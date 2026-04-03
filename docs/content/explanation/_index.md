---
title: "Explanation"
description: "Detailed explanations of Cloacina concepts and architecture"
weight: 50
---

# Explanations

Explanation documents are understanding-oriented. They provide context, background, and reasoning to help you understand why Cloacina works the way it does. Where tutorials teach you how and reference docs tell you what, explanations answer "why" -- covering design decisions, architectural trade-offs, and the thinking behind the system's behavior.

## Core Concepts

- **[Context Management]({{< ref "context-management" >}})** -- How task context flows through workflows and how data is shared between tasks
- **[Workflow Versioning]({{< ref "workflow-versioning" >}})** -- Version management for workflows and how changes are tracked
- **[Trigger Rules]({{< ref "trigger-rules" >}})** -- Conditional execution logic that controls when tasks run based on outcomes and context
- **[Macro System]({{< ref "macro-system" >}})** -- How the `#[task]` and `#[workflow]` attribute macros work under the hood
- **[Task Deferral]({{< ref "task-deferral" >}})** -- How TaskHandle and defer_until manage concurrency slots during long-running waits

## Architecture

- **[Task Execution Sequence]({{< ref "task-execution-sequence" >}})** -- Step-by-step walkthrough of how a task moves from scheduling to completion
- **[Dispatcher Architecture]({{< ref "dispatcher-architecture" >}})** -- Design of the task dispatcher and its role in the execution pipeline
- **[Guaranteed Execution Architecture]({{< ref "guaranteed-execution-architecture" >}})** -- How Cloacina ensures tasks run exactly once and recovers from failures

## Scheduling

- **[Cron Scheduling]({{< ref "cron-scheduling" >}})** -- Design and implementation of time-based workflow triggers
- **[Guaranteed Execution Architecture]({{< ref "guaranteed-execution-architecture" >}})** -- Recovery and deduplication mechanisms for scheduled workflows

## Packaging

- **[Package Format]({{< ref "package-format" >}})** -- Structure and contents of packaged workflow bundles
- **[Packaged Workflow Architecture]({{< ref "packaged-workflow-architecture" >}})** -- How packaged workflows are loaded, verified, and executed
- **[FFI System]({{< ref "ffi-system" >}})** -- Foreign function interface design for cross-language workflow execution

## Data

- **[Database Backends]({{< ref "database-backends" >}})** -- PostgreSQL and SQLite backend differences, trade-offs, and configuration
- **[Multi-Tenancy]({{< ref "multi-tenancy" >}})** -- Schema-based tenant isolation architecture and design decisions
- **[Performance Characteristics]({{< ref "performance-characteristics" >}})** -- Understanding the performance test suite and tuning guidance
