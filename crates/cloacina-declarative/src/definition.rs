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

//! Connector definitions and their compiler (COLLIERY-I-0634 /
//! CLOACINA-T-0988).
//!
//! A **declarative connector** is a YAML **connector definition** run on one
//! shared REST runtime: the `rest` source guest, or the `rest-dest`
//! destination guest. The model is weir's declarative manifest, moved; the
//! word "manifest" stays with cloacina's packages.
//!
//! [`compile_source`] turns a definition, one of its streams and the user's
//! config into what the guest reads ([`RestConfig`]), the stream's mapping and
//! typed schema, and the credential the host adds to its requests. It replaces
//! weir's chain of string-keyed transforms (COLLIERY-I-0604): the guest config
//! is one serde type, the mapping travels typed, and the auth vocabulary is
//! [`CredentialSpec`]'s.
//!
//! Compiling renders `{{ config['key'] }}` against the user's config, so the
//! guest gets concrete values and a key the config lacks fails here, by name.
//! `max_pages` and `max_retries` in the user's config override the runtime's
//! defaults, and `base_url` replaces the definition's (a self-hosted API, a
//! proxy, a test mock); every other key is a template value.

use cloacina_constructor_contract::sync::rest::{
    HttpMethod, Incremental, InjectInto, Pagination, PartitionRouter, RestConfig, RestDestConfig,
};
use cloacina_constructor_contract::sync::{
    Field as SchemaField, FieldType, MappingSpec, StreamSchema,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use cloacina_constructor_contract::sync::credential::{CredentialSpec, GrantKind};

/// A definition does not parse, or does not hold together.
#[derive(Debug, thiserror::Error)]
pub enum DefinitionError {
    #[error("connector definition does not parse: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("connector definition `{name}`: {problem}")]
    Invalid { name: String, problem: String },
}

/// A definition cannot be compiled with this config.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CompileError {
    #[error("connector definition `{definition}` has no stream `{stream}`; it has {known:?}")]
    NoStream {
        definition: String,
        stream: String,
        known: Vec<String>,
    },
    #[error("connector definition `{definition}` has no object `{object}`; it has {known:?}")]
    NoObject {
        definition: String,
        object: String,
        known: Vec<String>,
    },
    #[error("`{definition}` needs `{key}` in the connector config (in {at})")]
    MissingConfig {
        definition: String,
        key: String,
        at: String,
    },
    #[error("`{definition}` authenticates with `{kind}`, so the sync must name a secret for it")]
    NoSecret { definition: String, kind: String },
    #[error("`{definition}`: config `{key}` must be a non-negative integer")]
    BadOverride { definition: String, key: String },
}

/// The identity of a connector definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

/// How a definition authenticates. It names the scheme and the fields of the
/// sync's secret that hold the values; the host adds the credential to each
/// request, and the guest never holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Auth {
    #[default]
    None,
    /// `Authorization: Bearer <field>`.
    Bearer {
        #[serde(default = "default_token_field")]
        field: String,
    },
    /// A header carrying the secret's `field`.
    ApiKey { header: String, field: String },
    /// A query parameter carrying the secret's `field`.
    ApiKeyQuery { param: String, field: String },
    /// HTTP Basic from two secret fields.
    Basic {
        #[serde(default = "default_username_field")]
        username_field: String,
        #[serde(default = "default_password_field")]
        password_field: String,
    },
    /// OAuth2: the host runs the grant at `token_url` and adds the bearer.
    #[serde(rename = "oauth2")]
    OAuth2 {
        token_url: String,
        grant: OAuthGrant,
        #[serde(default)]
        scopes: Vec<String>,
    },
    /// A session token: the host logs in at `login_url`, reads the token at
    /// `token_path` of the answer, and sends it as `inject_header`.
    SessionToken {
        login_url: String,
        token_path: String,
        #[serde(default = "default_authorization")]
        inject_header: String,
    },
    /// A Google service account: the host signs a JWT-bearer grant with the
    /// account's key and adds the access token.
    GoogleServiceAccount {
        #[serde(default)]
        scopes: Vec<String>,
    },
    /// A Snowflake key-pair JWT. `account` and `user` come from the config.
    SnowflakeKeypairJwt {
        #[serde(default = "default_account_key")]
        account_key: String,
        #[serde(default = "default_user_key")]
        user_key: String,
    },
}

