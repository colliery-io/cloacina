---
title: "cloacina::dal::unified::reactor_subscriptions"
description: "Reactor-triggered workflow subscriptions — DB-backed event log + fan-out."
sidebar:
  order: 97
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Reactor-triggered workflow subscriptions — DB-backed event log + fan-out.

Implements the data layer for CLOACI-I-0100. Two tables:
- `reactor_firings` — append-only log written by the reactor runtime
on every fire. Each row carries the same boundary cache the
in-process CG traversal consumed.
- `reactor_trigger_subscriptions` — one row per (reactor, workflow,
tenant) tuple. The poller advances `last_seen_fired_at` as it
dispatches workflows from new firings.
Watermark advance is the at-least-once contract: if the dispatcher
crashes between dispatch and watermark advance, the next poll
re-dispatches. Workflow idempotency is the user's concern (same as
cron-triggered workflows).

## Structs

### <span id="struct.ReactorFiring"></span>`struct ReactorFiring`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`

One reactor firing event. Carries the boundary cache payload the in-process CG traversal consumed; subscribers receive the same data as their workflow's input context.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `reactor_name` | `String` |  |
| `tenant_id` | `String` |  |
| `payload` | `Option < UniversalBinary >` |  |
| `fired_at` | `UniversalTimestamp` |  |
| `created_at` | `UniversalTimestamp` |  |



### <span id="struct.ReactorSubscription"></span>`struct ReactorSubscription`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`

One subscription binding a workflow to a reactor's firings.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `reactor_name` | `String` |  |
| `workflow_name` | `String` |  |
| `tenant_id` | `String` |  |
| `enabled` | `UniversalBool` |  |
| `last_seen_fired_at` | `Option < UniversalTimestamp >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |
| `predicate_expression` | `Option < String >` | CLOACI-T-0602 — optional CEL filter expression. When `Some`, the
scheduler evaluates it against the firing payload before dispatch;
`Some(_) && false` means "skip + advance watermark". `None`
preserves the original unfiltered behavior (fire on every firing). |
| `predicate_error_count` | `i32` | CLOACI-T-0922 — consecutive predicate *evaluation errors* on the
current head-of-line firing. Reset to 0 when the firing changes or
a later evaluation succeeds. |
| `predicate_error_firing_id` | `Option < UniversalUuid >` | The firing `predicate_error_count` applies to. |
| `last_predicate_error` | `Option < String >` | Truncated text of the most recent predicate evaluation error.
Never cleared on recovery — it is the forensic trail. |
| `last_predicate_error_at` | `Option < UniversalTimestamp >` | When `last_predicate_error` was recorded. |
| `predicate_degraded` | `UniversalBool` | True once a firing was dead-lettered (the consecutive-error bound
was exceeded and the watermark was force-advanced past it). Cleared
by the next successful predicate evaluation. |



### <span id="struct.ReactorSubscriptionsDAL"></span>`struct ReactorSubscriptionsDAL<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Clone`

Data access layer for reactor subscriptions + firings.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



## Functions

### <span id="fn.truncate_predicate_text"></span>`fn truncate_predicate_text`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn truncate_predicate_text (s : & str) -> String
```

Truncate `s` to `PREDICATE_TEXT_TRUNCATE_LEN` characters (char-safe), appending an ellipsis marker when anything was dropped.

<details>
<summary>Source</summary>

```rust
pub fn truncate_predicate_text(s: &str) -> String {
    if s.chars().count() <= PREDICATE_TEXT_TRUNCATE_LEN {
        return s.to_string();
    }
    let head: String = s.chars().take(PREDICATE_TEXT_TRUNCATE_LEN).collect();
    format!("{}…[truncated]", head)
}
```

</details>



### <span id="fn.compile_predicate"></span>`fn compile_predicate`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn compile_predicate (expr : & str) -> Result < cel_interpreter :: Program , String >
```

CLOACI-T-0922 — compile a CEL predicate, converting BOTH failure modes of the underlying parser into an error string.

`cel_interpreter::Program::compile` is documented to return
`Err(ParseErrors)` on malformed input, but its ANTLR-generated parser
panics outright on some inputs (`"payload.x >"` reaches an
`unreachable!()` in `antlr4rust`). That panic pre-dates this change —
the T-0602 subscribe path called `compile` bare — and it turns a typo
in a user-supplied predicate into an unwind in whatever task is
subscribing or polling. Catching it here keeps a bad predicate a
*validation error* instead of a crash, which is the same principle this
ticket applies to evaluation errors.

<details>
<summary>Source</summary>

```rust
pub fn compile_predicate(expr: &str) -> Result<cel_interpreter::Program, String> {
    match std::panic::catch_unwind(|| cel_interpreter::Program::compile(expr)) {
        Ok(Ok(program)) => Ok(program),
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err(format!(
            "could not parse CEL predicate '{}' (the CEL parser rejected it)",
            truncate_predicate_text(expr)
        )),
    }
}
```

</details>



### <span id="fn.lint_predicate_variables"></span>`fn lint_predicate_variables`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn lint_predicate_variables (expr : & str) -> Result < () , String >
```

