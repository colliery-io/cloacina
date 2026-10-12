---
title: "cloacina::runner::default_runner::reactor_subscriptions_api"
description: "Reactor-subscription API for the DefaultRunner (CLOACI-I-0100 / T-0600)."
sidebar:
  order: 199
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Reactor-subscription API for the DefaultRunner (CLOACI-I-0100 / T-0600).

Surfaces a thin user-facing wrapper over the
`ReactorSubscriptionsDAL::subscribe / unsubscribe / list` operations.
The unfiltered registration path — "fire workflow on every reactor
firing for this tenant" — lives here. The optional Python filter
callback (`@trigger(reactor=...)`) is a follow-up surface.