fn default_token_field() -> String {
    "token".to_string()
}
fn default_username_field() -> String {
    "username".to_string()
}
fn default_password_field() -> String {
    "password".to_string()
}
fn default_authorization() -> String {
    "Authorization".to_string()
}
fn default_account_key() -> String {
    "account".to_string()
}
fn default_user_key() -> String {
    "user".to_string()
}

impl Auth {
    fn kind(&self) -> &'static str {
        match self {
            Auth::None => "none",
            Auth::Bearer { .. } => "bearer",
            Auth::ApiKey { .. } => "api_key",
            Auth::ApiKeyQuery { .. } => "api_key_query",
            Auth::Basic { .. } => "basic",
            Auth::OAuth2 { .. } => "oauth2",
            Auth::SessionToken { .. } => "session_token",
            Auth::GoogleServiceAccount { .. } => "google_service_account",
            Auth::SnowflakeKeypairJwt { .. } => "snowflake_keypair_jwt",
        }
    }
}

/// The OAuth2 grant the host runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthGrant {
    RefreshToken,
    ClientCredentials,
}

/// A field of a stream's declared schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: FieldKind,
    #[serde(default = "yes")]
    pub nullable: bool,
}

fn yes() -> bool {
    true
}

/// The types a definition can declare (weir's Arrow subset).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    Int64,
    Float64,
    Utf8,
    Bool,
    Timestamp,
}

impl FieldKind {
    fn typed(self) -> FieldType {
        match self {
            FieldKind::Int64 => FieldType::Integer,
            FieldKind::Float64 => FieldType::Float,
            FieldKind::Utf8 => FieldType::Str,
            FieldKind::Bool => FieldType::Boolean,
            FieldKind::Timestamp => FieldType::Timestamp,
        }
    }
}

/// One stream of a source definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreamDefinition {
    pub name: String,
    /// Appended to the definition's `base_url`.
    pub path: String,
    #[serde(default)]
    pub primary_key: Vec<String>,
    pub schema: Vec<Field>,
    #[serde(default)]
    pub record_selector: Option<String>,
    #[serde(default)]
    pub incremental: Option<Incremental>,
    #[serde(default)]
    pub pagination: Option<Pagination>,
    #[serde(default)]
    pub partition: Option<PartitionRouter>,
    #[serde(default)]
    pub header_row: bool,
    #[serde(default)]
    pub http_method: HttpMethod,
    #[serde(default)]
    pub request_body: Option<String>,
    #[serde(default)]
    pub request_headers: Vec<(String, String)>,
    /// In-flight transforms, applied by the Sync workflow's mapping task.
    #[serde(default)]
    pub mapping: MappingSpec,
}

/// A source connector definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectorDefinition {
    pub spec: Spec,
    #[serde(default)]
    pub auth: Auth,
    pub base_url: String,
    pub streams: Vec<StreamDefinition>,
}

/// One writable object of a destination definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectDefinition {
    pub name: String,
    /// Templated with `{{ record.<field> }}`.
    pub path: String,
    /// `None` takes the method from the write mode: `PATCH` for upsert,
    /// `POST` otherwise.
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub upsert_key: Option<String>,
    #[serde(default)]
    pub field_map: Vec<(String, String)>,
    #[serde(default)]
    pub body_wrap: Option<String>,
    /// Where a CDC delete goes, templated like `path`.
    #[serde(default)]
    pub delete_path: Option<String>,
    #[serde(default)]
    pub delete_method: Option<String>,
}

/// A destination connector definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DestinationDefinition {
    pub spec: Spec,
    #[serde(default)]
    pub auth: Auth,
    pub base_url: String,
    pub objects: Vec<ObjectDefinition>,
}

fn invalid(name: &str, problem: impl Into<String>) -> DefinitionError {
    DefinitionError::Invalid {
        name: name.to_string(),
        problem: problem.into(),
    }
}

impl ConnectorDefinition {
    /// Parse and validate a source definition.
    pub fn from_yaml(yaml: &str) -> Result<Self, DefinitionError> {
        let d: ConnectorDefinition = serde_yaml::from_str(yaml)?;
        d.validate()?;
        Ok(d)
    }

