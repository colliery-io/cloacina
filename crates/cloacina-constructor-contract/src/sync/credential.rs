/*
 *  Copyright 2025-2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! The credential a sync endpoint signs its requests with
//! (COLLIERY-I-0633 / CLOACINA-T-0982): the spec, as data. The host builds the
//! credential from it in `cloacina_sync::credential` (CLOACINA-T-0992 moved
//! the type here, so a definition compiler needs no engine).
//!
//! A sync definition names one credential per side in the `source_credential`
//! and `destination_credential` params: a JSON object with a `kind`, the
//! `secret` to read, and the non-secret fields of that kind. The secret is a
//! secret name or a local binding the instance mapped to one with
//! `{"$secret": "<name>"}`. The read and write tasks resolve it on the agent
//! with [`Context::secret`] and hand the [`Credential`] to the connector's
//! egress policy, which applies it to each allowed request on the host. The
//! guest never sees the secret, and a credential never widens the grants: a
//! request the grants deny is denied before the credential is applied.
//!
//! | kind | non-secret fields | secret fields |
//! |---|---|---|
//! | `header` | `name`, `field`, `prefix` (optional) | the one named by `field` |
//! | `query` | `param`, `field` | the one named by `field` |
//! | `aws_sigv4` | `region`, `service` (default `s3`) | `access_key_id`, `secret_access_key` |
//! | `snowflake_keypair_jwt` | `account`, `user` | `private_key` (PKCS#8 PEM) |
//! | `basic` | `username_field`, `password_field` (default `username`, `password`) | those two |
//! | `oauth2` | `token_url`, `grant` (`client_credentials` or `refresh_token`), `scopes` | `client_id`, `client_secret`, and `refresh_token` for that grant |
//! | `session` | `login_url`, `method` (default `POST`), `login_fields`, `token_path`, `inject_header`, `prefix`, `ttl_secs` (default 900) | the values of `login_fields` |
//! | `google_service_account` | `scopes` | `service_account_key` (the account's JSON key) |
//!
//! The `oauth2`, `session` and `google_service_account` kinds mint a token on
//! the host and keep it for the agent process (CLOACINA-T-0989), so a sync
//! does not mint one per segment. A provider that rotates its refresh token
//! fails the grant: cloacina does not store a rotated refresh token yet.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One side's credential, as the sync definition declares it. Holds no secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum CredentialSpec {
    /// A header whose value is `prefix` + the secret's `field`. A bearer token
    /// is `name = "Authorization"`, `prefix = "Bearer "`.
    #[serde(rename = "header")]
    Header {
        secret: String,
        name: String,
        field: String,
        #[serde(default)]
        prefix: String,
    },
    /// A query parameter whose value is the secret's `field`.
    #[serde(rename = "query")]
    Query {
        secret: String,
        param: String,
        field: String,
    },
    /// AWS SigV4 request signing (S3 and S3-compatible stores).
    #[serde(rename = "aws_sigv4")]
    AwsSigV4 {
        secret: String,
        region: String,
        #[serde(default = "default_service")]
        service: String,
    },
    /// A Snowflake key-pair JWT, signed on the host and re-signed before expiry.
    #[serde(rename = "snowflake_keypair_jwt")]
    SnowflakeKeypairJwt {
        secret: String,
        account: String,
        user: String,
    },
    /// HTTP Basic from two of the secret's fields.
    #[serde(rename = "basic")]
    Basic {
        secret: String,
        #[serde(default = "default_username")]
        username_field: String,
        #[serde(default = "default_password")]
        password_field: String,
    },
    /// OAuth2: the host runs the grant at `token_url` and adds the bearer.
    #[serde(rename = "oauth2")]
    OAuth2 {
        secret: String,
        token_url: String,
        grant: GrantKind,
        #[serde(default)]
        scopes: Vec<String>,
    },
    /// A session token: the host logs in at `login_url` with a JSON body built
    /// from the secret (`login_fields`: body key to secret field), reads the
    /// token at the dot-path `token_path`, and sends `prefix` + token as
    /// `inject_header`.
    #[serde(rename = "session")]
    Session {
        secret: String,
        login_url: String,
        #[serde(default = "default_post")]
        method: String,
        #[serde(default)]
        login_fields: BTreeMap<String, String>,
        token_path: String,
        #[serde(default = "default_authorization")]
        inject_header: String,
        #[serde(default)]
        prefix: String,
        #[serde(default = "default_session_ttl")]
        ttl_secs: u64,
    },
    /// A Google service account, from the secret's `service_account_key`.
    #[serde(rename = "google_service_account")]
    GoogleServiceAccount {
        secret: String,
        #[serde(default)]
        scopes: Vec<String>,
    },
}

/// The OAuth2 grant of an `oauth2` credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantKind {
    ClientCredentials,
    RefreshToken,
}

fn default_service() -> String {
    "s3".to_string()
}
fn default_username() -> String {
    "username".to_string()
}
fn default_password() -> String {
    "password".to_string()
}
fn default_post() -> String {
    "POST".to_string()
}
fn default_authorization() -> String {
    "Authorization".to_string()
}
fn default_session_ttl() -> u64 {
    900
}

/// The SigV4 secret's fields.
pub const AWS_ACCESS_KEY_ID: &str = "access_key_id";
pub const AWS_SECRET_ACCESS_KEY: &str = "secret_access_key";
/// The Snowflake key-pair secret's field.
pub const SNOWFLAKE_PRIVATE_KEY: &str = "private_key";
/// The OAuth2 secret's fields.
pub const OAUTH2_CLIENT_ID: &str = "client_id";
pub const OAUTH2_CLIENT_SECRET: &str = "client_secret";
pub const OAUTH2_REFRESH_TOKEN: &str = "refresh_token";
/// The Google service account secret's field: the account's JSON key.
pub const GOOGLE_SERVICE_ACCOUNT_KEY: &str = "service_account_key";

impl CredentialSpec {
    /// The secret name or local binding this credential reads.
    pub fn secret(&self) -> &str {
        match self {
            CredentialSpec::Header { secret, .. }
            | CredentialSpec::Query { secret, .. }
            | CredentialSpec::AwsSigV4 { secret, .. }
            | CredentialSpec::SnowflakeKeypairJwt { secret, .. }
            | CredentialSpec::Basic { secret, .. }
            | CredentialSpec::OAuth2 { secret, .. }
            | CredentialSpec::Session { secret, .. }
            | CredentialSpec::GoogleServiceAccount { secret, .. } => secret,
        }
    }
}
