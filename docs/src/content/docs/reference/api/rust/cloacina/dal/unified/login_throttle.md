---
title: "cloacina::dal::unified::login_throttle"
description: "Unified login-throttle DAL — brute-force defense for local login (CLOACI-T-0923, closes I-0118 OQ-13 \"required before production\")."
editUrl: false
sidebar:
  order: 76
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Unified login-throttle DAL — brute-force defense for local login (CLOACI-T-0923, closes I-0118 OQ-13 "required before production").

Argon2id makes each guess expensive for the **server**, not for the
attacker. Nothing previously bounded sustained guessing against a known
username, so `/v1/auth/local/login` was an open credential-stuffing target.

**Persisted, not per-replica.** A counter in one replica's memory is evaded
by spraying attempts across replicas behind a load balancer, and a lockout
decided on replica A is invisible to replica B. Same lesson (and same DAL
shape) as the T-0916 `ws_tickets` move.

**Why dual-keyed**

Every failed attempt increments **two** independent counters, and a login is
refused if **either** is locked:

| key | catches | threshold |
|-----|---------|-----------|
| `u:<tenant-or-_>/<username>` | an attacker rotating source IPs against one account | low (5) |
| `ip:<source>` | one host spraying many usernames, none of which alone trips | high (50) |

Neither scope alone is sufficient: username-only lets a single host walk an
entire user list; IP-only is defeated by a botnet or an open proxy pool. The
obvious third option — a composite `(username, ip)` key — is strictly the
worst, because it resets on every new source IP and therefore stops neither
attack. The IP threshold is deliberately an order of magnitude higher than
the username one so a shared-NAT office does not lock itself out over a
handful of typos.

**Not an enumeration oracle**

The caller records a failure for an **unknown** username exactly as for a
real one, so a locked key's `429` says only "this key has been failing",
never "this account exists". See `routes/local_auth.rs`.

**Decay and reset**

A counter whose last failure is older than `ThrottlePolicy::decay` starts
over. A successful login clears the *username* key outright (a legitimate
user who finally remembers their password is not left in a hole). The IP key
is deliberately **not** cleared on success — otherwise an attacker with one
valid account of their own could zero the spray counter at will — and is
left to decay instead.

## Structs

### <span id="struct.ThrottlePolicy"></span><code style="font-size: 0.85em">struct ThrottlePolicy</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`

Tunables for one throttle scope.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `threshold` | `i32` | Consecutive failures tolerated before the key locks. |
| `base_lock` | `ChronoDuration` | Lock duration for the first failure past the threshold. Each further failure doubles it, up to `max_lock`. |
| `max_lock` | `ChronoDuration` | Ceiling on the exponential backoff. |
| `decay` | `ChronoDuration` | Idle period after which a counter is considered stale and starts over. |

#### Methods

##### <span id="ThrottlePolicy.username_default"></span><code>username_​default</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn username_default() -> Self
```

Default username-scoped policy: 5 strikes, then 30s doubling to a 15min cap, counters forgotten after 15 idle minutes.

<details>
<summary>Source</summary>

```rust
    pub fn username_default() -> Self {
        Self {
            threshold: 5,
            base_lock: ChronoDuration::seconds(30),
            max_lock: ChronoDuration::minutes(15),
            decay: ChronoDuration::minutes(15),
        }
    }
```

</details>

##### <span id="ThrottlePolicy.ip_default"></span><code>ip_​default</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn ip_default() -> Self
```

Default source-IP policy: an order of magnitude looser than the username policy so shared NAT does not self-inflict a lockout, but still bounds a single host spraying a user list.

<details>
<summary>Source</summary>

```rust
    pub fn ip_default() -> Self {
        Self {
            threshold: 50,
            base_lock: ChronoDuration::minutes(15),
            max_lock: ChronoDuration::minutes(15),
            decay: ChronoDuration::minutes(15),
        }
    }
```

</details>

