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
//! (COLLIERY-I-0633 / CLOACINA-T-0982).
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

use cloacina::Context;
use cloacina::registry::loader::credentials::{
    Credential, GoogleServiceAccountProvider, OAuth2Provider, OAuthGrant, SessionProvider,
    SnowflakeKeypairProvider,
};
use cloacina_workflow::SecretAccessError;
use serde::Deserialize;
use serde_json::Value;

/// One side's credential, as the sync definition declares it. Holds no secret.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
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

/// A credential could not be built. Names secrets and fields, never values.
#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    #[error("{side} credential: secret `{secret}`: {error}")]
    Secret {
        side: &'static str,
        secret: String,
        error: SecretAccessError,
    },
    #[error("{side} credential: secret `{secret}` has no field `{field}`")]
    MissingField {
        side: &'static str,
        secret: String,
        field: String,
    },
    #[error("{side} credential: secret `{secret}`: {problem}")]
    Invalid {
        side: &'static str,
        secret: String,
        problem: String,
    },
}

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

    /// Build the credential from the secret's resolved fields.
    pub fn build(
        &self,
        side: &'static str,
        fields: &BTreeMap<String, String>,
    ) -> Result<Credential, CredentialError> {
        let get = |field: &str| {
            fields
                .get(field)
                .cloned()
                .ok_or_else(|| CredentialError::MissingField {
                    side,
                    secret: self.secret().to_string(),
                    field: field.to_string(),
                })
        };
        Ok(match self {
            CredentialSpec::Header {
                name,
                field,
                prefix,
                ..
            } => Credential::Header {
                name: name.clone(),
                value: format!("{prefix}{}", get(field)?),
            },
            CredentialSpec::Query { param, field, .. } => Credential::Query {
                param: param.clone(),
                value: get(field)?,
            },
            CredentialSpec::AwsSigV4 {
                region, service, ..
            } => Credential::AwsSigV4 {
                access_key: get(AWS_ACCESS_KEY_ID)?,
                secret_key: get(AWS_SECRET_ACCESS_KEY)?,
                region: region.clone(),
                service: service.clone(),
            },
            CredentialSpec::SnowflakeKeypairJwt { account, user, .. } => {
                Credential::SnowflakeKeypairJwt(SnowflakeKeypairProvider::new(
                    account.clone(),
                    user.clone(),
                    get(SNOWFLAKE_PRIVATE_KEY)?,
                ))
            }
            CredentialSpec::Basic {
                username_field,
                password_field,
                ..
            } => Credential::Basic {
                username: get(username_field)?,
                password: get(password_field)?,
            },
            CredentialSpec::OAuth2 {
                token_url,
                grant,
                scopes,
                ..
            } => Credential::OAuth2(OAuth2Provider::new(
                token_url.clone(),
                match grant {
                    GrantKind::ClientCredentials => OAuthGrant::ClientCredentials,
                    GrantKind::RefreshToken => OAuthGrant::RefreshToken {
                        refresh_token: get(OAUTH2_REFRESH_TOKEN)?,
                    },
                },
                get(OAUTH2_CLIENT_ID)?,
                get(OAUTH2_CLIENT_SECRET)?,
                scopes.clone(),
            )),
            CredentialSpec::Session {
                login_url,
                method,
                login_fields,
                token_path,
                inject_header,
                prefix,
                ttl_secs,
                ..
            } => {
                let body = if login_fields.is_empty() {
                    None
                } else {
                    let mut obj = serde_json::Map::new();
                    for (key, field) in login_fields {
                        obj.insert(key.clone(), Value::String(get(field)?));
                    }
                    Some(Value::Object(obj).to_string())
                };
                Credential::Session(SessionProvider::new(
                    login_url.clone(),
                    method.clone(),
                    body,
                    token_path.split('.').map(str::to_string).collect(),
                    inject_header.clone(),
                    prefix.clone(),
                    std::time::Duration::from_secs(*ttl_secs),
                ))
            }
            CredentialSpec::GoogleServiceAccount { scopes, .. } => {
                Credential::GoogleServiceAccount(
                    GoogleServiceAccountProvider::from_key_json(
                        &get(GOOGLE_SERVICE_ACCOUNT_KEY)?,
                        scopes.clone(),
                    )
                    .map_err(|problem| CredentialError::Invalid {
                        side,
                        secret: self.secret().to_string(),
                        problem,
                    })?,
                )
            }
        })
    }
}

