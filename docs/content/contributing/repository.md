---
title: "Repository Organization"
description: "Understanding the Cloacina repository structure"
weight: 62
reviewer: "dstorey"
review_date: "2024-03-19"
---


This guide explains how the Cloacina repository is organized to support both development and documentation needs.

## Project Overview

Cloacina is organized as a Rust workspace containing multiple crates that work together. The main components are:

- `cloacina/` - The core library implementation
- `cloacina-macros/` - Procedural macros used by the core library
- `examples/` - Standalone example implementations
- `.angreal/` - Development environment configuration and automation scripts

{{< hint info >}}
**Note:** All crates in the workspace share the same version number to ensure coordinated releases and compatibility.
{{< /hint >}}

## Testing Strategy

Cloacina employs a comprehensive testing approach:

1. **Unit Tests**
   - Located alongside the code they test in `cloacina/src/` and `cloacina-macros/src/`
   - Test individual components and functions in isolation
   - Run with `angreal tests unit`

2. **Integration Tests**
   - Located in `cloacina/tests/`
   - Test how different components work together
   - Include test utilities and helpers
   - Run with `angreal tests integration`

3. **End-to-End Tests**
   - Implemented as executable examples in the `examples/` directory
   - Tutorials that are executed via angreal
   - Demonstrate complete usage patterns and workflows
   - Run with `angreal examples all`


## Documentation Structure

Our documentation follows the [Diátaxis Framework](https://diataxis.fr/) and is organized into:
- `docs/content/tutorials/` - Step-by-step guides and examples that teach Cloacina features
- `docs/content/how-to/` - Task-oriented guides for specific operations
- `docs/content/reference/` - Technical reference and API documentation
- `docs/content/explanation/` - Conceptual documentation and deep dives

All documentation, including Rust documentation, is automatically bundled into a single site for easy reference. This ensures you have access to all project documentation in one place.

For detailed information about documentation standards and practices, please refer to the [Documentation Guide](./documentation.md).
