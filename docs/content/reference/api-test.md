---
title: "API Link Testing"
description: "Testing cross-linking between Hugo and API docs"
date: 2024-03-20
weight: 2
menu:
  main:
    weight: 2
---

# Testing API Cross-Links

This page tests various ways of linking to the API documentation.

## Basic Links
- Link to main crate: {{< api-link path="cloacina" >}}
- Link to a module: {{< api-link path="cloacina::models" >}}
- Link to a specific type: {{< api-link path="cloacina::models::task_execution" >}}

## Common Patterns
- Link to a macro: {{< api-link path="cloacina::workflow" >}}
- Link to a trait: {{< api-link path="cloacina::task" >}}
- Link with custom display: {{< api-link path="cloacina::models" display="Data Models" >}}
