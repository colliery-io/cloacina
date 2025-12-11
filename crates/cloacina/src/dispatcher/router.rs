/*
 *  Copyright 2025 Colliery Software
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

//! Task routing logic for the dispatcher.
//!
//! This module implements pattern matching for routing tasks to executors
//! based on configurable rules.

use super::types::{RoutingConfig, RoutingRule};

/// Router for matching tasks to executor keys.
///
/// Evaluates routing rules in order and returns the first matching executor,
/// or the default executor if no rules match.
#[derive(Debug, Clone)]
pub struct Router {
    config: RoutingConfig,
}

impl Router {
    /// Creates a new router with the given configuration.
    pub fn new(config: RoutingConfig) -> Self {
        Self { config }
    }

    /// Resolves the executor key for a given task name.
    ///
    /// Rules are evaluated in order, and the first matching rule's executor
    /// is returned. If no rules match, the default executor is returned.
    ///
    /// # Arguments
    ///
    /// * `task_name` - The fully qualified task name (e.g., "workflow::task")
    ///
    /// # Returns
    ///
    /// The executor key to use for this task.
    pub fn resolve(&self, task_name: &str) -> &str {
        for rule in &self.config.rules {
            if Self::matches_pattern(&rule.task_pattern, task_name) {
                return &rule.executor;
            }
        }
        &self.config.default_executor
    }

    /// Checks if a task name matches a glob pattern.
    ///
    /// Supports:
    /// - `*` matches any sequence of characters within a segment
    /// - `**` matches any sequence including namespace separators
    /// - Exact matches
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// assert!(Router::matches_pattern("ml::*", "ml::train"));
    /// assert!(Router::matches_pattern("ml::*", "ml::predict"));
    /// assert!(!Router::matches_pattern("ml::*", "etl::extract"));
    /// assert!(Router::matches_pattern("**::heavy_*", "ml::heavy_train"));
    /// assert!(Router::matches_pattern("*", "any_task"));
    /// ```
    fn matches_pattern(pattern: &str, task_name: &str) -> bool {
        // Handle exact match first
        if pattern == task_name {
            return true;
        }

        // Handle ** (match anything including ::)
        if pattern == "**" {
            return true;
        }

        // Split into segments for matching
        let pattern_parts: Vec<&str> = pattern.split("::").collect();
        let name_parts: Vec<&str> = task_name.split("::").collect();

        Self::match_segments(&pattern_parts, &name_parts)
    }

    /// Recursively matches pattern segments against name segments.
    fn match_segments(pattern_parts: &[&str], name_parts: &[&str]) -> bool {
        match (pattern_parts.first(), name_parts.first()) {
            // Both exhausted - match
            (None, None) => true,

            // Pattern exhausted but name remains - no match
            (None, Some(_)) => false,

            // Name exhausted but pattern remains - only match if pattern is **
            (Some(&"**"), None) => pattern_parts.len() == 1,
            (Some(_), None) => false,

            // ** matches zero or more segments
            (Some(&"**"), Some(_)) => {
                // Try matching ** against zero segments
                if Self::match_segments(&pattern_parts[1..], name_parts) {
                    return true;
                }
                // Try matching ** against one segment and continue
                Self::match_segments(pattern_parts, &name_parts[1..])
            }

            // Regular segment matching
            (Some(pattern_seg), Some(name_seg)) => {
                if Self::match_glob(pattern_seg, name_seg) {
                    Self::match_segments(&pattern_parts[1..], &name_parts[1..])
                } else {
                    false
                }
            }
        }
    }

    /// Matches a single segment with glob patterns (* only).
    fn match_glob(pattern: &str, text: &str) -> bool {
        // Exact match
        if pattern == text {
            return true;
        }

        // Single * matches anything
        if pattern == "*" {
            return true;
        }

        // Pattern with * wildcards
        if pattern.contains('*') {
            return Self::match_wildcard(pattern, text);
        }

        false
    }

    /// Matches text against a pattern with * wildcards.
    fn match_wildcard(pattern: &str, text: &str) -> bool {
        let parts: Vec<&str> = pattern.split('*').collect();

        if parts.len() == 1 {
            // No wildcards
            return pattern == text;
        }

        let mut text_pos = 0;
        let text_bytes = text.as_bytes();

        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }

            let part_bytes = part.as_bytes();

            if i == 0 {
                // First part must match at start
                if !text.starts_with(part) {
                    return false;
                }
                text_pos = part.len();
            } else if i == parts.len() - 1 {
                // Last part must match at end
                if !text.ends_with(part) {
                    return false;
                }
            } else {
                // Middle parts must be found somewhere after current position
                if let Some(pos) = Self::find_substring(&text_bytes[text_pos..], part_bytes) {
                    text_pos += pos + part.len();
                } else {
                    return false;
                }
            }
        }

        true
    }

    /// Finds substring position in byte slice.
    fn find_substring(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    /// Gets the current routing configuration.
    pub fn config(&self) -> &RoutingConfig {
        &self.config
    }

    /// Adds a new routing rule.
    pub fn add_rule(&mut self, rule: RoutingRule) {
        self.config.rules.push(rule);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let config = RoutingConfig::new("default").with_rule(RoutingRule::new("ml::train", "gpu"));
        let router = Router::new(config);

        assert_eq!(router.resolve("ml::train"), "gpu");
        assert_eq!(router.resolve("ml::predict"), "default");
    }

    #[test]
    fn test_wildcard_match() {
        let config = RoutingConfig::new("default").with_rule(RoutingRule::new("ml::*", "gpu"));
        let router = Router::new(config);

        assert_eq!(router.resolve("ml::train"), "gpu");
        assert_eq!(router.resolve("ml::predict"), "gpu");
        assert_eq!(router.resolve("etl::extract"), "default");
    }

    #[test]
    fn test_double_wildcard() {
        let config = RoutingConfig::new("default").with_rule(RoutingRule::new("**", "catch_all"));
        let router = Router::new(config);

        assert_eq!(router.resolve("anything"), "catch_all");
        assert_eq!(router.resolve("ml::deep::nested"), "catch_all");
    }

    #[test]
    fn test_prefix_wildcard() {
        let config =
            RoutingConfig::new("default").with_rule(RoutingRule::new("heavy_*", "high_memory"));
        let router = Router::new(config);

        assert_eq!(router.resolve("heavy_compute"), "high_memory");
        assert_eq!(router.resolve("light_compute"), "default");
    }

    #[test]
    fn test_suffix_wildcard() {
        let config =
            RoutingConfig::new("default").with_rule(RoutingRule::new("*_gpu", "gpu_executor"));
        let router = Router::new(config);

        assert_eq!(router.resolve("train_gpu"), "gpu_executor");
        assert_eq!(router.resolve("train_cpu"), "default");
    }

    #[test]
    fn test_rule_order_priority() {
        let config = RoutingConfig::new("default")
            .with_rule(RoutingRule::new("ml::train", "specific"))
            .with_rule(RoutingRule::new("ml::*", "general"));
        let router = Router::new(config);

        // First matching rule wins
        assert_eq!(router.resolve("ml::train"), "specific");
        assert_eq!(router.resolve("ml::predict"), "general");
    }

    #[test]
    fn test_namespace_wildcard() {
        let config = RoutingConfig::new("default")
            .with_rule(RoutingRule::new("**::heavy_*", "high_compute"));
        let router = Router::new(config);

        assert_eq!(router.resolve("ml::heavy_train"), "high_compute");
        assert_eq!(router.resolve("etl::data::heavy_load"), "high_compute");
        assert_eq!(router.resolve("ml::light_train"), "default");
    }
}