### <span id="struct.ThrottleState"></span><code style="font-size: 0.85em">struct ThrottleState</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `PartialEq`, `Eq`

The post-write state of a throttle key.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `scope` | `String` |  |
| `failure_count` | `i32` | Consecutive failures after this write. |
| `locked_until` | `Option<DateTime<Utc>>` | Set when the key is locked; the wall past which attempts are accepted again. |
| `newly_locked` | `bool` | True when *this* write is what pushed the key into a lock (the edge the caller audits + counts, so a lockout is logged once, not once per subsequent attempt). |

#### Methods

##### <span id="ThrottleState.is_locked_at"></span><code>is_​locked_​at</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_locked_at(&self, now: DateTime<Utc>) -> bool
```

Whether the key is locked as of `now`.

<details>
<summary>Source</summary>

```rust
    pub fn is_locked_at(&self, now: DateTime<Utc>) -> bool {
        self.locked_until.map(|t| t > now).unwrap_or(false)
    }
```

</details>

### <span id="struct.LoginThrottleDAL"></span><code style="font-size: 0.85em">struct LoginThrottleDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Data access layer for failed-login counters.

#### Methods

##### <span id="LoginThrottleDAL.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: &'a DAL) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }
```

</details>

##### <span id="LoginThrottleDAL.locked_until"></span><code>locked_​until</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn locked_until(
    &self,
    key: &str,
) -> Result<Option<DateTime<Utc>>, ValidationError>
```

Is `key` currently locked? Returns the lock expiry when it is.

Read-only and cheap — this is the pre-password-verify gate, so it must
not itself become the expensive part of a login attempt.

<details>
<summary>Source</summary>

```rust
    pub async fn locked_until(&self, key: &str) -> Result<Option<DateTime<Utc>>, ValidationError> {
        let key = key.to_string();
        let now = UniversalTimestamp::now();
        let row: Option<UnifiedLoginThrottle> = crate::interact_on_backend!(self.dal, |conn| {
            login_throttle::table
                .filter(login_throttle::throttle_key.eq(&key))
                .first::<UnifiedLoginThrottle>(conn)
                .optional()
        })
        .map_err(ValidationError::from)?;
        Ok(row
            .and_then(|r| r.locked_until)
            .filter(|t| t.0 > now.0)
            .map(|t| t.0))
    }
```

</details>

##### <span id="LoginThrottleDAL.record_failure"></span><code>record_​failure</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn record_failure(
    &self,
    key: &str,
    scope: &str,
    policy: ThrottlePolicy,
) -> Result<ThrottleState, ValidationError>
```

Record one failed attempt against `key` and return the resulting state.

Read-modify-write inside a single transaction so two concurrent
attempts (on any replica) cannot both read the same count and write the
same increment. A counter idle longer than `policy.decay` restarts at 1.

<details>
<summary>Source</summary>

```rust
    pub async fn record_failure(
        &self,
        key: &str,
        scope: &str,
        policy: ThrottlePolicy,
    ) -> Result<ThrottleState, ValidationError> {
        let key = key.to_string();
        let scope = scope.to_string();
        let now = UniversalTimestamp::now();

        let state: ThrottleState = crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<ThrottleState, diesel::result::Error, _>(|conn| {
                let existing: Option<UnifiedLoginThrottle> = login_throttle::table
                    .filter(login_throttle::throttle_key.eq(&key))
                    .first::<UnifiedLoginThrottle>(conn)
                    .optional()?;

                // Was the key already locked before this write? Used to report
                // the lock *edge* exactly once.
                let was_locked = existing
                    .as_ref()
                    .and_then(|r| r.locked_until)
                    .map(|t| t.0 > now.0)
                    .unwrap_or(false);

                let (failure_count, first_failure_at) = match &existing {
                    // Stale counter — the attacker (or the forgetful human)
                    // went quiet long enough; start over.
                    Some(r) if now.0 - r.last_failure_at.0 > policy.decay => (1, now),
                    Some(r) => (r.failure_count.saturating_add(1), r.first_failure_at),
                    None => (1, now),
                };

                let locked_until = policy
                    .lock_for(failure_count)
                    .map(|d| UniversalTimestamp(now.0 + d));

                let row = NewUnifiedLoginThrottle {
                    throttle_key: key.clone(),
                    scope: scope.clone(),
                    failure_count,
                    first_failure_at,
                    last_failure_at: now,
                    locked_until,
                };

                if existing.is_some() {
                    diesel::update(
                        login_throttle::table.filter(login_throttle::throttle_key.eq(&key)),
                    )
                    .set(&row)
                    .execute(conn)?;
                } else {
                    diesel::insert_into(login_throttle::table)
                        .values(&row)
                        .execute(conn)?;
                }

                Ok(ThrottleState {
                    scope: scope.clone(),
                    failure_count,
                    locked_until: locked_until.map(|t| t.0),
                    newly_locked: locked_until.is_some() && !was_locked,
                })
            })
        })
        .map_err(ValidationError::from)?;

        Ok(state)
    }
```

