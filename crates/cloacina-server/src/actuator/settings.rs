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

//! Agent pool settings (CLOACINA-T-1021).
//!
//! A tenant's managed agents are grouped in pools. Each pool can override the
//! server-wide agent defaults: resource requests and limits, max concurrency,
//! image, service account and its annotations, extra egress rules, and (for a
//! pool other than `default`) its agent count. The settings are stored as one
//! JSON object per (tenant, pool) in `agent_pools`, set through the pool
//! routes, and read by the actuator on each reconcile through
//! [`AgentPoolSource`].
//!
//! [`validate`] rejects at the API what Kubernetes would reject later at the
//! reconcile: a malformed quantity, a request above its limit, a bad name, a
//! bad CIDR block.

use std::net::IpAddr;

use async_trait::async_trait;
use cloacina::database::Database;
use cloacina_api_types::{AgentEgressRule, AgentPoolSettings, DEFAULT_AGENT_POOL};
// CLOACINA-T-1027: the quantity grammar and the pool name rule live in
// `cloacina-api-types`, so the task macro, the Python decorator and the agent
// check a task's pool and requests by the same rules as the pool routes.
use cloacina_api_types::resources::is_dns_label;
pub use cloacina_api_types::resources::{MAX_POOL_NAME_LEN, Quantity, validate_pool_name};

use super::ActuatorError;
use super::kubernetes::AgentResources;

/// Most agents a pool other than `default` can be given.
pub const MAX_POOL_AGENTS: u32 = 1000;
/// Upper bound on a pool's agent max concurrency.
pub const MAX_CONCURRENCY_CEILING: u32 = 1024;
/// Most extra egress rules a pool can have.
pub const MAX_EGRESS_RULES: usize = 64;
/// Most ports on one egress rule.
pub const MAX_EGRESS_PORTS: usize = 32;
/// Most annotations on a pool's service account.
pub const MAX_ANNOTATIONS: usize = 32;
/// Most labels, and most annotations, a pool can add to its pods.
pub const MAX_POD_METADATA: usize = 32;
/// Longest annotation value, in bytes.
pub const MAX_ANNOTATION_VALUE_BYTES: usize = 4096;

/// A DNS-1123 subdomain: labels joined by `.`, 253 characters at most. The
/// form Kubernetes wants for a ServiceAccount name.
fn is_dns_subdomain(s: &str) -> bool {
    !s.is_empty() && s.len() <= 253 && s.split('.').all(is_dns_label)
}

/// A Kubernetes qualified name, the form of an annotation key: an optional
/// DNS-subdomain prefix and `/`, then a name of 63 characters at most made of
/// alphanumerics, `-`, `_` and `.`, alphanumeric at both ends.
fn is_qualified_name(s: &str) -> bool {
    let (prefix, name) = match s.split_once('/') {
        Some((p, n)) => (Some(p), n),
        None => (None, s),
    };
    if let Some(p) = prefix
        && !is_dns_subdomain(p)
    {
        return false;
    }
    let b = name.as_bytes();
    !b.is_empty()
        && b.len() <= 63
        && b.iter()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
        && b[0].is_ascii_alphanumeric()
        && b[b.len() - 1].is_ascii_alphanumeric()
}

/// Check a CIDR block: an IPv4 or IPv6 address, `/`, a prefix length in range,
/// and no host bits set.
fn validate_cidr(raw: &str) -> Result<(), String> {
    let bad = |why: &str| format!("'{raw}' is not a valid CIDR block: {why}");
    let (addr, prefix) = raw
        .split_once('/')
        .ok_or_else(|| bad("it has no '/' and prefix length"))?;
    let addr: IpAddr = addr
        .parse()
        .map_err(|_| bad("the address is not an IPv4 or IPv6 address"))?;
    if prefix.is_empty() || !prefix.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad("the prefix length is not a number"));
    }
    let prefix: u32 = prefix
        .parse()
        .map_err(|_| bad("the prefix length is out of range"))?;
    let (bits, width): (u128, u32) = match addr {
        IpAddr::V4(a) => (u128::from(u32::from(a)), 32),
        IpAddr::V6(a) => (u128::from(a), 128),
    };
    if prefix > width {
        return Err(bad("the prefix length is out of range"));
    }
    let host_bits = width - prefix;
    let host_mask = if host_bits == 0 {
        0
    } else {
        u128::MAX >> (128 - host_bits)
    };
    if bits & host_mask != 0 {
        return Err(bad("it has host bits set"));
    }
    Ok(())
}

/// A Kubernetes label value: empty, or 63 characters at most of alphanumerics,
/// `-`, `_` and `.`, alphanumeric at both ends.
fn is_label_value(s: &str) -> bool {
    let b = s.as_bytes();
    b.is_empty()
        || (b.len() <= 63
            && b.iter()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
            && b[0].is_ascii_alphanumeric()
            && b[b.len() - 1].is_ascii_alphanumeric())
}

