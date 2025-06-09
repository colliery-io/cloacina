---
title: "Cloacina"
description: "Documentation for the Cloacina project"
---

# Cloacina Documentation

Welcome to the Cloacina documentation. This documentation is organized to help you find the information you need quickly and efficiently.

## About Cloacina

Cloacina is a workflow orchestration engine that helps you build resilient task pipelines directly within your applications. Unlike standalone orchestration services, Cloacina embeds into your existing applications to manage complex multi-step workflows with:

- Automatic retries and failure recovery
- State persistence
- Type-safe workflows
- Database-backed execution
- Async-first design
- Content versioning

Whether you're building data processing applications, background job systems, or complex integration workflows, Cloacina provides the tools you need to make your task pipelines reliable, maintainable, and scalable.

## Available Libraries

Cloacina provides libraries for multiple programming languages:

- **[Cloacina]({{< ref "/tutorials/" >}})**: Native Rust library for maximum performance and type safety
- **[Cloaca]({{< ref "/python-bindings/" >}})**: Python bindings providing the same powerful features with a Pythonic interface

Both libraries share the same core engine and can even share the same database, allowing you to use the best tool for each part of your system.
