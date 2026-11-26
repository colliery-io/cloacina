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

//! The size of a task context, and the limits on it (CLOACINA-T-1047).
//!
//! A context is stored as JSON text, one full copy per completed task, and on
//! a fleet it rides in the work packet. This module measures the serialized
//! size, names the largest top-level keys, and applies two limits: a warning
//! limit and a hard limit. It is used in two places:
//!
//! - [`super::TaskResultHandler`] checks the context a task returns, for the
//!   in-process and the fleet executor.
//! - [`super::TaskContextBuilder`] checks the input context it builds from
//!   the contexts of the task's dependencies.

use std::collections::HashMap;
use std::io;

use serde::Serialize;
use tracing::warn;

use crate::error::ExecutorError;

/// Default warning limit: 1 MiB.
pub const DEFAULT_CONTEXT_WARN_BYTES: u64 = 1024 * 1024;
/// Default hard limit: 16 MiB.
pub const DEFAULT_CONTEXT_MAX_BYTES: u64 = 16 * 1024 * 1024;
/// How many of the largest keys an error or a warning names.
pub const LARGEST_KEYS_SHOWN: usize = 5;
/// A key name longer than this many characters is cut in an error or a warning.
const KEY_NAME_SHOWN_CHARS: usize = 64;

/// The two limits on the serialized size of a task context.
///
/// A value of 0 turns that limit off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextSizeLimits {
    /// A context larger than this many bytes is logged and counted in
    /// `cloacina_context_size_warnings_total`. 0 = no warning.
    pub warn_bytes: u64,
    /// A context larger than this many bytes fails its task with
    /// [`ExecutorError::ContextTooLarge`]. 0 = no limit.
    pub max_bytes: u64,
}

impl Default for ContextSizeLimits {
    fn default() -> Self {
        Self {
            warn_bytes: DEFAULT_CONTEXT_WARN_BYTES,
            max_bytes: DEFAULT_CONTEXT_MAX_BYTES,
        }
    }
}

impl ContextSizeLimits {
    /// No warning and no limit.
    pub const OFF: Self = Self {
        warn_bytes: 0,
        max_bytes: 0,
    };

    /// `true` when `size_bytes` is over the hard limit.
    pub fn is_over_max(&self, size_bytes: u64) -> bool {
        self.max_bytes != 0 && size_bytes > self.max_bytes
    }

    /// `true` when `size_bytes` is over the warning limit.
    pub fn is_over_warn(&self, size_bytes: u64) -> bool {
        self.warn_bytes != 0 && size_bytes > self.warn_bytes
    }
}

/// Which context of a task was measured. The value of the `kind` label of
/// `cloacina_context_size_bytes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextKind {
    /// The context a task returned.
    Result,
    /// The context a task is given: the merge of its dependencies' contexts,
    /// or the run's first context.
    Input,
}

impl ContextKind {
    /// `result` or `input`.
    pub fn as_str(&self) -> &'static str {
        match self {
            ContextKind::Result => "result",
            ContextKind::Input => "input",
        }
    }
}

/// An `io::Write` that keeps only the number of bytes written.
struct ByteCounter(u64);

impl io::Write for ByteCounter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0 += buf.len() as u64;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The number of bytes of `value` as compact JSON, with no copy of the text.
fn json_len<V: Serialize + ?Sized>(value: &V) -> Result<u64, serde_json::Error> {
    let mut counter = ByteCounter(0);
    serde_json::to_writer(&mut counter, value)?;
    Ok(counter.0)
}

/// The size in bytes of a context as it is stored and sent: the compact JSON
/// text of its data, the same text `Context::to_json` gives.
pub fn serialized_size<V: Serialize>(data: &HashMap<String, V>) -> Result<u64, serde_json::Error> {
    json_len(data)
}

/// The `limit` largest top-level keys of a context with the size in bytes of
/// each value as compact JSON, largest first. Keys of the same size are in
/// name order.
pub fn largest_keys<V: Serialize>(
    data: &HashMap<String, V>,
    limit: usize,
) -> Result<Vec<(String, u64)>, serde_json::Error> {
    let mut sizes = Vec::with_capacity(data.len());
    for (key, value) in data {
        sizes.push((key.clone(), json_len(value)?));
    }
    sizes.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    sizes.truncate(limit);
    Ok(sizes)
}