CLOACI-T-0922 (item 4, deferred from T-0915) — reject predicates that reference identifiers outside the bound set at subscribe time.

The motivating incident: a predicate referencing a variable nobody
binds compiles fine and then evaluates false / errors forever, which —
fail-closed — silently never fires. `tennant == 'acme'` should be a
loud error at the door, not a silent no-op in production.

<details>
<summary>Source</summary>

```rust
pub fn lint_predicate_variables(expr: &str) -> Result<(), String> {
    use cel_parser::ast::{EntryExpr, Expr, IdedExpr};
    use std::collections::BTreeSet;

    // Same panic caveat as `compile_predicate` — this is the same parser.
    let parsed = match std::panic::catch_unwind(|| cel_parser::Parser::default().parse(expr)) {
        Ok(Ok(parsed)) => parsed,
        Ok(Err(e)) => return Err(format!("{}", e)),
        Err(_) => {
            return Err(format!(
                "could not parse CEL predicate '{}' (the CEL parser rejected it)",
                truncate_predicate_text(expr)
            ))
        }
    };

    fn walk(node: &IdedExpr, bound: &mut Vec<String>, free: &mut BTreeSet<String>) {
        match &node.expr {
            Expr::Unspecified | Expr::Literal(_) => {}
            Expr::Ident(name) => {
                // `@`-prefixed names are parser-internal accumulator
                // bindings and are never user-authored.
                if !name.starts_with('@') && !bound.iter().any(|b| b == name) {
                    free.insert(name.clone());
                }
            }
            Expr::Select(select) => walk(&select.operand, bound, free),
            Expr::Call(call) => {
                if let Some(target) = &call.target {
                    walk(target, bound, free);
                }
                for arg in &call.args {
                    walk(arg, bound, free);
                }
            }
            Expr::List(list) => {
                for elem in &list.elements {
                    walk(elem, bound, free);
                }
            }
            Expr::Map(map) => {
                for entry in &map.entries {
                    walk_entry(&entry.expr, bound, free);
                }
            }
            Expr::Struct(s) => {
                for entry in &s.entries {
                    walk_entry(&entry.expr, bound, free);
                }
            }
            Expr::Comprehension(comp) => {
                // The range is evaluated in the OUTER scope.
                walk(&comp.iter_range, bound, free);
                walk(&comp.accu_init, bound, free);
                let depth = bound.len();
                bound.push(comp.iter_var.clone());
                if let Some(v2) = &comp.iter_var2 {
                    bound.push(v2.clone());
                }
                bound.push(comp.accu_var.clone());
                walk(&comp.loop_cond, bound, free);
                walk(&comp.loop_step, bound, free);
                walk(&comp.result, bound, free);
                bound.truncate(depth);
            }
        }
    }

    fn walk_entry(entry: &EntryExpr, bound: &mut Vec<String>, free: &mut BTreeSet<String>) {
        match entry {
            EntryExpr::StructField(field) => walk(&field.value, bound, free),
            EntryExpr::MapEntry(e) => {
                walk(&e.key, bound, free);
                walk(&e.value, bound, free);
            }
        }
    }

    let mut bound: Vec<String> = Vec::new();
    let mut free: BTreeSet<String> = BTreeSet::new();
    walk(&parsed, &mut bound, &mut free);

    let unknown: Vec<String> = free
        .into_iter()
        .filter(|name| !PREDICATE_VARIABLES.contains(&name.as_str()))
        .collect();

    if unknown.is_empty() {
        return Ok(());
    }

    Err(format!(
        "predicate references unknown variable(s): {}. Available variables are: {}. \
         (Did you mean a field of `payload`, e.g. `payload.{}`?)",
        unknown.join(", "),
        PREDICATE_VARIABLES.join(", "),
        unknown[0],
    ))
}
```

</details>



