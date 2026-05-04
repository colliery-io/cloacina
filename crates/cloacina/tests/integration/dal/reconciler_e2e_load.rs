/*
 *  Copyright 2026 Colliery Software
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

//! T-0553 / T-0554 Phase 2 deferred AC — full reconciler boot exercising
//! the cross-package fan-out and reverse-order unload pipeline.
//!
//! Drives `RegistryReconciler::reconcile` end-to-end against a live
//! `ComputationGraphScheduler` + scoped `Runtime` + DAL-backed
//! `WorkflowRegistry`. Two fixtures: `reactor-only-rust` (publishes
//! `shared_rx` with accumulators α/β) and `reactor-subscriber-rust`
//! (binds a CG to `shared_rx` by string name). The test asserts:
//!
//! 1. Both packages load through `reconcile` in registration order.
//! 2. The publisher's reactor lands in the scheduler with the right
//!    accumulator contract.
//! 3. The subscriber's CG is bound to the publisher's reactor via the
//!    cross-package fan-out path (T-0544 M2 idempotent contract match).
//! 4. An event sent into the publisher's accumulator endpoint reaches
//!    the dispatcher without error (proves the wiring is live).
//! 5. Reverse-order unload: deleting the publisher first while the
//!    subscriber is still bound returns the T-0554 Phase 2 reject-with-
//!    bound-subscribers error; deleting the subscriber first then
//!    re-reconciling unloads cleanly; then deleting the publisher and
//!    re-reconciling tears the reactor down.
//!
//! NOTE: `mixed-rust` (single cdylib carrying every primitive,
//! including a workflow with `triggers = ["mixed_trigger"]`) cannot
//! load through this reconciler path today. The trigger validation
//! requires `runtime.get_trigger("mixed_trigger")` to succeed, but
//! `Runtime::seed_from_inventory` does not see entries submitted by
//! independently-compiled cdylibs (each fixture has its own
//! `[workspace]`, so its `cloacina-workflow-plugin` is a separate
//! compilation with distinct linker symbols). The daemon's
//! T-0553 trigger registration loop has the same dependency. A proper
//! fix requires constructing trigger impls from FFI metadata rather
//! than expecting host-process inventory to span cdylibs — tracked
//! separately so this e2e suite can ship.

use crate::fixtures::get_or_init_fixture;
use cloacina::computation_graph::registry::EndpointRegistry;
use cloacina::computation_graph::scheduler::ComputationGraphScheduler;
use cloacina::registry::reconciler::{ReconcilerConfig, RegistryReconciler};
use cloacina::registry::traits::WorkflowRegistry;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use cloacina::Runtime;
use serial_test::serial;
use std::sync::Arc;
use tokio::sync::watch;

fn pack_fixture(fixture_name: &str) -> Vec<u8> {
    let cargo_manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let workspace_root = std::path::PathBuf::from(&cargo_manifest_dir)
        .parent()
        .expect("crate parent")
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let project_path = workspace_root.join(format!("examples/fixtures/{}", fixture_name));
    assert!(
        project_path.join("package.toml").exists(),
        "fixture {} missing package.toml at {}",
        fixture_name,
        project_path.display()
    );

    let temp_dir = tempfile::TempDir::new().expect("tempdir");
    let archive_path = temp_dir.path().join(format!(
        "{}-{}.cloacina",
        fixture_name,
        uuid::Uuid::new_v4()
    ));
    fidius_core::package::pack_package(&project_path, Some(&archive_path))
        .expect("fidius pack_package");
    std::fs::read(&archive_path).expect("read archive")
}

fn read_fixture_dylib(fixture_name: &str) -> Vec<u8> {
    let cargo_manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let workspace_root = std::path::PathBuf::from(&cargo_manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let lib_basename = fixture_name.replace('-', "_");
    let ext = if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    };
    for profile in &["debug", "release"] {
        let path = workspace_root
            .join(format!("examples/fixtures/{}", fixture_name))
            .join("target")
            .join(profile)
            .join(format!("lib{}.{}", lib_basename, ext));
        if path.exists() {
            return std::fs::read(&path).expect("read dylib");
        }
    }
    panic!(
        "no prebuilt dylib for fixture '{}'; run `angreal test integration` first",
        fixture_name
    );
}

#[tokio::test]
#[serial]
async fn reconciler_loads_cross_package_publisher_subscriber_end_to_end() {
    let publisher_archive = pack_fixture("reactor-only-rust");
    let publisher_dylib = read_fixture_dylib("reactor-only-rust");
    let subscriber_archive = pack_fixture("reactor-subscriber-rust");
    let subscriber_dylib = read_fixture_dylib("reactor-subscriber-rust");

    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let storage_writer = fixture.create_storage();
    let mut registry_writer = dal.workflow_registry(storage_writer);

    // Register publisher first so its reactor exists by the time the
    // subscriber's CG load tries to bind. Ordering through the DAL
    // matters: the reconciler's reconcile() processes packages in the
    // order returned by list_workflows, which is registration order.
    let publisher_id = registry_writer
        .register_workflow_package(publisher_archive.clone())
        .await
        .expect("register publisher");
    registry_writer.claim_next_build().await.expect("claim 1");
    registry_writer
        .mark_build_success(publisher_id, publisher_dylib.clone())
        .await
        .expect("mark publisher built");

    let subscriber_id = registry_writer
        .register_workflow_package(subscriber_archive.clone())
        .await
        .expect("register subscriber");
    registry_writer.claim_next_build().await.expect("claim 2");
    registry_writer
        .mark_build_success(subscriber_id, subscriber_dylib.clone())
        .await
        .expect("mark subscriber built");

    let storage_reader = fixture.create_storage();
    let registry_reader: Arc<dyn WorkflowRegistry> = Arc::new(
        WorkflowRegistryImpl::new(storage_reader, fixture.get_database())
            .expect("WorkflowRegistryImpl::new"),
    );

    let endpoint_registry = EndpointRegistry::new();
    let scheduler = Arc::new(ComputationGraphScheduler::new(endpoint_registry.clone()));
    let runtime = Arc::new(Runtime::empty());

    let config = ReconcilerConfig {
        default_tenant_id: "public".to_string(),
        ..ReconcilerConfig::default()
    };
    let (_shutdown_tx, shutdown_rx) = watch::channel(false);
    let reconciler = RegistryReconciler::new(registry_reader, config, shutdown_rx)
        .expect("reconciler new")
        .with_runtime(runtime.clone())
        .with_graph_scheduler(scheduler.clone());

    // ============ Phase 1: load both packages ============
    let result = reconciler.reconcile().await.expect("reconcile load");
    assert!(
        result.packages_failed.is_empty(),
        "no failures expected on initial reconcile; got {:?}",
        result.packages_failed
    );
    assert_eq!(
        result.packages_loaded.len(),
        2,
        "both publisher + subscriber should load on initial reconcile"
    );

    // Publisher's reactor must be in the scheduler with the declared
    // accumulator contract.
    let reactor_accs = scheduler
        .reactor_accumulator_names("shared_rx")
        .await
        .expect("shared_rx must be loaded after reconcile");
    assert_eq!(
        reactor_accs,
        vec!["alpha".to_string(), "beta".to_string()],
        "shared_rx publishes (alpha, beta)"
    );

    // Subscriber's CG must be bound to shared_rx — list_graphs reflects
    // active subscribers via the graph_to_reactor map.
    let graphs = scheduler.list_graphs().await;
    assert!(
        graphs.iter().any(|g| g.name == "subscriber_graph"),
        "subscriber_graph must be loaded; got {:?}",
        graphs.iter().map(|g| &g.name).collect::<Vec<_>>()
    );

    // ============ Phase 2: live event delivery ============
    // Push raw bytes into the publisher's `alpha` accumulator endpoint
    // through the EndpointRegistry — same surface the WebSocket bridge
    // uses in production. The CG's graph_fn fires from the cdylib;
    // there is no host-side counter we can read from a packaged
    // graph, so the assertion is "the dispatcher accepts the event
    // without crashing the reactor task" — verified by checking the
    // scheduler still reports the reactor's accumulator contract
    // post-event.
    let event_bytes =
        serde_json::to_vec(&serde_json::json!({"value": 42.0})).expect("serialize event");
    endpoint_registry
        .send_to_accumulator("alpha", event_bytes)
        .await
        .expect("event send to accumulator should succeed");
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert!(
        scheduler
            .reactor_accumulator_names("shared_rx")
            .await
            .is_some(),
        "reactor must still be live after event delivery"
    );

    // ============ Phase 3: reverse-order unload — wrong order ============
    // Deleting the publisher while the subscriber is still bound must
    // surface T-0544 M4's reject-with-subscribers guard through the
    // T-0554 Phase 2 reverse-order pipeline. The reconciler reports
    // the rejection via the per-package failure list rather than
    // failing the whole reconcile (continue_on_package_error = true by
    // default).
    registry_writer
        .unregister_workflow_package_by_id(publisher_id)
        .await
        .expect("unregister publisher");
    let bad_unload = reconciler
        .reconcile()
        .await
        .expect("reconcile after publisher delete");
    assert!(
        bad_unload
            .packages_failed
            .iter()
            .any(|(_, msg)| msg.contains("shared_rx")
                && (msg.contains("subscribers") || msg.contains("subscriber"))),
        "reverse-order unload must reject publisher with subscribers-bound message; failures={:?}",
        bad_unload.packages_failed
    );
    assert!(
        scheduler
            .reactor_accumulator_names("shared_rx")
            .await
            .is_some(),
        "reactor must still be live after rejected publisher unload"
    );

    // ============ Phase 4: reverse-order unload — right order ============
    // Drop the subscriber first; reactor's last subscriber goes away
    // and the next reconcile attempt clears the publisher.
    registry_writer
        .unregister_workflow_package_by_id(subscriber_id)
        .await
        .expect("unregister subscriber");
    let _ = reconciler
        .reconcile()
        .await
        .expect("reconcile after subscriber delete");
    let _ = reconciler
        .reconcile()
        .await
        .expect("reconcile second pass — picks up publisher unload retry");
    assert!(
        scheduler
            .reactor_accumulator_names("shared_rx")
            .await
            .is_none(),
        "reactor must be torn down once publisher is unloaded after subscriber"
    );

    scheduler.shutdown_all().await;
}

/// Closes the gap documented earlier: mixed-rust packs every primitive
/// into one cdylib, including a workflow that subscribes to a trigger
/// declared in the same cdylib (`triggers = ["mixed_trigger"]`). Before
/// the FFI Trigger bridge landed, this load failed at
/// `validate_workflow_trigger_subscriptions` because
/// `Runtime::seed_from_inventory` doesn't see entries from
/// independently-compiled cdylibs (each fixture is its own
/// `[workspace]`). With the bridge, `step_load_custom_triggers` now
/// dlopens the cdylib and registers an `FfiTriggerImpl` per declared
/// trigger BEFORE workflow validation runs, so the validation finds
/// the trigger via `runtime.get_trigger`. This test asserts:
///
/// 1. mixed-rust loads through `reconcile()` without errors.
/// 2. `mixed_trigger` is registered in the runtime as an FfiTriggerImpl.
/// 3. The trigger's `poll()` actually round-trips through the FFI: the
///    cdylib's user code returns `Skip`, the host adapter receives
///    `Ok(TriggerResult::Skip)`.
/// 4. The trigger's metadata accessors (name, poll_interval,
///    cron_expression, allow_concurrent) come back with the values
///    declared by the macro.
#[tokio::test]
#[serial]
async fn reconciler_loads_mixed_rust_with_in_package_trigger_subscription() {
    let archive = pack_fixture("mixed-rust");
    let dylib = read_fixture_dylib("mixed-rust");

    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.reset_database().await;
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let storage_writer = fixture.create_storage();
    let mut registry_writer = dal.workflow_registry(storage_writer);
    let pkg_id = registry_writer
        .register_workflow_package(archive.clone())
        .await
        .expect("register mixed-rust");
    registry_writer.claim_next_build().await.expect("claim");
    registry_writer
        .mark_build_success(pkg_id, dylib.clone())
        .await
        .expect("mark built");

    let storage_reader = fixture.create_storage();
    let registry_reader: Arc<dyn WorkflowRegistry> = Arc::new(
        WorkflowRegistryImpl::new(storage_reader, fixture.get_database())
            .expect("WorkflowRegistryImpl::new"),
    );

    let endpoint_registry = EndpointRegistry::new();
    let scheduler = Arc::new(ComputationGraphScheduler::new(endpoint_registry));
    let runtime = Arc::new(Runtime::empty());

    let config = ReconcilerConfig {
        default_tenant_id: "public".to_string(),
        ..ReconcilerConfig::default()
    };
    let (_shutdown_tx, shutdown_rx) = watch::channel(false);
    let reconciler = RegistryReconciler::new(registry_reader, config, shutdown_rx)
        .expect("reconciler new")
        .with_runtime(runtime.clone())
        .with_graph_scheduler(scheduler.clone());

    let result = reconciler.reconcile().await.expect("reconcile mixed-rust");
    assert!(
        result.packages_failed.is_empty(),
        "mixed-rust must load cleanly with the FFI Trigger bridge; failures = {:?}",
        result.packages_failed
    );
    assert_eq!(
        result.packages_loaded.len(),
        1,
        "exactly one package should load"
    );

    // FFI Trigger bridge: trigger is registered as an FfiTriggerImpl
    // adapter (cdylib's inventory entries don't reach the host's
    // inventory::iter, so this only succeeds because the bridge ran).
    let trigger = runtime
        .get_trigger("mixed_trigger")
        .expect("mixed_trigger must be registered after reconcile");
    assert_eq!(trigger.name(), "mixed_trigger");
    assert!(
        trigger.cron_expression().is_none(),
        "mixed_trigger is custom-poll, not cron"
    );
    // Macro emits poll_interval = "5s".
    assert_eq!(
        trigger.poll_interval(),
        std::time::Duration::from_secs(5),
        "poll_interval should round-trip from the macro declaration"
    );

    // Round-trip the actual poll() through FFI. The cdylib's user code
    // returns Skip; if the bridge is wired correctly, the host sees
    // Ok(TriggerResult::Skip).
    let poll_outcome = trigger.poll().await.expect("FFI poll round-trip");
    assert!(
        !poll_outcome.should_fire(),
        "mixed_trigger's user-code returns Skip; got fire=true"
    );

    // Reactor + workflow + graph must also have landed.
    assert!(
        scheduler
            .reactor_accumulator_names("mixed_reactor")
            .await
            .is_some(),
        "mixed_reactor must be loaded"
    );
    assert!(
        runtime
            .workflow_names()
            .iter()
            .any(|n| n == "mixed_workflow"),
        "mixed_workflow must be registered"
    );
    assert!(
        runtime
            .computation_graph_names()
            .iter()
            .any(|n| n == "mixed_graph"),
        "mixed_graph must be registered"
    );

    scheduler.shutdown_all().await;
}
