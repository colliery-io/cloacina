---
title: "cloacina::registry::storage"
description: "Storage backend implementations for the workflow registry."
sidebar:
  order: 187
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Storage backend implementations for the workflow registry.

This module provides storage backends for persisting workflow binaries:
- `UnifiedRegistryStorage` - Database storage (PostgreSQL or SQLite)
- `FilesystemRegistryStorage` - Filesystem-based storage

