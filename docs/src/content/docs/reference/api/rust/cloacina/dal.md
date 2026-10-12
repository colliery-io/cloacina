---
title: "cloacina::dal"
description: "Data Access Layer with runtime backend selection"
sidebar:
  order: 78
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Data Access Layer with runtime backend selection

This module provides storage-specific DAL implementations:
- unified: Runtime backend selection (PostgreSQL or SQLite)
- filesystem_dal: For filesystem-based storage operations