    pub fn validate(&self) -> Result<(), DefinitionError> {
        let name = &self.spec.name;
        if name.trim().is_empty() {
            return Err(invalid(name, "spec.name is empty"));
        }
        if self.streams.is_empty() {
            return Err(invalid(name, "it declares no streams"));
        }
        for s in &self.streams {
            if s.schema.is_empty() {
                return Err(invalid(
                    name,
                    format!("stream `{}` declares no schema fields", s.name),
                ));
            }
            if let Some(inc) = &s.incremental
                && !s.schema.iter().any(|f| f.name == inc.cursor_field)
            {
                return Err(invalid(
                    name,
                    format!(
                        "stream `{}`: cursor_field `{}` is not in its schema",
                        s.name, inc.cursor_field
                    ),
                ));
            }
            let body = s
                .incremental
                .as_ref()
                .is_some_and(|i| i.inject_into == InjectInto::Body)
                || matches!(
                    &s.pagination,
                    Some(
                        Pagination::Page {
                            inject_into: InjectInto::Body,
                            ..
                        } | Pagination::Offset {
                            inject_into: InjectInto::Body,
                            ..
                        } | Pagination::Cursor {
                            inject_into: InjectInto::Body,
                            ..
                        }
                    )
                );
            if body && s.http_method != HttpMethod::Post {
                return Err(invalid(
                    name,
                    format!(
                        "stream `{}` sends parameters in the body (`inject_into: body`) but is not a POST",
                        s.name
                    ),
                ));
            }
        }
        Ok(())
    }

    pub fn stream_names(&self) -> Vec<String> {
        self.streams.iter().map(|s| s.name.clone()).collect()
    }
}

impl DestinationDefinition {
    /// Parse and validate a destination definition.
    pub fn from_yaml(yaml: &str) -> Result<Self, DefinitionError> {
        let d: DestinationDefinition = serde_yaml::from_str(yaml)?;
        if d.spec.name.trim().is_empty() {
            return Err(invalid(&d.spec.name, "spec.name is empty"));
        }
        if d.objects.is_empty() {
            return Err(invalid(&d.spec.name, "it declares no objects"));
        }
        Ok(d)
    }
}

/// A source stream, compiled.
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledSource {
    /// The guest's config: bind it as the connector's `Config` JSON.
    pub config: RestConfig,
    pub mapping: MappingSpec,
    pub schema: StreamSchema,
    pub primary_key: Vec<String>,
    /// The credential the host adds to the guest's requests.
    pub credential: Option<CredentialSpec>,
}

/// A destination object, compiled.
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledDestination {
    pub config: RestDestConfig,
    pub credential: Option<CredentialSpec>,
}

