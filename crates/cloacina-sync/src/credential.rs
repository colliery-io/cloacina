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

use std::collections::BTreeMap;

use cloacina::Context;
use cloacina::registry::loader::credentials::{Credential, SnowflakeKeypairProvider};
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
}

fn default_service() -> String {
    "s3".to_string()
}

/// The SigV4 secret's fields.
pub const AWS_ACCESS_KEY_ID: &str = "access_key_id";
pub const AWS_SECRET_ACCESS_KEY: &str = "secret_access_key";
/// The Snowflake key-pair secret's field.
pub const SNOWFLAKE_PRIVATE_KEY: &str = "private_key";

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
}

impl CredentialSpec {
    /// The secret name or local binding this credential reads.
    pub fn secret(&self) -> &str {
        match self {
            CredentialSpec::Header { secret, .. }
            | CredentialSpec::Query { secret, .. }
            | CredentialSpec::AwsSigV4 { secret, .. }
            | CredentialSpec::SnowflakeKeypairJwt { secret, .. } => secret,
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
