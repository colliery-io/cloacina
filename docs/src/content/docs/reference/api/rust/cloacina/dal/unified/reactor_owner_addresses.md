---
title: "cloacina::dal::unified::reactor_owner_addresses"
description: "DAL for reactor owner addresses (CLOACI-T-0851 / A-0012 Amendment 3)."
editUrl: false
sidebar:
  order: 80
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

DAL for reactor owner addresses (CLOACI-T-0851 / A-0012 Amendment 3).

**A routing hint, not ownership.** The advisory lock is the sole authority
on who owns a reactor; rows here only say where the last successful claimant
said it could be reached. Reads answer "where should I redirect this
inject", and a stale answer costs one wasted hop — the fallback (redirect
again, or the delivery outbox) covers it. No reader may treat presence or
absence of a row as evidence about ownership.

## Structs

### <span id="struct.ReactorOwnerAddressesDAL"></span><code style="font-size: 0.85em">struct ReactorOwnerAddressesDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

DAL accessor for the `reactor_owner_addresses` routing-hint table.

#### Methods

##### <span id="ReactorOwnerAddressesDAL.new"></span><code>new</code>

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

##### <span id="ReactorOwnerAddressesDAL.publish"></span><code>publish</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn publish(
    &self,
    tenant_id: Option<&str>,
    reactor_name: &str,
    address: &str,
) -> Result<(), ValidationError>
```

Publish (or overwrite) the owner address for a reactor.

Delete-then-insert rather than upsert: the unique index is an
EXPRESSION index (`COALESCE(tenant_id,'')`), which `ON CONFLICT` cannot
target portably across postgres and sqlite. The two statements are not
atomic, but this table is a hint — the worst interleaving leaves either
the old address (a wasted redirect) or none (outbox fallback), both of
which the routing design already absorbs.

<details>
<summary>Source</summary>

```rust
    pub async fn publish(
        &self,
        tenant_id: Option<&str>,
        reactor_name: &str,
        address: &str,
    ) -> Result<(), ValidationError> {
        let tenant_owned = tenant_id.map(|t| t.to_string());
        let row = NewUnifiedReactorOwnerAddress {
            tenant_id: tenant_owned.clone(),
            reactor_name: reactor_name.to_string(),
            address: address.to_string(),
            claimed_at: UniversalTimestamp::now(),
        };
        let name_owned = reactor_name.to_string();
        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction(|conn| {
                match &tenant_owned {
                    Some(t) => diesel::delete(
                        reactor_owner_addresses::table
                            .filter(reactor_owner_addresses::tenant_id.eq(t))
                            .filter(reactor_owner_addresses::reactor_name.eq(&name_owned)),
                    )
                    .execute(conn)?,
                    None => diesel::delete(
                        reactor_owner_addresses::table
                            .filter(reactor_owner_addresses::tenant_id.is_null())
                            .filter(reactor_owner_addresses::reactor_name.eq(&name_owned)),
                    )
                    .execute(conn)?,
                };
                diesel::insert_into(reactor_owner_addresses::table)
                    .values(&row)
                    .execute(conn)
            })
        })?;
        Ok(())
    }
```

</details>

##### <span id="ReactorOwnerAddressesDAL.remove_if_ours"></span><code>remove_​if_​ours</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn remove_if_ours(
    &self,
    tenant_id: Option<&str>,
    reactor_name: &str,
    our_address: &str,
) -> Result<(), ValidationError>
```

Remove the address row for a reactor this replica is releasing.

Callers pass the address they published so a NEW owner's row is not
torn down by a stale releaser: release-after-takeover would otherwise
delete the successor's perfectly valid address. Deleting nothing is
success — the row may already have been overwritten, which is the
normal takeover sequence.

<details>
<summary>Source</summary>

```rust
    pub async fn remove_if_ours(
        &self,
        tenant_id: Option<&str>,
        reactor_name: &str,
        our_address: &str,
    ) -> Result<(), ValidationError> {
        let tenant_owned = tenant_id.map(|t| t.to_string());
        let name_owned = reactor_name.to_string();
        let addr_owned = our_address.to_string();
        crate::interact_on_backend!(self.dal, |conn| {
            match &tenant_owned {
                Some(t) => diesel::delete(
                    reactor_owner_addresses::table
                        .filter(reactor_owner_addresses::tenant_id.eq(t))
                        .filter(reactor_owner_addresses::reactor_name.eq(&name_owned))
                        .filter(reactor_owner_addresses::address.eq(&addr_owned)),
                )
                .execute(conn),
                None => diesel::delete(
                    reactor_owner_addresses::table
                        .filter(reactor_owner_addresses::tenant_id.is_null())
                        .filter(reactor_owner_addresses::reactor_name.eq(&name_owned))
                        .filter(reactor_owner_addresses::address.eq(&addr_owned)),
                )
                .execute(conn),
            }
        })?;
        Ok(())
    }
```

</details>

##### <span id="ReactorOwnerAddressesDAL.lookup"></span><code>lookup</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn lookup(
    &self,
    tenant_id: Option<&str>,
    reactor_name: &str,
) -> Result<Option<String>, ValidationError>
```

Look up the advertised address for a reactor. `None` means "no hint" — callers fall back to the outbox; it does NOT mean the reactor is unowned.

<details>
<summary>Source</summary>

```rust
    pub async fn lookup(
        &self,
        tenant_id: Option<&str>,
        reactor_name: &str,
    ) -> Result<Option<String>, ValidationError> {
        let tenant_owned = tenant_id.map(|t| t.to_string());
        let name_owned = reactor_name.to_string();
        let row: Option<UnifiedReactorOwnerAddress> =
            crate::interact_on_backend!(self.dal, |conn| {
                match &tenant_owned {
                    Some(t) => reactor_owner_addresses::table
                        .filter(reactor_owner_addresses::tenant_id.eq(t))
                        .filter(reactor_owner_addresses::reactor_name.eq(&name_owned))
                        .first(conn)
                        .optional(),
                    None => reactor_owner_addresses::table
                        .filter(reactor_owner_addresses::tenant_id.is_null())
                        .filter(reactor_owner_addresses::reactor_name.eq(&name_owned))
                        .first(conn)
                        .optional(),
                }
            })?;
        Ok(row.map(|r| r.address))
    }
```

</details>
