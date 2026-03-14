---
title: "Architecture"
description: "C4 architecture diagrams and documentation for the Cloacina system"
weight: 1
---

# Architecture

This section documents Cloacina's architecture using the [C4 model](https://c4model.com/) — a hierarchical approach to software architecture documentation created by Simon Brown.

## Diagram Levels

The C4 model provides four levels of abstraction, each serving a different audience:

| Level | Name | Audience | Shows |
|-------|------|----------|-------|
| **L1** | System Context | Everyone | Cloacina's place in the world — who uses it and what it connects to |
| **L2** | Container | Developers & Operators | The major deployable units (crates, bindings, CLI tools) |
| **L3** | Component | Developers | Internal components within each container |
| **L4** | Code | Developers | Key traits, types, and contracts |

## How to Read These Diagrams

Start at Level 1 for the big picture, then drill into the container or subsystem you're interested in. Each level links to its children for easy navigation.