</details>

##### <span id="LoginThrottleDAL.clear"></span><code>clear</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn clear(&self, key: &str) -> Result<bool, ValidationError>
```

Forget `key` entirely — called on a successful login so a legitimate user's counter never carries over. Returns true if a row was removed.

<details>
<summary>Source</summary>

```rust
    pub async fn clear(&self, key: &str) -> Result<bool, ValidationError> {
        let key = key.to_string();
        let n: usize = crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(login_throttle::table.filter(login_throttle::throttle_key.eq(&key)))
                .execute(conn)
        })
        .map_err(ValidationError::from)?;
        Ok(n > 0)
    }
```

</details>

##### <span id="LoginThrottleDAL.prune_idle"></span><code>prune_​idle</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn prune_idle(
    &self,
    older_than: ChronoDuration,
) -> Result<usize, ValidationError>
```

Drop counters idle for longer than `older_than` **and** not currently locked. Keeps the table bounded without a dedicated sweeper; the login route calls it opportunistically.

<details>
<summary>Source</summary>

```rust
    pub async fn prune_idle(&self, older_than: ChronoDuration) -> Result<usize, ValidationError> {
        let now = UniversalTimestamp::now();
        let cutoff = UniversalTimestamp(now.0 - older_than);
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(
                login_throttle::table
                    .filter(login_throttle::last_failure_at.lt(cutoff))
                    .filter(
                        login_throttle::locked_until
                            .is_null()
                            .or(login_throttle::locked_until.lt(now)),
                    ),
            )
            .execute(conn)
        })
        .map_err(ValidationError::from)
    }
```

</details>

## Functions

### <span id="fn.username_key"></span><code style="font-size: 0.85em">fn username_​key</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn username_key(tenant: Option<&str>, username: &str) -> String
```

Build the username-scoped throttle key. `tenant` is the login request's tenant selector; `None` (a global account) collapses to `_` so the two namespaces never collide.

<details>
<summary>Source</summary>

```rust
pub fn username_key(tenant: Option<&str>, username: &str) -> String {
    format!("u:{}/{}", tenant.unwrap_or("_"), username)
}
```

</details>

### <span id="fn.ip_key"></span><code style="font-size: 0.85em">fn ip_​key</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn ip_key(ip: &str) -> String
```

Build the source-IP-scoped throttle key.

<details>
<summary>Source</summary>

```rust
pub fn ip_key(ip: &str) -> String {
    format!("ip:{ip}")
}
```

</details>

## Constants

| Name | Type | Value | Description |
|------|------|-------|-------------|
| <span id="const.SCOPE_USERNAME"></span>`SCOPE_USERNAME` | `&str` | `"username"` | Scope label stored on a row (also the metric/audit label). |
| <span id="const.SCOPE_IP"></span>`SCOPE_IP` | `&str` | `"ip"` | Scope label for source-IP rows. |