/// Check the labels and annotations a pool adds to its pods. A label key the
/// actuator sets itself is refused: one of them is the Deployment's selector.
fn validate_pod_metadata(settings: &AgentPoolSettings) -> Result<(), String> {
    for (field, map) in [
        ("pod_labels", &settings.pod_labels),
        ("pod_annotations", &settings.pod_annotations),
    ] {
        if map.len() > MAX_POD_METADATA {
            return Err(format!(
                "{field}: {} entries; the most is {MAX_POD_METADATA}",
                map.len()
            ));
        }
    }
    for (key, value) in &settings.pod_labels {
        if super::kubernetes::RESERVED_POD_LABELS.contains(&key.as_str()) {
            return Err(format!(
                "pod_labels: '{key}' is a label the fleet actuator sets itself; it cannot be set"
            ));
        }
        if !is_qualified_name(key) {
            return Err(format!("pod_labels: '{key}' is not a valid label key"));
        }
        if !is_label_value(value) {
            return Err(format!(
                "pod_labels: the value of '{key}' is not a valid label value (63 characters at \
                 most; letters, digits, '-', '_' and '.'; a letter or digit at both ends)"
            ));
        }
    }
    for (key, value) in &settings.pod_annotations {
        if !is_qualified_name(key) {
            return Err(format!(
                "pod_annotations: '{key}' is not a valid annotation key"
            ));
        }
        if value.len() > MAX_ANNOTATION_VALUE_BYTES {
            return Err(format!(
                "pod_annotations: the value of '{key}' is longer than \
                 {MAX_ANNOTATION_VALUE_BYTES} bytes"
            ));
        }
    }
    Ok(())
}

fn validate_egress(rules: &[AgentEgressRule]) -> Result<(), String> {
    if rules.len() > MAX_EGRESS_RULES {
        return Err(format!(
            "extra_egress has {} rules; the most is {MAX_EGRESS_RULES}",
            rules.len()
        ));
    }
    for (i, rule) in rules.iter().enumerate() {
        validate_cidr(&rule.cidr).map_err(|e| format!("extra_egress[{i}].cidr: {e}"))?;
        if rule.ports.len() > MAX_EGRESS_PORTS {
            return Err(format!(
                "extra_egress[{i}] has {} ports; the most is {MAX_EGRESS_PORTS}",
                rule.ports.len()
            ));
        }
        for (j, port) in rule.ports.iter().enumerate() {
            if port.port == 0 {
                return Err(format!(
                    "extra_egress[{i}].ports[{j}].port: 0 is not a port (1 to 65535)"
                ));
            }
            if let Some(protocol) = &port.protocol
                && !matches!(protocol.as_str(), "TCP" | "UDP" | "SCTP")
            {
                return Err(format!(
                    "extra_egress[{i}].ports[{j}].protocol: '{protocol}' is not TCP, UDP or SCTP"
                ));
            }
        }
    }
    Ok(())
}

/// What one agent of a pool can hold, as the pool's settings say it
/// (CLOACINA-T-1027): for CPU and for memory, the pool's limit, or its
/// request when it sets no limit. `None` when the pool sets neither.
///
/// It is the total a managed agent of the pool reports at registration
/// (`CLOACINA_AGENT_CPU`, `CLOACINA_AGENT_MEMORY`), and the most a task of the
/// pool can ask for at publish. A pool that sets neither adds nothing to its
/// agents: they report no total and are packed by their slot count, as before
/// requests existed.
pub fn pool_agent_totals(settings: &AgentPoolSettings) -> (Option<String>, Option<String>) {
    (
        settings
            .cpu_limit
            .clone()
            .or_else(|| settings.cpu_request.clone()),
        settings
            .memory_limit
            .clone()
            .or_else(|| settings.memory_request.clone()),
    )
}

/// How many agents the server keeps for a pool other than `default` when the
/// autoscaler has stored no count for it: its min. With no `min_agents` that
/// is its `max_agents`, as before pools were autoscaled (CLOACINA-T-1029).
pub fn fixed_agent_count(settings: &AgentPoolSettings) -> u32 {
    pool_agent_count(settings, None)
}

/// The count of a pool other than `default`: the count the autoscaler stored
/// (CLOACINA-T-1029), held to the pool's `[min_agents, max_agents]`; with no
/// stored count, its min.
pub fn pool_agent_count(settings: &AgentPoolSettings, stored: Option<u32>) -> u32 {
    crate::autoscaler::step::other_pool_bounds(settings, stored).2
}