/// `rows (16777300 bytes), meta (120 bytes)`. A long key name is cut.
pub fn format_largest_keys(keys: &[(String, u64)]) -> String {
    keys.iter()
        .map(|(key, size)| {
            let name: String = if key.chars().count() > KEY_NAME_SHOWN_CHARS {
                let mut cut: String = key.chars().take(KEY_NAME_SHOWN_CHARS).collect();
                cut.push_str("...");
                cut
            } else {
                key.clone()
            };
            format!("{name} ({size} bytes)")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Measure one context of a task and apply the limits.
///
/// - Records the size in the histogram `cloacina_context_size_bytes`
///   (labels `kind`, `tenant`).
/// - Over the warning limit and not over the hard limit: a warn log with the
///   task, the size and the largest keys, and one count in
///   `cloacina_context_size_warnings_total` (label `tenant`).
/// - Over the hard limit: [`ExecutorError::ContextTooLarge`].
///
/// Returns the size in bytes.
pub fn check_context_size<V: Serialize>(
    limits: &ContextSizeLimits,
    tenant: &str,
    task_name: &str,
    kind: ContextKind,
    data: &HashMap<String, V>,
) -> Result<u64, ExecutorError> {
    let size_bytes = serialized_size(data)?;
    metrics::histogram!(
        "cloacina_context_size_bytes",
        "kind" => kind.as_str(),
        "tenant" => tenant.to_string(),
    )
    .record(size_bytes as f64);

    if limits.is_over_max(size_bytes) {
        return Err(ExecutorError::ContextTooLarge {
            kind: kind.as_str(),
            task: task_name.to_string(),
            size_bytes,
            limit_bytes: limits.max_bytes,
            largest_keys: format_largest_keys(&largest_keys(data, LARGEST_KEYS_SHOWN)?),
        });
    }

    if limits.is_over_warn(size_bytes) {
        metrics::counter!(
            "cloacina_context_size_warnings_total",
            "tenant" => tenant.to_string(),
        )
        .increment(1);
        warn!(
            task_name = %task_name,
            kind = kind.as_str(),
            size_bytes,
            warn_bytes = limits.warn_bytes,
            max_bytes = limits.max_bytes,
            largest_keys = %format_largest_keys(&largest_keys(data, LARGEST_KEYS_SHOWN)?),
            "Task context is over the warning size. Write large data to storage \
             in the task and pass a reference in the context."
        );
    }

    Ok(size_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn data(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn serialized_size_is_the_length_of_the_stored_json() {
        let d = data(&[
            ("a", json!(1)),
            ("name", json!("Robin")),
            ("é", json!([1, 2])),
        ]);
        let stored = serde_json::to_string(&d).unwrap();
        assert_eq!(serialized_size(&d).unwrap(), stored.len() as u64);
    }

    #[test]
    fn serialized_size_of_an_empty_context_is_two_bytes() {
        assert_eq!(serialized_size(&data(&[])).unwrap(), 2);
    }

    #[test]
    fn largest_keys_are_sorted_by_size_then_name_and_cut_to_the_limit() {
        let d = data(&[
            ("small", json!(1)),
            ("rows", json!("x".repeat(100))),
            ("b_same", json!("yy")),
            ("a_same", json!("zz")),
            ("mid", json!([1, 2, 3, 4, 5, 6])),
        ]);
        assert_eq!(
            largest_keys(&d, 4).unwrap(),
            vec![
                ("rows".to_string(), 102),
                ("mid".to_string(), 13),
                ("a_same".to_string(), 4),
                ("b_same".to_string(), 4),
            ]
        );
    }

    #[test]
    fn largest_keys_of_fewer_keys_than_the_limit_gives_them_all() {
        let d = data(&[("only", json!(true))]);
        assert_eq!(
            largest_keys(&d, LARGEST_KEYS_SHOWN).unwrap(),
            vec![("only".to_string(), 4)]
        );
    }

    #[test]
    fn format_largest_keys_cuts_a_long_key_name() {
        let long = "k".repeat(200);
        let text = format_largest_keys(&[(long, 7), ("rows".to_string(), 3)]);
        assert_eq!(
            text,
            format!("{}... (7 bytes), rows (3 bytes)", "k".repeat(64))
        );
    }

    #[test]
    fn zero_turns_a_limit_off() {
        let off = ContextSizeLimits::OFF;
        assert!(!off.is_over_max(u64::MAX));
        assert!(!off.is_over_warn(u64::MAX));
    }

    #[test]
    fn the_defaults_are_one_and_sixteen_mebibytes() {
        let limits = ContextSizeLimits::default();
        assert_eq!(limits.warn_bytes, 1_048_576);
        assert_eq!(limits.max_bytes, 16_777_216);
    }

    #[test]
    fn a_size_equal_to_a_limit_is_not_over_it() {
        let limits = ContextSizeLimits {
            warn_bytes: 10,
            max_bytes: 20,
        };
        assert!(!limits.is_over_warn(10));
        assert!(limits.is_over_warn(11));
        assert!(!limits.is_over_max(20));
        assert!(limits.is_over_max(21));
    }

    #[test]
    fn a_context_over_the_hard_limit_is_an_error_with_size_limit_and_keys() {
        let limits = ContextSizeLimits {
            warn_bytes: 10,
            max_bytes: 50,
        };
        let d = data(&[("rows", json!("x".repeat(100))), ("meta", json!(1))]);
        let err = check_context_size(&limits, "public", "t::p::w::load", ContextKind::Result, &d)
            .unwrap_err();
        let size = serialized_size(&d).unwrap();
        assert_eq!(
            err.to_string(),
            format!(
                "Context too large: the result context of task 't::p::w::load' is {size} bytes, \
                 over the limit of 50 bytes. Largest keys: rows (102 bytes), meta (1 bytes). \
                 Write large data to storage in the task and pass a reference in the context."
            )
        );
    }

    #[test]
    fn a_context_under_the_limits_gives_its_size() {
        let d = data(&[("a", json!(1))]);
        let size = check_context_size(
            &ContextSizeLimits::default(),
            "public",
            "t::p::w::load",
            ContextKind::Input,
            &d,
        )
        .unwrap();
        assert_eq!(size, 7);
    }
}
