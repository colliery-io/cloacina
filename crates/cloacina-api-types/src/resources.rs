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

//! Resource quantities, the pool name rule and a task's placement
//! (CLOACINA-T-1021, CLOACINA-T-1027).
//!
//! One copy of the rules for everything that names a pool or a resource
//! amount: the pool routes of the server, the `#[task]` macro, the Python
//! `@task` decorator, the agent's flags and the fleet's selection.

use serde::{Deserialize, Serialize};

use crate::fleet::DEFAULT_AGENT_POOL;

/// Longest pool name. The pool's Deployment (`cloacina-agent-<pool>`) and
/// NetworkPolicy (`cloacina-agent-pool-<pool>`) names must fit in 63.
pub const MAX_POOL_NAME_LEN: usize = 40;

/// A Kubernetes resource quantity (`500m`, `32Gi`, `1e3`), held in nano units
/// so that two notations of one amount compare equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Quantity {
    nanos: i128,
}

impl Quantity {
    /// Parse a quantity with the grammar of the Kubernetes API: a number
    /// (`1`, `1.5`, `.5`), then no suffix, a decimal suffix (`n u m k M G T P
    /// E`), a binary suffix (`Ki Mi Gi Ti Pi Ei`) or a decimal exponent
    /// (`e3`, `E-2`). A negative amount is refused: no resource takes one.
    pub fn parse(raw: &str) -> Result<Self, String> {
        let bad = |why: &str| format!("'{raw}' is not a valid quantity: {why}");
        if raw.is_empty() {
            return Err(bad("it is empty"));
        }
        let rest = match raw.as_bytes()[0] {
            b'-' => return Err(bad("it is negative")),
            b'+' => &raw[1..],
            _ => raw,
        };
        let number_len = rest
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(rest.len());
        let (number, suffix) = rest.split_at(number_len);
        let (whole, frac) = match number.split_once('.') {
            Some((w, f)) => (w, f),
            None => (number, ""),
        };
        if frac.contains('.') {
            return Err(bad("the number has more than one decimal point"));
        }
        if whole.is_empty() && frac.is_empty() {
            return Err(bad("it has no digits"));
        }
        let mut digits: i128 = 0;
        for b in whole.bytes().chain(frac.bytes()) {
            digits = digits
                .checked_mul(10)
                .and_then(|d| d.checked_add(i128::from(b - b'0')))
                .ok_or_else(|| bad("the number is too large"))?;
        }

        // (power of two, power of ten) of the suffix.
        let (pow2, pow10): (u32, i32) = match suffix {
            "" => (0, 0),
            "n" => (0, -9),
            "u" => (0, -6),
            "m" => (0, -3),
            "k" => (0, 3),
            "M" => (0, 6),
            "G" => (0, 9),
            "T" => (0, 12),
            "P" => (0, 15),
            "E" => (0, 18),
            "Ki" => (10, 0),
            "Mi" => (20, 0),
            "Gi" => (30, 0),
            "Ti" => (40, 0),
            "Pi" => (50, 0),
            "Ei" => (60, 0),
            other => {
                let exp = other
                    .strip_prefix('e')
                    .or_else(|| other.strip_prefix('E'))
                    .ok_or_else(|| bad("the suffix is not one Kubernetes knows"))?;
                let exp_digits = exp.strip_prefix(['+', '-']).unwrap_or(exp);
                if exp_digits.is_empty() || !exp_digits.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(bad("the exponent is not a whole number"));
                }
                let exp: i32 = exp.parse().map_err(|_| bad("the exponent is too large"))?;
                if !(-30..=30).contains(&exp) {
                    return Err(bad("the exponent is too large"));
                }
                (0, exp)
            }
        };