/// Compile one stream of a source definition with the user's config. `secret`
/// is the secret (or local binding) the credential reads; a definition with
/// auth needs one.
pub fn compile_source(
    def: &ConnectorDefinition,
    stream: &str,
    user: &Map<String, Value>,
    secret: Option<&str>,
) -> Result<CompiledSource, CompileError> {
    let name = def.spec.name.as_str();
    let s = def
        .streams
        .iter()
        .find(|s| s.name == stream)
        .ok_or_else(|| CompileError::NoStream {
            definition: name.to_string(),
            stream: stream.to_string(),
            known: def.stream_names(),
        })?;
    let r = Renderer {
        definition: name,
        user,
    };
    let at = |what: &str| format!("stream `{}` {what}", s.name);
    let incremental = match &s.incremental {
        None => None,
        Some(i) => Some(Incremental {
            start_value: r.opt(i.start_value.as_deref(), &at("incremental.start_value"))?,
            end_value: r.opt(i.end_value.as_deref(), &at("incremental.end_value"))?,
            ..i.clone()
        }),
    };
    let partition = match &s.partition {
        Some(PartitionRouter::List { field, values }) => Some(PartitionRouter::List {
            field: field.clone(),
            values: values
                .iter()
                .map(|v| r.render(v, &at("partition.values")))
                .collect::<Result<_, _>>()?,
        }),
        Some(PartitionRouter::Substream {
            field,
            parent_path,
            parent_record_selector,
            parent_key,
        }) => Some(PartitionRouter::Substream {
            field: field.clone(),
            parent_path: r.render(parent_path, &at("partition.parent_path"))?,
            parent_record_selector: parent_record_selector.clone(),
            parent_key: parent_key.clone(),
        }),
        None => None,
    };
    let (max_retries, max_pages) = overrides(name, user)?;
    let config = RestConfig {
        base_url: r.render(base_url(&def.base_url, user), "base_url")?,
        path: r.render(&s.path, &at("path"))?,
        record_selector: s.record_selector.clone(),
        http_method: s.http_method,
        request_body: r.opt(s.request_body.as_deref(), &at("request_body"))?,
        request_headers: s
            .request_headers
            .iter()
            .map(|(k, v)| Ok((k.clone(), r.render(v, &at("request_headers"))?)))
            .collect::<Result<_, CompileError>>()?,
        header_row: s.header_row,
        pagination: s.pagination.clone(),
        incremental,
        partition,
        max_retries: max_retries.unwrap_or(4),
        max_pages: max_pages.unwrap_or(1000),
    };
    Ok(CompiledSource {
        config,
        mapping: s.mapping.clone(),
        schema: StreamSchema {
            fields: s
                .schema
                .iter()
                .map(|f| SchemaField {
                    name: f.name.clone(),
                    field_type: f.ty.typed(),
                    nullable: f.nullable,
                })
                .collect(),
        },
        primary_key: s.primary_key.clone(),
        credential: credential(name, &def.auth, user, secret)?,
    })
}

/// Compile one object of a destination definition with the user's config.
pub fn compile_destination(
    def: &DestinationDefinition,
    object: &str,
    user: &Map<String, Value>,
    secret: Option<&str>,
) -> Result<CompiledDestination, CompileError> {
    let name = def.spec.name.as_str();
    let o = def
        .objects
        .iter()
        .find(|o| o.name == object)
        .ok_or_else(|| CompileError::NoObject {
            definition: name.to_string(),
            object: object.to_string(),
            known: def.objects.iter().map(|o| o.name.clone()).collect(),
        })?;
    let r = Renderer {
        definition: name,
        user,
    };
    let (max_retries, _) = overrides(name, user)?;
    Ok(CompiledDestination {
        config: RestDestConfig {
            base_url: r.render(base_url(&def.base_url, user), "base_url")?,
            path: r.render(&o.path, &format!("object `{}` path", o.name))?,
            method: o.method.clone(),
            upsert_key: o.upsert_key.clone(),
            field_map: o.field_map.clone(),
            body_wrap: o.body_wrap.clone(),
            delete_path: o
                .delete_path
                .as_deref()
                .map(|p| r.render(p, &format!("object `{}` delete_path", o.name)))
                .transpose()?,
            delete_method: o.delete_method.clone(),
            max_retries: max_retries.unwrap_or(4),
        },
        credential: credential(name, &def.auth, user, secret)?,
    })
}

/// The user's `base_url` when given, else the definition's.
fn base_url<'a>(definition: &'a str, user: &'a Map<String, Value>) -> &'a str {
    match user.get("base_url") {
        Some(Value::String(s)) if !s.is_empty() => s,
        _ => definition,
    }
}

fn overrides(
    name: &str,
    user: &Map<String, Value>,
) -> Result<(Option<u32>, Option<u64>), CompileError> {
    let get = |key: &str| -> Result<Option<u64>, CompileError> {
        match user.get(key) {
            None => Ok(None),
            Some(v) => v
                .as_u64()
                .map(Some)
                .ok_or_else(|| CompileError::BadOverride {
                    definition: name.to_string(),
                    key: key.to_string(),
                }),
        }
    };
    Ok((
        get("max_retries")?.map(|n| n.min(u32::MAX as u64) as u32),
        get("max_pages")?,
    ))
}

