---
id: packageable-trigger-trait-user
level: initiative
title: "Packageable Trigger Trait — User-Defined Triggers in Workflows"
short_code: "CLOACI-I-0056"
created_at: 2026-03-26T17:25:26.506653+00:00
updated_at: 2026-03-26T17:25:26.506653+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: packageable-trigger-trait-user
---

# Packageable Trigger Trait — User-Defined Triggers in Workflows

## Context

Extracted from I-0050. The existing `Trigger` trait allows defining triggers, but they aren't first-class packageable components in ManifestV2. Cron-based triggers already work but are handled specially rather than through a general packageable trait.

This initiative generalizes the trigger system so that:
- Any trigger (cron, webhook, poll, file watch, custom) implements the same packageable trait
- Triggers can be declared in ManifestV2 manifests and auto-register on package load
- Users can define custom triggers in Rust or Python and distribute them alongside workflows
- Built-in types (webhook, http_poll, file_watch) are reference implementations, not special cases

Building on prior work from I-0044. The archive branch had a working implementation of packaged triggers.

### Key Learnings from Prior Work
- `TriggerDefinitionV2` needs name, type, workflow, poll_interval, config fields
- Python triggers via `@cloaca.trigger` decorator work well
- Cron triggers should be just another implementation of the packageable trait

## Goals & Non-Goals

**Goals:**
- Generalize `Trigger` trait to be packageable via ManifestV2
- Existing cron triggers conform to the same packageable trait
- `TriggerDefinitionV2` in ManifestV2 for declaring triggers
- Auto-registration on package load
- Built-in reference implementations: WebhookTrigger, HttpPollTrigger, FileWatchTrigger
- Users can define custom triggers in Rust or Python
- Python trigger support via `@cloaca.trigger` decorator

**Non-Goals:**
- Trigger REST API and daemon CLI commands (I-0049 server/daemon)
- Native Python in core (I-0050, should be done first)
- Pipeline claiming (I-0055)
- Continuous scheduling (I-0053)

## Detailed Design

### Packageable Trigger Trait
- Generalize the existing `Trigger` trait so implementations can be discovered and loaded from packages
- `TriggerDefinitionV2` in ManifestV2 with fields: name, type, workflow, poll_interval, config
- Auto-registration on package load via the package manager
- Cron-based triggers refactored to implement the same trait

### Built-in Reference Implementations
- `WebhookTrigger` — HTTP callback, fires when endpoint receives a request
- `HttpPollTrigger` — interval-based HTTP polling, fires on status change
- `FileWatchTrigger` — filesystem notify, fires on file change

### Python Triggers
- `@cloaca.trigger` decorator for defining triggers in Python
- Evaluated in the embedded Python runtime (requires I-0050 native Python)
- Same packaging and registration flow as Rust triggers

## Prior Art

Reference implementation on `archive/cloacina-server-week1`:
- Packaged triggers: commit `da82e1b` (feat: packaged triggers — manifest-declared, auto-registered)

## Alternatives Considered

- **Fixed set of built-in triggers only**: Rejected. Users need to define domain-specific triggers (e.g., message queue, database change detection) without forking core.
- **Separate trigger registry from package registry**: Rejected. Triggers should be co-packaged with the workflows they serve.

## Implementation Plan

1. **Trait generalization** — Refactor `Trigger` trait for packageability, update cron triggers
2. **ManifestV2 integration** — `TriggerDefinitionV2`, auto-registration on package load
3. **Built-in implementations** — Webhook, HTTP poll, file watch as reference examples
4. **Python triggers** — `@cloaca.trigger` decorator (depends on I-0050)
