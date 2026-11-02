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

//! The REST source against the mock JSON API (COLLIERY-I-0634 /
//! CLOACINA-T-0990): its conformance file, through the real connector host.
//! Needs only the `wasm32-wasip2` target.

use std::collections::BTreeMap;

use cloacina_testing::connectors::conformance;
use cloacina_testing::connectors::rest_mock::RestMock;

mod support;
use support::{ConnectorHost, repo};

#[test]
fn rest_conforms() {
    let mock = RestMock::start();
    let vars = BTreeMap::from([
        ("MOCK_URL".to_string(), mock.base_url()),
        ("MOCK_GRANT".to_string(), mock.grant()),
    ]);
    let report = conformance::run_with(
        &repo("connectors/rest/conformance.toml"),
        &vars,
        &ConnectorHost::new("rest"),
        Some(&mock),
    );
    report.assert_passed();
    // Every request to the guarded endpoint carried the host's header, and
    // the one refused carried the wrong token's.
    let pages = mock.requests_to("/pages");
    assert!(!pages.is_empty());
    let refused = pages
        .iter()
        .filter(|r| r.headers.get("authorization").map(String::as_str) != Some("Bearer t0k"))
        .count();
    assert_eq!(refused, 1, "only the wrong-token check is refused");
    println!("{report}");
}