/// Check the settings of the pool `pool`. `defaults` are the server-wide
/// resources: a request is compared with the limit that will be in force, set
/// here or not. The error is one plain sentence that names the field.
pub fn validate(
    pool: &str,
    settings: &AgentPoolSettings,
    defaults: &AgentResources,
) -> Result<(), String> {
    validate_pool_name(pool)?;
    if pool == DEFAULT_AGENT_POOL {
        if settings.min_agents.is_some() || settings.max_agents.is_some() {
            return Err(
                "min_agents and max_agents are not for the pool 'default': its max is the \
                 tenant's agent limit (the limits route) and its count is autoscaled"
                    .to_string(),
            );
        }
    } else {
        let Some(max) = settings.max_agents else {
            return Err(format!(
                "max_agents: the pool '{pool}' needs it; a pool other than 'default' runs \
                 exactly that many agents"
            ));
        };
        if max > MAX_POOL_AGENTS {
            return Err(format!("max_agents: {max} is more than {MAX_POOL_AGENTS}"));
        }
        // CLOACINA-T-1029: a min below the max lets the pool autoscale between
        // them; no min keeps the pool at its max.
        if let Some(min) = settings.min_agents
            && min > max
        {
            return Err(format!("min_agents: {min} is more than max_agents ({max})"));
        }
    }

    let quantity = |field: &str, value: &Option<String>| -> Result<Option<Quantity>, String> {
        value
            .as_deref()
            .map(|raw| Quantity::parse(raw).map_err(|e| format!("{field}: {e}")))
            .transpose()
    };
    let cpu_request = quantity("cpu_request", &settings.cpu_request)?;
    let memory_request = quantity("memory_request", &settings.memory_request)?;
    let cpu_limit = quantity("cpu_limit", &settings.cpu_limit)?;
    let memory_limit = quantity("memory_limit", &settings.memory_limit)?;
    for (field, limit) in [("cpu_limit", cpu_limit), ("memory_limit", memory_limit)] {
        if limit.is_some_and(|q| q.is_zero()) {
            return Err(format!("{field}: a limit must be more than zero"));
        }
    }
    // A request above its limit is refused by Kubernetes when it validates the
    // pod template. A server-wide default that does not parse is not compared.
    let in_force =
        |own: Option<Quantity>, default: &str| own.or_else(|| Quantity::parse(default).ok());
    for (what, request, limit) in [
        (
            "cpu",
            in_force(cpu_request, &defaults.cpu_request),
            in_force(cpu_limit, &defaults.cpu_limit),
        ),
        (
            "memory",
            in_force(memory_request, &defaults.memory_request),
            in_force(memory_limit, &defaults.memory_limit),
        ),
    ] {
        if let (Some(request), Some(limit)) = (request, limit)
            && request > limit
        {
            return Err(format!(
                "{what}_request is more than {what}_limit (with the server-wide default for the \
                 one that is not set); set both"
            ));
        }
    }

    if let Some(n) = settings.max_concurrency
        && !(1..=MAX_CONCURRENCY_CEILING).contains(&n)
    {
        return Err(format!(
            "max_concurrency: {n} is not from 1 to {MAX_CONCURRENCY_CEILING}"
        ));
    }

    if let Some(image) = &settings.image
        && (image.is_empty()
            || image.len() > 512
            || image.chars().any(|c| c.is_whitespace() || c.is_control()))
    {
        return Err(
            "image: must be 1 to 512 characters with no space or control character".to_string(),
        );
    }

    if let Some(name) = &settings.service_account_name
        && !is_dns_subdomain(name)
    {
        return Err(format!(
            "service_account_name: '{name}' is not a valid Kubernetes name (lowercase letters, \
             digits, '-' and '.', 253 characters at most)"
        ));
    }
    if !settings.service_account_annotations.is_empty() {
        if settings.service_account_name.is_none() {
            return Err(
                "service_account_annotations: set service_account_name too; the annotations go \
                 on that service account"
                    .to_string(),
            );
        }
        if settings.service_account_annotations.len() > MAX_ANNOTATIONS {
            return Err(format!(
                "service_account_annotations: {} entries; the most is {MAX_ANNOTATIONS}",
                settings.service_account_annotations.len()
            ));
        }
        for (key, value) in &settings.service_account_annotations {
            if !is_qualified_name(key) {
                return Err(format!(
                    "service_account_annotations: '{key}' is not a valid annotation key"
                ));
            }
            if value.len() > MAX_ANNOTATION_VALUE_BYTES {
                return Err(format!(
                    "service_account_annotations: the value of '{key}' is longer than \
                     {MAX_ANNOTATION_VALUE_BYTES} bytes"
                ));
            }
        }
    }

    validate_pod_metadata(settings)?;
    validate_egress(&settings.extra_egress)
}

/// Parse the stored JSON text of a pool's settings.
pub fn parse_stored(text: &str) -> Result<AgentPoolSettings, String> {
    serde_json::from_str(text).map_err(|e| format!("stored settings do not parse: {e}"))
}

