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

//! Glob pattern matching for workflow-level ABAC.

/// Check if a workflow name matches any of the allowed patterns.
///
/// - Empty patterns = unrestricted (allow all)
/// - Non-empty = at least one must match
/// - `*` matches any sequence of characters
/// - `::` is treated as a regular namespace separator (not special)
pub fn check_workflow_access(patterns: &[String], workflow_name: &str) -> bool {
    if patterns.is_empty() {
        return true; // unrestricted
    }
    patterns.iter().any(|p| glob_match(p, workflow_name))
}

/// Simple glob matching: `*` matches any sequence of characters.
fn glob_match(pattern: &str, text: &str) -> bool {
    let mut p_idx = 0;
    let mut t_idx = 0;
    let mut star_p_idx: Option<usize> = None;
    let mut star_t_idx: Option<usize> = None;
    let p_bytes = pattern.as_bytes();
    let t_bytes = text.as_bytes();

    while t_idx < t_bytes.len() {
        if p_idx < p_bytes.len() && p_bytes[p_idx] == b'*' {
            star_p_idx = Some(p_idx);
            star_t_idx = Some(t_idx);
            p_idx += 1;
        } else if p_idx < p_bytes.len()
            && (p_bytes[p_idx] == t_bytes[t_idx] || p_bytes[p_idx] == b'?')
        {
            p_idx += 1;
            t_idx += 1;
        } else if let Some(sp) = star_p_idx {
            p_idx = sp + 1;
            star_t_idx = Some(star_t_idx.unwrap() + 1);
            t_idx = star_t_idx.unwrap();
        } else {
            return false;
        }
    }

    while p_idx < p_bytes.len() && p_bytes[p_idx] == b'*' {
        p_idx += 1;
    }

    p_idx == p_bytes.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_patterns_allows_all() {
        assert!(check_workflow_access(&[], "anything"));
    }

    #[test]
    fn test_exact_match() {
        let patterns = vec!["etl::daily_load".to_string()];
        assert!(check_workflow_access(&patterns, "etl::daily_load"));
        assert!(!check_workflow_access(&patterns, "etl::weekly_load"));
    }

    #[test]
    fn test_glob_star() {
        let patterns = vec!["etl::*".to_string()];
        assert!(check_workflow_access(&patterns, "etl::daily_load"));
        assert!(check_workflow_access(&patterns, "etl::weekly_load"));
        assert!(!check_workflow_access(&patterns, "reports::daily"));
    }

    #[test]
    fn test_multiple_patterns() {
        let patterns = vec!["etl::*".to_string(), "reports::*".to_string()];
        assert!(check_workflow_access(&patterns, "etl::daily_load"));
        assert!(check_workflow_access(&patterns, "reports::monthly"));
        assert!(!check_workflow_access(&patterns, "ml::training"));
    }

    #[test]
    fn test_star_matches_everything() {
        let patterns = vec!["*".to_string()];
        assert!(check_workflow_access(&patterns, "anything::at::all"));
    }

    #[test]
    fn test_no_match() {
        let patterns = vec!["etl::*".to_string()];
        assert!(!check_workflow_access(&patterns, "reports::daily"));
    }

    #[test]
    fn test_glob_match_basic() {
        assert!(glob_match("hello", "hello"));
        assert!(!glob_match("hello", "world"));
        assert!(glob_match("hell*", "hello"));
        assert!(glob_match("*lo", "hello"));
        assert!(glob_match("h*o", "hello"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("", ""));
        assert!(!glob_match("", "something"));
    }
}
