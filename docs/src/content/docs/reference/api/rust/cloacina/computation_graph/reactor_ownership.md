---
title: "cloacina::computation_graph::reactor_ownership"
description: "Per-reactor ownership via a single long-lived advisory-lock session (CLOACI-T-0851, [`ADR CLOACI-A-0012`])."
editUrl: false
sidebar:
  order: 47
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Per-reactor ownership via a single long-lived advisory-lock session (CLOACI-T-0851, [`ADR CLOACI-A-0012`]).

**The shape, and why it differs from the fleet control loop**

`cloacina-server::autoscaler::leader` takes an advisory lock, does one
tick's work, and unlocks — all on one pooled connection held briefly. Reactor
ownership is not tick-shaped: a replica owns a reactor for as long as it runs
it. Copying the fleet pattern per reactor would pin one pooled connection per
owned reactor for the process lifetime and exhaust the pool as reactor count
grows.

Instead this holds **one dedicated connection per replica** — the *ownership
session* — which carries ALL of that replica's reactor locks. A Postgres
session can hold many advisory locks, so the connection cost is O(1) in
reactor count. If the replica dies, the session ends and Postgres releases
every one of its reactor locks at once, which is exactly the failover
behaviour we want.

**Operator note:** this permanently reserves one connection per replica from
the pool. Size the pool with that in mind (maintainer decision 2026-08-16:
take one and document it, rather than raising the default).

**Why there is a liveness check even though locks are session-scoped**

A-0012 records that session-scoped locks need "no lease/heartbeat
bookkeeping" because a crashed replica's locks auto-release. That is true for
*failover*, and it is NOT sufficient here — long-held ownership has a failure
mode the fleet loop cannot have:

> The ownership connection drops (network blip, PgBouncer recycle, database
> restart) while the replica keeps running. Postgres releases every lock the
> session held. Another replica legitimately claims the reactor. **The
> original replica never notices and keeps running it too.** Two replicas,
> one reactor, no error raised anywhere.

The fleet loop is immune because it re-acquires every tick, so a dropped
connection simply means it stops being leader. Ownership that is *assumed*
rather than *re-established* has to be re-verified, or the split-brain this
whole mechanism exists to prevent comes back through the side door.

So `OwnershipSession::verify_owned` re-asserts the locks this replica
believes it holds. This is loss DETECTION, not lease renewal: there is no
TTL, no clock assumption, and no bookkeeping row. Postgres stays the single
source of truth; we are only asking it whether what we believe is still true.
Callers must stop the affected reactors on loss before attempting re-claim —
see `OwnershipSession::verify_owned`'s contract.

## Re-exports

- `OwnershipSession` (`session::OwnershipSession`)

## Structs

### <span id="struct.ReactorId"></span><code style="font-size: 0.85em">struct ReactorId</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`

Identifies a reactor for ownership purposes: the tenant scope it was loaded under (`None` for single-tenant / admin-owned) plus its name.

This mirrors how `ComputationGraphScheduler` keys its `reactors` map, so an
ownership claim and the in-process reactor it guards cannot drift apart.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tenant` | `Option<String>` |  |
| `name` | `String` |  |

#### Methods

##### <span id="ReactorId.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(
    tenant: Option<impl Into<String>>,
    name: impl Into<String>,
) -> Self
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

##### <span id="ReactorId.lock_key"></span><code>lock_​key</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn lock_key(&self) -> i64
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

### <span id="struct.OwnershipState"></span><code style="font-size: 0.85em">struct OwnershipState</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Default`

Tracks which reactors this replica believes it owns.

The database is the source of truth; this set is only what we *believe*, and
[`OwnershipCheck`](../../../../rust/cloacina/computation_graph/reactor_ownership/#enum.OwnershipCheck) exists precisely because belief and truth can diverge.

#### Methods

##### <span id="OwnershipState.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new() -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Self {
        Self::default()
    }
```

</details>

##### <span id="OwnershipState.record_claimed"></span><code>record_​claimed</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn record_claimed(&mut self, id: ReactorId)
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

##### <span id="OwnershipState.record_released"></span><code>record_​released</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn record_released(&mut self, id: &ReactorId)
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

##### <span id="OwnershipState.believes_owned"></span><code>believes_​owned</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn believes_owned(&self, id: &ReactorId) -> bool
```

<details>
<summary>Source</summary>

```rust
    pub fn believes_owned(&self, id: &ReactorId) -> bool {
        self.held.contains(id)
    }
```

</details>

##### <span id="OwnershipState.owned"></span><code>owned</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn owned(&self) -> impl Iterator<Item = &ReactorId>
```

<details>
<summary>Source</summary>

```rust
    pub fn owned(&self) -> impl Iterator<Item = &ReactorId> {
        self.held.iter()
    }
```

</details>

##### <span id="OwnershipState.len"></span><code>len</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn len(&self) -> usize
```

<details>
<summary>Source</summary>

```rust
    pub fn len(&self) -> usize {
        self.held.len()
    }
```

</details>

##### <span id="OwnershipState.is_empty"></span><code>is_​empty</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_empty(&self) -> bool
```

<details>
<summary>Source</summary>

```rust
    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }
```

</details>

##### <span id="OwnershipState.diff_against_held_keys"></span><code>diff_​against_​held_​keys</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn diff_against_held_keys(
    &self,
    held_keys: &HashSet<i64>,
) -> Vec<ReactorId>
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

### <span id="struct.OwnershipWatchdog"></span><code style="font-size: 0.85em">struct OwnershipWatchdog</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`

