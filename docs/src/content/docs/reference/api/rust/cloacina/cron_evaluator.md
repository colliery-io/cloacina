---
title: "cloacina::cron_evaluator"
description: "T-0553 / I-0102: timezone-aware cron expression evaluator relocated to `cloacina-workflow` so packaged cdylibs (which depend on `cloacina-workflow` but not on `cloacina`) can use it from the `#[trigger(cron = \"...\")]` macro emission. Engine paths re-export."
editUrl: false
sidebar:
  order: 55
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

T-0553 / I-0102: timezone-aware cron expression evaluator relocated to `cloacina-workflow` so packaged cdylibs (which depend on `cloacina-workflow` but not on `cloacina`) can use it from the `#[trigger(cron = "...")]` macro emission. Engine paths re-export.

## Re-exports

- [`CronError`](../../../rust/cloacina-workflow/cron_evaluator/#enum.CronError)
- [`CronEvaluator`](../../../rust/cloacina-workflow/cron_evaluator/#struct.CronEvaluator)
