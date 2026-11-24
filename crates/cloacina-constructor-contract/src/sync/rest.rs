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

//! The config the REST runtime's guests read (COLLIERY-I-0634 /
//! CLOACINA-T-0988).
//!
//! A declarative connector is a connector definition run on one shared REST
//! runtime. The host compiles a definition, a stream and the user's config into
//! a [`RestConfig`] (or a [`RestDestConfig`]) and binds it as the guest's
//! `Config` JSON; the guest deserializes the same type. One type read on both
//! sides replaces weir's string keys, and an unknown field is an error.
//!
//! The values here are rendered: `{{ config['...'] }}` templates are already
//! replaced. What stays templated is what only the guest knows at run time:
//! `{{ stream_partition.<field> }}` in a path, and `{{ record.<field> }}` in a
//! destination path.
//!
//! Auth is not here. The host adds the credential to each request.

use serde::{Deserialize, Serialize};

/// Where a request parameter is carried: the query string, or a dot-path field
/// of the JSON request body (which needs a POST).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InjectInto {
    #[default]
    Query,
    Body,
}

/// The HTTP method of a source request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
}

/// How a source walks the pages of a stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Pagination {
    /// `?page=N&per_page=S`, from 1; stops at a short or empty page.
    Page {
        page_param: String,
        size_param: String,
        size: u32,
        #[serde(default)]
        inject_into: InjectInto,
    },
    /// `?offset=N&limit=S`; stops at a short or empty page.
    Offset {
        offset_param: String,
        limit_param: String,
        size: u32,
        #[serde(default)]
        inject_into: InjectInto,
    },
    /// An opaque next-page token, read at `cursor_path` of the response (or from
    /// `cursor_record_field` of the page's last record) and sent as
    /// `token_param`; stops when there is no token, or when
    /// `stop_on_false_path` reads `false`.
    Cursor {
        #[serde(default)]
        cursor_path: String,
        token_param: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cursor_record_field: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stop_on_false_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        size_param: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        size: Option<u32>,
        #[serde(default)]
        inject_into: InjectInto,
    },
    /// Follow `Link: <url>; rel="next"` until it is absent (GitHub style).
    LinkHeader,
}

/// An incremental read by a cursor field of the records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Incremental {
    /// The record field whose largest value is the cursor.
    pub cursor_field: String,
    /// The parameter the cursor is sent as. Empty means track only: the
    /// cursor advances, but nothing is sent (for APIs re-read over a window).
    #[serde(default)]
    pub cursor_param: String,
    /// A dot-path in the raw record to read the cursor from, when it differs
    /// from `cursor_field`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_value_path: Option<String>,
    /// The lower bound of a first read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_value: Option<String>,
    /// An upper bound sent on every request, as `end_param`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_param: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_value: Option<String>,
    #[serde(default)]
    pub inject_into: InjectInto,
}

/// Reads a stream once per slice. A slice's value replaces
/// `{{ stream_partition.<field> }}` in the request path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PartitionRouter {
    /// One slice per value.
    List { field: String, values: Vec<String> },
    /// One slice per record of a parent read: `parent_key` of each record of
    /// `parent_path`, found at `parent_record_selector`.
    Substream {
        field: String,
        parent_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent_record_selector: Option<String>,
        parent_key: String,
    },
}

fn default_max_retries() -> u32 {
    4
}

fn default_max_pages() -> u64 {
    1000
}

/// What the `rest` source guest reads: one stream of a connector definition,
/// rendered against the user's config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestConfig {
    pub base_url: String,
    pub path: String,
    /// A dot-path to the records array in the response; `None` means the body
    /// is the array.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_selector: Option<String>,
    #[serde(default)]
    pub http_method: HttpMethod,
    /// The JSON body of a POST.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_body: Option<String>,
    /// Static request headers. Auth headers are the host's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub request_headers: Vec<(String, String)>,
    /// The records are row arrays whose first row names the fields.
    #[serde(default)]
    pub header_row: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pagination: Option<Pagination>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incremental: Option<Incremental>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition: Option<PartitionRouter>,
    /// Retries of a 429, a 5xx or a transport failure. Default 4.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// Pages one read may fetch before it checkpoints and stops. Default 1000.
    #[serde(default = "default_max_pages")]
    pub max_pages: u64,
}

/// What the `rest-dest` guest reads: one object of a destination definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestDestConfig {
    pub base_url: String,
    /// The object's path; `{{ record.<field> }}` is replaced per record, its
    /// value percent-encoded.
    pub path: String,
    /// The write method. `None` takes it from the stream's write mode: `PATCH`
    /// for upsert (create or update the keyed path), `POST` otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// The business key templated into `path`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upsert_key: Option<String>,
    /// Record field to API property. Empty means the record as it is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub field_map: Vec<(String, String)>,
    /// Nest the fields under this key (HubSpot's `properties`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_wrap: Option<String>,
    /// The path a CDC delete goes to, templated like `path`. Without it, a
    /// delete is a dead-letter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delete_path: Option<String>,
    /// The delete method. Default `DELETE`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delete_method: Option<String>,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rest_config_round_trips_and_defaults() {
        let c: RestConfig = serde_json::from_str(
            r#"{"base_url": "https://api.example.com", "path": "/posts",
                "pagination": {"kind": "page", "page_param": "page", "size_param": "per_page", "size": 50}}"#,
        )
        .unwrap();
        assert_eq!(c.http_method, HttpMethod::Get);
        assert_eq!(c.max_retries, 4);
        assert_eq!(c.max_pages, 1000);
        let back: RestConfig = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn an_unknown_field_is_an_error() {
        let r = serde_json::from_str::<RestConfig>(
            r#"{"base_url": "x", "path": "/p", "page_size": 10}"#,
        );
        assert!(r.unwrap_err().to_string().contains("page_size"));
        let r = serde_json::from_str::<RestConfig>(
            r#"{"base_url": "x", "path": "/p", "pagination": {"kind": "page", "page_param": "p", "size_param": "s", "size": 1, "sise": 2}}"#,
        );
        assert!(r.is_err());
    }

    #[test]
    fn a_dest_config_leaves_the_method_to_the_write_mode() {
        let c: RestDestConfig =
            serde_json::from_str(r#"{"base_url": "x", "path": "/contacts"}"#).unwrap();
        assert_eq!(c.method, None);
        assert_eq!(c.delete_path, None);
        assert_eq!(c.max_retries, 4);
        assert!(
            serde_json::from_str::<RestDestConfig>(
                r#"{"base_url": "x", "path": "/c", "batch_size": 10}"#
            )
            .is_err()
        );
    }
}
