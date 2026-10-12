---
title: "cloacina::computation_graph::reactor_ownership"
description: "Per-reactor ownership via a single long-lived advisory-lock session (CLOACI-T-0851, [`ADR CLOACI-A-0012`])."
sidebar:
  order: 63
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Per-reactor ownership via a single long-lived advisory-lock session (CLOACI-T-0851, [`ADR CLOACI-A-0012`]).

## Structs

### <span id="struct.ReactorId"></span>`struct ReactorId`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`

Identifies a reactor for ownership purposes: the tenant scope it was loaded under (`None` for single-tenant / admin-owned) plus its name.

This mirrors how `ComputationGraphScheduler` keys its `reactors` map, so an
ownership claim and the in-process reactor it guards cannot drift apart.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tenant` | `Option < String >` |  |
| `name` | `String` |  |

#### Methods

##### <span id="ReactorId.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn new (tenant : Option < impl Into < String > > , name : impl Into < String >) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(tenant: Option<impl Into<String>>, name: impl Into<String>) -> Self {
        Self {
            tenant: tenant.map(Into::into),
            name: name.into(),
        }
    }
```

</details>



##### <span id="ReactorId.lock_key"></span>`lock_key`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn lock_key (& self) -> i64
```

The database-wide advisory-lock key for this reactor.

<details>
<summary>Source</summary>

```rust
    pub fn lock_key(&self) -> i64 {
        reactor_lock_key(self.tenant.as_deref(), &self.name)
    }
```

</details>





### <span id="struct.OwnershipState"></span>`struct OwnershipState`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Default`

Tracks which reactors this replica believes it owns.

The database is the source of truth; this set is only what we *believe*, and
[`OwnershipCheck`] exists precisely because belief and truth can diverge.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `held` | `HashSet < ReactorId >` |  |

#### Methods

##### <span id="OwnershipState.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn new () -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Self {
        Self::default()
    }
```

</details>



##### <span id="OwnershipState.record_claimed"></span>`record_claimed`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn record_claimed (& mut self , id : ReactorId)
```

Record a successful claim.

<details>
<summary>Source</summary>

```rust
    pub fn record_claimed(&mut self, id: ReactorId) {
        self.held.insert(id);
    }
```

</details>



##### <span id="OwnershipState.record_released"></span>`record_released`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn record_released (& mut self , id : & ReactorId)
```

Forget a reactor — after an explicit release, or after a liveness check reported it lost.

<details>
<summary>Source</summary>

```rust
    pub fn record_released(&mut self, id: &ReactorId) {
        self.held.remove(id);
    }
```

</details>



##### <span id="OwnershipState.believes_owned"></span>`believes_owned`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn believes_owned (& self , id : & ReactorId) -> bool
```

<details>
<summary>Source</summary>

```rust
    pub fn believes_owned(&self, id: &ReactorId) -> bool {
        self.held.contains(id)
    }
```

</details>



##### <span id="OwnershipState.owned"></span>`owned`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn owned (& self) -> impl Iterator < Item = & ReactorId >
```

<details>
<summary>Source</summary>

```rust
    pub fn owned(&self) -> impl Iterator<Item = &ReactorId> {
        self.held.iter()
    }
```

</details>



##### <span id="OwnershipState.len"></span>`len`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn len (& self) -> usize
```

<details>
<summary>Source</summary>

```rust
    pub fn len(&self) -> usize {
        self.held.len()
    }
```

</details>



##### <span id="OwnershipState.is_empty"></span>`is_empty`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn is_empty (& self) -> bool
```

<details>
<summary>Source</summary>

```rust
    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }
```

</details>



##### <span id="OwnershipState.diff_against_held_keys"></span>`diff_against_held_keys`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn diff_against_held_keys (& self , held_keys : & HashSet < i64 >) -> Vec < ReactorId >
```

Given the set of keys Postgres reports this session still holds, return the reactors we believe we own but do not.

Kept as a pure function so the divergence logic — the part that decides
whether reactors get stopped — is testable without a database.

<details>
<summary>Source</summary>

