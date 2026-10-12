---
title: "cloacina::dal::unified::login_throttle"
description: "Unified login-throttle DAL — brute-force defense for local login (CLOACI-T-0923, closes I-0118 OQ-13 \"required before production\")."
sidebar:
  order: 92
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

## Structs

### <span id="struct.ThrottlePolicy"></span>`struct ThrottlePolicy`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`

Tunables for one throttle scope.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `threshold` | `i32` | Consecutive failures tolerated before the key locks. |
| `base_lock` | `ChronoDuration` | Lock duration for the first failure past the threshold. Each further
failure doubles it, up to `max_lock`. |
| `max_lock` | `ChronoDuration` | Ceiling on the exponential backoff. |
| `decay` | `ChronoDuration` | Idle period after which a counter is considered stale and starts over. |

#### Methods

##### <span id="ThrottlePolicy.username_default"></span>`username_default`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn username_default () -> Self
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



##### <span id="ThrottlePolicy.ip_default"></span>`ip_default`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn ip_default () -> Self
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



##### <span id="ThrottlePolicy.lock_for"></span>`lock_for`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn lock_for (& self , failure_count : i32) -> Option < ChronoDuration >
```

Lock window for the `n`-th consecutive failure (1-based). Failures at or below `threshold` do not lock.

<details>
<summary>Source</summary>

```rust
    fn lock_for(&self, failure_count: i32) -> Option<ChronoDuration> {
        let over = failure_count - self.threshold;
        if over <= 0 {
            return None;
        }
        // 2^(over-1), saturating well before ChronoDuration overflows.
        let shift = (over - 1).clamp(0, 20) as u32;
        let scaled = self
            .base_lock
            .checked_mul(1i32 << shift)
            .unwrap_or(self.max_lock);
        Some(if scaled > self.max_lock {
            self.max_lock
        } else {
            scaled
        })
    }
```

</details>





### <span id="struct.ThrottleState"></span>`struct ThrottleState`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `PartialEq`, `Eq`

The post-write state of a throttle key.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `scope` | `String` |  |
| `failure_count` | `i32` | Consecutive failures after this write. |
| `locked_until` | `Option < DateTime < Utc > >` | Set when the key is locked; the wall past which attempts are accepted
again. |
| `newly_locked` | `bool` | True when *this* write is what pushed the key into a lock (the edge the
caller audits + counts, so a lockout is logged once, not once per
subsequent attempt). |

#### Methods

##### <span id="ThrottleState.is_locked_at"></span>`is_locked_at`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn is_locked_at (& self , now : DateTime < Utc >) -> bool
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





### <span id="struct.LoginThrottleDAL"></span>`struct LoginThrottleDAL<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Clone`

Data access layer for failed-login counters.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



## Functions

### <span id="fn.username_key"></span>`fn username_key`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn username_key (tenant : Option < & str > , username : & str) -> String
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



### <span id="fn.ip_key"></span>`fn ip_key`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn ip_key (ip : & str) -> String
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



