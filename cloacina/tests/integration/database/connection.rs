/*
 *  Copyright 2025 Colliery Software
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

use url::Url;

#[test]
fn test_url_parsing_basic() {
    // Test that we can parse PostgreSQL URLs (using the url crate directly)
    let db_url = "postgresql://user:pass@localhost:5432/test_db";
    let result = Url::parse(db_url);
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert_eq!(parsed.host_str(), Some("localhost"));
    assert_eq!(parsed.port(), Some(5432));
    assert_eq!(parsed.path(), "/test_db");
    assert_eq!(parsed.username(), "user");
    assert_eq!(parsed.password(), Some("pass"));
}

#[test]
fn test_url_parsing_without_password() {
    let db_url = "postgresql://user@localhost:5432/test_db";
    let result = Url::parse(db_url);
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert_eq!(parsed.username(), "user");
    assert_eq!(parsed.password(), None);
}

#[test]
fn test_url_parsing_with_default_port() {
    let db_url = "postgresql://user:pass@localhost/test_db";
    let result = Url::parse(db_url);
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert_eq!(parsed.port(), None); // No explicit port means None from url crate
                                     // Note: port_or_known_default() only works for http/https, not postgresql
    assert_eq!(parsed.scheme(), "postgresql");
}

#[test]
fn test_invalid_database_urls() {
    let invalid_urls = vec!["not-a-url-at-all", "://missing-scheme", ""];

    for url in invalid_urls {
        let result = Url::parse(url);
        assert!(result.is_err(), "Expected error for URL: {}", url);
    }

    // Note: "invalid-scheme://url" is actually valid according to url crate
    // since any scheme is allowed. Test that it parses but has unknown scheme.
    let result = Url::parse("invalid-scheme://url");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().scheme(), "invalid-scheme");
}

#[test]
fn test_database_connection_construction() {
    // Test that we can construct database URLs for the Database constructor
    let base_url = "postgresql://user:pass@localhost:5432";
    let database_name = "test_db";

    let mut url = Url::parse(base_url).unwrap();
    url.set_path(database_name);

    let final_url = url.as_str();
    assert!(final_url.contains("test_db"));
    assert!(final_url.contains("postgresql://"));
}

#[test]
fn test_database_url_modification() {
    // Test URL modification as done in Database::new
    let base_url = "postgresql://user:pass@localhost:5432";
    let database_name = "my_database";

    let mut url = Url::parse(base_url).expect("Invalid base URL");
    url.set_path(database_name);

    let modified_url = url.as_str();
    assert!(modified_url.ends_with("/my_database"));
    assert_eq!(url.path(), "/my_database");
}
