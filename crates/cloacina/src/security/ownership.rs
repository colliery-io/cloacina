/*
 *  Copyright 2025-2026 Colliery Software
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

//! Who owns what on the fleet and on the delivery socket (CLOACINA-T-1050,
//! CLOACINA-T-1051, CLOACINA-T-1052).
//!
//! The authorization table of the server says which LEVEL a route needs. It
//! does not say which agent, which dispatch or which outbox row a caller may
//! touch. Those answers are here, in one place, as pure functions:
//!
//! | Question | Function |
//! |---|---|
//! | May this key register this agent id? | [`Principal::may_register_agent`] |
//! | May this key act as this agent (heartbeat, result, key top-up)? | [`Principal::may_act_as_agent`] |
//! | May this key open this delivery recipient? | [`Principal::may_subscribe`] |
//! | May this report resolve this wait? | [`DispatchTarget::check_report`] |
//! | May this connection touch this outbox row? | [`DeliveryScope::owns_row`] |
//!
//! The rules fail closed: an unknown agent, an unknown recipient kind, a
//! report with no dispatch id for a task dispatch and a row of another
//! recipient are all refused. A platform admin key gets no bypass here: it
//! acts as an agent only when it registered that agent.

use uuid::Uuid;

/// The recipient of the work packets of one agent: `agent:<agent id>`. The
/// same value as [`crate::fleet::AGENT_RECIPIENT_PREFIX`].
const AGENT_PREFIX: &str = "agent:";
/// The recipient of the events of one workflow execution.
const EXEC_EVENTS_PREFIX: &str = "exec_events:";
/// The recipient of the operations snapshots.
const OPS_METRICS_PREFIX: &str = "ops_metrics:";

/// The key that makes a call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    /// `api_keys.id` of the key.
    pub key_id: Uuid,
    /// The tenant of the key. `None` for a key with no tenant.
    pub tenant_id: Option<String>,
    /// Whether the key is a platform admin key.
    pub is_admin: bool,
}

/// Who holds an agent id: the tenant and the key of the roster row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentOwner {
    /// The tenant the agent registered in.
    pub tenant_id: Option<String>,
    /// The key that registered the agent. `None` on a row from before the
    /// owner column (CLOACINA-T-1051): see [`Principal::may_register_agent`].
    pub owner_key_id: Option<Uuid>,
}

/// Why a call was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// No roster row has this agent id.
    UnknownAgent,
    /// The agent is in another tenant than the caller.
    OtherTenant,
    /// Another key holds the agent.
    OtherKey,
    /// The agent row has no owner yet, and this call needs one.
    NoOwner,
    /// The key has no tenant and is not a platform admin.
    NoTenant,
    /// The delivery socket does not serve this recipient.
    UnknownRecipient,
    /// The report carries no dispatch id and the wait is for a task dispatch.
    NoDispatchId,
    /// The report is for another dispatch of the task.
    OtherDispatch,
    /// The report comes from an agent the packet is not addressed to.
    NotTheTarget,
}

impl Refusal {
    /// A short stable name, for log lines and metric labels.
    pub fn as_str(self) -> &'static str {
        match self {
            Refusal::UnknownAgent => "unknown_agent",
            Refusal::OtherTenant => "other_tenant",
            Refusal::OtherKey => "other_key",
            Refusal::NoOwner => "no_owner",
            Refusal::NoTenant => "no_tenant",
            Refusal::UnknownRecipient => "unknown_recipient",
            Refusal::NoDispatchId => "no_dispatch_id",
            Refusal::OtherDispatch => "other_dispatch",
            Refusal::NotTheTarget => "not_the_target",
        }
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Principal {
    /// Whether this key may register `agent_id`, given the row that holds the
    /// id now (`None`: no row).
    ///
    /// - A key with no tenant: no, whatever its role and whatever the row
    ///   (CLOACINA-T-1118). An agent with no tenant serves no tenant's work.
    /// - No row: yes. The key becomes the owner.
    /// - The row is held by this key, in this key's tenant: yes (an agent
    ///   that restarts and registers again).
    /// - The row has no owner (it is from before the owner column) and is in
    ///   this key's tenant: yes. The register claims the row.
    /// - Any other row: no.
    pub fn may_register_agent(&self, existing: Option<&AgentOwner>) -> Result<(), Refusal> {
        if self.tenant_id.is_none() {
            return Err(Refusal::NoTenant);
        }
        let Some(row) = existing else {
            return Ok(());
        };
        if row.tenant_id != self.tenant_id {
            return Err(Refusal::OtherTenant);
        }
        match row.owner_key_id {
            None => Ok(()),
            Some(owner) if owner == self.key_id => Ok(()),
            Some(_) => Err(Refusal::OtherKey),
        }
    }

    /// Whether this key may act as the agent of `row`: heartbeat, report a
    /// result, top up the one-time keys (`None`: no row has the id).
    ///
    /// The row must exist, be in this key's tenant and be held by this key.
    /// A row with no owner (from before the owner column) is open to a key of
    /// its tenant until a register claims it; that is the rule those rows
    /// were written under.
    pub fn may_act_as_agent(&self, row: Option<&AgentOwner>) -> Result<(), Refusal> {
        let row = row.ok_or(Refusal::UnknownAgent)?;
        if row.tenant_id != self.tenant_id {
            return Err(Refusal::OtherTenant);
        }
        match row.owner_key_id {
            None => Ok(()),
            Some(owner) if owner == self.key_id => Ok(()),
            Some(_) => Err(Refusal::OtherKey),
        }
    }

    /// Whether this key may open the delivery socket of `recipient`.
    ///
    /// `agent` is the roster row of the agent the recipient names, for an
    /// `agent:<id>` recipient (`None`: no such row). It is not read for the
    /// other kinds.
    ///
    /// - `agent:<id>`: only the key that holds the agent. A row with no owner
    ///   has no socket: the agent registers first, which claims the row.
    /// - `exec_events:<id>`: a key with a tenant, or a platform admin key.
    ///   The connection gets only the rows of its own tenant (see
    ///   [`DeliveryScope`]), so the id of another tenant's execution gives an
    ///   empty stream.
    /// - `ops_metrics:*`: any key. Which view a key gets is decided where
    ///   the snapshots are published, not here.
    /// - Any other recipient: no. The socket serves no other kind.
    pub fn may_subscribe(
        &self,
        recipient: &str,
        agent: Option<&AgentOwner>,
    ) -> Result<(), Refusal> {
        match RecipientKind::of(recipient) {
            RecipientKind::Agent(_) => {
                let row = agent.ok_or(Refusal::UnknownAgent)?;
                if row.tenant_id != self.tenant_id {
                    return Err(Refusal::OtherTenant);
                }
                match row.owner_key_id {
                    None => Err(Refusal::NoOwner),
                    Some(owner) if owner == self.key_id => Ok(()),
                    Some(_) => Err(Refusal::OtherKey),
                }
            }
            RecipientKind::ExecEvents(_) => {
                if self.tenant_id.is_some() || self.is_admin {
                    Ok(())
                } else {
                    Err(Refusal::NoTenant)
                }
            }
            RecipientKind::OpsMetrics => Ok(()),
            RecipientKind::Other => Err(Refusal::UnknownRecipient),
        }
    }
}

/// The kinds of recipient the delivery socket knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipientKind<'a> {
    /// `agent:<agent id>`: the work packets of one agent.
    Agent(&'a str),
    /// `exec_events:<execution id>`: the events of one workflow execution.
    ExecEvents(&'a str),
    /// `ops_metrics:*`: the operations snapshots.
    OpsMetrics,
    /// Anything else. Rows of other recipients (the alert notices, the
    /// accumulator events) are read inside the server, never over the socket.
    Other,
}

impl<'a> RecipientKind<'a> {
    /// The kind of `recipient`. An empty id after the prefix is [`Other`](Self::Other).
    pub fn of(recipient: &'a str) -> Self {
        if let Some(id) = recipient.strip_prefix(AGENT_PREFIX) {
            if !id.is_empty() {
                return RecipientKind::Agent(id);
            }
        } else if let Some(id) = recipient.strip_prefix(EXEC_EVENTS_PREFIX) {
            if !id.is_empty() {
                return RecipientKind::ExecEvents(id);
            }
        } else if recipient.starts_with(OPS_METRICS_PREFIX) {
            return RecipientKind::OpsMetrics;
        }
        RecipientKind::Other
    }

    /// Whether every connection of the recipient gets each frame. A recipient
    /// that is not shared has one consumer: a new connection replaces the
    /// old one.
    ///
    /// The work packets of an agent have one consumer, the agent. The events
    /// of an execution and the operations snapshots are read by people, and
    /// two of them can follow the same stream.
    pub fn is_shared(self) -> bool {
        matches!(
            self,
            RecipientKind::ExecEvents(_) | RecipientKind::OpsMetrics
        )
    }
}

/// What one delivery connection may touch: the rows addressed to its own
/// recipient, in its own tenant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryScope {
    pub recipient: String,
    pub tenant_id: Option<String>,
}

impl DeliveryScope {
    pub fn new(recipient: impl Into<String>, tenant_id: Option<String>) -> Self {
        Self {
            recipient: recipient.into(),
            tenant_id,
        }
    }

    /// The scope of the work packets of one agent.
    pub fn of_agent(agent_id: &str, tenant_id: Option<String>) -> Self {
        Self::new(format!("{AGENT_PREFIX}{agent_id}"), tenant_id)
    }

    /// Whether an outbox row with this recipient and tenant is in the scope.
    /// Both must be equal; a row with no tenant is in the scope only of a
    /// connection with no tenant.
    pub fn owns_row(&self, row_recipient: &str, row_tenant: Option<&str>) -> bool {
        self.recipient == row_recipient && self.tenant_id.as_deref() == row_tenant
    }
}

/// Where a dispatch went: what a wait for an agent report knows about its
/// packet (CLOACINA-T-1050).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchTarget {
    /// The tenant of the work.
    pub tenant_id: Option<String>,
    /// The agent the packet was addressed to when it was enqueued.
    pub agent_id: String,
    /// The id of the dispatch (CLOACINA-A-0016). `Some` for a task dispatch.
    /// `None` for a graph firing: its wait is keyed by the firing id, which
    /// names one dispatch already.
    pub dispatch_id: Option<String>,
}

/// Where a report comes from. `agent_id` and `tenant_id` are the ones of the
/// roster row the caller was checked against
/// ([`Principal::may_act_as_agent`]), never values taken from the body alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportOrigin<'a> {
    pub tenant_id: Option<&'a str>,
    pub agent_id: &'a str,
    pub dispatch_id: Option<&'a str>,
}

/// The answer of [`DispatchTarget::check_report`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportCheck {
    /// The report resolves the wait.
    Accept,
    /// Tenant and dispatch are right, and the report comes from another
    /// agent than the one the packet was enqueued for. It resolves the wait
    /// only if the packet was re-targeted to that agent: the outbox row of
    /// the packet must be in [`DeliveryScope::of_agent`] of the reporting
    /// agent ([`DeliveryScope::owns_row`]).
    IfRetargeted,
    /// The report does not resolve the wait.
    Refuse(Refusal),
}

impl DispatchTarget {
    /// Whether a report from `origin` may resolve the wait for this dispatch.
    ///
    /// The report must come from the tenant of the dispatch. For a task
    /// dispatch it must carry the dispatch id of the packet: a report with
    /// no id is an orphan (no agent older than the field exists, it shipped
    /// under the same protocol version). It must come from the agent the
    /// packet is addressed to.
    pub fn check_report(&self, origin: &ReportOrigin<'_>) -> ReportCheck {
        if self.tenant_id.as_deref() != origin.tenant_id {
            return ReportCheck::Refuse(Refusal::OtherTenant);
        }
        if let Some(want) = self.dispatch_id.as_deref() {
            match origin.dispatch_id {
                None => return ReportCheck::Refuse(Refusal::NoDispatchId),
                Some(got) if got != want => return ReportCheck::Refuse(Refusal::OtherDispatch),
                Some(_) => {}
            }
        }
        if self.agent_id == origin.agent_id {
            ReportCheck::Accept
        } else {
            ReportCheck::IfRetargeted
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(tenant: Option<&str>) -> Principal {
        Principal {
            key_id: Uuid::new_v4(),
            tenant_id: tenant.map(str::to_string),
            is_admin: false,
        }
    }

    fn held_by(p: &Principal) -> AgentOwner {
        AgentOwner {
            tenant_id: p.tenant_id.clone(),
            owner_key_id: Some(p.key_id),
        }
    }

    fn unowned(tenant: Option<&str>) -> AgentOwner {
        AgentOwner {
            tenant_id: tenant.map(str::to_string),
            owner_key_id: None,
        }
    }

    // ── register ────────────────────────────────────────────────────────

    #[test]
    fn a_free_agent_id_can_be_registered() {
        assert_eq!(key(Some("a")).may_register_agent(None), Ok(()));
    }

    #[test]
    fn the_owner_registers_its_agent_again() {
        let k = key(Some("a"));
        assert_eq!(k.may_register_agent(Some(&held_by(&k))), Ok(()));
    }

    #[test]
    fn a_key_of_another_tenant_cannot_take_an_agent_id() {
        let owner = key(Some("b"));
        assert_eq!(
            key(Some("a")).may_register_agent(Some(&held_by(&owner))),
            Err(Refusal::OtherTenant)
        );
    }

    #[test]
    fn another_key_of_the_same_tenant_cannot_take_an_agent_id() {
        let owner = key(Some("a"));
        assert_eq!(
            key(Some("a")).may_register_agent(Some(&held_by(&owner))),
            Err(Refusal::OtherKey)
        );
    }

    #[test]
    fn a_platform_admin_key_cannot_take_an_agent_id() {
        let owner = key(Some("a"));
        let admin = Principal {
            key_id: Uuid::new_v4(),
            tenant_id: None,
            is_admin: true,
        };
        assert_eq!(
            admin.may_register_agent(Some(&held_by(&owner))),
            Err(Refusal::NoTenant)
        );
        assert_eq!(
            admin.may_act_as_agent(Some(&held_by(&owner))),
            Err(Refusal::OtherTenant)
        );
    }

    #[test]
    fn a_row_with_no_owner_is_claimed_by_a_key_of_its_tenant_only() {
        assert_eq!(
            key(Some("a")).may_register_agent(Some(&unowned(Some("a")))),
            Ok(())
        );
        assert_eq!(
            key(Some("b")).may_register_agent(Some(&unowned(Some("a")))),
            Err(Refusal::OtherTenant)
        );
        assert_eq!(
            key(None).may_register_agent(Some(&unowned(Some("a")))),
            Err(Refusal::NoTenant)
        );
    }

    #[test]
    fn t1118_a_key_with_no_tenant_cannot_register_an_agent() {
        let admin = Principal {
            key_id: Uuid::new_v4(),
            tenant_id: None,
            is_admin: true,
        };
        for k in [key(None), admin] {
            assert_eq!(k.may_register_agent(None), Err(Refusal::NoTenant));
            assert_eq!(
                k.may_register_agent(Some(&unowned(None))),
                Err(Refusal::NoTenant)
            );
            assert_eq!(
                k.may_register_agent(Some(&held_by(&k))),
                Err(Refusal::NoTenant)
            );
        }
    }

    // ── act as an agent ─────────────────────────────────────────────────

    #[test]
    fn an_unknown_agent_id_is_refused() {
        assert_eq!(
            key(Some("a")).may_act_as_agent(None),
            Err(Refusal::UnknownAgent)
        );
    }

    #[test]
    fn only_the_owner_key_acts_as_the_agent() {
        let owner = key(Some("a"));
        let row = held_by(&owner);
        assert_eq!(owner.may_act_as_agent(Some(&row)), Ok(()));
        assert_eq!(
            key(Some("a")).may_act_as_agent(Some(&row)),
            Err(Refusal::OtherKey)
        );
        assert_eq!(
            key(Some("b")).may_act_as_agent(Some(&row)),
            Err(Refusal::OtherTenant)
        );
    }

    #[test]
    fn a_row_with_no_owner_is_open_to_its_tenant_until_it_is_claimed() {
        let row = unowned(Some("a"));
        assert_eq!(key(Some("a")).may_act_as_agent(Some(&row)), Ok(()));
        assert_eq!(
            key(Some("b")).may_act_as_agent(Some(&row)),
            Err(Refusal::OtherTenant)
        );
    }

    // ── subscribe ───────────────────────────────────────────────────────

    #[test]
    fn the_agent_socket_is_open_to_the_owner_key_only() {
        let owner = key(Some("a"));
        let row = held_by(&owner);
        assert_eq!(owner.may_subscribe("agent:x", Some(&row)), Ok(()));
        assert_eq!(
            key(Some("a")).may_subscribe("agent:x", Some(&row)),
            Err(Refusal::OtherKey)
        );
        assert_eq!(
            key(Some("b")).may_subscribe("agent:x", Some(&row)),
            Err(Refusal::OtherTenant)
        );
        assert_eq!(
            owner.may_subscribe("agent:x", None),
            Err(Refusal::UnknownAgent)
        );
    }

    #[test]
    fn a_row_with_no_owner_has_no_socket() {
        assert_eq!(
            key(Some("a")).may_subscribe("agent:x", Some(&unowned(Some("a")))),
            Err(Refusal::NoOwner)
        );
    }

    #[test]
    fn execution_events_are_open_to_a_key_with_a_tenant_or_a_platform_admin() {
        assert_eq!(key(Some("a")).may_subscribe("exec_events:1", None), Ok(()));
        let admin = Principal {
            key_id: Uuid::new_v4(),
            tenant_id: None,
            is_admin: true,
        };
        assert_eq!(admin.may_subscribe("exec_events:1", None), Ok(()));
        assert_eq!(
            key(None).may_subscribe("exec_events:1", None),
            Err(Refusal::NoTenant)
        );
    }

    #[test]
    fn other_recipients_have_no_socket() {
        let k = key(Some("a"));
        for recipient in [
            "alert_webhook",
            "accumulator:orders",
            "agent:",
            "exec_events:",
            "",
            "something:else",
        ] {
            assert_eq!(
                k.may_subscribe(recipient, None),
                Err(Refusal::UnknownRecipient),
                "{recipient}"
            );
        }
        assert_eq!(k.may_subscribe("ops_metrics:global", None), Ok(()));
    }

    #[test]
    fn recipient_kinds() {
        assert_eq!(RecipientKind::of("agent:a1"), RecipientKind::Agent("a1"));
        assert_eq!(
            RecipientKind::of("exec_events:e1"),
            RecipientKind::ExecEvents("e1")
        );
        assert_eq!(
            RecipientKind::of("ops_metrics:global"),
            RecipientKind::OpsMetrics
        );
        assert_eq!(RecipientKind::of("alert_webhook"), RecipientKind::Other);
        assert!(!RecipientKind::of("agent:a1").is_shared());
        assert!(RecipientKind::of("exec_events:e1").is_shared());
        assert!(RecipientKind::of("ops_metrics:global").is_shared());
        assert!(!RecipientKind::of("alert_webhook").is_shared());
        assert_eq!(
            format!("{}a1", crate::fleet::AGENT_RECIPIENT_PREFIX),
            DeliveryScope::of_agent("a1", None).recipient
        );
    }

    // ── outbox rows ─────────────────────────────────────────────────────

    #[test]
    fn a_connection_owns_only_the_rows_of_its_recipient_and_tenant() {
        let scope = DeliveryScope::new("agent:a1", Some("a".to_string()));
        assert!(scope.owns_row("agent:a1", Some("a")));
        assert!(!scope.owns_row("agent:a2", Some("a")), "another recipient");
        assert!(!scope.owns_row("agent:a1", Some("b")), "another tenant");
        assert!(!scope.owns_row("agent:a1", None), "a row with no tenant");

        let no_tenant = DeliveryScope::new("exec_events:1", None);
        assert!(no_tenant.owns_row("exec_events:1", None));
        assert!(!no_tenant.owns_row("exec_events:1", Some("a")));
    }

    // ── reports ─────────────────────────────────────────────────────────

    fn task_dispatch() -> DispatchTarget {
        DispatchTarget {
            tenant_id: Some("a".to_string()),
            agent_id: "a1".to_string(),
            dispatch_id: Some("d1".to_string()),
        }
    }

    fn origin<'a>(
        tenant: Option<&'a str>,
        agent: &'a str,
        dispatch: Option<&'a str>,
    ) -> ReportOrigin<'a> {
        ReportOrigin {
            tenant_id: tenant,
            agent_id: agent,
            dispatch_id: dispatch,
        }
    }

    #[test]
    fn the_report_of_the_target_agent_resolves_the_wait() {
        assert_eq!(
            task_dispatch().check_report(&origin(Some("a"), "a1", Some("d1"))),
            ReportCheck::Accept
        );
    }

    #[test]
    fn a_report_of_another_tenant_is_refused() {
        assert_eq!(
            task_dispatch().check_report(&origin(Some("b"), "a1", Some("d1"))),
            ReportCheck::Refuse(Refusal::OtherTenant)
        );
        assert_eq!(
            task_dispatch().check_report(&origin(None, "a1", Some("d1"))),
            ReportCheck::Refuse(Refusal::OtherTenant)
        );
    }

    #[test]
    fn a_report_with_no_dispatch_id_is_an_orphan_for_a_task_dispatch() {
        assert_eq!(
            task_dispatch().check_report(&origin(Some("a"), "a1", None)),
            ReportCheck::Refuse(Refusal::NoDispatchId)
        );
    }

    #[test]
    fn a_report_of_another_dispatch_is_refused() {
        assert_eq!(
            task_dispatch().check_report(&origin(Some("a"), "a1", Some("d0"))),
            ReportCheck::Refuse(Refusal::OtherDispatch)
        );
    }

    #[test]
    fn a_report_of_another_agent_of_the_tenant_needs_a_retargeted_packet() {
        assert_eq!(
            task_dispatch().check_report(&origin(Some("a"), "a2", Some("d1"))),
            ReportCheck::IfRetargeted
        );
    }

    #[test]
    fn a_graph_firing_has_no_dispatch_id_and_keeps_the_tenant_and_agent_check() {
        let firing = DispatchTarget {
            tenant_id: Some("a".to_string()),
            agent_id: "a1".to_string(),
            dispatch_id: None,
        };
        assert_eq!(
            firing.check_report(&origin(Some("a"), "a1", None)),
            ReportCheck::Accept
        );
        assert_eq!(
            firing.check_report(&origin(Some("b"), "a1", None)),
            ReportCheck::Refuse(Refusal::OtherTenant)
        );
        assert_eq!(
            firing.check_report(&origin(Some("a"), "a2", None)),
            ReportCheck::IfRetargeted
        );
    }
}
