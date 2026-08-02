# cloacina::dal::unified::oidc_login_flows <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


OIDC in-flight login state (CLOACI-T-0801). **Postgres only.**

Persists the authorization-code flow's `state -> (nonce, pkce_verifier)` so
the callback can land on any replica (NFR-003, no sticky sessions). Single
use: [`take`](OidcLoginFlowDAL::take) deletes-and-returns in one statement,
and only for an unexpired row, so a replayed/expired/unknown `state` yields
`None` (the callback then fails closed).

## Structs

### `cloacina::dal::unified::oidc_login_flows::NewLoginFlow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


**Derives:** `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `state` | `String` |  |
| `nonce` | `String` |  |
| `pkce_verifier` | `String` |  |
| `expires_at` | `chrono :: NaiveDateTime` |  |



### `cloacina::dal::unified::oidc_login_flows::OidcLoginFlowDAL`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


DAL for the OIDC login-flow state. Postgres only.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |
