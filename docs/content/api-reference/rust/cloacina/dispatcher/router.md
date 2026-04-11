# cloacina::dispatcher::router <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Task routing logic for the dispatcher.

This module implements pattern matching for routing tasks to executors
based on configurable rules.

## Structs

### `cloacina::dispatcher::router::Router`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Router for matching tasks to executor keys.

Evaluates routing rules in order and returns the first matching executor,
or the default executor if no rules match.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `config` | `RoutingConfig` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (config : RoutingConfig) -> Self
```

Creates a new router with the given configuration.

<details>
<summary>Source</summary>

```rust
    pub fn new(config: RoutingConfig) -> Self {
        Self { config }
    }
```

</details>



##### `resolve` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn resolve (& self , task_name : & str) -> & str
```

Resolves the executor key for a given task name.

Rules are evaluated in order, and the first matching rule's executor
is returned. If no rules match, the default executor is returned.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task_name` | `-` | The fully qualified task name (e.g., "workflow::task") |


**Returns:**

The executor key to use for this task.

<details>
<summary>Source</summary>

```rust
    pub fn resolve(&self, task_name: &str) -> &str {
        for rule in &self.config.rules {
            if Self::matches_pattern(&rule.task_pattern, task_name) {
                return &rule.executor;
            }
        }
        &self.config.default_executor
    }
```

</details>



##### `matches_pattern` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn matches_pattern (pattern : & str , task_name : & str) -> bool
```

Checks if a task name matches a glob pattern.

Supports:
- `*` matches any sequence of characters within a segment
- `**` matches any sequence including namespace separators
- Exact matches

**Examples:**

```rust,ignore
assert!(Router::matches_pattern("ml::*", "ml::train"));
assert!(Router::matches_pattern("ml::*", "ml::predict"));
assert!(!Router::matches_pattern("ml::*", "etl::extract"));
assert!(Router::matches_pattern("**::heavy_*", "ml::heavy_train"));
assert!(Router::matches_pattern("*", "any_task"));
```

<details>
<summary>Source</summary>

```rust
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
```

</details>



##### `match_segments` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn match_segments (pattern_parts : & [& str] , name_parts : & [& str]) -> bool
```

Recursively matches pattern segments against name segments.

<details>
<summary>Source</summary>

```rust
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
```

</details>



##### `match_glob` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn match_glob (pattern : & str , text : & str) -> bool
```

Matches a single segment with glob patterns (* only).

<details>
<summary>Source</summary>

```rust
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
```

</details>



##### `match_wildcard` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn match_wildcard (pattern : & str , text : & str) -> bool
```

Matches text against a pattern with * wildcards.

<details>
<summary>Source</summary>

```rust
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
```

</details>



##### `find_substring` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn find_substring (haystack : & [u8] , needle : & [u8]) -> Option < usize >
```

Finds substring position in byte slice.

<details>
<summary>Source</summary>

```rust
    fn find_substring(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }
```

</details>



##### `config` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn config (& self) -> & RoutingConfig
```

Gets the current routing configuration.

<details>
<summary>Source</summary>

```rust
    pub fn config(&self) -> &RoutingConfig {
        &self.config
    }
```

</details>



##### `add_rule` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn add_rule (& mut self , rule : RoutingRule)
```

Adds a new routing rule.

<details>
<summary>Source</summary>

```rust
    pub fn add_rule(&mut self, rule: RoutingRule) {
        self.config.rules.push(rule);
    }
```

</details>
