---
title: "Trigger Rules"
description: "A comprehensive guide to understanding and using Cloacina's trigger rules system"
date: 2024-03-19
weight: 3
---


## Introduction

Trigger rules are a fundamental feature in Cloacina that determine when tasks should execute beyond simple dependency satisfaction. They provide a powerful way to implement complex workflow logic and conditional execution patterns.

## Core Concepts

### What are Trigger Rules?

Trigger rules are conditions that determine whether a task should execute after its dependencies are satisfied. They provide:

- Conditional task execution based on workflow state
- Complex decision-making logic
- Integration with workflow context
- Flexible composition of conditions

### Default Behavior

By default, tasks use the "Always" trigger rule, meaning they execute whenever their dependencies are satisfied. This is the simplest and most common case.

## Rule Constructs

### Task Status Rules
These rules evaluate the status of other tasks in the workflow:

- `TaskSuccess`: Execute when a specific task completes successfully
- `TaskFailed`: Execute when a specific task fails
- `TaskSkipped`: Execute when a specific task is skipped

{{< api-link path="cloacina::scheduler::TriggerCondition" type="enum" display="TriggerCondition" >}}

### Context Value Rules
These rules evaluate values in the workflow context:

- `Equals`: Value matches exactly
- `NotEquals`: Value doesn't match
- `GreaterThan`: Value is greater than expected
- `LessThan`: Value is less than expected
- `Contains`: Value contains the expected value
- `NotContains`: Value doesn't contain the expected value
- `Exists`: Value exists in context
- `NotExists`: Value doesn't exist in context

{{< api-link path="cloacina::scheduler::ValueOperator" type="enum" display="ValueOperator" >}}

### Logical Operators
Rules can be combined using logical operators:

- `All`: All conditions must be true
- `Any`: Any condition can be true
- `None`: No conditions should be true

{{< api-link path="cloacina::scheduler::TriggerRule" type="enum" display="TriggerRule" >}}

For practical examples of combining different types of rules, see our [Error Handling Tutorial](/tutorials/04-error-handling/) which demonstrates:
- Combining task status rules for fallback behavior
- Using context values for conditional execution
- Complex rule composition patterns

## Operational Considerations

1. Context Management
   - Don't use trigger rules as a substitute for proper task dependencies
   - Avoid using context values that could be expressed as task dependencies
   - Keep workflow structure clear and explicit

2. Rule Evolution
   - Keep rules focused on business outcomes and workflow paths
   - Document the business logic behind complex rules
   - Consider how rules will evolve with business requirements

3. Rule Design
   - Start with the default "Always" rule and add conditions only when necessary
   - Use context values for dynamic decisions, keeping keys consistent
   - Document expected context values and their impact on workflow paths
   - Consider task dependencies carefully when using status-based rules

## Conclusion

Trigger rules provide a flexible way to control task execution based on workflow state and context. By combining task status rules, context value rules, and logical operators, you can create sophisticated workflow patterns that adapt to your business needs. Remember to consider the operational aspects of rule design and maintenance to ensure your workflows remain clear and maintainable over time.