/// Resolve one side's credential on the agent: read the secret through the
/// execution's resolver and build the credential. `None` when the side names
/// none.
pub async fn resolve(
    ctx: &Context<Value>,
    side: &'static str,
    spec: Option<&CredentialSpec>,
) -> Result<Option<Credential>, CredentialError> {
    let Some(spec) = spec else {
        return Ok(None);
    };
    let fields = ctx
        .secret(spec.secret())
        .await
        .map_err(|error| CredentialError::Secret {
            side,
            secret: spec.secret().to_string(),
            error,
        })?;
    spec.build(side, &fields).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloacina_workflow::secret::MapSecretResolver;
    use std::sync::Arc;

    fn spec(v: Value) -> CredentialSpec {
        serde_json::from_value(v).expect("spec parses")
    }

    #[test]
    fn specs_parse_by_kind() {
        assert_eq!(
            spec(serde_json::json!({
                "kind": "header", "secret": "api", "name": "Authorization",
                "field": "token", "prefix": "Bearer "
            })),
            CredentialSpec::Header {
                secret: "api".into(),
                name: "Authorization".into(),
                field: "token".into(),
                prefix: "Bearer ".into(),
            }
        );
        assert_eq!(
            spec(serde_json::json!({"kind": "aws_sigv4", "secret": "aws", "region": "us-east-1"})),
            CredentialSpec::AwsSigV4 {
                secret: "aws".into(),
                region: "us-east-1".into(),
                service: "s3".into(),
            }
        );
        assert!(matches!(
            spec(serde_json::json!({
                "kind": "snowflake_keypair_jwt", "secret": "sf", "account": "a", "user": "u"
            })),
            CredentialSpec::SnowflakeKeypairJwt { .. }
        ));
    }

    fn fields(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn the_minted_kinds_parse_and_build() {
        let oauth = spec(serde_json::json!({
            "kind": "oauth2", "secret": "sf", "token_url": "https://login.example.com/token",
            "grant": "refresh_token", "scopes": ["api"]
        }));
        assert_eq!(oauth.secret(), "sf");
        let built = oauth
            .build(
                "destination",
                &fields(&[
                    ("client_id", "c"),
                    ("client_secret", "s"),
                    ("refresh_token", "r"),
                ]),
            )
            .unwrap();
        assert!(format!("{built:?}").starts_with("OAuth2("));
        // The refresh grant needs its token; client credentials does not.
        let err = oauth
            .build(
                "destination",
                &fields(&[("client_id", "c"), ("client_secret", "s")]),
            )
            .unwrap_err();
        assert!(err.to_string().contains("refresh_token"), "{err}");
        let cc = spec(serde_json::json!({
            "kind": "oauth2", "secret": "x", "token_url": "https://t", "grant": "client_credentials"
        }));
        assert!(
            cc.build(
                "source",
                &fields(&[("client_id", "c"), ("client_secret", "s")])
            )
            .is_ok()
        );

        let basic = spec(serde_json::json!({"kind": "basic", "secret": "b"}));
        let built = basic
            .build(
                "source",
                &fields(&[("username", "robin"), ("password", "pw")]),
            )
            .unwrap();
        assert_eq!(format!("{built:?}"), "Basic(robin)");

        let session = spec(serde_json::json!({
            "kind": "session", "secret": "login", "login_url": "https://x/login",
            "login_fields": {"user": "username", "pass": "password"},
            "token_path": "data.token", "inject_header": "X-Session"
        }));
        let built = session
            .build(
                "source",
                &fields(&[("username", "robin"), ("password", "pw")]),
            )
            .unwrap();
        assert!(format!("{built:?}").starts_with("Session("));
        assert!(!format!("{built:?}").contains("pw"));
        assert!(
            session
                .build("source", &fields(&[("username", "robin")]))
                .is_err()
        );

        let google = spec(serde_json::json!({
            "kind": "google_service_account", "secret": "ga", "scopes": ["https://scope"]
        }));
        let err = google
            .build(
                "source",
                &fields(&[("service_account_key", "not json at all")]),
            )
            .unwrap_err();
        assert!(matches!(err, CredentialError::Invalid { .. }), "{err}");
        assert!(!err.to_string().contains("not json at all"), "{err}");
        // Unknown fields stay refused on the new kinds too.
        assert!(
            serde_json::from_value::<CredentialSpec>(serde_json::json!({
                "kind": "oauth2", "secret": "x", "token_url": "t", "grant": "client_credentials",
                "client_secret": "inline"
            }))
            .is_err()
        );
    }

    #[test]
    fn a_secret_value_inline_is_rejected() {
        // A value pasted into the spec would land in the durable context.
        let r = serde_json::from_value::<CredentialSpec>(serde_json::json!({
            "kind": "query", "secret": "s", "param": "key", "field": "key", "value": "inline"
        }));
        assert!(r.is_err());
        let r = serde_json::from_value::<CredentialSpec>(serde_json::json!({
            "kind": "oauth2", "secret": "s"
        }));
        assert!(r.is_err());
    }

    fn ctx_with(secrets: &[(&str, &[(&str, &str)])]) -> Context<Value> {
        let map = secrets
            .iter()
            .map(|(n, f)| {
                (
                    n.to_string(),
                    f.iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                )
            })
            .collect();
        Context::new().with_secret_resolver(Arc::new(MapSecretResolver::new(map)))
    }

    #[tokio::test]
    async fn resolve_builds_from_the_secret_through_a_binding() {
        let mut ctx = ctx_with(&[("prod-api", &[("token", "s3cr3t-token")])]);
        // The instance mapped the local binding `source_secret` to `prod-api`.
        ctx.insert(
            cloacina_workflow::secret::SECRET_REFS_KEY,
            serde_json::json!({"source_secret": "prod-api"}),
        )
        .unwrap();
        let s = spec(serde_json::json!({
            "kind": "header", "secret": "source_secret", "name": "Authorization",
            "field": "token", "prefix": "Bearer "
        }));
        let cred = resolve(&ctx, "source", Some(&s)).await.unwrap().unwrap();
        match cred {
            Credential::Header { name, value } => {
                assert_eq!(name, "Authorization");
                assert_eq!(value, "Bearer s3cr3t-token");
            }
            other => panic!("expected a header credential, got {other:?}"),
        }
        assert!(resolve(&ctx, "source", None).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn errors_name_the_field_not_the_value() {
        let ctx = ctx_with(&[("aws", &[("access_key_id", "AKIDEXAMPLE")])]);
        let s = spec(serde_json::json!({"kind": "aws_sigv4", "secret": "aws", "region": "r"}));
        let err = resolve(&ctx, "destination", Some(&s)).await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("secret_access_key"), "{msg}");
        assert!(!msg.contains("AKIDEXAMPLE"), "{msg}");

        let s = spec(serde_json::json!({
            "kind": "query", "secret": "absent", "param": "k", "field": "k"
        }));
        let err = resolve(&ctx, "source", Some(&s)).await.unwrap_err();
        assert!(matches!(err, CredentialError::Secret { .. }), "{err}");
    }
}
