/*
 *  Copyright 2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! Reactor-triggered workflow subscriptions — DB-backed event log + fan-out.
//!
//! Implements the data layer for CLOACI-I-0100. Two tables:
//!
//! - `reactor_firings` — append-only log written by the reactor runtime
//!   on every fire. Each row carries the same boundary cache the
//!   in-process CG traversal consumed.
//! - `reactor_trigger_subscriptions` — one row per (reactor, workflow,
//!   tenant) tuple. The poller advances `last_seen_fired_at` as it
//!   dispatches workflows from new firings.
//!
//! Watermark advance is the at-least-once contract: if the dispatcher
//! crashes between dispatch and watermark advance, the next poll
//! re-dispatches. Workflow idempotency is the user's concern (same as
//! cron-triggered workflows).

use super::DAL;
use crate::database::schema::unified::{reactor_firings, reactor_trigger_subscriptions};
use crate::database::universal_types::{
    UniversalBinary, UniversalBool, UniversalTimestamp, UniversalUuid,
};
use crate::error::ValidationError;
use diesel::prelude::*;
use uuid::Uuid;

/// One reactor firing event. Carries the boundary cache payload the
/// in-process CG traversal consumed; subscribers receive the same data
/// as their workflow's input context.
#[derive(Debug, Clone, Queryable)]
pub struct ReactorFiring {
    pub id: UniversalUuid,
    pub reactor_name: String,
    pub tenant_id: String,
    pub payload: Option<UniversalBinary>,
    pub fired_at: UniversalTimestamp,
    pub created_at: UniversalTimestamp,
}

/// One subscription binding a workflow to a reactor's firings.
#[derive(Debug, Clone, Queryable)]
pub struct ReactorSubscription {
    pub id: UniversalUuid,
    pub reactor_name: String,
    pub workflow_name: String,
    pub tenant_id: String,
    pub enabled: UniversalBool,
    pub last_seen_fired_at: Option<UniversalTimestamp>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
    /// CLOACI-T-0602 — optional CEL filter expression. When `Some`, the
    /// scheduler evaluates it against the firing payload before dispatch;
    /// `Some(_) && false` means "skip + advance watermark". `None`
    /// preserves the original unfiltered behavior (fire on every firing).
    pub predicate_expression: Option<String>,
    /// CLOACI-T-0922 — consecutive predicate *evaluation errors* on the
    /// current head-of-line firing. Reset to 0 when the firing changes or
    /// a later evaluation succeeds.
    pub predicate_error_count: i32,
    /// The firing `predicate_error_count` applies to.
    pub predicate_error_firing_id: Option<UniversalUuid>,
    /// Truncated text of the most recent predicate evaluation error.
    /// Never cleared on recovery — it is the forensic trail.
    pub last_predicate_error: Option<String>,
    /// When `last_predicate_error` was recorded.
    pub last_predicate_error_at: Option<UniversalTimestamp>,
    /// True once a firing was dead-lettered (the consecutive-error bound
    /// was exceeded and the watermark was force-advanced past it). Cleared
    /// by the next successful predicate evaluation.
    pub predicate_degraded: UniversalBool,
}

/// CLOACI-T-0922 — the complete set of variables the reactor predicate
/// evaluator binds into the CEL context. Kept here (next to the
/// subscribe-time validation that enforces it) so the lint and the
/// evaluator cannot drift apart; `cron_trigger_scheduler::
/// eval_cel_predicate_program` binds exactly these names.
pub const PREDICATE_VARIABLES: [&str; 3] = ["payload", "reactor", "tenant"];

/// Max characters of predicate/error text we persist or log. Predicates
/// are user-authored and unbounded; log lines and DB columns are not.
pub const PREDICATE_TEXT_TRUNCATE_LEN: usize = 240;

/// Truncate `s` to `PREDICATE_TEXT_TRUNCATE_LEN` characters (char-safe),
/// appending an ellipsis marker when anything was dropped.
pub fn truncate_predicate_text(s: &str) -> String {
    if s.chars().count() <= PREDICATE_TEXT_TRUNCATE_LEN {
        return s.to_string();
    }
    let head: String = s.chars().take(PREDICATE_TEXT_TRUNCATE_LEN).collect();
    format!("{}…[truncated]", head)
}

/// CLOACI-T-0922 — compile a CEL predicate, converting BOTH failure modes
/// of the underlying parser into an error string.
///
/// `cel_interpreter::Program::compile` is documented to return
/// `Err(ParseErrors)` on malformed input, but its ANTLR-generated parser
/// panics outright on some inputs (`"payload.x >"` reaches an
/// `unreachable!()` in `antlr4rust`). That panic pre-dates this change —
/// the T-0602 subscribe path called `compile` bare — and it turns a typo
/// in a user-supplied predicate into an unwind in whatever task is
/// subscribing or polling. Catching it here keeps a bad predicate a
/// *validation error* instead of a crash, which is the same principle this
/// ticket applies to evaluation errors.
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

/// CLOACI-T-0922 (item 4, deferred from T-0915) — reject predicates that
/// reference identifiers outside the bound set at subscribe time.
///
/// The motivating incident: a predicate referencing a variable nobody
/// binds compiles fine and then evaluates false / errors forever, which —
/// fail-closed — silently never fires. `tennant == 'acme'` should be a
/// loud error at the door, not a silent no-op in production.
///
/// # Why the AST walk instead of `Program::references()`
///
/// `cel_interpreter::Program::references()` does report referenced
/// variables, but CEL's comprehension macros (`exists`, `all`, `map`,
/// `filter`, `exists_one`) are expanded by the parser into a
/// `Comprehension` node whose iteration variable appears as a bare
/// `Ident` inside the loop body. `references()` therefore reports the
/// iteration variable as if it were a free variable, and a lint built on
/// it would reject the perfectly valid
/// `payload.items.exists(i, i.price > 100)`. We walk the AST ourselves so
/// comprehension-bound names (`iter_var`, `iter_var2`, `accu_var`) are
/// treated as bound. Erring toward acceptance is deliberate: a false
/// rejection is worse than a missed lint.
///
/// Returns `Ok(())` when every free identifier is in
/// [`PREDICATE_VARIABLES`]; otherwise an error naming the offenders and
/// the allowed set.
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

/// Data access layer for reactor subscriptions + firings.
#[derive(Clone)]
pub struct ReactorSubscriptionsDAL<'a> {
    dal: &'a DAL,
}

impl<'a> ReactorSubscriptionsDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    // ─────────────────────────────────────────────────────────────────
    // Firings
    // ─────────────────────────────────────────────────────────────────

    /// Insert a firing row. Called by the reactor runtime on every
    /// fire; best-effort from the caller's perspective (a DAL failure
    /// is logged but doesn't fail the in-process CG dispatch).
    pub async fn insert_firing(
        &self,
        reactor: &str,
        tenant: &str,
        payload: Option<Vec<u8>>,
        fired_at: UniversalTimestamp,
    ) -> Result<Uuid, ValidationError> {
        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();
        let reactor = reactor.to_string();
        let tenant = tenant.to_string();
        let id_for_move = id;
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(reactor_firings::table)
                .values((
                    reactor_firings::id.eq(id_for_move),
                    reactor_firings::reactor_name.eq(reactor),
                    reactor_firings::tenant_id.eq(tenant),
                    reactor_firings::payload.eq(payload.map(UniversalBinary::new)),
                    reactor_firings::fired_at.eq(fired_at),
                    reactor_firings::created_at.eq(now),
                ))
                .execute(conn)
        })?;
        Ok(id.0)
    }

    /// Poll firings for a subscription. Returns rows strictly newer
    /// than `after`, in `fired_at` order, capped at `limit`. The
    /// caller advances the watermark as it dispatches each row.
    pub async fn poll_unconsumed(
        &self,
        tenant: &str,
        reactor: &str,
        after: Option<UniversalTimestamp>,
        limit: i64,
    ) -> Result<Vec<ReactorFiring>, ValidationError> {
        let tenant = tenant.to_string();
        let reactor = reactor.to_string();
        let rows: Vec<ReactorFiring> = crate::interact_on_backend!(self.dal, |conn| {
            let mut q = reactor_firings::table
                .filter(reactor_firings::tenant_id.eq(tenant))
                .filter(reactor_firings::reactor_name.eq(reactor))
                .into_boxed();
            if let Some(after) = after {
                q = q.filter(reactor_firings::fired_at.gt(after));
            }
            q.order(reactor_firings::fired_at.asc())
                .limit(limit)
                .load::<ReactorFiring>(conn)
        })?;
        Ok(rows)
    }

    /// TTL prune. Deletes firings whose `fired_at` is older than the
    /// cutoff. Returns the row count deleted.
    pub async fn prune_firings_older_than(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<usize, ValidationError> {
        let n = crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(reactor_firings::table.filter(reactor_firings::fired_at.lt(cutoff)))
                .execute(conn)
        })?;
        Ok(n)
    }

    // ─────────────────────────────────────────────────────────────────
    // Subscriptions
    // ─────────────────────────────────────────────────────────────────

    /// Create a subscription. Idempotent: calling twice with the same
    /// `(reactor, workflow, tenant)` upserts; the second call's
    /// `predicate` (if any) replaces the first one's.
    ///
    /// `predicate` is an optional CEL expression (CLOACI-T-0602). When
    /// `Some(_)`, the expression is compiled at subscribe time and any
    /// syntax error is returned as a `ValidationError` before the row is
    /// written, so a bad expression never lands in the DB. The scheduler
    /// re-compiles + caches at dispatch time.
    pub async fn subscribe(
        &self,
        reactor: &str,
        workflow: &str,
        tenant: &str,
        predicate: Option<&str>,
    ) -> Result<Uuid, ValidationError> {
        if let Some(expr) = predicate {
            // Compile-time validation: reject malformed expressions before
            // they reach the DB. Cheap (single parse), centralizes the
            // error message at the API boundary.
            compile_predicate(expr).map_err(ValidationError::InvalidPredicate)?;
            // CLOACI-T-0922 — and reject well-formed expressions that
            // reference variables nobody binds. Those compile fine and then
            // silently never fire (fail-closed), which is exactly the class
            // of bug the `tenant` stub incident (T-0915) produced.
            lint_predicate_variables(expr).map_err(ValidationError::InvalidPredicate)?;
        }
        let predicate = predicate.map(str::to_string);
        crate::dispatch_backend!(
            self.dal.backend(),
            self.subscribe_postgres(reactor, workflow, tenant, predicate.clone())
                .await,
            self.subscribe_sqlite(reactor, workflow, tenant, predicate)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn subscribe_postgres(
        &self,
        reactor: &str,
        workflow: &str,
        tenant: &str,
        predicate: Option<String>,
    ) -> Result<Uuid, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();
        let reactor = reactor.to_string();
        let workflow = workflow.to_string();
        let tenant = tenant.to_string();
        let predicate_for_update = predicate.clone();
        let row: ReactorSubscription = conn
            .interact(move |conn| {
                diesel::insert_into(reactor_trigger_subscriptions::table)
                    .values((
                        reactor_trigger_subscriptions::id.eq(id),
                        reactor_trigger_subscriptions::reactor_name.eq(&reactor),
                        reactor_trigger_subscriptions::workflow_name.eq(&workflow),
                        reactor_trigger_subscriptions::tenant_id.eq(&tenant),
                        reactor_trigger_subscriptions::enabled.eq(UniversalBool::from(true)),
                        reactor_trigger_subscriptions::created_at.eq(now),
                        reactor_trigger_subscriptions::updated_at.eq(now),
                        reactor_trigger_subscriptions::predicate_expression.eq(&predicate),
                    ))
                    .on_conflict((
                        reactor_trigger_subscriptions::reactor_name,
                        reactor_trigger_subscriptions::workflow_name,
                        reactor_trigger_subscriptions::tenant_id,
                    ))
                    .do_update()
                    .set((
                        reactor_trigger_subscriptions::updated_at.eq(now),
                        reactor_trigger_subscriptions::predicate_expression
                            .eq(&predicate_for_update),
                    ))
                    .get_result::<ReactorSubscription>(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(row.id.0)
    }

    #[cfg(feature = "sqlite")]
    async fn subscribe_sqlite(
        &self,
        reactor: &str,
        workflow: &str,
        tenant: &str,
        predicate: Option<String>,
    ) -> Result<Uuid, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let new_id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();
        let reactor = reactor.to_string();
        let workflow = workflow.to_string();
        let tenant = tenant.to_string();
        let row: ReactorSubscription = conn
            .interact(move |conn| {
                // SQLite: try insert; on conflict, update predicate +
                // updated_at and re-read.
                let insert_result = diesel::insert_into(reactor_trigger_subscriptions::table)
                    .values((
                        reactor_trigger_subscriptions::id.eq(new_id),
                        reactor_trigger_subscriptions::reactor_name.eq(&reactor),
                        reactor_trigger_subscriptions::workflow_name.eq(&workflow),
                        reactor_trigger_subscriptions::tenant_id.eq(&tenant),
                        reactor_trigger_subscriptions::enabled.eq(UniversalBool::from(true)),
                        reactor_trigger_subscriptions::created_at.eq(now),
                        reactor_trigger_subscriptions::updated_at.eq(now),
                        reactor_trigger_subscriptions::predicate_expression.eq(&predicate),
                    ))
                    .execute(conn);
                match insert_result {
                    Ok(_) => reactor_trigger_subscriptions::table
                        .filter(reactor_trigger_subscriptions::id.eq(new_id))
                        .first::<ReactorSubscription>(conn),
                    Err(diesel::result::Error::DatabaseError(
                        diesel::result::DatabaseErrorKind::UniqueViolation,
                        _,
                    )) => {
                        // Existing row — overwrite predicate so the
                        // upsert semantics match postgres.
                        diesel::update(
                            reactor_trigger_subscriptions::table
                                .filter(reactor_trigger_subscriptions::reactor_name.eq(&reactor))
                                .filter(reactor_trigger_subscriptions::workflow_name.eq(&workflow))
                                .filter(reactor_trigger_subscriptions::tenant_id.eq(&tenant)),
                        )
                        .set((
                            reactor_trigger_subscriptions::updated_at.eq(now),
                            reactor_trigger_subscriptions::predicate_expression.eq(&predicate),
                        ))
                        .execute(conn)?;
                        reactor_trigger_subscriptions::table
                            .filter(reactor_trigger_subscriptions::reactor_name.eq(&reactor))
                            .filter(reactor_trigger_subscriptions::workflow_name.eq(&workflow))
                            .filter(reactor_trigger_subscriptions::tenant_id.eq(&tenant))
                            .first::<ReactorSubscription>(conn)
                    }
                    Err(e) => Err(e),
                }
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(row.id.0)
    }

    /// Advance the watermark for a subscription. Caller is the
    /// dispatcher loop; the watermark advances after each row is
    /// dispatched (at-least-once on crash).
    pub async fn advance_watermark(
        &self,
        subscription_id: Uuid,
        new_last_seen: UniversalTimestamp,
    ) -> Result<(), ValidationError> {
        let sid = UniversalUuid(subscription_id);
        let now = UniversalTimestamp::now();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                reactor_trigger_subscriptions::table
                    .filter(reactor_trigger_subscriptions::id.eq(sid)),
            )
            .set((
                reactor_trigger_subscriptions::last_seen_fired_at.eq(Some(new_last_seen)),
                reactor_trigger_subscriptions::updated_at.eq(now),
            ))
            .execute(conn)
        })?;
        Ok(())
    }

    /// CLOACI-T-0922 — record a predicate evaluation error against a
    /// subscription and return the new consecutive-error count for
    /// `firing_id`.
    ///
    /// The count is per (subscription, firing): if `firing_id` differs
    /// from the firing the stored count applies to, the count restarts at
    /// 1. Because the watermark is held on error, the same firing stays
    /// head-of-line until it either evaluates cleanly or gets
    /// dead-lettered, so "consecutive errors on this firing" is exactly
    /// "attempts spent on this firing".
    ///
    /// Read-modify-write on a single pooled connection. The reactor poller
    /// is the only writer of these columns and processes a subscription
    /// serially, so no lock is needed; a lost update under a hypothetical
    /// second poller would only make the bound approximate, never unsafe.
    pub async fn record_predicate_error(
        &self,
        subscription_id: Uuid,
        firing_id: UniversalUuid,
        error: &str,
    ) -> Result<i32, ValidationError> {
        let sid = UniversalUuid(subscription_id);
        let now = UniversalTimestamp::now();
        let error = truncate_predicate_text(error);
        let count = crate::interact_on_backend!(self.dal, |conn| {
            let (prev_count, prev_firing): (i32, Option<UniversalUuid>) =
                reactor_trigger_subscriptions::table
                    .filter(reactor_trigger_subscriptions::id.eq(sid))
                    .select((
                        reactor_trigger_subscriptions::predicate_error_count,
                        reactor_trigger_subscriptions::predicate_error_firing_id,
                    ))
                    .first(conn)?;
            let next = if prev_firing == Some(firing_id) {
                prev_count.saturating_add(1)
            } else {
                1
            };
            diesel::update(
                reactor_trigger_subscriptions::table
                    .filter(reactor_trigger_subscriptions::id.eq(sid)),
            )
            .set((
                reactor_trigger_subscriptions::predicate_error_count.eq(next),
                reactor_trigger_subscriptions::predicate_error_firing_id.eq(Some(firing_id)),
                reactor_trigger_subscriptions::last_predicate_error.eq(Some(error)),
                reactor_trigger_subscriptions::last_predicate_error_at.eq(Some(now)),
                reactor_trigger_subscriptions::updated_at.eq(now),
            ))
            .execute(conn)?;
            Ok::<i32, diesel::result::Error>(next)
        })?;
        Ok(count)
    }

    /// CLOACI-T-0922 — mark a subscription degraded because a firing was
    /// dead-lettered: its predicate errored more times than the bound
    /// allows, so the caller is about to force-advance the watermark past
    /// it. The error text + firing id stay on the row as the durable
    /// record of what was dropped.
    ///
    /// Resets the consecutive-error counter so the NEXT firing starts with
    /// a full retry budget.
    pub async fn mark_predicate_degraded(
        &self,
        subscription_id: Uuid,
        firing_id: UniversalUuid,
        error: &str,
    ) -> Result<(), ValidationError> {
        let sid = UniversalUuid(subscription_id);
        let now = UniversalTimestamp::now();
        let error = truncate_predicate_text(error);
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                reactor_trigger_subscriptions::table
                    .filter(reactor_trigger_subscriptions::id.eq(sid)),
            )
            .set((
                reactor_trigger_subscriptions::predicate_degraded.eq(UniversalBool::from(true)),
                reactor_trigger_subscriptions::predicate_error_count.eq(0),
                reactor_trigger_subscriptions::predicate_error_firing_id.eq(Some(firing_id)),
                reactor_trigger_subscriptions::last_predicate_error.eq(Some(error)),
                reactor_trigger_subscriptions::last_predicate_error_at.eq(Some(now)),
                reactor_trigger_subscriptions::updated_at.eq(now),
            ))
            .execute(conn)
        })?;
        Ok(())
    }

    /// CLOACI-T-0922 — a predicate evaluated cleanly, so clear the retry
    /// counter and the degraded flag.
    ///
    /// `last_predicate_error` / `last_predicate_error_at` are deliberately
    /// NOT cleared: recovery should not erase the evidence that firings
    /// were dropped. "Currently degraded" is `predicate_degraded`; "what
    /// went wrong last" is the `last_predicate_error*` pair.
    pub async fn clear_predicate_error(
        &self,
        subscription_id: Uuid,
    ) -> Result<(), ValidationError> {
        let sid = UniversalUuid(subscription_id);
        let now = UniversalTimestamp::now();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                reactor_trigger_subscriptions::table
                    .filter(reactor_trigger_subscriptions::id.eq(sid)),
            )
            .set((
                reactor_trigger_subscriptions::predicate_error_count.eq(0),
                reactor_trigger_subscriptions::predicate_error_firing_id.eq(None::<UniversalUuid>),
                reactor_trigger_subscriptions::predicate_degraded.eq(UniversalBool::from(false)),
                reactor_trigger_subscriptions::updated_at.eq(now),
            ))
            .execute(conn)
        })?;
        Ok(())
    }

    /// CLOACI-T-0922 — fetch a single subscription by id. Used by the
    /// health/inspection path so an operator (or a test) can read the
    /// degraded state without scanning a tenant's whole subscription list.
    pub async fn get_subscription(
        &self,
        subscription_id: Uuid,
    ) -> Result<Option<ReactorSubscription>, ValidationError> {
        let sid = UniversalUuid(subscription_id);
        let row = crate::interact_on_backend!(self.dal, |conn| {
            reactor_trigger_subscriptions::table
                .filter(reactor_trigger_subscriptions::id.eq(sid))
                .first::<ReactorSubscription>(conn)
                .optional()
        })?;
        Ok(row)
    }

    /// Remove a subscription. Returns true if a row was deleted.
    pub async fn unsubscribe(
        &self,
        reactor: &str,
        workflow: &str,
        tenant: &str,
    ) -> Result<bool, ValidationError> {
        let reactor = reactor.to_string();
        let workflow = workflow.to_string();
        let tenant = tenant.to_string();
        let n = crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(
                reactor_trigger_subscriptions::table
                    .filter(reactor_trigger_subscriptions::reactor_name.eq(reactor))
                    .filter(reactor_trigger_subscriptions::workflow_name.eq(workflow))
                    .filter(reactor_trigger_subscriptions::tenant_id.eq(tenant)),
            )
            .execute(conn)
        })?;
        Ok(n > 0)
    }

    /// List all enabled subscriptions across every tenant. Used by the
    /// unified scheduler's reactor poll tick (CLOACI-I-0100 / T-0599).
    pub async fn list_all_enabled(&self) -> Result<Vec<ReactorSubscription>, ValidationError> {
        let rows = crate::interact_on_backend!(self.dal, |conn| {
            reactor_trigger_subscriptions::table
                .filter(reactor_trigger_subscriptions::enabled.eq(UniversalBool::from(true)))
                .load::<ReactorSubscription>(conn)
        })?;
        Ok(rows)
    }

    /// List enabled subscriptions for a tenant.
    pub async fn list_subscriptions(
        &self,
        tenant: &str,
    ) -> Result<Vec<ReactorSubscription>, ValidationError> {
        let tenant = tenant.to_string();
        let rows = crate::interact_on_backend!(self.dal, |conn| {
            reactor_trigger_subscriptions::table
                .filter(reactor_trigger_subscriptions::tenant_id.eq(tenant))
                .filter(reactor_trigger_subscriptions::enabled.eq(UniversalBool::from(true)))
                .load::<ReactorSubscription>(conn)
        })?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────
    // CLOACI-T-0922 — subscribe-time unbound-variable lint.
    //
    // The bar is asymmetric on purpose: a FALSE REJECTION breaks a valid
    // production predicate, while a missed lint only restores today's
    // behaviour. Every "accepts" case below is therefore a regression pin.
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn lint_accepts_the_bound_variables() {
        lint_predicate_variables("payload.x > 1 && tenant == 'acme'").expect("payload + tenant");
        lint_predicate_variables("reactor == 'pricing'").expect("reactor");
        lint_predicate_variables("payload.price > 100 && payload.region == 'us-east'")
            .expect("nested payload selects");
        lint_predicate_variables("tenant == 'acme' || tenant == 'public'").expect("tenant only");
    }

    #[test]
    fn lint_rejects_a_typod_identifier() {
        let err = lint_predicate_variables("payload.x > 1 && tennant == 'acme'")
            .expect_err("typo'd `tennant` must be rejected");
        assert!(
            err.contains("tennant"),
            "error should name the offending variable, got: {}",
            err
        );
        assert!(
            err.contains("payload") && err.contains("reactor") && err.contains("tenant"),
            "error should list the allowed variables, got: {}",
            err
        );
    }

    #[test]
    fn lint_rejects_a_bare_unknown_identifier() {
        assert!(lint_predicate_variables("foo").is_err());
        assert!(lint_predicate_variables("payload.a == bar").is_err());
    }

    /// The whole reason the lint walks the AST instead of using
    /// `Program::references()`: comprehension macros bind their iteration
    /// variable as a bare `Ident` in the loop body, and `references()`
    /// reports it as free. These MUST be accepted.
    #[test]
    fn lint_accepts_comprehension_iteration_variables() {
        lint_predicate_variables("payload.items.exists(i, i.price > 100)").expect("exists");
        lint_predicate_variables("payload.items.all(x, x > 0)").expect("all");
        lint_predicate_variables("payload.items.filter(v, v != 0).size() > 0").expect("filter");
        lint_predicate_variables("payload.items.map(m, m.id).size() > 1").expect("map");
        lint_predicate_variables("payload.a.exists(i, payload.b.exists(j, i == j))")
            .expect("nested comprehensions");
    }

    /// An iteration variable is only bound INSIDE its comprehension. Once
    /// the scope closes, the same name is free again.
    #[test]
    fn lint_rejects_iteration_variable_used_outside_its_scope() {
        assert!(
            lint_predicate_variables("payload.items.exists(i, i > 0) && i > 1").is_err(),
            "`i` outside the comprehension body is a free variable"
        );
    }

    #[test]
    fn lint_accepts_builtin_functions_and_literals() {
        lint_predicate_variables("size(payload.items) > 0").expect("size()");
        lint_predicate_variables("has(payload.price)").expect("has()");
        lint_predicate_variables("payload.name.startsWith('acme')").expect("receiver call");
        lint_predicate_variables("1 > 0").expect("no variables at all");
        lint_predicate_variables("payload.tags == ['a', 'b']").expect("list literal");
        lint_predicate_variables("{'k': payload.v}['k'] == 1").expect("map literal");
    }

    /// The CEL parser panics (not errors) on some malformed input —
    /// `"payload.x >"` hits an `unreachable!()` inside `antlr4rust`.
    /// Both entry points must convert that into an ordinary error rather
    /// than unwinding into the subscriber / poller task.
    #[test]
    fn malformed_predicates_error_instead_of_panicking() {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {})); // keep the test output clean

        let compile = std::panic::catch_unwind(|| compile_predicate("payload.x >"));
        let lint = std::panic::catch_unwind(|| lint_predicate_variables("payload.x >"));

        std::panic::set_hook(previous);

        let compile = compile.expect("compile_predicate must not unwind");
        assert!(compile.is_err(), "malformed predicate must be an error");
        let lint = lint.expect("lint_predicate_variables must not unwind");
        assert!(lint.is_err(), "malformed predicate must be an error");
    }

    #[test]
    fn compile_predicate_accepts_valid_expressions() {
        assert!(compile_predicate("payload.x > 1 && tenant == 'acme'").is_ok());
        assert!(compile_predicate("this is not cel ((").is_err());
    }

    #[test]
    fn lint_allowed_set_matches_the_documented_variables() {
        assert_eq!(PREDICATE_VARIABLES, ["payload", "reactor", "tenant"]);
    }

    #[test]
    fn truncate_predicate_text_is_char_safe_and_marks_truncation() {
        let short = "payload.x > 1";
        assert_eq!(truncate_predicate_text(short), short);

        let long = "é".repeat(PREDICATE_TEXT_TRUNCATE_LEN + 50);
        let out = truncate_predicate_text(&long);
        assert!(out.ends_with("…[truncated]"));
        assert_eq!(
            out.chars().count(),
            PREDICATE_TEXT_TRUNCATE_LEN + "…[truncated]".chars().count()
        );
    }
}