```rust
    pub fn diff_against_held_keys(&self, held_keys: &HashSet<i64>) -> Vec<ReactorId> {
        let mut lost: Vec<ReactorId> = self
            .held
            .iter()
            .filter(|id| !held_keys.contains(&id.lock_key()))
            .cloned()
            .collect();
        // Deterministic order so logs and tests do not depend on HashSet
        // iteration order.
        lost.sort_by(|a, b| (&a.tenant, &a.name).cmp(&(&b.tenant, &b.name)));
        lost
    }
```

</details>





### <span id="struct.OwnershipWatchdog"></span>`struct OwnershipWatchdog`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`

Turns a sequence of [`OwnershipCheck`]s into decisions.

The interesting case is [`OwnershipCheck::Indeterminate`], which the ADR
amendment deliberately leaves as a policy question. Both naive answers are
wrong:
* **Treat it as healthy** → a replica partitioned from Postgres keeps running
its reactors forever. But a partition is exactly what KILLS the ownership
session, and Postgres releases the locks the moment that connection drops,
so another replica will legitimately take over. Optimism here produces the
split-brain in its worst form: indefinite, and invisible.
* **Treat it as loss immediately** → one dropped query, one restarting
connection pool, and healthy reactors stop for no reason. Reactive
workloads would flap on ordinary transient errors.
So indeterminacy is tolerated, but only for a bounded number of consecutive
checks. Cross the threshold and we presume loss and stop, because the longer
we cannot verify, the likelier it is that our session is already gone and
someone else owns these reactors. Fail toward stopping: a stopped reactor is
recoverable by re-claiming, while two live reactors silently double-process.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `consecutive_indeterminate` | `u32` |  |
| `max_indeterminate` | `u32` |  |

#### Methods

##### <span id="OwnershipWatchdog.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn new (max_indeterminate : u32) -> Self
```

`max_indeterminate` is how many consecutive unverifiable checks to tolerate before presuming loss. Must be >= 1; 0 is clamped to 1 so a misconfiguration cannot make every transient blip stop reactors.

<details>
<summary>Source</summary>

```rust
    pub fn new(max_indeterminate: u32) -> Self {
        Self {
            consecutive_indeterminate: 0,
            max_indeterminate: max_indeterminate.max(1),
        }
    }
```

</details>



##### <span id="OwnershipWatchdog.consecutive_indeterminate"></span>`consecutive_indeterminate`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn consecutive_indeterminate (& self) -> u32
```

<details>
<summary>Source</summary>

```rust
    pub fn consecutive_indeterminate(&self) -> u32 {
        self.consecutive_indeterminate
    }
```

</details>



##### <span id="OwnershipWatchdog.observe"></span>`observe`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn observe (& mut self , check : OwnershipCheck , believed_owned : & [ReactorId] ,) -> WatchdogAction
```

Feed one check result; get the action to take.

`believed_owned` is what we would have to stop if we presume total loss.

<details>
<summary>Source</summary>

```rust
    pub fn observe(
        &mut self,
        check: OwnershipCheck,
        believed_owned: &[ReactorId],
    ) -> WatchdogAction {
        match check {
            OwnershipCheck::AllHeld => {
                // A successful verification clears the streak — transient
                // failures must not accumulate across healthy checks.
                self.consecutive_indeterminate = 0;
                WatchdogAction::Continue
            }
            OwnershipCheck::Lost(lost) => {
                // We got a definitive answer, so the streak is irrelevant.
                self.consecutive_indeterminate = 0;
                WatchdogAction::StopReactors(lost)
            }
            OwnershipCheck::Indeterminate(_) => {
                self.consecutive_indeterminate += 1;
                if self.consecutive_indeterminate >= self.max_indeterminate {
                    WatchdogAction::StopAllPresumedLost(believed_owned.to_vec())
                } else {
                    WatchdogAction::Continue
                }
            }
        }
    }