/// Put the pool `default` first, with default settings when it has no row.
fn with_default_pool(
    mut stored: Vec<(String, AgentPoolSettings)>,
) -> Vec<(String, AgentPoolSettings)> {
    let default = match stored
        .iter()
        .position(|(name, _)| name == DEFAULT_AGENT_POOL)
    {
        Some(i) => stored.remove(i),
        None => (DEFAULT_AGENT_POOL.to_string(), AgentPoolSettings::default()),
    };
    stored.insert(0, default);
    stored
}

/// Read a tenant's pools: `default` first (with default settings when it has
/// no row), then the others by name. Text that does not parse is an error: the
/// caller must not fall back to the defaults, which would resize the agents.
pub async fn load_pools(
    database: &Database,
    tenant_id: &str,
) -> Result<Vec<(String, AgentPoolSettings)>, String> {
    let dal = cloacina::dal::DAL::new(database.clone());
    let rows = dal
        .agent_pools()
        .list(tenant_id)
        .await
        .map_err(|e| format!("read failed: {e}"))?;
    let mut pools = Vec::with_capacity(rows.len() + 1);
    for (name, text) in rows {
        let settings = parse_stored(&text).map_err(|e| format!("pool '{name}': {e}"))?;
        pools.push((name, settings));
    }
    Ok(with_default_pool(pools))
}

