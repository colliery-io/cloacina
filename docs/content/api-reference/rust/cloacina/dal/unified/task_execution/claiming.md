# cloacina::dal::unified::task_execution::claiming <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Task claiming and retry scheduling operations.

All operations are transactional: state changes and execution events
are written atomically. If either fails, both are rolled back.

## Functions

### `cloacina::dal::unified::task_execution::claiming::is_sqlite_busy`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn is_sqlite_busy (err : & diesel :: result :: Error) -> bool
```

CLOACI-T-0622: best-effort detection of a transient SQLite busy/locked condition, used to drive retries inside the sqlite claim path. Diesel surfaces sqlite busy as `DatabaseError(DatabaseErrorKind::Unknown, info)` with an info message like "database is locked" — sqlite's stable strings.

<details>
<summary>Source</summary>

```rust
fn is_sqlite_busy(err: &diesel::result::Error) -> bool {
    use diesel::result::{DatabaseErrorKind, Error};
    if let Error::DatabaseError(DatabaseErrorKind::Unknown, info) = err {
        let msg = info.message();
        return msg.contains("database is locked") || msg.contains("database table is locked");
    }
    false
}
```

</details>
