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

//! The Airbyte low-code importer (COLLIERY-I-0634 / CLOACINA-T-0992), moved
//! from weir's weir-importer.
//!
//! [`import_yaml`] maps an Airbyte low-code (declarative) source onto a
//! [`ConnectorDefinition`] that the REST runtime runs. It covers the common
//! declarative shape: unknown fields are ignored, and a construct the runtime
//! cannot run is an [`ImportError`]. [`analyze`] reports, without running
//! anything, what a file uses that the runtime supports and what it does not.
//!
//! An authenticator's `{{ config['key'] }}` reference becomes the name of the
//! field of the sync's secret that holds the value; the secret itself is
//! named by the sync definition.

use std::collections::BTreeMap;

use cloacina_constructor_contract::sync::rest::{
    HttpMethod, Incremental, InjectInto, Pagination, PartitionRouter,
};
use cloacina_constructor_contract::sync::{CompareOp, ComputeExpr, MappingOp, MappingSpec};
use serde::Deserialize;

use crate::definition::{
    Auth, ConnectorDefinition, DefinitionError, Field, FieldKind as ArrowType, OAuthGrant, Spec,
    StreamDefinition as Stream,
};

/// Errors importing an Airbyte manifest.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("unsupported manifest: {0}")]
    Unsupported(String),
    #[error("produced an invalid connector definition: {0}")]
    Definition(#[from] DefinitionError),
}

// ---- Airbyte declarative-manifest subset (only the fields we read) ----

#[derive(Debug, Deserialize)]
pub struct AirbyteManifest {
    #[serde(default)]
    streams: Vec<DeclarativeStream>,
}

#[derive(Debug, Deserialize)]
struct DeclarativeStream {
    name: String,
    retriever: Retriever,
    #[serde(default)]
    primary_key: PrimaryKey,
    /// weir extension: the records array is row-arrays whose first
    /// row is a header — the runtime zips the rest into objects.
    #[serde(default)]
    header_row: bool,
    #[serde(default)]
    incremental_sync: Option<IncrementalSync>,
    #[serde(default)]
    schema_loader: Option<SchemaLoader>,
    /// In-flight record transforms — `AddFields` / `RemoveFields`.
    #[serde(default)]
    transformations: Vec<Transformation>,
}

