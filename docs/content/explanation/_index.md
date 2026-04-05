---
title: "Explanation"
description: "Detailed explanations of Cloacina concepts and architecture"
weight: 50
---

# Explanations

Explanation documents answer "why" — covering design decisions, architectural trade-offs, and the thinking behind the system's behavior.

## [Workflows]({{< ref "workflows" >}})

Architecture of the workflow orchestration system — the unified scheduler, task execution, context management, dispatching, trigger rules, and cron scheduling.

## [Computation Graphs]({{< ref "computation-graphs" >}})

Architecture of the reactive computation graph system — accumulators, reactors, compiled graph execution, and the reactive scheduler.

## [Platform]({{< ref "platform" >}})

Cross-cutting concerns shared by both systems — database backends, packaging, multi-tenancy, FFI, and performance.

{{< toc-tree >}}