```

</details>





### <span id="struct.PostgresOwnership"></span>`struct PostgresOwnership`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


[`ReactorOwnership`] over an [`OwnershipSession`].

The session needs `&mut self` (it mutates believed-ownership and drives one
connection), while the trait takes `&self` so the scheduler can hold it
behind an `Arc`. A `tokio::Mutex` bridges that.
Serializing ownership operations is a feature, not a cost: claim, release
and verify all read-modify-write the same believed-ownership set against the
same connection. Letting them interleave is how you get a verify that
observes a half-applied claim and "helpfully" halts a reactor that was just
acquired.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `session` | `tokio :: sync :: Mutex < OwnershipSession >` |  |
| `publication` | `Option < AddressPublication >` | Address publication (A-0012 Amendment 3). `None` when this replica has
no advertised address (reactorAffinity disabled in the chart): claims
work exactly as before and injects that miss fall back to the outbox. |

#### Methods

##### <span id="PostgresOwnership.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn new (session : OwnershipSession) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(session: OwnershipSession) -> Self {
        Self {
            session: tokio::sync::Mutex::new(session),
            publication: None,
        }
    }
```

</details>



##### <span id="PostgresOwnership.with_publication"></span>`with_publication`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn with_publication (mut self , publication : AddressPublication) -> Self
```

Enable owner-address publication for edge routing.

<details>
<summary>Source</summary>

```rust
    pub fn with_publication(mut self, publication: AddressPublication) -> Self {
        self.publication = Some(publication);
        self
    }
```

</details>



##### <span id="PostgresOwnership.publish_address"></span>`publish_address`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn publish_address (& self , id : & ReactorId)
```

Best-effort publish after a WON claim.

Failure is logged, not returned: the claim itself is what makes this
replica the owner, and refusing ownership because a HINT could not be
written would invert the authority order this design is built on. The
cost of a missing row is the outbox fallback — slow, correct.

<details>
<summary>Source</summary>

```rust
    async fn publish_address(&self, id: &ReactorId) {
        let Some(p) = self.publication.as_ref() else {
            return;
        };
        if let Err(e) = p
            .dal
            .reactor_owner_addresses()
            .publish(id.tenant.as_deref(), &id.name, &p.advertised_address)
            .await
        {
            tracing::warn!(
                reactor = %id.name,
                tenant = ?id.tenant,
                "failed to publish owner address (injects will use the outbox fallback): {e}"
            );
        }
    }
```

</details>



##### <span id="PostgresOwnership.retract_address"></span>`retract_address`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn retract_address (& self , id : & ReactorId)
```

Best-effort retract on release or detected loss. Uses `remove_if_ours` so a stale retraction can never tear down a successor's row.

<details>
<summary>Source</summary>

```rust
    async fn retract_address(&self, id: &ReactorId) {
        let Some(p) = self.publication.as_ref() else {
            return;
        };
        if let Err(e) = p
            .dal
            .reactor_owner_addresses()
            .remove_if_ours(id.tenant.as_deref(), &id.name, &p.advertised_address)
            .await
        {
            tracing::warn!(
                reactor = %id.name,
                tenant = ?id.tenant,
                "failed to retract owner address (a stale hint remains; costs a wasted redirect): {e}"
            );
        }
    }
```

</details>





### <span id="struct.AddressPublication"></span>`struct AddressPublication`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Everything needed to publish/retract this replica's owner-address rows.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `crate :: dal :: unified :: DAL` |  |
| `advertised_address` | `String` | This replica's reachable address (`CLOACINA_ADVERTISED_ADDRESS`). |



## Enums

### <span id="enum.OwnershipCheck"></span>`enum OwnershipCheck`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


What a liveness check found. Deliberately not a bool: "we lost some locks" is a different situation from "the check itself could not run", and conflating them is how a replica ends up either needlessly stopping healthy reactors or confidently running unowned ones.

#### Variants

- **`AllHeld`** - Every reactor this session believes it owns is still locked by it.
- **`Lost`** - These reactors are no longer owned. The caller MUST stop them locally
before attempting to re-claim — another replica may already be running
them.
- **`Indeterminate`** - The check could not be completed (connection down, query failed). The
caller must treat this as *unknown*, not as healthy: a session that
cannot reach Postgres cannot know whether it still holds anything.



### <span id="enum.WatchdogAction"></span>`enum WatchdogAction`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


What the watchdog decided a caller should do after one liveness check.

#### Variants

- **`Continue`** - Ownership confirmed (or nothing owned). Keep running.
- **`StopReactors`** - These reactors are provably no longer ours. Stop them locally.
- **`StopAllPresumedLost`** - We have been unable to verify for too long. Stop everything we believed
we owned and treat it as lost. See [`OwnershipWatchdog`] for why this is
the safe direction.