/// Airbyte record transforms lowered onto the engine mapping stage.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum Transformation {
    AddFields {
        #[serde(default)]
        fields: Vec<AddField>,
    },
    RemoveFields {
        #[serde(default)]
        field_pointers: Vec<Vec<String>>,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct AddField {
    #[serde(default)]
    path: Vec<String>,
    #[serde(default)]
    value: Option<String>,
}

/// `record_filter` under the record selector: a jinja `condition` string.
#[derive(Debug, Deserialize)]
struct RecordFilter {
    #[serde(default)]
    condition: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Retriever {
    requester: Requester,
    #[serde(default)]
    record_selector: Option<RecordSelector>,
    #[serde(default)]
    paginator: Option<Paginator>,
    #[serde(default)]
    partition_router: Option<AbPartitionRouter>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AbPartitionRouter {
    ListPartitionRouter {
        /// Static list of slice values (a YAML sequence). A `{{ config[...] }}` template
        /// here isn't a static list → reported unsupported.
        #[serde(default)]
        values: Option<serde_yaml::Value>,
        /// The partition field name (Airbyte keys it `cursor_field` on this router).
        #[serde(default)]
        cursor_field: Option<String>,
    },
    SubstreamPartitionRouter {
        #[serde(default)]
        parent_stream_configs: Vec<ParentStreamConfig>,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct ParentStreamConfig {
    #[serde(default)]
    parent_key: String,
    #[serde(default)]
    partition_field: String,
    /// The parent stream — inline (or a YAML anchor, expanded at parse). Held as a raw
    /// value so an unexpected `$ref` shape doesn't fail the whole manifest parse.
    #[serde(default)]
    stream: Option<serde_yaml::Value>,
}

#[derive(Debug, Deserialize)]
struct Requester {
    url_base: String,
    path: String,
    #[serde(default)]
    authenticator: Option<Authenticator>,
    /// Request options: `GET`/`POST`, a JSON body, static headers.
    #[serde(default, rename = "http_method")]
    http_method: Option<String>,
    #[serde(default)]
    request_body_json: Option<serde_yaml::Value>,
    #[serde(default)]
    request_headers: Option<serde_yaml::Value>,
}

// Variant names mirror Airbyte's `type` tag values (the serde tag), so they
// must keep the `Authenticator` suffix.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum Authenticator {
    BearerAuthenticator {
        api_token: String,
    },
    ApiKeyAuthenticator {
        #[serde(default)]
        header: String,
        api_token: String,
        /// Airbyte's structured injection: `{ field_name, inject_into: header|request_parameter }`.
        /// Present → carries the key as a query param or a named header; absent → the flat
        /// `header` field (or `Authorization`).
        #[serde(default)]
        inject_into: Option<RequestInjection>,
    },
    /// OAuth2 — mapped to a host-side grant. The `client_id` /
    /// `client_secret` / `refresh_token` are `{{ config['…'] }}` references; we keep the
    /// config-key names, never the values.
    OAuthAuthenticator {
        token_refresh_endpoint: String,
        #[serde(default)]
        client_id: String,
        #[serde(default)]
        client_secret: String,
        #[serde(default)]
        refresh_token: Option<String>,
        /// `refresh_token` (default) | `client_credentials`.
        #[serde(default)]
        grant_type: Option<String>,
        #[serde(default)]
        scopes: Vec<String>,
    },
    /// Session token — host logs in via `login_requester`, extracts the token at
    /// `session_token_path`, injects it per `request_authentication`.
    SessionTokenAuthenticator {
        #[serde(default)]
        login_requester: Option<LoginRequester>,
        #[serde(default)]
        session_token_path: Vec<String>,
        #[serde(default)]
        request_authentication: Option<RequestAuthentication>,
    },
    /// HTTP Basic — `username` / `password` are `{{ config['…'] }}` refs; host injects
    /// `Authorization: Basic base64(user:pass)`.
    BasicHttpAuthenticator {
        #[serde(default)]
        username: String,
        #[serde(default)]
        password: String,
    },
    /// weir extension — Google service-account JWT-bearer grant, minted
    /// host-side. `service_account_key` is a `{{ config['…'] }}` ref naming the connection-
    /// config key holding the **whole SA JSON key**; the key never enters the guest.
    GoogleServiceAccountAuthenticator {
        #[serde(default)]
        service_account_key: String,
        #[serde(default)]
        scopes: Vec<String>,
    },
    /// weir extension — Snowflake key-pair JWT, self-signed host-side.
    /// All three fields are `{{ config['…'] }}` refs; only the private key is a secret.
    SnowflakeKeypairAuthenticator {
        #[serde(default)]
        account: String,
        #[serde(default)]
        user: String,
        #[serde(default)]
        private_key: String,
    },
    #[serde(other)]
    Other,
}

/// The minimal slice of Airbyte's `SessionTokenAuthenticator.login_requester` we need:
/// where the login request goes.
#[derive(Debug, Deserialize)]
struct LoginRequester {
    url_base: String,
    #[serde(default)]
    path: String,
}

/// How the session token is attached to subsequent requests.
#[derive(Debug, Deserialize)]
struct RequestAuthentication {
    #[serde(default)]
    inject_into: Option<RequestInjection>,
}

#[derive(Debug, Deserialize)]
struct RequestInjection {
    #[serde(default)]
    field_name: Option<String>,
    /// `header` | `request_parameter` (the Airbyte location tag, also keyed `inject_into`).
    #[serde(default, rename = "inject_into")]
    location: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RecordSelector {
    extractor: Extractor,
    #[serde(default)]
    record_filter: Option<RecordFilter>,
}

#[derive(Debug, Deserialize)]
struct Extractor {
    #[serde(default)]
    field_path: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum Paginator {
    DefaultPaginator {
        pagination_strategy: PaginationStrategy,
        #[serde(default)]
        page_size_option: Option<RequestOption>,
        #[serde(default)]
        page_token_option: Option<RequestOption>,
    },
    #[serde(other)]
    NoPagination,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum PaginationStrategy {
    PageIncrement {
        #[serde(default)]
        page_size: Option<u32>,
    },
    OffsetIncrement {
        #[serde(default)]
        page_size: Option<u32>,
    },
    CursorPagination {
        /// e.g. `{{ response['response_metadata']['next_cursor'] }}` — the next-page token —
        /// or `{{ last_record['id'] }}` for Stripe-style cursor-from-last-record.
        cursor_value: String,
        /// e.g. `{{ not response['has_more'] }}` — stop when the referenced response bool
        /// is false; only that idiom is expressible.
        #[serde(default)]
        stop_condition: Option<String>,
        /// Airbyte carries the cursor strategy's page size on the strategy itself.
        #[serde(default)]
        page_size: Option<u32>,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct RequestOption {
    #[serde(default)]
    field_name: Option<String>,
    /// Nested body injection names the target as a path (`field_path: [filter, ts]`).
    #[serde(default)]
    field_path: Vec<String>,
    /// `request_parameter` (default) | `header` | `body_json` | `body_data` — where the
    /// param goes. `body_json`/`body_data` lower to `inject_into: body`.
    #[serde(default)]
    inject_into: Option<String>,
}

/// A [`RequestOption`]'s param name (`field_name`, or dotted `field_path` for nested body
/// injection) and whether it targets the request body.
fn option_param(o: &RequestOption) -> (Option<String>, bool) {
    let name = o
        .field_name
        .clone()
        .or_else(|| (!o.field_path.is_empty()).then(|| o.field_path.join(".")));
    let body = matches!(
        o.inject_into.as_deref(),
        Some("body_json") | Some("body_data")
    );
    (name, body)
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum IncrementalSync {
    DatetimeBasedCursor {
        cursor_field: String,
        // Boxed to keep the enum variant small (clippy `large_enum_variant`).
        #[serde(default)]
        start_time_option: Option<Box<RequestOption>>,
        #[serde(default)]
        end_time_option: Option<Box<RequestOption>>,
        /// A literal / `{{ config[...] }}` string, or a `MinMaxDatetime` object with a
        /// `datetime` field. Kept as a raw value so unexpected shapes don't fail the parse;
        /// boxed to keep the enum variant small (clippy `large_enum_variant`).
        #[serde(default)]
        start_datetime: Option<Box<serde_yaml::Value>>,
        #[serde(default)]
        end_datetime: Option<Box<serde_yaml::Value>>,
        /// weir extension: raw-record dot-path the cursor value is
        /// extracted from (columnar responses), when it differs from `cursor_field`.
        #[serde(default)]
        cursor_value_path: Option<String>,
        /// weir extension: track-only cursor — checkpoint advances but
        /// nothing is injected into the request (APIs that restate history, e.g. GA4).
        #[serde(default)]
        track_only: bool,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum SchemaLoader {
    InlineSchemaLoader {
        schema: JsonSchema,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct JsonSchema {
    #[serde(default)]
    properties: BTreeMap<String, Prop>,
}

#[derive(Debug, Deserialize)]
struct Prop {
    #[serde(rename = "type", default)]
    ty: Option<serde_yaml::Value>,
    #[serde(default)]
    format: Option<String>,
}

/// Airbyte `primary_key` is a string, an array, or an array of arrays.
#[derive(Debug, Default, Deserialize)]
#[serde(untagged)]
enum PrimaryKey {
    One(String),
    Many(Vec<String>),
    Nested(Vec<Vec<String>>),
    #[default]
    None,
}

impl PrimaryKey {
    fn to_vec(&self) -> Vec<String> {
        match self {
            PrimaryKey::One(s) => vec![s.clone()],
            PrimaryKey::Many(v) => v.clone(),
            PrimaryKey::Nested(vv) => vv.iter().flatten().cloned().collect(),
            PrimaryKey::None => Vec::new(),
        }
    }
}

// ---- Mapping ----

/// Parse Airbyte YAML and map it to a validated [`ConnectorDefinition`] named
/// `name`.
pub fn import_yaml(name: &str, yaml: &str) -> Result<ConnectorDefinition, ImportError> {
    let airbyte: AirbyteManifest = serde_yaml::from_str(yaml)?;
    to_definition(name, &airbyte)
}

/// Map a parsed Airbyte manifest to a [`ConnectorDefinition`].
pub fn to_definition(
    name: &str,
    airbyte: &AirbyteManifest,
) -> Result<ConnectorDefinition, ImportError> {
    let first = airbyte
        .streams
        .first()
        .ok_or_else(|| ImportError::Unsupported("manifest declares no streams".into()))?;

    let base_url = first.retriever.requester.url_base.clone();
    let auth = match &first.retriever.requester.authenticator {
        Some(Authenticator::BearerAuthenticator { api_token }) => Auth::Bearer {
            field: cfg_key_ref(api_token, "token"),
        },
        Some(Authenticator::ApiKeyAuthenticator {
            header,
            api_token,
            inject_into,
        }) => {
            let field = cfg_key_ref(api_token, "api_key");
            let injected_field = inject_into.as_ref().and_then(|i| i.field_name.clone());
            let location = inject_into.as_ref().and_then(|i| i.location.as_deref());
            if location == Some("request_parameter") {
                Auth::ApiKeyQuery {
                    param: injected_field.unwrap_or_else(|| "api_key".into()),
                    field,
                }
            } else {
                // Header mode: explicit `header`, else the injected field name, else default.
                let h = if !header.is_empty() {
                    header.clone()
                } else {
                    injected_field.unwrap_or_else(|| "Authorization".into())
                };
                Auth::ApiKey { header: h, field }
            }
        }
        Some(Authenticator::OAuthAuthenticator {
            token_refresh_endpoint,
            client_id,
            client_secret,
            refresh_token,
            grant_type,
            scopes,
        }) => {
            let grant = match grant_type.as_deref() {
                Some("client_credentials") => OAuthGrant::ClientCredentials,
                // Default + explicit `refresh_token`: refresh-token grant.
                _ => OAuthGrant::RefreshToken,
            };
            // The client id, client secret and refresh token are the fixed
            // fields of the sync's OAuth2 secret.
            let _ = (client_id, client_secret, refresh_token);
            Auth::OAuth2 {
                token_url: token_refresh_endpoint.clone(),
                grant,
                scopes: scopes.clone(),
            }
        }
        Some(Authenticator::SessionTokenAuthenticator {
            login_requester: Some(lr),
            session_token_path,
            request_authentication,
        }) => Auth::SessionToken {
            login_url: format!("{}{}", lr.url_base, lr.path),
            token_path: session_token_path.join("."),
            inject_header: request_authentication
                .as_ref()
                .and_then(|ra| ra.inject_into.as_ref())
                .and_then(|i| i.field_name.clone())
                .unwrap_or_else(|| "Authorization".to_string()),
        },
        Some(Authenticator::BasicHttpAuthenticator { username, password }) => Auth::Basic {
            username_field: cfg_key_ref(username, "username"),
            password_field: cfg_key_ref(password, "password"),
        },
        Some(Authenticator::GoogleServiceAccountAuthenticator {
            service_account_key,
            scopes,
        }) => {
            // The account's JSON key is the sync secret's
            // `service_account_key` field.
            let _ = service_account_key;
            Auth::GoogleServiceAccount {
                scopes: scopes.clone(),
            }
        }
        Some(Authenticator::SnowflakeKeypairAuthenticator {
            account,
            user,
            private_key,
        }) => {
            // The private key is the sync secret's `private_key` field.
            let _ = private_key;
            Auth::SnowflakeKeypairJwt {
                account_key: cfg_key_ref(account, "account"),
                user_key: cfg_key_ref(user, "user"),
            }
        }
        _ => Auth::None,
    };

    let mut streams = Vec::new();
    for s in &airbyte.streams {
        let record_selector = s
            .retriever
            .record_selector
            .as_ref()
            .and_then(|rs| rs.extractor.field_path.first().cloned());

        let incremental = match &s.incremental_sync {
            Some(IncrementalSync::DatetimeBasedCursor {
                cursor_field,
                start_time_option,
                end_time_option,
                start_datetime,
                end_datetime,
                cursor_value_path,
                track_only,
            }) => {
                let (cursor_param, start_body) = start_time_option
                    .as_deref()
                    .map(option_param)
                    .unwrap_or((None, false));
                let (end_param, end_body) = end_time_option
                    .as_deref()
                    .map(option_param)
                    .unwrap_or((None, false));
                Some(Incremental {
                    cursor_field: cursor_field.clone(),
                    // Track-only: checkpoint advances, nothing injected.
                    cursor_param: if *track_only {
                        String::new()
                    } else {
                        cursor_param.unwrap_or_else(|| cursor_field.clone())
                    },
                    cursor_value_path: cursor_value_path.clone(),
                    start_value: start_datetime.as_ref().and_then(|v| datetime_str(v)),
                    end_param,
                    end_value: end_datetime.as_ref().and_then(|v| datetime_str(v)),
                    // Either bound asking for the body puts the pair there.
                    inject_into: if start_body || end_body {
                        InjectInto::Body
                    } else {
                        InjectInto::Query
                    },
                })
            }
            _ => None,
        };

        let schema = map_schema(&s.schema_loader);
        if schema.is_empty() {
            return Err(ImportError::Unsupported(format!(
                "stream `{}` has no inline schema (v0 needs an InlineSchemaLoader)",
                s.name
            )));
        }

        let req = &s.retriever.requester;
        let request_headers: Vec<(String, String)> = req
            .request_headers
            .as_ref()
            .and_then(|v| v.as_mapping())
            .map(|m| {
                m.iter()
                    .filter_map(|(k, val)| {
                        Some((k.as_str()?.to_string(), val.as_str()?.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        streams.push(Stream {
            name: s.name.clone(),
            path: req.path.clone(),
            primary_key: s.primary_key.to_vec(),
            schema,
            record_selector,
            incremental,
            pagination: map_paginator(&s.retriever.paginator),
            partition: s
                .retriever
                .partition_router
                .as_ref()
                .and_then(map_partition_router),
            header_row: s.header_row,
            http_method: match req.http_method.as_deref() {
                Some(m) if m.eq_ignore_ascii_case("post") => HttpMethod::Post,
                _ => HttpMethod::Get,
            },
            // `request_body_json` (an object) → the POST body as a JSON string.
            request_body: req
                .request_body_json
                .as_ref()
                .and_then(|v| serde_json::to_string(v).ok()),
            request_headers,
            mapping: map_transforms(s),
        });
    }

    let definition = ConnectorDefinition {
        spec: Spec {
            name: name.to_string(),
            version: "0.1.0".to_string(),
        },
        auth,
        base_url,
        streams,
    };

    // Round-trip through the parser so the output is valid and serializable.
    let yaml = serde_yaml::to_string(&definition)?;
    Ok(ConnectorDefinition::from_yaml(&yaml)?)
}

/// A pre-onboard **fidelity report** for a declarative manifest:
/// what the shared runtime will + won't support, *without* running anything. The
/// preview gate so we never onboard a silently-broken connector.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportReport {
    /// `A` full · `B` degraded · `C` significant gaps · `F` unparseable.
    pub tier: String,
    /// 0.0–1.0 fraction of declared features the runtime supports.
    pub confidence: f32,
    pub streams: Vec<String>,
    /// Named, human-readable gaps — never silently dropped.
    pub unsupported: Vec<String>,
}

/// Analyze a declarative manifest against the **shared runtime's current
/// capability** (HTTP `base_url`/`path`/record-selector, page-increment pagination,
/// datetime cursor). Auth, offset pagination, and anything the importer can't parse
/// are reported as gaps. Pure — no run, no registration.
pub fn analyze(yaml: &str) -> ImportReport {
    let m: AirbyteManifest = match serde_yaml::from_str(yaml) {
        Ok(m) => m,
        Err(e) => {
            return ImportReport {
                tier: "F".to_string(),
                confidence: 0.0,
                streams: Vec::new(),
                unsupported: vec![format!("unparseable manifest: {e}")],
            };
        }
    };
    let mut streams = Vec::new();
    let mut unsupported = Vec::new();
    if m.streams.is_empty() {
        unsupported.push("no streams declared".to_string());
    }
    for s in &m.streams {
        streams.push(s.name.clone());
        match &s.retriever.requester.authenticator {
            // Bearer + header api-key are applied by the runtime; the
            // secret is supplied per-connection as `api_key`, so it's a config
            // requirement, not an unsupported construct.
            // Bearer / api-key / OAuth2 / session-token are all injected host-side by the
            // egress policy; the secret is supplied per-connection, so
            // these are a config requirement, not an unsupported construct.
            Some(Authenticator::BearerAuthenticator { .. })
            | Some(Authenticator::ApiKeyAuthenticator { .. })
            | Some(Authenticator::OAuthAuthenticator { .. })
            | Some(Authenticator::SessionTokenAuthenticator { .. })
            | Some(Authenticator::BasicHttpAuthenticator { .. })
            | Some(Authenticator::GoogleServiceAccountAuthenticator { .. })
            | Some(Authenticator::SnowflakeKeypairAuthenticator { .. }) => {}
            Some(Authenticator::Other) => {
                unsupported.push(format!("{}: unsupported authenticator", s.name))
            }
            None => {}
        }
        if let Some(Paginator::DefaultPaginator {
            pagination_strategy,
            ..
        }) = &s.retriever.paginator
        {
            match pagination_strategy {
                // Page-increment, offset, and opaque cursor are applied by the runtime.
                PaginationStrategy::PageIncrement { .. }
                | PaginationStrategy::OffsetIncrement { .. }
                | PaginationStrategy::CursorPagination { .. } => {}
                PaginationStrategy::Other => {
                    unsupported.push(format!("{}: unsupported pagination strategy", s.name))
                }
            }
        }
        if let Some(IncrementalSync::Other) = &s.incremental_sync {
            unsupported.push(format!("{}: unsupported incremental cursor", s.name));
        }
        // List + substream routers are supported; a router present but
        // not translatable (custom, templated `values`, `$ref` parent) is reported so we
        // don't silently drop the partitioning.
        if let Some(pr) = &s.retriever.partition_router
            && map_partition_router(pr).is_none()
        {
            unsupported.push(format!("{}: unsupported partition router", s.name));
        }
        // Transforms map onto the engine mapping stage; forms the lowering
        // can't express (complex AddFields values, unknown transforms, compound conditions)
        // are reported so they're never silently dropped.
        for t in &s.transformations {
            match t {
                Transformation::AddFields { fields } => {
                    for f in fields {
                        if let Some(raw) = &f.value
                            && f.path.last().is_some()
                            && compute_expr(raw).is_none()
                        {
                            unsupported
                                .push(format!("{}: unsupported AddFields value `{raw}`", s.name));
                        }
                    }
                }
                Transformation::RemoveFields { .. } => {}
                Transformation::Other => {
                    unsupported.push(format!("{}: unsupported transformation", s.name))
                }
            }
        }
        if let Some(cond) = s
            .retriever
            .record_selector
            .as_ref()
            .and_then(|r| r.record_filter.as_ref())
            .and_then(|rf| rf.condition.as_ref())
            && filter_op(cond).is_none()
        {
            unsupported.push(format!("{}: unsupported record_filter `{cond}`", s.name));
        }
        if !matches!(
            s.schema_loader,
            Some(SchemaLoader::InlineSchemaLoader { .. })
        ) {
            unsupported.push(format!(
                "{}: no inline schema (record types unknown)",
                s.name
            ));
        }
    }
    let (tier, confidence) = if streams.is_empty() {
        ("F".to_string(), 0.0)
    } else {
        let ratio = 1.0 - (unsupported.len() as f32 / (streams.len() as f32 * 4.0)).min(1.0);
        let tier = if unsupported.is_empty() {
            "A"
        } else if ratio > 0.6 {
            "B"
        } else {
            "C"
        };
        (tier.to_string(), ratio)
    };
    ImportReport {
        tier,
        confidence,
        streams,
        unsupported,
    }
}

/// Map an Airbyte partition router → weir [`PartitionRouter`]. Returns `None` (no
/// routing, reported as a gap by `analyze`) for shapes we can't translate: templated
/// `ListPartitionRouter` values, a `$ref`/missing parent stream, or a custom router.
fn map_partition_router(r: &AbPartitionRouter) -> Option<PartitionRouter> {
    match r {
        AbPartitionRouter::ListPartitionRouter {
            values,
            cursor_field,
        } => {
            let vals: Vec<String> = values
                .as_ref()
                .and_then(|v| v.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            if vals.is_empty() {
                return None; // templated/config-driven values aren't a static list (v1)
            }
            Some(PartitionRouter::List {
                field: cursor_field
                    .clone()
                    .unwrap_or_else(|| "partition".to_string()),
                values: vals,
            })
        }
        AbPartitionRouter::SubstreamPartitionRouter {
            parent_stream_configs,
        } => {
            let pc = parent_stream_configs.first()?;
            let (parent_path, parent_record_selector) = parent_request(pc.stream.as_ref()?)?;
            Some(PartitionRouter::Substream {
                field: if pc.partition_field.is_empty() {
                    "parent_id".to_string()
                } else {
                    pc.partition_field.clone()
                },
                parent_path,
                parent_record_selector,
                parent_key: if pc.parent_key.is_empty() {
                    "id".to_string()
                } else {
                    pc.parent_key.clone()
                },
            })
        }
        AbPartitionRouter::Other => None,
    }
}

/// Pull the request `path` + record selector out of an (inline / anchor-expanded) parent
/// stream value, without requiring it to deserialize as a full `DeclarativeStream`.
fn parent_request(stream: &serde_yaml::Value) -> Option<(String, Option<String>)> {
    let retriever = stream.get("retriever")?;
    let path = retriever
        .get("requester")?
        .get("path")?
        .as_str()?
        .to_string();
    let record_selector = retriever
        .get("record_selector")
        .and_then(|rs| rs.get("extractor"))
        .and_then(|e| e.get("field_path"))
        .and_then(|fp| fp.as_sequence())
        .and_then(|s| s.first())
        .and_then(|v| v.as_str())
        .map(String::from);
    Some((path, record_selector))
}

/// Lower a stream's Airbyte transforms + `record_filter` onto the engine mapping stage
///. Clean forms map; jinja grammar beyond the simple
/// cases is skipped (reported by `analyze`, not silently emitted as wrong).
fn map_transforms(s: &DeclarativeStream) -> MappingSpec {
    let mut ops: Vec<MappingOp> = Vec::new();
    for t in &s.transformations {
        match t {
            Transformation::RemoveFields { field_pointers } => {
                let fields: Vec<String> = field_pointers
                    .iter()
                    .filter_map(|p| p.last().cloned())
                    .collect();
                if !fields.is_empty() {
                    ops.push(MappingOp::Drop { fields });
                }
            }
            Transformation::AddFields { fields } => {
                for f in fields {
                    if let (Some(field), Some(raw)) = (f.path.last().cloned(), f.value.as_ref())
                        && let Some(value) = compute_expr(raw)
                    {
                        ops.push(MappingOp::Compute { field, value });
                    }
                }
            }
            Transformation::Other => {}
        }
    }
    if let Some(cond) = s
        .retriever
        .record_selector
        .as_ref()
        .and_then(|r| r.record_filter.as_ref())
        .and_then(|rf| rf.condition.as_ref())
        && let Some(op) = filter_op(cond)
    {
        ops.push(op);
    }
    MappingSpec { ops }
}

/// A **lone** field reference — `record['x']` / `record["x"]` / `record.x`, or a
/// **chained accessor** into nested values:
/// `record['dimensionValues'][0]['value']` → the dot-path `dimensionValues.0.value`
/// (numeric segments index arrays in the engine's mapping stage). Rejects anything
/// that isn't a clean accessor chain (compound expressions parse to `None`, not junk).
fn record_field(s: &str) -> Option<String> {
    let mut rest = s.trim().strip_prefix("record")?.trim();
    let mut segs: Vec<String> = Vec::new();
    while !rest.is_empty() {
        if let Some(b) = rest.strip_prefix('[') {
            let end = b.find(']')?;
            let seg = b[..end].trim().trim_matches(|c| c == '\'' || c == '"');
            segs.push(seg.to_string());
            rest = b[end + 1..].trim_start();
        } else if let Some(f) = rest.strip_prefix('.') {
            // Dotted tail: `record.a.b` — the remainder is the whole (dot-joined) path.
            segs.extend(f.trim().split('.').map(str::to_string));
            rest = "";
        } else {
            return None; // trailing junk → a larger expression, not a lone reference
        }
    }
    let clean = !segs.is_empty()
        && segs
            .iter()
            .all(|f| !f.is_empty() && f.chars().all(|c| c.is_alphanumeric() || c == '_'));
    clean.then(|| segs.join("."))
}

/// An `AddFields` value → a bounded compute expr: a literal (`Const`) or a single record
/// reference (`Field`). Richer jinja (concat, filters, arithmetic) is skipped/reported.
fn compute_expr(raw: &str) -> Option<ComputeExpr> {
    let t = raw.trim();
    if !t.contains("{{") {
        return Some(ComputeExpr::Const(t.to_string()));
    }
    let inner = t.trim_start_matches("{{").trim_end_matches("}}").trim();
    record_field(inner).map(ComputeExpr::Field)
}

/// A `record_filter` condition `{{ record['f'] OP value }}` → a `Filter` op. Compound
/// conditions (and/or, functions) are skipped/reported.
fn filter_op(cond: &str) -> Option<MappingOp> {
    let inner = cond
        .trim()
        .trim_start_matches("{{")
        .trim_end_matches("}}")
        .trim();
    // Two-char operators before one-char so `>=`/`<=` aren't split as `>`/`<`.
    for (sym, op) in [
        ("==", CompareOp::Eq),
        ("!=", CompareOp::Ne),
        (">=", CompareOp::Ge),
        ("<=", CompareOp::Le),
        (">", CompareOp::Gt),
        ("<", CompareOp::Lt),
    ] {
        if let Some(idx) = inner.find(sym) {
            let field = record_field(&inner[..idx])?;
            let rhs = inner[idx + sym.len()..].trim();
            // Reject compound conditions (a second field ref / boolean join) — reported, not
            // mis-parsed into a wrong single filter.
            if rhs.contains("record") || rhs.contains(" and ") || rhs.contains(" or ") {
                return None;
            }
            let value = rhs.trim_matches(|c| c == '\'' || c == '"');
            return Some(MappingOp::Filter {
                field,
                op,
                value: value.to_string(),
            });
        }
    }
    None
}

/// Extract a datetime string from an Airbyte `start_datetime`/`end_datetime`: either a bare
/// string (literal or `{{ config[...] }}`) or a `MinMaxDatetime` object's `datetime` field.
fn datetime_str(v: &serde_yaml::Value) -> Option<String> {
    match v {
        serde_yaml::Value::String(s) => Some(s.clone()),
        serde_yaml::Value::Mapping(_) => {
            v.get("datetime").and_then(|d| d.as_str()).map(String::from)
        }
        _ => None,
    }
}

fn map_paginator(p: &Option<Paginator>) -> Option<Pagination> {
    let Some(Paginator::DefaultPaginator {
        pagination_strategy,
        page_size_option,
        page_token_option,
    }) = p
    else {
        return None;
    };
    let (page_param, token_body) = page_token_option
        .as_ref()
        .map(option_param)
        .unwrap_or((None, false));
    let (size_param, size_body) = page_size_option
        .as_ref()
        .map(option_param)
        .unwrap_or((None, false));
    // Either option asking for the body puts the pair there — in
    // practice APIs carry both in the same place.
    let inject_into = if token_body || size_body {
        InjectInto::Body
    } else {
        InjectInto::Query
    };
    match pagination_strategy {
        PaginationStrategy::PageIncrement { page_size } => Some(Pagination::Page {
            page_param: page_param.unwrap_or_else(|| "page".into()),
            size_param: size_param.unwrap_or_else(|| "page_size".into()),
            size: page_size.unwrap_or(50),
            inject_into,
        }),
        PaginationStrategy::OffsetIncrement { page_size } => Some(Pagination::Offset {
            offset_param: page_param.unwrap_or_else(|| "offset".into()),
            limit_param: size_param.unwrap_or_else(|| "limit".into()),
            size: page_size.unwrap_or(50),
            inject_into,
        }),
        PaginationStrategy::CursorPagination {
            cursor_value,
            stop_condition,
            page_size,
        } => {
            // Link-header form: the cursor references the response `Link` header (rel="next"),
            // not a body path → follow the header. Otherwise a body cursor —
            // sourced from the response, or from the LAST record ( Stripe form).
            let lc = cursor_value.to_ascii_lowercase();
            if lc.contains("headers") && lc.contains("link") {
                Some(Pagination::LinkHeader)
            } else {
                let cursor_record_field = lc
                    .contains("last_record")
                    .then(|| bracket_path(cursor_value, "last_record"))
                    .filter(|p| !p.is_empty());
                let cursor_path = if cursor_record_field.is_some() {
                    String::new()
                } else {
                    response_path(cursor_value)
                };
                // `{{ not response['has_more'] }}` (or `… is false`) — the only stop idiom
                // the runtime expresses: stop when the response bool is false. Airbyte's
                // contract is "stop when the template is TRUE", so ONLY the negated idiom
                // lowers; a positive condition (`{{ response['is_last_page'] }}`) would
                // invert — stopping after page 1 exactly when it should continue — so it
                // is dropped and cursor-absence terminates pagination instead.
                let stop_on_false_path = stop_condition
                    .as_ref()
                    .filter(|s| {
                        let l = s.to_ascii_lowercase();
                        l.contains("not ") || l.contains("is false")
                    })
                    .map(|s| bracket_path(s, "response"))
                    .filter(|p| !p.is_empty());
                Some(Pagination::Cursor {
                    cursor_path,
                    token_param: page_param.unwrap_or_else(|| "cursor".into()),
                    cursor_record_field,
                    stop_on_false_path,
                    size_param: size_param.clone(),
                    size: *page_size,
                    inject_into,
                })
            }
        }
        PaginationStrategy::Other => None,
    }
}

/// Extract the dpath from a `{{ response['a']['b'] }}` cursor template → `"a.b"`
/// (the bracketed keys after `response`). Returns "" if no brackets found.
fn response_path(template: &str) -> String {
    bracket_path(template, "response")
}

/// The dotted path of the bracketed keys after `root` in a `{{ root['a']['b'] }}` template
/// (e.g. `bracket_path("{{ last_record['id'] }}", "last_record")` → `"id"`). Returns ""
/// when `root` isn't referenced or carries no bracketed keys.
fn bracket_path(template: &str, root: &str) -> String {
    let Some(start) = template.find(root) else {
        return String::new();
    };
    let mut parts: Vec<&str> = Vec::new();
    let mut rest = &template[start + root.len()..];
    while let Some(i) = rest.find('[') {
        // Brackets must follow contiguously (allowing whitespace) — stop at the first
        // non-bracket so `response['a'] and other['b']` doesn't merge paths.
        if !rest[..i].trim().is_empty() {
            break;
        }
        rest = &rest[i + 1..];
        let key = rest.trim_start_matches([' ', '\'', '"']);
        match key.find(['\'', '"']) {
            Some(end) => {
                parts.push(&key[..end]);
                rest = &key[end..];
                // Skip the closing quote+bracket before looking for the next '['.
                if let Some(close) = rest.find(']') {
                    rest = &rest[close + 1..];
                }
            }
            None => break,
        }
    }
    parts.join(".")
}

fn map_schema(loader: &Option<SchemaLoader>) -> Vec<Field> {
    let Some(SchemaLoader::InlineSchemaLoader { schema }) = loader else {
        return Vec::new();
    };
    schema
        .properties
        .iter()
        .map(|(name, prop)| Field {
            name: name.clone(),
            ty: arrow_type(prop),
            nullable: is_nullable(prop),
        })
        .collect()
}

fn type_strings(v: &Option<serde_yaml::Value>) -> Vec<String> {
    match v {
        Some(serde_yaml::Value::String(s)) => vec![s.clone()],
        Some(serde_yaml::Value::Sequence(seq)) => seq
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

fn arrow_type(p: &Prop) -> ArrowType {
    let types = type_strings(&p.ty);
    let primary = types
        .iter()
        .find(|t| *t != "null")
        .map(String::as_str)
        .unwrap_or("string");
    match primary {
        "integer" => ArrowType::Int64,
        "number" => ArrowType::Float64,
        "boolean" => ArrowType::Bool,
        "string" if p.format.as_deref() == Some("date-time") => ArrowType::Timestamp,
        _ => ArrowType::Utf8,
    }
}

fn is_nullable(p: &Prop) -> bool {
    let types = type_strings(&p.ty);
    types.iter().any(|t| t == "null") || types.len() != 1
}

/// The key of a `{{ config['key'] }}` reference, case kept: it names the field
/// of the sync's secret that holds the value. `default` when the token is not
/// a config reference.
fn cfg_key_ref(token: &str, default: &str) -> String {
    if let Some(start) = token.find("config[") {
        let inner = token[start + "config[".len()..].trim_start_matches(['\'', '"']);
        if let Some(end) = inner.find(['\'', '"']) {
            return inner[..end].to_string();
        }
    }
    default.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const AIRBYTE: &str = r#"
type: DeclarativeSource
streams:
  - type: DeclarativeStream
    name: posts
    primary_key: id
    retriever:
      type: SimpleRetriever
      requester:
        type: HttpRequester
        url_base: "https://api.example.com"
        path: "/posts"
        authenticator:
          type: BearerAuthenticator
          api_token: "{{ config['api_key'] }}"
      record_selector:
        type: RecordSelector
        extractor:
          type: DpathExtractor
          field_path: ["data"]
      paginator:
        type: DefaultPaginator
        page_size_option:
          type: RequestOption
          field_name: "_limit"
          inject_into: request_parameter
        page_token_option:
          type: RequestOption
          field_name: "_page"
        pagination_strategy:
          type: PageIncrement
          page_size: 50
    incremental_sync:
      type: DatetimeBasedCursor
      cursor_field: updated_at
      start_time_option:
        type: RequestOption
        field_name: "since"
    schema_loader:
      type: InlineSchemaLoader
      schema:
        type: object
        properties:
          id: { type: integer }
          title: { type: ["null", "string"] }
          updated_at: { type: string, format: date-time }
"#;

    #[test]
    fn imports_declarative_manifest() {
        let m = import_yaml("jsonplaceholder", AIRBYTE).expect("import");
        assert_eq!(m.spec.name, "jsonplaceholder");
        assert_eq!(m.base_url, "https://api.example.com");
        assert_eq!(
            m.auth,
            Auth::Bearer {
                field: "api_key".into()
            }
        );

        assert_eq!(m.streams.len(), 1);
        let s = &m.streams[0];
        assert_eq!(s.path, "/posts");
        assert_eq!(s.primary_key, vec!["id".to_string()]);
        assert_eq!(s.record_selector.as_deref(), Some("data"));

        let inc = s.incremental.as_ref().expect("incremental");
        assert_eq!(inc.cursor_field, "updated_at");
        assert_eq!(inc.cursor_param, "since");

        match s.pagination.as_ref().expect("pagination") {
            Pagination::Page {
                page_param,
                size_param,
                size,
                ..
            } => {
                assert_eq!(page_param, "_page");
                assert_eq!(size_param, "_limit");
                assert_eq!(*size, 50);
            }
            _ => panic!("expected page pagination"),
        }

        // Schema: BTreeMap ordering → id, title, updated_at.
        let names: Vec<_> = s.schema.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["id", "title", "updated_at"]);
        assert_eq!(s.schema[0].ty, ArrowType::Int64);
        assert!(!s.schema[0].nullable); // single concrete type
        assert_eq!(s.schema[1].ty, ArrowType::Utf8);
        assert!(s.schema[1].nullable); // ["null","string"]
        assert_eq!(s.schema[2].ty, ArrowType::Timestamp); // string + date-time
    }

    #[test]
    fn errors_without_streams() {
        let r = import_yaml("x", "type: DeclarativeSource\nstreams: []\n");
        assert!(r.is_err());
    }

    const CLEAN: &str = r#"
type: DeclarativeSource
streams:
  - type: DeclarativeStream
    name: items
    retriever:
      type: SimpleRetriever
      requester:
        type: HttpRequester
        url_base: "https://api.example.com"
        path: "/items"
      record_selector:
        type: RecordSelector
        extractor:
          type: DpathExtractor
          field_path: []
    schema_loader:
      type: InlineSchemaLoader
      schema:
        type: object
        properties:
          id: { type: integer }
"#;

    #[test]
    fn analyze_reports_runtime_gaps_and_clean_tier() {
        // AIRBYTE uses BearerAuthenticator — now applied by the runtime (the secret is
        // supplied per-connection), so it's a config requirement, not a gap: clean tier A.
        let r = analyze(AIRBYTE);
        assert_eq!(r.streams, vec!["posts"]);
        assert!(
            !r.unsupported.iter().any(|u| u.contains("auth")),
            "auth is applied now, not a gap: {:?}",
            r.unsupported
        );
        assert_eq!(r.tier, "A");

        // A clean no-auth manifest → tier A, no gaps.
        let rc = analyze(CLEAN);
        assert!(
            rc.unsupported.is_empty(),
            "clean → no gaps: {:?}",
            rc.unsupported
        );
        assert_eq!(rc.tier, "A");
        assert!((rc.confidence - 1.0).abs() < f32::EPSILON);

        // Garbage → tier F, named.
        let rf = analyze("not: [a, valid: manifest");
        assert_eq!(rf.tier, "F");
        assert!(!rf.unsupported.is_empty());
    }

    /// A single-stream manifest whose requester carries `auth_block` (the block must
    /// bring its own 8-space indentation to sit under `requester:`).
    fn manifest_with_auth(auth_block: &str) -> String {
        format!(
            r#"
type: DeclarativeSource
streams:
  - type: DeclarativeStream
    name: items
    retriever:
      type: SimpleRetriever
      requester:
        type: HttpRequester
        url_base: "https://api.example.com"
        path: "/items"
{auth_block}
      record_selector:
        type: RecordSelector
        extractor:
          type: DpathExtractor
          field_path: []
    schema_loader:
      type: InlineSchemaLoader
      schema:
        type: object
        properties:
          id: {{ type: integer }}
"#
        )
    }

    #[test]
    fn imports_oauth_refresh_token() {
        let yaml = manifest_with_auth(
            "        authenticator:\n          type: OAuthAuthenticator\n          \
             token_refresh_endpoint: \"https://api.example.com/oauth/token\"\n          \
             client_id: \"{{ config['client_id'] }}\"\n          \
             client_secret: \"{{ config['client_secret'] }}\"\n          \
             refresh_token: \"{{ config['refresh_token'] }}\"\n          \
             scopes: [\"read\", \"write\"]",
        );
        let m = import_yaml("oauthy", &yaml).expect("import");
        assert_eq!(
            m.auth,
            Auth::OAuth2 {
                token_url: "https://api.example.com/oauth/token".into(),
                grant: OAuthGrant::RefreshToken,
                scopes: vec!["read".into(), "write".into()],
            }
        );
    }

    #[test]
    fn imports_oauth_client_credentials() {
        let yaml = manifest_with_auth(
            "        authenticator:\n          type: OAuthAuthenticator\n          \
             token_refresh_endpoint: \"https://api.example.com/oauth/token\"\n          \
             grant_type: client_credentials\n          \
             client_id: \"{{ config['client_id'] }}\"\n          \
             client_secret: \"{{ config['client_secret'] }}\"",
        );
        let m = import_yaml("ccreds", &yaml).expect("import");
        match m.auth {
            Auth::OAuth2 { grant, .. } => {
                assert_eq!(grant, OAuthGrant::ClientCredentials);
            }
            other => panic!("expected OAuth2 client_credentials, got {other:?}"),
        }
    }

    #[test]
    fn imports_session_token() {
        let yaml = manifest_with_auth(
            "        authenticator:\n          type: SessionTokenAuthenticator\n          \
             login_requester:\n            url_base: \"https://api.example.com\"\n            \
             path: \"/login\"\n          session_token_path: [\"data\", \"token\"]\n          \
             request_authentication:\n            type: ApiKey\n            inject_into:\n              \
             type: RequestOption\n              field_name: \"X-Session-Token\"\n              \
             inject_into: header",
        );
        let m = import_yaml("sessiony", &yaml).expect("import");
        assert_eq!(
            m.auth,
            Auth::SessionToken {
                login_url: "https://api.example.com/login".into(),
                token_path: "data.token".into(),
                inject_header: "X-Session-Token".into(),
            }
        );
    }

    #[test]
    fn imports_basic_auth() {
        let yaml = manifest_with_auth(
            "        authenticator:\n          type: BasicHttpAuthenticator\n          \
             username: \"{{ config['username'] }}\"\n          \
             password: \"{{ config['password'] }}\"",
        );
        let m = import_yaml("basicy", &yaml).expect("import");
        assert_eq!(
            m.auth,
            Auth::Basic {
                username_field: "username".into(),
                password_field: "password".into()
            }
        );
    }

    #[test]
    fn imports_link_header_pagination() {
        let yaml = r#"
type: DeclarativeSource
streams:
  - type: DeclarativeStream
    name: items
    retriever:
      type: SimpleRetriever
      requester:
        type: HttpRequester
        url_base: "https://api.example.com"
        path: "/items"
      record_selector:
        type: RecordSelector
        extractor:
          type: DpathExtractor
          field_path: []
      paginator:
        type: DefaultPaginator
        pagination_strategy:
          type: CursorPagination
          cursor_value: "{{ headers['link']['next']['url'] }}"
        page_token_option:
          type: RequestPath
    schema_loader:
      type: InlineSchemaLoader
      schema:
        type: object
        properties:
          id: { type: integer }
"#;
        let m = import_yaml("linky", yaml).expect("import");
        assert_eq!(m.streams[0].pagination, Some(Pagination::LinkHeader));
    }

    #[test]
    fn imports_richer_datetime_cursor() {
        let yaml = r#"
type: DeclarativeSource
streams:
  - type: DeclarativeStream
    name: items
    retriever:
      type: SimpleRetriever
      requester:
        type: HttpRequester
        url_base: "https://api.example.com"
        path: "/items"
      record_selector:
        type: RecordSelector
        extractor:
          type: DpathExtractor
          field_path: []
    incremental_sync:
      type: DatetimeBasedCursor
      cursor_field: updated_at
      start_datetime: "{{ config['start_date'] }}"
      end_datetime: "2026-12-31T00:00:00Z"
      start_time_option:
        type: RequestOption
        field_name: "since"
      end_time_option:
        type: RequestOption
        field_name: "until"
    schema_loader:
      type: InlineSchemaLoader
      schema:
        type: object
        properties:
          id: { type: integer }
          updated_at: { type: string, format: date-time }
"#;
        let m = import_yaml("dty", yaml).expect("import");
        let inc = m.streams[0].incremental.as_ref().expect("incremental");
        assert_eq!(inc.cursor_field, "updated_at");
        assert_eq!(inc.cursor_param, "since");
        assert_eq!(
            inc.start_value.as_deref(),
            Some("{{ config['start_date'] }}")
        );
        assert_eq!(inc.end_param.as_deref(), Some("until"));
        assert_eq!(inc.end_value.as_deref(), Some("2026-12-31T00:00:00Z"));
    }

    #[test]
    fn imports_post_request_options() {
        let yaml = r#"
type: DeclarativeSource
streams:
  - type: DeclarativeStream
    name: items
    retriever:
      type: SimpleRetriever
      requester:
        type: HttpRequester
        url_base: "https://api.example.com"
        path: "/query"
        http_method: POST
        request_body_json:
          page_size: 100
        request_headers:
          Notion-Version: "2022-06-28"
      record_selector:
        type: RecordSelector
        extractor:
          type: DpathExtractor
          field_path: ["results"]
    schema_loader:
      type: InlineSchemaLoader
      schema:
        type: object
        properties:
          id: { type: integer }
"#;
        let m = import_yaml("posty", yaml).expect("import");
        let s = &m.streams[0];
        assert_eq!(s.http_method, HttpMethod::Post);
        assert_eq!(s.request_body.as_deref(), Some(r#"{"page_size":100}"#));
        assert_eq!(
            s.request_headers,
            vec![("Notion-Version".to_string(), "2022-06-28".to_string())]
        );
    }

    /// The weir `GoogleServiceAccountAuthenticator` extension lowers to
    /// `Auth::GoogleServiceAccount` — config-key ref preserved, scopes carried.
    #[test]
    fn imports_google_service_account_authenticator() {
        let yaml = r#"
type: DeclarativeSource
streams:
  - type: DeclarativeStream
    name: reports
    retriever:
      type: SimpleRetriever
      requester:
        type: HttpRequester
        url_base: "https://analyticsdata.googleapis.com"
        path: "/v1beta/properties/123:runReport"
        authenticator:
          type: GoogleServiceAccountAuthenticator
          service_account_key: "{{ config['sa_json'] }}"
          scopes:
            - "https://www.googleapis.com/auth/analytics.readonly"
      record_selector:
        type: RecordSelector
        extractor:
          type: DpathExtractor
          field_path: ["rows"]
    schema_loader:
      type: InlineSchemaLoader
      schema:
        type: object
        properties:
          date: { type: string }
"#;
        let m = import_yaml("ga4ish", yaml).expect("import");
        assert_eq!(
            m.auth,
            Auth::GoogleServiceAccount {
                scopes: vec!["https://www.googleapis.com/auth/analytics.readonly".into()],
            }
        );
    }

    /// The `header_row` stream extension carries through to the weir
    /// manifest (Google Sheets shape).
    #[test]
    fn imports_header_row_stream() {
        let yaml = r#"
type: DeclarativeSource
streams:
  - type: DeclarativeStream
    name: rows
    header_row: true
    retriever:
      type: SimpleRetriever
      requester:
        type: HttpRequester
        url_base: "https://sheets.googleapis.com"
        path: "/v4/spreadsheets/{{ config['spreadsheet_id'] }}/values/{{ config['tab'] }}"
        authenticator:
          type: GoogleServiceAccountAuthenticator
          service_account_key: "{{ config['service_account_key'] }}"
          scopes: ["https://www.googleapis.com/auth/spreadsheets.readonly"]
      record_selector:
        type: RecordSelector
        extractor:
          type: DpathExtractor
          field_path: ["values"]
    schema_loader:
      type: InlineSchemaLoader
      schema:
        type: object
        properties:
          _row: { type: integer }
"#;
        let m = import_yaml("sheetsish", yaml).expect("import");
        assert!(m.streams[0].header_row, "header_row carried through");
        assert_eq!(
            m.auth,
            Auth::GoogleServiceAccount {
                scopes: vec!["https://www.googleapis.com/auth/spreadsheets.readonly".into()],
            }
        );
    }

    /// Chained record accessors lower to dot-paths, and the
    /// `cursor_value_path`/`track_only` extensions land on the Incremental block —
    /// the GA4 columnar shape end-to-end at the import layer.
    #[test]
    fn imports_ga4_columnar_constructs() {
        // Chained accessor forms → dot-paths; junk stays rejected.
        assert_eq!(
            record_field("record['dimensionValues'][0]['value']").as_deref(),
            Some("dimensionValues.0.value")
        );
        assert_eq!(record_field("record.a.b").as_deref(), Some("a.b"));
        assert_eq!(record_field("record['a'] ~ 'x'"), None);

        let yaml = r#"
type: DeclarativeSource
streams:
  - type: DeclarativeStream
    name: traffic
    retriever:
      type: SimpleRetriever
      requester:
        type: HttpRequester
        url_base: "https://analyticsdata.googleapis.com"
        path: "/v1beta/properties/{{ config['property_id'] }}:runReport"
        http_method: POST
        request_body_json:
          dateRanges:
            - startDate: "{{ config['start_date'] }}"
              endDate: today
      record_selector:
        type: RecordSelector
        extractor:
          type: DpathExtractor
          field_path: ["rows"]
    incremental_sync:
      type: DatetimeBasedCursor
      cursor_field: date
      cursor_value_path: dimensionValues.0.value
      track_only: true
    transformations:
      - type: AddFields
        fields:
          - path: [date]
            value: "{{ record['dimensionValues'][0]['value'] }}"
    schema_loader:
      type: InlineSchemaLoader
      schema:
        type: object
        properties:
          date: { type: string }
"#;
        let m = import_yaml("ga4ish", yaml).expect("import");
        let inc = m.streams[0].incremental.as_ref().expect("incremental");
        assert_eq!(inc.cursor_field, "date");
        assert_eq!(inc.cursor_param, "", "track-only: nothing injected");
        assert_eq!(
            inc.cursor_value_path.as_deref(),
            Some("dimensionValues.0.value")
        );
        assert!(
            m.streams[0].mapping.ops.iter().any(|op| matches!(
                op,
                MappingOp::Compute { field, value: ComputeExpr::Field(p) }
                    if field == "date" && p == "dimensionValues.0.value"
            )),
            "AddFields chained accessor lowered to a dot-path Compute"
        );
    }

    /// The weir `SnowflakeKeypairAuthenticator` extension lowers to
    /// `Auth::SnowflakeKeypairJwt` with the three config-key refs preserved.
    #[test]
    fn imports_snowflake_keypair_authenticator() {
        let yaml = r#"
type: DeclarativeSource
streams:
  - type: DeclarativeStream
    name: rows
    retriever:
      type: SimpleRetriever
      requester:
        type: HttpRequester
        url_base: "https://{{ config['account'] }}.snowflakecomputing.com"
        path: "/api/v2/statements"
        authenticator:
          type: SnowflakeKeypairAuthenticator
          account: "{{ config['account'] }}"
          user: "{{ config['user'] }}"
          private_key: "{{ config['private_key'] }}"
      record_selector:
        type: RecordSelector
        extractor:
          type: DpathExtractor
          field_path: ["data"]
    schema_loader:
      type: InlineSchemaLoader
      schema:
        type: object
        properties:
          id: { type: string }
"#;
        let m = import_yaml("snowish", yaml).expect("import");
        assert_eq!(
            m.auth,
            Auth::SnowflakeKeypairJwt {
                account_key: "account".into(),
                user_key: "user".into(),
            }
        );
    }

    /// Airbyte `inject_into: body_json` on paginator + incremental options lowers to
    /// `InjectInto::Body`, with `field_path` joined as the dot-path.
    /// This was previously a silent mis-lowering (treated as query params).
    #[test]
    fn imports_body_json_injection() {
        let yaml = r#"
type: DeclarativeSource
streams:
  - type: DeclarativeStream
    name: search
    retriever:
      type: SimpleRetriever
      requester:
        type: HttpRequester
        url_base: "https://api.notion.example"
        path: "/v1/search"
        http_method: POST
        request_body_json:
          filter: { property: object, value: page }
      record_selector:
        type: RecordSelector
        extractor:
          type: DpathExtractor
          field_path: ["results"]
      paginator:
        type: DefaultPaginator
        pagination_strategy:
          type: CursorPagination
          cursor_value: "{{ response['next_cursor'] }}"
        page_token_option:
          type: RequestOption
          inject_into: body_json
          field_name: start_cursor
    incremental_sync:
      type: DatetimeBasedCursor
      cursor_field: last_edited_time
      start_time_option:
        type: RequestOption
        inject_into: body_json
        field_path: ["filter", "timestamp_after"]
    schema_loader:
      type: InlineSchemaLoader
      schema:
        type: object
        properties:
          id: { type: string }
          last_edited_time: { type: string, format: date-time }
"#;
        let m = import_yaml("notionish", yaml).expect("import");
        let s = &m.streams[0];
        match s.pagination.as_ref().expect("pagination") {
            Pagination::Cursor {
                cursor_path,
                token_param,
                cursor_record_field,
                stop_on_false_path,
                inject_into,
                ..
            } => {
                assert_eq!(cursor_path, "next_cursor");
                assert_eq!(token_param, "start_cursor");
                assert!(cursor_record_field.is_none());
                assert!(stop_on_false_path.is_none());
                assert_eq!(*inject_into, InjectInto::Body);
            }
            other => panic!("expected cursor pagination, got {other:?}"),
        }
        let inc = s.incremental.as_ref().expect("incremental");
        assert_eq!(inc.cursor_param, "filter.timestamp_after");
        assert_eq!(inc.inject_into, InjectInto::Body);
    }

    #[test]
    fn imports_stripe_style_last_record_cursor() {
        // : cursor-from-last-record + has_more stop + the strategy's page
        // size lower onto the runtime's Stripe-shaped pagination keys.
        let yaml = r#"
type: DeclarativeSource
streams:
  - type: DeclarativeStream
    name: charges
    retriever:
      type: SimpleRetriever
      requester:
        type: HttpRequester
        url_base: "https://api.stripe.com"
        path: "/v1/charges"
      record_selector:
        type: RecordSelector
        extractor:
          type: DpathExtractor
          field_path: ["data"]
      paginator:
        type: DefaultPaginator
        page_size_option:
          type: RequestOption
          inject_into: request_parameter
          field_name: limit
        page_token_option:
          type: RequestOption
          inject_into: request_parameter
          field_name: starting_after
        pagination_strategy:
          type: CursorPagination
          page_size: 100
          cursor_value: "{{ last_record['id'] }}"
          stop_condition: "{{ not response['has_more'] }}"
    schema_loader:
      type: InlineSchemaLoader
      schema:
        type: object
        properties:
          id: { type: string }
"#;
        let m = import_yaml("stripeish", yaml).expect("import");
        match m.streams[0].pagination.as_ref().expect("pagination") {
            Pagination::Cursor {
                cursor_path,
                token_param,
                cursor_record_field,
                stop_on_false_path,
                size_param,
                size,
                inject_into,
            } => {
                assert_eq!(cursor_path, "");
                assert_eq!(token_param, "starting_after");
                assert_eq!(cursor_record_field.as_deref(), Some("id"));
                assert_eq!(stop_on_false_path.as_deref(), Some("has_more"));
                assert_eq!(size_param.as_deref(), Some("limit"));
                assert_eq!(*size, Some(100));
                assert_eq!(*inject_into, InjectInto::Query);
            }
            other => panic!("expected cursor pagination, got {other:?}"),
        }
    }

    /// Polarity guard: the runtime's `stop_on_false_path` stops when the field is
    /// FALSE, but Airbyte's `stop_condition` stops when the template is TRUE — so a
    /// POSITIVE condition (`{{ response['is_last_page'] }}`) must NOT lower (it would
    /// invert: stop after page 1 exactly when it should continue). Only the negated
    /// idiom (`not …` / `… is false`) lowers.
    #[test]
    fn positive_stop_condition_is_dropped_not_inverted() {
        let paginator = |stop: &str| {
            format!(
                r#"
type: DeclarativeSource
streams:
  - type: DeclarativeStream
    name: things
    retriever:
      type: SimpleRetriever
      requester:
        type: HttpRequester
        url_base: "https://api.example.com"
        path: "/things"
      record_selector:
        type: RecordSelector
        extractor:
          type: DpathExtractor
          field_path: ["data"]
      paginator:
        type: DefaultPaginator
        page_token_option:
          type: RequestOption
          inject_into: request_parameter
          field_name: starting_after
        pagination_strategy:
          type: CursorPagination
          cursor_value: "{{{{ last_record['id'] }}}}"
          stop_condition: "{stop}"
    schema_loader:
      type: InlineSchemaLoader
      schema:
        type: object
        properties:
          id: {{ type: string }}
"#
            )
        };
        let stop_path = |yaml: &str| match import_yaml("ex", yaml).expect("import").streams[0]
            .pagination
            .as_ref()
            .expect("pagination")
        {
            Pagination::Cursor {
                stop_on_false_path, ..
            } => stop_on_false_path.clone(),
            other => panic!("expected cursor pagination, got {other:?}"),
        };

        // Positive condition: dropped (cursor-absence terminates), never inverted.
        assert_eq!(
            stop_path(&paginator("{{ response['is_last_page'] }}")),
            None
        );
        // Both negated idioms still lower.
        assert_eq!(
            stop_path(&paginator("{{ not response['has_more'] }}")).as_deref(),
            Some("has_more")
        );
        assert_eq!(
            stop_path(&paginator("{{ response['has_more'] is false }}")).as_deref(),
            Some("has_more")
        );
    }

    #[test]
    fn imports_transforms_to_mapping() {
        let yaml = r#"
type: DeclarativeSource
streams:
  - type: DeclarativeStream
    name: items
    retriever:
      type: SimpleRetriever
      requester:
        type: HttpRequester
        url_base: "https://api.example.com"
        path: "/items"
      record_selector:
        type: RecordSelector
        extractor:
          type: DpathExtractor
          field_path: []
        record_filter:
          type: RecordFilter
          condition: "{{ record['status'] == 'active' }}"
    transformations:
      - type: AddFields
        fields:
          - path: ["source"]
            value: "manual"
          - path: ["name_copy"]
            value: "{{ record['name'] }}"
      - type: RemoveFields
        field_pointers:
          - ["secret"]
    schema_loader:
      type: InlineSchemaLoader
      schema:
        type: object
        properties:
          id: { type: integer }
"#;
        let m = import_yaml("xf", yaml).expect("import");
        assert_eq!(
            m.streams[0].mapping.ops,
            vec![
                MappingOp::Compute {
                    field: "source".into(),
                    value: ComputeExpr::Const("manual".into())
                },
                MappingOp::Compute {
                    field: "name_copy".into(),
                    value: ComputeExpr::Field("name".into())
                },
                MappingOp::Drop {
                    fields: vec!["secret".into()]
                },
                MappingOp::Filter {
                    field: "status".into(),
                    op: CompareOp::Eq,
                    value: "active".into()
                },
            ]
        );
    }

    #[test]
    fn analyze_reports_complex_transforms() {
        let yaml = r#"
type: DeclarativeSource
streams:
  - type: DeclarativeStream
    name: items
    retriever:
      type: SimpleRetriever
      requester:
        type: HttpRequester
        url_base: "https://api.example.com"
        path: "/items"
      record_selector:
        type: RecordSelector
        extractor:
          type: DpathExtractor
          field_path: []
        record_filter:
          type: RecordFilter
          condition: "{{ record['x'] > 5 and record['y'] < 10 }}"
    transformations:
      - type: AddFields
        fields:
          - path: ["combo"]
            value: "{{ record['a'] ~ record['b'] }}"
    schema_loader:
      type: InlineSchemaLoader
      schema:
        type: object
        properties:
          id: { type: integer }
"#;
        let r = analyze(yaml);
        assert!(
            r.unsupported.iter().any(|u| u.contains("AddFields")),
            "complex AddFields reported: {:?}",
            r.unsupported
        );
        assert!(
            r.unsupported.iter().any(|u| u.contains("record_filter")),
            "compound record_filter reported: {:?}",
            r.unsupported
        );
    }

    #[test]
    fn analyze_treats_oauth_as_supported() {
        // OAuth is injected host-side (, proven by the wasm wire test), so
        // it's a config requirement — not a coverage gap.
        let yaml = manifest_with_auth(
            "        authenticator:\n          type: OAuthAuthenticator\n          \
             token_refresh_endpoint: \"https://api.example.com/oauth/token\"\n          \
             client_id: \"{{ config['client_id'] }}\"\n          \
             client_secret: \"{{ config['client_secret'] }}\"\n          \
             refresh_token: \"{{ config['refresh_token'] }}\"",
        );
        let r = analyze(&yaml);
        assert!(
            !r.unsupported
                .iter()
                .any(|u| u.contains("oauth") || u.contains("auth")),
            "oauth is supported host-side, not a gap: {:?}",
            r.unsupported
        );
        assert_eq!(r.tier, "A");
    }

    #[test]
    fn imports_list_partition_router() {
        let yaml = r#"
type: DeclarativeSource
streams:
  - type: DeclarativeStream
    name: items
    retriever:
      type: SimpleRetriever
      requester:
        type: HttpRequester
        url_base: "https://api.example.com"
        path: "/cat/{{ stream_partition.category }}/items"
      record_selector:
        type: RecordSelector
        extractor:
          type: DpathExtractor
          field_path: []
      partition_router:
        type: ListPartitionRouter
        cursor_field: category
        values: ["a", "b", "c"]
    schema_loader:
      type: InlineSchemaLoader
      schema:
        type: object
        properties:
          id: { type: integer }
"#;
        let m = import_yaml("listy", yaml).expect("import");
        assert_eq!(
            m.streams[0].partition,
            Some(PartitionRouter::List {
                field: "category".into(),
                values: vec!["a".into(), "b".into(), "c".into()],
            })
        );
    }

    #[test]
    fn imports_substream_partition_router() {
        let yaml = r#"
type: DeclarativeSource
streams:
  - type: DeclarativeStream
    name: comments
    retriever:
      type: SimpleRetriever
      requester:
        type: HttpRequester
        url_base: "https://api.example.com"
        path: "/posts/{{ stream_partition.post_id }}/comments"
      record_selector:
        type: RecordSelector
        extractor:
          type: DpathExtractor
          field_path: []
      partition_router:
        type: SubstreamPartitionRouter
        parent_stream_configs:
          - type: ParentStreamConfig
            parent_key: id
            partition_field: post_id
            stream:
              type: DeclarativeStream
              name: posts
              retriever:
                type: SimpleRetriever
                requester:
                  type: HttpRequester
                  url_base: "https://api.example.com"
                  path: "/posts"
                record_selector:
                  type: RecordSelector
                  extractor:
                    type: DpathExtractor
                    field_path: ["data"]
    schema_loader:
      type: InlineSchemaLoader
      schema:
        type: object
        properties:
          id: { type: integer }
"#;
        let m = import_yaml("substreamy", yaml).expect("import");
        assert_eq!(
            m.streams[0].partition,
            Some(PartitionRouter::Substream {
                field: "post_id".into(),
                parent_path: "/posts".into(),
                parent_record_selector: Some("data".into()),
                parent_key: "id".into(),
            })
        );
    }
}