/// Where an actuator reads a tenant's pools from. A trait so that the
/// actuators are unit-testable without a database.
#[async_trait]
pub trait AgentPoolSource: Send + Sync {
    /// The tenant's pools, `default` first and always present.
    async fn pools(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<(String, AgentPoolSettings)>, ActuatorError>;

    /// The counts the autoscaler stored for the tenant's pools other than
    /// `default` (CLOACINA-T-1029). A pool with none runs its min. The
    /// default: none stored.
    async fn stored_counts(
        &self,
        _tenant_id: &str,
    ) -> Result<std::collections::BTreeMap<String, u32>, ActuatorError> {
        Ok(std::collections::BTreeMap::new())
    }
}

/// Real [`AgentPoolSource`]: the `agent_pools` table.
pub struct DalPoolSource {
    database: Database,
}

impl DalPoolSource {
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl AgentPoolSource for DalPoolSource {
    async fn pools(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<(String, AgentPoolSettings)>, ActuatorError> {
        load_pools(&self.database, tenant_id)
            .await
            .map_err(ActuatorError::Settings)
    }

    #[cfg(feature = "postgres")]
    async fn stored_counts(
        &self,
        tenant_id: &str,
    ) -> Result<std::collections::BTreeMap<String, u32>, ActuatorError> {
        let rows = cloacina::dal::DAL::new(self.database.clone())
            .agent_pool_scale()
            .list_all()
            .await
            .map_err(|e| ActuatorError::Settings(format!("pool counts read failed: {e}")))?;
        Ok(rows
            .into_iter()
            .filter(|r| r.tenant_id == tenant_id && r.pool != DEFAULT_AGENT_POOL)
            .filter_map(|r| r.desired_count.map(|d| (r.pool, d)))
            .collect())
    }
}

/// An [`AgentPoolSource`] with no stored pools: every tenant has the pool
/// `default` on the server-wide defaults.
pub struct DefaultPoolOnly;

#[async_trait]
impl AgentPoolSource for DefaultPoolOnly {
    async fn pools(
        &self,
        _tenant_id: &str,
    ) -> Result<Vec<(String, AgentPoolSettings)>, ActuatorError> {
        Ok(with_default_pool(Vec::new()))
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    /// A fixed set of stored pools for every tenant.
    pub struct FixedPools(pub Vec<(String, AgentPoolSettings)>);

    impl FixedPools {
        /// Only the pool `default`, with `settings`.
        pub fn default_pool(settings: AgentPoolSettings) -> Self {
            Self(vec![(DEFAULT_AGENT_POOL.to_string(), settings)])
        }
    }

    #[async_trait]
    impl AgentPoolSource for FixedPools {
        async fn pools(
            &self,
            _tenant_id: &str,
        ) -> Result<Vec<(String, AgentPoolSettings)>, ActuatorError> {
            Ok(with_default_pool(self.0.clone()))
        }
    }

    /// A source that always fails, as an unreadable row does.
    pub struct BrokenPools;

    #[async_trait]
    impl AgentPoolSource for BrokenPools {
        async fn pools(
            &self,
            _tenant_id: &str,
        ) -> Result<Vec<(String, AgentPoolSettings)>, ActuatorError> {
            Err(ActuatorError::Settings("read failed".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloacina_api_types::AgentEgressPort;

    fn nanos(raw: &str) -> i128 {
        Quantity::parse(raw)
            .unwrap_or_else(|e| panic!("{raw}: {e}"))
            .nanos()
    }

    #[test]
    fn quantity_parses_the_kubernetes_notations() {
        assert_eq!(nanos("1"), 1_000_000_000);
        assert_eq!(nanos("+1"), 1_000_000_000);
        assert_eq!(nanos("250m"), 250_000_000);
        assert_eq!(nanos("0.25"), 250_000_000);
        assert_eq!(nanos(".5"), 500_000_000);
        assert_eq!(nanos("1."), 1_000_000_000);
        assert_eq!(nanos("100n"), 100);
        assert_eq!(nanos("5u"), 5_000);
        assert_eq!(nanos("2k"), 2_000_000_000_000);
        assert_eq!(nanos("1e3"), 1_000_000_000_000);
        assert_eq!(nanos("1E3"), 1_000_000_000_000);
        assert_eq!(nanos("15e-1"), 1_500_000_000);
        assert_eq!(Quantity::parse("1Ki").unwrap().units_ceil(), 1024);
        assert_eq!(Quantity::parse("256Mi").unwrap().units_ceil(), 256 << 20);
        assert_eq!(Quantity::parse("32Gi").unwrap().units_ceil(), 32 << 30);
        assert_eq!(
            Quantity::parse("1.5Gi").unwrap().units_ceil(),
            3 * (1 << 29)
        );
        assert_eq!(
            Quantity::parse("1G").unwrap().units_ceil(),
            1_000_000_000i128
        );
        // "E" alone is exa; "E" with digits is an exponent.
        assert_eq!(Quantity::parse("1E").unwrap().units_ceil(), 10i128.pow(18));
        assert_eq!(Quantity::parse("0").unwrap().nanos(), 0);
    }

    #[test]
    fn quantity_compares_across_notations() {
        assert_eq!(Quantity::parse("1Gi"), Quantity::parse("1024Mi"));
        assert_eq!(Quantity::parse("0.1"), Quantity::parse("100m"));
        assert!(Quantity::parse("33Gi").unwrap() > Quantity::parse("32Gi").unwrap());
        assert!(Quantity::parse("1G").unwrap() < Quantity::parse("1Gi").unwrap());
    }

    #[test]
    fn quantity_rounds_up_below_one_nano() {
        assert_eq!(nanos("0.1n"), 1);
        assert_eq!(nanos("1e-30"), 1);
        assert_eq!(Quantity::parse("100m").unwrap().units_ceil(), 1);
    }

    #[test]
    fn quantity_rejects_what_kubernetes_rejects() {
        for raw in [
            "",
            "abc",
            "1gb",
            "1GB",
            "32 Gi",
            "32Gi ",
            "1Kib",
            "1mi",
            "1K",
            "1g",
            "-1",
            "-500m",
            "1.2.3",
            ".",
            "+",
            "1e",
            "1e+",
            "1e1.5",
            "Gi",
            "1,5",
            "0x10",
            "1e99",
            "1ee3",
            "99999999999999999999Ei",
        ] {
            assert!(Quantity::parse(raw).is_err(), "'{raw}' should be refused");
        }
    }

    fn defaults() -> AgentResources {
        AgentResources::default()
    }

    fn ok(settings: &AgentPoolSettings) {
        validate(DEFAULT_AGENT_POOL, settings, &defaults())
            .unwrap_or_else(|e| panic!("should pass: {e}"));
    }

    fn err(settings: &AgentPoolSettings) -> String {
        validate(DEFAULT_AGENT_POOL, settings, &defaults()).expect_err("should be refused")
    }

    // CLOACINA-T-1027: what an agent of a pool reports as its totals.
    #[test]
    fn pool_agent_totals_are_the_limit_or_else_the_request() {
        let pool = |cpu_request: Option<&str>,
                    memory_request: Option<&str>,
                    cpu_limit: Option<&str>,
                    memory_limit: Option<&str>| AgentPoolSettings {
            cpu_request: cpu_request.map(str::to_string),
            memory_request: memory_request.map(str::to_string),
            cpu_limit: cpu_limit.map(str::to_string),
            memory_limit: memory_limit.map(str::to_string),
            ..Default::default()
        };
        let some = |s: &str| Some(s.to_string());
        assert_eq!(
            pool_agent_totals(&pool(Some("1"), Some("8Gi"), Some("4"), Some("32Gi"))),
            (some("4"), some("32Gi"))
        );
        assert_eq!(
            pool_agent_totals(&pool(Some("1"), Some("8Gi"), None, None)),
            (some("1"), some("8Gi"))
        );
        assert_eq!(
            pool_agent_totals(&pool(None, Some("8Gi"), Some("4"), None)),
            (some("4"), some("8Gi"))
        );
        // A pool that sets nothing reports nothing: packed by slots only.
        assert_eq!(
            pool_agent_totals(&AgentPoolSettings::default()),
            (None, None)
        );
    }

    #[test]
    fn pool_names_are_dns_labels_that_fit_object_names() {
        for name in ["default", "big", "mem-32g", "a", "0"] {
            validate_pool_name(name).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
        let long = "x".repeat(MAX_POOL_NAME_LEN + 1);
        for name in [
            "",
            "Big",
            "big_mem",
            "-big",
            "big-",
            "big.mem",
            "a/b",
            long.as_str(),
        ] {
            assert!(
                validate_pool_name(name).is_err(),
                "'{name}' should be refused"
            );
        }
        validate_pool_name(&"x".repeat(MAX_POOL_NAME_LEN)).unwrap();
    }

    #[test]
    fn default_pool_takes_no_agent_count() {
        for settings in [
            AgentPoolSettings {
                max_agents: Some(3),
                ..Default::default()
            },
            AgentPoolSettings {
                min_agents: Some(1),
                ..Default::default()
            },
        ] {
            assert!(err(&settings).contains("not for the pool 'default'"));
        }
    }

    /// CLOACINA-T-1029: a min below the max autoscales the pool between them;
    /// no min keeps it at its max; a min above the max is refused.
    #[test]
    fn other_pool_needs_a_max_and_a_min_not_above_it() {
        let check = |min: Option<u32>, max: Option<u32>| {
            validate(
                "big",
                &AgentPoolSettings {
                    min_agents: min,
                    max_agents: max,
                    ..Default::default()
                },
                &defaults(),
            )
        };
        assert!(check(None, None).unwrap_err().starts_with("max_agents:"));
        assert!(check(Some(1), None).unwrap_err().starts_with("max_agents:"));
        assert!(
            check(Some(3), Some(2))
                .unwrap_err()
                .starts_with("min_agents:")
        );
        check(Some(1), Some(2)).unwrap();
        check(Some(0), Some(5)).unwrap();
        assert!(
            check(None, Some(MAX_POOL_AGENTS + 1))
                .unwrap_err()
                .starts_with("max_agents:")
        );
        check(None, Some(0)).unwrap();
        check(None, Some(2)).unwrap();
        check(Some(2), Some(2)).unwrap();
        assert_eq!(
            fixed_agent_count(&AgentPoolSettings {
                max_agents: Some(2),
                ..Default::default()
            }),
            2
        );
        assert_eq!(fixed_agent_count(&AgentPoolSettings::default()), 0);
        let scaled = AgentPoolSettings {
            min_agents: Some(1),
            max_agents: Some(4),
            ..Default::default()
        };
        assert_eq!(fixed_agent_count(&scaled), 1);
        assert_eq!(pool_agent_count(&scaled, Some(3)), 3);
        assert_eq!(pool_agent_count(&scaled, Some(0)), 1);
        assert_eq!(pool_agent_count(&scaled, Some(7)), 4);
    }

    #[test]
    fn default_pool_is_first_and_always_present() {
        let pools = with_default_pool(vec![(
            "big".to_string(),
            AgentPoolSettings {
                max_agents: Some(1),
                ..Default::default()
            },
        )]);
        assert_eq!(pools[0].0, DEFAULT_AGENT_POOL);
        assert_eq!(pools[0].1, AgentPoolSettings::default());
        assert_eq!(pools[1].0, "big");

        let stored_default = AgentPoolSettings {
            max_concurrency: Some(2),
            ..Default::default()
        };
        let pools = with_default_pool(vec![
            ("big".to_string(), AgentPoolSettings::default()),
            (DEFAULT_AGENT_POOL.to_string(), stored_default.clone()),
        ]);
        assert_eq!(pools.len(), 2);
        assert_eq!(pools[0], (DEFAULT_AGENT_POOL.to_string(), stored_default));
    }

    #[test]
    fn validate_accepts_empty_and_full_settings() {
        ok(&AgentPoolSettings::default());
        let mut annotations = std::collections::BTreeMap::new();
        annotations.insert(
            "eks.amazonaws.com/role-arn".to_string(),
            "arn:aws:iam::000000000000:role/example-team".to_string(),
        );
        ok(&AgentPoolSettings {
            cpu_request: Some("2".into()),
            memory_request: Some("16Gi".into()),
            cpu_limit: Some("8".into()),
            memory_limit: Some("32Gi".into()),
            max_concurrency: Some(1),
            image: Some("registry.example/cloacina-agent:1.2.3".into()),
            service_account_name: Some("team-data".into()),
            service_account_annotations: annotations,
            extra_egress: vec![
                AgentEgressRule {
                    cidr: "198.51.100.0/24".into(),
                    ports: vec![AgentEgressPort {
                        port: 443,
                        protocol: None,
                    }],
                },
                AgentEgressRule {
                    cidr: "2001:db8::/32".into(),
                    ports: vec![AgentEgressPort {
                        port: 53,
                        protocol: Some("UDP".into()),
                    }],
                },
                AgentEgressRule {
                    cidr: "192.0.2.7/32".into(),
                    ports: vec![],
                },
            ],
            min_agents: None,
            max_agents: None,
            pod_labels: [("azure.workload.identity/use", "true"), ("team", "data")]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            pod_annotations: [("example.com/note", "any text, with spaces")]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            automount_service_account_token: Some(false),
        });
    }

    fn map(pairs: &[(&str, &str)]) -> std::collections::BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    /// A label the actuator sets itself cannot be set by a pool: one of them
    /// is the Deployment's selector. The error names the key.
    #[test]
    fn validate_refuses_the_label_keys_the_actuator_owns() {
        for key in [
            "app.kubernetes.io/name",
            "cloacina.managed",
            "cloacina.tenant",
            "cloacina.pool",
        ] {
            let e = err(&AgentPoolSettings {
                pod_labels: map(&[(key, "x")]),
                ..Default::default()
            });
            assert!(e.starts_with("pod_labels:"), "{e}");
            assert!(e.contains(&format!("'{key}'")), "{e}");
            assert!(e.contains("sets itself"), "{e}");
        }
    }

    #[test]
    fn validate_checks_pod_label_keys_and_values() {
        for key in ["", "bad key", "Example.com/use", "example.com/", "-x"] {
            let e = err(&AgentPoolSettings {
                pod_labels: map(&[(key, "true")]),
                ..Default::default()
            });
            assert!(e.contains("not a valid label key"), "{key}: {e}");
        }
        let long = "x".repeat(64);
        for value in ["has space", "-x", "x-", "a/b", long.as_str()] {
            let e = err(&AgentPoolSettings {
                pod_labels: map(&[("example.com/use", value)]),
                ..Default::default()
            });
            assert!(e.contains("not a valid label value"), "{value}: {e}");
        }
        // An empty value is a valid label value.
        ok(&AgentPoolSettings {
            pod_labels: map(&[("example.com/use", ""), ("tier", "a_b.c-1")]),
            ..Default::default()
        });
        let too_many: Vec<(String, String)> = (0..=MAX_POD_METADATA)
            .map(|i| (format!("k{i}"), "v".to_string()))
            .collect();
        assert!(
            err(&AgentPoolSettings {
                pod_labels: too_many.into_iter().collect(),
                ..Default::default()
            })
            .contains("the most is")
        );
    }

    #[test]
    fn validate_checks_pod_annotations() {
        let e = err(&AgentPoolSettings {
            pod_annotations: map(&[("bad key", "x")]),
            ..Default::default()
        });
        assert!(e.starts_with("pod_annotations:"), "{e}");
        let long = "x".repeat(MAX_ANNOTATION_VALUE_BYTES + 1);
        let e = err(&AgentPoolSettings {
            pod_annotations: map(&[("example.com/note", long.as_str())]),
            ..Default::default()
        });
        assert!(e.contains("longer than"), "{e}");
        // An annotation key is not held to the label rules of the actuator's
        // own labels, and its value is free text.
        ok(&AgentPoolSettings {
            pod_annotations: map(&[("cloacina.pool", "free text / any")]),
            ..Default::default()
        });
    }

    #[test]
    fn validate_names_the_field_of_a_bad_quantity() {
        let e = err(&AgentPoolSettings {
            memory_limit: Some("32GB".into()),
            ..Default::default()
        });
        assert!(e.starts_with("memory_limit:"), "{e}");
        let e = err(&AgentPoolSettings {
            cpu_request: Some("half".into()),
            ..Default::default()
        });
        assert!(e.starts_with("cpu_request:"), "{e}");
        let e = err(&AgentPoolSettings {
            cpu_limit: Some("0".into()),
            ..Default::default()
        });
        assert!(e.starts_with("cpu_limit:"), "{e}");
    }

    #[test]
    fn validate_compares_a_request_with_the_limit_in_force() {
        // Both set, request above limit.
        let e = err(&AgentPoolSettings {
            memory_request: Some("33Gi".into()),
            memory_limit: Some("32Gi".into()),
            ..Default::default()
        });
        assert!(
            e.contains("memory_request is more than memory_limit"),
            "{e}"
        );
        // Only the request set: compared with the server-wide limit (1Gi).
        let e = err(&AgentPoolSettings {
            memory_request: Some("8Gi".into()),
            ..Default::default()
        });
        assert!(
            e.contains("memory_request is more than memory_limit"),
            "{e}"
        );
        // Only the limit set, below the server-wide request (250m).
        let e = err(&AgentPoolSettings {
            cpu_limit: Some("100m".into()),
            ..Default::default()
        });
        assert!(e.contains("cpu_request is more than cpu_limit"), "{e}");
        // Equal in two notations is fine.
        ok(&AgentPoolSettings {
            memory_request: Some("1024Mi".into()),
            memory_limit: Some("1Gi".into()),
            ..Default::default()
        });
    }

    #[test]
    fn validate_bounds_max_concurrency() {
        assert!(
            err(&AgentPoolSettings {
                max_concurrency: Some(0),
                ..Default::default()
            })
            .starts_with("max_concurrency:")
        );
        assert!(
            err(&AgentPoolSettings {
                max_concurrency: Some(MAX_CONCURRENCY_CEILING + 1),
                ..Default::default()
            })
            .starts_with("max_concurrency:")
        );
        ok(&AgentPoolSettings {
            max_concurrency: Some(MAX_CONCURRENCY_CEILING),
            ..Default::default()
        });
    }

    #[test]
    fn validate_checks_image_and_service_account() {
        for image in ["", "has space:1", "tab\there"] {
            assert!(
                err(&AgentPoolSettings {
                    image: Some(image.into()),
                    ..Default::default()
                })
                .starts_with("image:")
            );
        }
        for name in ["", "Team", "team_data", "-team", "team-", "a..b"] {
            assert!(
                err(&AgentPoolSettings {
                    service_account_name: Some(name.into()),
                    ..Default::default()
                })
                .starts_with("service_account_name:"),
                "{name}"
            );
        }
        // Annotations with no service account name.
        let mut annotations = std::collections::BTreeMap::new();
        annotations.insert("example.com/role".to_string(), "x".to_string());
        assert!(
            err(&AgentPoolSettings {
                service_account_annotations: annotations.clone(),
                ..Default::default()
            })
            .contains("set service_account_name too")
        );
        // A bad annotation key.
        for key in ["", "bad key", "Example.com/role", "example.com/", "a/b/c"] {
            let mut annotations = std::collections::BTreeMap::new();
            annotations.insert(key.to_string(), "x".to_string());
            assert!(
                err(&AgentPoolSettings {
                    service_account_name: Some("team-data".into()),
                    service_account_annotations: annotations,
                    ..Default::default()
                })
                .contains("not a valid annotation key"),
                "{key}"
            );
        }
        // A value that is too long.
        annotations.insert("example.com/role".to_string(), "x".repeat(5000));
        assert!(
            err(&AgentPoolSettings {
                service_account_name: Some("team-data".into()),
                service_account_annotations: annotations,
                ..Default::default()
            })
            .contains("longer than")
        );
    }

    #[test]
    fn validate_checks_egress_rules() {
        let rule = |cidr: &str, ports: Vec<AgentEgressPort>| AgentPoolSettings {
            extra_egress: vec![AgentEgressRule {
                cidr: cidr.into(),
                ports,
            }],
            ..Default::default()
        };
        for cidr in [
            "",
            "198.51.100.0",
            "198.51.100.0/33",
            "198.51.100.1/24",
            "198.51.100.0/",
            "198.51.100.0/+24",
            "example.com/24",
            "2001:db8::1/32",
            "2001:db8::/129",
            "198.51.100.0/24/1",
        ] {
            let e = err(&rule(cidr, vec![]));
            assert!(e.starts_with("extra_egress[0].cidr:"), "{cidr}: {e}");
        }
        for cidr in ["0.0.0.0/0", "198.51.100.0/24", "192.0.2.7/32", "::/0"] {
            ok(&rule(cidr, vec![]));
        }
        let e = err(&rule(
            "198.51.100.0/24",
            vec![AgentEgressPort {
                port: 0,
                protocol: None,
            }],
        ));
        assert!(e.starts_with("extra_egress[0].ports[0].port:"), "{e}");
        let e = err(&rule(
            "198.51.100.0/24",
            vec![AgentEgressPort {
                port: 443,
                protocol: Some("tcp".into()),
            }],
        ));
        assert!(e.starts_with("extra_egress[0].ports[0].protocol:"), "{e}");

        let too_many = AgentPoolSettings {
            extra_egress: (0..=MAX_EGRESS_RULES)
                .map(|_| AgentEgressRule {
                    cidr: "198.51.100.0/24".into(),
                    ports: vec![],
                })
                .collect(),
            ..Default::default()
        };
        assert!(err(&too_many).contains("the most is"));
    }

    #[test]
    fn stored_settings_round_trip_and_reject_unknown_fields() {
        let settings = AgentPoolSettings {
            memory_limit: Some("32Gi".into()),
            max_concurrency: Some(1),
            ..Default::default()
        };
        let text = serde_json::to_string(&settings).unwrap();
        assert_eq!(text, r#"{"memory_limit":"32Gi","max_concurrency":1}"#);
        assert_eq!(parse_stored(&text).unwrap(), settings);
        assert_eq!(parse_stored("{}").unwrap(), AgentPoolSettings::default());
        // A misspelt field is an error, not a silent default.
        assert!(parse_stored(r#"{"memory_limt":"32Gi"}"#).is_err());
    }
}
