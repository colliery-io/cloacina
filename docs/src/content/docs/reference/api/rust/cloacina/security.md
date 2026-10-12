---
title: "cloacina::security"
description: "Security module for package signing and key management."
sidebar:
  order: 205
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Security module for package signing and key management.

This module provides:
- [`KeyManager`] trait for managing signing keys and trust relationships
- [`DbKeyManager`] database-backed implementation
- Key generation, encryption, and PEM export/import
- Security audit logging for SIEM integration

