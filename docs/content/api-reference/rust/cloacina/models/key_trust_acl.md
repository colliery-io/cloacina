# cloacina::models::key_trust_acl <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Domain models for key trust ACLs.

Key trust ACLs define explicit trust relationships between organizations.
When a parent org grants trust to a child org, the parent implicitly
trusts packages signed by the child org's trusted keys.

## Structs

### `cloacina::models::key_trust_acl::KeyTrustAcl`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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

##### `is_active` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



##### `is_revoked` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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





### `cloacina::models::key_trust_acl::NewKeyTrustAcl`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Model for creating a new key trust ACL.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `parent_org_id` | `UniversalUuid` |  |
| `child_org_id` | `UniversalUuid` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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