Turns a sequence of [`OwnershipCheck`](../../../../rust/cloacina/computation_graph/reactor_ownership/#enum.OwnershipCheck)s into decisions.

The interesting case is [`OwnershipCheck::Indeterminate`](../../../../rust/cloacina/computation_graph/reactor_ownership/#enum.OwnershipCheck), which the ADR
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

#### Methods

##### <span id="OwnershipWatchdog.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(max_indeterminate: u32) -> Self
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

##### <span id="OwnershipWatchdog.consecutive_indeterminate"></span><code>consecutive_​indeterminate</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn consecutive_indeterminate(&self) -> u32
```

<details>
<summary>Source</summary>

```rust
    pub fn consecutive_indeterminate(&self) -> u32 {
        self.consecutive_indeterminate
    }
```

</details>

##### <span id="OwnershipWatchdog.observe"></span><code>observe</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn observe(
    &mut self,
    check: OwnershipCheck,
    believed_owned: &[ReactorId],
) -> WatchdogAction
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

### <span id="struct.PostgresOwnership"></span><code style="font-size: 0.85em">struct PostgresOwnership</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

[`ReactorOwnership`](../../../../rust/cloacina/computation_graph/reactor_ownership/#trait.ReactorOwnership) over an `OwnershipSession`.

The session needs `&mut self` (it mutates believed-ownership and drives one
connection), while the trait takes `&self` so the scheduler can hold it
behind an `Arc`. A `tokio::Mutex` bridges that.

Serializing ownership operations is a feature, not a cost: claim, release
and verify all read-modify-write the same believed-ownership set against the
same connection. Letting them interleave is how you get a verify that
observes a half-applied claim and "helpfully" halts a reactor that was just
acquired.

#### Methods

##### <span id="PostgresOwnership.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(session: OwnershipSession) -> Self
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

##### <span id="PostgresOwnership.with_publication"></span><code>with_​publication</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_publication(mut self, publication: AddressPublication) -> Self
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

### <span id="struct.AddressPublication"></span><code style="font-size: 0.85em">struct AddressPublication</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Everything needed to publish/retract this replica's owner-address rows.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `crate::dal::unified::DAL` |  |
| `advertised_address` | `String` | This replica's reachable address (`CLOACINA_ADVERTISED_ADDRESS`). |

## Enums

### <span id="enum.OwnershipCheck"></span><code style="font-size: 0.85em">enum OwnershipCheck</code>

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

### <span id="enum.WatchdogAction"></span><code style="font-size: 0.85em">enum WatchdogAction</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

What the watchdog decided a caller should do after one liveness check.

#### Variants

- **`Continue`** - Ownership confirmed (or nothing owned). Keep running.
- **`StopReactors`** - These reactors are provably no longer ours. Stop them locally.
- **`StopAllPresumedLost`** - We have been unable to verify for too long. Stop everything we believed
  we owned and treat it as lost. See [`OwnershipWatchdog`](../../../../rust/cloacina/computation_graph/reactor_ownership/#struct.OwnershipWatchdog) for why this is
  the safe direction.

## Traits

### <span id="trait.ReactorOwnership"></span><code style="font-size: 0.85em">trait ReactorOwnership</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync`

The coordination surface the scheduler needs, independent of Postgres.

The scheduler holds an `Option<Arc<dyn ReactorOwnership>>`. **`None` is the
embedded / single-replica / sqlite path and MUST behave exactly as before
this feature existed** — no claim, no verification, no behaviour change.
That is not merely a convenience: A-0012 requires single-replica and
embedded behaviour to stay byte-for-byte identical, and the only way to be
sure of that is for those deployments to execute none of this code.

A trait rather than a concrete `OwnershipSession` so the scheduler does not
become postgres-gated, and so the claim/loss paths are testable with a fake
instead of a live database. Coordination bugs are exactly the ones you
cannot reproduce on demand against a real cluster.

#### Methods

##### <span id="ReactorOwnership.claim"></span><code>claim</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn claim(&self, id: &ReactorId) -> Result<bool, String>
```

Try to take ownership. `Ok(false)` = another replica owns it, which is an ordinary outcome and not an error.

<details>
<summary>Source</summary>

```rust
    async fn claim(&self, id: &ReactorId) -> Result<bool, String>;
```

</details>

##### <span id="ReactorOwnership.release"></span><code>release</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn release(&self, id: &ReactorId) -> Result<(), String>
```

Give up ownership (normal unload).

<details>
<summary>Source</summary>

```rust
    async fn release(&self, id: &ReactorId) -> Result<(), String>;
```

</details>

##### <span id="ReactorOwnership.verify"></span><code>verify</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn verify(&self) -> OwnershipCheck
```

Re-assert everything this replica believes it owns.

<details>
<summary>Source</summary>

```rust
    async fn verify(&self) -> OwnershipCheck;
```

</details>

##### <span id="ReactorOwnership.believed_owned"></span><code>believed_​owned</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn believed_owned(&self) -> Vec<ReactorId>
```

What this replica currently believes it owns.

<details>
<summary>Source</summary>

```rust
    async fn believed_owned(&self) -> Vec<ReactorId>;
```

</details>

## Constants

| Name | Type | Value | Description |
|------|------|-------|-------------|
| <span id="const.SESSION_HELD_LOCKS_SQL"></span>`SESSION_HELD_LOCKS_SQL` | `&str` | `"SELECT (classid::bigint <<32) \| objid::bigint AS key \ F...` | SQL asking Postgres which advisory locks the CURRENT session holds.<br>`pg_locks.pid = pg_backend_pid()` restricts to this session, which is the whole point: another replica holding the same lock must NOT read as "we still own it". The 64-bit key is reassembled from the split `classid`/`objid` columns (see `reactor_lock_key` and the k8s-leader lane's `advisory_lock_parts` for the same split on the test side). |
