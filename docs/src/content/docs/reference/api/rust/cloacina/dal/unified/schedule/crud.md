---
title: "cloacina::dal::unified::schedule::crud"
description: "Backend-divergent CRUD operations for unified schedules (CLOACI-I-0135)."
editUrl: false
sidebar:
  order: 84
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Backend-divergent CRUD operations for unified schedules (CLOACI-I-0135).

Every other schedule DAL method is backend-agnostic diesel and lives inline
in the public methods (`mod.rs`) via `interact_on_backend!`. `claim_and_update_cron`
is the one method whose bodies genuinely diverge by backend: the Postgres arm
issues `SET TRANSACTION ISOLATION LEVEL SERIALIZABLE` to make the claim
atomic under concurrent schedulers, which has no SQLite equivalent (SQLite's
single-writer model already serializes the update). It therefore stays an
explicit `*_postgres`/`*_sqlite` twin pair.
