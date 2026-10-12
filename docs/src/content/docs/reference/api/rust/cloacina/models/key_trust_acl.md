---
title: "cloacina::models::key_trust_acl"
description: "Domain models for key trust ACLs."
sidebar:
  order: 154
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Domain models for key trust ACLs.

Key trust ACLs define explicit trust relationships between organizations.
When a parent org grants trust to a child org, the parent implicitly
trusts packages signed by the child org's trusted keys.

## Structs

### <span id="struct.KeyTrustAcl"></span>`struct KeyTrustAcl`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Domain model for a key trust ACL (Access Control List).

Represents an explicit trust relationship where `parent_org_id` trusts
packages signed by keys trusted by `child_org_id`.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `parent_org_id` | `UniversalUuid` | The organization granting trust |
| `child_org_id` | `UniversalUuid` | The organization being trusted |
| `granted_at` | `UniversalTimestamp` |  |
| `revoked_at` | `Option < UniversalTimestamp >` | None if active, Some if revoked |

#### Methods

##### <span id="KeyTrustAcl.is_active"></span>`is_active`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn is_active (& self) -> bool
```

Check if this trust relationship is currently active

<details>
<summary>Source</summary>

```rust
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none()
    }
```

</details>



##### <span id="KeyTrustAcl.is_revoked"></span>`is_revoked`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn is_revoked (& self) -> bool
```

Check if this trust relationship has been revoked

<details>
<summary>Source</summary>

```rust
    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }
```

</details>





### <span id="struct.NewKeyTrustAcl"></span>`struct NewKeyTrustAcl`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Model for creating a new key trust ACL.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `parent_org_id` | `UniversalUuid` |  |
| `child_org_id` | `UniversalUuid` |  |

#### Methods

##### <span id="NewKeyTrustAcl.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn new (parent_org_id : UniversalUuid , child_org_id : UniversalUuid) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(parent_org_id: UniversalUuid, child_org_id: UniversalUuid) -> Self {
        Self {
            parent_org_id,
            child_org_id,
        }
    }
```

</details>





