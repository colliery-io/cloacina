---
title: "Reference"
description: "Technical reference documentation for Cloacina"
weight: 40
---

# Reference

This section contains information-oriented technical descriptions of the Cloacina framework. These pages are structured for lookup rather than sequential reading. They document exact interfaces, configuration options, error types, and API contracts.

## Contents

| Document | Description |
|---|---|
| [CLI Reference]({{< ref "cli" >}}) | Complete command reference for `cloacinactl` -- flags, subcommands, environment variables, and config file schema |
| [HTTP API]({{< ref "http-api" >}}) | REST API endpoints exposed by `cloacinactl serve` -- authentication, request/response formats, status codes |
| [Configuration]({{< ref "configuration" >}}) | `DefaultRunnerConfig` fields, builder methods, config.toml schema, and environment variables |
| [Macros]({{< ref "macros" >}}) | `#[task]`, `#[workflow]`, and `#[trigger]` attribute macros -- every attribute, code fingerprinting, compile-time validation |
| [Errors]({{< ref "errors" >}}) | Every error enum and variant in the framework with descriptions and common causes |
| [Repository Structure]({{< ref "repository-structure" >}}) | Overview of the Cloacina repository organization and crate architecture |
| [DatabaseAdmin API]({{< ref "database-admin" >}}) | API documentation for per-tenant database credential management |
| [cloacina-testing API]({{< ref "testing-crate" >}}) | API documentation for the `cloacina-testing` crate -- no-database test utilities for workflows |