        // nanos = digits * 2^pow2 * 10^(pow10 - frac digits + 9), rounded up.
        let too_large = || bad("the amount is too large");
        let mut nanos = digits.checked_mul(1i128 << pow2).ok_or_else(too_large)?;
        let frac_len = i32::try_from(frac.len()).map_err(|_| too_large())?;
        let scale = pow10 - frac_len + 9;
        if scale >= 0 {
            let factor = 10i128
                .checked_pow(scale.unsigned_abs())
                .ok_or_else(too_large)?;
            nanos = nanos.checked_mul(factor).ok_or_else(too_large)?;
        } else {
            match 10i128.checked_pow(scale.unsigned_abs()) {
                Some(divisor) => {
                    nanos = nanos / divisor + i128::from(nanos % divisor != 0);
                }
                // Smaller than any divisor we can hold: rounds up to one nano.
                None => nanos = i128::from(nanos != 0),
            }
        }
        // Kubernetes holds a quantity in an int64 of its unit.
        if nanos / 1_000_000_000 > i128::from(i64::MAX) {
            return Err(too_large());
        }
        Ok(Self { nanos })
    }

    /// The amount in nano units (a CPU quantity of `1` is 1_000_000_000).
    pub fn nanos(&self) -> i128 {
        self.nanos
    }

    /// The amount in whole units, rounded up (bytes, for a memory quantity).
    pub fn units_ceil(&self) -> i128 {
        self.nanos / 1_000_000_000 + i128::from(self.nanos % 1_000_000_000 != 0)
    }

    /// True for an amount of zero.
    pub fn is_zero(&self) -> bool {
        self.nanos == 0
    }
}

/// A DNS-1123 label: lowercase alphanumerics and `-`, alphanumeric at both ends.
pub fn is_dns_label(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
        && b[0] != b'-'
        && b[b.len() - 1] != b'-'
}

/// Check a pool name: a DNS label (lowercase letters, digits and `-`) of
/// [`MAX_POOL_NAME_LEN`] characters at most. It becomes part of object names.
pub fn validate_pool_name(pool: &str) -> Result<(), String> {
    if is_dns_label(pool) && pool.len() <= MAX_POOL_NAME_LEN {
        Ok(())
    } else {
        Err(format!(
            "pool name '{pool}' is not valid: lowercase letters, digits and '-', a letter or \
             digit at both ends, {MAX_POOL_NAME_LEN} characters at most"
        ))
    }
}

/// Parse a CPU amount (`500m`, `4`) to millicores, rounded up. Zero is
/// refused: a request or a total of nothing means nothing.
pub fn parse_cpu_millis(raw: &str) -> Result<u64, String> {
    let quantity = Quantity::parse(raw)?;
    if quantity.is_zero() {
        return Err(format!("'{raw}' is not a valid amount: it is zero"));
    }
    let nanos = quantity.nanos();
    let millis = nanos / 1_000_000 + i128::from(nanos % 1_000_000 != 0);
    u64::try_from(millis).map_err(|_| format!("'{raw}' is not a valid amount: it is too large"))
}

/// Parse a memory amount (`512Mi`, `32Gi`) to bytes, rounded up. Zero is
/// refused.
pub fn parse_memory_bytes(raw: &str) -> Result<u64, String> {
    let quantity = Quantity::parse(raw)?;
    if quantity.is_zero() {
        return Err(format!("'{raw}' is not a valid amount: it is zero"));
    }
    u64::try_from(quantity.units_ceil())
        .map_err(|_| format!("'{raw}' is not a valid amount: it is too large"))
}

/// Where a task wants to run and what it asks for (CLOACINA-T-1027).
///
/// Declared on the task: `#[task(pool = "large", cpu = "4", memory = "32Gi")]`
/// in Rust, `@task(pool="large", cpu="4", memory="32Gi")` in Python. Every
/// field is optional. The fleet places the task on an agent of its pool that
/// has room for the requests. A request is used for placement only: nothing
/// stops a task that uses more than it asked for.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPlacement {
    /// The agent pool the task runs in. `None` means the pool `default`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool: Option<String>,
    /// CPU request, a Kubernetes quantity (`500m`, `4`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu: Option<String>,
    /// Memory request, a Kubernetes quantity (`512Mi`, `32Gi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<String>,
    /// `inline = true`: the author marks the task as one that runs inside the
    /// agent process. Carried with the task; nothing reads it for a decision
    /// yet (CLOACINA-T-1028 will).
    #[serde(default)]
    pub inline: bool,
}