/// The credential the host adds, from the definition's auth and the sync's
/// secret.
fn credential(
    name: &str,
    auth: &Auth,
    user: &Map<String, Value>,
    secret: Option<&str>,
) -> Result<Option<CredentialSpec>, CompileError> {
    if *auth == Auth::None {
        return Ok(None);
    }
    let secret = secret
        .ok_or_else(|| CompileError::NoSecret {
            definition: name.to_string(),
            kind: auth.kind().to_string(),
        })?
        .to_string();
    let r = Renderer {
        definition: name,
        user,
    };
    Ok(Some(match auth {
        Auth::None => unreachable!(),
        Auth::Bearer { field } => CredentialSpec::Header {
            secret,
            name: "Authorization".to_string(),
            field: field.clone(),
            prefix: "Bearer ".to_string(),
        },
        Auth::ApiKey { header, field } => CredentialSpec::Header {
            secret,
            name: header.clone(),
            field: field.clone(),
            prefix: String::new(),
        },
        Auth::ApiKeyQuery { param, field } => CredentialSpec::Query {
            secret,
            param: param.clone(),
            field: field.clone(),
        },
        Auth::SnowflakeKeypairJwt {
            account_key,
            user_key,
        } => CredentialSpec::SnowflakeKeypairJwt {
            secret,
            account: r.key(account_key, "auth.account_key")?,
            user: r.key(user_key, "auth.user_key")?,
        },
        Auth::Basic {
            username_field,
            password_field,
        } => CredentialSpec::Basic {
            secret,
            username_field: username_field.clone(),
            password_field: password_field.clone(),
        },
        Auth::OAuth2 {
            token_url,
            grant,
            scopes,
        } => CredentialSpec::OAuth2 {
            secret,
            token_url: r.render(token_url, "auth.token_url")?,
            grant: match grant {
                OAuthGrant::ClientCredentials => GrantKind::ClientCredentials,
                OAuthGrant::RefreshToken => GrantKind::RefreshToken,
            },
            scopes: scopes.clone(),
        },
        // A bare login: a definition does not yet say how to build a login
        // body from the secret.
        Auth::SessionToken {
            login_url,
            token_path,
            inject_header,
        } => CredentialSpec::Session {
            secret,
            login_url: r.render(login_url, "auth.login_url")?,
            method: "POST".to_string(),
            login_fields: Default::default(),
            token_path: token_path.clone(),
            inject_header: inject_header.clone(),
            prefix: String::new(),
            ttl_secs: 900,
        },
        Auth::GoogleServiceAccount { scopes } => CredentialSpec::GoogleServiceAccount {
            secret,
            scopes: scopes.clone(),
        },
    }))
}

/// Renders `{{ config['key'] }}` against the user's config. Other templates
/// (`{{ stream_partition.id }}`, `{{ record.id }}`) are the guest's and stay.
struct Renderer<'a> {
    definition: &'a str,
    user: &'a Map<String, Value>,
}

impl Renderer<'_> {
    fn key(&self, key: &str, at: &str) -> Result<String, CompileError> {
        match self.user.get(key) {
            Some(Value::String(s)) => Ok(s.clone()),
            Some(Value::Null) | None => Err(CompileError::MissingConfig {
                definition: self.definition.to_string(),
                key: key.to_string(),
                at: at.to_string(),
            }),
            Some(other) => Ok(other.to_string()),
        }
    }

    fn opt(&self, t: Option<&str>, at: &str) -> Result<Option<String>, CompileError> {
        t.map(|t| self.render(t, at)).transpose()
    }

    fn render(&self, t: &str, at: &str) -> Result<String, CompileError> {
        let mut out = String::with_capacity(t.len());
        let mut rest = t;
        while let Some(start) = rest.find("{{") {
            let Some(len) = rest[start..].find("}}") else {
                break;
            };
            let expr = &rest[start + 2..start + len];
            out.push_str(&rest[..start]);
            match config_key(expr) {
                Some(key) => out.push_str(&self.key(key, at)?),
                None => out.push_str(&rest[start..start + len + 2]),
            }
            rest = &rest[start + len + 2..];
        }
        out.push_str(rest);
        Ok(out)
    }
}

