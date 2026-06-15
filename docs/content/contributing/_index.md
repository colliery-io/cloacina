---
title: "Contributing to Cloacina"
description: "How to contribute to Cloacina"
weight: 60
reviewer: "dstorey"
review_date: "2026-05-18"
---


Thank you for your interest in contributing to Cloacina! This guide will help you understand our contribution process and ensure your contributions are as effective as possible.

## Planning is Metis-tracked

Cloacina's planning and work tracking is done in [Metis](https://github.com/colliery-io/metis), the Flight Levels work management system. Initiatives, tasks, ADRs, and specifications live in `.metis/` and are the source of truth for what's being worked on and why. If your contribution is more than a quick fix, open a Metis initiative (or comment on an existing one) before writing code.

## Documentation nomenclature must match CLOACI-S-0011

Cloacina's nomenclature is locked by specification [`CLOACI-S-0011`](https://github.com/colliery-io/cloacina/blob/main/.metis/specifications/CLOACI-S-0011/specification.md). When writing docs or comments, never use the banned phrases (`reactive scheduler`, `reactive computation graph`, `reactive subsystem`); always use `reactor`, `computation graph`, and `traversal` per the spec. PR reviewers will flag any drift.

## Repository Features and Resources

Cloacina provides several tools and resources to help you get started and maintain high-quality contributions:

### Development Tools
- **Angreal**: Our development environment management tool that helps set up and maintain consistent development environments. Use it to ensure your local setup matches our development standards.

  **Installation**:
  ```bash
  pip install angreal
  ```

  Run `angreal tree` after install to discover the project's task tree (build, test, docs, demos). Angreal helps manage dependencies, development tools, and common tasks. It ensures all contributors have a consistent development experience.

## Before You Start

1. **Open an Issue First**
   - Before beginning any work, please open an issue to describe what you plan to do
   - This allows maintainers to provide feedback and ensure your contribution aligns with project goals
   - It also helps prevent duplicate work and ensures your time is well spent

## Making Your Contribution

1. **Development Process**
   - Fork the repository
   - Create a new branch for your feature or fix
   - Make your changes
   - Write or update tests as needed
   - Ensure all tests pass
   - Update documentation as necessary

2. **Code Quality Requirements**
   - Write clear, maintainable code
   - Include comprehensive tests for new functionality
   - Follow existing code style and patterns
   - Document technical artifacts (code comments, API documentation, etc.)

3. **Documentation**
   - Documentation is a first-class citizen in our project - it's not an afterthought
   - All contributions must include appropriate documentation updates
   - If your contribution adds new features or changes behavior, you may need to add:
     - How-to guides
     - Explanatory documentation
     - Tutorials for new users
   - **Adding a Prometheus metric?** Also update the
     [Metrics Catalog]({{< relref "/reference/metrics-catalog" >}})
     — add a row describing the metric, its labels, and one example PromQL
     query. The `angreal test metrics-format` CI job validates exposition
     format but does not validate the docs are up to date.
   - We're happy to help guide you through the documentation process, but we won't merge PRs until documentation meets our standards
   - Keep documentation clear, concise, and up-to-date
   - For detailed documentation guidelines, including API documentation and cross-linking, see our [Documentation Guidelines]({{< relref "documentation" >}})

## Submitting Your Changes

1. **Pull Request Process**
   - Submit a pull request with a clear description of your changes
   - Reference any related issues
   - Ensure CI checks pass
   - Be responsive to feedback and requested changes

2. **Review Process**
   - Maintainers will review your PR
   - They may request changes or improvements
   - Be prepared to iterate on your changes based on feedback

## Questions?

If you have any questions about contributing, feel free to:
- Comment on the relevant issue
- Reach out to maintainers
- Check existing documentation

Thank you for helping make Cloacina better!