impl TaskPlacement {
    /// Check the declared values: the pool name by the pool name rule, each
    /// request by the quantity grammar and above zero. The error is one plain
    /// sentence that names the attribute.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(pool) = &self.pool {
            validate_pool_name(pool).map_err(|e| format!("pool: {e}"))?;
        }
        if let Some(cpu) = &self.cpu {
            parse_cpu_millis(cpu).map_err(|e| format!("cpu: {e}"))?;
        }
        if let Some(memory) = &self.memory {
            parse_memory_bytes(memory).map_err(|e| format!("memory: {e}"))?;
        }
        Ok(())
    }

    /// The pool the task runs in: the one it names, or `default`.
    pub fn pool_name(&self) -> &str {
        self.pool.as_deref().unwrap_or(DEFAULT_AGENT_POOL)
    }

    /// The CPU request in millicores. `None` when the task declares none, or
    /// when the text does not parse (a declared value is checked when the
    /// task is built, so this is text from a package that was not).
    pub fn cpu_millis(&self) -> Option<u64> {
        self.cpu
            .as_deref()
            .and_then(|raw| parse_cpu_millis(raw).ok())
    }

    /// The memory request in bytes. `None` as for [`Self::cpu_millis`].
    pub fn memory_bytes(&self) -> Option<u64> {
        self.memory
            .as_deref()
            .and_then(|raw| parse_memory_bytes(raw).ok())
    }

    /// True when the task declares nothing: no pool, no request, not inline.
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_is_read_in_millicores() {
        assert_eq!(parse_cpu_millis("500m"), Ok(500));
        assert_eq!(parse_cpu_millis("4"), Ok(4000));
        assert_eq!(parse_cpu_millis("0.1"), Ok(100));
        assert_eq!(parse_cpu_millis("1500u"), Ok(2), "rounded up");
        assert!(parse_cpu_millis("0").is_err());
        assert!(parse_cpu_millis("four").is_err());
        assert!(parse_cpu_millis("-1").is_err());
    }

    #[test]
    fn memory_is_read_in_bytes() {
        assert_eq!(parse_memory_bytes("512Mi"), Ok(512 << 20));
        assert_eq!(parse_memory_bytes("32Gi"), Ok(32 << 30));
        assert_eq!(parse_memory_bytes("1G"), Ok(1_000_000_000));
        assert!(parse_memory_bytes("0Gi").is_err());
        assert!(parse_memory_bytes("32GB").is_err());
    }

    #[test]
    fn a_placement_with_nothing_declared_is_the_default_pool_with_no_request() {
        let placement = TaskPlacement::default();
        assert!(placement.is_default());
        assert_eq!(placement.validate(), Ok(()));
        assert_eq!(placement.pool_name(), DEFAULT_AGENT_POOL);
        assert_eq!(placement.cpu_millis(), None);
        assert_eq!(placement.memory_bytes(), None);
        assert!(!placement.inline);
    }

    #[test]
    fn a_declared_placement_is_read_back_in_numbers() {
        let placement = TaskPlacement {
            pool: Some("large".into()),
            cpu: Some("4".into()),
            memory: Some("32Gi".into()),
            inline: true,
        };
        assert_eq!(placement.validate(), Ok(()));
        assert_eq!(placement.pool_name(), "large");
        assert_eq!(placement.cpu_millis(), Some(4000));
        assert_eq!(placement.memory_bytes(), Some(32 << 30));
    }

    #[test]
    fn validate_names_the_attribute_of_a_bad_value() {
        let bad = |placement: TaskPlacement| placement.validate().unwrap_err();
        let err = bad(TaskPlacement {
            pool: Some("Large_Pool".into()),
            ..Default::default()
        });
        assert!(err.starts_with("pool:"), "{err}");
        let err = bad(TaskPlacement {
            cpu: Some("lots".into()),
            ..Default::default()
        });
        assert!(err.starts_with("cpu:"), "{err}");
        let err = bad(TaskPlacement {
            memory: Some("32GB".into()),
            ..Default::default()
        });
        assert!(err.starts_with("memory:"), "{err}");
        let err = bad(TaskPlacement {
            memory: Some("0".into()),
            ..Default::default()
        });
        assert!(err.contains("zero"), "{err}");
    }

    #[test]
    fn placement_serde_omits_what_is_not_declared() {
        let json = serde_json::to_string(&TaskPlacement::default()).unwrap();
        assert_eq!(json, r#"{"inline":false}"#);
        let back: TaskPlacement = serde_json::from_str("{}").unwrap();
        assert!(back.is_default());
    }
}
