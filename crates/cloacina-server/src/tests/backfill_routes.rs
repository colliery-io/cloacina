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

//! Router tests of the backfill routes and of the interval check on a manual
//! run (CLOACINA-T-1020). They need Postgres, like the other router tests of
//! this crate.
//!
//! The backfills here have a range in the future, so the scheduler of the
//! shared test runner takes none of their intervals.

use super::*;

fn json_request(
    method: &str,
    uri: &str,
    token: &str,
    body: serde_json::Value,
) -> axum::http::Request<Body> {
    axum::http::Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn get_request(uri: &str, token: &str) -> axum::http::Request<Body> {
    axum::http::Request::builder()
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

/// The top of an hour, `days` days from now.
fn future_hour(days: i64) -> chrono::DateTime<chrono::Utc> {
    let now = chrono::Utc::now().timestamp();
    chrono::DateTime::from_timestamp(now / 3600 * 3600, 0).unwrap() + chrono::Duration::days(days)
}

/// Create, list, inspect and cancel a backfill, and a retry with nothing to
/// retry.
#[tokio::test]
#[serial]
async fn backfill_create_list_inspect_cancel() {
    let state = test_state().await;
    let token = create_test_api_key(&state).await;
    let workflow = format!("t1020_bf_{}", uuid::Uuid::new_v4().simple());
    let base = format!("/v1/tenants/public/workflows/{workflow}/backfills");
    let start = future_hour(30);
    let end = start + chrono::Duration::hours(4);

    let (status, created) = send_request(
        build_router(state.clone()),
        json_request(
            "POST",
            &base,
            &token,
            serde_json::json!({
                "cron": "0 * * * *",
                "start": start.to_rfc3339(),
                "end": end.to_rfc3339(),
                "max_active_runs": 3,
                "params": {"region": "eu"},
            }),
        ),
    )
    .await;
    assert!(status.is_success(), "create failed: {status} {created}");
    let id = created["id"].as_str().expect("id").to_string();
    assert_eq!(created["workflow_name"], workflow.as_str());
    assert_eq!(created["cron_expression"], "0 * * * *");
    assert_eq!(created["timezone"], "UTC");
    assert_eq!(created["max_active_runs"], 3);
    assert_eq!(created["params"]["region"], "eu");
    assert_eq!(created["status"], "running");
    assert_eq!(created["progress"]["total"], 5);
    assert_eq!(created["progress"]["remaining"], 5);
    assert_eq!(created["progress"]["completed"], 0);
    assert_eq!(created["progress"]["failed"], 0);
    assert_eq!(created["progress"]["running"], 0);
    assert!(created["progress"].get("last_interval_end").is_none());
    assert_eq!(
        created["progress"]["next_interval_end"],
        start.to_rfc3339().as_str()
    );

    // The list of the workflow holds it; the list of live schedules does not.
    let (status, list) =
        send_request(build_router(state.clone()), get_request(&base, &token)).await;
    assert_eq!(status, StatusCode::OK, "{list}");
    assert_eq!(list["total"], 1);
    assert_eq!(list["items"][0]["id"], id.as_str());
    let (status, triggers) = send_request(
        build_router(state.clone()),
        get_request("/v1/tenants/public/triggers?limit=1000", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{triggers}");
    assert!(
        triggers["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["id"] != id.as_str()),
        "a backfill is not in the list of live schedules"
    );

    // Inspect.
    let (status, detail) = send_request(
        build_router(state.clone()),
        get_request(&format!("{base}/{id}"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{detail}");
    assert_eq!(detail["backfill"]["id"], id.as_str());
    assert_eq!(detail["backfill"]["progress"]["total"], 5);
    assert_eq!(detail["failed_intervals"].as_array().unwrap().len(), 0);
    assert_eq!(detail["failed_intervals_truncated"], false);

    // Not found: an unknown id, and the id under another workflow.
    let unknown = uuid::Uuid::new_v4();
    for uri in [
        format!("{base}/{unknown}"),
        format!("/v1/tenants/public/workflows/other_workflow/backfills/{id}"),
    ] {
        let (status, body) =
            send_request(build_router(state.clone()), get_request(&uri, &token)).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
        assert_eq!(body["code"], "backfill_not_found");
    }
    let (status, body) = send_request(
        build_router(state.clone()),
        get_request(&format!("{base}/not-a-uuid"), &token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    // A retry needs a failed interval.
    let (status, body) = send_request(
        build_router(state.clone()),
        json_request(
            "POST",
            &format!("{base}/{id}/retry"),
            &token,
            serde_json::json!({}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["code"], "backfill_has_no_failed_interval");

    // Cancel, twice: the second changes nothing.
    for _ in 0..2 {
        let (status, cancelled) = send_request(
            build_router(state.clone()),
            json_request(
                "POST",
                &format!("{base}/{id}/cancel"),
                &token,
                serde_json::json!({}),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{cancelled}");
        assert_eq!(cancelled["status"], "cancelled");
        assert_eq!(cancelled["progress"]["remaining"], 5);
    }

    // The audit record has the create and the cancel.
    let (status, audit) = send_request(
        build_router(state.clone()),
        get_request("/v1/tenants/public/audit?limit=200", &token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{audit}");
    let actions: Vec<&str> = audit["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["target"] == id.as_str())
        .filter_map(|row| row["action"].as_str())
        .collect();
    assert!(actions.contains(&"backfill.create"), "{actions:?}");
    assert!(actions.contains(&"backfill.cancel"), "{actions:?}");

    // Leave no schedule behind in the shared test database.
    let uuid = uuid::Uuid::parse_str(&id).unwrap();
    cloacina::dal::DAL::new(state.database.clone())
        .schedule()
        .delete(cloacina::UniversalUuid(uuid))
        .await
        .expect("delete backfill row");
}

/// A request that cannot make a backfill is a 400 with a message.
#[tokio::test]
#[serial]
async fn backfill_create_rejects_bad_requests() {
    let state = test_state().await;
    let token = create_test_api_key(&state).await;
    let workflow = format!("t1020_bf_{}", uuid::Uuid::new_v4().simple());
    let base = format!("/v1/tenants/public/workflows/{workflow}/backfills");
    let start = future_hour(30).to_rfc3339();
    let end = (future_hour(30) + chrono::Duration::hours(2)).to_rfc3339();

    let cases = [
        (
            serde_json::json!({"cron": "0 * * * *", "start": "yesterday", "end": end}),
            "invalid_backfill",
            "start must be an RFC 3339 timestamp",
        ),
        (
            serde_json::json!({"cron": "0 * * * *", "start": end, "end": start}),
            "invalid_backfill",
            "is before start",
        ),
        (
            serde_json::json!({"cron": "not cron", "start": start, "end": end}),
            "invalid_cron",
            "invalid cron expression",
        ),
        (
            serde_json::json!({"cron": "0 * * * *", "timezone": "Mars/Base", "start": start, "end": end}),
            "invalid_cron",
            "invalid cron expression or timezone",
        ),
        (
            serde_json::json!({"cron": "0 * * * *", "start": start, "end": end, "max_active_runs": 0}),
            "invalid_backfill",
            "max_active_runs 0",
        ),
        (
            serde_json::json!({"cron": "0 0 1 1 *", "start": start, "end": start}),
            "invalid_backfill",
            "holds no interval",
        ),
        (
            serde_json::json!({"cron": "0 * * * *", "start": start, "end": end, "params": [1]}),
            "backfill_params_invalid",
            "params must be a JSON object",
        ),
        // No cron in the body, and the workflow has no cron schedule.
        (
            serde_json::json!({"start": start, "end": end}),
            "invalid_backfill",
            "cron is required",
        ),
    ];
    for (body, code, message) in cases {
        let (status, response) = send_request(
            build_router(state.clone()),
            json_request("POST", &base, &token, body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}: {response}");
        assert_eq!(response["code"], code, "{body}: {response}");
        assert!(
            response["error"]
                .as_str()
                .unwrap_or_default()
                .contains(message),
            "{body}: {response}"
        );
    }
}

/// A key with the read role lists and inspects, and cannot create, cancel
/// or retry.
#[tokio::test]
#[serial]
async fn backfill_writes_need_the_write_role() {
    let state = test_state().await;
    let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();
    cloacina::dal::DAL::new(state.database.clone())
        .api_keys()
        .create_key(
            &hash,
            &format!("backfill-read-{}", uuid::Uuid::new_v4().simple()),
            Some("public"),
            false,
            "read",
        )
        .await
        .expect("create read key");
    let base = "/v1/tenants/public/workflows/t1020_any/backfills";
    let id = uuid::Uuid::new_v4();

    let (status, body) =
        send_request(build_router(state.clone()), get_request(base, &plaintext)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = send_request(
        build_router(state.clone()),
        get_request(&format!("{base}/{id}"), &plaintext),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");

    for uri in [
        base.to_string(),
        format!("{base}/{id}/cancel"),
        format!("{base}/{id}/retry"),
    ] {
        let (status, body) = send_request(
            build_router(state.clone()),
            json_request(
                "POST",
                &uri,
                &plaintext,
                serde_json::json!({"cron": "0 * * * *", "start": "2026-01-01T00:00:00Z", "end": "2026-01-01T01:00:00Z"}),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{uri}: {body}");
    }
}

/// A manual run that sets the interval keys must set two timestamps, the end
/// after the start. The check comes before any work for the run.
#[tokio::test]
#[serial]
async fn manual_run_checks_the_interval_keys() {
    let state = test_state().await;
    let token = create_test_api_key(&state).await;
    let uri = "/v1/tenants/public/workflows/t1020_no_such_workflow/execute";

    let cases = [
        (
            serde_json::json!({"interval_start": "2026-01-31", "interval_end": "2026-01-31T06:00:00Z"}),
            "interval_start must be an RFC 3339 timestamp",
        ),
        (
            serde_json::json!({"interval_start": "2026-01-31T06:00:00Z", "interval_end": "2026-01-31T05:00:00Z"}),
            "interval_end must be after interval_start",
        ),
        (
            serde_json::json!({"interval_start": "2026-01-31T06:00:00Z"}),
            "set both or none",
        ),
        (
            serde_json::json!({"interval_start": 5, "interval_end": "2026-01-31T05:00:00Z"}),
            "in a string",
        ),
    ];
    for (context, message) in cases {
        let (status, body) = send_request(
            build_router(state.clone()),
            json_request(
                "POST",
                uri,
                &token,
                serde_json::json!({ "context": context }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{context}: {body}");
        assert_eq!(body["code"], "invalid_interval", "{context}: {body}");
        assert!(
            body["error"].as_str().unwrap_or_default().contains(message),
            "{context}: {body}"
        );
    }

    // A good interval passes the check: the answer is about the workflow.
    let (_, body) = send_request(
        build_router(state.clone()),
        json_request(
            "POST",
            uri,
            &token,
            serde_json::json!({ "context": {
                "interval_start": "2026-01-31T05:00:00Z",
                "interval_end": "2026-01-31T06:00:00Z",
            }}),
        ),
    )
    .await;
    assert_ne!(body["code"], "invalid_interval", "{body}");
}