/// `KEY` of a `config['KEY']` or `config["KEY"]` expression.
fn config_key(expr: &str) -> Option<&str> {
    let inner = expr.trim().strip_prefix("config[")?.strip_suffix(']')?;
    let inner = inner.trim();
    inner
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .or_else(|| inner.strip_prefix('"').and_then(|s| s.strip_suffix('"')))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SOURCE: &str = r#"
spec: { name: example }
base_url: "https://{{ config['subdomain'] }}.example.com/api"
auth: { kind: bearer, field: api_token }
streams:
  - name: posts
    path: /posts
    primary_key: [id]
    schema:
      - { name: id, type: int64, nullable: false }
      - { name: title, type: utf8 }
      - { name: updated_at, type: timestamp }
    incremental:
      cursor_field: updated_at
      cursor_param: since
      start_value: "{{ config['start_date'] }}"
    pagination: { kind: page, page_param: page, size_param: per_page, size: 50 }
  - name: comments
    path: "/posts/{{ stream_partition.post_id }}/comments"
    schema: [ { name: id, type: int64 } ]
    partition:
      kind: substream
      field: post_id
      parent_path: /posts
      parent_key: id
    pagination: { kind: link_header }
  - name: search
    path: /search
    http_method: POST
    request_body: '{"workspace": "{{ config["workspace"] }}"}'
    request_headers: [["X-Version", "{{ config['api_version'] }}"]]
    schema: [ { name: id, type: utf8 } ]
    pagination: { kind: cursor, cursor_path: next, token_param: start_cursor, inject_into: body }
"#;

    fn user(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    fn def() -> ConnectorDefinition {
        ConnectorDefinition::from_yaml(SOURCE).expect("the sample parses")
    }

    #[test]
    fn a_stream_compiles_with_its_templates_rendered() {
        let c = compile_source(
            &def(),
            "posts",
            &user(json!({"subdomain": "acme", "start_date": "2026-01-01"})),
            Some("example_api"),
        )
        .unwrap();
        assert_eq!(c.config.base_url, "https://acme.example.com/api");
        assert_eq!(c.config.path, "/posts");
        assert_eq!(
            c.config
                .incremental
                .as_ref()
                .unwrap()
                .start_value
                .as_deref(),
            Some("2026-01-01")
        );
        assert_eq!(c.config.max_pages, 1000);
        assert_eq!(c.primary_key, vec!["id"]);
        assert_eq!(c.schema.fields[0].field_type, FieldType::Integer);
        assert!(!c.schema.fields[0].nullable);
        assert_eq!(c.schema.fields[2].field_type, FieldType::Timestamp);
        assert_eq!(
            c.credential,
            Some(CredentialSpec::Header {
                secret: "example_api".into(),
                name: "Authorization".into(),
                field: "api_token".into(),
                prefix: "Bearer ".into(),
            })
        );
        // The compiled config is what the guest deserializes.
        let text = serde_json::to_string(&c.config).unwrap();
        assert_eq!(serde_json::from_str::<RestConfig>(&text).unwrap(), c.config);
    }

    #[test]
    fn guest_templates_stay_and_body_and_headers_render() {
        let u = user(json!({"subdomain": "acme", "workspace": "w1", "api_version": "2026-10"}));
        let c = compile_source(&def(), "comments", &u, Some("s")).unwrap();
        assert_eq!(
            c.config.path,
            "/posts/{{ stream_partition.post_id }}/comments"
        );
        assert!(matches!(
            c.config.partition,
            Some(PartitionRouter::Substream { .. })
        ));
        assert_eq!(c.config.pagination, Some(Pagination::LinkHeader));
        let c = compile_source(&def(), "search", &u, Some("s")).unwrap();
        assert_eq!(c.config.http_method, HttpMethod::Post);
        assert_eq!(
            c.config.request_body.as_deref(),
            Some(r#"{"workspace": "w1"}"#)
        );
        assert_eq!(
            c.config.request_headers,
            vec![("X-Version".to_string(), "2026-10".to_string())]
        );
    }

    #[test]
    fn offset_pagination_and_a_list_partition_with_templated_values() {
        let d = ConnectorDefinition::from_yaml(
            r#"
spec: { name: listy }
base_url: https://api.example.com
streams:
  - name: rates
    path: "/rates/{{ stream_partition.currency }}"
    schema: [ { name: rate, type: float64 } ]
    partition:
      kind: list
      field: currency
      values: ["USD", "{{ config['home_currency'] }}"]
    pagination: { kind: offset, offset_param: offset, limit_param: limit, size: 100 }
"#,
        )
        .unwrap();
        let c = compile_source(&d, "rates", &user(json!({"home_currency": "EUR"})), None).unwrap();
        assert_eq!(
            c.config.partition,
            Some(PartitionRouter::List {
                field: "currency".into(),
                values: vec!["USD".into(), "EUR".into()],
            })
        );
        assert!(matches!(
            c.config.pagination,
            Some(Pagination::Offset { size: 100, .. })
        ));
        assert_eq!(c.schema.fields[0].field_type, FieldType::Float);
        assert_eq!(c.credential, None);
    }

    #[test]
    fn a_missing_config_key_is_named() {
        let err = compile_source(
            &def(),
            "posts",
            &user(json!({"subdomain": "acme"})),
            Some("s"),
        )
        .unwrap_err();
        assert_eq!(
            err,
            CompileError::MissingConfig {
                definition: "example".into(),
                key: "start_date".into(),
                at: "stream `posts` incremental.start_value".into(),
            }
        );
        assert!(err.to_string().contains("start_date"));
    }

    #[test]
    fn an_unknown_stream_lists_the_known_ones() {
        let err = compile_source(&def(), "postz", &Map::new(), Some("s")).unwrap_err();
        assert!(
            err.to_string()
                .contains("[\"posts\", \"comments\", \"search\"]"),
            "{err}"
        );
    }

    #[test]
    fn a_user_base_url_replaces_the_definitions() {
        let u = user(
            json!({"subdomain": "a", "start_date": "x", "base_url": "http://127.0.0.1:9/api"}),
        );
        let c = compile_source(&def(), "posts", &u, Some("s")).unwrap();
        assert_eq!(c.config.base_url, "http://127.0.0.1:9/api");
    }

    #[test]
    fn overrides_and_their_errors() {
        let u =
            user(json!({"subdomain": "a", "start_date": "x", "max_pages": 5, "max_retries": 1}));
        let c = compile_source(&def(), "posts", &u, Some("s")).unwrap();
        assert_eq!((c.config.max_pages, c.config.max_retries), (5, 1));
        let u = user(json!({"subdomain": "a", "start_date": "x", "max_pages": "lots"}));
        assert!(matches!(
            compile_source(&def(), "posts", &u, Some("s")),
            Err(CompileError::BadOverride { .. })
        ));
    }

    #[test]
    fn auth_maps_onto_the_credential_spec() {
        let none = Map::new();
        let with = |auth: &str| {
            let y = format!(
                "spec: {{ name: a }}\nbase_url: https://x\nauth: {auth}\nstreams:\n  - {{ name: s, path: /s, schema: [ {{ name: id, type: int64 }} ] }}\n"
            );
            ConnectorDefinition::from_yaml(&y).unwrap()
        };
        let c = compile_source(
            &with("{ kind: api_key, header: X-Api-Key, field: key }"),
            "s",
            &none,
            Some("k"),
        )
        .unwrap();
        assert_eq!(
            c.credential,
            Some(CredentialSpec::Header {
                secret: "k".into(),
                name: "X-Api-Key".into(),
                field: "key".into(),
                prefix: String::new()
            })
        );
        let c = compile_source(
            &with("{ kind: api_key_query, param: apikey, field: key }"),
            "s",
            &none,
            Some("k"),
        )
        .unwrap();
        assert_eq!(
            c.credential,
            Some(CredentialSpec::Query {
                secret: "k".into(),
                param: "apikey".into(),
                field: "key".into()
            })
        );
        let sf = user(json!({"account": "org-acct", "user": "loader"}));
        let c = compile_source(
            &with("{ kind: snowflake_keypair_jwt }"),
            "s",
            &sf,
            Some("k"),
        )
        .unwrap();
        assert_eq!(
            c.credential,
            Some(CredentialSpec::SnowflakeKeypairJwt {
                secret: "k".into(),
                account: "org-acct".into(),
                user: "loader".into()
            })
        );
        // Auth needs a secret; none needs nothing.
        assert!(matches!(
            compile_source(&with("{ kind: bearer }"), "s", &none, None),
            Err(CompileError::NoSecret { .. })
        ));
        assert_eq!(
            compile_source(&with("{ kind: none }"), "s", &none, None)
                .unwrap()
                .credential,
            None
        );
        // The minted kinds (CLOACINA-T-0989).
        let c = compile_source(
            &with("{ kind: oauth2, token_url: 'https://x/token', grant: client_credentials, scopes: [api] }"),
            "s",
            &none,
            Some("k"),
        )
        .unwrap();
        assert_eq!(
            c.credential,
            Some(CredentialSpec::OAuth2 {
                secret: "k".into(),
                token_url: "https://x/token".into(),
                grant: GrantKind::ClientCredentials,
                scopes: vec!["api".into()],
            })
        );
        let c = compile_source(&with("{ kind: basic }"), "s", &none, Some("k")).unwrap();
        assert!(matches!(c.credential, Some(CredentialSpec::Basic { .. })));
        let c = compile_source(
            &with("{ kind: google_service_account, scopes: [s] }"),
            "s",
            &none,
            Some("k"),
        )
        .unwrap();
        assert!(matches!(
            c.credential,
            Some(CredentialSpec::GoogleServiceAccount { .. })
        ));
        let c = compile_source(
            &with("{ kind: session_token, login_url: 'https://x/login', token_path: data.token }"),
            "s",
            &none,
            Some("k"),
        )
        .unwrap();
        assert!(matches!(c.credential, Some(CredentialSpec::Session { .. })));
    }

    #[test]
    fn a_misspelled_field_fails_by_name() {
        let bad = SOURCE.replace("primary_key: [id]", "primary_keys: [id]");
        let err = ConnectorDefinition::from_yaml(&bad)
            .unwrap_err()
            .to_string();
        assert!(err.contains("primary_keys"), "{err}");
        let bad = SOURCE.replace("size: 50 }", "size: 50, sise: 2 }");
        assert!(ConnectorDefinition::from_yaml(&bad).is_err());
        let bad = SOURCE.replace(
            "kind: bearer, field: api_token",
            "kind: bearer, token_env: X",
        );
        let err = ConnectorDefinition::from_yaml(&bad)
            .unwrap_err()
            .to_string();
        assert!(err.contains("token_env"), "{err}");
    }

    #[test]
    fn validation_keeps_weirs_rules() {
        let bad = SOURCE.replace("cursor_field: updated_at", "cursor_field: nope");
        assert!(
            ConnectorDefinition::from_yaml(&bad)
                .unwrap_err()
                .to_string()
                .contains("nope")
        );
        let bad = SOURCE.replace("    http_method: POST\n", "");
        let err = ConnectorDefinition::from_yaml(&bad)
            .unwrap_err()
            .to_string();
        assert!(err.contains("not a POST"), "{err}");
        let bad = "spec: { name: x }\nbase_url: https://x\nstreams: []\n";
        assert!(ConnectorDefinition::from_yaml(bad).is_err());
    }

    #[test]
    fn a_destination_compiles() {
        let d = DestinationDefinition::from_yaml(
            r#"
spec: { name: acme-crm }
base_url: "https://{{ config['region'] }}.acme.com"
auth: { kind: bearer }
objects:
  - name: contacts
    path: "/crm/contacts/{{ record.email }}"
    method: PATCH
    upsert_key: email
    field_map: [[email, email], [full_name, name]]
    body_wrap: properties
    delete_path: "/crm/contacts/{{ record.email }}"
"#,
        )
        .unwrap();
        let c = compile_destination(&d, "contacts", &user(json!({"region": "eu"})), Some("crm"))
            .unwrap();
        assert_eq!(c.config.base_url, "https://eu.acme.com");
        assert_eq!(c.config.path, "/crm/contacts/{{ record.email }}");
        assert_eq!(c.config.method.as_deref(), Some("PATCH"));
        assert_eq!(
            c.config.delete_path.as_deref(),
            Some("/crm/contacts/{{ record.email }}")
        );
        assert!(matches!(c.credential, Some(CredentialSpec::Header { .. })));
        assert!(matches!(
            compile_destination(&d, "deals", &Map::new(), Some("crm")),
            Err(CompileError::NoObject { .. })
        ));
        assert!(
            DestinationDefinition::from_yaml(
                "spec: { name: x }\nbase_url: https://x\nobjects: []\n"
            )
            .is_err()
        );
    }
}
