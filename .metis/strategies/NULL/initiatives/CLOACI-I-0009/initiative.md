---
id: replace-runtime-regex-compilation
level: initiative
title: "Replace Runtime Regex Compilation with Static Compilation"
short_code: "CLOACI-I-0009"
created_at: 2025-11-29T02:40:15.067907+00:00
updated_at: 2025-11-29T02:40:15.067907+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: XS
strategy_id: NULL
initiative_id: replace-runtime-regex-compilation
---

# Replace Runtime Regex Compilation with Static Compilation Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context

In `cloacina/src/packaging/manifest.rs` (line 306), a regex is compiled at runtime with `.expect()`:

```rust
let packaged_workflow_regex =
    Regex::new(r#"#\[packaged_workflow\s*\(\s*[^)]*package\s*=\s*"([^"]+)"[^)]*\)\s*\]"#)
        .expect("Failed to compile regex");  // Line 306
```

**Problems:**
- Runtime compilation on every call (performance cost)
- `.expect()` will panic if regex is invalid
- Pattern is hardcoded and never changes - ideal for static compilation

## Goals & Non-Goals

**Goals:**
- Compile regex once at startup using `lazy_static!` or `once_cell`
- Eliminate panic risk from regex compilation
- Improve performance of package inspection

**Non-Goals:**
- Changing the regex pattern itself
- Adding configurable regex patterns

## Detailed Design

### Using once_cell (Recommended)

```rust
use once_cell::sync::Lazy;
use regex::Regex;

static PACKAGED_WORKFLOW_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"#\[packaged_workflow\s*\(\s*[^)]*package\s*=\s*"([^"]+)"[^)]*\)\s*\]"#)
        .expect("Invalid packaged_workflow regex - this is a bug")
});

// Usage
fn find_packaged_workflows(content: &str) -> Vec<&str> {
    PACKAGED_WORKFLOW_REGEX
        .captures_iter(content)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str()))
        .collect()
}
```

### Alternative: lazy_static

```rust
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref PACKAGED_WORKFLOW_REGEX: Regex = Regex::new(
        r#"#\[packaged_workflow\s*\(\s*[^)]*package\s*=\s*"([^"]+)"[^)]*\)\s*\]"#
    ).expect("Invalid packaged_workflow regex - this is a bug");
}
```

## Testing Strategy

- Verify regex compiles successfully at module load
- Test pattern matching against known inputs
- Benchmark improvement in package inspection

## Alternatives Considered

1. **Keep runtime compilation** - Unnecessary performance cost
2. **Build-time regex (regex-literal)** - Adds build complexity

## Implementation Plan

1. Add `once_cell` dependency (already in workspace)
2. Move regex to static `Lazy<Regex>`
3. Update all usages to reference the static
4. Add test for regex compilation
5. Remove `.expect()` comment explaining it's a compile-time bug if it fails
