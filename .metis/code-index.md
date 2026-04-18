# Code Index

> Generated: 2026-04-18T13:09:06Z | 467 files | JavaScript, Python, Rust

## Project Structure

```
├── crates/
│   ├── cloacina/
│   │   ├── build.rs
│   │   ├── src/
│   │   │   ├── computation_graph/
│   │   │   │   ├── accumulator.rs
│   │   │   │   ├── global_registry.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── packaging_bridge.rs
│   │   │   │   ├── reactor.rs
│   │   │   │   ├── registry.rs
│   │   │   │   ├── scheduler.rs
│   │   │   │   ├── stream_backend.rs
│   │   │   │   └── types.rs
│   │   │   ├── context.rs
│   │   │   ├── cron_evaluator.rs
│   │   │   ├── cron_recovery.rs
│   │   │   ├── cron_trigger_scheduler.rs
│   │   │   ├── crypto/
│   │   │   │   ├── key_encryption.rs
│   │   │   │   ├── mod.rs
│   │   │   │   └── signing.rs
│   │   │   ├── dal/
│   │   │   │   ├── filesystem_dal/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── workflow_registry_storage.rs
│   │   │   │   ├── mod.rs
│   │   │   │   └── unified/
│   │   │   │       ├── api_keys/
│   │   │   │       │   ├── crud.rs
│   │   │   │       │   └── mod.rs
│   │   │   │       ├── checkpoint.rs
│   │   │   │       ├── context.rs
│   │   │   │       ├── execution_event.rs
│   │   │   │       ├── mod.rs
│   │   │   │       ├── models.rs
│   │   │   │       ├── recovery_event.rs
│   │   │   │       ├── schedule/
│   │   │   │       │   ├── crud.rs
│   │   │   │       │   └── mod.rs
│   │   │   │       ├── schedule_execution/
│   │   │   │       │   ├── crud.rs
│   │   │   │       │   └── mod.rs
│   │   │   │       ├── task_execution/
│   │   │   │       │   ├── claiming.rs
│   │   │   │       │   ├── crud.rs
│   │   │   │       │   ├── mod.rs
│   │   │   │       │   ├── queries.rs
│   │   │   │       │   ├── recovery.rs
│   │   │   │       │   └── state.rs
│   │   │   │       ├── task_execution_metadata.rs
│   │   │   │       ├── task_outbox.rs
│   │   │   │       ├── workflow_execution.rs
│   │   │   │       ├── workflow_packages.rs
│   │   │   │       ├── workflow_registry.rs
│   │   │   │       └── workflow_registry_storage.rs
│   │   │   ├── database/
│   │   │   │   ├── admin.rs
│   │   │   │   ├── connection/
│   │   │   │   │   ├── backend.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── schema_validation.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── schema.rs
│   │   │   │   └── universal_types.rs
│   │   │   ├── dispatcher/
│   │   │   │   ├── default.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── router.rs
│   │   │   │   ├── traits.rs
│   │   │   │   ├── types.rs
│   │   │   │   └── work_distributor.rs
│   │   │   ├── error.rs
│   │   │   ├── execution_planner/
│   │   │   │   ├── context_manager.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── recovery.rs
│   │   │   │   ├── scheduler_loop.rs
│   │   │   │   ├── stale_claim_sweeper.rs
│   │   │   │   ├── state_manager.rs
│   │   │   │   └── trigger_rules.rs
│   │   │   ├── executor/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── slot_token.rs
│   │   │   │   ├── task_handle.rs
│   │   │   │   ├── thread_task_executor.rs
│   │   │   │   ├── types.rs
│   │   │   │   └── workflow_executor.rs
│   │   │   ├── graph.rs
│   │   │   ├── inventory_entries.rs
│   │   │   ├── lib.rs
│   │   │   ├── logging.rs
│   │   │   ├── models/
│   │   │   │   ├── context.rs
│   │   │   │   ├── execution_event.rs
│   │   │   │   ├── key_trust_acl.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── package_signature.rs
│   │   │   │   ├── recovery_event.rs
│   │   │   │   ├── schedule.rs
│   │   │   │   ├── signing_key.rs
│   │   │   │   ├── task_execution.rs
│   │   │   │   ├── task_execution_metadata.rs
│   │   │   │   ├── task_outbox.rs
│   │   │   │   ├── trusted_key.rs
│   │   │   │   ├── workflow_execution.rs
│   │   │   │   ├── workflow_packages.rs
│   │   │   │   └── workflow_registry.rs
│   │   │   ├── packaging/
│   │   │   │   ├── debug.rs
│   │   │   │   ├── manifest.rs
│   │   │   │   ├── manifest_schema.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── platform.rs
│   │   │   │   ├── tests.rs
│   │   │   │   ├── types.rs
│   │   │   │   └── validation.rs
│   │   │   ├── python/
│   │   │   │   ├── bindings/
│   │   │   │   │   ├── admin.rs
│   │   │   │   │   ├── context.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── runner.rs
│   │   │   │   │   ├── trigger.rs
│   │   │   │   │   └── value_objects/
│   │   │   │   │       ├── mod.rs
│   │   │   │   │       └── retry.rs
│   │   │   │   ├── computation_graph.rs
│   │   │   │   ├── computation_graph_tests.rs
│   │   │   │   ├── context.rs
│   │   │   │   ├── executor.rs
│   │   │   │   ├── loader.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── namespace.rs
│   │   │   │   ├── task.rs
│   │   │   │   ├── trigger.rs
│   │   │   │   ├── workflow.rs
│   │   │   │   └── workflow_context.rs
│   │   │   ├── registry/
│   │   │   │   ├── error.rs
│   │   │   │   ├── loader/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── package_loader.rs
│   │   │   │   │   ├── python_loader.rs
│   │   │   │   │   ├── task_registrar/
│   │   │   │   │   │   ├── dynamic_task.rs
│   │   │   │   │   │   ├── extraction.rs
│   │   │   │   │   │   ├── mod.rs
│   │   │   │   │   │   └── types.rs
│   │   │   │   │   └── validator/
│   │   │   │   │       ├── format.rs
│   │   │   │   │       ├── metadata.rs
│   │   │   │   │       ├── mod.rs
│   │   │   │   │       ├── security.rs
│   │   │   │   │       ├── size.rs
│   │   │   │   │       ├── symbols.rs
│   │   │   │   │       └── types.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── reconciler/
│   │   │   │   │   ├── extraction.rs
│   │   │   │   │   ├── loading.rs
│   │   │   │   │   └── mod.rs
│   │   │   │   ├── storage/
│   │   │   │   │   └── mod.rs
│   │   │   │   ├── traits.rs
│   │   │   │   ├── types.rs
│   │   │   │   └── workflow_registry/
│   │   │   │       ├── database.rs
│   │   │   │       ├── filesystem.rs
│   │   │   │       ├── mod.rs
│   │   │   │       └── package.rs
│   │   │   ├── retry.rs
│   │   │   ├── runner/
│   │   │   │   ├── default_runner/
│   │   │   │   │   ├── config.rs
│   │   │   │   │   ├── cron_api.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── services.rs
│   │   │   │   │   ├── workflow_executor_impl.rs
│   │   │   │   │   └── workflow_result.rs
│   │   │   │   └── mod.rs
│   │   │   ├── runtime.rs
│   │   │   ├── security/
│   │   │   │   ├── api_keys.rs
│   │   │   │   ├── audit.rs
│   │   │   │   ├── db_key_manager.rs
│   │   │   │   ├── key_manager.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── package_signer.rs
│   │   │   │   └── verification.rs
│   │   │   ├── task/
│   │   │   │   └── namespace.rs
│   │   │   ├── task.rs
│   │   │   ├── trigger/
│   │   │   │   ├── mod.rs
│   │   │   │   └── registry.rs
│   │   │   ├── var.rs
│   │   │   └── workflow/
│   │   │       ├── builder.rs
│   │   │       ├── graph.rs
│   │   │       ├── metadata.rs
│   │   │       ├── mod.rs
│   │   │       └── registry.rs
│   │   └── tests/
│   │       ├── fixtures.rs
│   │       └── integration/
│   │           ├── computation_graph.rs
│   │           ├── context.rs
│   │           ├── dal/
│   │           │   ├── api_keys.rs
│   │           │   ├── context.rs
│   │           │   ├── execution_events.rs
│   │           │   ├── mod.rs
│   │           │   ├── sub_status.rs
│   │           │   ├── task_claiming.rs
│   │           │   ├── workflow_packages.rs
│   │           │   ├── workflow_registry.rs
│   │           │   └── workflow_registry_reconciler_integration.rs
│   │           ├── database/
│   │           │   ├── connection.rs
│   │           │   ├── migrations.rs
│   │           │   └── mod.rs
│   │           ├── error.rs
│   │           ├── error_paths.rs
│   │           ├── event_dedup.rs
│   │           ├── executor/
│   │           │   ├── context_merging.rs
│   │           │   ├── defer_until.rs
│   │           │   ├── mod.rs
│   │           │   ├── multi_tenant.rs
│   │           │   ├── pause_resume.rs
│   │           │   └── task_execution.rs
│   │           ├── fidius_validation.rs
│   │           ├── logging.rs
│   │           ├── main.rs
│   │           ├── models/
│   │           │   ├── context.rs
│   │           │   └── mod.rs
│   │           ├── packaging.rs
│   │           ├── packaging_inspection.rs
│   │           ├── python_package.rs
│   │           ├── registry_simple_functional_test.rs
│   │           ├── registry_storage_tests.rs
│   │           ├── registry_workflow_registry_tests.rs
│   │           ├── runner_configurable_registry_tests.rs
│   │           ├── scheduler/
│   │           │   ├── basic_scheduling.rs
│   │           │   ├── cron_basic.rs
│   │           │   ├── dependency_resolution.rs
│   │           │   ├── mod.rs
│   │           │   ├── recovery.rs
│   │           │   ├── stale_claims.rs
│   │           │   └── trigger_rules.rs
│   │           ├── signing/
│   │           │   ├── key_rotation.rs
│   │           │   ├── mod.rs
│   │           │   ├── security_failures.rs
│   │           │   ├── sign_and_verify.rs
│   │           │   └── trust_chain.rs
│   │           ├── task/
│   │           │   ├── checkpoint.rs
│   │           │   ├── debug_macro.rs
│   │           │   ├── handle_macro.rs
│   │           │   ├── macro_test.rs
│   │           │   ├── mod.rs
│   │           │   └── simple_macro.rs
│   │           ├── test_dlopen_packaged.rs
│   │           ├── test_registry_dynamic_loading.rs
│   │           ├── test_registry_dynamic_loading_simple.rs
│   │           ├── trigger_packaging.rs
│   │           ├── unified_workflow.rs
│   │           └── workflow/
│   │               ├── basic.rs
│   │               ├── callback_test.rs
│   │               ├── macro_test.rs
│   │               ├── mod.rs
│   │               └── subgraph.rs
│   ├── cloacina-build/
│   │   └── src/
│   │       └── lib.rs
│   ├── cloacina-compiler/
│   │   └── src/
│   │       ├── config.rs
│   │       ├── health.rs
│   │       ├── lib.rs
│   │       ├── loopp.rs
│   │       └── main.rs
│   ├── cloacina-computation-graph/
│   │   └── src/
│   │       └── lib.rs
│   ├── cloacina-macros/
│   │   └── src/
│   │       ├── computation_graph/
│   │       │   ├── accumulator_macros.rs
│   │       │   ├── codegen.rs
│   │       │   ├── graph_ir.rs
│   │       │   ├── mod.rs
│   │       │   └── parser.rs
│   │       ├── lib.rs
│   │       ├── packaged_workflow.rs
│   │       ├── registry.rs
│   │       ├── tasks.rs
│   │       ├── trigger_attr.rs
│   │       └── workflow_attr.rs
│   ├── cloacina-server/
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── main.rs
│   │       └── routes/
│   │           ├── auth.rs
│   │           ├── error.rs
│   │           ├── executions.rs
│   │           ├── health_reactive.rs
│   │           ├── keys.rs
│   │           ├── mod.rs
│   │           ├── tenants.rs
│   │           ├── triggers.rs
│   │           ├── workflows.rs
│   │           └── ws.rs
│   ├── cloacina-testing/
│   │   └── src/
│   │       ├── assertions.rs
│   │       ├── boundary.rs
│   │       ├── lib.rs
│   │       ├── mock.rs
│   │       ├── result.rs
│   │       └── runner.rs
│   ├── cloacina-workflow/
│   │   └── src/
│   │       ├── context.rs
│   │       ├── error.rs
│   │       ├── lib.rs
│   │       ├── namespace.rs
│   │       ├── retry.rs
│   │       ├── task.rs
│   │       └── trigger.rs
│   ├── cloacina-workflow-plugin/
│   │   └── src/
│   │       ├── lib.rs
│   │       └── types.rs
│   └── cloacinactl/
│       ├── build.rs
│       └── src/
│           ├── commands/
│           │   ├── cleanup_events.rs
│           │   ├── config.rs
│           │   ├── daemon.rs
│           │   ├── health.rs
│           │   ├── mod.rs
│           │   ├── status.rs
│           │   └── watcher.rs
│           ├── main.rs
│           ├── nouns/
│           │   ├── daemon/
│           │   │   ├── health.rs
│           │   │   ├── mod.rs
│           │   │   ├── start.rs
│           │   │   ├── status.rs
│           │   │   └── stop.rs
│           │   ├── execution/
│           │   │   └── mod.rs
│           │   ├── graph/
│           │   │   └── mod.rs
│           │   ├── key/
│           │   │   └── mod.rs
│           │   ├── mod.rs
│           │   ├── package/
│           │   │   ├── build.rs
│           │   │   ├── delete.rs
│           │   │   ├── inspect.rs
│           │   │   ├── list.rs
│           │   │   ├── mod.rs
│           │   │   ├── pack.rs
│           │   │   ├── publish.rs
│           │   │   └── upload.rs
│           │   ├── server/
│           │   │   ├── health.rs
│           │   │   ├── mod.rs
│           │   │   ├── start.rs
│           │   │   ├── status.rs
│           │   │   └── stop.rs
│           │   ├── tenant/
│           │   │   └── mod.rs
│           │   ├── trigger/
│           │   │   └── mod.rs
│           │   └── workflow/
│           │       └── mod.rs
│           └── shared/
│               ├── client.rs
│               ├── client_ctx.rs
│               ├── error.rs
│               ├── mod.rs
│               ├── output.rs
│               ├── pid.rs
│               └── render.rs
├── docs/
│   └── themes/
│       └── hugo-geekdoc/
│           ├── eslint.config.js
│           └── static/
│               └── js/
│                   ├── 130-3b252fb9.chunk.min.js
│                   ├── 147-5647664f.chunk.min.js
│                   ├── 164-f339d58d.chunk.min.js
│                   ├── 165-d20df99c.chunk.min.js
│                   ├── 248-d3b4979c.chunk.min.js
│                   ├── 295-8a201dad.chunk.min.js
│                   ├── 297-baccf39c.chunk.min.js
│                   ├── 301-504b6216.chunk.min.js
│                   ├── 343-07706d94.chunk.min.js
│                   ├── 370-0e626739.chunk.min.js
│                   ├── 387-d98ee904.chunk.min.js
│                   ├── 388-0f08b415.chunk.min.js
│                   ├── 391-a0aaa95e.chunk.min.js
│                   ├── 420-35785222.chunk.min.js
│                   ├── 428-1733cd76.chunk.min.js
│                   ├── 435-95a7762e.chunk.min.js
│                   ├── 440-00a1e1fb.chunk.min.js
│                   ├── 452-56ef13c4.chunk.min.js
│                   ├── 475-5c92875f.chunk.min.js
│                   ├── 559-fa1bc454.chunk.min.js
│                   ├── 567-6c3220fd.chunk.min.js
│                   ├── 623-da9b1ffc.chunk.min.js
│                   ├── 687-3d36056d.chunk.min.js
│                   ├── 704-ed584c37.chunk.min.js
│                   ├── 719-e4d0dfca.chunk.min.js
│                   ├── 720-9be19eb2.chunk.min.js
│                   ├── 723-dc4c5ebb.chunk.min.js
│                   ├── 731-7d3aeec3.chunk.min.js
│                   ├── 740-2f747788.chunk.min.js
│                   ├── 768-19f4d0a4.chunk.min.js
│                   ├── 846-699d57b4.chunk.min.js
│                   ├── 848-160cde0b.chunk.min.js
│                   ├── 890-8401ddb1.chunk.min.js
│                   ├── 906-5e2ec84c.chunk.min.js
│                   ├── 938-e8554e58.chunk.min.js
│                   ├── 975-7b2dc052.chunk.min.js
│                   ├── colortheme-05deda6f.bundle.min.js
│                   ├── katex-13a419d8.bundle.min.js
│                   ├── main-c5dd8165.bundle.min.js
│                   ├── mermaid-6735100e.bundle.min.js
│                   └── search-16a110ff.bundle.min.js
├── examples/
│   ├── features/
│   │   ├── computation-graphs/
│   │   │   ├── continuous-scheduling/
│   │   │   │   └── src/
│   │   │   │       └── main.rs
│   │   │   ├── packaged-graph/
│   │   │   │   ├── build.rs
│   │   │   │   └── src/
│   │   │   │       └── lib.rs
│   │   │   └── python-packaged-graph/
│   │   │       └── market_maker/
│   │   │           ├── __init__.py
│   │   │           └── graph.py
│   │   └── workflows/
│   │       ├── complex-dag/
│   │       │   ├── build.rs
│   │       │   └── src/
│   │       │       └── lib.rs
│   │       ├── cron-scheduling/
│   │       │   ├── build.rs
│   │       │   └── src/
│   │       │       └── main.rs
│   │       ├── deferred-tasks/
│   │       │   ├── build.rs
│   │       │   └── src/
│   │       │       └── main.rs
│   │       ├── event-triggers/
│   │       │   ├── build.rs
│   │       │   └── src/
│   │       │       ├── main.rs
│   │       │       └── triggers.rs
│   │       ├── multi-tenant/
│   │       │   ├── build.rs
│   │       │   └── src/
│   │       │       └── main.rs
│   │       ├── packaged-triggers/
│   │       │   ├── build.rs
│   │       │   └── src/
│   │       │       └── lib.rs
│   │       ├── packaged-workflows/
│   │       │   ├── build.rs
│   │       │   └── src/
│   │       │       └── lib.rs
│   │       ├── per-tenant-credentials/
│   │       │   ├── build.rs
│   │       │   └── src/
│   │       │       └── main.rs
│   │       ├── python-workflow/
│   │       │   ├── data_pipeline/
│   │       │   │   ├── __init__.py
│   │       │   │   └── tasks.py
│   │       │   └── run_pipeline.py
│   │       ├── registry-execution/
│   │       │   ├── build.rs
│   │       │   └── src/
│   │       │       └── main.rs
│   │       ├── simple-packaged/
│   │       │   ├── build.rs
│   │       │   ├── src/
│   │       │   │   └── lib.rs
│   │       │   └── tests/
│   │       │       ├── ffi_tests.rs
│   │       │       └── host_managed_registry_tests.rs
│   │       └── validation-failures/
│   │           ├── build.rs
│   │           └── src/
│   │               ├── circular_dependency.rs
│   │               ├── duplicate_task_ids.rs
│   │               ├── missing_dependency.rs
│   │               └── missing_workflow_task.rs
│   ├── performance/
│   │   ├── computation-graph/
│   │   │   ├── build.rs
│   │   │   └── src/
│   │   │       ├── bench.rs
│   │   │       └── main.rs
│   │   ├── parallel/
│   │   │   ├── build.rs
│   │   │   └── src/
│   │   │       └── main.rs
│   │   ├── pipeline/
│   │   │   ├── build.rs
│   │   │   └── src/
│   │   │       └── main.rs
│   │   └── simple/
│   │       ├── build.rs
│   │       └── src/
│   │           └── main.rs
│   └── tutorials/
│       ├── computation-graphs/
│       │   └── library/
│       │       ├── 07-computation-graph/
│       │       │   ├── build.rs
│       │       │   └── src/
│       │       │       └── main.rs
│       │       ├── 08-accumulators/
│       │       │   ├── build.rs
│       │       │   └── src/
│       │       │       └── main.rs
│       │       ├── 09-full-pipeline/
│       │       │   ├── build.rs
│       │       │   └── src/
│       │       │       └── main.rs
│       │       └── 10-routing/
│       │           ├── build.rs
│       │           └── src/
│       │               └── main.rs
│       ├── python/
│       │   ├── computation-graphs/
│       │   │   ├── 09_computation_graph.py
│       │   │   ├── 10_accumulators.py
│       │   │   └── 11_routing.py
│       │   └── workflows/
│       │       ├── 01_first_workflow.py
│       │       ├── 02_context_handling.py
│       │       ├── 03_complex_workflows.py
│       │       ├── 04_error_handling.py
│       │       ├── 05_cron_scheduling.py
│       │       ├── 06_multi_tenancy.py
│       │       ├── 07_event_triggers.py
│       │       └── 08_packaged_triggers.py
│       └── workflows/
│           └── library/
│               ├── 01-basic-workflow/
│               │   ├── build.rs
│               │   └── src/
│               │       └── main.rs
│               ├── 02-multi-task/
│               │   ├── build.rs
│               │   └── src/
│               │       ├── main.rs
│               │       └── tasks.rs
│               ├── 03-dependencies/
│               │   ├── build.rs
│               │   └── src/
│               │       └── main.rs
│               ├── 04-error-handling/
│               │   ├── build.rs
│               │   └── src/
│               │       └── main.rs
│               ├── 05-advanced/
│               │   ├── build.rs
│               │   └── src/
│               │       ├── main.rs
│               │       └── tasks.rs
│               └── 06-multi-tenancy/
│                   ├── build.rs
│                   └── src/
│                       └── main.rs
└── tests/
    └── python/
        ├── conftest.py
        ├── test_scenario_01_basic_api.py
        ├── test_scenario_02_single_task_workflow_execution.py
        ├── test_scenario_03_function_based_dag_topology.py
        ├── test_scenario_08_multi_task_workflow_execution.py
        ├── test_scenario_09_context_propagation.py
        ├── test_scenario_10_workflow_error_handling.py
        ├── test_scenario_11_retry_mechanisms.py
        ├── test_scenario_12_workflow_performance.py
        ├── test_scenario_13_complex_dependency_chains.py
        ├── test_scenario_14_trigger_rules.py
        ├── test_scenario_15_workflow_versioning.py
        ├── test_scenario_16_registry_management.py
        ├── test_scenario_17_advanced_error_handling.py
        ├── test_scenario_18_basic_shared_runner_functionality.py
        ├── test_scenario_19_context_passing_runner.py
        ├── test_scenario_20_multiple_workflow_execution_runner.py
        ├── test_scenario_21_success_validation_runner.py
        ├── test_scenario_22_simple_workflow_context_manager.py
        ├── test_scenario_23_multi_task_workflow_dependencies_builder.py
        ├── test_scenario_24_parameterized_workflows.py
        ├── test_scenario_25_async_task_support.py
        ├── test_scenario_26_simple_workflow_execution.py
        ├── test_scenario_27_cron_scheduling.py
        ├── test_scenario_28_multi_tenancy.py
        ├── test_scenario_29_event_triggers.py
        ├── test_scenario_30_task_callbacks.py
        ├── test_scenario_31_task_handle.py
        └── utilities.py
```

## Modules

### crates/cloacina

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/build.rs

-  `main` function L17-19 — `()`

### crates/cloacina/src/computation_graph

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/computation_graph/accumulator.rs

- pub `AccumulatorHealth` enum L42-53 — `Starting | Connecting | Live | Disconnected | SocketOnly` — Health state of an accumulator, reported via watch channel.
- pub `health_channel` function L68-73 — `() -> ( watch::Sender<AccumulatorHealth>, watch::Receiver<AccumulatorHealth>, )` — Create a health reporting channel for an accumulator.
- pub `AccumulatorError` enum L77-86 — `Init | Run | Send | Checkpoint` — Errors from accumulator operations.
- pub `Accumulator` interface L100-114 — `{ fn process(), fn init() }` — An accumulator consumes events from a source and pushes boundaries to a reactor.
- pub `EventSource` interface L126-134 — `{ fn run() }` — An event source actively pulls events from an external source and pushes
- pub `CheckpointHandle` struct L141-145 — `{ dal: crate::dal::unified::DAL, graph_name: String, accumulator_name: String }` — Handle for persisting accumulator state via the DAL.
- pub `new` function L149-159 — `( dal: crate::dal::unified::DAL, graph_name: String, accumulator_name: String, )...` — Create a new checkpoint handle for the given graph and accumulator.
- pub `save` function L162-170 — `(&self, state: &T) -> Result<(), AccumulatorError>` — Persist accumulator state.
- pub `load` function L173-189 — `(&self) -> Result<Option<T>, AccumulatorError>` — Load previously persisted accumulator state.
- pub `dal` function L192-194 — `(&self) -> &crate::dal::unified::DAL` — Access the underlying DAL for direct checkpoint operations.
- pub `graph_name` function L197-199 — `(&self) -> &str` — Get the graph name this handle is scoped to.
- pub `accumulator_name` function L202-204 — `(&self) -> &str` — Get the accumulator name this handle is scoped to.
- pub `AccumulatorContext` struct L208-221 — `{ output: BoundarySender, name: String, shutdown: watch::Receiver<bool>, checkpo...` — Context provided to the accumulator by the runtime.
- pub `BoundarySender` struct L229-234 — `{ inner: mpsc::Sender<(SourceName, Vec<u8>)>, source_name: SourceName, sequence:...` — Sends serialized boundaries to the reactor.
- pub `new` function L237-243 — `(sender: mpsc::Sender<(SourceName, Vec<u8>)>, source_name: SourceName) -> Self` — See CLOACI-S-0004 for the full specification.
- pub `with_sequence` function L246-256 — `( sender: mpsc::Sender<(SourceName, Vec<u8>)>, source_name: SourceName, start_se...` — Create a sender with a specific starting sequence number (for restart recovery).
- pub `send` function L260-269 — `(&self, boundary: &T) -> Result<(), AccumulatorError>` — Serialize and send a boundary to the reactor.
- pub `source_name` function L272-274 — `(&self) -> &SourceName` — Get the source name this sender is associated with.
- pub `sequence_number` function L277-279 — `(&self) -> u64` — Get the current sequence number (last emitted).
- pub `AccumulatorRuntimeConfig` struct L283-286 — `{ merge_channel_capacity: usize }` — Configuration for the accumulator runtime.
- pub `accumulator_runtime` function L317-324 — `( acc: A, ctx: AccumulatorContext, socket_rx: mpsc::Receiver<Vec<u8>>, config: A...` — Run an accumulator as 2-3 tokio tasks connected by a merge channel.
- pub `accumulator_runtime_with_source` function L329-340 — `( acc: A, ctx: AccumulatorContext, socket_rx: mpsc::Receiver<Vec<u8>>, config: A...` — Run an accumulator with an active event source that pulls events from
- pub `shutdown_signal` function L445-447 — `() -> (watch::Sender<bool>, watch::Receiver<bool>)` — Create a shutdown signal pair.
- pub `PollingAccumulator` interface L458-468 — `{ fn poll(), fn interval() }` — A polling accumulator periodically calls an async poll function to query
- pub `polling_accumulator_runtime` function L474-546 — `( mut poller: P, ctx: AccumulatorContext, socket_rx: mpsc::Receiver<Vec<u8>>, )` — Run a polling accumulator as a timer-based loop.
- pub `BatchAccumulator` interface L560-568 — `{ fn process_batch() }` — A batch accumulator buffers incoming events and processes them all at once
- pub `BatchAccumulatorConfig` struct L571-576 — `{ flush_interval: Option<std::time::Duration>, max_buffer_size: Option<usize> }` — Configuration for the batch accumulator runtime.
- pub `flush_signal` function L591-593 — `() -> (mpsc::Sender<()>, mpsc::Receiver<()>)` — Create a flush signal pair for batch accumulators.
- pub `batch_accumulator_runtime` function L600-673 — `( mut acc: B, ctx: AccumulatorContext, socket_rx: mpsc::Receiver<Vec<u8>>, mut f...` — Run a batch accumulator that buffers events and flushes on signal, timer, or size threshold.
- pub `StateAccumulator` struct L751-754 — `{ buffer: std::collections::VecDeque<T>, capacity: i32 }` — A state accumulator holds a bounded VecDeque<T> that receives values from
- pub `new` function L757-762 — `(capacity: i32) -> Self` — See CLOACI-S-0004 for the full specification.
- pub `state_accumulator_runtime` function L769-873 — `( mut acc: StateAccumulator<T>, ctx: AccumulatorContext, socket_rx: mpsc::Receiv...` — Run a state accumulator.
-  `AccumulatorHealth` type L55-65 — `= AccumulatorHealth` — See CLOACI-S-0004 for the full specification.
-  `fmt` function L56-64 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — See CLOACI-S-0004 for the full specification.
-  `init` function L111-113 — `(&mut self, _ctx: &AccumulatorContext) -> Result<(), AccumulatorError>` — Called on startup before first receive.
-  `CheckpointHandle` type L147-205 — `= CheckpointHandle` — See CLOACI-S-0004 for the full specification.
-  `BoundarySender` type L236-280 — `= BoundarySender` — See CLOACI-S-0004 for the full specification.
-  `AccumulatorRuntimeConfig` type L288-294 — `impl Default for AccumulatorRuntimeConfig` — See CLOACI-S-0004 for the full specification.
-  `default` function L289-293 — `() -> Self` — See CLOACI-S-0004 for the full specification.
-  `NoEventSource` struct L343 — `-` — Placeholder type for when no event source is provided.
-  `NoEventSource` type L346-354 — `impl EventSource for NoEventSource` — See CLOACI-S-0004 for the full specification.
-  `run` function L347-353 — `( self, _events: mpsc::Sender<Vec<u8>>, _shutdown: watch::Receiver<bool>, ) -> R...` — See CLOACI-S-0004 for the full specification.
-  `accumulator_runtime_inner` function L357-442 — `( mut acc: A, ctx: AccumulatorContext, socket_rx: mpsc::Receiver<Vec<u8>>, confi...` — Inner runtime shared by both `accumulator_runtime` and `accumulator_runtime_with_source`.
-  `BatchAccumulatorConfig` type L578-585 — `impl Default for BatchAccumulatorConfig` — See CLOACI-S-0004 for the full specification.
-  `default` function L579-584 — `() -> Self` — See CLOACI-S-0004 for the full specification.
-  `persist_batch_buffer` function L676-682 — `(ctx: &AccumulatorContext, buffer: &[Vec<u8>])` — Persist batch buffer snapshot to DAL for crash resilience (best-effort).
-  `flush_batch` function L685-703 — `( acc: &mut B, buffer: &mut Vec<Vec<u8>>, ctx: &AccumulatorContext, )` — Flush the buffer through the batch accumulator and send boundary if produced.
-  `set_health` function L710-714 — `(ctx: &AccumulatorContext, health: AccumulatorHealth)` — Set health state (best-effort, no-op if health channel not configured).
-  `persist_boundary` function L717-736 — `(ctx: &AccumulatorContext, boundary: &T)` — Persist last-emitted boundary with sequence number to DAL (best-effort, logs on failure).
-  `tests` module L876-1470 — `-` — See CLOACI-S-0004 for the full specification.
-  `TestEvent` struct L881-883 — `{ value: f64 }` — See CLOACI-S-0004 for the full specification.
-  `TestBoundary` struct L886-888 — `{ result: f64 }` — See CLOACI-S-0004 for the full specification.
-  `DoubleAccumulator` struct L890 — `-` — See CLOACI-S-0004 for the full specification.
-  `DoubleAccumulator` type L893-902 — `impl Accumulator for DoubleAccumulator` — See CLOACI-S-0004 for the full specification.
-  `Output` type L894 — `= TestBoundary` — See CLOACI-S-0004 for the full specification.
-  `process` function L896-901 — `(&mut self, event: Vec<u8>) -> Option<TestBoundary>` — See CLOACI-S-0004 for the full specification.
-  `test_boundary_sender_round_trip` function L905-917 — `()` — See CLOACI-S-0004 for the full specification.
-  `test_accumulator_runtime_processes_socket_events` function L920-958 — `()` — See CLOACI-S-0004 for the full specification.
-  `test_accumulator_runtime_multiple_events` function L961-997 — `()` — See CLOACI-S-0004 for the full specification.
-  `test_accumulator_shutdown` function L1000-1029 — `()` — See CLOACI-S-0004 for the full specification.
-  `CountingPoller` struct L1033-1036 — `{ count: u32, max: u32 }` — See CLOACI-S-0004 for the full specification.
-  `CountingPoller` type L1039-1056 — `impl PollingAccumulator for CountingPoller` — See CLOACI-S-0004 for the full specification.
-  `Output` type L1040 — `= TestBoundary` — See CLOACI-S-0004 for the full specification.
-  `poll` function L1042-1051 — `(&mut self) -> Option<TestBoundary>` — See CLOACI-S-0004 for the full specification.
-  `interval` function L1053-1055 — `(&self) -> std::time::Duration` — See CLOACI-S-0004 for the full specification.
-  `test_polling_accumulator_emits_on_some` function L1059-1096 — `()` — See CLOACI-S-0004 for the full specification.
-  `test_polling_accumulator_skips_on_none` function L1099-1128 — `()` — See CLOACI-S-0004 for the full specification.
-  `test_polling_accumulator_shutdown` function L1131-1155 — `()` — See CLOACI-S-0004 for the full specification.
-  `SumBatchAccumulator` struct L1159 — `-` — See CLOACI-S-0004 for the full specification.
-  `SumBatchAccumulator` type L1162-1173 — `impl BatchAccumulator for SumBatchAccumulator` — See CLOACI-S-0004 for the full specification.
-  `Output` type L1163 — `= TestBoundary` — See CLOACI-S-0004 for the full specification.
-  `process_batch` function L1165-1172 — `(&mut self, events: Vec<Vec<u8>>) -> Option<TestBoundary>` — See CLOACI-S-0004 for the full specification.
-  `test_batch_accumulator_flush_on_signal` function L1176-1225 — `()` — See CLOACI-S-0004 for the full specification.
-  `test_batch_accumulator_flush_on_timer` function L1228-1274 — `()` — See CLOACI-S-0004 for the full specification.
-  `test_batch_accumulator_empty_flush_skips` function L1277-1313 — `()` — See CLOACI-S-0004 for the full specification.
-  `test_batch_accumulator_max_buffer_size` function L1316-1361 — `()` — See CLOACI-S-0004 for the full specification.
-  `test_batch_accumulator_shutdown_drains` function L1364-1407 — `()` — See CLOACI-S-0004 for the full specification.
-  `FilterAccumulator` struct L1409 — `-` — See CLOACI-S-0004 for the full specification.
-  `FilterAccumulator` type L1412-1426 — `impl Accumulator for FilterAccumulator` — See CLOACI-S-0004 for the full specification.
-  `Output` type L1413 — `= TestBoundary` — See CLOACI-S-0004 for the full specification.
-  `process` function L1415-1425 — `(&mut self, event: Vec<u8>) -> Option<TestBoundary>` — See CLOACI-S-0004 for the full specification.
-  `test_accumulator_process_returns_none` function L1429-1469 — `()` — See CLOACI-S-0004 for the full specification.

#### crates/cloacina/src/computation_graph/global_registry.rs

- pub `ComputationGraphRegistration` struct L30-37 — `{ graph_fn: CompiledGraphFn, accumulator_names: Vec<String>, reaction_mode: Stri...` — Metadata about a registered computation graph.
- pub `ComputationGraphConstructor` type L39 — `= Box<dyn Fn() -> ComputationGraphRegistration + Send + Sync>` — Mirrors the global workflow/task registries used by the reconciler.
- pub `GlobalComputationGraphRegistry` type L40 — `= Arc<RwLock<HashMap<String, ComputationGraphConstructor>>>` — Mirrors the global workflow/task registries used by the reconciler.
- pub `register_computation_graph_constructor` function L48-55 — `(graph_name: String, constructor: F)` — Register a computation graph constructor in the global registry.
- pub `global_computation_graph_registry` function L58-60 — `() -> GlobalComputationGraphRegistry` — Get a reference to the global computation graph registry.
- pub `list_registered_graphs` function L63-66 — `() -> Vec<String>` — List all registered computation graph names.
- pub `deregister_computation_graph` function L69-73 — `(graph_name: &str)` — Remove a computation graph from the global registry.
-  `GLOBAL_COMPUTATION_GRAPH_REGISTRY` variable L42-43 — `: Lazy<GlobalComputationGraphRegistry>` — Mirrors the global workflow/task registries used by the reconciler.
-  `tests` module L76-95 — `-` — Mirrors the global workflow/task registries used by the reconciler.
-  `test_register_and_list` function L81-94 — `()` — Mirrors the global workflow/task registries used by the reconciler.

#### crates/cloacina/src/computation_graph/mod.rs

- pub `accumulator` module L26 — `-` — # Computation Graph Runtime Types
- pub `global_registry` module L27 — `-` — - [`SourceName`] — identifies an accumulator source
- pub `packaging_bridge` module L28 — `-` — - [`SourceName`] — identifies an accumulator source
- pub `reactor` module L29 — `-` — - [`SourceName`] — identifies an accumulator source
- pub `registry` module L30 — `-` — - [`SourceName`] — identifies an accumulator source
- pub `scheduler` module L31 — `-` — - [`SourceName`] — identifies an accumulator source
- pub `stream_backend` module L32 — `-` — - [`SourceName`] — identifies an accumulator source
- pub `types` module L33 — `-` — - [`SourceName`] — identifies an accumulator source

#### crates/cloacina/src/computation_graph/packaging_bridge.rs

- pub `build_declaration_from_ffi` function L115-178 — `( graph_meta: &GraphPackageMetadata, library_data: Vec<u8>, ) -> ComputationGrap...` — Convert FFI graph metadata + library data into a `ComputationGraphDeclaration`
- pub `PassthroughAccumulatorFactory` struct L262 — `-` — A generic passthrough accumulator factory for FFI-loaded packages.
- pub `StreamBackendAccumulatorFactory` struct L315-318 — `{ config: std::collections::HashMap<String, String> }` — A stream-backed accumulator factory for FFI-loaded packages.
- pub `new` function L321-323 — `(config: std::collections::HashMap<String, String>) -> Self` — `execute_graph()` via fidius FFI.
-  `LoadedGraphPlugin` struct L46-50 — `{ handle: std::sync::Mutex<fidius_host::PluginHandle>, _temp_dir: tempfile::Temp...` — A persistent handle to a loaded FFI graph plugin.
-  `LoadedGraphPlugin` type L54 — `impl Send for LoadedGraphPlugin` — `execute_graph()` via fidius FFI.
-  `LoadedGraphPlugin` type L55 — `impl Sync for LoadedGraphPlugin` — `execute_graph()` via fidius FFI.
-  `LoadedGraphPlugin` type L57-108 — `= LoadedGraphPlugin` — `execute_graph()` via fidius FFI.
-  `load` function L60-93 — `(library_data: &[u8]) -> Result<Self, String>` — Load a graph plugin from library bytes.
-  `execute_graph` function L96-107 — `( &self, request: GraphExecutionRequest, ) -> Result<cloacina_workflow_plugin::G...` — Call execute_graph (method index 3) on the loaded plugin.
-  `execute_graph_via_ffi` function L181-255 — `(plugin: &Arc<LoadedGraphPlugin>, cache: &InputCache) -> GraphResult` — Execute a computation graph via FFI using the pre-loaded plugin handle.
-  `GenericPassthroughAccumulator` struct L264 — `-` — `execute_graph()` via fidius FFI.
-  `GenericPassthroughAccumulator` type L267-273 — `= GenericPassthroughAccumulator` — `execute_graph()` via fidius FFI.
-  `Output` type L268 — `= Vec<u8>` — `execute_graph()` via fidius FFI.
-  `process` function L270-272 — `(&mut self, event: Vec<u8>) -> Option<Vec<u8>>` — `execute_graph()` via fidius FFI.
-  `PassthroughAccumulatorFactory` type L275-307 — `impl AccumulatorFactory for PassthroughAccumulatorFactory` — `execute_graph()` via fidius FFI.
-  `spawn` function L276-306 — `( &self, name: String, boundary_tx: mpsc::Sender<(SourceName, Vec<u8>)>, shutdow...` — `execute_graph()` via fidius FFI.
-  `StreamBackendAccumulatorFactory` type L320-324 — `= StreamBackendAccumulatorFactory` — `execute_graph()` via fidius FFI.
-  `KafkaEventSource` struct L328-334 — `{ broker_var: String, topic: String, group: String, extra: std::collections::Has...` — EventSource that reads raw bytes from a Kafka topic.
-  `KafkaEventSource` type L338-394 — `= KafkaEventSource` — `execute_graph()` via fidius FFI.
-  `run` function L339-393 — `( self, events: mpsc::Sender<Vec<u8>>, mut shutdown: watch::Receiver<bool>, ) ->...` — `execute_graph()` via fidius FFI.
-  `StreamBackendAccumulatorFactory` type L396-469 — `impl AccumulatorFactory for StreamBackendAccumulatorFactory` — `execute_graph()` via fidius FFI.
-  `spawn` function L397-468 — `( &self, name: String, boundary_tx: mpsc::Sender<(SourceName, Vec<u8>)>, shutdow...` — `execute_graph()` via fidius FFI.
-  `tests` module L472-539 — `-` — `execute_graph()` via fidius FFI.
-  `test_build_declaration_from_ffi_metadata` function L477-505 — `()` — `execute_graph()` via fidius FFI.
-  `test_reaction_mode_parsing` function L508-538 — `()` — `execute_graph()` via fidius FFI.

#### crates/cloacina/src/computation_graph/reactor.rs

- pub `ReactorHealth` enum L43-55 — `Starting | Warming | Live | Degraded` — Health state of a reactor.
- pub `reactor_health_channel` function L69-71 — `() -> (watch::Sender<ReactorHealth>, watch::Receiver<ReactorHealth>)` — Create a reactor health reporting channel.
- pub `ReactionCriteria` enum L75-80 — `WhenAny | WhenAll` — Reaction criteria — when to fire the graph.
- pub `InputStrategy` enum L84-89 — `Latest | Sequential` — Input strategy — how the reactor handles data between executions.
- pub `DirtyFlags` struct L93-95 — `{ flags: HashMap<SourceName, bool> }` — Dirty flags — one boolean per source.
- pub `new` function L98-102 — `() -> Self` — See CLOACI-S-0005 for the full specification.
- pub `with_sources` function L108-114 — `(sources: &[SourceName]) -> Self` — Create dirty flags pre-seeded with expected source names (all initially false).
- pub `set` function L116-118 — `(&mut self, source: SourceName, dirty: bool)` — See CLOACI-S-0005 for the full specification.
- pub `any_set` function L120-122 — `(&self) -> bool` — See CLOACI-S-0005 for the full specification.
- pub `all_set` function L124-126 — `(&self) -> bool` — See CLOACI-S-0005 for the full specification.
- pub `clear_all` function L128-132 — `(&mut self)` — See CLOACI-S-0005 for the full specification.
- pub `StrategySignal` enum L143-148 — `BoundaryReceived | ForceFire` — Signals sent from receiver to executor.
- pub `ManualCommand` enum L152-157 — `ForceFire | FireWith` — Manual commands accepted by the reactor.
- pub `ReactorCommand` enum L162-168 — `ForceFire | FireWith | GetState | Pause | Resume` — Commands sent by WebSocket operators to a reactor.
- pub `ReactorResponse` enum L173-179 — `Fired | State | Paused | Resumed | Error` — Responses sent back to WebSocket operators.
- pub `ReactorHandle` struct L185-190 — `{ cache: Arc<RwLock<InputCache>>, paused: Arc<AtomicBool> }` — Handle to a running reactor — exposes shared state for WebSocket queries.
- pub `get_state` function L194-197 — `(&self) -> HashMap<String, String>` — Read the current cache as a JSON-friendly map.
- pub `is_paused` function L200-202 — `(&self) -> bool` — Check if the reactor is paused.
- pub `pause` function L205-207 — `(&self)` — Pause the reactor (stop executing, continue accepting boundaries).
- pub `resume` function L210-212 — `(&self)` — Resume the reactor.
- pub `Reactor` struct L220-252 — `{ graph: CompiledGraphFn, criteria: ReactionCriteria, input_strategy: InputStrat...` — The Reactor.
- pub `new` function L255-279 — `( graph: CompiledGraphFn, criteria: ReactionCriteria, input_strategy: InputStrat...` — See CLOACI-S-0005 for the full specification.
- pub `with_batch_flush_senders` function L282-285 — `(mut self, senders: Vec<mpsc::Sender<()>>) -> Self` — Add batch flush senders — reactor will signal these after each graph execution.
- pub `with_graph_name` function L288-291 — `(mut self, name: String) -> Self` — Set the graph name (used as key for DAL persistence).
- pub `with_dal` function L294-297 — `(mut self, dal: crate::dal::unified::DAL) -> Self` — Set the DAL handle for cache persistence.
- pub `with_health` function L300-303 — `(mut self, health: watch::Sender<ReactorHealth>) -> Self` — Set the health reporter channel.
- pub `with_expected_sources` function L309-312 — `(mut self, sources: Vec<SourceName>) -> Self` — Set the expected source names for WhenAll criteria.
- pub `with_accumulator_health` function L315-324 — `( mut self, rxs: Vec<( String, watch::Receiver<super::accumulator::AccumulatorHe...` — Set accumulator health receivers for startup gating and degraded mode.
- pub `handle` function L330-335 — `(&self) -> ReactorHandle` — Get a handle to this reactor's shared state.
- pub `run` function L338-654 — `(mut self)` — Run the reactor.
-  `ReactorHealth` type L57-66 — `= ReactorHealth` — See CLOACI-S-0005 for the full specification.
-  `fmt` function L58-65 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — See CLOACI-S-0005 for the full specification.
-  `DirtyFlags` type L97-133 — `= DirtyFlags` — See CLOACI-S-0005 for the full specification.
-  `DirtyFlags` type L135-139 — `impl Default for DirtyFlags` — See CLOACI-S-0005 for the full specification.
-  `default` function L136-138 — `() -> Self` — See CLOACI-S-0005 for the full specification.
-  `ReactorHandle` type L192-213 — `= ReactorHandle` — See CLOACI-S-0005 for the full specification.
-  `Reactor` type L254-655 — `= Reactor` — See CLOACI-S-0005 for the full specification.
-  `persist_reactor_state` function L658-714 — `( dal: &Option<crate::dal::unified::DAL>, graph_name: &str, cache: &Arc<RwLock<I...` — Persist reactor state to DAL (best-effort, logs on failure).
-  `tests` module L717-895 — `-` — See CLOACI-S-0005 for the full specification.
-  `test_dirty_flags_when_any` function L721-730 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_dirty_flags_when_all` function L733-741 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_dirty_flags_clear_all` function L744-752 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_dirty_flags_empty_all_set` function L755-759 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_reactor_fires_on_boundary` function L762-804 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_reactor_manual_force_fire` function L807-843 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_reactor_cache_snapshot_isolation` function L846-894 — `()` — See CLOACI-S-0005 for the full specification.

#### crates/cloacina/src/computation_graph/registry.rs

- pub `RegistryError` enum L35-56 — `AccumulatorNotFound | ReactorNotFound | AccumulatorSendFailed | ReactorSendFaile...` — Errors from registry operations.
- pub `ReactorOp` enum L61-68 — `ForceFire | FireWith | GetState | Pause | Resume | GetHealth` — Operations that can be performed on a reactor via WebSocket.
- pub `KeyContext` struct L71-75 — `{ key_id: &'a uuid::Uuid, tenant_id: Option<&'a str>, is_admin: bool }` — Caller identity for authorization checks.
- pub `AccumulatorAuthPolicy` struct L79-86 — `{ allow_all_authenticated: bool, allowed_tenants: Vec<String>, allowed_producers...` — Authorization policy for an accumulator endpoint.
- pub `ReactorAuthPolicy` struct L90-100 — `{ allow_all_authenticated: bool, allowed_tenants: Vec<String>, allowed_operators...` — Authorization policy for a reactor endpoint.
- pub `allow_all` function L104-110 — `() -> Self` — Create a policy that allows any authenticated key (global/single-tenant).
- pub `for_tenant` function L113-119 — `(tenant_id: &str) -> Self` — Create a policy scoped to a specific tenant.
- pub `is_authorized` function L122-133 — `(&self, ctx: &KeyContext) -> bool` — Check if a key is authorized.
- pub `allow_all` function L138-145 — `() -> Self` — Create a policy that allows any authenticated key (global/single-tenant).
- pub `for_tenant` function L148-155 — `(tenant_id: &str) -> Self` — Create a policy scoped to a specific tenant.
- pub `is_authorized` function L158-169 — `(&self, ctx: &KeyContext) -> bool` — Check if a key is authorized to connect.
- pub `is_operation_permitted` function L172-184 — `(&self, ctx: &KeyContext, op: &ReactorOp) -> bool` — Check if a key is authorized for a specific operation.
- pub `EndpointRegistry` struct L192-194 — `{ inner: Arc<RwLock<RegistryInner>> }` — Registry mapping endpoint names to channel senders.
- pub `new` function L212-223 — `() -> Self` — under the same name all receive the message.
- pub `register_accumulator` function L229-236 — `(&self, name: String, sender: mpsc::Sender<Vec<u8>>)` — Register an accumulator's socket sender under a name.
- pub `register_reactor` function L239-248 — `( &self, name: String, sender: mpsc::Sender<ManualCommand>, handle: ReactorHandl...` — Register a reactor's manual command sender and shared handle.
- pub `deregister_accumulator` function L251-254 — `(&self, name: &str)` — Deregister all accumulators under a name.
- pub `deregister_reactor` function L257-261 — `(&self, name: &str)` — Deregister a reactor by name.
- pub `get_reactor_handle` function L264-267 — `(&self, name: &str) -> Option<ReactorHandle>` — Get a reactor's shared handle (for GetState/Pause/Resume).
- pub `set_accumulator_policy` function L270-273 — `(&self, name: String, policy: AccumulatorAuthPolicy)` — Set the auth policy for an accumulator endpoint.
- pub `set_reactor_policy` function L276-279 — `(&self, name: String, policy: ReactorAuthPolicy)` — Set the auth policy for a reactor endpoint.
- pub `check_accumulator_auth` function L285-301 — `( &self, name: &str, ctx: &KeyContext<'_>, ) -> Result<(), RegistryError>` — Check if a key is authorized for an accumulator endpoint.
- pub `check_reactor_auth` function L304-320 — `( &self, name: &str, ctx: &KeyContext<'_>, ) -> Result<(), RegistryError>` — Check if a key is authorized for a reactor endpoint.
- pub `check_reactor_op_auth` function L323-343 — `( &self, name: &str, ctx: &KeyContext<'_>, op: &ReactorOp, ) -> Result<(), Regis...` — Check if a key is authorized for a specific reactor operation.
- pub `send_to_accumulator` function L349-393 — `( &self, name: &str, bytes: Vec<u8>, ) -> Result<usize, RegistryError>` — Send bytes to all accumulators registered under `name`.
- pub `send_to_reactor` function L396-413 — `( &self, name: &str, command: ManualCommand, ) -> Result<(), RegistryError>` — Send a manual command to a reactor.
- pub `list_accumulators` function L416-419 — `(&self) -> Vec<String>` — List all registered accumulator names.
- pub `list_reactors` function L422-425 — `(&self) -> Vec<String>` — List all registered reactor names.
- pub `accumulator_count` function L428-431 — `(&self, name: &str) -> usize` — Get the number of accumulators registered under a name.
- pub `register_accumulator_health` function L434-441 — `( &self, name: String, health_rx: watch::Receiver<AccumulatorHealth>, )` — Register a health watch receiver for an accumulator.
- pub `get_accumulator_health` function L444-450 — `(&self, name: &str) -> Option<AccumulatorHealth>` — Get the current health of an accumulator.
- pub `list_accumulators_with_health` function L453-467 — `(&self) -> Vec<(String, AccumulatorHealth)>` — List all accumulators with their current health status.
-  `AccumulatorAuthPolicy` type L102-134 — `= AccumulatorAuthPolicy` — under the same name all receive the message.
-  `ReactorAuthPolicy` type L136-185 — `= ReactorAuthPolicy` — under the same name all receive the message.
-  `RegistryInner` struct L196-209 — `{ accumulators: HashMap<String, Vec<mpsc::Sender<Vec<u8>>>>, reactors: HashMap<S...` — under the same name all receive the message.
-  `EndpointRegistry` type L211-468 — `= EndpointRegistry` — under the same name all receive the message.
-  `EndpointRegistry` type L470-474 — `impl Default for EndpointRegistry` — under the same name all receive the message.
-  `default` function L471-473 — `() -> Self` — under the same name all receive the message.
-  `tests` module L477-793 — `-` — under the same name all receive the message.
-  `dummy_handle` function L481-486 — `() -> ReactorHandle` — under the same name all receive the message.
-  `test_register_send_deregister_accumulator` function L489-512 — `()` — under the same name all receive the message.
-  `test_broadcast_to_multiple_accumulators` function L515-538 — `()` — under the same name all receive the message.
-  `test_send_to_unregistered_accumulator` function L541-548 — `()` — under the same name all receive the message.
-  `test_register_send_deregister_reactor` function L551-574 — `()` — under the same name all receive the message.
-  `test_send_to_unregistered_reactor` function L577-584 — `()` — under the same name all receive the message.
-  `test_closed_accumulator_channel_pruned` function L587-613 — `()` — under the same name all receive the message.
-  `test_list_accumulators_and_reactors` function L616-633 — `()` — under the same name all receive the message.
-  `test_accumulator_auth_deny_by_default` function L636-650 — `()` — under the same name all receive the message.
-  `test_accumulator_auth_authorized_key` function L653-691 — `()` — under the same name all receive the message.
-  `test_accumulator_auth_tenant_scoped` function L694-748 — `()` — under the same name all receive the message.
-  `test_reactor_auth_with_operation_permissions` function L751-792 — `()` — under the same name all receive the message.

#### crates/cloacina/src/computation_graph/scheduler.rs

- pub `ComputationGraphDeclaration` struct L40-49 — `{ name: String, accumulators: Vec<AccumulatorDeclaration>, reactor: ReactorDecla...` — Declaration of a computation graph to be loaded by the Reactive Scheduler.
- pub `AccumulatorDeclaration` struct L53-58 — `{ name: String, factory: Arc<dyn AccumulatorFactory> }` — Declaration for a single accumulator.
- pub `AccumulatorSpawnConfig` struct L61-68 — `{ dal: Option<crate::dal::unified::DAL>, health_tx: Option<watch::Sender<Accumul...` — Configuration passed to [`AccumulatorFactory::spawn`] for resilience wiring.
- pub `AccumulatorFactory` interface L73-86 — `{ fn spawn() }` — Factory trait for creating accumulator instances.
- pub `ReactorDeclaration` struct L90-97 — `{ criteria: ReactionCriteria, strategy: InputStrategy, graph_fn: CompiledGraphFn...` — Declaration for the reactor.
- pub `GraphStatus` struct L101-108 — `{ name: String, accumulators: Vec<String>, reactor_paused: bool, running: bool, ...` — Status of a managed computation graph.
- pub `ReactiveScheduler` struct L147-154 — `{ registry: EndpointRegistry, graphs: Arc<RwLock<HashMap<String, RunningGraph>>>...` — The Reactive Scheduler.
- pub `new` function L157-163 — `(registry: EndpointRegistry) -> Self` — and restarts tasks on panic.
- pub `with_dal` function L166-172 — `(registry: EndpointRegistry, dal: crate::dal::unified::DAL) -> Self` — Create a scheduler with DAL support for persistence and health tracking.
- pub `load_graph` function L175-305 — `(&self, decl: ComputationGraphDeclaration) -> Result<(), String>` — Load and start a computation graph.
- pub `unload_graph` function L308-334 — `(&self, name: &str) -> Result<(), String>` — Unload and shut down a computation graph.
- pub `list_graphs` function L337-356 — `(&self) -> Vec<GraphStatus>` — List all loaded computation graphs with status.
- pub `check_and_restart_failed` function L363-619 — `(&self) -> usize` — Check all graphs for crashed tasks and restart them.
- pub `start_supervision` function L624-649 — `( self: &Arc<Self>, mut shutdown_rx: watch::Receiver<bool>, check_interval: std:...` — Start a background supervision loop that checks for crashed tasks.
- pub `shutdown_all` function L674-685 — `(&self)` — Graceful shutdown of all graphs.
-  `RunningGraph` struct L111-132 — `{ shutdown_tx: watch::Sender<bool>, shutdown_rx: watch::Receiver<bool>, boundary...` — State for a running computation graph.
-  `MAX_RECOVERY_ATTEMPTS` variable L135 — `: u32` — Maximum consecutive failures before a component is permanently abandoned.
-  `BACKOFF_BASE_SECS` variable L138 — `: u64` — Base delay for exponential backoff (doubles on each failure, capped at 60s).
-  `BACKOFF_MAX_SECS` variable L141 — `: u64` — Maximum backoff delay.
-  `SUCCESS_RESET_SECS` variable L144 — `: u64` — Duration of successful operation before failure counter resets.
-  `ReactiveScheduler` type L156-686 — `= ReactiveScheduler` — and restarts tasks on panic.
-  `record_recovery_event` function L652-671 — `(&self, component: &str, attempt: u32, backoff_secs: u64)` — Record a recovery event in the DAL (best-effort, logs on failure).
-  `tests` module L689-865 — `-` — and restarts tasks on panic.
-  `TestEvent` struct L700-702 — `{ value: f64 }` — and restarts tasks on panic.
-  `TestAccumulatorFactory` struct L705 — `-` — A simple passthrough accumulator for testing.
-  `TestAccumulatorFactory` type L707-749 — `impl AccumulatorFactory for TestAccumulatorFactory` — and restarts tasks on panic.
-  `spawn` function L708-748 — `( &self, name: String, boundary_tx: mpsc::Sender<(SourceName, Vec<u8>)>, shutdow...` — and restarts tasks on panic.
-  `Passthrough` struct L717 — `-` — and restarts tasks on panic.
-  `Passthrough` type L720-725 — `impl Accumulator for Passthrough` — and restarts tasks on panic.
-  `Output` type L721 — `= TestEvent` — and restarts tasks on panic.
-  `process` function L722-724 — `(&mut self, event: Vec<u8>) -> Option<TestEvent>` — and restarts tasks on panic.
-  `test_load_graph_push_event_fires` function L752-799 — `()` — and restarts tasks on panic.
-  `test_unload_graph_deregisters` function L802-838 — `()` — and restarts tasks on panic.
-  `test_duplicate_load_rejected` function L841-864 — `()` — and restarts tasks on panic.

#### crates/cloacina/src/computation_graph/stream_backend.rs

- pub `StreamConfig` struct L28-33 — `{ broker_url: String, topic: String, group: String, extra: HashMap<String, Strin...` — Configuration for connecting to a stream broker.
- pub `RawMessage` struct L37-41 — `{ payload: Vec<u8>, offset: u64, timestamp: Option<i64> }` — A raw message from a stream broker.
- pub `StreamError` enum L45-54 — `Connection | Receive | Commit | NotFound` — Errors from stream backend operations.
- pub `StreamBackend` interface L58-72 — `{ fn connect(), fn recv(), fn commit(), fn current_offset() }` — Trait for pluggable stream broker backends (Kafka, Redpanda, Iggy, etc.).
- pub `StreamBackendFactory` type L75-82 — `= Box< dyn Fn( StreamConfig, ) -> Pin<Box<dyn Future<Output = Result<Box<dyn Str...` — Factory function type for creating stream backends.
- pub `StreamBackendRegistry` struct L85-87 — `{ backends: HashMap<String, StreamBackendFactory> }` — Registry of stream backend factories.
- pub `new` function L90-94 — `() -> Self` — StreamBackend trait and registry for pluggable broker backends.
- pub `register` function L97-99 — `(&mut self, type_name: &str, factory: StreamBackendFactory)` — Register a backend factory by type name.
- pub `create` function L102-111 — `( &self, type_name: &str, config: StreamConfig, ) -> Result<Box<dyn StreamBacken...` — Create a backend instance by type name.
- pub `has` function L114-116 — `(&self, type_name: &str) -> bool` — Check if a backend type is registered.
- pub `create_future` function L120-128 — `( &self, type_name: &str, config: StreamConfig, ) -> Option<Pin<Box<dyn Future<O...` — Get the creation future for a backend type without holding the lock across await.
- pub `global_stream_registry` function L142-144 — `() -> &'static Mutex<StreamBackendRegistry>` — Get a reference to the global stream backend registry.
- pub `register_stream_backend` function L147-152 — `(type_name: &str, factory: StreamBackendFactory)` — Register a backend in the global registry.
- pub `MockBackend` struct L159-163 — `{ receiver: tokio::sync::mpsc::Receiver<Vec<u8>>, offset: u64, committed_offset:...` — In-memory mock stream backend for testing without a real broker.
- pub `MockBackendProducer` struct L167-169 — `{ sender: tokio::sync::mpsc::Sender<Vec<u8>> }` — Handle for pushing messages into a MockBackend.
- pub `send` function L173-178 — `(&self, payload: Vec<u8>) -> Result<(), StreamError>` — Push a message into the mock backend.
- pub `mock_backend` function L182-192 — `(capacity: usize) -> (MockBackend, MockBackendProducer)` — Create a mock backend + producer pair.
- pub `register_mock_backend` function L232-243 — `()` — Register the mock backend in the global registry.
- pub `kafka` module L250-375 — `-` — StreamBackend trait and registry for pluggable broker backends.
- pub `KafkaStreamBackend` struct L261-266 — `{ consumer: StreamConsumer, topic: String, offset: u64, committed_offset: u64 }` — Kafka stream backend using rdkafka (librdkafka wrapper).
- pub `register_kafka_backend` function L364-374 — `()` — Register the Kafka backend in the global registry.
-  `StreamBackendRegistry` type L89-129 — `= StreamBackendRegistry` — StreamBackend trait and registry for pluggable broker backends.
-  `StreamBackendRegistry` type L131-135 — `impl Default for StreamBackendRegistry` — StreamBackend trait and registry for pluggable broker backends.
-  `default` function L132-134 — `() -> Self` — StreamBackend trait and registry for pluggable broker backends.
-  `GLOBAL_REGISTRY` variable L138-139 — `: Lazy<Mutex<StreamBackendRegistry>>` — Global stream backend registry.
-  `MockBackendProducer` type L171-179 — `= MockBackendProducer` — StreamBackend trait and registry for pluggable broker backends.
-  `MockBackend` type L195-229 — `impl StreamBackend for MockBackend` — StreamBackend trait and registry for pluggable broker backends.
-  `connect` function L196-201 — `(_config: &StreamConfig) -> Result<Self, StreamError>` — StreamBackend trait and registry for pluggable broker backends.
-  `recv` function L203-215 — `(&mut self) -> Result<RawMessage, StreamError>` — StreamBackend trait and registry for pluggable broker backends.
-  `commit` function L217-220 — `(&mut self) -> Result<(), StreamError>` — StreamBackend trait and registry for pluggable broker backends.
-  `current_offset` function L222-228 — `(&self) -> Option<u64>` — StreamBackend trait and registry for pluggable broker backends.
-  `KafkaStreamBackend` type L269-361 — `impl StreamBackend for KafkaStreamBackend` — StreamBackend trait and registry for pluggable broker backends.
-  `connect` function L270-310 — `(config: &StreamConfig) -> Result<Self, StreamError>` — StreamBackend trait and registry for pluggable broker backends.
-  `recv` function L312-336 — `(&mut self) -> Result<RawMessage, StreamError>` — StreamBackend trait and registry for pluggable broker backends.
-  `commit` function L338-352 — `(&mut self) -> Result<(), StreamError>` — StreamBackend trait and registry for pluggable broker backends.
-  `current_offset` function L354-360 — `(&self) -> Option<u64>` — StreamBackend trait and registry for pluggable broker backends.
-  `tests` module L378-442 — `-` — StreamBackend trait and registry for pluggable broker backends.
-  `test_mock_backend_recv` function L382-395 — `()` — StreamBackend trait and registry for pluggable broker backends.
-  `test_mock_backend_commit` function L398-408 — `()` — StreamBackend trait and registry for pluggable broker backends.
-  `test_registry_lookup` function L411-424 — `()` — StreamBackend trait and registry for pluggable broker backends.
-  `test_registry_not_found` function L427-441 — `()` — StreamBackend trait and registry for pluggable broker backends.

#### crates/cloacina/src/computation_graph/types.rs

- pub `SourceName` struct L27 — `-` — Identifies an accumulator source by name.
- pub `new` function L30-32 — `(name: impl Into<String>) -> Self` — Core types for computation graph execution.
- pub `as_str` function L34-36 — `(&self) -> &str` — Core types for computation graph execution.
- pub `InputCache` struct L68-70 — `{ entries: HashMap<SourceName, Vec<u8>> }` — The input cache holds the last-seen serialized boundary per source.
- pub `new` function L73-77 — `() -> Self` — Core types for computation graph execution.
- pub `update` function L80-82 — `(&mut self, source: SourceName, bytes: Vec<u8>)` — Update the cached value for a source.
- pub `get` function L88-91 — `(&self, name: &str) -> Option<Result<T, GraphError>>` — Get and deserialize a cached value by source name.
- pub `has` function L94-96 — `(&self, name: &str) -> bool` — Check if a source has an entry in the cache.
- pub `get_raw` function L99-103 — `(&self, name: &str) -> Option<&[u8]>` — Get the raw bytes for a source (for forwarding without deserialization).
- pub `snapshot` function L106-108 — `(&self) -> InputCache` — Create a snapshot (clone) of the cache for the executor.
- pub `len` function L111-113 — `(&self) -> usize` — Number of sources in the cache.
- pub `is_empty` function L116-118 — `(&self) -> bool` — Whether the cache is empty.
- pub `replace_all` function L121-123 — `(&mut self, other: InputCache)` — Replace all entries (used for manual fire-with-state).
- pub `sources` function L126-128 — `(&self) -> Vec<&SourceName>` — List all source names in the cache.
- pub `entries_raw` function L131-133 — `(&self) -> &HashMap<SourceName, Vec<u8>>` — Get a reference to the raw entries map (for serialization/persistence).
- pub `entries_as_json` function L139-154 — `(&self) -> std::collections::HashMap<String, String>` — Return entries as a JSON-friendly map (base64-encoded raw bytes per source).
- pub `serialize` function L172-181 — `(value: &T) -> Result<Vec<u8>, GraphError>` — Core types for computation graph execution.
- pub `deserialize` function L184-193 — `(bytes: &[u8]) -> Result<T, GraphError>` — Deserialize bytes to a value using the build-profile-appropriate format.
- pub `GraphResult` enum L201-206 — `Completed | Error` — Result of executing a compiled computation graph.
- pub `completed` function L210-212 — `(outputs: Vec<Box<dyn Any + Send>>) -> Self` — Create a completed result with terminal node outputs.
- pub `completed_empty` function L215-219 — `() -> Self` — Create a completed result with no outputs (all branches short-circuited).
- pub `error` function L222-224 — `(err: GraphError) -> Self` — Create an error result.
- pub `is_completed` function L227-229 — `(&self) -> bool` — Check if the graph completed successfully.
- pub `is_error` function L232-234 — `(&self) -> bool` — Check if the graph errored.
- pub `GraphError` enum L239-254 — `Serialization | Deserialization | MissingInput | NodeExecution | Execution` — Errors that can occur during graph execution.
-  `SourceName` type L29-37 — `= SourceName` — Core types for computation graph execution.
-  `SourceName` type L39-43 — `= SourceName` — Core types for computation graph execution.
-  `fmt` function L40-42 — `(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result` — Core types for computation graph execution.
-  `SourceName` type L45-49 — `= SourceName` — Core types for computation graph execution.
-  `from` function L46-48 — `(s: &str) -> Self` — Core types for computation graph execution.
-  `SourceName` type L51-55 — `= SourceName` — Core types for computation graph execution.
-  `from` function L52-54 — `(s: String) -> Self` — Core types for computation graph execution.
-  `InputCache` type L72-155 — `= InputCache` — Core types for computation graph execution.
-  `InputCache` type L157-161 — `impl Default for InputCache` — Core types for computation graph execution.
-  `default` function L158-160 — `() -> Self` — Core types for computation graph execution.
-  `hex_encode` function L168-170 — `(bytes: &[u8]) -> String` — Serialize a value to bytes using the build-profile-appropriate format.
-  `GraphResult` type L208-235 — `= GraphResult` — Core types for computation graph execution.
-  `tests` module L257-439 — `-` — Core types for computation graph execution.
-  `TestData` struct L262-265 — `{ value: f64, label: String }` — Core types for computation graph execution.
-  `test_input_cache_update_and_get` function L268-280 — `()` — Core types for computation graph execution.
-  `test_input_cache_missing_source` function L283-287 — `()` — Core types for computation graph execution.
-  `test_input_cache_overwrite` function L290-307 — `()` — Core types for computation graph execution.
-  `test_input_cache_snapshot` function L310-332 — `()` — Core types for computation graph execution.
-  `test_input_cache_has` function L335-342 — `()` — Core types for computation graph execution.
-  `test_input_cache_len_and_empty` function L345-356 — `()` — Core types for computation graph execution.
-  `test_serialization_round_trip` function L359-367 — `()` — Core types for computation graph execution.
-  `test_serialization_round_trip_primitives` function L370-385 — `()` — Core types for computation graph execution.
-  `test_deserialization_type_mismatch` function L388-392 — `()` — Core types for computation graph execution.
-  `test_graph_result_completed` function L395-399 — `()` — Core types for computation graph execution.
-  `test_graph_result_completed_empty` function L402-408 — `()` — Core types for computation graph execution.
-  `test_graph_result_error` function L411-415 — `()` — Core types for computation graph execution.
-  `test_source_name_equality` function L418-424 — `()` — Core types for computation graph execution.
-  `test_replace_all` function L427-438 — `()` — Core types for computation graph execution.

### crates/cloacina/src

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/context.rs

- pub `ContextDbExt` interface L139-183 — `{ fn from_db_record(), fn to_new_db_record(), fn to_db_record() }` — Extension trait providing database operations for Context.
-  `from_db_record` function L189-192 — `(db_context: &DbContext) -> Result<Context<T>, ContextError>` — ```
-  `to_new_db_record` function L194-198 — `(&self) -> Result<NewDbContext, ContextError>` — ```
-  `to_db_record` function L200-210 — `(&self, id: Uuid) -> Result<DbContext, ContextError>` — ```
-  `tests` module L214-295 — `-` — ```
-  `setup_test_context` function L219-222 — `() -> Context<i32>` — ```
-  `test_context_operations` function L225-250 — `()` — ```
-  `test_context_serialization` function L253-261 — `()` — ```
-  `test_context_db_conversion` function L264-294 — `()` — ```

#### crates/cloacina/src/cron_evaluator.rs

- pub `CronError` enum L51-67 — `InvalidExpression | InvalidTimezone | NoNextExecution | CronParsingError` — Errors that can occur during cron evaluation.
- pub `CronEvaluator` struct L92-101 — `{ cron: Cron, timezone: Tz, expression: String, timezone_str: String }` — Timezone-aware cron expression evaluator.
- pub `new` function L130-147 — `(cron_expr: &str, timezone_str: &str) -> Result<Self, CronError>` — Creates a new cron evaluator with the specified expression and timezone.
- pub `next_execution` function L176-188 — `(&self, after: DateTime<Utc>) -> Result<DateTime<Utc>, CronError>` — Finds the next execution time after the given timestamp.
- pub `next_executions` function L216-236 — `( &self, after: DateTime<Utc>, limit: usize, ) -> Result<Vec<DateTime<Utc>>, Cro...` — Finds multiple next execution times after the given timestamp.
- pub `executions_between` function L267-291 — `( &self, start: DateTime<Utc>, end: DateTime<Utc>, max_executions: usize, ) -> R...` — Finds all execution times between two timestamps.
- pub `expression` function L294-296 — `(&self) -> &str` — Returns the original cron expression string.
- pub `timezone_str` function L299-301 — `(&self) -> &str` — Returns the timezone string.
- pub `timezone` function L304-306 — `(&self) -> Tz` — Returns the timezone object.
- pub `validate_expression` function L315-321 — `(cron_expr: &str) -> Result<(), CronError>` — Validates a cron expression without creating an evaluator.
- pub `validate_timezone` function L330-335 — `(timezone_str: &str) -> Result<(), CronError>` — Validates a timezone string.
- pub `validate` function L345-349 — `(cron_expr: &str, timezone_str: &str) -> Result<(), CronError>` — Validates both cron expression and timezone.
-  `CronEvaluator` type L103-350 — `= CronEvaluator` — ```
-  `CronEvaluator` type L352-378 — `impl FromStr for CronEvaluator` — ```
-  `Err` type L353 — `= CronError` — ```
-  `from_str` function L368-377 — `(s: &str) -> Result<Self, Self::Err>` — Creates a CronEvaluator from a string in the format "expression@timezone"
-  `tests` module L381-531 — `-` — ```
-  `test_cron_evaluator_creation` function L386-390 — `()` — ```
-  `test_invalid_cron_expression` function L393-400 — `()` — ```
-  `test_invalid_timezone` function L403-407 — `()` — ```
-  `test_next_execution_utc` function L410-419 — `()` — ```
-  `test_next_execution_timezone` function L422-431 — `()` — ```
-  `test_next_executions` function L434-444 — `()` — ```
-  `test_executions_between` function L447-459 — `()` — ```
-  `test_validation_functions` function L462-472 — `()` — ```
-  `test_from_str` function L475-482 — `()` — ```
-  `test_executions_between_respects_max_limit` function L485-492 — `()` — ```
-  `test_executions_between_empty_range` function L495-505 — `()` — ```
-  `test_executions_between_multiple_days` function L508-518 — `()` — ```
-  `test_executions_between_timezone_aware` function L521-530 — `()` — ```

#### crates/cloacina/src/cron_recovery.rs

- pub `CronRecoveryConfig` struct L57-68 — `{ check_interval: Duration, lost_threshold_minutes: i32, max_recovery_age: Durat...` — Configuration for the cron recovery service.
- pub `CronRecoveryService` struct L87-94 — `{ dal: Arc<DAL>, executor: Arc<dyn WorkflowExecutor>, config: CronRecoveryConfig...` — Recovery service for lost cron executions.
- pub `new` function L104-117 — `( dal: Arc<DAL>, executor: Arc<dyn WorkflowExecutor>, config: CronRecoveryConfig...` — Creates a new cron recovery service.
- pub `with_defaults` function L120-126 — `( dal: Arc<DAL>, executor: Arc<dyn WorkflowExecutor>, shutdown: watch::Receiver<...` — Creates a new recovery service with default configuration.
- pub `run_recovery_loop` function L132-160 — `(&mut self) -> Result<(), WorkflowExecutionError>` — Runs the recovery service loop.
- pub `clear_recovery_attempts` function L366-370 — `(&self)` — Clears the recovery attempts cache.
- pub `get_recovery_attempts` function L373-379 — `( &self, execution_id: crate::database::UniversalUuid, ) -> usize` — Gets the current recovery attempts for an execution.
-  `CronRecoveryConfig` type L70-80 — `impl Default for CronRecoveryConfig` — - The execution is too old (beyond recovery window)
-  `default` function L71-79 — `() -> Self` — - The execution is too old (beyond recovery window)
-  `CronRecoveryService` type L96-380 — `= CronRecoveryService` — - The execution is too old (beyond recovery window)
-  `check_and_recover_lost_executions` function L163-195 — `(&self) -> Result<(), WorkflowExecutionError>` — Checks for lost executions and attempts to recover them.
-  `recover_execution` function L198-360 — `( &self, execution: &ScheduleExecution, ) -> Result<(), WorkflowExecutionError>` — Attempts to recover a single lost execution.
-  `tests` module L383-430 — `-` — - The execution is too old (beyond recovery window)
-  `test_recovery_config_default` function L387-394 — `()` — - The execution is too old (beyond recovery window)
-  `test_recovery_config_custom` function L397-411 — `()` — - The execution is too old (beyond recovery window)
-  `test_recovery_config_clone` function L414-420 — `()` — - The execution is too old (beyond recovery window)
-  `test_recovery_config_default_recovery_window` function L423-429 — `()` — - The execution is too old (beyond recovery window)

#### crates/cloacina/src/cron_trigger_scheduler.rs

- pub `SchedulerConfig` struct L64-75 — `{ cron_poll_interval: Duration, max_catchup_executions: usize, max_acceptable_de...` — Configuration for the unified scheduler.
- pub `Scheduler` struct L114-123 — `{ dal: Arc<DAL>, executor: Arc<dyn WorkflowExecutor>, config: SchedulerConfig, s...` — Unified scheduler for both cron and trigger-based workflow execution.
- pub `new` function L133-147 — `( dal: Arc<DAL>, executor: Arc<dyn WorkflowExecutor>, config: SchedulerConfig, s...` — Creates a new unified scheduler.
- pub `with_defaults` function L150-156 — `( dal: Arc<DAL>, executor: Arc<dyn WorkflowExecutor>, shutdown: watch::Receiver<...` — Creates a new unified scheduler with default configuration.
- pub `run_polling_loop` function L170-212 — `(&mut self) -> Result<(), WorkflowExecutionError>` — Runs the main polling loop.
- pub `register_trigger` function L780-793 — `( &self, trigger: &dyn Trigger, workflow_name: &str, ) -> Result<Schedule, Valid...` — Registers a trigger with the scheduler.
- pub `disable_trigger` function L796-807 — `(&self, trigger_name: &str) -> Result<(), ValidationError>` — Disables a trigger by name.
- pub `enable_trigger` function L810-821 — `(&self, trigger_name: &str) -> Result<(), ValidationError>` — Enables a trigger by name.
-  `SchedulerConfig` type L77-87 — `impl Default for SchedulerConfig` — ```
-  `default` function L78-86 — `() -> Self` — ```
-  `Scheduler` type L125-822 — `= Scheduler` — ```
-  `check_and_execute_cron_schedules` function L219-246 — `(&self) -> Result<(), WorkflowExecutionError>` — Checks for due cron schedules and executes them.
-  `process_cron_schedule` function L249-357 — `( &self, schedule: &Schedule, now: DateTime<Utc>, ) -> Result<(), WorkflowExecut...` — Processes a single cron schedule using the saga pattern.
-  `is_cron_schedule_active` function L360-372 — `(&self, schedule: &Schedule, now: DateTime<Utc>) -> bool` — Checks if a cron schedule is within its active time window.
-  `calculate_execution_times` function L375-420 — `( &self, schedule: &Schedule, now: DateTime<Utc>, ) -> Result<Vec<DateTime<Utc>>...` — Calculates execution times based on the schedule's catchup policy.
-  `calculate_next_run` function L423-442 — `( &self, schedule: &Schedule, after: DateTime<Utc>, ) -> Result<DateTime<Utc>, W...` — Calculates the next run time for a cron schedule.
-  `execute_cron_workflow` function L445-497 — `( &self, schedule: &Schedule, scheduled_time: DateTime<Utc>, ) -> Result<Univers...` — Executes a cron workflow by handing it off to the workflow executor.
-  `create_cron_execution_audit` function L500-521 — `( &self, schedule_id: UniversalUuid, scheduled_time: DateTime<Utc>, ) -> Result<...` — Creates an audit record for a cron execution.
-  `check_and_process_triggers` function L528-579 — `(&mut self) -> Result<(), WorkflowExecutionError>` — Checks all enabled triggers and processes those that are due.
-  `process_trigger` function L582-704 — `(&self, schedule: &Schedule) -> Result<(), TriggerError>` — Processes a single trigger schedule.
-  `create_trigger_execution_audit` function L707-733 — `( &self, schedule_id: UniversalUuid, context_hash: &str, ) -> Result<crate::mode...` — Creates an audit record for a trigger execution.
-  `execute_trigger_workflow` function L736-765 — `( &self, schedule: &Schedule, mut context: Context<serde_json::Value>, ) -> Resu...` — Executes a trigger workflow by handing it off to the workflow executor.
-  `tests` module L825-1113 — `-` — ```
-  `create_test_cron_schedule` function L829-850 — `(cron_expr: &str, timezone: &str) -> Schedule` — ```
-  `create_test_trigger_schedule` function L852-873 — `(trigger_name: &str) -> Schedule` — ```
-  `test_scheduler_config_default` function L876-883 — `()` — ```
-  `test_is_cron_schedule_active_no_window` function L886-906 — `()` — ```
-  `test_is_cron_schedule_active_with_start_date_future` function L909-919 — `()` — ```
-  `test_is_cron_schedule_active_with_end_date_past` function L922-932 — `()` — ```
-  `test_catchup_policy_from_schedule` function L935-940 — `()` — ```
-  `test_catchup_policy_run_all` function L943-949 — `()` — ```
-  `test_trigger_schedule_helpers` function L952-959 — `()` — ```
-  `test_trigger_schedule_trigger_name_fallback` function L962-974 — `()` — ```
-  `test_scheduler_config_custom` function L981-994 — `()` — ```
-  `test_scheduler_config_clone` function L997-1008 — `()` — ```
-  `test_scheduler_config_debug` function L1011-1016 — `()` — ```
-  `test_is_cron_schedule_active_both_bounds_containing_now` function L1023-1034 — `()` — ```
-  `test_is_cron_schedule_active_both_bounds_excluding_now` function L1037-1049 — `()` — ```
-  `test_catchup_policy_unknown_defaults_to_skip` function L1056-1059 — `()` — ```
-  `test_catchup_policy_none_defaults_to_skip` function L1062-1067 — `()` — ```
-  `test_catchup_policy_missing_defaults_correctly` function L1070-1076 — `()` — ```
-  `test_cron_schedule_helpers` function L1083-1090 — `()` — ```
-  `test_trigger_schedule_no_poll_interval` function L1093-1098 — `()` — ```
-  `test_trigger_schedule_allows_concurrent` function L1101-1105 — `()` — ```
-  `test_trigger_schedule_no_concurrent_flag_defaults_false` function L1108-1112 — `()` — ```

#### crates/cloacina/src/error.rs

- pub `ContextError` enum L132-153 — `Serialization | KeyNotFound | TypeMismatch | KeyExists | Database | ConnectionPo...` — Errors that can occur during context operations.
- pub `RegistrationError` enum L175-184 — `DuplicateTaskId | InvalidTaskId | RegistrationFailed` — Errors that can occur during task registration.
- pub `ValidationError` enum L191-249 — `CyclicDependency | MissingDependency | DuplicateTaskId | EmptyWorkflow | Invalid...` — Errors that can occur during Workflow and dependency validation.
- pub `ExecutorError` enum L265-301 — `Database | ConnectionPool | TaskNotFound | TaskExecution | Context | TaskTimeout...` — Errors that can occur during task execution.
- pub `WorkflowError` enum L313-337 — `DuplicateTask | TaskNotFound | InvalidDependency | CyclicDependency | Unreachabl...` — Errors that can occur during workflow construction and management.
- pub `SubgraphError` enum L344-350 — `TaskNotFound | UnsupportedOperation` — Errors that can occur when creating Workflow subgraphs.
-  `ContextError` type L155-168 — `= ContextError` — relevant context information to aid in troubleshooting and recovery.
-  `from` function L156-167 — `(err: cloacina_workflow::ContextError) -> Self` — relevant context information to aid in troubleshooting and recovery.
-  `ValidationError` type L251-255 — `= ValidationError` — relevant context information to aid in troubleshooting and recovery.
-  `from` function L252-254 — `(err: deadpool::managed::PoolError<deadpool_diesel::Error>) -> Self` — relevant context information to aid in troubleshooting and recovery.
-  `ContextError` type L257-261 — `= ContextError` — relevant context information to aid in troubleshooting and recovery.
-  `from` function L258-260 — `(err: deadpool::managed::PoolError<deadpool_diesel::Error>) -> Self` — relevant context information to aid in troubleshooting and recovery.
-  `ExecutorError` type L303-307 — `= ExecutorError` — relevant context information to aid in troubleshooting and recovery.
-  `from` function L304-306 — `(err: deadpool::managed::PoolError<deadpool_diesel::Error>) -> Self` — relevant context information to aid in troubleshooting and recovery.
-  `TaskError` type L353-376 — `= TaskError` — relevant context information to aid in troubleshooting and recovery.
-  `from` function L354-375 — `(error: ContextError) -> Self` — relevant context information to aid in troubleshooting and recovery.

#### crates/cloacina/src/graph.rs

- pub `TaskNode` struct L38-49 — `{ id: String, name: String, description: Option<String>, source_location: Option...` — Node data for tasks in the workflow graph
- pub `DependencyEdge` struct L53-60 — `{ dependency_type: String, weight: Option<f64>, metadata: HashMap<String, serde_...` — Edge data representing dependencies between tasks
- pub `WorkflowGraph` struct L74-79 — `{ graph: DiGraph<TaskNode, DependencyEdge>, task_index: HashMap<String, NodeInde...` — Main workflow graph structure using petgraph
- pub `new` function L83-88 — `() -> Self` — Create a new empty workflow graph
- pub `add_task` function L91-96 — `(&mut self, node: TaskNode) -> NodeIndex` — Add a task node to the graph
- pub `add_dependency` function L99-116 — `( &mut self, from_task_id: &str, to_task_id: &str, edge: DependencyEdge, ) -> Re...` — Add a dependency edge between tasks
- pub `get_task` function L119-123 — `(&self, task_id: &str) -> Option<&TaskNode>` — Get a task node by ID
- pub `task_ids` function L126-128 — `(&self) -> impl Iterator<Item = &str>` — Get an iterator over task IDs without allocation
- pub `task_count` function L131-133 — `(&self) -> usize` — Get the number of tasks in the graph (O(1))
- pub `has_cycles` function L136-138 — `(&self) -> bool` — Check if the graph has cycles
- pub `topological_sort` function L141-149 — `(&self) -> Result<Vec<String>, String>` — Get topological ordering of tasks
- pub `get_dependencies` function L152-161 — `(&self, task_id: &str) -> impl Iterator<Item = &str>` — Get an iterator over direct dependencies of a task
- pub `get_dependents` function L164-173 — `(&self, task_id: &str) -> impl Iterator<Item = &str>` — Get an iterator over tasks that depend on the given task
- pub `find_roots` function L176-189 — `(&self) -> impl Iterator<Item = &str>` — Get an iterator over root tasks (tasks with no dependencies)
- pub `find_leaves` function L192-205 — `(&self) -> impl Iterator<Item = &str>` — Get an iterator over leaf tasks (tasks with no dependents)
- pub `calculate_depths` function L208-248 — `(&self) -> HashMap<String, usize>` — Calculate the depth of each task (longest path from root)
- pub `find_parallel_groups` function L251-262 — `(&self) -> Vec<Vec<String>>` — Find parallel execution groups (tasks that can run simultaneously)
- pub `to_serializable` function L265-308 — `(&self) -> WorkflowGraphData` — Convert to serializable format
- pub `from_serializable` function L311-325 — `(data: &WorkflowGraphData) -> Result<Self, String>` — Create from serializable format
- pub `WorkflowGraphData` struct L336-343 — `{ nodes: Vec<GraphNode>, edges: Vec<GraphEdge>, metadata: GraphMetadata }` — Serializable representation of the workflow graph
- pub `GraphNode` struct L347-352 — `{ id: String, data: TaskNode }` — Serializable node representation
- pub `GraphEdge` struct L356-363 — `{ from: String, to: String, data: DependencyEdge }` — Serializable edge representation
- pub `GraphMetadata` struct L367-380 — `{ task_count: usize, edge_count: usize, has_cycles: bool, depth_levels: usize, r...` — Graph metadata and statistics
-  `DependencyEdge` type L62-70 — `impl Default for DependencyEdge` — - Graph algorithms for analysis and optimization
-  `default` function L63-69 — `() -> Self` — - Graph algorithms for analysis and optimization
-  `WorkflowGraph` type L81-326 — `= WorkflowGraph` — - Graph algorithms for analysis and optimization
-  `WorkflowGraph` type L328-332 — `impl Default for WorkflowGraph` — - Graph algorithms for analysis and optimization
-  `default` function L329-331 — `() -> Self` — - Graph algorithms for analysis and optimization
-  `tests` module L383-522 — `-` — - Graph algorithms for analysis and optimization
-  `test_workflow_graph_creation` function L387-424 — `()` — - Graph algorithms for analysis and optimization
-  `test_parallel_groups` function L427-456 — `()` — - Graph algorithms for analysis and optimization
-  `test_serialization` function L459-476 — `()` — - Graph algorithms for analysis and optimization
-  `test_task_count` function L479-500 — `()` — - Graph algorithms for analysis and optimization
-  `test_task_ids_iterator` function L503-521 — `()` — - Graph algorithms for analysis and optimization

#### crates/cloacina/src/inventory_entries.rs

- pub `TaskEntry` struct L44-50 — `{ namespace: fn() -> TaskNamespace, constructor: fn() -> Arc<dyn Task> }` — Task entry emitted by `#[task]`.
- pub `WorkflowEntry` struct L54-57 — `{ name: &'static str, constructor: fn() -> Workflow }` — Workflow entry emitted by `#[workflow]`.
- pub `TriggerEntry` struct L61-64 — `{ name: &'static str, constructor: fn() -> Arc<dyn Trigger> }` — Trigger entry emitted by `#[trigger]`.
- pub `ComputationGraphEntry` struct L68-71 — `{ name: &'static str, constructor: fn() -> ComputationGraphRegistration }` — Computation graph entry emitted by `#[computation_graph]`.
- pub `StreamBackendFactoryFn` type L80-83 — `= fn( StreamConfig, ) -> Pin<Box<dyn Future<Output = Result<Box<dyn StreamBacken...` — Stream-backend entry emitted by the stream-backend registration helper.
- pub `StreamBackendEntry` struct L85-88 — `{ type_name: &'static str, factory: StreamBackendFactoryFn }` — together with the removal of the global static registries.

#### crates/cloacina/src/lib.rs

- pub `prelude` module L453-486 — `-` — Prelude module for convenient imports.
- pub `computation_graph` module L490 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `context` module L491 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `cron_evaluator` module L492 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `cron_recovery` module L493 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `cron_trigger_scheduler` module L496 — `-` — Cron and event-trigger schedule management.
- pub `crypto` module L497 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `dal` module L498 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `database` module L499 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `dispatcher` module L500 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `error` module L501 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `execution_planner` module L504 — `-` — Task readiness evaluation, workflow processing, and stale claim sweeping.
- pub `executor` module L505 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `graph` module L506 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `inventory_entries` module L507 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `logging` module L508 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `models` module L509 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `packaging` module L510 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `python` module L511 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `registry` module L512 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `retry` module L513 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `runner` module L514 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `runtime` module L515 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `security` module L520 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `task` module L521 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `trigger` module L522 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `var` module L523 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `workflow` module L524 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `setup_test` function L533-535 — `()` — - [`retry`]: Retry policies and backoff strategies
-  `cloaca` function L601-663 — `(m: &Bound<'_, PyModule>) -> PyResult<()>` — - [`retry`]: Retry policies and backoff strategies

#### crates/cloacina/src/logging.rs

- pub `init_logging` function L136-146 — `(level: Option<Level>)` — Initializes the logging system with the specified log level.
- pub `init_test_logging` function L170-175 — `()` — Initializes the logging system for test environments.
- pub `mask_db_url` function L211-220 — `(url: &str) -> String` — Mask the password in a database URL for safe logging.
-  `tests` module L178-191 — `-` — - Test logging initialization is idempotent and safe to call multiple times
-  `test_logging_levels` function L183-190 — `()` — - Test logging initialization is idempotent and safe to call multiple times

#### crates/cloacina/src/runtime.rs

- pub `TaskConstructorFn` type L53 — `= Box<dyn Fn() -> Arc<dyn Task> + Send + Sync>` — Type alias for task constructor functions.
- pub `WorkflowConstructorFn` type L56 — `= Box<dyn Fn() -> Workflow + Send + Sync>` — Type alias for workflow constructor functions.
- pub `TriggerConstructorFn` type L59 — `= Box<dyn Fn() -> Arc<dyn Trigger> + Send + Sync>` — Type alias for trigger constructor functions.
- pub `Runtime` struct L67-69 — `{ inner: Arc<RuntimeInner> }` — A scoped runtime holding the registries for every cloacina extension point.
- pub `new` function L89-98 — `() -> Self` — Create a runtime seeded with every macro-registered entry from the
- pub `empty` function L105-115 — `() -> Self` — Create an empty runtime with no registered entries in any namespace.
- pub `seed_from_globals` function L164-309 — `(&self)` — Copy every entry from the process-global registries into this runtime.
- pub `register_task` function L316-324 — `(&self, namespace: TaskNamespace, constructor: F)` — Register a task constructor for the given namespace.
- pub `unregister_task` function L327-329 — `(&self, namespace: &TaskNamespace) -> bool` — Remove a task constructor.
- pub `get_task` function L332-334 — `(&self, namespace: &TaskNamespace) -> Option<Arc<dyn Task>>` — Look up and instantiate a task by namespace.
- pub `has_task` function L337-339 — `(&self, namespace: &TaskNamespace) -> bool` — Check if a task is registered for the given namespace.
- pub `register_workflow` function L346-354 — `(&self, name: String, constructor: F)` — Register a workflow constructor by name.
- pub `unregister_workflow` function L357-359 — `(&self, name: &str) -> bool` — Remove a workflow constructor.
- pub `get_workflow` function L362-364 — `(&self, name: &str) -> Option<Workflow>` — Look up and instantiate a workflow by name.
- pub `workflow_names` function L367-369 — `(&self) -> Vec<String>` — Get all registered workflow names.
- pub `all_workflows` function L372-379 — `(&self) -> Vec<Workflow>` — Get all registered workflows (instantiated).
- pub `register_trigger` function L386-394 — `(&self, name: String, constructor: F)` — Register a trigger constructor by name.
- pub `unregister_trigger` function L397-399 — `(&self, name: &str) -> bool` — Remove a trigger constructor.
- pub `get_trigger` function L402-404 — `(&self, name: &str) -> Option<Arc<dyn Trigger>>` — Look up and instantiate a trigger by name.
- pub `trigger_names` function L407-409 — `(&self) -> Vec<String>` — Get all registered trigger names.
- pub `all_triggers` function L412-419 — `(&self) -> HashMap<String, Arc<dyn Trigger>>` — Get all registered triggers (instantiated).
- pub `register_computation_graph` function L426-434 — `(&self, name: String, constructor: F)` — Register a computation graph constructor by graph name.
- pub `unregister_computation_graph` function L437-439 — `(&self, name: &str) -> bool` — Remove a computation graph constructor.
- pub `get_computation_graph` function L442-448 — `(&self, name: &str) -> Option<ComputationGraphRegistration>` — Look up and instantiate a computation graph registration by name.
- pub `computation_graph_names` function L451-458 — `(&self) -> Vec<String>` — Get all registered computation graph names.
- pub `register_stream_backend` function L465-470 — `(&self, type_name: String, factory: StreamBackendFactory)` — Register a stream backend factory by type name (e.g.
- pub `unregister_stream_backend` function L473-479 — `(&self, type_name: &str) -> bool` — Remove a stream backend factory.
- pub `has_stream_backend` function L482-484 — `(&self, type_name: &str) -> bool` — Check if a stream backend is registered for the given type name.
- pub `create_stream_backend` function L488-497 — `( &self, type_name: &str, config: StreamConfig, ) -> Option<Pin<Box<dyn Future<O...` — Get the creation future for a stream backend without holding the lock
- pub `stream_backend_names` function L500-502 — `(&self) -> Vec<String>` — Get all registered stream backend type names.
-  `RuntimeInner` struct L71-77 — `{ tasks: RwLock<HashMap<TaskNamespace, TaskConstructorFn>>, workflows: RwLock<Ha...` — ```
-  `Runtime` type L79-503 — `= Runtime` — ```
-  `seed_from_inventory` function L119-152 — `(&self)` — Populate the runtime from the `inventory` entries emitted by the
-  `Runtime` type L505-509 — `impl Default for Runtime` — ```
-  `default` function L506-508 — `() -> Self` — ```
-  `Runtime` type L511-526 — `= Runtime` — ```
-  `fmt` function L512-525 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — ```
-  `tests` module L529-593 — `-` — ```
-  `register_and_unregister_workflow` function L534-546 — `()` — ```
-  `register_and_unregister_trigger_by_name` function L549-557 — `()` — ```
-  `register_and_unregister_task` function L560-565 — `()` — ```
-  `stream_backend_roundtrip_names_only` function L568-573 — `()` — ```
-  `runtimes_are_independent` function L576-584 — `()` — ```
-  `debug_format_reports_sizes` function L587-592 — `()` — ```

#### crates/cloacina/src/task.rs

- pub `namespace` module L336 — `-` — # Task Management
- pub `TaskRegistry` struct L392-394 — `{ tasks: HashMap<TaskNamespace, Arc<dyn Task>> }` — Registry for managing collections of tasks and validating their dependencies.
- pub `new` function L398-402 — `() -> Self` — Create a new empty task registry
- pub `register` function L415-436 — `( &mut self, namespace: TaskNamespace, task: T, ) -> Result<(), RegistrationErro...` — Register a task in the registry
- pub `register_arc` function L439-460 — `( &mut self, namespace: TaskNamespace, task: Arc<dyn Task>, ) -> Result<(), Regi...` — Register a boxed task in the registry (used internally)
- pub `get_task` function L472-474 — `(&self, namespace: &TaskNamespace) -> Option<Arc<dyn Task>>` — Get a task by namespace
- pub `task_ids` function L481-483 — `(&self) -> Vec<TaskNamespace>` — Get all registered task namespaces
- pub `task_count` function L486-488 — `(&self) -> usize` — Get the number of registered tasks (O(1))
- pub `validate_dependencies` function L500-526 — `(&self) -> Result<(), ValidationError>` — Validate all task dependencies
- pub `topological_sort` function L567-621 — `(&self) -> Result<Vec<TaskNamespace>, ValidationError>` — Get tasks in topological order (dependencies first)
- pub `register_task_constructor` function L644-654 — `(namespace: TaskNamespace, constructor: F)` — Register a task constructor function globally with namespace
- pub `global_task_registry` function L660-662 — `() -> GlobalTaskRegistry` — Get the global task registry
- pub `get_task` function L668-671 — `(namespace: &TaskNamespace) -> Option<Arc<dyn Task>>` — Get a task instance from the global registry by namespace
-  `TaskRegistry` type L396-622 — `= TaskRegistry` — Tasks track their execution state for monitoring and recovery:
-  `check_cycles` function L529-556 — `( &self, namespace: &TaskNamespace, visited: &mut HashMap<TaskNamespace, bool>, ...` — Helper method to detect circular dependencies using DFS
-  `TaskRegistry` type L624-628 — `impl Default for TaskRegistry` — Tasks track their execution state for monitoring and recovery:
-  `default` function L625-627 — `() -> Self` — Tasks track their execution state for monitoring and recovery:
-  `TaskConstructor` type L631 — `= Box<dyn Fn() -> Arc<dyn Task> + Send + Sync>` — Type alias for the task constructor function stored in the global registry
-  `GlobalTaskRegistry` type L634 — `= Arc<RwLock<HashMap<TaskNamespace, TaskConstructor>>>` — Type alias for the global task registry containing task constructors
-  `GLOBAL_TASK_REGISTRY` variable L637-638 — `: Lazy<GlobalTaskRegistry>` — Global registry for automatically registering tasks created with the `#[task]` macro
-  `tests` module L674-888 — `-` — Tasks track their execution state for monitoring and recovery:
-  `TestTask` struct L683-687 — `{ id: String, dependencies: Vec<TaskNamespace>, fingerprint: Option<String> }` — Tasks track their execution state for monitoring and recovery:
-  `TestTask` type L689-702 — `= TestTask` — Tasks track their execution state for monitoring and recovery:
-  `new` function L690-696 — `(id: &str, dependencies: Vec<TaskNamespace>) -> Self` — Tasks track their execution state for monitoring and recovery:
-  `with_fingerprint` function L698-701 — `(mut self, fingerprint: &str) -> Self` — Tasks track their execution state for monitoring and recovery:
-  `TestTask` type L705-725 — `impl Task for TestTask` — Tasks track their execution state for monitoring and recovery:
-  `execute` function L706-712 — `( &self, context: Context<serde_json::Value>, ) -> Result<Context<serde_json::Va...` — Tasks track their execution state for monitoring and recovery:
-  `id` function L714-716 — `(&self) -> &str` — Tasks track their execution state for monitoring and recovery:
-  `dependencies` function L718-720 — `(&self) -> &[TaskNamespace]` — Tasks track their execution state for monitoring and recovery:
-  `code_fingerprint` function L722-724 — `(&self) -> Option<String>` — Tasks track their execution state for monitoring and recovery:
-  `test_task_state` function L728-755 — `()` — Tasks track their execution state for monitoring and recovery:
-  `test_task_registry_basic` function L758-774 — `()` — Tasks track their execution state for monitoring and recovery:
-  `test_task_registry_duplicate_id` function L777-792 — `()` — Tasks track their execution state for monitoring and recovery:
-  `test_dependency_validation` function L795-819 — `()` — Tasks track their execution state for monitoring and recovery:
-  `test_circular_dependency_detection` function L822-840 — `()` — Tasks track their execution state for monitoring and recovery:
-  `test_topological_sort` function L843-871 — `()` — Tasks track their execution state for monitoring and recovery:
-  `test_code_fingerprint_none_by_default` function L874-879 — `()` — Tasks track their execution state for monitoring and recovery:
-  `test_code_fingerprint_when_provided` function L882-887 — `()` — Tasks track their execution state for monitoring and recovery:

#### crates/cloacina/src/var.rs

- pub `VarNotFound` struct L59-62 — `{ name: String }` — Error returned when a required variable is not found.
- pub `var` function L90-95 — `(name: &str) -> Result<String, VarNotFound>` — Resolve a variable by name from `CLOACINA_VAR_{NAME}`.
- pub `var_or` function L107-109 — `(name: &str, default: &str) -> String` — Resolve a variable by name, returning a default if not set.
- pub `resolve_template` function L123-156 — `(input: &str) -> Result<String, Vec<VarNotFound>>` — Resolve template references in a string, replacing `{{ VAR_NAME }}`
-  `PREFIX` variable L55 — `: &str` — Use [`resolve_template`] to expand these references.
-  `VarNotFound` type L64-72 — `= VarNotFound` — Use [`resolve_template`] to expand these references.
-  `fmt` function L65-71 — `(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result` — Use [`resolve_template`] to expand these references.
-  `VarNotFound` type L74 — `= VarNotFound` — Use [`resolve_template`] to expand these references.
-  `tests` module L159-232 — `-` — Use [`resolve_template`] to expand these references.
-  `test_var_found` function L163-167 — `()` — Use [`resolve_template`] to expand these references.
-  `test_var_not_found` function L170-176 — `()` — Use [`resolve_template`] to expand these references.
-  `test_var_or_found` function L179-183 — `()` — Use [`resolve_template`] to expand these references.
-  `test_var_or_default` function L186-188 — `()` — Use [`resolve_template`] to expand these references.
-  `test_resolve_template_simple` function L191-200 — `()` — Use [`resolve_template`] to expand these references.
-  `test_resolve_template_no_placeholders` function L203-205 — `()` — Use [`resolve_template`] to expand these references.
-  `test_resolve_template_missing_var` function L208-212 — `()` — Use [`resolve_template`] to expand these references.
-  `test_resolve_template_mixed` function L215-223 — `()` — Use [`resolve_template`] to expand these references.
-  `test_resolve_template_whitespace_trimmed` function L226-231 — `()` — Use [`resolve_template`] to expand these references.

### crates/cloacina/src/crypto

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/crypto/key_encryption.rs

- pub `KeyEncryptionError` enum L36-48 — `EncryptionFailed | DecryptionFailed | InvalidKeyLength | InvalidEncryptedData` — Errors that can occur during key encryption/decryption.
- pub `encrypt_private_key` function L68-95 — `( private_key: &[u8], encryption_key: &[u8], ) -> Result<Vec<u8>, KeyEncryptionE...` — Encrypts an Ed25519 private key using AES-256-GCM.
- pub `decrypt_private_key` function L112-138 — `( encrypted_data: &[u8], encryption_key: &[u8], ) -> Result<Vec<u8>, KeyEncrypti...` — Decrypts an Ed25519 private key that was encrypted with AES-256-GCM.
-  `NONCE_SIZE` variable L51 — `: usize` — Size of the AES-256-GCM nonce in bytes.
-  `tests` module L141-208 — `-` — - A key management service (KMS)
-  `test_encrypt_decrypt_roundtrip` function L145-157 — `()` — - A key management service (KMS)
-  `test_wrong_key_fails` function L160-169 — `()` — - A key management service (KMS)
-  `test_invalid_key_length` function L172-181 — `()` — - A key management service (KMS)
-  `test_invalid_encrypted_data` function L184-193 — `()` — - A key management service (KMS)
-  `test_tampered_ciphertext_fails` function L196-207 — `()` — - A key management service (KMS)

#### crates/cloacina/src/crypto/mod.rs

-  `key_encryption` module L24 — `-` — Cryptographic utilities for package signing.
-  `signing` module L25 — `-` — - Key fingerprint computation

#### crates/cloacina/src/crypto/signing.rs

- pub `SigningError` enum L31-49 — `InvalidPrivateKeyLength | InvalidPublicKeyLength | InvalidSignatureLength | KeyC...` — Errors that can occur during signing operations.
- pub `GeneratedKeypair` struct L52-59 — `{ private_key: Vec<u8>, public_key: Vec<u8>, fingerprint: String }` — A generated Ed25519 keypair.
- pub `generate_signing_keypair` function L66-79 — `() -> GeneratedKeypair` — Generates a new Ed25519 signing keypair.
- pub `compute_key_fingerprint` function L90-95 — `(public_key: &[u8]) -> String` — Computes the SHA256 hex fingerprint of a public key.
- pub `sign_package` function L111-124 — `(package_hash: &[u8], private_key: &[u8]) -> Result<Vec<u8>, SigningError>` — Signs a package hash using an Ed25519 private key.
- pub `verify_signature` function L141-169 — `( package_hash: &[u8], signature: &[u8], public_key: &[u8], ) -> Result<(), Sign...` — Verifies a package signature using an Ed25519 public key.
- pub `compute_package_hash` function L181-186 — `(data: &[u8]) -> String` — Computes the SHA256 hash of package data.
-  `tests` module L189-286 — `-` — - Verifying signatures
-  `test_generate_keypair` function L193-199 — `()` — - Verifying signatures
-  `test_sign_and_verify` function L202-213 — `()` — - Verifying signatures
-  `test_verify_wrong_key_fails` function L216-226 — `()` — - Verifying signatures
-  `test_verify_tampered_data_fails` function L229-239 — `()` — - Verifying signatures
-  `test_fingerprint_is_deterministic` function L242-249 — `()` — - Verifying signatures
-  `test_invalid_key_lengths` function L252-272 — `()` — - Verifying signatures
-  `test_compute_package_hash` function L275-285 — `()` — - Verifying signatures

### crates/cloacina/src/dal/filesystem_dal

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/dal/filesystem_dal/mod.rs

- pub `workflow_registry_storage` module L23 — `-` — Filesystem Data Access Layer

#### crates/cloacina/src/dal/filesystem_dal/workflow_registry_storage.rs

- pub `FilesystemRegistryStorage` struct L68-70 — `{ storage_dir: PathBuf }` — Filesystem-based DAL for workflow registry storage operations.
- pub `new` function L94-106 — `(storage_dir: P) -> Result<Self, std::io::Error>` — Create a new filesystem workflow registry DAL.
- pub `storage_dir` function L109-111 — `(&self) -> &Path` — Get the storage directory path.
- pub `check_disk_space` function L119-133 — `(&self) -> Result<u64, StorageError>` — Check available disk space and validate against a threshold.
-  `FilesystemRegistryStorage` type L72-134 — `= FilesystemRegistryStorage` — non-database storage backends.
-  `file_path` function L114-116 — `(&self, id: &str) -> PathBuf` — Generate the file path for a given workflow ID.
-  `FilesystemRegistryStorage` type L137-241 — `impl RegistryStorage for FilesystemRegistryStorage` — non-database storage backends.
-  `store_binary` function L138-192 — `(&mut self, data: Vec<u8>) -> Result<String, StorageError>` — non-database storage backends.
-  `retrieve_binary` function L194-214 — `(&self, id: &str) -> Result<Option<Vec<u8>>, StorageError>` — non-database storage backends.
-  `delete_binary` function L216-236 — `(&mut self, id: &str) -> Result<(), StorageError>` — non-database storage backends.
-  `storage_type` function L238-240 — `(&self) -> StorageType` — non-database storage backends.
-  `tests` module L244-442 — `-` — non-database storage backends.
-  `create_test_storage` function L248-252 — `() -> (FilesystemRegistryStorage, TempDir)` — non-database storage backends.
-  `test_store_and_retrieve` function L255-263 — `()` — non-database storage backends.
-  `test_retrieve_nonexistent` function L266-272 — `()` — non-database storage backends.
-  `test_delete_binary` function L275-294 — `()` — non-database storage backends.
-  `test_invalid_uuid` function L297-306 — `()` — non-database storage backends.
-  `test_empty_file_handling` function L309-320 — `()` — non-database storage backends.
-  `test_atomic_write` function L323-341 — `()` — non-database storage backends.
-  `test_file_permissions` function L344-362 — `()` — non-database storage backends.
-  `test_directory_creation` function L365-382 — `()` — non-database storage backends.
-  `test_uuid_format` function L385-398 — `()` — non-database storage backends.
-  `test_binary_data_integrity` function L401-414 — `()` — non-database storage backends.
-  `test_very_large_file` function L417-426 — `()` — non-database storage backends.
-  `test_storage_dir_access` function L429-433 — `()` — non-database storage backends.
-  `test_check_disk_space` function L436-441 — `()` — non-database storage backends.

### crates/cloacina/src/dal

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/dal/mod.rs

- pub `unified` module L30 — `-` — selection happens at runtime based on the database connection URL.
-  `filesystem_dal` module L33 — `-` — selection happens at runtime based on the database connection URL.

### crates/cloacina/src/dal/unified/api_keys

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/dal/unified/api_keys/crud.rs

- pub `create_key` function L67-101 — `( dal: &DAL, key_hash: &str, name: &str, tenant_id: Option<&str>, is_admin: bool...` — Postgres CRUD operations for api_keys table.
- pub `validate_hash` function L103-126 — `( dal: &DAL, key_hash: &str, ) -> Result<Option<ApiKeyInfo>, ValidationError>` — Postgres CRUD operations for api_keys table.
- pub `has_any_keys` function L128-146 — `(dal: &DAL) -> Result<bool, ValidationError>` — Postgres CRUD operations for api_keys table.
- pub `list_keys` function L148-165 — `(dal: &DAL) -> Result<Vec<ApiKeyInfo>, ValidationError>` — Postgres CRUD operations for api_keys table.
- pub `revoke_key` function L167-189 — `(dal: &DAL, id: Uuid) -> Result<bool, ValidationError>` — Postgres CRUD operations for api_keys table.
-  `ApiKeyRow` struct L31-41 — `{ id: Uuid, key_hash: String, name: String, permissions: String, created_at: chr...` — Diesel model for reading api_keys rows.
-  `NewApiKey` struct L46-53 — `{ id: Uuid, key_hash: String, name: String, permissions: String, tenant_id: Opti...` — Diesel model for inserting api_keys rows.
-  `to_info` function L55-65 — `(row: ApiKeyRow) -> ApiKeyInfo` — Postgres CRUD operations for api_keys table.

#### crates/cloacina/src/dal/unified/api_keys/mod.rs

- pub `ApiKeyInfo` struct L31-39 — `{ id: uuid::Uuid, name: String, permissions: String, created_at: chrono::DateTim...` — Information about an API key (never includes the hash).
- pub `ApiKeyDAL` struct L43-45 — `{ dal: &'a DAL }` — DAL for API key operations.
- pub `new` function L48-50 — `(dal: &'a DAL) -> Self` — for the `api_keys` table.
- pub `create_key` function L54-63 — `( &self, key_hash: &str, name: &str, tenant_id: Option<&str>, is_admin: bool, ro...` — Create a new API key record.
- pub `validate_hash` function L67-72 — `( &self, key_hash: &str, ) -> Result<Option<ApiKeyInfo>, ValidationError>` — Validate a key hash — returns key info if found and not revoked.
- pub `has_any_keys` function L76-78 — `(&self) -> Result<bool, ValidationError>` — Check if any non-revoked API keys exist.
- pub `list_keys` function L82-84 — `(&self) -> Result<Vec<ApiKeyInfo>, ValidationError>` — List all API keys (no hashes).
- pub `revoke_key` function L88-90 — `(&self, id: uuid::Uuid) -> Result<bool, ValidationError>` — Soft-revoke a key.
-  `crud` module L24 — `-` — API key DAL — Postgres only.

### crates/cloacina/src/dal/unified

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/dal/unified/checkpoint.rs

- pub `CheckpointDAL` struct L38-40 — `{ dal: &'a DAL }` — Data access layer for computation graph checkpoint operations.
- pub `new` function L43-45 — `(dal: &'a DAL) -> Self` — semantics keyed by (graph_name, accumulator_name) or (graph_name).
- pub `save_checkpoint` function L52-65 — `( &self, graph_name: &str, accumulator_name: &str, data: Vec<u8>, ) -> Result<()...` — Save (upsert) an accumulator checkpoint.
- pub `load_checkpoint` function L162-174 — `( &self, graph_name: &str, accumulator_name: &str, ) -> Result<Option<Vec<u8>>, ...` — Load an accumulator checkpoint.
- pub `save_boundary` function L241-255 — `( &self, graph_name: &str, accumulator_name: &str, data: Vec<u8>, sequence_numbe...` — Save (upsert) a boundary with sequence number.
- pub `load_boundary` function L358-370 — `( &self, graph_name: &str, accumulator_name: &str, ) -> Result<Option<(Vec<u8>, ...` — Load a boundary and its sequence number.
- pub `save_reactor_state` function L437-451 — `( &self, graph_name: &str, cache_data: Vec<u8>, dirty_flags: Vec<u8>, sequential...` — Save (upsert) reactor state.
- pub `load_reactor_state` function L552-561 — `( &self, graph_name: &str, ) -> Result<Option<(Vec<u8>, Vec<u8>, Option<Vec<u8>>...` — Load reactor state.
- pub `save_state_buffer` function L634-648 — `( &self, graph_name: &str, accumulator_name: &str, data: Vec<u8>, capacity: i32,...` — Save (upsert) a state accumulator buffer.
- pub `load_state_buffer` function L751-763 — `( &self, graph_name: &str, accumulator_name: &str, ) -> Result<Option<(Vec<u8>, ...` — Load a state accumulator buffer.
- pub `delete_graph_state` function L830-836 — `(&self, graph_name: &str) -> Result<(), ValidationError>` — Delete all state for a graph (used on graph unload/removal).
-  `save_checkpoint_postgres` function L68-112 — `( &self, graph_name: &str, accumulator_name: &str, data: Vec<u8>, ) -> Result<()...` — semantics keyed by (graph_name, accumulator_name) or (graph_name).
-  `save_checkpoint_sqlite` function L115-159 — `( &self, graph_name: &str, accumulator_name: &str, data: Vec<u8>, ) -> Result<()...` — semantics keyed by (graph_name, accumulator_name) or (graph_name).
-  `load_checkpoint_postgres` function L177-204 — `( &self, graph_name: &str, accumulator_name: &str, ) -> Result<Option<Vec<u8>>, ...` — semantics keyed by (graph_name, accumulator_name) or (graph_name).
-  `load_checkpoint_sqlite` function L207-234 — `( &self, graph_name: &str, accumulator_name: &str, ) -> Result<Option<Vec<u8>>, ...` — semantics keyed by (graph_name, accumulator_name) or (graph_name).
-  `save_boundary_postgres` function L258-305 — `( &self, graph_name: &str, accumulator_name: &str, data: Vec<u8>, sequence_numbe...` — semantics keyed by (graph_name, accumulator_name) or (graph_name).
-  `save_boundary_sqlite` function L308-355 — `( &self, graph_name: &str, accumulator_name: &str, data: Vec<u8>, sequence_numbe...` — semantics keyed by (graph_name, accumulator_name) or (graph_name).
-  `load_boundary_postgres` function L373-400 — `( &self, graph_name: &str, accumulator_name: &str, ) -> Result<Option<(Vec<u8>, ...` — semantics keyed by (graph_name, accumulator_name) or (graph_name).
-  `load_boundary_sqlite` function L403-430 — `( &self, graph_name: &str, accumulator_name: &str, ) -> Result<Option<(Vec<u8>, ...` — semantics keyed by (graph_name, accumulator_name) or (graph_name).
-  `save_reactor_state_postgres` function L454-500 — `( &self, graph_name: &str, cache_data: Vec<u8>, dirty_flags: Vec<u8>, sequential...` — semantics keyed by (graph_name, accumulator_name) or (graph_name).
-  `save_reactor_state_sqlite` function L503-549 — `( &self, graph_name: &str, cache_data: Vec<u8>, dirty_flags: Vec<u8>, sequential...` — semantics keyed by (graph_name, accumulator_name) or (graph_name).
-  `load_reactor_state_postgres` function L564-594 — `( &self, graph_name: &str, ) -> Result<Option<(Vec<u8>, Vec<u8>, Option<Vec<u8>>...` — semantics keyed by (graph_name, accumulator_name) or (graph_name).
-  `load_reactor_state_sqlite` function L597-627 — `( &self, graph_name: &str, ) -> Result<Option<(Vec<u8>, Vec<u8>, Option<Vec<u8>>...` — semantics keyed by (graph_name, accumulator_name) or (graph_name).
-  `save_state_buffer_postgres` function L651-698 — `( &self, graph_name: &str, accumulator_name: &str, data: Vec<u8>, capacity: i32,...` — semantics keyed by (graph_name, accumulator_name) or (graph_name).
-  `save_state_buffer_sqlite` function L701-748 — `( &self, graph_name: &str, accumulator_name: &str, data: Vec<u8>, capacity: i32,...` — semantics keyed by (graph_name, accumulator_name) or (graph_name).
-  `load_state_buffer_postgres` function L766-793 — `( &self, graph_name: &str, accumulator_name: &str, ) -> Result<Option<(Vec<u8>, ...` — semantics keyed by (graph_name, accumulator_name) or (graph_name).
-  `load_state_buffer_sqlite` function L796-823 — `( &self, graph_name: &str, accumulator_name: &str, ) -> Result<Option<(Vec<u8>, ...` — semantics keyed by (graph_name, accumulator_name) or (graph_name).
-  `delete_graph_state_postgres` function L839-876 — `(&self, graph_name: &str) -> Result<(), ValidationError>` — semantics keyed by (graph_name, accumulator_name) or (graph_name).
-  `delete_graph_state_sqlite` function L879-916 — `(&self, graph_name: &str) -> Result<(), ValidationError>` — semantics keyed by (graph_name, accumulator_name) or (graph_name).

#### crates/cloacina/src/dal/unified/context.rs

- pub `ContextDAL` struct L32-34 — `{ dal: &'a DAL }` — Data access layer for context operations with runtime backend selection.
- pub `new` function L38-40 — `(dal: &'a DAL) -> Self` — Creates a new ContextDAL instance.
- pub `create` function L55-80 — `( &self, context: &Context<T>, ) -> Result<Option<UniversalUuid>, ContextError>` — Create a new context in the database.
- pub `read` function L155-164 — `(&self, id: UniversalUuid) -> Result<Context<T>, ContextError>` — Read a context from the database.
- pub `update` function L213-228 — `( &self, id: UniversalUuid, context: &Context<T>, ) -> Result<(), ContextError>` — Update an existing context in the database.
- pub `delete` function L279-285 — `(&self, id: UniversalUuid) -> Result<(), ContextError>` — Delete a context from the database.
- pub `list` function L327-336 — `(&self, limit: i64, offset: i64) -> Result<Vec<Context<T>>, ContextError>` — List contexts with pagination.
-  `create_postgres` function L83-115 — `(&self, value: String) -> Result<Option<UniversalUuid>, ContextError>` — at runtime based on the database connection type.
-  `create_sqlite` function L118-150 — `(&self, value: String) -> Result<Option<UniversalUuid>, ContextError>` — at runtime based on the database connection type.
-  `read_postgres` function L167-187 — `(&self, id: UniversalUuid) -> Result<Context<T>, ContextError>` — at runtime based on the database connection type.
-  `read_sqlite` function L190-210 — `(&self, id: UniversalUuid) -> Result<Context<T>, ContextError>` — at runtime based on the database connection type.
-  `update_postgres` function L231-252 — `(&self, id: UniversalUuid, value: String) -> Result<(), ContextError>` — at runtime based on the database connection type.
-  `update_sqlite` function L255-276 — `(&self, id: UniversalUuid, value: String) -> Result<(), ContextError>` — at runtime based on the database connection type.
-  `delete_postgres` function L288-303 — `(&self, id: UniversalUuid) -> Result<(), ContextError>` — at runtime based on the database connection type.
-  `delete_sqlite` function L306-321 — `(&self, id: UniversalUuid) -> Result<(), ContextError>` — at runtime based on the database connection type.
-  `list_postgres` function L339-375 — `( &self, limit: i64, offset: i64, ) -> Result<Vec<Context<T>>, ContextError>` — at runtime based on the database connection type.
-  `list_sqlite` function L378-410 — `(&self, limit: i64, offset: i64) -> Result<Vec<Context<T>>, ContextError>` — at runtime based on the database connection type.

#### crates/cloacina/src/dal/unified/execution_event.rs

- pub `ExecutionEventDAL` struct L39-41 — `{ dal: &'a DAL }` — Data access layer for execution event operations with runtime backend selection.
- pub `new` function L45-47 — `(dal: &'a DAL) -> Self` — Creates a new ExecutionEventDAL instance.
- pub `create` function L53-62 — `( &self, new_event: NewExecutionEvent, ) -> Result<ExecutionEvent, ValidationErr...` — Creates a new execution event record.
- pub `list_by_workflow` function L148-157 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<Vec<ExecutionEvent>, ...` — Gets all execution events for a specific workflow execution, ordered by sequence.
- pub `list_by_task` function L210-219 — `( &self, task_execution_id: UniversalUuid, ) -> Result<Vec<ExecutionEvent>, Vali...` — Gets all execution events for a specific task execution, ordered by sequence.
- pub `list_by_type` function L272-282 — `( &self, event_type: ExecutionEventType, limit: i64, ) -> Result<Vec<ExecutionEv...` — Gets execution events by type for monitoring and analysis.
- pub `get_recent` function L341-347 — `(&self, limit: i64) -> Result<Vec<ExecutionEvent>, ValidationError>` — Gets recent execution events for monitoring purposes.
- pub `delete_older_than` function L400-409 — `( &self, cutoff: UniversalTimestamp, ) -> Result<usize, ValidationError>` — Deletes execution events older than the specified timestamp.
- pub `count_by_workflow` function L462-471 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<i64, ValidationError>` — Counts total execution events for a workflow execution.
- pub `count_older_than` function L526-535 — `( &self, cutoff: UniversalTimestamp, ) -> Result<i64, ValidationError>` — Counts execution events older than the specified timestamp.
-  `create_postgres` function L65-99 — `( &self, new_event: NewExecutionEvent, ) -> Result<ExecutionEvent, ValidationErr...` — state transitions for debugging, compliance, and replay capability.
-  `create_sqlite` function L102-145 — `( &self, new_event: NewExecutionEvent, ) -> Result<ExecutionEvent, ValidationErr...` — state transitions for debugging, compliance, and replay capability.
-  `list_by_workflow_postgres` function L160-182 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<Vec<ExecutionEvent>, ...` — state transitions for debugging, compliance, and replay capability.
-  `list_by_workflow_sqlite` function L185-207 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<Vec<ExecutionEvent>, ...` — state transitions for debugging, compliance, and replay capability.
-  `list_by_task_postgres` function L222-244 — `( &self, task_execution_id: UniversalUuid, ) -> Result<Vec<ExecutionEvent>, Vali...` — state transitions for debugging, compliance, and replay capability.
-  `list_by_task_sqlite` function L247-269 — `( &self, task_execution_id: UniversalUuid, ) -> Result<Vec<ExecutionEvent>, Vali...` — state transitions for debugging, compliance, and replay capability.
-  `list_by_type_postgres` function L285-310 — `( &self, event_type: ExecutionEventType, limit: i64, ) -> Result<Vec<ExecutionEv...` — state transitions for debugging, compliance, and replay capability.
-  `list_by_type_sqlite` function L313-338 — `( &self, event_type: ExecutionEventType, limit: i64, ) -> Result<Vec<ExecutionEv...` — state transitions for debugging, compliance, and replay capability.
-  `get_recent_postgres` function L350-372 — `( &self, limit: i64, ) -> Result<Vec<ExecutionEvent>, ValidationError>` — state transitions for debugging, compliance, and replay capability.
-  `get_recent_sqlite` function L375-394 — `(&self, limit: i64) -> Result<Vec<ExecutionEvent>, ValidationError>` — state transitions for debugging, compliance, and replay capability.
-  `delete_older_than_postgres` function L412-434 — `( &self, cutoff: UniversalTimestamp, ) -> Result<usize, ValidationError>` — state transitions for debugging, compliance, and replay capability.
-  `delete_older_than_sqlite` function L437-459 — `( &self, cutoff: UniversalTimestamp, ) -> Result<usize, ValidationError>` — state transitions for debugging, compliance, and replay capability.
-  `count_by_workflow_postgres` function L474-496 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<i64, ValidationError>` — state transitions for debugging, compliance, and replay capability.
-  `count_by_workflow_sqlite` function L499-521 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<i64, ValidationError>` — state transitions for debugging, compliance, and replay capability.
-  `count_older_than_postgres` function L538-560 — `( &self, cutoff: UniversalTimestamp, ) -> Result<i64, ValidationError>` — state transitions for debugging, compliance, and replay capability.
-  `count_older_than_sqlite` function L563-585 — `( &self, cutoff: UniversalTimestamp, ) -> Result<i64, ValidationError>` — state transitions for debugging, compliance, and replay capability.

#### crates/cloacina/src/dal/unified/mod.rs

- pub `api_keys` module L47 — `-` — ```
- pub `checkpoint` module L48 — `-` — ```
- pub `context` module L49 — `-` — ```
- pub `execution_event` module L50 — `-` — ```
- pub `models` module L51 — `-` — ```
- pub `recovery_event` module L52 — `-` — ```
- pub `schedule` module L53 — `-` — ```
- pub `schedule_execution` module L54 — `-` — ```
- pub `task_execution` module L55 — `-` — ```
- pub `task_execution_metadata` module L56 — `-` — ```
- pub `task_outbox` module L57 — `-` — ```
- pub `workflow_execution` module L58 — `-` — ```
- pub `workflow_packages` module L59 — `-` — ```
- pub `workflow_registry` module L60 — `-` — ```
- pub `workflow_registry_storage` module L61 — `-` — ```
- pub `DAL` struct L95-98 — `{ database: Database }` — Helper macro for dispatching operations based on backend type.
- pub `new` function L110-112 — `(database: Database) -> Self` — Creates a new unified DAL instance.
- pub `backend` function L115-117 — `(&self) -> BackendType` — Returns the backend type for this DAL instance.
- pub `database` function L120-122 — `(&self) -> &Database` — Returns a reference to the underlying database.
- pub `pool` function L125-127 — `(&self) -> AnyPool` — Returns the connection pool.
- pub `api_keys` function L131-133 — `(&self) -> ApiKeyDAL<'_>` — Returns an API key DAL (Postgres only).
- pub `checkpoint` function L136-138 — `(&self) -> CheckpointDAL<'_>` — Returns a checkpoint DAL for computation graph state persistence.
- pub `context` function L141-143 — `(&self) -> ContextDAL<'_>` — Returns a context DAL for context operations.
- pub `workflow_execution` function L146-148 — `(&self) -> WorkflowExecutionDAL<'_>` — Returns a workflow execution DAL for workflow execution operations.
- pub `task_execution` function L151-153 — `(&self) -> TaskExecutionDAL<'_>` — Returns a task execution DAL for task operations.
- pub `task_execution_metadata` function L156-158 — `(&self) -> TaskExecutionMetadataDAL<'_>` — Returns a task execution metadata DAL for metadata operations.
- pub `task_outbox` function L161-163 — `(&self) -> TaskOutboxDAL<'_>` — Returns a task outbox DAL for work distribution operations.
- pub `recovery_event` function L166-168 — `(&self) -> RecoveryEventDAL<'_>` — Returns a recovery event DAL for recovery operations.
- pub `execution_event` function L171-173 — `(&self) -> ExecutionEventDAL<'_>` — Returns an execution event DAL for execution event operations.
- pub `schedule` function L176-178 — `(&self) -> ScheduleDAL<'_>` — Returns a unified schedule DAL for schedule operations.
- pub `schedule_execution` function L181-183 — `(&self) -> ScheduleExecutionDAL<'_>` — Returns a unified schedule execution DAL for schedule execution operations.
- pub `workflow_packages` function L186-188 — `(&self) -> WorkflowPackagesDAL<'_>` — Returns a workflow packages DAL for package operations.
- pub `workflow_registry` function L200-206 — `( &self, storage: S, ) -> crate::registry::workflow_registry::WorkflowRegistryIm...` — Creates a workflow registry implementation with the given storage backend.
- pub `try_workflow_registry` function L219-230 — `( &self, storage: S, ) -> Result< crate::registry::workflow_registry::WorkflowRe...` — Creates a workflow registry implementation with the given storage backend.
-  `DAL` type L100-231 — `= DAL` — ```

#### crates/cloacina/src/dal/unified/models.rs

- pub `UnifiedDbContext` struct L40-45 — `{ id: UniversalUuid, value: String, created_at: UniversalTimestamp, updated_at: ...` — Unified context model that works with both PostgreSQL and SQLite.
- pub `NewUnifiedDbContext` struct L50-55 — `{ id: UniversalUuid, value: String, created_at: UniversalTimestamp, updated_at: ...` — Insertable context with explicit ID and timestamps (for SQLite compatibility).
- pub `UnifiedWorkflowExecution` struct L63-78 — `{ id: UniversalUuid, workflow_name: String, workflow_version: String, status: St...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedWorkflowExecution` struct L82-91 — `{ id: UniversalUuid, workflow_name: String, workflow_version: String, status: St...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedTaskExecution` struct L99-120 — `{ id: UniversalUuid, workflow_execution_id: UniversalUuid, task_name: String, st...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedTaskExecution` struct L124-135 — `{ id: UniversalUuid, workflow_execution_id: UniversalUuid, task_name: String, st...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedTaskExecutionMetadata` struct L143-151 — `{ id: UniversalUuid, task_execution_id: UniversalUuid, workflow_execution_id: Un...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedTaskExecutionMetadata` struct L155-163 — `{ id: UniversalUuid, task_execution_id: UniversalUuid, workflow_execution_id: Un...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedRecoveryEvent` struct L171-180 — `{ id: UniversalUuid, workflow_execution_id: UniversalUuid, task_execution_id: Op...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedRecoveryEvent` struct L184-193 — `{ id: UniversalUuid, workflow_execution_id: UniversalUuid, task_execution_id: Op...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedExecutionEvent` struct L203-212 — `{ id: UniversalUuid, workflow_execution_id: UniversalUuid, task_execution_id: Op...` — Unified execution event model for audit trail of state transitions.
- pub `NewUnifiedExecutionEvent` struct L216-224 — `{ id: UniversalUuid, workflow_execution_id: UniversalUuid, task_execution_id: Op...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedTaskOutbox` struct L234-238 — `{ id: i64, task_execution_id: UniversalUuid, created_at: UniversalTimestamp }` — Unified task outbox model for work distribution.
- pub `NewUnifiedTaskOutbox` struct L242-245 — `{ task_execution_id: UniversalUuid, created_at: UniversalTimestamp }` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedSchedule` struct L253-271 — `{ id: UniversalUuid, schedule_type: String, workflow_name: String, enabled: Univ...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedSchedule` struct L275-291 — `{ id: UniversalUuid, schedule_type: String, workflow_name: String, enabled: Univ...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedScheduleExecution` struct L299-310 — `{ id: UniversalUuid, schedule_id: UniversalUuid, workflow_execution_id: Option<U...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedScheduleExecution` struct L314-324 — `{ id: UniversalUuid, schedule_id: UniversalUuid, workflow_execution_id: Option<U...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedWorkflowRegistryEntry` struct L332-336 — `{ id: UniversalUuid, created_at: UniversalTimestamp, data: UniversalBinary }` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedWorkflowRegistryEntry` struct L340-344 — `{ id: UniversalUuid, created_at: UniversalTimestamp, data: UniversalBinary }` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedWorkflowPackage` struct L352-371 — `{ id: UniversalUuid, registry_id: UniversalUuid, package_name: String, version: ...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedWorkflowPackage` struct L375-394 — `{ id: UniversalUuid, registry_id: UniversalUuid, package_name: String, version: ...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedSigningKey` struct L402-411 — `{ id: UniversalUuid, org_id: UniversalUuid, key_name: String, encrypted_private_...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedSigningKey` struct L415-423 — `{ id: UniversalUuid, org_id: UniversalUuid, key_name: String, encrypted_private_...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedTrustedKey` struct L431-439 — `{ id: UniversalUuid, org_id: UniversalUuid, key_fingerprint: String, public_key:...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedTrustedKey` struct L443-450 — `{ id: UniversalUuid, org_id: UniversalUuid, key_fingerprint: String, public_key:...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedKeyTrustAcl` struct L458-464 — `{ id: UniversalUuid, parent_org_id: UniversalUuid, child_org_id: UniversalUuid, ...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedKeyTrustAcl` struct L468-473 — `{ id: UniversalUuid, parent_org_id: UniversalUuid, child_org_id: UniversalUuid, ...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedPackageSignature` struct L481-487 — `{ id: UniversalUuid, package_hash: String, key_fingerprint: String, signature: U...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedPackageSignature` struct L491-497 — `{ id: UniversalUuid, package_hash: String, key_fingerprint: String, signature: U...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedAccumulatorCheckpoint` struct L757-764 — `{ id: UniversalUuid, graph_name: String, accumulator_name: String, checkpoint_da...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedAccumulatorCheckpoint` struct L768-775 — `{ id: UniversalUuid, graph_name: String, accumulator_name: String, checkpoint_da...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedAccumulatorBoundary` struct L779-787 — `{ id: UniversalUuid, graph_name: String, accumulator_name: String, boundary_data...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedAccumulatorBoundary` struct L791-799 — `{ id: UniversalUuid, graph_name: String, accumulator_name: String, boundary_data...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedReactorState` struct L803-811 — `{ id: UniversalUuid, graph_name: String, cache_data: UniversalBinary, dirty_flag...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedReactorState` struct L815-823 — `{ id: UniversalUuid, graph_name: String, cache_data: UniversalBinary, dirty_flag...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedStateAccumulatorBuffer` struct L827-835 — `{ id: UniversalUuid, graph_name: String, accumulator_name: String, buffer_data: ...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedStateAccumulatorBuffer` struct L839-847 — `{ id: UniversalUuid, graph_name: String, accumulator_name: String, buffer_data: ...` — SQL types that work with both PostgreSQL and SQLite backends.
-  `DbContext` type L519-528 — `= DbContext` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L520-527 — `(u: UnifiedDbContext) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `WorkflowExecutionRecord` type L530-549 — `= WorkflowExecutionRecord` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L531-548 — `(u: UnifiedWorkflowExecution) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `TaskExecution` type L551-576 — `= TaskExecution` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L552-575 — `(u: UnifiedTaskExecution) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `TaskExecutionMetadata` type L578-590 — `= TaskExecutionMetadata` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L579-589 — `(u: UnifiedTaskExecutionMetadata) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `RecoveryEvent` type L592-605 — `= RecoveryEvent` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L593-604 — `(u: UnifiedRecoveryEvent) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `ExecutionEvent` type L607-620 — `= ExecutionEvent` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L608-619 — `(u: UnifiedExecutionEvent) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `WorkflowRegistryEntry` type L622-630 — `= WorkflowRegistryEntry` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L623-629 — `(u: UnifiedWorkflowRegistryEntry) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `WorkflowPackage` type L632-655 — `= WorkflowPackage` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L633-654 — `(u: UnifiedWorkflowPackage) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `SigningKey` type L657-670 — `= SigningKey` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L658-669 — `(u: UnifiedSigningKey) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `TrustedKey` type L672-684 — `= TrustedKey` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L673-683 — `(u: UnifiedTrustedKey) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `KeyTrustAcl` type L686-696 — `= KeyTrustAcl` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L687-695 — `(u: UnifiedKeyTrustAcl) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `PackageSignature` type L698-708 — `= PackageSignature` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L699-707 — `(u: UnifiedPackageSignature) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `Schedule` type L710-732 — `= Schedule` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L711-731 — `(u: UnifiedSchedule) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `ScheduleExecution` type L734-749 — `= ScheduleExecution` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L735-748 — `(u: UnifiedScheduleExecution) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.

#### crates/cloacina/src/dal/unified/recovery_event.rs

- pub `RecoveryEventDAL` struct L36-38 — `{ dal: &'a DAL }` — Data access layer for recovery event operations with runtime backend selection.
- pub `new` function L42-44 — `(dal: &'a DAL) -> Self` — Creates a new RecoveryEventDAL instance.
- pub `create` function L47-56 — `( &self, new_event: NewRecoveryEvent, ) -> Result<RecoveryEvent, ValidationError...` — Creates a new recovery event record.
- pub `get_by_workflow` function L143-152 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<Vec<RecoveryEvent>, V...` — Gets all recovery events for a specific workflow execution.
- pub `get_by_task` function L205-214 — `( &self, task_execution_id: UniversalUuid, ) -> Result<Vec<RecoveryEvent>, Valid...` — Gets all recovery events for a specific task execution.
- pub `get_by_type` function L267-276 — `( &self, recovery_type: &str, ) -> Result<Vec<RecoveryEvent>, ValidationError>` — Gets recovery events by type for monitoring and analysis.
- pub `get_workflow_unavailable_events` function L331-336 — `( &self, ) -> Result<Vec<RecoveryEvent>, ValidationError>` — Gets all workflow unavailability events for monitoring unknown workflow cleanup.
- pub `get_recent` function L339-345 — `(&self, limit: i64) -> Result<Vec<RecoveryEvent>, ValidationError>` — Gets recent recovery events for monitoring purposes.
-  `create_postgres` function L59-98 — `( &self, new_event: NewRecoveryEvent, ) -> Result<RecoveryEvent, ValidationError...` — at runtime based on the database connection type.
-  `create_sqlite` function L101-140 — `( &self, new_event: NewRecoveryEvent, ) -> Result<RecoveryEvent, ValidationError...` — at runtime based on the database connection type.
-  `get_by_workflow_postgres` function L155-177 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<Vec<RecoveryEvent>, V...` — at runtime based on the database connection type.
-  `get_by_workflow_sqlite` function L180-202 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<Vec<RecoveryEvent>, V...` — at runtime based on the database connection type.
-  `get_by_task_postgres` function L217-239 — `( &self, task_execution_id: UniversalUuid, ) -> Result<Vec<RecoveryEvent>, Valid...` — at runtime based on the database connection type.
-  `get_by_task_sqlite` function L242-264 — `( &self, task_execution_id: UniversalUuid, ) -> Result<Vec<RecoveryEvent>, Valid...` — at runtime based on the database connection type.
-  `get_by_type_postgres` function L279-302 — `( &self, recovery_type: &str, ) -> Result<Vec<RecoveryEvent>, ValidationError>` — at runtime based on the database connection type.
-  `get_by_type_sqlite` function L305-328 — `( &self, recovery_type: &str, ) -> Result<Vec<RecoveryEvent>, ValidationError>` — at runtime based on the database connection type.
-  `get_recent_postgres` function L348-367 — `(&self, limit: i64) -> Result<Vec<RecoveryEvent>, ValidationError>` — at runtime based on the database connection type.
-  `get_recent_sqlite` function L370-389 — `(&self, limit: i64) -> Result<Vec<RecoveryEvent>, ValidationError>` — at runtime based on the database connection type.

#### crates/cloacina/src/dal/unified/task_execution_metadata.rs

- pub `TaskExecutionMetadataDAL` struct L34-36 — `{ dal: &'a DAL }` — Data access layer for task execution metadata operations with runtime backend selection.
- pub `new` function L40-42 — `(dal: &'a DAL) -> Self` — Creates a new TaskExecutionMetadataDAL instance.
- pub `create` function L45-54 — `( &self, new_metadata: NewTaskExecutionMetadata, ) -> Result<TaskExecutionMetada...` — Creates a new task execution metadata record.
- pub `get_by_workflow_and_task` function L139-151 — `( &self, workflow_id: UniversalUuid, task_namespace: &TaskNamespace, ) -> Result...` — Retrieves task execution metadata for a specific workflow and task.
- pub `get_by_task_execution` function L208-217 — `( &self, task_execution_id: UniversalUuid, ) -> Result<TaskExecutionMetadata, Va...` — Retrieves task execution metadata by task execution ID.
- pub `update_context_id` function L268-280 — `( &self, task_execution_id: UniversalUuid, context_id: Option<UniversalUuid>, ) ...` — Updates the context ID for a specific task execution.
- pub `upsert_task_execution_metadata` function L341-352 — `( &self, new_metadata: NewTaskExecutionMetadata, ) -> Result<TaskExecutionMetada...` — Creates or updates task execution metadata.
- pub `get_dependency_metadata` function L496-508 — `( &self, workflow_id: UniversalUuid, dependency_task_names: &[String], ) -> Resu...` — Retrieves metadata for multiple dependency tasks within a workflow execution.
- pub `get_dependency_metadata_with_contexts` function L565-587 — `( &self, workflow_id: UniversalUuid, dependency_task_namespaces: &[TaskNamespace...` — Retrieves metadata and context data for multiple dependency tasks in a single query.
-  `create_postgres` function L57-95 — `( &self, new_metadata: NewTaskExecutionMetadata, ) -> Result<TaskExecutionMetada...` — at runtime based on the database connection type.
-  `create_sqlite` function L98-136 — `( &self, new_metadata: NewTaskExecutionMetadata, ) -> Result<TaskExecutionMetada...` — at runtime based on the database connection type.
-  `get_by_workflow_and_task_postgres` function L154-178 — `( &self, workflow_id: UniversalUuid, task_namespace: &TaskNamespace, ) -> Result...` — at runtime based on the database connection type.
-  `get_by_workflow_and_task_sqlite` function L181-205 — `( &self, workflow_id: UniversalUuid, task_namespace: &TaskNamespace, ) -> Result...` — at runtime based on the database connection type.
-  `get_by_task_execution_postgres` function L220-241 — `( &self, task_execution_id: UniversalUuid, ) -> Result<TaskExecutionMetadata, Va...` — at runtime based on the database connection type.
-  `get_by_task_execution_sqlite` function L244-265 — `( &self, task_execution_id: UniversalUuid, ) -> Result<TaskExecutionMetadata, Va...` — at runtime based on the database connection type.
-  `update_context_id_postgres` function L283-309 — `( &self, task_execution_id: UniversalUuid, context_id: Option<UniversalUuid>, ) ...` — at runtime based on the database connection type.
-  `update_context_id_sqlite` function L312-338 — `( &self, task_execution_id: UniversalUuid, context_id: Option<UniversalUuid>, ) ...` — at runtime based on the database connection type.
-  `upsert_task_execution_metadata_postgres` function L355-403 — `( &self, new_metadata: NewTaskExecutionMetadata, ) -> Result<TaskExecutionMetada...` — at runtime based on the database connection type.
-  `upsert_task_execution_metadata_sqlite` function L406-493 — `( &self, new_metadata: NewTaskExecutionMetadata, ) -> Result<TaskExecutionMetada...` — at runtime based on the database connection type.
-  `get_dependency_metadata_postgres` function L511-535 — `( &self, workflow_id: UniversalUuid, dependency_task_names: &[String], ) -> Resu...` — at runtime based on the database connection type.
-  `get_dependency_metadata_sqlite` function L538-562 — `( &self, workflow_id: UniversalUuid, dependency_task_names: &[String], ) -> Resu...` — at runtime based on the database connection type.
-  `get_dependency_metadata_with_contexts_postgres` function L590-626 — `( &self, workflow_id: UniversalUuid, dependency_task_namespaces: &[TaskNamespace...` — at runtime based on the database connection type.
-  `get_dependency_metadata_with_contexts_sqlite` function L629-665 — `( &self, workflow_id: UniversalUuid, dependency_task_namespaces: &[TaskNamespace...` — at runtime based on the database connection type.
-  `tests` module L669-1159 — `-` — at runtime based on the database connection type.
-  `unique_dal` function L678-688 — `() -> DAL` — at runtime based on the database connection type.
-  `create_workflow_and_task` function L692-722 — `( dal: &DAL, task_name: &str, ) -> (UniversalUuid, UniversalUuid)` — Helper: create a workflow execution and a task, returning (workflow_id, task_id).
-  `test_create_metadata` function L728-747 — `()` — at runtime based on the database connection type.
-  `test_create_metadata_with_context` function L751-773 — `()` — at runtime based on the database connection type.
-  `test_get_by_workflow_and_task` function L779-803 — `()` — at runtime based on the database connection type.
-  `test_get_by_workflow_and_task_not_found` function L807-815 — `()` — at runtime based on the database connection type.
-  `test_get_by_task_execution` function L821-843 — `()` — at runtime based on the database connection type.
-  `test_update_context_id` function L849-880 — `()` — at runtime based on the database connection type.
-  `test_update_context_id_to_none` function L884-915 — `()` — at runtime based on the database connection type.
-  `test_upsert_insert` function L921-938 — `()` — at runtime based on the database connection type.
-  `test_upsert_update` function L942-979 — `()` — at runtime based on the database connection type.
-  `test_get_dependency_metadata` function L985-1035 — `()` — at runtime based on the database connection type.
-  `test_get_dependency_metadata_empty` function L1039-1047 — `()` — at runtime based on the database connection type.
-  `test_get_dependency_metadata_with_contexts_empty_input` function L1053-1061 — `()` — at runtime based on the database connection type.
-  `test_get_dependency_metadata_with_contexts` function L1066-1158 — `()` — at runtime based on the database connection type.

#### crates/cloacina/src/dal/unified/task_outbox.rs

- pub `TaskOutboxDAL` struct L43-45 — `{ dal: &'a DAL }` — Data access layer for task outbox operations with runtime backend selection.
- pub `new` function L49-51 — `(dal: &'a DAL) -> Self` — Creates a new TaskOutboxDAL instance.
- pub `create` function L57-63 — `(&self, new_entry: NewTaskOutbox) -> Result<TaskOutbox, ValidationError>` — Creates a new outbox entry.
- pub `delete_by_task` function L133-142 — `( &self, task_execution_id: UniversalUuid, ) -> Result<(), ValidationError>` — Deletes an outbox entry by task execution ID.
- pub `list_pending` function L195-201 — `(&self, limit: i64) -> Result<Vec<TaskOutbox>, ValidationError>` — Lists all pending outbox entries (for polling-based claiming).
- pub `count_pending` function L262-268 — `(&self) -> Result<i64, ValidationError>` — Counts pending outbox entries (for monitoring).
- pub `delete_older_than` function L308-317 — `( &self, cutoff: UniversalTimestamp, ) -> Result<i64, ValidationError>` — Deletes stale outbox entries older than the specified timestamp.
-  `create_postgres` function L66-97 — `( &self, new_entry: NewTaskOutbox, ) -> Result<TaskOutbox, ValidationError>` — for claiming and cleanup.
-  `create_sqlite` function L100-128 — `(&self, new_entry: NewTaskOutbox) -> Result<TaskOutbox, ValidationError>` — for claiming and cleanup.
-  `delete_by_task_postgres` function L145-166 — `( &self, task_execution_id: UniversalUuid, ) -> Result<(), ValidationError>` — for claiming and cleanup.
-  `delete_by_task_sqlite` function L169-190 — `( &self, task_execution_id: UniversalUuid, ) -> Result<(), ValidationError>` — for claiming and cleanup.
-  `list_pending_postgres` function L204-230 — `(&self, limit: i64) -> Result<Vec<TaskOutbox>, ValidationError>` — for claiming and cleanup.
-  `list_pending_sqlite` function L233-259 — `(&self, limit: i64) -> Result<Vec<TaskOutbox>, ValidationError>` — for claiming and cleanup.
-  `count_pending_postgres` function L271-285 — `(&self) -> Result<i64, ValidationError>` — for claiming and cleanup.
-  `count_pending_sqlite` function L288-302 — `(&self) -> Result<i64, ValidationError>` — for claiming and cleanup.
-  `delete_older_than_postgres` function L320-340 — `( &self, cutoff: UniversalTimestamp, ) -> Result<i64, ValidationError>` — for claiming and cleanup.
-  `delete_older_than_sqlite` function L343-363 — `( &self, cutoff: UniversalTimestamp, ) -> Result<i64, ValidationError>` — for claiming and cleanup.
-  `tests` module L367-661 — `-` — for claiming and cleanup.
-  `unique_dal` function L375-385 — `() -> DAL` — for claiming and cleanup.
-  `create_ready_task` function L390-419 — `(dal: &DAL, task_name: &str) -> UniversalUuid` — Helper: create a workflow execution + task, mark it ready (which inserts into outbox),
-  `test_create_outbox_entry` function L425-432 — `()` — for claiming and cleanup.
-  `test_list_pending_empty` function L436-440 — `()` — for claiming and cleanup.
-  `test_list_pending_respects_limit` function L444-455 — `()` — for claiming and cleanup.
-  `test_list_pending_ordered_oldest_first` function L459-470 — `()` — for claiming and cleanup.
-  `test_count_pending_empty` function L476-480 — `()` — for claiming and cleanup.
-  `test_count_pending_after_inserts` function L484-491 — `()` — for claiming and cleanup.
-  `test_delete_by_task` function L497-510 — `()` — for claiming and cleanup.
-  `test_delete_by_task_nonexistent` function L514-519 — `()` — for claiming and cleanup.
-  `test_delete_by_task_only_removes_target` function L523-533 — `()` — for claiming and cleanup.
-  `test_delete_older_than` function L539-556 — `()` — for claiming and cleanup.
-  `test_delete_older_than_keeps_recent` function L560-576 — `()` — for claiming and cleanup.
-  `test_direct_create` function L582-619 — `()` — for claiming and cleanup.
-  `test_mark_ready_populates_outbox` function L625-660 — `()` — for claiming and cleanup.

#### crates/cloacina/src/dal/unified/workflow_execution.rs

- pub `WorkflowExecutionDAL` struct L35-37 — `{ dal: &'a DAL }` — Data access layer for workflow execution operations with compile-time backend selection.
- pub `new` function L40-42 — `(dal: &'a DAL) -> Self` — are written atomically.
- pub `create` function L48-57 — `( &self, new_execution: NewWorkflowExecution, ) -> Result<WorkflowExecutionRecor...` — Creates a new workflow execution record in the database.
- pub `get_by_id` function L185-194 — `( &self, id: UniversalUuid, ) -> Result<WorkflowExecutionRecord, ValidationError...` — are written atomically.
- pub `get_active_executions` function L236-244 — `( &self, ) -> Result<Vec<WorkflowExecutionRecord>, ValidationError>` — are written atomically.
- pub `update_status` function L292-302 — `( &self, id: UniversalUuid, status: &str, ) -> Result<(), ValidationError>` — are written atomically.
- pub `mark_completed` function L366-372 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — Marks a workflow execution as completed.
- pub `get_last_version` function L482-491 — `( &self, workflow_name: &str, ) -> Result<Option<String>, ValidationError>` — are written atomically.
- pub `mark_failed` function L553-563 — `( &self, id: UniversalUuid, reason: &str, ) -> Result<(), ValidationError>` — Marks a workflow execution as failed with an error reason.
- pub `increment_recovery_attempts` function L687-696 — `( &self, id: UniversalUuid, ) -> Result<(), ValidationError>` — are written atomically.
- pub `cancel` function L756-762 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — are written atomically.
- pub `pause` function L771-781 — `( &self, id: UniversalUuid, reason: Option<&str>, ) -> Result<(), ValidationErro...` — Pauses a running workflow execution.
- pub `resume` function L897-903 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — Resumes a paused workflow execution.
- pub `update_final_context` function L1051-1062 — `( &self, id: UniversalUuid, final_context_id: UniversalUuid, ) -> Result<(), Val...` — are written atomically.
- pub `list_recent` function L1120-1129 — `( &self, limit: i64, ) -> Result<Vec<WorkflowExecutionRecord>, ValidationError>` — are written atomically.
-  `create_postgres` function L60-120 — `( &self, new_execution: NewWorkflowExecution, ) -> Result<WorkflowExecutionRecor...` — are written atomically.
-  `create_sqlite` function L123-183 — `( &self, new_execution: NewWorkflowExecution, ) -> Result<WorkflowExecutionRecor...` — are written atomically.
-  `get_by_id_postgres` function L197-214 — `( &self, id: UniversalUuid, ) -> Result<WorkflowExecutionRecord, ValidationError...` — are written atomically.
-  `get_by_id_sqlite` function L217-234 — `( &self, id: UniversalUuid, ) -> Result<WorkflowExecutionRecord, ValidationError...` — are written atomically.
-  `get_active_executions_postgres` function L247-267 — `( &self, ) -> Result<Vec<WorkflowExecutionRecord>, ValidationError>` — are written atomically.
-  `get_active_executions_sqlite` function L270-290 — `( &self, ) -> Result<Vec<WorkflowExecutionRecord>, ValidationError>` — are written atomically.
-  `update_status_postgres` function L305-331 — `( &self, id: UniversalUuid, status: &str, ) -> Result<(), ValidationError>` — are written atomically.
-  `update_status_sqlite` function L334-360 — `( &self, id: UniversalUuid, status: &str, ) -> Result<(), ValidationError>` — are written atomically.
-  `mark_completed_postgres` function L375-426 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — are written atomically.
-  `mark_completed_sqlite` function L429-480 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — are written atomically.
-  `get_last_version_postgres` function L494-519 — `( &self, workflow_name: &str, ) -> Result<Option<String>, ValidationError>` — are written atomically.
-  `get_last_version_sqlite` function L522-547 — `( &self, workflow_name: &str, ) -> Result<Option<String>, ValidationError>` — are written atomically.
-  `mark_failed_postgres` function L566-624 — `( &self, id: UniversalUuid, reason: &str, ) -> Result<(), ValidationError>` — are written atomically.
-  `mark_failed_sqlite` function L627-685 — `( &self, id: UniversalUuid, reason: &str, ) -> Result<(), ValidationError>` — are written atomically.
-  `increment_recovery_attempts_postgres` function L699-725 — `( &self, id: UniversalUuid, ) -> Result<(), ValidationError>` — are written atomically.
-  `increment_recovery_attempts_sqlite` function L728-754 — `( &self, id: UniversalUuid, ) -> Result<(), ValidationError>` — are written atomically.
-  `pause_postgres` function L784-835 — `( &self, id: UniversalUuid, reason: Option<&str>, ) -> Result<(), ValidationErro...` — are written atomically.
-  `pause_sqlite` function L838-889 — `( &self, id: UniversalUuid, reason: Option<&str>, ) -> Result<(), ValidationErro...` — are written atomically.
-  `resume_postgres` function L906-951 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — are written atomically.
-  `resume_sqlite` function L954-999 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — are written atomically.
-  `cancel_postgres` function L1002-1024 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — are written atomically.
-  `cancel_sqlite` function L1027-1049 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — are written atomically.
-  `update_final_context_postgres` function L1065-1090 — `( &self, id: UniversalUuid, final_context_id: UniversalUuid, ) -> Result<(), Val...` — are written atomically.
-  `update_final_context_sqlite` function L1093-1118 — `( &self, id: UniversalUuid, final_context_id: UniversalUuid, ) -> Result<(), Val...` — are written atomically.
-  `list_recent_postgres` function L1132-1154 — `( &self, limit: i64, ) -> Result<Vec<WorkflowExecutionRecord>, ValidationError>` — are written atomically.
-  `list_recent_sqlite` function L1157-1179 — `( &self, limit: i64, ) -> Result<Vec<WorkflowExecutionRecord>, ValidationError>` — are written atomically.

#### crates/cloacina/src/dal/unified/workflow_packages.rs

- pub `WorkflowPackagesDAL` struct L35-37 — `{ dal: &'a DAL }` — Data access layer for workflow package operations with runtime backend selection.
- pub `new` function L41-43 — `(dal: &'a DAL) -> Self` — Creates a new WorkflowPackagesDAL instance.
- pub `store_package_metadata` function L46-70 — `( &self, registry_id: &str, package_metadata: &PackageMetadata, storage_type: cr...` — Store package metadata in the database.
- pub `get_package_metadata` function L209-221 — `( &self, package_name: &str, version: &str, ) -> Result<Option<(String, PackageM...` — Retrieve package metadata from the database.
- pub `get_package_metadata_by_id` function L298-307 — `( &self, package_id: Uuid, ) -> Result<Option<(String, PackageMetadata)>, Regist...` — Retrieve package metadata by UUID from the database.
- pub `list_all_packages` function L376-382 — `(&self) -> Result<Vec<WorkflowPackage>, RegistryError>` — List all packages in the registry.
- pub `delete_package_metadata` function L421-433 — `( &self, package_name: &str, version: &str, ) -> Result<(), RegistryError>` — Delete package metadata from the database.
- pub `delete_package_metadata_by_id` function L498-508 — `( &self, package_id: Uuid, ) -> Result<(), RegistryError>` — Delete package metadata by UUID from the database.
-  `store_package_metadata_postgres` function L73-138 — `( &self, registry_id: &str, package_metadata: &PackageMetadata, storage_type: cr...` — at runtime based on the database connection type.
-  `store_package_metadata_sqlite` function L141-206 — `( &self, registry_id: &str, package_metadata: &PackageMetadata, storage_type: cr...` — at runtime based on the database connection type.
-  `get_package_metadata_postgres` function L224-258 — `( &self, package_name: &str, version: &str, ) -> Result<Option<(String, PackageM...` — at runtime based on the database connection type.
-  `get_package_metadata_sqlite` function L261-295 — `( &self, package_name: &str, version: &str, ) -> Result<Option<(String, PackageM...` — at runtime based on the database connection type.
-  `get_package_metadata_by_id_postgres` function L310-340 — `( &self, package_id: Uuid, ) -> Result<Option<(String, PackageMetadata)>, Regist...` — at runtime based on the database connection type.
-  `get_package_metadata_by_id_sqlite` function L343-373 — `( &self, package_id: Uuid, ) -> Result<Option<(String, PackageMetadata)>, Regist...` — at runtime based on the database connection type.
-  `list_all_packages_postgres` function L385-400 — `(&self) -> Result<Vec<WorkflowPackage>, RegistryError>` — at runtime based on the database connection type.
-  `list_all_packages_sqlite` function L403-418 — `(&self) -> Result<Vec<WorkflowPackage>, RegistryError>` — at runtime based on the database connection type.
-  `delete_package_metadata_postgres` function L436-464 — `( &self, package_name: &str, version: &str, ) -> Result<(), RegistryError>` — at runtime based on the database connection type.
-  `delete_package_metadata_sqlite` function L467-495 — `( &self, package_name: &str, version: &str, ) -> Result<(), RegistryError>` — at runtime based on the database connection type.
-  `delete_package_metadata_by_id_postgres` function L511-533 — `( &self, package_id: Uuid, ) -> Result<(), RegistryError>` — at runtime based on the database connection type.
-  `delete_package_metadata_by_id_sqlite` function L536-558 — `( &self, package_id: Uuid, ) -> Result<(), RegistryError>` — at runtime based on the database connection type.
-  `tests` module L562-813 — `-` — at runtime based on the database connection type.
-  `unique_dal` function L568-578 — `() -> DAL` — at runtime based on the database connection type.
-  `sample_metadata` function L581-599 — `(name: &str, version: &str) -> PackageMetadata` — at runtime based on the database connection type.
-  `test_store_and_get_package_metadata` function L603-632 — `()` — at runtime based on the database connection type.
-  `test_get_package_metadata_not_found` function L636-645 — `()` — at runtime based on the database connection type.
-  `test_get_package_metadata_by_id` function L649-673 — `()` — at runtime based on the database connection type.
-  `test_get_package_metadata_by_id_not_found` function L677-686 — `()` — at runtime based on the database connection type.
-  `test_list_all_packages` function L690-722 — `()` — at runtime based on the database connection type.
-  `test_delete_package_metadata` function L726-762 — `()` — at runtime based on the database connection type.
-  `test_delete_package_metadata_by_id` function L766-795 — `()` — at runtime based on the database connection type.
-  `test_delete_nonexistent_does_not_error` function L799-812 — `()` — at runtime based on the database connection type.

#### crates/cloacina/src/dal/unified/workflow_registry.rs

- pub `WorkflowRegistryDAL` struct L23-25 — `{ _dal: &'a DAL }` — Data access layer for workflow registry operations.
- pub `new` function L29-31 — `(dal: &'a DAL) -> Self` — Creates a new WorkflowRegistryDAL instance.

#### crates/cloacina/src/dal/unified/workflow_registry_storage.rs

- pub `UnifiedRegistryStorage` struct L37-39 — `{ database: Database }` — Unified registry storage that works with both PostgreSQL and SQLite.
- pub `new` function L43-45 — `(database: Database) -> Self` — Creates a new UnifiedRegistryStorage instance.
- pub `database` function L48-50 — `(&self) -> &Database` — Returns a reference to the underlying database.
-  `UnifiedRegistryStorage` type L41-51 — `= UnifiedRegistryStorage` — at runtime based on the database connection type.
-  `UnifiedRegistryStorage` type L54-82 — `impl RegistryStorage for UnifiedRegistryStorage` — at runtime based on the database connection type.
-  `store_binary` function L55-61 — `(&mut self, data: Vec<u8>) -> Result<String, StorageError>` — at runtime based on the database connection type.
-  `retrieve_binary` function L63-69 — `(&self, id: &str) -> Result<Option<Vec<u8>>, StorageError>` — at runtime based on the database connection type.
-  `delete_binary` function L71-77 — `(&mut self, id: &str) -> Result<(), StorageError>` — at runtime based on the database connection type.
-  `storage_type` function L79-81 — `(&self) -> StorageType` — at runtime based on the database connection type.
-  `UnifiedRegistryStorage` type L84-238 — `= UnifiedRegistryStorage` — at runtime based on the database connection type.
-  `store_binary_postgres` function L86-110 — `(&self, data: Vec<u8>) -> Result<String, StorageError>` — at runtime based on the database connection type.
-  `store_binary_sqlite` function L113-139 — `(&self, data: Vec<u8>) -> Result<String, StorageError>` — at runtime based on the database connection type.
-  `retrieve_binary_postgres` function L142-163 — `(&self, id: &str) -> Result<Option<Vec<u8>>, StorageError>` — at runtime based on the database connection type.
-  `retrieve_binary_sqlite` function L166-192 — `(&self, id: &str) -> Result<Option<Vec<u8>>, StorageError>` — at runtime based on the database connection type.
-  `delete_binary_postgres` function L195-213 — `(&self, id: &str) -> Result<(), StorageError>` — at runtime based on the database connection type.
-  `delete_binary_sqlite` function L216-237 — `(&self, id: &str) -> Result<(), StorageError>` — at runtime based on the database connection type.

### crates/cloacina/src/dal/unified/schedule

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/dal/unified/schedule/crud.rs

-  `create_postgres` function L35-83 — `( &self, new_schedule: NewSchedule, ) -> Result<Schedule, ValidationError>` — CRUD operations for unified schedules.
-  `create_sqlite` function L86-134 — `( &self, new_schedule: NewSchedule, ) -> Result<Schedule, ValidationError>` — CRUD operations for unified schedules.
-  `get_by_id_postgres` function L137-154 — `( &self, id: UniversalUuid, ) -> Result<Schedule, ValidationError>` — CRUD operations for unified schedules.
-  `get_by_id_sqlite` function L157-174 — `( &self, id: UniversalUuid, ) -> Result<Schedule, ValidationError>` — CRUD operations for unified schedules.
-  `list_postgres` function L177-215 — `( &self, schedule_type: Option<String>, enabled_only: bool, limit: i64, offset: ...` — CRUD operations for unified schedules.
-  `list_sqlite` function L218-256 — `( &self, schedule_type: Option<String>, enabled_only: bool, limit: i64, offset: ...` — CRUD operations for unified schedules.
-  `enable_postgres` function L259-282 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — CRUD operations for unified schedules.
-  `enable_sqlite` function L285-308 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — CRUD operations for unified schedules.
-  `disable_postgres` function L311-334 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — CRUD operations for unified schedules.
-  `disable_sqlite` function L337-360 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — CRUD operations for unified schedules.
-  `delete_postgres` function L363-376 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — CRUD operations for unified schedules.
-  `delete_sqlite` function L379-392 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — CRUD operations for unified schedules.
-  `get_due_cron_schedules_postgres` function L395-422 — `( &self, now: DateTime<Utc>, ) -> Result<Vec<Schedule>, ValidationError>` — CRUD operations for unified schedules.
-  `get_due_cron_schedules_sqlite` function L425-452 — `( &self, now: DateTime<Utc>, ) -> Result<Vec<Schedule>, ValidationError>` — CRUD operations for unified schedules.
-  `claim_and_update_cron_postgres` function L455-496 — `( &self, id: UniversalUuid, current_time: DateTime<Utc>, last_run: DateTime<Utc>...` — CRUD operations for unified schedules.
-  `claim_and_update_cron_sqlite` function L499-536 — `( &self, id: UniversalUuid, current_time: DateTime<Utc>, last_run: DateTime<Utc>...` — CRUD operations for unified schedules.
-  `update_schedule_times_postgres` function L539-569 — `( &self, id: UniversalUuid, last_run: DateTime<Utc>, next_run: DateTime<Utc>, ) ...` — CRUD operations for unified schedules.
-  `update_schedule_times_sqlite` function L572-602 — `( &self, id: UniversalUuid, last_run: DateTime<Utc>, next_run: DateTime<Utc>, ) ...` — CRUD operations for unified schedules.
-  `get_enabled_triggers_postgres` function L605-629 — `( &self, ) -> Result<Vec<Schedule>, ValidationError>` — CRUD operations for unified schedules.
-  `get_enabled_triggers_sqlite` function L632-656 — `( &self, ) -> Result<Vec<Schedule>, ValidationError>` — CRUD operations for unified schedules.
-  `update_last_poll_postgres` function L659-686 — `( &self, id: UniversalUuid, last_poll_at: DateTime<Utc>, ) -> Result<(), Validat...` — CRUD operations for unified schedules.
-  `update_last_poll_sqlite` function L689-716 — `( &self, id: UniversalUuid, last_poll_at: DateTime<Utc>, ) -> Result<(), Validat...` — CRUD operations for unified schedules.
-  `upsert_trigger_postgres` function L719-823 — `( &self, new_schedule: NewSchedule, ) -> Result<Schedule, ValidationError>` — CRUD operations for unified schedules.
-  `upsert_trigger_sqlite` function L826-930 — `( &self, new_schedule: NewSchedule, ) -> Result<Schedule, ValidationError>` — CRUD operations for unified schedules.
-  `get_by_trigger_name_postgres` function L933-956 — `( &self, name: String, ) -> Result<Option<Schedule>, ValidationError>` — CRUD operations for unified schedules.
-  `get_by_trigger_name_sqlite` function L959-982 — `( &self, name: String, ) -> Result<Option<Schedule>, ValidationError>` — CRUD operations for unified schedules.
-  `find_by_workflow_postgres` function L985-1007 — `( &self, workflow_name: String, ) -> Result<Vec<Schedule>, ValidationError>` — CRUD operations for unified schedules.
-  `find_by_workflow_sqlite` function L1010-1032 — `( &self, workflow_name: String, ) -> Result<Vec<Schedule>, ValidationError>` — CRUD operations for unified schedules.
-  `update_cron_expression_and_timezone_postgres` function L1035-1066 — `( &self, id: UniversalUuid, cron_expression: Option<String>, timezone: Option<St...` — CRUD operations for unified schedules.
-  `update_cron_expression_and_timezone_sqlite` function L1069-1100 — `( &self, id: UniversalUuid, cron_expression: Option<String>, timezone: Option<St...` — CRUD operations for unified schedules.

#### crates/cloacina/src/dal/unified/schedule/mod.rs

- pub `ScheduleDAL` struct L34-36 — `{ dal: &'a DAL }` — Data access layer for unified schedule operations with runtime backend selection.
- pub `new` function L40-42 — `(dal: &'a DAL) -> Self` — Creates a new ScheduleDAL instance.
- pub `create` function L45-51 — `(&self, new_schedule: NewSchedule) -> Result<Schedule, ValidationError>` — Creates a new schedule record in the database.
- pub `get_by_id` function L54-60 — `(&self, id: UniversalUuid) -> Result<Schedule, ValidationError>` — Retrieves a schedule by its ID.
- pub `list` function L63-78 — `( &self, schedule_type: Option<&str>, enabled_only: bool, limit: i64, offset: i6...` — Lists schedules with optional filtering by type and enabled status.
- pub `enable` function L81-87 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — Enables a schedule.
- pub `disable` function L90-96 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — Disables a schedule.
- pub `delete` function L99-105 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — Deletes a schedule from the database.
- pub `get_due_cron_schedules` function L108-117 — `( &self, now: DateTime<Utc>, ) -> Result<Vec<Schedule>, ValidationError>` — Retrieves all enabled cron schedules that are due for execution.
- pub `claim_and_update_cron` function L120-134 — `( &self, id: UniversalUuid, current_time: DateTime<Utc>, last_run: DateTime<Utc>...` — Atomically claims and updates a cron schedule's timing.
- pub `update_schedule_times` function L137-150 — `( &self, id: UniversalUuid, last_run: DateTime<Utc>, next_run: DateTime<Utc>, ) ...` — Updates the last run and next run times for a schedule.
- pub `get_enabled_triggers` function L153-159 — `(&self) -> Result<Vec<Schedule>, ValidationError>` — Retrieves all enabled trigger schedules.
- pub `update_last_poll` function L162-172 — `( &self, id: UniversalUuid, last_poll_at: DateTime<Utc>, ) -> Result<(), Validat...` — Updates the last poll time for a trigger schedule.
- pub `upsert_trigger` function L175-184 — `( &self, new_schedule: NewSchedule, ) -> Result<Schedule, ValidationError>` — Upserts a trigger schedule by trigger_name.
- pub `get_by_trigger_name` function L187-197 — `( &self, name: &str, ) -> Result<Option<Schedule>, ValidationError>` — Retrieves a schedule by its trigger name.
- pub `find_by_workflow` function L200-210 — `( &self, workflow_name: &str, ) -> Result<Vec<Schedule>, ValidationError>` — Finds schedules by workflow name.
- pub `update_cron_expression_and_timezone` function L213-239 — `( &self, id: UniversalUuid, cron_expression: Option<&str>, timezone: Option<&str...` — Updates the cron expression and timezone for a cron schedule.
-  `crud` module L24 — `-` — Unified Schedule DAL with runtime backend selection
-  `tests` module L243-751 — `-` — implementation at runtime based on the database connection type.
-  `unique_dal` function L251-261 — `() -> DAL` — implementation at runtime based on the database connection type.
-  `test_create_cron_schedule` function L267-283 — `()` — implementation at runtime based on the database connection type.
-  `test_create_trigger_schedule` function L287-299 — `()` — implementation at runtime based on the database connection type.
-  `test_get_by_id` function L303-316 — `()` — implementation at runtime based on the database connection type.
-  `test_get_by_id_not_found` function L320-325 — `()` — implementation at runtime based on the database connection type.
-  `test_list_all` function L331-350 — `()` — implementation at runtime based on the database connection type.
-  `test_list_by_schedule_type` function L354-386 — `()` — implementation at runtime based on the database connection type.
-  `test_list_enabled_only` function L390-410 — `()` — implementation at runtime based on the database connection type.
-  `test_list_limit_and_offset` function L414-437 — `()` — implementation at runtime based on the database connection type.
-  `test_enable_disable` function L443-460 — `()` — implementation at runtime based on the database connection type.
-  `test_delete` function L466-479 — `()` — implementation at runtime based on the database connection type.
-  `test_find_by_workflow` function L485-511 — `()` — implementation at runtime based on the database connection type.
-  `test_find_by_workflow_no_match` function L515-523 — `()` — implementation at runtime based on the database connection type.
-  `test_update_schedule_times` function L529-549 — `()` — implementation at runtime based on the database connection type.
-  `test_get_due_cron_schedules` function L555-589 — `()` — implementation at runtime based on the database connection type.
-  `test_claim_and_update_cron` function L595-620 — `()` — implementation at runtime based on the database connection type.
-  `test_get_enabled_triggers` function L626-655 — `()` — implementation at runtime based on the database connection type.
-  `test_update_last_poll` function L659-676 — `()` — implementation at runtime based on the database connection type.
-  `test_get_by_trigger_name` function L680-705 — `()` — implementation at runtime based on the database connection type.
-  `test_upsert_trigger_insert` function L709-721 — `()` — implementation at runtime based on the database connection type.
-  `test_update_cron_expression_and_timezone` function L727-750 — `()` — implementation at runtime based on the database connection type.

### crates/cloacina/src/dal/unified/schedule_execution

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/dal/unified/schedule_execution/crud.rs

-  `create_postgres` function L35-75 — `( &self, new_execution: NewScheduleExecution, ) -> Result<ScheduleExecution, Val...` — CRUD operations for unified schedule executions.
-  `create_sqlite` function L78-118 — `( &self, new_execution: NewScheduleExecution, ) -> Result<ScheduleExecution, Val...` — CRUD operations for unified schedule executions.
-  `get_by_id_postgres` function L121-138 — `( &self, id: UniversalUuid, ) -> Result<ScheduleExecution, ValidationError>` — CRUD operations for unified schedule executions.
-  `get_by_id_sqlite` function L141-158 — `( &self, id: UniversalUuid, ) -> Result<ScheduleExecution, ValidationError>` — CRUD operations for unified schedule executions.
-  `list_by_schedule_postgres` function L161-187 — `( &self, schedule_id: UniversalUuid, limit: i64, offset: i64, ) -> Result<Vec<Sc...` — CRUD operations for unified schedule executions.
-  `list_by_schedule_sqlite` function L190-216 — `( &self, schedule_id: UniversalUuid, limit: i64, offset: i64, ) -> Result<Vec<Sc...` — CRUD operations for unified schedule executions.
-  `complete_postgres` function L219-246 — `( &self, id: UniversalUuid, completed_at: DateTime<Utc>, ) -> Result<(), Validat...` — CRUD operations for unified schedule executions.
-  `complete_sqlite` function L249-276 — `( &self, id: UniversalUuid, completed_at: DateTime<Utc>, ) -> Result<(), Validat...` — CRUD operations for unified schedule executions.
-  `has_active_execution_postgres` function L279-304 — `( &self, schedule_id: UniversalUuid, context_hash: String, ) -> Result<bool, Val...` — CRUD operations for unified schedule executions.
-  `has_active_execution_sqlite` function L307-332 — `( &self, schedule_id: UniversalUuid, context_hash: String, ) -> Result<bool, Val...` — CRUD operations for unified schedule executions.
-  `update_workflow_execution_id_postgres` function L335-361 — `( &self, id: UniversalUuid, workflow_execution_id: UniversalUuid, ) -> Result<()...` — CRUD operations for unified schedule executions.
-  `update_workflow_execution_id_sqlite` function L364-390 — `( &self, id: UniversalUuid, workflow_execution_id: UniversalUuid, ) -> Result<()...` — CRUD operations for unified schedule executions.
-  `find_lost_executions_postgres` function L393-419 — `( &self, older_than_minutes: i32, ) -> Result<Vec<ScheduleExecution>, Validation...` — CRUD operations for unified schedule executions.
-  `find_lost_executions_sqlite` function L422-448 — `( &self, older_than_minutes: i32, ) -> Result<Vec<ScheduleExecution>, Validation...` — CRUD operations for unified schedule executions.
-  `get_latest_by_schedule_postgres` function L451-474 — `( &self, schedule_id: UniversalUuid, ) -> Result<Option<ScheduleExecution>, Vali...` — CRUD operations for unified schedule executions.
-  `get_latest_by_schedule_sqlite` function L477-500 — `( &self, schedule_id: UniversalUuid, ) -> Result<Option<ScheduleExecution>, Vali...` — CRUD operations for unified schedule executions.
-  `get_execution_stats_postgres` function L503-562 — `( &self, since: DateTime<Utc>, ) -> Result<super::ScheduleExecutionStats, Valida...` — CRUD operations for unified schedule executions.
-  `get_execution_stats_sqlite` function L565-630 — `( &self, since: DateTime<Utc>, ) -> Result<super::ScheduleExecutionStats, Valida...` — CRUD operations for unified schedule executions.

#### crates/cloacina/src/dal/unified/schedule_execution/mod.rs

- pub `ScheduleExecutionStats` struct L34-43 — `{ total_executions: i64, successful_executions: i64, lost_executions: i64, succe...` — Statistics about schedule execution performance
- pub `ScheduleExecutionDAL` struct L47-49 — `{ dal: &'a DAL }` — Data access layer for unified schedule execution operations with runtime backend selection.
- pub `new` function L53-55 — `(dal: &'a DAL) -> Self` — Creates a new ScheduleExecutionDAL instance.
- pub `create` function L58-67 — `( &self, new_execution: NewScheduleExecution, ) -> Result<ScheduleExecution, Val...` — Creates a new schedule execution record in the database.
- pub `get_by_id` function L70-76 — `(&self, id: UniversalUuid) -> Result<ScheduleExecution, ValidationError>` — Retrieves a schedule execution by its ID.
- pub `list_by_schedule` function L79-92 — `( &self, schedule_id: UniversalUuid, limit: i64, offset: i64, ) -> Result<Vec<Sc...` — Lists schedule executions for a given schedule.
- pub `complete` function L95-105 — `( &self, id: UniversalUuid, completed_at: DateTime<Utc>, ) -> Result<(), Validat...` — Marks a schedule execution as completed.
- pub `has_active_execution` function L108-121 — `( &self, schedule_id: UniversalUuid, context_hash: &str, ) -> Result<bool, Valid...` — Checks if there is an active (uncompleted) execution for a schedule with the given context hash.
- pub `update_workflow_execution_id` function L124-136 — `( &self, id: UniversalUuid, workflow_execution_id: UniversalUuid, ) -> Result<()...` — Updates the workflow execution ID for a schedule execution.
- pub `find_lost_executions` function L139-148 — `( &self, older_than_minutes: i32, ) -> Result<Vec<ScheduleExecution>, Validation...` — Finds lost executions (started but not completed) older than the specified minutes.
- pub `get_latest_by_schedule` function L151-160 — `( &self, schedule_id: UniversalUuid, ) -> Result<Option<ScheduleExecution>, Vali...` — Gets the latest execution for a given schedule.
- pub `get_execution_stats` function L163-172 — `( &self, since: DateTime<Utc>, ) -> Result<ScheduleExecutionStats, ValidationErr...` — Gets execution statistics for monitoring and alerting.
-  `crud` module L24 — `-` — Unified Schedule Execution DAL with runtime backend selection
-  `tests` module L176-588 — `-` — implementation at runtime based on the database connection type.
-  `unique_dal` function L183-193 — `() -> DAL` — implementation at runtime based on the database connection type.
-  `create_schedule` function L197-205 — `(dal: &DAL) -> UniversalUuid` — Helper: create a cron schedule and return its ID.
-  `new_exec` function L209-217 — `(schedule_id: UniversalUuid) -> NewScheduleExecution` — Helper: build a NewScheduleExecution for a given schedule.
-  `test_create_execution` function L223-238 — `()` — implementation at runtime based on the database connection type.
-  `test_get_by_id` function L242-258 — `()` — implementation at runtime based on the database connection type.
-  `test_get_by_id_not_found` function L262-269 — `()` — implementation at runtime based on the database connection type.
-  `test_list_by_schedule` function L275-314 — `()` — implementation at runtime based on the database connection type.
-  `test_complete_execution` function L320-338 — `()` — implementation at runtime based on the database connection type.
-  `test_has_active_execution` function L344-375 — `()` — implementation at runtime based on the database connection type.
-  `test_has_active_execution_completed_not_active` function L379-399 — `()` — implementation at runtime based on the database connection type.
-  `test_update_workflow_execution_id` function L405-435 — `()` — implementation at runtime based on the database connection type.
-  `test_get_latest_by_schedule` function L441-472 — `()` — implementation at runtime based on the database connection type.
-  `test_find_lost_executions_none_lost` function L478-495 — `()` — implementation at runtime based on the database connection type.
-  `test_find_lost_executions_completed_not_lost` function L499-521 — `()` — implementation at runtime based on the database connection type.
-  `test_get_execution_stats_empty` function L527-541 — `()` — implementation at runtime based on the database connection type.
-  `test_get_execution_stats_with_data` function L545-587 — `()` — implementation at runtime based on the database connection type.

### crates/cloacina/src/dal/unified/task_execution

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/dal/unified/task_execution/claiming.rs

- pub `schedule_retry` function L37-50 — `( &self, task_id: UniversalUuid, retry_at: UniversalTimestamp, new_attempt: i32,...` — Updates a task's retry schedule with a new attempt count and retry time.
- pub `claim_ready_task` function L206-215 — `( &self, limit: usize, ) -> Result<Vec<ClaimResult>, ValidationError>` — Atomically claims up to `limit` ready tasks for execution.
- pub `claim_for_runner` function L424-434 — `( &self, task_id: UniversalUuid, runner_id: UniversalUuid, ) -> Result<RunnerCla...` — Atomically claim a task for a specific runner.
- pub `heartbeat` function L516-526 — `( &self, task_id: UniversalUuid, runner_id: UniversalUuid, ) -> Result<Heartbeat...` — Update heartbeat for a claimed task.
- pub `release_runner_claim` function L605-614 — `( &self, task_id: UniversalUuid, ) -> Result<(), ValidationError>` — Release a runner's claim on a task (on completion or failure).
- pub `find_stale_claims` function L676-685 — `( &self, threshold: std::time::Duration, ) -> Result<Vec<StaleClaim>, Validation...` — Find tasks with stale claims (heartbeat older than threshold).
- pub `get_ready_for_retry` function L768-774 — `(&self) -> Result<Vec<TaskExecution>, ValidationError>` — Retrieves tasks that are ready for retry (retry_at time has passed).
-  `schedule_retry_postgres` function L53-125 — `( &self, task_id: UniversalUuid, retry_at: UniversalTimestamp, new_attempt: i32,...` — are written atomically.
-  `schedule_retry_sqlite` function L128-200 — `( &self, task_id: UniversalUuid, retry_at: UniversalTimestamp, new_attempt: i32,...` — are written atomically.
-  `claim_ready_task_postgres` function L218-311 — `( &self, limit: usize, ) -> Result<Vec<ClaimResult>, ValidationError>` — are written atomically.
-  `PgClaimResult` struct L235-244 — `{ id: Uuid, workflow_execution_id: Uuid, task_name: String, attempt: i32 }` — are written atomically.
-  `claim_ready_task_sqlite` function L314-414 — `( &self, limit: usize, ) -> Result<Vec<ClaimResult>, ValidationError>` — are written atomically.
-  `claim_for_runner_postgres` function L437-472 — `( &self, task_id: UniversalUuid, runner_id: UniversalUuid, ) -> Result<RunnerCla...` — are written atomically.
-  `claim_for_runner_sqlite` function L475-510 — `( &self, task_id: UniversalUuid, runner_id: UniversalUuid, ) -> Result<RunnerCla...` — are written atomically.
-  `heartbeat_postgres` function L529-563 — `( &self, task_id: UniversalUuid, runner_id: UniversalUuid, ) -> Result<Heartbeat...` — are written atomically.
-  `heartbeat_sqlite` function L566-600 — `( &self, task_id: UniversalUuid, runner_id: UniversalUuid, ) -> Result<Heartbeat...` — are written atomically.
-  `release_runner_claim_postgres` function L617-642 — `( &self, task_id: UniversalUuid, ) -> Result<(), ValidationError>` — are written atomically.
-  `release_runner_claim_sqlite` function L645-670 — `( &self, task_id: UniversalUuid, ) -> Result<(), ValidationError>` — are written atomically.
-  `find_stale_claims_postgres` function L688-725 — `( &self, threshold: std::time::Duration, ) -> Result<Vec<StaleClaim>, Validation...` — are written atomically.
-  `find_stale_claims_sqlite` function L728-765 — `( &self, threshold: std::time::Duration, ) -> Result<Vec<StaleClaim>, Validation...` — are written atomically.
-  `get_ready_for_retry_postgres` function L777-801 — `(&self) -> Result<Vec<TaskExecution>, ValidationError>` — are written atomically.
-  `get_ready_for_retry_sqlite` function L804-828 — `(&self) -> Result<Vec<TaskExecution>, ValidationError>` — are written atomically.

#### crates/cloacina/src/dal/unified/task_execution/crud.rs

- pub `create` function L38-47 — `( &self, new_task: NewTaskExecution, ) -> Result<TaskExecution, ValidationError>` — Creates a new task execution record in the database.
- pub `get_by_id` function L172-181 — `( &self, task_id: UniversalUuid, ) -> Result<TaskExecution, ValidationError>` — Retrieves a specific task execution by its ID.
- pub `get_all_tasks_for_workflow` function L224-235 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<Vec<TaskExecution>, V...` — Retrieves all tasks associated with a workflow execution.
-  `create_postgres` function L50-108 — `( &self, new_task: NewTaskExecution, ) -> Result<TaskExecution, ValidationError>` — are written atomically.
-  `create_sqlite` function L111-169 — `( &self, new_task: NewTaskExecution, ) -> Result<TaskExecution, ValidationError>` — are written atomically.
-  `get_by_id_postgres` function L184-201 — `( &self, task_id: UniversalUuid, ) -> Result<TaskExecution, ValidationError>` — are written atomically.
-  `get_by_id_sqlite` function L204-221 — `( &self, task_id: UniversalUuid, ) -> Result<TaskExecution, ValidationError>` — are written atomically.
-  `get_all_tasks_for_workflow_postgres` function L238-259 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<Vec<TaskExecution>, V...` — are written atomically.
-  `get_all_tasks_for_workflow_sqlite` function L262-283 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<Vec<TaskExecution>, V...` — are written atomically.

#### crates/cloacina/src/dal/unified/task_execution/mod.rs

- pub `RetryStats` struct L40-49 — `{ tasks_with_retries: i32, total_retries: i32, max_attempts_used: i32, tasks_exh...` — Statistics about retry behavior for a workflow execution.
- pub `ClaimResult` struct L53-62 — `{ id: UniversalUuid, workflow_execution_id: UniversalUuid, task_name: String, at...` — Result structure for atomic task claiming operations.
- pub `RunnerClaimResult` enum L66-71 — `Claimed | AlreadyClaimed` — Result of attempting to claim a task for a specific runner.
- pub `HeartbeatResult` enum L75-80 — `Ok | ClaimLost` — Result of a heartbeat attempt.
- pub `StaleClaim` struct L84-91 — `{ task_id: UniversalUuid, claimed_by: UniversalUuid, heartbeat_at: chrono::DateT...` — A task with a stale claim (heartbeat expired).
- pub `TaskExecutionDAL` struct L95-97 — `{ dal: &'a DAL }` — Data access layer for task execution operations with runtime backend selection.
- pub `new` function L101-103 — `(dal: &'a DAL) -> Self` — Creates a new TaskExecutionDAL instance.
-  `claiming` module L29 — `-` — Task Execution Data Access Layer for Unified Backend Support
-  `crud` module L30 — `-` — - Workflow completion and failure detection
-  `queries` module L31 — `-` — - Workflow completion and failure detection
-  `recovery` module L32 — `-` — - Workflow completion and failure detection
-  `state` module L33 — `-` — - Workflow completion and failure detection

#### crates/cloacina/src/dal/unified/task_execution/queries.rs

- pub `get_pending_tasks` function L29-38 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<Vec<TaskExecution>, V...` — Retrieves all pending (NotStarted) tasks for a specific workflow execution.
- pub `get_pending_tasks_batch` function L91-102 — `( &self, workflow_execution_ids: Vec<UniversalUuid>, ) -> Result<Vec<TaskExecuti...` — Gets all pending tasks for multiple workflow executions in a single query.
- pub `check_workflow_completion` function L163-174 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<bool, ValidationError...` — Checks if all tasks in a workflow execution have reached a terminal state.
- pub `get_task_status` function L229-241 — `( &self, workflow_execution_id: UniversalUuid, task_name: &str, ) -> Result<Stri...` — Gets the current status of a specific task in a workflow execution.
- pub `get_task_statuses_batch` function L300-312 — `( &self, workflow_execution_id: UniversalUuid, task_names: Vec<String>, ) -> Res...` — Gets the status of multiple tasks in a single database query.
-  `get_pending_tasks_postgres` function L41-63 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<Vec<TaskExecution>, V...` — Query operations for task executions.
-  `get_pending_tasks_sqlite` function L66-88 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<Vec<TaskExecution>, V...` — Query operations for task executions.
-  `get_pending_tasks_batch_postgres` function L105-131 — `( &self, workflow_execution_ids: Vec<UniversalUuid>, ) -> Result<Vec<TaskExecuti...` — Query operations for task executions.
-  `get_pending_tasks_batch_sqlite` function L134-160 — `( &self, workflow_execution_ids: Vec<UniversalUuid>, ) -> Result<Vec<TaskExecuti...` — Query operations for task executions.
-  `check_workflow_completion_postgres` function L177-200 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<bool, ValidationError...` — Query operations for task executions.
-  `check_workflow_completion_sqlite` function L203-226 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<bool, ValidationError...` — Query operations for task executions.
-  `get_task_status_postgres` function L244-269 — `( &self, workflow_execution_id: UniversalUuid, task_name: &str, ) -> Result<Stri...` — Query operations for task executions.
-  `get_task_status_sqlite` function L272-297 — `( &self, workflow_execution_id: UniversalUuid, task_name: &str, ) -> Result<Stri...` — Query operations for task executions.
-  `get_task_statuses_batch_postgres` function L315-345 — `( &self, workflow_execution_id: UniversalUuid, task_names: Vec<String>, ) -> Res...` — Query operations for task executions.
-  `get_task_statuses_batch_sqlite` function L348-378 — `( &self, workflow_execution_id: UniversalUuid, task_names: Vec<String>, ) -> Res...` — Query operations for task executions.

#### crates/cloacina/src/dal/unified/task_execution/recovery.rs

- pub `get_orphaned_tasks` function L29-35 — `(&self) -> Result<Vec<TaskExecution>, ValidationError>` — Retrieves tasks that are stuck in "Running" state (orphaned tasks).
- pub `reset_task_for_recovery` function L80-89 — `( &self, task_id: UniversalUuid, ) -> Result<(), ValidationError>` — Resets a task from "Running" to "Ready" state for recovery.
- pub `check_workflow_failure` function L152-163 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<bool, ValidationError...` — Checks if a workflow should be marked as failed due to abandoned tasks.
- pub `get_retry_stats` function L220-247 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<RetryStats, Validatio...` — Calculates retry statistics for a specific workflow execution.
- pub `get_exhausted_retry_tasks` function L250-265 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<Vec<TaskExecution>, V...` — Retrieves tasks that have exceeded their retry limit.
-  `get_orphaned_tasks_postgres` function L38-56 — `(&self) -> Result<Vec<TaskExecution>, ValidationError>` — Recovery operations for orphaned and failed tasks.
-  `get_orphaned_tasks_sqlite` function L59-77 — `(&self) -> Result<Vec<TaskExecution>, ValidationError>` — Recovery operations for orphaned and failed tasks.
-  `reset_task_for_recovery_postgres` function L92-119 — `( &self, task_id: UniversalUuid, ) -> Result<(), ValidationError>` — Recovery operations for orphaned and failed tasks.
-  `reset_task_for_recovery_sqlite` function L122-149 — `( &self, task_id: UniversalUuid, ) -> Result<(), ValidationError>` — Recovery operations for orphaned and failed tasks.
-  `check_workflow_failure_postgres` function L166-190 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<bool, ValidationError...` — Recovery operations for orphaned and failed tasks.
-  `check_workflow_failure_sqlite` function L193-217 — `( &self, workflow_execution_id: UniversalUuid, ) -> Result<bool, ValidationError...` — Recovery operations for orphaned and failed tasks.
-  `tests` module L269-543 — `-` — Recovery operations for orphaned and failed tasks.
-  `unique_dal` function L277-287 — `() -> DAL` — Recovery operations for orphaned and failed tasks.
-  `create_workflow` function L291-302 — `(dal: &DAL) -> UniversalUuid` — Helper: create a workflow execution and return its ID.
-  `create_task` function L306-327 — `( dal: &DAL, workflow_id: UniversalUuid, name: &str, status: &str, attempt: i32,...` — Helper: create a task with a given status, returning its ID.
-  `test_get_orphaned_tasks_none` function L333-341 — `()` — Recovery operations for orphaned and failed tasks.
-  `test_get_orphaned_tasks_finds_running` function L345-355 — `()` — Recovery operations for orphaned and failed tasks.
-  `test_reset_task_for_recovery` function L361-376 — `()` — Recovery operations for orphaned and failed tasks.
-  `test_reset_task_increments_recovery_attempts` function L380-401 — `()` — Recovery operations for orphaned and failed tasks.
-  `test_check_workflow_failure_no_abandoned` function L407-418 — `()` — Recovery operations for orphaned and failed tasks.
-  `test_check_workflow_failure_with_abandoned` function L422-439 — `()` — Recovery operations for orphaned and failed tasks.
-  `test_check_workflow_failure_regular_failure_not_abandoned` function L443-460 — `()` — Recovery operations for orphaned and failed tasks.
-  `test_get_retry_stats_no_retries` function L466-481 — `()` — Recovery operations for orphaned and failed tasks.
-  `test_get_retry_stats_with_retries` function L485-505 — `()` — Recovery operations for orphaned and failed tasks.
-  `test_get_exhausted_retry_tasks` function L511-527 — `()` — Recovery operations for orphaned and failed tasks.
-  `test_get_exhausted_retry_tasks_empty` function L531-542 — `()` — Recovery operations for orphaned and failed tasks.

#### crates/cloacina/src/dal/unified/task_execution/state.rs

- pub `mark_completed` function L41-51 — `( &self, task_id: UniversalUuid, runner_id: Option<UniversalUuid>, ) -> Result<b...` — Marks a task execution as completed.
- pub `mark_failed` function L199-212 — `( &self, task_id: UniversalUuid, error_message: &str, runner_id: Option<Universa...` — Marks a task execution as failed with an error message.
- pub `mark_ready` function L367-373 — `(&self, task_id: UniversalUuid) -> Result<(), ValidationError>` — Marks a task as ready for execution.
- pub `mark_skipped` function L499-509 — `( &self, task_id: UniversalUuid, reason: &str, ) -> Result<(), ValidationError>` — Marks a task as skipped with a provided reason.
- pub `mark_abandoned` function L633-643 — `( &self, task_id: UniversalUuid, reason: &str, ) -> Result<(), ValidationError>` — Marks a task as permanently abandoned after too many recovery attempts.
- pub `set_sub_status` function L766-776 — `( &self, task_id: UniversalUuid, sub_status: Option<&str>, ) -> Result<(), Valid...` — Updates the sub_status of a running task execution.
- pub `reset_retry_state` function L910-916 — `(&self, task_id: UniversalUuid) -> Result<(), ValidationError>` — Resets the retry state for a task to its initial state.
-  `mark_completed_postgres` function L54-120 — `( &self, task_id: UniversalUuid, runner_id: Option<UniversalUuid>, ) -> Result<b...` — are written atomically.
-  `mark_completed_sqlite` function L123-189 — `( &self, task_id: UniversalUuid, runner_id: Option<UniversalUuid>, ) -> Result<b...` — are written atomically.
-  `mark_failed_postgres` function L215-285 — `( &self, task_id: UniversalUuid, error_message: &str, runner_id: Option<Universa...` — are written atomically.
-  `mark_failed_sqlite` function L288-358 — `( &self, task_id: UniversalUuid, error_message: &str, runner_id: Option<Universa...` — are written atomically.
-  `mark_ready_postgres` function L376-433 — `(&self, task_id: UniversalUuid) -> Result<(), ValidationError>` — are written atomically.
-  `mark_ready_sqlite` function L436-493 — `(&self, task_id: UniversalUuid) -> Result<(), ValidationError>` — are written atomically.
-  `mark_skipped_postgres` function L512-568 — `( &self, task_id: UniversalUuid, reason: &str, ) -> Result<(), ValidationError>` — are written atomically.
-  `mark_skipped_sqlite` function L571-627 — `( &self, task_id: UniversalUuid, reason: &str, ) -> Result<(), ValidationError>` — are written atomically.
-  `mark_abandoned_postgres` function L646-701 — `( &self, task_id: UniversalUuid, reason: &str, ) -> Result<(), ValidationError>` — are written atomically.
-  `mark_abandoned_sqlite` function L704-759 — `( &self, task_id: UniversalUuid, reason: &str, ) -> Result<(), ValidationError>` — are written atomically.
-  `set_sub_status_postgres` function L779-840 — `( &self, task_id: UniversalUuid, sub_status: Option<&str>, ) -> Result<(), Valid...` — are written atomically.
-  `set_sub_status_sqlite` function L843-904 — `( &self, task_id: UniversalUuid, sub_status: Option<&str>, ) -> Result<(), Valid...` — are written atomically.
-  `reset_retry_state_postgres` function L919-974 — `( &self, task_id: UniversalUuid, ) -> Result<(), ValidationError>` — are written atomically.
-  `reset_retry_state_sqlite` function L977-1032 — `( &self, task_id: UniversalUuid, ) -> Result<(), ValidationError>` — are written atomically.

### crates/cloacina/src/database

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/database/admin.rs

- pub `DatabaseAdmin` struct L37-39 — `{ database: Database }` — Database administrator for tenant provisioning
- pub `TenantConfig` struct L42-49 — `{ schema_name: String, username: String, password: String }` — Configuration for creating a new tenant
- pub `TenantCredentials` struct L52-61 — `{ username: String, password: String, schema_name: String, connection_string: St...` — Credentials returned after tenant creation
- pub `AdminError` enum L65-83 — `Database | Pool | SqlExecution | InvalidConfig | InvalidSchema | InvalidUsername` — Errors that can occur during database administration
- pub `new` function L100-102 — `(database: Database) -> Self` — Create a new database administrator
- pub `create_tenant` function L108-236 — `( &self, tenant_config: TenantConfig, ) -> Result<TenantCredentials, AdminError>` — Create a complete tenant setup (schema + user + permissions + migrations)
- pub `remove_tenant` function L241-304 — `( &self, schema_name: &str, username: &str, ) -> Result<(), AdminError>` — Remove a tenant (user + schema)
- pub `list_tenant_schemas` function L319-354 — `(&self) -> Result<Vec<String>, AdminError>` — List all non-system schemas (tenant schemas).
-  `postgres_impl` module L26-472 — `-` — Note: This module is only available when using the PostgreSQL backend.
-  `AdminError` type L85-89 — `= AdminError` — Note: This module is only available when using the PostgreSQL backend.
-  `from` function L86-88 — `(err: deadpool::managed::PoolError<deadpool_diesel::postgres::Manager>) -> Self` — Note: This module is only available when using the PostgreSQL backend.
-  `AdminError` type L91-95 — `= AdminError` — Note: This module is only available when using the PostgreSQL backend.
-  `from` function L92-94 — `(err: deadpool::managed::PoolError<deadpool_diesel::Error>) -> Self` — Note: This module is only available when using the PostgreSQL backend.
-  `DatabaseAdmin` type L98-355 — `= DatabaseAdmin` — Note: This module is only available when using the PostgreSQL backend.
-  `build_connection_string` function L306-316 — `(&self, username: &str, password: &str) -> String` — Note: This module is only available when using the PostgreSQL backend.
-  `SchemaRow` struct L331-334 — `{ nspname: String }` — Note: This module is only available when using the PostgreSQL backend.
-  `generate_secure_password` function L358-370 — `(length: usize) -> String` — Note: This module is only available when using the PostgreSQL backend.
-  `tests` module L373-471 — `-` — Note: This module is only available when using the PostgreSQL backend.
-  `test_generate_secure_password` function L377-387 — `()` — Note: This module is only available when using the PostgreSQL backend.
-  `test_tenant_config_validation` function L390-402 — `()` — Note: This module is only available when using the PostgreSQL backend.
-  `test_username_validation_rejects_sql_injection` function L405-425 — `()` — Note: This module is only available when using the PostgreSQL backend.
-  `test_schema_validation_rejects_sql_injection` function L428-442 — `()` — Note: This module is only available when using the PostgreSQL backend.
-  `test_reserved_usernames_rejected` function L445-457 — `()` — Note: This module is only available when using the PostgreSQL backend.
-  `test_password_escaping` function L460-470 — `()` — Note: This module is only available when using the PostgreSQL backend.

#### crates/cloacina/src/database/mod.rs

- pub `admin` module L99 — `-` — # Database Layer
- pub `connection` module L100 — `-` — database access, migrations can be run manually using `run_migrations()`.
- pub `schema` module L101 — `-` — database access, migrations can be run manually using `run_migrations()`.
- pub `universal_types` module L102 — `-` — database access, migrations can be run manually using `run_migrations()`.
- pub `Result` type L123 — `= std::result::Result<T, diesel::result::Error>` — Type alias for database operation results.
- pub `POSTGRES_MIGRATIONS` variable L133-134 — `: EmbeddedMigrations` — Embedded migrations for PostgreSQL.
- pub `SQLITE_MIGRATIONS` variable L138-139 — `: EmbeddedMigrations` — Embedded migrations for SQLite.
- pub `MIGRATIONS` variable L147 — `: EmbeddedMigrations` — Embedded migrations for automatic schema management.
- pub `MIGRATIONS` variable L151 — `: EmbeddedMigrations` — Embedded migrations alias (defaults to SQLite when postgres not enabled)
- pub `run_migrations` function L185-189 — `(conn: &mut DbConnection) -> Result<()>` — database access, migrations can be run manually using `run_migrations()`.
- pub `run_migrations_postgres` function L206-210 — `(conn: &mut diesel::pg::PgConnection) -> Result<()>` — Runs pending PostgreSQL database migrations.
- pub `run_migrations_sqlite` function L227-231 — `(conn: &mut diesel::sqlite::SqliteConnection) -> Result<()>` — Runs pending SQLite database migrations.

#### crates/cloacina/src/database/schema.rs

- pub `unified` module L1067-1069 — `-`
- pub `postgres` module L1074-1076 — `-`
- pub `sqlite` module L1079-1081 — `-`
-  `unified_schema` module L25-392 — `-`
-  `postgres_schema` module L399-769 — `-`
-  `sqlite_schema` module L772-1062 — `-`

#### crates/cloacina/src/database/universal_types.rs

- pub `DbUuid` struct L56 — `-` — Custom SQL type for UUIDs that works across backends.
- pub `DbTimestamp` struct L64 — `-` — Custom SQL type for timestamps that works across backends.
- pub `DbBool` struct L72 — `-` — Custom SQL type for booleans that works across backends.
- pub `DbBinary` struct L80 — `-` — Custom SQL type for binary data that works across backends.
- pub `UniversalUuid` struct L90 — `-` — Diesel-specific code isolated in backend-specific model modules.
- pub `new_v4` function L93-95 — `() -> Self` — Diesel-specific code isolated in backend-specific model modules.
- pub `as_uuid` function L97-99 — `(&self) -> Uuid` — Diesel-specific code isolated in backend-specific model modules.
- pub `as_bytes` function L102-104 — `(&self) -> &[u8; 16]` — Convert to bytes for SQLite BLOB storage
- pub `from_bytes` function L107-109 — `(bytes: &[u8]) -> Result<Self, uuid::Error>` — Create from bytes (SQLite BLOB)
- pub `UniversalTimestamp` struct L184 — `-` — Diesel-specific code isolated in backend-specific model modules.
- pub `now` function L187-189 — `() -> Self` — Diesel-specific code isolated in backend-specific model modules.
- pub `as_datetime` function L191-193 — `(&self) -> &DateTime<Utc>` — Diesel-specific code isolated in backend-specific model modules.
- pub `into_inner` function L195-197 — `(self) -> DateTime<Utc>` — Diesel-specific code isolated in backend-specific model modules.
- pub `to_rfc3339` function L200-202 — `(&self) -> String` — Convert to RFC3339 string for SQLite TEXT storage
- pub `from_rfc3339` function L205-207 — `(s: &str) -> Result<Self, chrono::ParseError>` — Create from RFC3339 string (SQLite TEXT)
- pub `to_naive` function L210-212 — `(&self) -> chrono::NaiveDateTime` — Convert to NaiveDateTime for PostgreSQL TIMESTAMP storage
- pub `from_naive` function L215-218 — `(naive: chrono::NaiveDateTime) -> Self` — Create from NaiveDateTime (PostgreSQL TIMESTAMP)
- pub `current_timestamp` function L295-297 — `() -> UniversalTimestamp` — Helper function for current timestamp
- pub `UniversalBool` struct L307 — `-` — Diesel-specific code isolated in backend-specific model modules.
- pub `new` function L310-312 — `(value: bool) -> Self` — Diesel-specific code isolated in backend-specific model modules.
- pub `is_true` function L314-316 — `(&self) -> bool` — Diesel-specific code isolated in backend-specific model modules.
- pub `is_false` function L318-320 — `(&self) -> bool` — Diesel-specific code isolated in backend-specific model modules.
- pub `to_i32` function L323-329 — `(&self) -> i32` — Convert to i32 for SQLite INTEGER storage
- pub `from_i32` function L332-334 — `(value: i32) -> Self` — Create from i32 (SQLite INTEGER)
- pub `UniversalBinary` struct L400 — `-` — Universal binary wrapper for cross-database compatibility
- pub `new` function L403-405 — `(data: Vec<u8>) -> Self` — Diesel-specific code isolated in backend-specific model modules.
- pub `as_slice` function L407-409 — `(&self) -> &[u8]` — Diesel-specific code isolated in backend-specific model modules.
- pub `into_inner` function L411-413 — `(self) -> Vec<u8>` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalUuid` type L92-110 — `= UniversalUuid` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalUuid` type L112-116 — `= UniversalUuid` — Diesel-specific code isolated in backend-specific model modules.
-  `fmt` function L113-115 — `(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalUuid` type L118-122 — `= UniversalUuid` — Diesel-specific code isolated in backend-specific model modules.
-  `from` function L119-121 — `(uuid: Uuid) -> Self` — Diesel-specific code isolated in backend-specific model modules.
-  `Uuid` type L124-128 — `= Uuid` — Diesel-specific code isolated in backend-specific model modules.
-  `from` function L125-127 — `(wrapper: UniversalUuid) -> Self` — Diesel-specific code isolated in backend-specific model modules.
-  `Uuid` type L130-134 — `= Uuid` — Diesel-specific code isolated in backend-specific model modules.
-  `from` function L131-133 — `(wrapper: &UniversalUuid) -> Self` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalUuid` type L138-144 — `= UniversalUuid` — Diesel-specific code isolated in backend-specific model modules.
-  `from_sql` function L139-143 — `(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self>` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalUuid` type L147-151 — `= UniversalUuid` — Diesel-specific code isolated in backend-specific model modules.
-  `to_sql` function L148-150 — `(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Resul...` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalUuid` type L155-163 — `= UniversalUuid` — Diesel-specific code isolated in backend-specific model modules.
-  `from_sql` function L156-162 — `( bytes: diesel::sqlite::SqliteValue<'_, '_, '_>, ) -> diesel::deserialize::Resu...` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalUuid` type L166-174 — `= UniversalUuid` — Diesel-specific code isolated in backend-specific model modules.
-  `to_sql` function L167-173 — `( &'b self, out: &mut Output<'b, '_, diesel::sqlite::Sqlite>, ) -> diesel::seria...` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalTimestamp` type L186-219 — `= UniversalTimestamp` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalTimestamp` type L221-225 — `= UniversalTimestamp` — Diesel-specific code isolated in backend-specific model modules.
-  `fmt` function L222-224 — `(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalTimestamp` type L227-231 — `= UniversalTimestamp` — Diesel-specific code isolated in backend-specific model modules.
-  `from` function L228-230 — `(dt: DateTime<Utc>) -> Self` — Diesel-specific code isolated in backend-specific model modules.
-  `from` function L234-236 — `(wrapper: UniversalTimestamp) -> Self` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalTimestamp` type L239-243 — `= UniversalTimestamp` — Diesel-specific code isolated in backend-specific model modules.
-  `from` function L240-242 — `(naive: chrono::NaiveDateTime) -> Self` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalTimestamp` type L247-252 — `= UniversalTimestamp` — Diesel-specific code isolated in backend-specific model modules.
-  `from_sql` function L248-251 — `(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self>` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalTimestamp` type L255-269 — `= UniversalTimestamp` — Diesel-specific code isolated in backend-specific model modules.
-  `to_sql` function L256-268 — `(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Resul...` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalTimestamp` type L273-281 — `= UniversalTimestamp` — Diesel-specific code isolated in backend-specific model modules.
-  `from_sql` function L274-280 — `( bytes: diesel::sqlite::SqliteValue<'_, '_, '_>, ) -> diesel::deserialize::Resu...` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalTimestamp` type L284-292 — `= UniversalTimestamp` — Diesel-specific code isolated in backend-specific model modules.
-  `to_sql` function L285-291 — `( &'b self, out: &mut Output<'b, '_, diesel::sqlite::Sqlite>, ) -> diesel::seria...` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalBool` type L309-335 — `= UniversalBool` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalBool` type L337-341 — `= UniversalBool` — Diesel-specific code isolated in backend-specific model modules.
-  `from` function L338-340 — `(value: bool) -> Self` — Diesel-specific code isolated in backend-specific model modules.
-  `from` function L344-346 — `(wrapper: UniversalBool) -> Self` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalBool` type L349-353 — `= UniversalBool` — Diesel-specific code isolated in backend-specific model modules.
-  `fmt` function L350-352 — `(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalBool` type L357-362 — `= UniversalBool` — Diesel-specific code isolated in backend-specific model modules.
-  `from_sql` function L358-361 — `(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self>` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalBool` type L365-369 — `= UniversalBool` — Diesel-specific code isolated in backend-specific model modules.
-  `to_sql` function L366-368 — `(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Resul...` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalBool` type L373-380 — `= UniversalBool` — Diesel-specific code isolated in backend-specific model modules.
-  `from_sql` function L374-379 — `( bytes: diesel::sqlite::SqliteValue<'_, '_, '_>, ) -> diesel::deserialize::Resu...` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalBool` type L383-392 — `= UniversalBool` — Diesel-specific code isolated in backend-specific model modules.
-  `to_sql` function L384-391 — `( &'b self, out: &mut Output<'b, '_, diesel::sqlite::Sqlite>, ) -> diesel::seria...` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalBinary` type L402-414 — `= UniversalBinary` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalBinary` type L416-420 — `= UniversalBinary` — Diesel-specific code isolated in backend-specific model modules.
-  `from` function L417-419 — `(data: Vec<u8>) -> Self` — Diesel-specific code isolated in backend-specific model modules.
-  `from` function L423-425 — `(wrapper: UniversalBinary) -> Self` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalBinary` type L428-432 — `= UniversalBinary` — Diesel-specific code isolated in backend-specific model modules.
-  `from` function L429-431 — `(data: &[u8]) -> Self` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalBinary` type L436-442 — `= UniversalBinary` — Diesel-specific code isolated in backend-specific model modules.
-  `from_sql` function L437-441 — `(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self>` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalBinary` type L445-450 — `= UniversalBinary` — Diesel-specific code isolated in backend-specific model modules.
-  `to_sql` function L446-449 — `(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Resul...` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalBinary` type L454-461 — `= UniversalBinary` — Diesel-specific code isolated in backend-specific model modules.
-  `from_sql` function L455-460 — `( bytes: diesel::sqlite::SqliteValue<'_, '_, '_>, ) -> diesel::deserialize::Resu...` — Diesel-specific code isolated in backend-specific model modules.
-  `UniversalBinary` type L464-472 — `= UniversalBinary` — Diesel-specific code isolated in backend-specific model modules.
-  `to_sql` function L465-471 — `( &'b self, out: &mut Output<'b, '_, diesel::sqlite::Sqlite>, ) -> diesel::seria...` — Diesel-specific code isolated in backend-specific model modules.
-  `tests` module L475-582 — `-` — Diesel-specific code isolated in backend-specific model modules.
-  `test_universal_uuid_creation` function L479-488 — `()` — Diesel-specific code isolated in backend-specific model modules.
-  `test_universal_uuid_bytes` function L491-496 — `()` — Diesel-specific code isolated in backend-specific model modules.
-  `test_universal_uuid_display` function L499-503 — `()` — Diesel-specific code isolated in backend-specific model modules.
-  `test_universal_timestamp_now` function L506-509 — `()` — Diesel-specific code isolated in backend-specific model modules.
-  `test_universal_timestamp_rfc3339` function L512-519 — `()` — Diesel-specific code isolated in backend-specific model modules.
-  `test_universal_timestamp_naive` function L522-529 — `()` — Diesel-specific code isolated in backend-specific model modules.
-  `test_current_timestamp` function L532-535 — `()` — Diesel-specific code isolated in backend-specific model modules.
-  `test_universal_bool_creation` function L538-546 — `()` — Diesel-specific code isolated in backend-specific model modules.
-  `test_universal_bool_i32` function L549-559 — `()` — Diesel-specific code isolated in backend-specific model modules.
-  `test_universal_bool_conversion` function L562-572 — `()` — Diesel-specific code isolated in backend-specific model modules.
-  `test_universal_bool_display` function L575-581 — `()` — Diesel-specific code isolated in backend-specific model modules.

### crates/cloacina/src/database/connection

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/database/connection/backend.rs

- pub `BackendType` enum L36-43 — `Postgres | Sqlite` — Represents the database backend type, detected at runtime from the connection URL.
- pub `from_url` function L57-105 — `(url: &str) -> Self` — Detect the backend type from a connection URL.
- pub `AnyConnection` enum L121-126 — `Postgres | Sqlite` — Multi-connection enum that wraps both PostgreSQL and SQLite connections.
- pub `AnyConnection` type L130 — `= PgConnection` — When only PostgreSQL is enabled, AnyConnection is just a PgConnection.
- pub `AnyConnection` type L134 — `= SqliteConnection` — When only SQLite is enabled, AnyConnection is just a SqliteConnection.
- pub `AnyPool` enum L147-152 — `Postgres | Sqlite` — Pool enum that wraps both PostgreSQL and SQLite connection pools.
- pub `as_postgres` function L167-172 — `(&self) -> Option<&PgPool>` — Returns a reference to the PostgreSQL pool if this is a PostgreSQL backend.
- pub `as_sqlite` function L175-180 — `(&self) -> Option<&SqlitePool>` — Returns a reference to the SQLite pool if this is a SQLite backend.
- pub `expect_postgres` function L183-188 — `(&self) -> &PgPool` — Returns the PostgreSQL pool, panicking if this is not a PostgreSQL backend.
- pub `expect_sqlite` function L191-196 — `(&self) -> &SqlitePool` — Returns the SQLite pool, panicking if this is not a SQLite backend.
- pub `close` function L202-207 — `(&self)` — Closes the connection pool, releasing all connections.
- pub `AnyPool` type L212 — `= PgPool` — When only PostgreSQL is enabled, AnyPool is just a PgPool.
- pub `AnyPool` type L216 — `= SqlitePool` — When only SQLite is enabled, AnyPool is just a SqlitePool.
- pub `DbConnection` type L226 — `= PgConnection` — Type alias for the connection type (defaults to PostgreSQL)
- pub `DbConnection` type L230 — `= SqliteConnection` — Type alias for the connection type (SQLite when postgres not enabled)
- pub `DbConnectionManager` type L234 — `= PgManager` — Type alias for the connection manager (defaults to PostgreSQL)
- pub `DbPool` type L238 — `= PgPool` — Type alias for the connection pool (defaults to PostgreSQL)
- pub `DbPool` type L242 — `= SqlitePool` — Type alias for the connection pool (SQLite when postgres not enabled)
-  `BackendType` type L45-106 — `= BackendType` — Database backend types and runtime backend selection.
-  `AnyPool` type L155-162 — `= AnyPool` — Database backend types and runtime backend selection.
-  `fmt` function L156-161 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — Database backend types and runtime backend selection.
-  `AnyPool` type L165-208 — `= AnyPool` — Database backend types and runtime backend selection.
-  `dispatch_backend` macro L265-290 — `-` — Dispatches to backend-specific code based on compile-time features.

#### crates/cloacina/src/database/connection/mod.rs

- pub `DatabaseError` enum L83-103 — `PoolCreation | InvalidUrl | Schema | Migration` — Errors that can occur during database operations.
- pub `Database` struct L116-123 — `{ pool: AnyPool, backend: BackendType, schema: Option<String> }` — Represents a pool of database connections.
- pub `new` function L151-153 — `(connection_string: &str, database_name: &str, max_size: u32) -> Self` — Creates a new database connection pool with automatic backend detection.
- pub `new_with_schema` function L171-179 — `( connection_string: &str, database_name: &str, max_size: u32, schema: Option<&s...` — Creates a new database connection pool with optional schema support.
- pub `try_new_with_schema` function L197-313 — `( connection_string: &str, _database_name: &str, max_size: u32, schema: Option<&...` — Creates a new database connection pool with optional schema support.
- pub `backend` function L316-318 — `(&self) -> BackendType` — Returns the detected backend type.
- pub `schema` function L321-323 — `(&self) -> Option<&str>` — Returns the schema name if set.
- pub `pool` function L326-328 — `(&self) -> AnyPool` — Returns a clone of the connection pool.
- pub `get_connection` function L331-333 — `(&self) -> AnyPool` — Alias for `pool()` for backward compatibility.
- pub `close` function L349-352 — `(&self)` — Closes the connection pool, releasing all database connections.
- pub `run_migrations` function L374-449 — `(&self) -> Result<(), String>` — Runs pending database migrations for the appropriate backend.
- pub `setup_schema` function L461-513 — `(&self, schema: &str) -> Result<(), String>` — Sets up the PostgreSQL schema for multi-tenant isolation.
- pub `get_connection_with_schema` function L523-561 — `( &self, ) -> Result< deadpool::managed::Object<PgManager>, deadpool::managed::P...` — Gets a PostgreSQL connection with the schema search path set.
- pub `get_postgres_connection` function L567-574 — `( &self, ) -> Result< deadpool::managed::Object<PgManager>, deadpool::managed::P...` — Gets a PostgreSQL connection.
- pub `get_sqlite_connection` function L580-608 — `( &self, ) -> Result< deadpool::managed::Object<SqliteManager>, deadpool::manage...` — Gets a SQLite connection.
-  `backend` module L51 — `-` — Database connection management module supporting both PostgreSQL and SQLite.
-  `schema_validation` module L52 — `-` — ```
-  `Database` type L125-133 — `= Database` — ```
-  `fmt` function L126-132 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — ```
-  `Database` type L135-609 — `= Database` — ```
-  `build_postgres_url` function L355-359 — `(base_url: &str, database_name: &str) -> Result<String, url::ParseError>` — Builds a PostgreSQL connection URL.
-  `build_sqlite_url` function L362-369 — `(connection_string: &str) -> String` — Builds a SQLite connection URL.
-  `tests` module L612-709 — `-` — ```
-  `test_postgres_url_parsing_scenarios` function L616-640 — `()` — ```
-  `test_sqlite_connection_strings` function L643-659 — `()` — ```
-  `test_backend_type_detection` function L662-708 — `()` — ```

#### crates/cloacina/src/database/connection/schema_validation.rs

- pub `SchemaError` enum L39-57 — `InvalidLength | InvalidStart | InvalidCharacters | ReservedName` — Errors that can occur during schema name validation.
- pub `validate_schema_name` function L84-111 — `(name: &str) -> Result<&str, SchemaError>` — Validates a PostgreSQL schema name to prevent SQL injection.
- pub `UsernameError` enum L139-157 — `InvalidLength | InvalidStart | InvalidCharacters | ReservedName` — Errors that can occur during username validation.
- pub `validate_username` function L184-211 — `(name: &str) -> Result<&str, UsernameError>` — Validates a PostgreSQL username to prevent SQL injection.
- pub `escape_password` function L236-238 — `(password: &str) -> String` — Escapes a password string for safe use in PostgreSQL SQL statements.
-  `MAX_SCHEMA_NAME_LENGTH` variable L29 — `: usize` — Maximum length for PostgreSQL schema names (NAMEDATALEN - 1).
-  `RESERVED_SCHEMA_NAMES` variable L32 — `: &[&str]` — Reserved PostgreSQL schema names that cannot be used.
-  `RESERVED_USERNAMES` variable L118-132 — `: &[&str]` — Reserved PostgreSQL role names that cannot be used as tenant usernames.
-  `tests` module L241-590 — `-` — (schema names, usernames, etc.) to ensure they cannot be used for SQL injection.
-  `test_valid_schema_names` function L245-262 — `()` — (schema names, usernames, etc.) to ensure they cannot be used for SQL injection.
-  `test_sql_injection_attempts_rejected` function L265-301 — `()` — (schema names, usernames, etc.) to ensure they cannot be used for SQL injection.
-  `test_invalid_length` function L304-324 — `()` — (schema names, usernames, etc.) to ensure they cannot be used for SQL injection.
-  `test_invalid_start_character` function L327-351 — `()` — (schema names, usernames, etc.) to ensure they cannot be used for SQL injection.
-  `test_invalid_characters` function L354-386 — `()` — (schema names, usernames, etc.) to ensure they cannot be used for SQL injection.
-  `test_reserved_names` function L389-426 — `()` — (schema names, usernames, etc.) to ensure they cannot be used for SQL injection.
-  `test_schema_error_display` function L429-442 — `()` — (schema names, usernames, etc.) to ensure they cannot be used for SQL injection.
-  `test_unicode_characters_rejected` function L445-469 — `()` — (schema names, usernames, etc.) to ensure they cannot be used for SQL injection.
-  `test_valid_usernames` function L476-482 — `()` — (schema names, usernames, etc.) to ensure they cannot be used for SQL injection.
-  `test_username_sql_injection_rejected` function L485-515 — `()` — (schema names, usernames, etc.) to ensure they cannot be used for SQL injection.
-  `test_reserved_usernames` function L518-535 — `()` — (schema names, usernames, etc.) to ensure they cannot be used for SQL injection.
-  `test_username_invalid_length` function L538-549 — `()` — (schema names, usernames, etc.) to ensure they cannot be used for SQL injection.
-  `test_username_invalid_start` function L552-561 — `()` — (schema names, usernames, etc.) to ensure they cannot be used for SQL injection.
-  `test_escape_password_no_quotes` function L568-572 — `()` — (schema names, usernames, etc.) to ensure they cannot be used for SQL injection.
-  `test_escape_password_with_quotes` function L575-580 — `()` — (schema names, usernames, etc.) to ensure they cannot be used for SQL injection.
-  `test_escape_password_sql_injection_safe` function L583-589 — `()` — (schema names, usernames, etc.) to ensure they cannot be used for SQL injection.

### crates/cloacina/src/dispatcher

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/dispatcher/default.rs

- pub `DefaultDispatcher` struct L52-59 — `{ executors: RwLock<HashMap<String, Arc<dyn TaskExecutor>>>, router: Router, dal...` — Default dispatcher implementation with glob-based routing.
- pub `new` function L63-69 — `(dal: DAL, routing: RoutingConfig) -> Self` — Creates a new DefaultDispatcher with the given DAL and routing configuration.
- pub `with_defaults` function L72-74 — `(dal: DAL) -> Self` — Creates a dispatcher with default routing (all tasks go to "default" executor).
- pub `router` function L77-79 — `(&self) -> &Router` — Gets a reference to the router for inspection.
- pub `dal` function L82-84 — `(&self) -> &DAL` — Gets a reference to the DAL.
-  `DefaultDispatcher` type L61-133 — `= DefaultDispatcher` — configurable glob patterns.
-  `handle_result` function L89-132 — `( &self, event: &TaskReadyEvent, result: super::types::ExecutionResult, ) -> Res...` — Logs the execution result.
-  `DefaultDispatcher` type L136-185 — `impl Dispatcher for DefaultDispatcher` — configurable glob patterns.
-  `dispatch` function L137-165 — `(&self, event: TaskReadyEvent) -> Result<(), DispatchError>` — configurable glob patterns.
-  `register_executor` function L167-175 — `(&self, key: &str, executor: Arc<dyn TaskExecutor>)` — configurable glob patterns.
-  `has_capacity` function L177-180 — `(&self) -> bool` — configurable glob patterns.
-  `resolve_executor_key` function L182-184 — `(&self, task_name: &str) -> String` — configurable glob patterns.
-  `tests` module L188-385 — `-` — configurable glob patterns.
-  `MockExecutor` struct L196-200 — `{ name: String, has_capacity: AtomicBool, execute_count: AtomicUsize }` — Mock executor for testing
-  `MockExecutor` type L202-215 — `= MockExecutor` — configurable glob patterns.
-  `new` function L203-209 — `(name: &str) -> Self` — configurable glob patterns.
-  `execution_count` function L212-214 — `(&self) -> usize` — configurable glob patterns.
-  `MockExecutor` type L218-244 — `impl TaskExecutor for MockExecutor` — configurable glob patterns.
-  `execute` function L219-225 — `(&self, event: TaskReadyEvent) -> Result<ExecutionResult, DispatchError>` — configurable glob patterns.
-  `has_capacity` function L227-229 — `(&self) -> bool` — configurable glob patterns.
-  `metrics` function L231-239 — `(&self) -> ExecutorMetrics` — configurable glob patterns.
-  `name` function L241-243 — `(&self) -> &str` — configurable glob patterns.
-  `create_test_event` function L247-254 — `(task_name: &str) -> TaskReadyEvent` — configurable glob patterns.
-  `test_register_executor` function L257-263 — `()` — configurable glob patterns.
-  `test_resolve_executor_key` function L266-274 — `()` — configurable glob patterns.
-  `test_routing_config_default` function L277-281 — `()` — configurable glob patterns.
-  `test_routing_config_with_multiple_rules` function L284-293 — `()` — configurable glob patterns.
-  `test_mock_executor_has_capacity` function L296-302 — `()` — configurable glob patterns.
-  `test_mock_executor_metrics` function L305-310 — `()` — configurable glob patterns.
-  `test_mock_executor_name` function L313-316 — `()` — configurable glob patterns.
-  `test_mock_executor_execute_increments_count` function L319-330 — `()` — configurable glob patterns.
-  `test_task_ready_event_creation` function L333-337 — `()` — configurable glob patterns.
-  `test_execution_result_success` function L340-346 — `()` — configurable glob patterns.
-  `test_execution_result_failure` function L349-354 — `()` — configurable glob patterns.
-  `test_execution_result_retry` function L357-362 — `()` — configurable glob patterns.
-  `test_executor_metrics_available_capacity` function L365-374 — `()` — configurable glob patterns.
-  `test_executor_metrics_at_capacity` function L377-384 — `()` — configurable glob patterns.

#### crates/cloacina/src/dispatcher/mod.rs

- pub `default` module L58 — `-` — # Dispatcher Layer for Executor Decoupling
- pub `router` module L59 — `-` — ```
- pub `traits` module L60 — `-` — ```
- pub `types` module L61 — `-` — ```
- pub `work_distributor` module L62 — `-` — ```

#### crates/cloacina/src/dispatcher/router.rs

- pub `Router` struct L29-31 — `{ config: RoutingConfig }` — Router for matching tasks to executor keys.
- pub `new` function L35-37 — `(config: RoutingConfig) -> Self` — Creates a new router with the given configuration.
- pub `resolve` function L51-58 — `(&self, task_name: &str) -> &str` — Resolves the executor key for a given task name.
- pub `config` function L199-201 — `(&self) -> &RoutingConfig` — Gets the current routing configuration.
- pub `add_rule` function L204-206 — `(&mut self, rule: RoutingRule)` — Adds a new routing rule.
-  `Router` type L33-207 — `= Router` — based on configurable rules.
-  `matches_pattern` function L76-92 — `(pattern: &str, task_name: &str) -> bool` — Checks if a task name matches a glob pattern.
-  `match_segments` function L95-126 — `(pattern_parts: &[&str], name_parts: &[&str]) -> bool` — Recursively matches pattern segments against name segments.
-  `match_glob` function L129-146 — `(pattern: &str, text: &str) -> bool` — Matches a single segment with glob patterns (* only).
-  `match_wildcard` function L149-189 — `(pattern: &str, text: &str) -> bool` — Matches text against a pattern with * wildcards.
-  `find_substring` function L192-196 — `(haystack: &[u8], needle: &[u8]) -> Option<usize>` — Finds substring position in byte slice.
-  `tests` module L210-283 — `-` — based on configurable rules.
-  `test_exact_match` function L214-220 — `()` — based on configurable rules.
-  `test_wildcard_match` function L223-230 — `()` — based on configurable rules.
-  `test_double_wildcard` function L233-239 — `()` — based on configurable rules.
-  `test_prefix_wildcard` function L242-249 — `()` — based on configurable rules.
-  `test_suffix_wildcard` function L252-259 — `()` — based on configurable rules.
-  `test_rule_order_priority` function L262-271 — `()` — based on configurable rules.
-  `test_namespace_wildcard` function L274-282 — `()` — based on configurable rules.

#### crates/cloacina/src/dispatcher/traits.rs

- pub `Dispatcher` interface L60-98 — `{ fn dispatch(), fn register_executor(), fn has_capacity(), fn resolve_executor_...` — Dispatcher routes task-ready events to appropriate executors.
- pub `TaskExecutor` interface L136-169 — `{ fn execute(), fn has_capacity(), fn metrics(), fn name() }` — Executor receives task-ready events and executes them.

#### crates/cloacina/src/dispatcher/types.rs

- pub `TaskReadyEvent` struct L31-40 — `{ task_execution_id: UniversalUuid, workflow_execution_id: UniversalUuid, task_n...` — Event emitted when a task becomes ready for execution.
- pub `new` function L44-56 — `( task_execution_id: UniversalUuid, workflow_execution_id: UniversalUuid, task_n...` — Creates a new TaskReadyEvent.
- pub `ExecutionStatus` enum L61-70 — `Completed | Failed | Retry | Skipped` — Simplified status for execution results.
- pub `ExecutionResult` struct L77-86 — `{ task_execution_id: UniversalUuid, status: ExecutionStatus, error: Option<Strin...` — Result of task execution from an executor.
- pub `success` function L90-97 — `(task_execution_id: UniversalUuid, duration: Duration) -> Self` — Creates a successful execution result.
- pub `failure` function L100-111 — `( task_execution_id: UniversalUuid, error: impl Into<String>, duration: Duration...` — Creates a failed execution result.
- pub `skipped` function L114-121 — `(task_execution_id: UniversalUuid) -> Self` — Creates a skipped execution result (task claimed by another runner).
- pub `retry` function L124-135 — `( task_execution_id: UniversalUuid, error: impl Into<String>, duration: Duration...` — Creates a retry execution result.
- pub `ExecutorMetrics` struct L140-151 — `{ active_tasks: usize, max_concurrent: usize, total_executed: u64, total_failed:...` — Metrics for monitoring executor performance.
- pub `available_capacity` function L155-157 — `(&self) -> usize` — Returns the current capacity (available slots).
- pub `RoutingConfig` struct L165-170 — `{ default_executor: String, rules: Vec<RoutingRule> }` — Configuration for task routing.
- pub `new` function L183-188 — `(default_executor: impl Into<String>) -> Self` — Creates a new routing configuration with a default executor.
- pub `with_rule` function L191-194 — `(mut self, rule: RoutingRule) -> Self` — Adds a routing rule.
- pub `with_rules` function L197-200 — `(mut self, rules: impl IntoIterator<Item = RoutingRule>) -> Self` — Adds multiple routing rules.
- pub `RoutingRule` struct L208-213 — `{ task_pattern: String, executor: String }` — A routing rule for directing tasks to specific executors.
- pub `new` function L217-222 — `(task_pattern: impl Into<String>, executor: impl Into<String>) -> Self` — Creates a new routing rule.
- pub `DispatchError` enum L227-255 — `ExecutorNotFound | ExecutionFailed | DatabaseError | ContextError | ValidationEr...` — Errors that can occur during dispatch operations.
-  `TaskReadyEvent` type L42-57 — `= TaskReadyEvent` — tasks from the scheduler to executors.
-  `ExecutionResult` type L88-136 — `= ExecutionResult` — tasks from the scheduler to executors.
-  `ExecutorMetrics` type L153-158 — `= ExecutorMetrics` — tasks from the scheduler to executors.
-  `RoutingConfig` type L172-179 — `impl Default for RoutingConfig` — tasks from the scheduler to executors.
-  `default` function L173-178 — `() -> Self` — tasks from the scheduler to executors.
-  `RoutingConfig` type L181-201 — `= RoutingConfig` — tasks from the scheduler to executors.
-  `RoutingRule` type L215-223 — `= RoutingRule` — tasks from the scheduler to executors.

#### crates/cloacina/src/dispatcher/work_distributor.rs

- pub `WorkDistributor` interface L56-71 — `{ fn wait_for_work(), fn shutdown() }` — Trait for abstracting work notification mechanisms.
- pub `PostgresDistributor` struct L85-95 — `{ database_url: String, notify: Arc<Notify>, shutdown: Arc<std::sync::atomic::At...` — PostgreSQL work distributor using LISTEN/NOTIFY.
- pub `new` function L114-129 — `(database_url: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>` — Creates a new PostgreSQL work distributor.
- pub `SqliteDistributor` struct L258-265 — `{ poll_interval: Duration, shutdown: Arc<std::sync::atomic::AtomicBool>, notify:...` — SQLite work distributor using periodic polling.
- pub `new` function L273-275 — `() -> Self` — Creates a new SQLite work distributor with default poll interval (500ms).
- pub `with_poll_interval` function L282-288 — `(poll_interval: Duration) -> Self` — Creates a new SQLite work distributor with custom poll interval.
- pub `create_work_distributor` function L332-347 — `( database: &crate::Database, ) -> Result<Box<dyn WorkDistributor>, Box<dyn std:...` — Creates the appropriate work distributor based on database backend.
-  `PostgresDistributor` type L98-219 — `= PostgresDistributor` — ```
-  `POLL_FALLBACK` variable L100 — `: Duration` — Fallback poll interval when no notifications received
-  `spawn_listener` function L132-218 — `( database_url: String, notify: Arc<Notify>, shutdown: Arc<std::sync::atomic::At...` — Spawns the background listener task.
-  `PostgresDistributor` type L223-241 — `impl WorkDistributor for PostgresDistributor` — ```
-  `wait_for_work` function L224-234 — `(&self)` — ```
-  `shutdown` function L236-240 — `(&self)` — ```
-  `PostgresDistributor` type L244-251 — `impl Drop for PostgresDistributor` — ```
-  `drop` function L245-250 — `(&mut self)` — ```
-  `SqliteDistributor` type L268-289 — `= SqliteDistributor` — ```
-  `DEFAULT_POLL_INTERVAL` variable L270 — `: Duration` — Default poll interval for SQLite
-  `SqliteDistributor` type L292-296 — `impl Default for SqliteDistributor` — ```
-  `default` function L293-295 — `() -> Self` — ```
-  `SqliteDistributor` type L300-321 — `impl WorkDistributor for SqliteDistributor` — ```
-  `wait_for_work` function L301-314 — `(&self)` — ```
-  `shutdown` function L316-320 — `(&self)` — ```
-  `tests` module L350-389 — `-` — ```
-  `test_sqlite_distributor_poll_interval` function L355-365 — `()` — ```
-  `test_sqlite_distributor_shutdown` function L369-388 — `()` — ```

### crates/cloacina/src/execution_planner

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/execution_planner/context_manager.rs

- pub `ContextManager` struct L35-38 — `{ dal: &'a DAL, runtime: Arc<Runtime> }` — Context management operations for the scheduler.
- pub `new` function L42-44 — `(dal: &'a DAL, runtime: Arc<Runtime>) -> Self` — Creates a new ContextManager.
- pub `load_context_for_task` function L47-144 — `( &self, task_execution: &TaskExecution, ) -> Result<Context<serde_json::Value>,...` — Loads the context for a specific task based on its dependencies.
- pub `evaluate_context_condition` function L201-240 — `( context: &Context<serde_json::Value>, key: &str, operator: &ValueOperator, exp...` — Evaluates a context-based condition using the provided operator.
-  `merge_dependency_contexts` function L147-198 — `( &self, task_execution: &TaskExecution, dependencies: &[crate::task::TaskNamesp...` — Merges contexts from multiple dependencies.
-  `tests` module L244-588 — `-` — their dependencies.
-  `ctx_with` function L248-254 — `(pairs: Vec<(&str, serde_json::Value)>) -> Context<serde_json::Value>` — their dependencies.
-  `exists_returns_true_when_key_present` function L259-269 — `()` — their dependencies.
-  `exists_returns_false_when_key_missing` function L272-282 — `()` — their dependencies.
-  `not_exists_returns_true_when_key_missing` function L285-295 — `()` — their dependencies.
-  `not_exists_returns_false_when_key_present` function L298-308 — `()` — their dependencies.
-  `equals_string_match` function L313-323 — `()` — their dependencies.
-  `equals_string_mismatch` function L326-336 — `()` — their dependencies.
-  `equals_number_match` function L339-349 — `()` — their dependencies.
-  `equals_boolean_match` function L352-362 — `()` — their dependencies.
-  `equals_missing_key_returns_false` function L365-375 — `()` — their dependencies.
-  `not_equals_different_values` function L378-388 — `()` — their dependencies.
-  `not_equals_same_values` function L391-401 — `()` — their dependencies.
-  `greater_than_true` function L406-416 — `()` — their dependencies.
-  `greater_than_false_when_equal` function L419-429 — `()` — their dependencies.
-  `greater_than_non_number_returns_false` function L432-442 — `()` — their dependencies.
-  `greater_than_missing_key_returns_false` function L445-455 — `()` — their dependencies.
-  `less_than_true` function L458-468 — `()` — their dependencies.
-  `less_than_float` function L471-481 — `()` — their dependencies.
-  `contains_string_substring` function L486-496 — `()` — their dependencies.
-  `contains_string_not_found` function L499-509 — `()` — their dependencies.
-  `contains_array_element` function L512-522 — `()` — their dependencies.
-  `contains_array_element_missing` function L525-535 — `()` — their dependencies.
-  `contains_non_string_non_array_returns_false` function L538-548 — `()` — their dependencies.
-  `not_contains_string` function L551-561 — `()` — their dependencies.
-  `not_contains_array` function L564-574 — `()` — their dependencies.
-  `not_contains_when_present` function L577-587 — `()` — their dependencies.

#### crates/cloacina/src/execution_planner/mod.rs

- pub `stale_claim_sweeper` module L119 — `-` — ```
- pub `TaskScheduler` struct L187-196 — `{ dal: DAL, runtime: Arc<Runtime>, instance_id: Uuid, poll_interval: Duration, d...` — The main Task Scheduler that manages workflow execution and task readiness.
- pub `new` function L226-229 — `(database: Database) -> Result<Self, ValidationError>` — Creates a new TaskScheduler instance with default configuration using global workflow registry.
- pub `with_poll_interval` function L247-255 — `( database: Database, poll_interval: Duration, ) -> Result<Self, ValidationError...` — Creates a new TaskScheduler with custom poll interval using global workflow registry.
- pub `with_runtime` function L272-275 — `(mut self, runtime: Arc<Runtime>) -> Self` — Sets the runtime for this scheduler, replacing the default.
- pub `runtime` function L278-280 — `(&self) -> &Arc<Runtime>` — Returns a reference to the runtime used by this scheduler.
- pub `with_shutdown` function L283-286 — `(mut self, shutdown_rx: tokio::sync::watch::Receiver<bool>) -> Self` — Sets the shutdown receiver for graceful termination of the scheduling loop.
- pub `with_dispatcher` function L300-303 — `(mut self, dispatcher: Arc<dyn Dispatcher>) -> Self` — Sets the dispatcher for push-based task execution.
- pub `dispatcher` function L306-308 — `(&self) -> Option<&Arc<dyn Dispatcher>>` — Returns a reference to the dispatcher if configured.
- pub `schedule_workflow_execution` function L353-438 — `( &self, workflow_name: &str, input_context: Context<serde_json::Value>, ) -> Re...` — Schedules a new workflow execution with the provided input context.
- pub `run_scheduling_loop` function L600-612 — `(&self) -> Result<(), ValidationError>` — Runs the main scheduling loop that continuously processes active workflow executions.
- pub `process_active_executions` function L615-624 — `(&self) -> Result<(), ValidationError>` — Processes all active workflow executions to update task readiness.
-  `context_manager` module L116 — `-` — # Task Scheduler
-  `recovery` module L117 — `-` — ```
-  `scheduler_loop` module L118 — `-` — ```
-  `state_manager` module L120 — `-` — ```
-  `trigger_rules` module L121 — `-` — ```
-  `TaskScheduler` type L198-647 — `= TaskScheduler` — ```
-  `with_poll_interval_sync` function L258-269 — `(database: Database, poll_interval: Duration) -> Self` — Creates a new TaskScheduler with custom poll interval (synchronous version).
-  `create_workflow_execution_postgres` function L442-499 — `( &self, workflow_execution_id: UniversalUuid, now: UniversalTimestamp, workflow...` — Creates workflow execution and tasks in PostgreSQL.
-  `create_workflow_execution_sqlite` function L503-560 — `( &self, workflow_execution_id: UniversalUuid, now: UniversalTimestamp, workflow...` — Creates workflow execution and tasks in SQLite.
-  `get_task_trigger_rules` function L627-636 — `( &self, workflow: &Workflow, task_namespace: &TaskNamespace, ) -> serde_json::V...` — Gets trigger rules for a specific task from the task implementation.
-  `get_task_configuration` function L639-646 — `( &self, _workflow: &Workflow, _task_namespace: &TaskNamespace, ) -> serde_json:...` — Gets task configuration (currently returns empty object).

#### crates/cloacina/src/execution_planner/recovery.rs

- pub `RecoveryResult` enum L35-40 — `Recovered | Abandoned` — Result of attempting to recover a task.
- pub `RecoveryManager` struct L46-49 — `{ dal: &'a DAL, runtime: Arc<Runtime> }` — Recovery operations for the scheduler.
- pub `new` function L53-55 — `(dal: &'a DAL, runtime: Arc<Runtime>) -> Self` — Creates a new RecoveryManager.
- pub `recover_orphaned_tasks` function L67-173 — `(&self) -> Result<(), ValidationError>` — Detects and recovers tasks orphaned by system interruptions.
-  `MAX_RECOVERY_ATTEMPTS` variable L43 — `: i32` — Maximum number of recovery attempts before abandoning a task.
-  `recover_tasks_for_known_workflow` function L176-203 — `( &self, tasks: Vec<TaskExecution>, ) -> Result<usize, ValidationError>` — Recovers tasks from workflows that are still available in the registry.
-  `abandon_tasks_for_unknown_workflow` function L206-286 — `( &self, workflow_exec: WorkflowExecutionRecord, tasks: Vec<TaskExecution>, avai...` — Abandons tasks from workflows that are no longer available in the registry.
-  `recover_single_task` function L289-329 — `( &self, task: TaskExecution, ) -> Result<RecoveryResult, ValidationError>` — Recovers a single orphaned task with retry limit enforcement.
-  `abandon_task_permanently` function L332-378 — `(&self, task: TaskExecution) -> Result<(), ValidationError>` — Permanently abandons a task that has exceeded recovery limits.
-  `record_recovery_event` function L381-384 — `(&self, event: NewRecoveryEvent) -> Result<(), ValidationError>` — Records a recovery event for monitoring and debugging.

#### crates/cloacina/src/execution_planner/scheduler_loop.rs

- pub `SchedulerLoop` struct L47-58 — `{ dal: &'a DAL, runtime: Arc<Runtime>, instance_id: Uuid, poll_interval: Duratio...` — Scheduler loop operations.
- pub `new` function L63-78 — `( dal: &'a DAL, runtime: Arc<Runtime>, instance_id: Uuid, poll_interval: Duratio...` — Creates a new SchedulerLoop.
- pub `with_dispatcher` function L81-97 — `( dal: &'a DAL, runtime: Arc<Runtime>, instance_id: Uuid, poll_interval: Duratio...` — Creates a new SchedulerLoop with an optional dispatcher.
- pub `with_shutdown` function L100-103 — `(mut self, shutdown_rx: tokio::sync::watch::Receiver<bool>) -> Self` — Set the shutdown receiver for graceful termination.
- pub `run` function L112-175 — `(&mut self) -> Result<(), ValidationError>` — Runs the main scheduling loop that continuously processes active workflow executions.
- pub `process_active_executions` function L178-202 — `(&self) -> Result<(), ValidationError>` — Processes all active workflow executions to update task readiness.
-  `MAX_BACKOFF` variable L41 — `: Duration` — Maximum backoff interval during sustained errors (30 seconds).
-  `CIRCUIT_OPEN_THRESHOLD` variable L44 — `: u32` — Number of consecutive errors before logging a circuit-open warning.
-  `process_executions_batch` function L210-261 — `( &self, active_executions: Vec<WorkflowExecutionRecord>, ) -> Result<(), Valida...` — Processes multiple workflow executions in batch for better performance.
-  `dispatch_ready_tasks` function L268-296 — `(&self) -> Result<(), ValidationError>` — Dispatches all Ready tasks to the executor.
-  `complete_execution` function L303-379 — `( &self, execution: &WorkflowExecutionRecord, ) -> Result<(), ValidationError>` — Completes a workflow execution by updating its final context and marking it as completed.
-  `update_execution_final_context` function L386-443 — `( &self, workflow_execution_id: UniversalUuid, all_tasks: &[TaskExecution], ) ->...` — Updates the workflow execution's final context when it completes.

#### crates/cloacina/src/execution_planner/stale_claim_sweeper.rs

- pub `StaleClaimSweeperConfig` struct L40-46 — `{ sweep_interval: Duration, stale_threshold: Duration }` — Configuration for the stale claim sweeper.
- pub `StaleClaimSweeper` struct L58-64 — `{ dal: Arc<DAL>, config: StaleClaimSweeperConfig, shutdown_rx: watch::Receiver<b...` — Background service that sweeps for stale task claims.
- pub `new` function L68-79 — `( dal: Arc<DAL>, config: StaleClaimSweeperConfig, shutdown_rx: watch::Receiver<b...` — Create a new stale claim sweeper.
- pub `run` function L82-106 — `(&mut self)` — Run the sweep loop.
- pub `sweep` function L109-187 — `(&self)` — Perform a single sweep pass.
-  `StaleClaimSweeperConfig` type L48-55 — `impl Default for StaleClaimSweeperConfig` — because the sweeper wasn't running to observe their heartbeats.
-  `default` function L49-54 — `() -> Self` — because the sweeper wasn't running to observe their heartbeats.
-  `StaleClaimSweeper` type L66-188 — `= StaleClaimSweeper` — because the sweeper wasn't running to observe their heartbeats.
-  `tests` module L191-218 — `-` — because the sweeper wasn't running to observe their heartbeats.
-  `config_defaults` function L195-199 — `()` — because the sweeper wasn't running to observe their heartbeats.
-  `config_custom_values` function L202-209 — `()` — because the sweeper wasn't running to observe their heartbeats.
-  `config_clone` function L212-217 — `()` — because the sweeper wasn't running to observe their heartbeats.

#### crates/cloacina/src/execution_planner/state_manager.rs

- pub `StateManager` struct L37-40 — `{ dal: &'a DAL, runtime: Arc<Runtime> }` — State management operations for the scheduler.
- pub `new` function L44-46 — `(dal: &'a DAL, runtime: Arc<Runtime>) -> Self` — Creates a new StateManager.
- pub `update_workflow_task_readiness` function L53-86 — `( &self, workflow_execution_id: UniversalUuid, pending_tasks: &[TaskExecution], ...` — Updates task readiness for a specific workflow execution using pre-loaded tasks.
- pub `check_task_dependencies` function L91-145 — `( &self, task_execution: &TaskExecution, ) -> Result<bool, ValidationError>` — Checks if all dependencies for a task are satisfied.
- pub `evaluate_trigger_rules` function L148-242 — `( &self, task_execution: &TaskExecution, ) -> Result<bool, ValidationError>` — Evaluates trigger rules for a task based on its configuration.
-  `evaluate_condition` function L245-321 — `( &self, condition: &TriggerCondition, task_execution: &TaskExecution, ) -> Resu...` — Evaluates a specific trigger condition.

#### crates/cloacina/src/execution_planner/trigger_rules.rs

- pub `TriggerRule` enum L86-95 — `Always | All | Any | None` — Trigger rule definitions for conditional task execution.
- pub `TriggerCondition` enum L143-156 — `TaskSuccess | TaskFailed | TaskSkipped | ContextValue` — Individual conditions that can be evaluated for trigger rules.
- pub `ValueOperator` enum L199-216 — `Equals | NotEquals | GreaterThan | LessThan | Contains | NotContains | Exists | ...` — Operators for evaluating context values in trigger conditions.
-  `tests` module L219-417 — `-` — when tasks should be executed based on various conditions.
-  `trigger_rule_always_roundtrip` function L226-231 — `()` — when tasks should be executed based on various conditions.
-  `trigger_rule_all_roundtrip` function L234-253 — `()` — when tasks should be executed based on various conditions.
-  `trigger_rule_any_roundtrip` function L256-268 — `()` — when tasks should be executed based on various conditions.
-  `trigger_rule_none_roundtrip` function L271-283 — `()` — when tasks should be executed based on various conditions.
-  `trigger_rule_all_empty_conditions` function L286-294 — `()` — when tasks should be executed based on various conditions.
-  `condition_task_success_roundtrip` function L299-310 — `()` — when tasks should be executed based on various conditions.
-  `condition_task_failed_roundtrip` function L313-323 — `()` — when tasks should be executed based on various conditions.
-  `condition_task_skipped_roundtrip` function L326-336 — `()` — when tasks should be executed based on various conditions.
-  `condition_context_value_roundtrip` function L339-359 — `()` — when tasks should be executed based on various conditions.
-  `all_value_operators_roundtrip` function L364-381 — `()` — when tasks should be executed based on various conditions.
-  `trigger_rule_from_json_literal` function L386-390 — `()` — when tasks should be executed based on various conditions.
-  `trigger_rule_all_from_json_literal` function L393-416 — `()` — when tasks should be executed based on various conditions.

### crates/cloacina/src/executor

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/executor/mod.rs

- pub `slot_token` module L47 — `-` — # Task Executor
- pub `task_handle` module L48 — `-` — All components are thread-safe and can be used in concurrent environments.
- pub `thread_task_executor` module L49 — `-` — All components are thread-safe and can be used in concurrent environments.
- pub `types` module L50 — `-` — All components are thread-safe and can be used in concurrent environments.
- pub `workflow_executor` module L51 — `-` — All components are thread-safe and can be used in concurrent environments.

#### crates/cloacina/src/executor/slot_token.rs

- pub `SlotToken` struct L42-45 — `{ permit: Option<OwnedSemaphorePermit>, semaphore: Arc<Semaphore> }` — A token representing a held concurrency slot in the executor.
- pub `release` function L63-65 — `(&mut self) -> bool` — Release the concurrency slot, freeing it for other tasks.
- pub `reclaim` function L75-91 — `(&mut self) -> Result<(), ExecutorError>` — Reclaim a concurrency slot after it was released.
- pub `is_held` function L94-96 — `(&self) -> bool` — Returns whether the token currently holds a concurrency slot.
-  `SlotToken` type L47-97 — `= SlotToken` — extensions like weighted slots or cross-executor management.
-  `new` function L49-54 — `(permit: OwnedSemaphorePermit, semaphore: Arc<Semaphore>) -> Self` — Creates a new SlotToken from an already-acquired permit.
-  `tests` module L100-192 — `-` — extensions like weighted slots or cross-executor management.
-  `test_slot_token_release_frees_permit` function L104-119 — `()` — extensions like weighted slots or cross-executor management.
-  `test_slot_token_reclaim_reacquires_permit` function L122-133 — `()` — extensions like weighted slots or cross-executor management.
-  `test_slot_token_reclaim_when_already_held_is_noop` function L136-145 — `()` — extensions like weighted slots or cross-executor management.
-  `test_slot_token_drop_releases_permit` function L148-158 — `()` — extensions like weighted slots or cross-executor management.
-  `test_slot_token_reclaim_waits_for_availability` function L161-191 — `()` — extensions like weighted slots or cross-executor management.

#### crates/cloacina/src/executor/task_handle.rs

- pub `take_task_handle` function L67-73 — `() -> TaskHandle` — Takes the current task's `TaskHandle` out of task-local storage.
- pub `return_task_handle` function L79-83 — `(handle: TaskHandle)` — Returns a `TaskHandle` to task-local storage after the user function completes.
- pub `with_task_handle` function L89-100 — `(handle: TaskHandle, f: F) -> (T, Option<TaskHandle>)` — Runs an async future with a `TaskHandle` available in task-local storage.
- pub `TaskHandle` struct L110-114 — `{ slot_token: SlotToken, task_execution_id: UniversalUuid, dal: Option<DAL> }` — Execution control handle passed to tasks that need concurrency management.
- pub `defer_until` function L163-228 — `( &mut self, condition: F, poll_interval: Duration, ) -> Result<(), ExecutorErro...` — Release the concurrency slot while polling an external condition.
- pub `task_execution_id` function L231-233 — `(&self) -> UniversalUuid` — Returns the task execution ID associated with this handle.
- pub `is_slot_held` function L236-238 — `(&self) -> bool` — Returns whether the handle currently holds a concurrency slot.
-  `TaskHandle` type L116-248 — `= TaskHandle` — ```
-  `new` function L121-127 — `(slot_token: SlotToken, task_execution_id: UniversalUuid) -> Self` — Creates a new TaskHandle.
-  `with_dal` function L130-140 — `( slot_token: SlotToken, task_execution_id: UniversalUuid, dal: DAL, ) -> Self` — Creates a new TaskHandle with DAL for sub_status persistence.
-  `into_slot_token` function L245-247 — `(self) -> SlotToken` — Consumes the handle, returning the inner SlotToken.
-  `tests` module L251-412 — `-` — ```
-  `make_handle` function L257-264 — `(semaphore: &Arc<Semaphore>) -> TaskHandle` — ```
-  `test_defer_until_releases_and_reclaims_slot` function L267-295 — `()` — ```
-  `test_defer_until_immediate_condition` function L298-309 — `()` — ```
-  `test_defer_until_frees_slot_for_other_tasks` function L312-343 — `()` — ```
-  `test_task_local_round_trip` function L346-368 — `()` — ```
-  `test_task_local_not_returned_yields_none` function L371-386 — `()` — ```
-  `test_with_task_handle_preserves_handle_through_defer` function L389-411 — `()` — ```

#### crates/cloacina/src/executor/thread_task_executor.rs

- pub `ThreadTaskExecutor` struct L71-90 — `{ database: Database, dal: DAL, task_registry: Arc<TaskRegistry>, runtime: Arc<R...` — ThreadTaskExecutor is a thread-based implementation of task execution.
- pub `new` function L102-108 — `( database: Database, task_registry: Arc<TaskRegistry>, config: ExecutorConfig, ...` — Creates a new ThreadTaskExecutor instance.
- pub `with_runtime_and_registry` function L111-131 — `( database: Database, task_registry: Arc<TaskRegistry>, runtime: Arc<Runtime>, c...` — Creates a new ThreadTaskExecutor with a specific runtime.
- pub `with_runtime` function L134-137 — `(mut self, runtime: Arc<Runtime>) -> Self` — Sets the runtime for this executor, replacing the default.
- pub `with_global_registry` function L150-164 — `( database: Database, config: ExecutorConfig, ) -> Result<Self, crate::error::Re...` — Creates a TaskExecutor using the global task registry.
- pub `semaphore` function L170-172 — `(&self) -> &Arc<Semaphore>` — Returns a reference to the concurrency semaphore.
-  `ThreadTaskExecutor` type L92-706 — `= ThreadTaskExecutor` — to the executor based on routing rules.
-  `build_task_context` function L182-307 — `( &self, claimed_task: &ClaimedTask, dependencies: &[crate::task::TaskNamespace]...` — Builds the execution context for a task by loading its dependencies.
-  `merge_context_values` function L321-356 — `( existing: &serde_json::Value, new: &serde_json::Value, ) -> serde_json::Value` — Merges two context values using smart merging strategy.
-  `execute_with_timeout` function L366-375 — `( &self, task: &dyn Task, context: Context<serde_json::Value>, ) -> Result<Conte...` — Executes a task with timeout protection.
-  `handle_task_result` function L392-441 — `( &self, claimed_task: ClaimedTask, result: Result<Context<serde_json::Value>, E...` — Handles the result of task execution.
-  `save_task_context` function L451-481 — `( &self, claimed_task: &ClaimedTask, context: Context<serde_json::Value>, ) -> R...` — Saves the task's execution context to the database.
-  `complete_task_transaction` function L494-541 — `( &self, claimed_task: &ClaimedTask, context: Context<serde_json::Value>, ) -> R...` — Marks a task as completed in the database.
-  `mark_task_failed` function L552-589 — `( &self, task_execution_id: UniversalUuid, error: &ExecutorError, ) -> Result<()...` — Marks a task as failed in the database.
-  `should_retry_task` function L605-642 — `( &self, claimed_task: &ClaimedTask, error: &ExecutorError, retry_policy: &Retry...` — Determines if a failed task should be retried.
-  `is_transient_error` function L651-668 — `(&self, error: &ExecutorError) -> bool` — Determines if an error is transient and potentially retryable.
-  `schedule_task_retry` function L678-705 — `( &self, claimed_task: &ClaimedTask, retry_policy: &RetryPolicy, ) -> Result<(),...` — Schedules a task for retry execution.
-  `ThreadTaskExecutor` type L708-723 — `impl Clone for ThreadTaskExecutor` — to the executor based on routing rules.
-  `clone` function L709-722 — `(&self) -> Self` — to the executor based on routing rules.
-  `ThreadTaskExecutor` type L730-1054 — `impl TaskExecutor for ThreadTaskExecutor` — Implementation of the dispatcher's TaskExecutor trait.
-  `execute` function L731-1033 — `(&self, event: TaskReadyEvent) -> Result<ExecutionResult, DispatchError>` — to the executor based on routing rules.
-  `has_capacity` function L1035-1037 — `(&self) -> bool` — to the executor based on routing rules.
-  `metrics` function L1039-1049 — `(&self) -> ExecutorMetrics` — to the executor based on routing rules.
-  `name` function L1051-1053 — `(&self) -> &str` — to the executor based on routing rules.
-  `tests` module L1057-1341 — `-` — to the executor based on routing rules.
-  `test_merge_primitives_latest_wins` function L1066-1071 — `()` — to the executor based on routing rules.
-  `test_merge_string_latest_wins` function L1074-1079 — `()` — to the executor based on routing rules.
-  `test_merge_different_types_latest_wins` function L1082-1087 — `()` — to the executor based on routing rules.
-  `test_merge_arrays_deduplicates` function L1090-1095 — `()` — to the executor based on routing rules.
-  `test_merge_arrays_no_overlap` function L1098-1103 — `()` — to the executor based on routing rules.
-  `test_merge_arrays_complete_overlap` function L1106-1111 — `()` — to the executor based on routing rules.
-  `test_merge_objects_no_conflict` function L1114-1119 — `()` — to the executor based on routing rules.
-  `test_merge_objects_conflicting_keys` function L1122-1127 — `()` — to the executor based on routing rules.
-  `test_merge_objects_recursive` function L1130-1135 — `()` — to the executor based on routing rules.
-  `test_merge_nested_arrays_in_objects` function L1138-1143 — `()` — to the executor based on routing rules.
-  `test_merge_null_latest_wins` function L1146-1151 — `()` — to the executor based on routing rules.
-  `test_merge_bool_latest_wins` function L1154-1159 — `()` — to the executor based on routing rules.
-  `sqlite_tests` module L1165-1294 — `-` — to the executor based on routing rules.
-  `test_executor` function L1168-1173 — `() -> ThreadTaskExecutor` — to the executor based on routing rules.
-  `test_is_transient_timeout` function L1176-1179 — `()` — to the executor based on routing rules.
-  `test_is_transient_task_not_found` function L1182-1185 — `()` — to the executor based on routing rules.
-  `test_is_transient_connection_pool` function L1188-1192 — `()` — to the executor based on routing rules.
-  `test_is_transient_task_execution_with_timeout_msg` function L1195-1204 — `()` — to the executor based on routing rules.
-  `test_is_transient_task_execution_permanent` function L1207-1216 — `()` — to the executor based on routing rules.
-  `test_is_transient_task_execution_network` function L1219-1228 — `()` — to the executor based on routing rules.
-  `test_is_transient_task_execution_unavailable` function L1231-1240 — `()` — to the executor based on routing rules.
-  `test_executor_has_capacity_initially` function L1247-1250 — `()` — to the executor based on routing rules.
-  `test_executor_metrics_initial` function L1253-1260 — `()` — to the executor based on routing rules.
-  `test_executor_name` function L1263-1266 — `()` — to the executor based on routing rules.
-  `test_executor_clone_shares_semaphore` function L1269-1277 — `()` — to the executor based on routing rules.
-  `test_executor_custom_config` function L1280-1293 — `()` — to the executor based on routing rules.
-  `test_new_uses_empty_runtime_not_from_global` function L1302-1315 — `()` — to the executor based on routing rules.
-  `test_with_runtime_and_registry_uses_provided_runtime` function L1319-1340 — `()` — to the executor based on routing rules.

#### crates/cloacina/src/executor/types.rs

- pub `ExecutionScope` struct L37-44 — `{ workflow_execution_id: UniversalUuid, task_execution_id: Option<UniversalUuid>...` — Execution scope information for a context
- pub `DependencyLoader` struct L52-61 — `{ database: Database, workflow_execution_id: UniversalUuid, dependency_tasks: Ve...` — Dependency loader for automatic context merging with lazy loading
- pub `new` function L70-81 — `( database: Database, workflow_execution_id: UniversalUuid, dependency_tasks: Ve...` — Creates a new dependency loader instance
- pub `load_from_dependencies` function L93-130 — `( &self, key: &str, ) -> Result<Option<serde_json::Value>, ExecutorError>` — Loads a value from dependency contexts using a "latest wins" strategy
- pub `ExecutorConfig` struct L164-174 — `{ max_concurrent_tasks: usize, task_timeout: std::time::Duration, enable_claimin...` — Configuration settings for the executor
- pub `ClaimedTask` struct L199-208 — `{ task_execution_id: UniversalUuid, workflow_execution_id: UniversalUuid, task_n...` — Represents a task that has been claimed for execution
-  `DependencyLoader` type L63-157 — `= DependencyLoader` — and configure the behavior of the execution engine.
-  `load_dependency_context_data` function L139-156 — `( &self, task_namespace: &crate::task::TaskNamespace, ) -> Result<HashMap<String...` — Loads the context data for a specific dependency task
-  `ExecutorConfig` type L176-192 — `impl Default for ExecutorConfig` — and configure the behavior of the execution engine.
-  `default` function L184-191 — `() -> Self` — Creates a new executor configuration with default values
-  `tests` module L211-379 — `-` — and configure the behavior of the execution engine.
-  `test_execution_scope_full` function L219-230 — `()` — and configure the behavior of the execution engine.
-  `test_execution_scope_minimal` function L233-242 — `()` — and configure the behavior of the execution engine.
-  `test_execution_scope_clone` function L245-255 — `()` — and configure the behavior of the execution engine.
-  `test_execution_scope_debug` function L258-267 — `()` — and configure the behavior of the execution engine.
-  `test_executor_config_default` function L274-283 — `()` — and configure the behavior of the execution engine.
-  `test_executor_config_custom` function L286-297 — `()` — and configure the behavior of the execution engine.
-  `test_executor_config_clone` function L300-312 — `()` — and configure the behavior of the execution engine.
-  `test_executor_config_debug` function L315-321 — `()` — and configure the behavior of the execution engine.
-  `test_claimed_task_construction` function L328-341 — `()` — and configure the behavior of the execution engine.
-  `test_claimed_task_retry_attempt` function L344-352 — `()` — and configure the behavior of the execution engine.
-  `test_claimed_task_debug` function L355-365 — `()` — and configure the behavior of the execution engine.
-  `test_dependency_loader_debug` function L372-378 — `()` — and configure the behavior of the execution engine.
-  `assert_send_sync` function L376 — `()` — and configure the behavior of the execution engine.

#### crates/cloacina/src/executor/workflow_executor.rs

- pub `StatusCallback` interface L59-66 — `{ fn on_status_change() }` — Callback trait for receiving real-time status updates during workflow execution.
- pub `TaskResult` struct L73-88 — `{ task_name: String, status: TaskState, start_time: Option<DateTime<Utc>>, end_t...` — Represents the outcome of a single task execution within a workflow.
- pub `WorkflowExecutionError` enum L96-120 — `DatabaseConnection | WorkflowNotFound | ExecutionFailed | Timeout | Validation |...` — Unified error type for workflow execution operations.
- pub `WorkflowStatus` enum L128-141 — `Pending | Running | Completed | Failed | Cancelled | Paused` — Represents the current state of a workflow execution.
- pub `is_terminal` function L151-156 — `(&self) -> bool` — Determines if this status represents a terminal state.
- pub `WorkflowExecutionResult` struct L164-183 — `{ execution_id: Uuid, workflow_name: String, status: WorkflowStatus, start_time:...` — Contains the complete result of a workflow execution.
- pub `WorkflowExecution` struct L189-195 — `{ execution_id: Uuid, workflow_name: String, executor: crate::runner::DefaultRun...` — Handle for managing an asynchronous workflow execution.
- pub `new` function L205-215 — `( execution_id: Uuid, workflow_name: String, executor: crate::runner::DefaultRun...` — Creates a new workflow execution handle.
- pub `wait_for_completion` function L225-229 — `( self, ) -> Result<WorkflowExecutionResult, WorkflowExecutionError>` — Waits for the workflow to complete execution.
- pub `wait_for_completion_with_timeout` function L241-271 — `( self, timeout: Option<Duration>, ) -> Result<WorkflowExecutionResult, Workflow...` — Waits for completion with a specified timeout.
- pub `get_status` function L279-281 — `(&self) -> Result<WorkflowStatus, WorkflowExecutionError>` — Gets the current status of the workflow execution.
- pub `cancel` function L291-293 — `(&self) -> Result<(), WorkflowExecutionError>` — Cancels the workflow execution.
- pub `pause` function L308-312 — `(&self, reason: Option<&str>) -> Result<(), WorkflowExecutionError>` — Pauses the workflow execution.
- pub `resume` function L323-325 — `(&self) -> Result<(), WorkflowExecutionError>` — Resumes a paused workflow execution.
- pub `WorkflowExecutor` interface L334-487 — `{ fn execute(), fn execute_async(), fn get_execution_status(), fn get_execution_...` — Core trait defining the interface for workflow execution engines.
-  `WorkflowStatus` type L143-157 — `= WorkflowStatus` — ```
-  `WorkflowExecution` type L197-326 — `= WorkflowExecution` — ```
-  `WorkflowStatus` type L489-522 — `= WorkflowStatus` — ```
-  `from_str` function L511-521 — `(s: &str) -> Self` — Creates a WorkflowStatus from a string representation.
-  `tests` module L525-777 — `-` — ```
-  `test_workflow_status_is_terminal` function L534-538 — `()` — ```
-  `test_workflow_status_is_not_terminal` function L541-545 — `()` — ```
-  `test_workflow_status_from_str_valid` function L548-561 — `()` — ```
-  `test_workflow_status_from_str_invalid_defaults_to_failed` function L564-569 — `()` — ```
-  `test_workflow_status_eq` function L572-575 — `()` — ```
-  `test_workflow_status_clone` function L578-582 — `()` — ```
-  `test_workflow_status_debug` function L585-588 — `()` — ```
-  `test_workflow_error_display_database_connection` function L595-603 — `()` — ```
-  `test_workflow_error_display_workflow_not_found` function L606-611 — `()` — ```
-  `test_workflow_error_display_execution_failed` function L614-622 — `()` — ```
-  `test_workflow_error_display_timeout` function L625-630 — `()` — ```
-  `test_workflow_error_display_configuration` function L633-638 — `()` — ```
-  `test_task_result_construction` function L645-661 — `()` — ```
-  `test_task_result_with_error` function L664-679 — `()` — ```
-  `test_task_result_clone` function L682-694 — `()` — ```
-  `test_workflow_result_construction` function L701-717 — `()` — ```
-  `test_workflow_result_with_tasks` function L720-758 — `()` — ```
-  `test_workflow_result_debug` function L761-776 — `()` — ```

### crates/cloacina/src/models

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/models/context.rs

- pub `DbContext` struct L31-36 — `{ id: UniversalUuid, value: String, created_at: UniversalTimestamp, updated_at: ...` — Represents a context record (domain type).
- pub `NewDbContext` struct L40-42 — `{ value: String }` — Structure for creating new context records (domain type).
-  `tests` module L45-72 — `-` — models handle actual database interaction.
-  `test_db_context_creation` function L50-62 — `()` — models handle actual database interaction.
-  `test_new_db_context_creation` function L65-71 — `()` — models handle actual database interaction.

#### crates/cloacina/src/models/execution_event.rs

- pub `ExecutionEvent` struct L34-51 — `{ id: UniversalUuid, workflow_execution_id: UniversalUuid, task_execution_id: Op...` — Represents an execution event record (domain type).
- pub `NewExecutionEvent` struct L55-66 — `{ workflow_execution_id: UniversalUuid, task_execution_id: Option<UniversalUuid>...` — Structure for creating new execution event records (domain type).
- pub `workflow_event` function L70-83 — `( workflow_execution_id: UniversalUuid, event_type: ExecutionEventType, event_da...` — Creates a new execution event for a workflow-level transition.
- pub `task_event` function L86-100 — `( workflow_execution_id: UniversalUuid, task_execution_id: UniversalUuid, event_...` — Creates a new execution event for a task-level transition.
- pub `ExecutionEventType` enum L108-146 — `TaskCreated | TaskMarkedReady | TaskClaimed | TaskStarted | TaskDeferred | TaskR...` — Enumeration of execution event types in the system.
- pub `as_str` function L150-172 — `(&self) -> &'static str` — Returns the string representation of the event type.
- pub `from_str` function L176-199 — `(s: &str) -> Option<Self>` — Parses an event type from its string representation.
- pub `is_task_event` function L202-218 — `(&self) -> bool` — Returns true if this is a task-level event.
- pub `is_workflow_event` function L221-230 — `(&self) -> bool` — Returns true if this is a workflow-level event.
-  `NewExecutionEvent` type L68-101 — `= NewExecutionEvent` — These are API-level types; backend-specific models handle database storage.
-  `ExecutionEventType` type L148-231 — `= ExecutionEventType` — These are API-level types; backend-specific models handle database storage.
-  `String` type L233-237 — `= String` — These are API-level types; backend-specific models handle database storage.
-  `from` function L234-236 — `(event_type: ExecutionEventType) -> Self` — These are API-level types; backend-specific models handle database storage.
-  `ExecutionEventType` type L239-243 — `= ExecutionEventType` — These are API-level types; backend-specific models handle database storage.
-  `fmt` function L240-242 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — These are API-level types; backend-specific models handle database storage.

#### crates/cloacina/src/models/key_trust_acl.rs

- pub `KeyTrustAcl` struct L31-40 — `{ id: UniversalUuid, parent_org_id: UniversalUuid, child_org_id: UniversalUuid, ...` — Domain model for a key trust ACL (Access Control List).
- pub `is_active` function L44-46 — `(&self) -> bool` — Check if this trust relationship is currently active
- pub `is_revoked` function L49-51 — `(&self) -> bool` — Check if this trust relationship has been revoked
- pub `NewKeyTrustAcl` struct L56-59 — `{ parent_org_id: UniversalUuid, child_org_id: UniversalUuid }` — Model for creating a new key trust ACL.
- pub `new` function L62-67 — `(parent_org_id: UniversalUuid, child_org_id: UniversalUuid) -> Self` — trusts packages signed by the child org's trusted keys.
-  `KeyTrustAcl` type L42-52 — `= KeyTrustAcl` — trusts packages signed by the child org's trusted keys.
-  `NewKeyTrustAcl` type L61-68 — `= NewKeyTrustAcl` — trusts packages signed by the child org's trusted keys.

#### crates/cloacina/src/models/mod.rs

- pub `context` module L71 — `-` — - Keep model definitions in sync with database schema migrations
- pub `execution_event` module L72 — `-` — - Keep model definitions in sync with database schema migrations
- pub `recovery_event` module L73 — `-` — - Keep model definitions in sync with database schema migrations
- pub `schedule` module L74 — `-` — - Keep model definitions in sync with database schema migrations
- pub `task_execution` module L75 — `-` — - Keep model definitions in sync with database schema migrations
- pub `task_execution_metadata` module L76 — `-` — - Keep model definitions in sync with database schema migrations
- pub `task_outbox` module L77 — `-` — - Keep model definitions in sync with database schema migrations
- pub `workflow_execution` module L78 — `-` — - Keep model definitions in sync with database schema migrations
- pub `workflow_packages` module L79 — `-` — - Keep model definitions in sync with database schema migrations
- pub `workflow_registry` module L80 — `-` — - Keep model definitions in sync with database schema migrations
- pub `key_trust_acl` module L83 — `-` — - Keep model definitions in sync with database schema migrations
- pub `package_signature` module L84 — `-` — - Keep model definitions in sync with database schema migrations
- pub `signing_key` module L85 — `-` — - Keep model definitions in sync with database schema migrations
- pub `trusted_key` module L86 — `-` — - Keep model definitions in sync with database schema migrations

#### crates/cloacina/src/models/package_signature.rs

- pub `PackageSignature` struct L28-37 — `{ id: UniversalUuid, package_hash: String, key_fingerprint: String, signature: V...` — Domain model for a package signature.
- pub `NewPackageSignature` struct L41-45 — `{ package_hash: String, key_fingerprint: String, signature: Vec<u8> }` — Model for creating a new package signature.
- pub `new` function L48-54 — `(package_hash: String, key_fingerprint: String, signature: Vec<u8>) -> Self` — the SHA256 hash of the package binary.
- pub `SignatureVerification` struct L59-68 — `{ is_valid: bool, signer_fingerprint: String, signed_at: UniversalTimestamp, sig...` — Result of signature verification.
-  `NewPackageSignature` type L47-55 — `= NewPackageSignature` — the SHA256 hash of the package binary.

#### crates/cloacina/src/models/recovery_event.rs

- pub `RecoveryEvent` struct L27-36 — `{ id: UniversalUuid, workflow_execution_id: UniversalUuid, task_execution_id: Op...` — Represents a recovery event record (domain type).
- pub `NewRecoveryEvent` struct L40-45 — `{ workflow_execution_id: UniversalUuid, task_execution_id: Option<UniversalUuid>...` — Structure for creating new recovery event records (domain type).
- pub `RecoveryType` enum L49-54 — `TaskReset | TaskAbandoned | WorkflowFailed | WorkflowUnavailable` — Enumeration of possible recovery types in the system.
- pub `as_str` function L57-64 — `(&self) -> &'static str` — These are API-level types; backend-specific models handle database storage.
-  `RecoveryType` type L56-65 — `= RecoveryType` — These are API-level types; backend-specific models handle database storage.
-  `String` type L67-71 — `= String` — These are API-level types; backend-specific models handle database storage.
-  `from` function L68-70 — `(recovery_type: RecoveryType) -> Self` — These are API-level types; backend-specific models handle database storage.

#### crates/cloacina/src/models/schedule.rs

- pub `CatchupPolicy` enum L28-31 — `Skip | RunAll` — Enum representing the different catchup policies for missed cron executions.
- pub `ScheduleType` enum L60-63 — `Cron | Trigger` — The type of schedule — determines which fields are relevant.
- pub `Schedule` struct L94-119 — `{ id: UniversalUuid, schedule_type: String, workflow_name: String, enabled: Univ...` — Represents a unified schedule record (domain type).
- pub `get_type` function L123-125 — `(&self) -> ScheduleType` — Returns the schedule type as an enum.
- pub `is_cron` function L128-130 — `(&self) -> bool` — Returns true if this is a cron schedule.
- pub `is_trigger` function L133-135 — `(&self) -> bool` — Returns true if this is a trigger schedule.
- pub `is_enabled` function L138-140 — `(&self) -> bool` — Returns true if the schedule is enabled.
- pub `poll_interval` function L143-146 — `(&self) -> Option<Duration>` — Returns the poll interval as a Duration (trigger schedules only).
- pub `allows_concurrent` function L149-154 — `(&self) -> bool` — Returns true if concurrent executions are allowed (trigger schedules only).
- pub `NewSchedule` struct L159-178 — `{ schedule_type: String, workflow_name: String, enabled: Option<UniversalBool>, ...` — Structure for creating new schedule records.
- pub `cron` function L182-201 — `( workflow_name: &str, cron_expression: &str, next_run_at: UniversalTimestamp, )...` — Create a new cron schedule.
- pub `trigger` function L204-219 — `(trigger_name: &str, workflow_name: &str, poll_interval: Duration) -> Self` — Create a new trigger schedule.
- pub `ScheduleExecution` struct L224-240 — `{ id: UniversalUuid, schedule_id: UniversalUuid, workflow_execution_id: Option<U...` — Represents a schedule execution record (domain type).
- pub `NewScheduleExecution` struct L244-250 — `{ schedule_id: UniversalUuid, workflow_execution_id: Option<UniversalUuid>, sche...` — Structure for creating new schedule execution records.
-  `String` type L33-40 — `= String` — `schedule_executions` tables, replacing the separate cron and trigger models.
-  `from` function L34-39 — `(policy: CatchupPolicy) -> Self` — `schedule_executions` tables, replacing the separate cron and trigger models.
-  `CatchupPolicy` type L42-50 — `= CatchupPolicy` — `schedule_executions` tables, replacing the separate cron and trigger models.
-  `from` function L43-49 — `(s: String) -> Self` — `schedule_executions` tables, replacing the separate cron and trigger models.
-  `CatchupPolicy` type L52-56 — `= CatchupPolicy` — `schedule_executions` tables, replacing the separate cron and trigger models.
-  `from` function L53-55 — `(s: &str) -> Self` — `schedule_executions` tables, replacing the separate cron and trigger models.
-  `ScheduleType` type L65-72 — `= ScheduleType` — `schedule_executions` tables, replacing the separate cron and trigger models.
-  `from` function L66-71 — `(s: &str) -> Self` — `schedule_executions` tables, replacing the separate cron and trigger models.
-  `ScheduleType` type L74-78 — `= ScheduleType` — `schedule_executions` tables, replacing the separate cron and trigger models.
-  `from` function L75-77 — `(s: String) -> Self` — `schedule_executions` tables, replacing the separate cron and trigger models.
-  `ScheduleType` type L80-87 — `= ScheduleType` — `schedule_executions` tables, replacing the separate cron and trigger models.
-  `fmt` function L81-86 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — `schedule_executions` tables, replacing the separate cron and trigger models.
-  `Schedule` type L121-155 — `= Schedule` — `schedule_executions` tables, replacing the separate cron and trigger models.
-  `NewSchedule` type L180-220 — `= NewSchedule` — `schedule_executions` tables, replacing the separate cron and trigger models.
-  `tests` module L253-316 — `-` — `schedule_executions` tables, replacing the separate cron and trigger models.
-  `test_schedule_type_conversions` function L258-264 — `()` — `schedule_executions` tables, replacing the separate cron and trigger models.
-  `test_new_cron_schedule` function L267-274 — `()` — `schedule_executions` tables, replacing the separate cron and trigger models.
-  `test_new_trigger_schedule` function L277-285 — `()` — `schedule_executions` tables, replacing the separate cron and trigger models.
-  `test_schedule_helpers` function L288-315 — `()` — `schedule_executions` tables, replacing the separate cron and trigger models.

#### crates/cloacina/src/models/signing_key.rs

- pub `SigningKey` struct L29-42 — `{ id: UniversalUuid, org_id: UniversalUuid, key_name: String, encrypted_private_...` — Domain model for a signing key.
- pub `is_active` function L46-48 — `(&self) -> bool` — Check if this key is currently active (not revoked)
- pub `is_revoked` function L51-53 — `(&self) -> bool` — Check if this key has been revoked
- pub `NewSigningKey` struct L58-64 — `{ org_id: UniversalUuid, key_name: String, encrypted_private_key: Vec<u8>, publi...` — Model for creating a new signing key.
- pub `new` function L67-81 — `( org_id: UniversalUuid, key_name: String, encrypted_private_key: Vec<u8>, publi...` — Private keys are stored encrypted at rest using AES-256-GCM.
- pub `SigningKeyInfo` struct L86-93 — `{ id: UniversalUuid, org_id: UniversalUuid, key_name: String, key_fingerprint: S...` — Information about a signing key (without the private key material).
-  `SigningKey` type L44-54 — `= SigningKey` — Private keys are stored encrypted at rest using AES-256-GCM.
-  `NewSigningKey` type L66-82 — `= NewSigningKey` — Private keys are stored encrypted at rest using AES-256-GCM.
-  `SigningKeyInfo` type L95-106 — `= SigningKeyInfo` — Private keys are stored encrypted at rest using AES-256-GCM.
-  `from` function L96-105 — `(key: SigningKey) -> Self` — Private keys are stored encrypted at rest using AES-256-GCM.

#### crates/cloacina/src/models/task_execution.rs

- pub `TaskExecution` struct L27-48 — `{ id: UniversalUuid, workflow_execution_id: UniversalUuid, task_name: String, st...` — Represents a task execution record (domain type).
- pub `NewTaskExecution` struct L52-60 — `{ workflow_execution_id: UniversalUuid, task_name: String, status: String, attem...` — Structure for creating new task executions (domain type).

#### crates/cloacina/src/models/task_execution_metadata.rs

- pub `TaskExecutionMetadata` struct L27-35 — `{ id: UniversalUuid, task_execution_id: UniversalUuid, workflow_execution_id: Un...` — Represents a task execution metadata record (domain type).
- pub `NewTaskExecutionMetadata` struct L39-44 — `{ task_execution_id: UniversalUuid, workflow_execution_id: UniversalUuid, task_n...` — Structure for creating new task execution metadata (domain type).

#### crates/cloacina/src/models/task_outbox.rs

- pub `TaskOutbox` struct L37-44 — `{ id: i64, task_execution_id: UniversalUuid, created_at: UniversalTimestamp }` — Represents a task outbox entry (domain type).
- pub `NewTaskOutbox` struct L50-53 — `{ task_execution_id: UniversalUuid }` — Structure for creating new task outbox entries (domain type).

#### crates/cloacina/src/models/trusted_key.rs

- pub `TrustedKey` struct L28-40 — `{ id: UniversalUuid, org_id: UniversalUuid, key_fingerprint: String, public_key:...` — Domain model for a trusted public key.
- pub `is_active` function L44-46 — `(&self) -> bool` — Check if this key is currently trusted (not revoked)
- pub `is_revoked` function L49-51 — `(&self) -> bool` — Check if this key has been revoked
- pub `NewTrustedKey` struct L56-61 — `{ org_id: UniversalUuid, key_fingerprint: String, public_key: Vec<u8>, key_name:...` — Model for creating a new trusted key.
- pub `new` function L64-76 — `( org_id: UniversalUuid, key_fingerprint: String, public_key: Vec<u8>, key_name:...` — derived from the organization's own signing keys.
- pub `from_signing_key` function L79-91 — `( org_id: UniversalUuid, key_fingerprint: String, public_key: Vec<u8>, key_name:...` — Create a trusted key from a signing key's public key.
-  `TrustedKey` type L42-52 — `= TrustedKey` — derived from the organization's own signing keys.
-  `NewTrustedKey` type L63-92 — `= NewTrustedKey` — derived from the organization's own signing keys.

#### crates/cloacina/src/models/workflow_execution.rs

- pub `WorkflowExecutionRecord` struct L27-42 — `{ id: UniversalUuid, workflow_name: String, workflow_version: String, status: St...` — Represents a workflow execution record (domain type).
- pub `NewWorkflowExecution` struct L46-51 — `{ workflow_name: String, workflow_version: String, status: String, context_id: O...` — Structure for creating new workflow executions (domain type).

#### crates/cloacina/src/models/workflow_packages.rs

- pub `StorageType` enum L27-32 — `Database | Filesystem` — Storage type for workflow binary data.
- pub `as_str` function L35-40 — `(&self) -> &'static str` — These are API-level types; backend-specific models handle database storage.
- pub `WorkflowPackage` struct L62-81 — `{ id: UniversalUuid, registry_id: UniversalUuid, package_name: String, version: ...` — Domain model for workflow package metadata.
- pub `NewWorkflowPackage` struct L85-93 — `{ registry_id: UniversalUuid, package_name: String, version: String, description...` — Model for creating new workflow package metadata entries (domain type).
- pub `new` function L96-114 — `( registry_id: UniversalUuid, package_name: String, version: String, description...` — These are API-level types; backend-specific models handle database storage.
-  `StorageType` type L34-41 — `= StorageType` — These are API-level types; backend-specific models handle database storage.
-  `StorageType` type L43-52 — `= StorageType` — These are API-level types; backend-specific models handle database storage.
-  `Err` type L44 — `= std::convert::Infallible` — These are API-level types; backend-specific models handle database storage.
-  `from_str` function L46-51 — `(s: &str) -> Result<Self, Self::Err>` — These are API-level types; backend-specific models handle database storage.
-  `StorageType` type L54-58 — `= StorageType` — These are API-level types; backend-specific models handle database storage.
-  `fmt` function L55-57 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — These are API-level types; backend-specific models handle database storage.
-  `NewWorkflowPackage` type L95-115 — `= NewWorkflowPackage` — These are API-level types; backend-specific models handle database storage.

#### crates/cloacina/src/models/workflow_registry.rs

- pub `WorkflowRegistryEntry` struct L27-31 — `{ id: UniversalUuid, created_at: UniversalTimestamp, data: Vec<u8> }` — Domain model for a workflow registry entry.
- pub `NewWorkflowRegistryEntry` struct L35-37 — `{ data: Vec<u8> }` — Model for creating new workflow registry entries (domain type).
- pub `new` function L40-42 — `(data: Vec<u8>) -> Self` — These are API-level types; backend-specific models handle database storage.
- pub `NewWorkflowRegistryEntryWithId` struct L47-51 — `{ id: UniversalUuid, created_at: UniversalTimestamp, data: Vec<u8> }` — Model for creating new workflow registry entries with explicit ID and timestamp.
-  `NewWorkflowRegistryEntry` type L39-43 — `= NewWorkflowRegistryEntry` — These are API-level types; backend-specific models handle database storage.

### crates/cloacina/src/packaging

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/packaging/debug.rs

- pub `extract_manifest_from_package` function L40-90 — `(package_path: &PathBuf) -> Result<Manifest>` — Extract metadata from a fidius source package and synthesize a [`Manifest`].
- pub `execute_task_from_library` function L93-120 — `( library_path: &PathBuf, task_name: &str, context_json: &str, ) -> Result<Strin...` — Execute a task from a dynamic library via the fidius-host plugin API.
- pub `resolve_task_name` function L123-150 — `(manifest: &Manifest, task_identifier: &str) -> Result<String>` — Resolve a task identifier (index or name) to a task name.
- pub `debug_package` function L153-202 — `( package_path: &PathBuf, task_identifier: Option<&str>, context_json: Option<&s...` — High-level debug function that handles both listing and executing tasks.
- pub `DebugResult` enum L206-209 — `TaskList | TaskExecution` — Result of a debug operation.
- pub `TaskDebugInfo` struct L213-218 — `{ index: usize, id: String, description: String, dependencies: Vec<String> }` — Information about a task for debugging purposes.

#### crates/cloacina/src/packaging/manifest.rs

- pub `ManifestError` enum L38-57 — `InvalidDependencies | InvalidGraphData | LibraryError` — Errors that can occur during manifest extraction.
- pub `generate_manifest` function L63-142 — `( cargo_toml: &CargoToml, so_path: &Path, target: &Option<String>, project_path:...` — Generate a package manifest from Cargo.toml and compiled library.
-  `PACKAGED_WORKFLOW_REGEX` variable L29-34 — `: Lazy<Regex>` — Statically compiled regex for matching workflow attributes.
-  `PackageMetadata` struct L146-150 — `{ description: Option<String>, _author: Option<String>, workflow_fingerprint: Op...` — Package metadata extracted from the plugin.
-  `FfiTaskInfo` struct L154-160 — `{ _index: u32, id: String, dependencies: Vec<String>, description: String, _sour...` — Task information extracted from a cdylib via the fidius plugin API (internal type).
-  `extract_task_info_and_graph_from_library` function L163-229 — `( so_path: &Path, project_path: &Path, ) -> Result<( Vec<FfiTaskInfo>, Option<cr...` — Extract task information and graph data from a compiled library using the fidius plugin API.
-  `extract_package_names_from_source` function L233-256 — `(project_path: &Path) -> Result<Vec<String>>` — Extract package names from source files by looking for #[packaged_workflow] attributes.
-  `get_current_platform` function L258-269 — `() -> String`
-  `get_current_architecture` function L273-275 — `() -> String` — Kept for backward compatibility with external callers.

#### crates/cloacina/src/packaging/manifest_schema.rs

- pub `ManifestValidationError` enum L31-68 — `MissingRuntime | UnsupportedTarget | NoTasks | DuplicateTaskId | InvalidDependen...` — Errors from manifest validation.
- pub `PackageLanguage` enum L73-76 — `Python | Rust` — Package language discriminator.
- pub `PythonRuntime` struct L80-85 — `{ requires_python: String, entry_module: String }` — Python runtime configuration.
- pub `RustRuntime` struct L89-92 — `{ library_path: String }` — Rust runtime configuration.
- pub `PackageInfo` struct L96-108 — `{ name: String, version: String, description: Option<String>, fingerprint: Strin...` — Package metadata.
- pub `TaskDefinition` struct L112-132 — `{ id: String, function: String, dependencies: Vec<String>, description: Option<S...` — Task definition within a package.
- pub `TriggerDefinition` struct L139-155 — `{ name: String, trigger_type: String, workflow: String, poll_interval: String, a...` — Trigger definition within a package.
- pub `Manifest` struct L161-184 — `{ format_version: String, package: PackageInfo, language: PackageLanguage, pytho...` — Unified package manifest (v2).
- pub `validate` function L188-284 — `(&self) -> Result<(), ManifestValidationError>` — Validate the manifest for structural correctness.
- pub `is_compatible_with_platform` function L287-289 — `(&self, platform_str: &str) -> bool` — Check if this package is compatible with a specific platform.
- pub `parse_duration_str` function L293-322 — `(s: &str) -> Result<std::time::Duration, String>` — Parse a duration string like "30s", "5m", "2h", "100ms" into a [`std::time::Duration`].
-  `Manifest` type L186-290 — `= Manifest` — runtime configuration applies.
-  `tests` module L325-654 — `-` — runtime configuration applies.
-  `make_python_manifest` function L328-366 — `() -> Manifest` — runtime configuration applies.
-  `make_rust_manifest` function L368-395 — `() -> Manifest` — runtime configuration applies.
-  `make_manifest_with_triggers` function L397-418 — `() -> Manifest` — runtime configuration applies.
-  `test_python_manifest_validates` function L421-423 — `()` — runtime configuration applies.
-  `test_rust_manifest_validates` function L426-428 — `()` — runtime configuration applies.
-  `test_missing_python_runtime` function L431-438 — `()` — runtime configuration applies.
-  `test_missing_rust_runtime` function L441-448 — `()` — runtime configuration applies.
-  `test_unsupported_target` function L451-458 — `()` — runtime configuration applies.
-  `test_no_tasks` function L461-468 — `()` — runtime configuration applies.
-  `test_duplicate_task_id` function L471-478 — `()` — runtime configuration applies.
-  `test_invalid_dependency` function L481-488 — `()` — runtime configuration applies.
-  `test_invalid_python_function_path` function L491-498 — `()` — runtime configuration applies.
-  `test_rust_function_path_no_colon_ok` function L501-504 — `()` — runtime configuration applies.
-  `test_invalid_format_version` function L507-514 — `()` — runtime configuration applies.
-  `test_serialization_roundtrip` function L517-529 — `()` — runtime configuration applies.
-  `test_platform_compatibility` function L532-537 — `()` — runtime configuration applies.
-  `test_language_serde` function L540-545 — `()` — runtime configuration applies.
-  `test_manifest_with_triggers_validates` function L550-552 — `()` — runtime configuration applies.
-  `test_manifest_no_triggers_still_validates` function L555-559 — `()` — runtime configuration applies.
-  `test_duplicate_trigger_name` function L562-569 — `()` — runtime configuration applies.
-  `test_trigger_invalid_workflow_reference` function L572-579 — `()` — runtime configuration applies.
-  `test_trigger_references_task_id` function L582-587 — `()` — runtime configuration applies.
-  `test_trigger_invalid_poll_interval` function L590-597 — `()` — runtime configuration applies.
-  `test_trigger_poll_interval_variants` function L600-607 — `()` — runtime configuration applies.
-  `test_trigger_serialization_roundtrip` function L610-625 — `()` — runtime configuration applies.
-  `test_trigger_no_config` function L628-637 — `()` — runtime configuration applies.
-  `test_deserialization_without_triggers_field` function L640-653 — `()` — runtime configuration applies.

#### crates/cloacina/src/packaging/mod.rs

- pub `debug` module L23 — `-` — Workflow packaging functionality for creating distributable workflow packages.
- pub `manifest` module L24 — `-` — tools, tests, or other applications that need to package workflows.
- pub `manifest_schema` module L25 — `-` — tools, tests, or other applications that need to package workflows.
- pub `platform` module L26 — `-` — tools, tests, or other applications that need to package workflows.
- pub `types` module L27 — `-` — tools, tests, or other applications that need to package workflows.
- pub `validation` module L28 — `-` — tools, tests, or other applications that need to package workflows.
- pub `package_workflow` function L51-74 — `(project_path: PathBuf, output_path: PathBuf) -> Result<()>` — High-level function to package a workflow project using fidius source packaging.
-  `tests` module L31 — `-` — tools, tests, or other applications that need to package workflows.

#### crates/cloacina/src/packaging/platform.rs

- pub `SUPPORTED_TARGETS` variable L20-21 — `: &[&str]` — Supported target platforms for workflow packages.
- pub `detect_current_platform` function L24-50 — `() -> &'static str` — Detect the current platform as a target string.
-  `tests` module L53-67 — `-` — Platform detection and target validation for workflow packages.
-  `test_detect_current_platform_is_known` function L57-61 — `()` — Platform detection and target validation for workflow packages.
-  `test_supported_targets_not_empty` function L64-66 — `()` — Platform detection and target validation for workflow packages.

#### crates/cloacina/src/packaging/tests.rs

-  `tests` module L21-327 — `-` — Unit tests for packaging functionality
-  `create_test_cargo_toml` function L27-42 — `() -> types::CargoToml` — Create a minimal test Cargo.toml structure
-  `create_mock_library_file` function L45-53 — `() -> (TempDir, PathBuf)` — Create a mock compiled library file for testing
-  `create_test_project` function L56-81 — `() -> (TempDir, PathBuf)` — Create a test project structure
-  `test_generate_manifest_basic` function L84-113 — `()` — Unit tests for packaging functionality
-  `test_generate_manifest_with_target` function L116-135 — `()` — Unit tests for packaging functionality
-  `test_generate_manifest_missing_package` function L138-150 — `()` — Unit tests for packaging functionality
-  `test_extract_package_names_from_source` function L153-167 — `()` — Unit tests for packaging functionality
-  `test_extract_package_names_no_packages` function L170-195 — `()` — Unit tests for packaging functionality
-  `test_extract_package_names_missing_src` function L198-208 — `()` — Unit tests for packaging functionality
-  `test_get_current_architecture` function L211-224 — `()` — Unit tests for packaging functionality
-  `test_compile_options_builder_pattern` function L227-239 — `()` — Unit tests for packaging functionality
-  `test_manifest_schema_rust_package` function L242-294 — `()` — Unit tests for packaging functionality
-  `test_constants` function L297-316 — `()` — Unit tests for packaging functionality
-  `test_manifest_error_display` function L319-326 — `()` — Unit tests for packaging functionality

#### crates/cloacina/src/packaging/types.rs

- pub `CompileOptions` struct L21-30 — `{ target: Option<String>, profile: String, cargo_flags: Vec<String>, jobs: Optio...` — Options for compiling a workflow
- pub `CargoToml` struct L45-49 — `{ package: Option<CargoPackage>, lib: Option<CargoLib>, dependencies: Option<tom...` — Parsed Cargo.toml structure
- pub `CargoPackage` struct L53-61 — `{ name: String, version: String, description: Option<String>, authors: Option<Ve...` — Package section from Cargo.toml
- pub `CargoLib` struct L65-68 — `{ crate_type: Option<Vec<String>> }` — Library section from Cargo.toml
- pub `MANIFEST_FILENAME` variable L71 — `: &str` — Constants
- pub `CLOACINA_VERSION` variable L72 — `: &str`
-  `CompileOptions` type L32-41 — `impl Default for CompileOptions`
-  `default` function L33-40 — `() -> Self`

#### crates/cloacina/src/packaging/validation.rs

- pub `validate_rust_crate_structure` function L25-44 — `(project_path: &PathBuf) -> Result<()>` — Validate that the project has a valid Rust crate structure
- pub `validate_cargo_toml` function L47-71 — `(project_path: &Path) -> Result<CargoToml>` — Parse and validate Cargo.toml
- pub `validate_cloacina_compatibility` function L77-94 — `(cargo_toml: &CargoToml) -> Result<()>` — Validate cloacina dependency compatibility.
- pub `validate_packaged_workflow_presence` function L99-128 — `(project_path: &Path) -> Result<()>` — Check for workflow macros in the source code.
- pub `validate_rust_version_compatibility` function L131-153 — `(cargo_toml: &CargoToml) -> Result<()>` — Validate Rust version compatibility

### crates/cloacina/src/python/bindings

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/python/bindings/admin.rs

- pub `PyTenantConfig` struct L29-31 — `{ inner: TenantConfig }` — Python wrapper for TenantConfig
- pub `new` function L36-44 — `(schema_name: String, username: String, password: Option<String>) -> Self` — multi-tenant PostgreSQL deployments.
- pub `schema_name` function L47-49 — `(&self) -> String` — multi-tenant PostgreSQL deployments.
- pub `username` function L52-54 — `(&self) -> String` — multi-tenant PostgreSQL deployments.
- pub `password` function L57-59 — `(&self) -> String` — multi-tenant PostgreSQL deployments.
- pub `__repr__` function L61-66 — `(&self) -> String` — multi-tenant PostgreSQL deployments.
- pub `PyTenantCredentials` struct L71-73 — `{ inner: TenantCredentials }` — Python wrapper for TenantCredentials
- pub `username` function L78-80 — `(&self) -> String` — multi-tenant PostgreSQL deployments.
- pub `password` function L83-85 — `(&self) -> String` — multi-tenant PostgreSQL deployments.
- pub `schema_name` function L88-90 — `(&self) -> String` — multi-tenant PostgreSQL deployments.
- pub `connection_string` function L93-95 — `(&self) -> String` — multi-tenant PostgreSQL deployments.
- pub `__repr__` function L97-102 — `(&self) -> String` — multi-tenant PostgreSQL deployments.
- pub `PyDatabaseAdmin` struct L115-117 — `{ inner: DatabaseAdmin }` — Python wrapper for DatabaseAdmin
- pub `new` function L122-165 — `(database_url: String) -> PyResult<Self>` — multi-tenant PostgreSQL deployments.
- pub `create_tenant` function L167-184 — `(&self, config: &PyTenantConfig) -> PyResult<PyTenantCredentials>` — multi-tenant PostgreSQL deployments.
- pub `remove_tenant` function L186-196 — `(&self, schema_name: String, username: String) -> PyResult<()>` — multi-tenant PostgreSQL deployments.
- pub `__repr__` function L198-200 — `(&self) -> String` — multi-tenant PostgreSQL deployments.
-  `PyTenantConfig` type L34-67 — `= PyTenantConfig` — multi-tenant PostgreSQL deployments.
-  `PyTenantCredentials` type L76-103 — `= PyTenantCredentials` — multi-tenant PostgreSQL deployments.
-  `is_postgres_url` function L106-108 — `(url: &str) -> bool` — Helper to check if a URL is a PostgreSQL connection string
-  `PyDatabaseAdmin` type L120-201 — `= PyDatabaseAdmin` — multi-tenant PostgreSQL deployments.
-  `tests` module L204-264 — `-` — multi-tenant PostgreSQL deployments.
-  `test_tenant_config_new` function L208-217 — `()` — multi-tenant PostgreSQL deployments.
-  `test_tenant_config_default_password` function L220-223 — `()` — multi-tenant PostgreSQL deployments.
-  `test_tenant_config_repr` function L226-237 — `()` — multi-tenant PostgreSQL deployments.
-  `test_is_postgres_url` function L240-245 — `()` — multi-tenant PostgreSQL deployments.
-  `test_database_admin_rejects_sqlite` function L248-251 — `()` — multi-tenant PostgreSQL deployments.
-  `test_database_admin_rejects_invalid_url` function L254-257 — `()` — multi-tenant PostgreSQL deployments.
-  `test_database_admin_rejects_missing_db_name` function L260-263 — `()` — multi-tenant PostgreSQL deployments.

#### crates/cloacina/src/python/bindings/context.rs

- pub `PyDefaultRunnerConfig` struct L26-28 — `{ inner: crate::runner::DefaultRunnerConfig }` — PyDefaultRunnerConfig - Python wrapper for Rust DefaultRunnerConfig
- pub `new` function L51-119 — `( max_concurrent_tasks: Option<usize>, scheduler_poll_interval_ms: Option<u64>, ...`
- pub `default` function L124-128 — `() -> Self` — Creates a DefaultRunnerConfig with all default values
- pub `max_concurrent_tasks` function L131-133 — `(&self) -> usize`
- pub `scheduler_poll_interval_ms` function L136-138 — `(&self) -> u64`
- pub `task_timeout_seconds` function L141-143 — `(&self) -> u64`
- pub `workflow_timeout_seconds` function L146-148 — `(&self) -> Option<u64>`
- pub `db_pool_size` function L151-153 — `(&self) -> u32`
- pub `enable_recovery` function L156-158 — `(&self) -> bool`
- pub `enable_cron_scheduling` function L161-163 — `(&self) -> bool`
- pub `cron_poll_interval_seconds` function L166-168 — `(&self) -> u64`
- pub `cron_max_catchup_executions` function L171-173 — `(&self) -> usize`
- pub `cron_enable_recovery` function L176-178 — `(&self) -> bool`
- pub `cron_recovery_interval_seconds` function L181-183 — `(&self) -> u64`
- pub `cron_lost_threshold_minutes` function L186-188 — `(&self) -> i32`
- pub `cron_max_recovery_age_seconds` function L191-193 — `(&self) -> u64`
- pub `cron_max_recovery_attempts` function L196-198 — `(&self) -> usize`
- pub `set_max_concurrent_tasks` function L201-203 — `(&mut self, value: usize)`
- pub `set_scheduler_poll_interval_ms` function L206-209 — `(&mut self, value: u64)`
- pub `set_task_timeout_seconds` function L212-214 — `(&mut self, value: u64)`
- pub `set_workflow_timeout_seconds` function L217-220 — `(&mut self, value: Option<u64>)`
- pub `set_db_pool_size` function L223-225 — `(&mut self, value: u32)`
- pub `set_enable_recovery` function L228-230 — `(&mut self, value: bool)`
- pub `set_enable_cron_scheduling` function L233-235 — `(&mut self, value: bool)`
- pub `set_cron_poll_interval_seconds` function L238-240 — `(&mut self, value: u64)`
- pub `set_cron_max_catchup_executions` function L243-245 — `(&mut self, value: usize)`
- pub `set_cron_enable_recovery` function L248-250 — `(&mut self, value: bool)`
- pub `set_cron_recovery_interval_seconds` function L253-256 — `(&mut self, value: u64)`
- pub `set_cron_lost_threshold_minutes` function L259-261 — `(&mut self, value: i32)`
- pub `set_cron_max_recovery_age_seconds` function L264-267 — `(&mut self, value: u64)`
- pub `set_cron_max_recovery_attempts` function L270-272 — `(&mut self, value: usize)`
- pub `to_dict` function L275-321 — `(&self, py: Python<'_>) -> PyResult<PyObject>` — Returns a dictionary representation of the configuration
- pub `__repr__` function L324-331 — `(&self) -> String` — String representation of the configuration
-  `PyDefaultRunnerConfig` type L31-332 — `= PyDefaultRunnerConfig`
-  `PyDefaultRunnerConfig` type L334-366 — `= PyDefaultRunnerConfig`
-  `to_rust_config` function L336-338 — `(&self) -> crate::runner::DefaultRunnerConfig` — Get the inner Rust config (for internal use)
-  `rebuild` function L340-365 — `( &self, apply: impl FnOnce( crate::runner::DefaultRunnerConfigBuilder, ) -> cra...`
-  `tests` module L369-477 — `-`
-  `test_default_construction` function L373-380 — `()`
-  `test_new_with_defaults` function L383-399 — `()`
-  `test_new_with_custom_params` function L402-435 — `()`
-  `test_repr` function L438-445 — `()`
-  `test_setters` function L448-463 — `()`
-  `test_to_dict` function L466-476 — `()`

#### crates/cloacina/src/python/bindings/mod.rs

- pub `admin` module L27 — `-` — Python API wrapper types for the cloaca wheel.
- pub `context` module L28 — `-` — - `PyRetryPolicy` / `PyBackoffStrategy` / `PyRetryCondition` — retry config
- pub `runner` module L29 — `-` — - `PyRetryPolicy` / `PyBackoffStrategy` / `PyRetryCondition` — retry config
- pub `trigger` module L30 — `-` — - `PyRetryPolicy` / `PyBackoffStrategy` / `PyRetryCondition` — retry config
- pub `value_objects` module L31 — `-` — - `PyRetryPolicy` / `PyBackoffStrategy` / `PyRetryCondition` — retry config

#### crates/cloacina/src/python/bindings/runner.rs

- pub `ShutdownError` enum L34-46 — `ChannelClosed | ThreadPanic | Timeout` — Errors that can occur during async runtime shutdown
- pub `PyWorkflowResult` struct L216-218 — `{ inner: crate::executor::WorkflowExecutionResult }` — Python wrapper for WorkflowExecutionResult
- pub `status` function L223-225 — `(&self) -> String`
- pub `start_time` function L228-230 — `(&self) -> String`
- pub `end_time` function L233-235 — `(&self) -> Option<String>`
- pub `final_context` function L238-241 — `(&self) -> PyContext`
- pub `error_message` function L244-246 — `(&self) -> Option<&str>`
- pub `__repr__` function L248-254 — `(&self) -> String`
- pub `from_result` function L258-260 — `(result: crate::executor::WorkflowExecutionResult) -> Self`
- pub `PyDefaultRunner` struct L687-689 — `{ runtime_handle: Mutex<AsyncRuntimeHandle> }` — Python wrapper for DefaultRunner
- pub `new` function L722-731 — `(database_url: &str) -> PyResult<Self>` — Create a new DefaultRunner with database connection
- pub `with_config` function L735-747 — `( database_url: &str, config: &super::context::PyDefaultRunnerConfig, ) -> PyRes...` — Create a new DefaultRunner with custom configuration
- pub `with_schema` function L758-785 — `(database_url: &str, schema: &str) -> PyResult<PyDefaultRunner>` — Create a new DefaultRunner with PostgreSQL schema-based multi-tenancy
- pub `execute` function L788-802 — `( &self, workflow_name: &str, context: &PyContext, py: Python, ) -> PyResult<PyW...` — Execute a workflow by name with context
- pub `shutdown` function L805-821 — `(&self, py: Python) -> PyResult<()>` — Shutdown the runner and cleanup resources
- pub `register_cron_workflow` function L836-851 — `( &self, workflow_name: String, cron_expression: String, timezone: String, py: P...` — Register a cron workflow for automatic execution at scheduled times
- pub `list_cron_schedules` function L859-879 — `( &self, enabled_only: Option<bool>, limit: Option<i64>, offset: Option<i64>, py...` — List all cron schedules
- pub `set_cron_schedule_enabled` function L882-895 — `( &self, schedule_id: String, enabled: bool, py: Python, ) -> PyResult<()>` — Enable or disable a cron schedule
- pub `delete_cron_schedule` function L898-905 — `(&self, schedule_id: String, py: Python) -> PyResult<()>` — Delete a cron schedule
- pub `get_cron_schedule` function L908-917 — `(&self, schedule_id: String, py: Python) -> PyResult<PyObject>` — Get details of a specific cron schedule
- pub `update_cron_schedule` function L920-935 — `( &self, schedule_id: String, cron_expression: String, timezone: String, py: Pyt...` — Update a cron schedule's expression and timezone
- pub `get_cron_execution_history` function L938-962 — `( &self, schedule_id: String, limit: Option<i64>, offset: Option<i64>, py: Pytho...` — Get execution history for a specific cron schedule
- pub `get_cron_execution_stats` function L968-991 — `(&self, since: String, py: Python) -> PyResult<PyObject>` — Get execution statistics for cron schedules
- pub `list_trigger_schedules` function L999-1019 — `( &self, enabled_only: Option<bool>, limit: Option<i64>, offset: Option<i64>, py...` — List all trigger schedules
- pub `get_trigger_schedule` function L1022-1038 — `( &self, trigger_name: String, py: Python, ) -> PyResult<Option<PyObject>>` — Get details of a specific trigger schedule
- pub `set_trigger_enabled` function L1041-1054 — `( &self, trigger_name: String, enabled: bool, py: Python, ) -> PyResult<()>` — Enable or disable a trigger
- pub `get_trigger_execution_history` function L1058-1082 — `( &self, trigger_name: String, limit: Option<i64>, offset: Option<i64>, py: Pyth...` — Get execution history for a specific trigger
- pub `__repr__` function L1088-1090 — `(&self) -> String`
- pub `__enter__` function L1092-1094 — `(slf: PyRef<Self>) -> PyRef<Self>`
- pub `__exit__` function L1096-1105 — `( &self, py: Python, _exc_type: Option<&Bound<PyAny>>, _exc_value: Option<&Bound...`
-  `SHUTDOWN_TIMEOUT` variable L30 — `: Duration` — Timeout for waiting on runtime thread shutdown
-  `RuntimeMessage` enum L49-146 — `Execute | RegisterCronWorkflow | ListCronSchedules | SetCronScheduleEnabled | De...` — Message types for communication with the async runtime thread
-  `AsyncRuntimeHandle` struct L149-152 — `{ tx: mpsc::UnboundedSender<RuntimeMessage>, thread_handle: Option<thread::JoinH...` — Handle to the background async runtime thread
-  `AsyncRuntimeHandle` type L154-204 — `= AsyncRuntimeHandle`
-  `shutdown` function L156-203 — `(&mut self) -> Result<(), ShutdownError>` — Shutdown the runtime thread and wait for it to complete
-  `AsyncRuntimeHandle` type L206-212 — `impl Drop for AsyncRuntimeHandle`
-  `drop` function L207-211 — `(&mut self)`
-  `PyWorkflowResult` type L221-255 — `= PyWorkflowResult`
-  `PyWorkflowResult` type L257-261 — `= PyWorkflowResult`
-  `parse_schedule_id` function L268-278 — `( schedule_id: &str, ) -> Result<crate::database::universal_types::UniversalUuid...` — Parse a schedule ID string into a UniversalUuid.
-  `schedule_to_cron_dict` function L281-303 — `( schedule: crate::models::schedule::Schedule, py: Python, ) -> PyResult<PyObjec...` — Convert a cron Schedule to a Python dict.
-  `schedule_to_trigger_dict` function L306-324 — `( schedule: crate::models::schedule::Schedule, py: Python, ) -> PyResult<PyObjec...` — Convert a trigger Schedule to a Python dict.
-  `cron_execution_to_dict` function L327-346 — `( execution: crate::models::schedule::ScheduleExecution, py: Python, ) -> PyResu...` — Convert a cron ScheduleExecution to a Python dict.
-  `trigger_execution_to_dict` function L349-368 — `( execution: crate::models::schedule::ScheduleExecution, py: Python, ) -> PyResu...` — Convert a trigger ScheduleExecution to a Python dict.
-  `run_event_loop` function L373-619 — `( runner: Arc<crate::DefaultRunner>, mut rx: mpsc::UnboundedReceiver<RuntimeMess...` — The single event loop that dispatches RuntimeMessages to the DefaultRunner.
-  `spawn_runtime` function L626-679 — `(create_runner: F) -> PyResult<PyDefaultRunner>` — Spawn a background thread running a Tokio runtime with a DefaultRunner
-  `PyDefaultRunner` type L692-716 — `= PyDefaultRunner` — Internal (non-Python) helpers.
-  `send_and_recv` function L696-715 — `( &self, message: RuntimeMessage, response_rx: oneshot::Receiver<Result<T, crate...` — Send a message to the runtime thread and block until a response arrives.
-  `PyDefaultRunner` type L719-1106 — `= PyDefaultRunner`
-  `tests` module L1110-1646 — `-`
-  `TEST_PG_URL` variable L1114 — `: &str`
-  `unique_sqlite_url` function L1116-1121 — `() -> String`
-  `test_runner_repr` function L1125-1132 — `()`
-  `test_runner_shutdown` function L1136-1142 — `()`
-  `test_runner_context_manager` function L1146-1158 — `()`
-  `test_runner_list_cron_schedules_empty` function L1162-1172 — `()`
-  `test_runner_list_trigger_schedules_empty` function L1176-1186 — `()`
-  `test_runner_get_trigger_schedule_not_found` function L1190-1199 — `()`
-  `test_runner_register_cron_workflow` function L1203-1220 — `()`
-  `test_runner_list_cron_schedules_after_register` function L1224-1244 — `()`
-  `test_runner_get_cron_schedule` function L1248-1268 — `()`
-  `test_runner_set_cron_schedule_enabled` function L1272-1294 — `()`
-  `test_runner_delete_cron_schedule` function L1298-1320 — `()`
-  `test_runner_update_cron_schedule` function L1324-1348 — `()`
-  `test_runner_get_cron_execution_history_empty` function L1352-1372 — `()`
-  `test_runner_get_cron_execution_stats` function L1376-1388 — `()`
-  `test_runner_set_cron_schedule_enabled_invalid_id` function L1392-1401 — `()`
-  `test_runner_set_trigger_enabled` function L1405-1414 — `()`
-  `test_runner_get_trigger_execution_history` function L1418-1428 — `()`
-  `test_workflow_result_completed` function L1432-1463 — `()`
-  `test_workflow_result_failed` function L1467-1486 — `()`
-  `test_runner_execute_nonexistent_workflow` function L1490-1503 — `()`
-  `test_runner_execute_registered_workflow` function L1507-1554 — `()`
-  `NoOpTask` struct L1514 — `-`
-  `NoOpTask` type L1517-1530 — `= NoOpTask`
-  `execute` function L1518-1523 — `( &self, context: crate::Context<serde_json::Value>, ) -> Result<crate::Context<...`
-  `id` function L1524-1526 — `(&self) -> &str`
-  `dependencies` function L1527-1529 — `(&self) -> &[crate::TaskNamespace]`
-  `test_runner_get_cron_execution_stats_invalid_date` function L1558-1567 — `()`
-  `test_runner_list_cron_schedules_enabled_only` function L1571-1600 — `()`
-  `test_with_schema_rejects_sqlite` function L1606-1610 — `()`
-  `test_with_schema_rejects_empty_schema` function L1614-1621 — `()`
-  `test_with_schema_rejects_invalid_chars` function L1625-1632 — `()`
-  `test_shutdown_error_display` function L1636-1645 — `()`

#### crates/cloacina/src/python/bindings/trigger.rs

- pub `PyTriggerResult` struct L37-40 — `{ is_fire: bool, data: Option<std::collections::HashMap<String, Value>> }` — Python TriggerResult class - represents the result of a trigger poll.
- pub `into_rust` function L90-103 — `(self) -> TriggerResult` — Convert to Rust TriggerResult
- pub `PythonTriggerWrapper` struct L110-116 — `{ name: String, workflow_name: String, poll_interval: Duration, allow_concurrent...` — Python trigger wrapper implementing Rust Trigger trait.
- pub `workflow_name` function L197-199 — `(&self) -> &str` — Get the workflow name this trigger is associated with
- pub `TriggerDecorator` struct L229-234 — `{ name: Option<String>, workflow: String, poll_interval: Duration, allow_concurr...` — Decorator class that holds trigger configuration
- pub `__call__` function L238-276 — `(&self, py: Python, func: PyObject) -> PyResult<PyObject>` — user-defined conditions and fire workflows when those conditions are met.
- pub `trigger` function L311-326 — `( workflow: String, name: Option<String>, poll_interval: &str, allow_concurrent:...` — user-defined conditions and fire workflows when those conditions are met.
-  `PyTriggerResult` type L43-86 — `= PyTriggerResult` — user-defined conditions and fire workflows when those conditions are met.
-  `skip` function L46-51 — `() -> Self` — Create a Skip result - condition not met, continue polling.
-  `fire` function L59-65 — `(context: Option<&PyContext>) -> Self` — Create a Fire result - condition met, trigger the workflow.
-  `__repr__` function L67-75 — `(&self) -> String` — user-defined conditions and fire workflows when those conditions are met.
-  `is_fire_result` function L78-80 — `(&self) -> bool` — Check if this is a Fire result
-  `is_skip_result` function L83-85 — `(&self) -> bool` — Check if this is a Skip result
-  `PyTriggerResult` type L88-104 — `= PyTriggerResult` — user-defined conditions and fire workflows when those conditions are met.
-  `PythonTriggerWrapper` type L118-128 — `= PythonTriggerWrapper` — user-defined conditions and fire workflows when those conditions are met.
-  `fmt` function L119-127 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — user-defined conditions and fire workflows when those conditions are met.
-  `PythonTriggerWrapper` type L130 — `impl Send for PythonTriggerWrapper` — user-defined conditions and fire workflows when those conditions are met.
-  `PythonTriggerWrapper` type L131 — `impl Sync for PythonTriggerWrapper` — user-defined conditions and fire workflows when those conditions are met.
-  `PythonTriggerWrapper` type L134-193 — `impl Trigger for PythonTriggerWrapper` — user-defined conditions and fire workflows when those conditions are met.
-  `name` function L135-137 — `(&self) -> &str` — user-defined conditions and fire workflows when those conditions are met.
-  `poll_interval` function L139-141 — `(&self) -> Duration` — user-defined conditions and fire workflows when those conditions are met.
-  `allow_concurrent` function L143-145 — `(&self) -> bool` — user-defined conditions and fire workflows when those conditions are met.
-  `poll` function L147-192 — `(&self) -> Result<TriggerResult, TriggerError>` — user-defined conditions and fire workflows when those conditions are met.
-  `PythonTriggerWrapper` type L195-200 — `= PythonTriggerWrapper` — user-defined conditions and fire workflows when those conditions are met.
-  `parse_duration` function L203-225 — `(s: &str) -> Result<Duration, String>` — Parse duration string like "5s", "100ms", "1m" into Duration
-  `TriggerDecorator` type L237-277 — `= TriggerDecorator` — user-defined conditions and fire workflows when those conditions are met.
-  `tests` module L329-339 — `-` — user-defined conditions and fire workflows when those conditions are met.
-  `test_parse_duration` function L333-338 — `()` — user-defined conditions and fire workflows when those conditions are met.

### crates/cloacina/src/python/bindings/value_objects

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/python/bindings/value_objects/mod.rs

- pub `retry` module L23 — `-`

#### crates/cloacina/src/python/bindings/value_objects/retry.rs

- pub `PyRetryPolicy` struct L23-25 — `{ inner: crate::retry::RetryPolicy }` — Python wrapper for RetryPolicy
- pub `PyBackoffStrategy` struct L30-32 — `{ inner: crate::retry::BackoffStrategy }` — Python wrapper for BackoffStrategy
- pub `PyRetryCondition` struct L37-39 — `{ inner: crate::retry::RetryCondition }` — Python wrapper for RetryCondition
- pub `PyRetryPolicyBuilder` struct L44-51 — `{ max_attempts: Option<i32>, backoff_strategy: Option<crate::retry::BackoffStrat...` — Python wrapper for RetryPolicy::Builder
- pub `builder` function L57-66 — `() -> PyRetryPolicyBuilder` — Create a builder for constructing RetryPolicy
- pub `default` function L71-75 — `() -> Self` — Create a default RetryPolicy
- pub `should_retry` function L78-82 — `(&self, attempt: i32, _error_type: &str) -> bool` — Check if a retry should be attempted
- pub `calculate_delay` function L85-88 — `(&self, attempt: i32) -> f64` — Calculate delay for a given attempt
- pub `max_attempts` function L92-94 — `(&self) -> i32` — Get maximum number of attempts
- pub `initial_delay` function L98-100 — `(&self) -> f64` — Get initial delay in seconds
- pub `max_delay` function L104-106 — `(&self) -> f64` — Get maximum delay in seconds
- pub `with_jitter` function L110-112 — `(&self) -> bool` — Check if jitter is enabled
- pub `__repr__` function L115-123 — `(&self) -> String` — String representation
- pub `fixed` function L130-134 — `() -> Self` — Fixed delay strategy
- pub `linear` function L138-142 — `(multiplier: f64) -> Self` — Linear backoff strategy
- pub `exponential` function L146-153 — `(base: f64, multiplier: Option<f64>) -> Self` — Exponential backoff strategy
- pub `__repr__` function L156-172 — `(&self) -> String` — String representation
- pub `never` function L179-183 — `() -> Self` — Never retry
- pub `transient_only` function L187-191 — `() -> Self` — Retry only on transient errors
- pub `all_errors` function L195-199 — `() -> Self` — Retry on all errors
- pub `error_pattern` function L203-207 — `(patterns: Vec<String>) -> Self` — Retry on specific error patterns
- pub `__repr__` function L210-221 — `(&self) -> String` — String representation
- pub `max_attempts` function L227-231 — `(&self, attempts: i32) -> Self` — Set maximum number of retry attempts
- pub `initial_delay` function L234-238 — `(&self, delay_seconds: f64) -> Self` — Set initial delay
- pub `max_delay` function L241-245 — `(&self, delay_seconds: f64) -> Self` — Set maximum delay
- pub `backoff_strategy` function L248-252 — `(&self, strategy: PyBackoffStrategy) -> Self` — Set backoff strategy
- pub `retry_condition` function L255-259 — `(&self, condition: PyRetryCondition) -> Self` — Set retry condition
- pub `with_jitter` function L262-266 — `(&self, jitter: bool) -> Self` — Enable/disable jitter
- pub `build` function L269-294 — `(&self) -> PyRetryPolicy` — Build the RetryPolicy
- pub `from_rust` function L299-301 — `(policy: crate::retry::RetryPolicy) -> Self` — Convert from Rust RetryPolicy (for internal use)
- pub `to_rust` function L304-306 — `(&self) -> crate::retry::RetryPolicy` — Convert to Rust RetryPolicy (for internal use)
-  `PyRetryPolicy` type L54-124 — `= PyRetryPolicy`
-  `PyBackoffStrategy` type L127-173 — `= PyBackoffStrategy`
-  `PyRetryCondition` type L176-222 — `= PyRetryCondition`
-  `PyRetryPolicyBuilder` type L225-295 — `= PyRetryPolicyBuilder`
-  `PyRetryPolicy` type L297-307 — `= PyRetryPolicy`
-  `tests` module L310-443 — `-`
-  `test_default_policy` function L314-321 — `()`
-  `test_builder_defaults` function L324-329 — `()`
-  `test_builder_chain` function L332-344 — `()`
-  `test_should_retry` function L347-355 — `()`
-  `test_calculate_delay` function L358-366 — `()`
-  `test_retry_policy_repr` function L369-376 — `()`
-  `test_backoff_strategy_fixed` function L379-383 — `()`
-  `test_backoff_strategy_linear` function L386-392 — `()`
-  `test_backoff_strategy_exponential` function L395-401 — `()`
-  `test_retry_condition_never` function L404-408 — `()`
-  `test_retry_condition_transient_only` function L411-415 — `()`
-  `test_retry_condition_all_errors` function L418-422 — `()`
-  `test_retry_condition_error_pattern` function L425-431 — `()`
-  `test_from_rust_to_rust_roundtrip` function L434-442 — `()`

### crates/cloacina/src/python

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/python/computation_graph.rs

- pub `PyAccumulatorRegistration` struct L95-99 — `{ name: String, accumulator_type: String, config: HashMap<String, String> }` — Metadata for a registered Python accumulator.
- pub `get_registered_accumulators` function L112-119 — `() -> Vec<PyAccumulatorRegistration>` — Get all registered accumulators (for testing/inspection).
- pub `drain_accumulators` function L122-125 — `() -> HashMap<String, (PyObject, PyAccumulatorRegistration)>` — Drain all registered accumulators (used by builder on __exit__).
- pub `passthrough_accumulator_decorator` function L135-144 — `(py: Python<'_>, func: PyObject) -> PyResult<PyObject>` — The `@cloaca.passthrough_accumulator` decorator.
- pub `stream_accumulator_decorator` function L154-194 — `( py: Python<'_>, r#type: String, topic: String, group: Option<String>, ) -> PyR...` — Factory for `@cloaca.stream_accumulator(type=..., topic=...)`.
- pub `polling_accumulator_decorator` function L203-231 — `(py: Python<'_>, interval: String) -> PyResult<PyObject>` — Factory for `@cloaca.polling_accumulator(interval=...)`.
- pub `batch_accumulator_decorator` function L240-276 — `( py: Python<'_>, flush_interval: String, max_buffer_size: Option<usize>, ) -> P...` — Factory for `@cloaca.batch_accumulator(flush_interval=..., max_buffer_size=...)`.
- pub `node` function L303-315 — `(py: Python<'_>, func: PyObject) -> PyResult<PyObject>` — The `@cloaca.node` decorator.
- pub `PyComputationGraphBuilder` struct L322-327 — `{ name: String, react_mode: String, accumulators: Vec<String>, nodes_decl: Vec<P...` — ```
- pub `new` function L333-361 — `( _py: Python<'_>, name: &str, react: &Bound<'_, PyDict>, graph: &Bound<'_, PyDi...` — ```
- pub `__enter__` function L364-367 — `(slf: PyRef<Self>) -> PyRef<Self>` — Context manager entry — establish graph context for @node decorators
- pub `__exit__` function L370-423 — `( &self, py: Python, _exc_type: Option<&Bound<PyAny>>, _exc_value: Option<&Bound...` — Context manager exit — validate nodes against topology, build executor
- pub `__repr__` function L425-431 — `(&self) -> String` — ```
- pub `execute` function L437-455 — `(&self, py: Python<'_>, inputs: &Bound<'_, PyDict>) -> PyResult<PyObject>` — Execute the computation graph with the given input cache.
- pub `get_graph_executor` function L476-478 — `(name: &str) -> Option<PythonGraphExecutor>` — Get a registered graph executor by name (for testing / reactor use).
- pub `PythonGraphExecutor` struct L481-488 — `{ name: String, node_functions: HashMap<String, PyObject>, node_map: HashMap<Str...` — ```
- pub `execute_sync` function L515-556 — `( &self, py: Python<'_>, inputs: &HashMap<String, PyObject>, ) -> PyResult<PyObj...` — Execute the graph synchronously from Python with dict inputs.
- pub `execute` function L559-594 — `( &self, cache: &crate::computation_graph::types::InputCache, ) -> GraphResult` — Execute the graph with the given input cache.
- pub `build_python_graph_declaration` function L601-664 — `( graph_name: &str, tenant_id: Option<String>, accumulator_overrides: &[cloacina...` — Build a [`ComputationGraphDeclaration`] from a registered Python graph executor.
-  `NODE_REGISTRY` variable L62-63 — `: Lazy<Mutex<HashMap<String, PyObject>>>` — ```
-  `ACTIVE_GRAPH_CONTEXT` variable L64 — `: Lazy<Mutex<Option<String>>>` — ```
-  `push_graph_context` function L66-69 — `(name: String)` — ```
-  `pop_graph_context` function L71-74 — `()` — ```
-  `current_graph_context` function L76-78 — `() -> Option<String>` — ```
-  `register_node` function L80-82 — `(name: String, func: PyObject)` — ```
-  `drain_nodes` function L84-87 — `() -> HashMap<String, PyObject>` — ```
-  `ACCUMULATOR_REGISTRY` variable L101-102 — `: Lazy<Mutex<HashMap<String, (PyObject, PyAccumulatorRegistration)>>>` — ```
-  `register_accumulator` function L104-109 — `(name: String, func: PyObject, reg: PyAccumulatorRegistration)` — ```
-  `PyNodeDecl` struct L283-287 — `{ name: String, cache_inputs: Vec<String>, edge: PyEdgeDecl }` — ```
-  `PyEdgeDecl` enum L290-294 — `Linear | Routing | Terminal` — ```
-  `PyComputationGraphBuilder` type L330-456 — `= PyComputationGraphBuilder` — ```
-  `GRAPH_EXECUTORS` variable L463-464 — `: Lazy<Mutex<HashMap<String, PythonGraphExecutor>>>` — Global registry of graph executors.
-  `register_graph_executor` function L466-473 — `( name: String, executor: PythonGraphExecutor, _py: Python<'_>, ) -> PyResult<()...` — ```
-  `PythonGraphExecutor` type L491 — `impl Send for PythonGraphExecutor` — ```
-  `PythonGraphExecutor` type L492 — `impl Sync for PythonGraphExecutor` — ```
-  `PythonGraphExecutor` type L494-509 — `impl Clone for PythonGraphExecutor` — ```
-  `clone` function L495-508 — `(&self) -> Self` — ```
-  `PythonGraphExecutor` type L511-595 — `= PythonGraphExecutor` — ```
-  `execute_graph_sync` function L670-812 — `( py: Python<'_>, node_functions: &HashMap<String, PyObject>, execution_order: &...` — ```
-  `build_node_args` function L814-855 — `( py: Python<'py>, node_name: &str, node_decl: &PyNodeDecl, cache_values: &HashM...` — ```
-  `parse_graph_dict` function L861-906 — `(graph: &Bound<'_, PyDict>) -> PyResult<Vec<PyNodeDecl>>` — ```
-  `compute_execution_order` function L908-967 — `(nodes: &[PyNodeDecl]) -> Vec<String>` — ```

#### crates/cloacina/src/python/computation_graph_tests.rs

-  `tests` module L23-560 — `-` — Tests for the Python computation graph bindings.
-  `define_graph_and_get_executor` function L32-56 — `( py: Python<'_>, graph_name: &str, python_code: &std::ffi::CStr, )` — Helper: run a Python script that defines a computation graph using the
-  `test_linear_graph_via_builder` function L60-93 — `()` — WorkflowBuilder + @task pattern.
-  `test_routing_graph_via_builder` function L97-139 — `()` — WorkflowBuilder + @task pattern.
-  `test_missing_node_errors` function L143-185 — `()` — WorkflowBuilder + @task pattern.
-  `test_orphan_node_errors` function L189-233 — `()` — WorkflowBuilder + @task pattern.
-  `test_linear_graph_executes` function L237-298 — `()` — WorkflowBuilder + @task pattern.
-  `test_routing_graph_executes_signal_path` function L302-381 — `()` — WorkflowBuilder + @task pattern.
-  `setup_accumulator_env` function L388-420 — `(py: Python<'_>) -> Bound<'_, pyo3::types::PyDict>` — Helper: set up Python environment with accumulator decorators available.
-  `test_passthrough_accumulator_decorator` function L424-456 — `()` — WorkflowBuilder + @task pattern.
-  `test_stream_accumulator_decorator` function L460-491 — `()` — WorkflowBuilder + @task pattern.
-  `test_polling_accumulator_decorator` function L495-522 — `()` — WorkflowBuilder + @task pattern.
-  `test_batch_accumulator_decorator` function L526-559 — `()` — WorkflowBuilder + @task pattern.

#### crates/cloacina/src/python/context.rs

- pub `PyContext` struct L25-27 — `{ inner: crate::Context<serde_json::Value> }` — PyContext - Python wrapper for Rust Context<serde_json::Value>
- pub `new` function L34-51 — `(data: Option<&Bound<'_, PyDict>>) -> PyResult<Self>` — Creates a new empty context
- pub `get` function L55-63 — `(&self, key: &str, default: Option<&Bound<'_, PyAny>>) -> PyResult<PyObject>` — Gets a value from the context
- pub `set` function L66-80 — `(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()>` — Sets a value in the context (insert or update)
- pub `update` function L83-88 — `(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()>` — Updates an existing value in the context
- pub `insert` function L91-96 — `(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()>` — Inserts a new value into the context
- pub `remove` function L99-104 — `(&mut self, key: &str) -> PyResult<Option<PyObject>>` — Removes and returns a value from the context
- pub `to_dict` function L107-109 — `(&self, py: Python<'_>) -> PyResult<PyObject>` — Returns the context as a Python dictionary
- pub `update_from_dict` function L112-130 — `(&mut self, data: &Bound<'_, PyDict>) -> PyResult<()>` — Updates the context with values from a Python dictionary
- pub `to_json` function L133-140 — `(&self) -> PyResult<String>` — Serializes the context to a JSON string
- pub `from_json` function L144-152 — `(json_str: &str) -> PyResult<Self>` — Creates a context from a JSON string
- pub `__len__` function L155-157 — `(&self) -> usize` — Returns the number of key-value pairs in the context
- pub `__contains__` function L160-162 — `(&self, key: &str) -> bool` — Checks if a key exists in the context
- pub `__repr__` function L165-170 — `(&self) -> String` — String representation of the context
- pub `__getitem__` function L173-185 — `(&self, key: &str) -> PyResult<PyObject>` — Dictionary-style item access
- pub `__setitem__` function L188-190 — `(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()>` — Dictionary-style item assignment
- pub `__delitem__` function L193-201 — `(&mut self, key: &str) -> PyResult<()>` — Dictionary-style item deletion
- pub `from_rust_context` function L206-208 — `(context: crate::Context<serde_json::Value>) -> Self` — Create a PyContext from a Rust Context (for internal use)
- pub `into_inner` function L211-213 — `(self) -> crate::Context<serde_json::Value>` — Extract the inner Rust Context (for internal use)
- pub `clone_inner` function L216-218 — `(&self) -> crate::Context<serde_json::Value>` — Clone the inner Rust Context (for internal use)
- pub `get_data_clone` function L221-223 — `(&self) -> std::collections::HashMap<String, serde_json::Value>` — Get a clone of the context data as a HashMap (for internal use)
-  `PyContext` type L30-202 — `= PyContext`
-  `PyContext` type L204-224 — `= PyContext`
-  `PyContext` type L227-238 — `impl Clone for PyContext` — Manual implementation of Clone since Context<T> doesn't implement Clone
-  `clone` function L228-237 — `(&self) -> Self`
-  `tests` module L241-433 — `-`
-  `test_new_empty` function L246-250 — `()`
-  `test_new_from_dict` function L253-262 — `()`
-  `test_set_and_get` function L265-276 — `()`
-  `test_insert_new_key` function L279-287 — `()`
-  `test_insert_duplicate_errors` function L290-299 — `()`
-  `test_update_existing_key` function L302-315 — `()`
-  `test_update_missing_key_errors` function L318-325 — `()`
-  `test_remove_existing` function L328-339 — `()`
-  `test_remove_missing_returns_none` function L342-347 — `()`
-  `test_len_and_contains` function L350-362 — `()`
-  `test_to_json_and_from_json` function L365-381 — `()`
-  `test_to_dict` function L384-396 — `()`
-  `test_repr` function L399-404 — `()`
-  `test_from_rust_context_and_clone_inner` function L407-418 — `()`
-  `test_clone_preserves_data` function L421-432 — `()`

#### crates/cloacina/src/python/executor.rs

- pub `PythonExecutionError` enum L28-56 — `EnvironmentSetup | TaskNotFound | TaskException | SerializationError | ImportErr...` — Errors that can occur during Python task execution.
- pub `PythonTaskResult` struct L60-65 — `{ task_id: String, output_json: String }` — Result of executing a Python task.
- pub `PythonTaskExecutor` interface L79-108 — `{ fn execute_task(), fn discover_tasks() }` — Trait for executing Python tasks from extracted packages.
-  `tests` module L111-209 — `-` — crate provides the concrete implementation.
-  `MockPythonExecutor` struct L115-117 — `{ task_ids: Vec<String> }` — A mock executor for testing without PyO3.
-  `MockPythonExecutor` type L120-149 — `impl PythonTaskExecutor for MockPythonExecutor` — crate provides the concrete implementation.
-  `execute_task` function L121-139 — `( &self, _workflow_dir: &Path, _vendor_dir: &Path, _entry_module: &str, task_id:...` — crate provides the concrete implementation.
-  `discover_tasks` function L141-148 — `( &self, _workflow_dir: &Path, _vendor_dir: &Path, _entry_module: &str, ) -> Res...` — crate provides the concrete implementation.
-  `test_mock_executor_discover` function L152-161 — `()` — crate provides the concrete implementation.
-  `test_mock_executor_execute` function L164-180 — `()` — crate provides the concrete implementation.
-  `test_mock_executor_task_not_found` function L183-196 — `()` — crate provides the concrete implementation.
-  `test_error_display` function L199-208 — `()` — crate provides the concrete implementation.

#### crates/cloacina/src/python/loader.rs

- pub `PythonLoaderError` enum L69-81 — `ImportError | ValidationError | RegistrationError | RuntimeError` — Error type for Python package loading operations.
- pub `ensure_cloaca_module` function L94-157 — `(py: Python) -> PyResult<()>` — Ensure the `cloaca` Python module is available in the embedded interpreter.
- pub `validate_no_stdlib_shadowing` function L183-207 — `( workflow_dir: &Path, vendor_dir: &Path, ) -> Result<(), PythonLoaderError>` — Import a Python workflow module and register its tasks.
- pub `import_and_register_python_workflow` function L209-225 — `( workflow_dir: &Path, vendor_dir: &Path, entry_module: &str, package_name: &str...` — cloacina task execution engine.
- pub `import_and_register_python_workflow_named` function L227-380 — `( workflow_dir: &Path, vendor_dir: &Path, entry_module: &str, package_name: &str...` — cloacina task execution engine.
- pub `import_python_computation_graph` function L388-465 — `( workflow_dir: &Path, vendor_dir: &Path, entry_module: &str, graph_name: &str, ...` — Import a Python computation graph module and return the graph name.
-  `IMPORT_TIMEOUT_SECS` variable L35 — `: u64` — Default timeout for Python module import (seconds).
-  `STDLIB_DENY_LIST` variable L39-65 — `: &[&str]` — Python stdlib module names that must never appear in extracted packages.
-  `PythonLoaderError` type L83-87 — `= PythonLoaderError` — cloacina task execution engine.
-  `from` function L84-86 — `(err: PyErr) -> Self` — cloacina task execution engine.
-  `py_var` function L470-472 — `(name: &str) -> PyResult<String>` — Python binding: `cloaca.var(name)` — resolve a `CLOACINA_VAR_{NAME}` env var.
-  `py_var_or` function L477-479 — `(name: &str, default: &str) -> String` — Python binding: `cloaca.var_or(name, default)` — resolve with a fallback.

#### crates/cloacina/src/python/mod.rs

- pub `computation_graph` module L29 — `-` — `#[pymodule]` definition.
- pub `executor` module L34 — `-` — `#[pymodule]` definition.
- pub `context` module L37 — `-` — `#[pymodule]` definition.
- pub `loader` module L38 — `-` — `#[pymodule]` definition.
- pub `namespace` module L39 — `-` — `#[pymodule]` definition.
- pub `task` module L40 — `-` — `#[pymodule]` definition.
- pub `trigger` module L41 — `-` — `#[pymodule]` definition.
- pub `workflow` module L42 — `-` — `#[pymodule]` definition.
- pub `workflow_context` module L43 — `-` — `#[pymodule]` definition.
- pub `bindings` module L71 — `-` — `#[pymodule]` definition.
-  `computation_graph_tests` module L31 — `-` — `#[pymodule]` definition.
-  `tests` module L74-291 — `-` — `#[pymodule]` definition.
-  `test_python_workflow_via_with_gil` function L80-126 — `()` — `#[pymodule]` definition.
-  `test_ensure_cloaca_module_registers_in_sys_modules` function L129-159 — `()` — `#[pymodule]` definition.
-  `test_cloaca_var_and_var_or_from_python` function L162-218 — `()` — `#[pymodule]` definition.
-  `test_cloaca_cg_decorators_are_callable` function L221-255 — `()` — `#[pymodule]` definition.
-  `test_validate_no_stdlib_shadowing_rejects_os_py` function L258-274 — `()` — `#[pymodule]` definition.
-  `test_validate_no_stdlib_shadowing_allows_normal_packages` function L277-290 — `()` — `#[pymodule]` definition.

#### crates/cloacina/src/python/namespace.rs

- pub `PyTaskNamespace` struct L23-25 — `{ inner: crate::TaskNamespace }` — Python wrapper for TaskNamespace
- pub `new` function L31-35 — `(tenant_id: &str, package_name: &str, workflow_id: &str, task_id: &str) -> Self` — Create a new TaskNamespace
- pub `from_string` function L39-43 — `(namespace_str: &str) -> PyResult<Self>` — Parse TaskNamespace from string format "tenant::package::workflow::task"
- pub `tenant_id` function L47-49 — `(&self) -> &str` — Get tenant ID
- pub `package_name` function L53-55 — `(&self) -> &str` — Get package name
- pub `workflow_id` function L59-61 — `(&self) -> &str` — Get workflow ID
- pub `task_id` function L65-67 — `(&self) -> &str` — Get task ID
- pub `parent` function L70-79 — `(&self) -> Self` — Get parent namespace (without task_id)
- pub `is_child_of` function L82-88 — `(&self, parent: &PyTaskNamespace) -> bool` — Check if this namespace is a child of another
- pub `is_sibling_of` function L91-98 — `(&self, other: &PyTaskNamespace) -> bool` — Check if this namespace is a sibling of another (same parent)
- pub `__str__` function L101-103 — `(&self) -> String` — String representation
- pub `__repr__` function L106-114 — `(&self) -> String` — String representation
- pub `__eq__` function L117-119 — `(&self, other: &PyTaskNamespace) -> bool` — Equality comparison
- pub `__hash__` function L122-129 — `(&self) -> u64` — Hash for use in sets/dicts
- pub `from_rust` function L134-136 — `(namespace: crate::TaskNamespace) -> Self` — Convert from Rust TaskNamespace (for internal use)
- pub `to_rust` function L139-141 — `(&self) -> crate::TaskNamespace` — Convert to Rust TaskNamespace (for internal use)
-  `PyTaskNamespace` type L28-130 — `= PyTaskNamespace`
-  `PyTaskNamespace` type L132-142 — `= PyTaskNamespace`
-  `tests` module L145-247 — `-`
-  `test_new_and_getters` function L149-156 — `()`
-  `test_from_string_valid` function L159-166 — `()`
-  `test_from_string_invalid` function L169-174 — `()`
-  `test_parent` function L177-185 — `()`
-  `test_is_child_of` function L188-198 — `()`
-  `test_is_sibling_of` function L201-211 — `()`
-  `test_str_and_repr` function L214-219 — `()`
-  `test_eq` function L222-229 — `()`
-  `test_hash_consistency` function L232-237 — `()`
-  `test_from_rust_to_rust_roundtrip` function L240-246 — `()`

#### crates/cloacina/src/python/task.rs

- pub `PyTaskHandle` struct L27-29 — `{ inner: Option<crate::TaskHandle> }` — Python wrapper for TaskHandle providing defer_until capability.
- pub `defer_until` function L35-69 — `( &mut self, py: Python, condition: PyObject, poll_interval_ms: u64, ) -> PyResu...` — Release the concurrency slot while polling an external condition.
- pub `is_slot_held` function L72-78 — `(&self) -> PyResult<bool>` — Returns whether the handle currently holds a concurrency slot.
- pub `WorkflowBuilderRef` struct L83-85 — `{ context: PyWorkflowContext }` — Workflow builder reference for automatic task registration
- pub `push_workflow_context` function L91-95 — `(context: PyWorkflowContext)` — Push a workflow context onto the stack (called when entering workflow scope)
- pub `pop_workflow_context` function L98-100 — `() -> Option<WorkflowBuilderRef>` — Pop a workflow context from the stack (called when exiting workflow scope)
- pub `current_workflow_context` function L103-110 — `() -> PyResult<PyWorkflowContext>` — Get the current workflow context (used by task decorator)
- pub `PythonTaskWrapper` struct L113-121 — `{ id: String, dependencies: Vec<crate::TaskNamespace>, retry_policy: crate::retr...` — Python task wrapper implementing Rust Task trait
- pub `TaskDecorator` struct L345-351 — `{ id: Option<String>, dependencies: Vec<PyObject>, retry_policy: crate::retry::R...` — Decorator class that holds task configuration
- pub `__call__` function L355-424 — `(&self, py: Python, func: PyObject) -> PyResult<PyObject>`
- pub `task` function L502-530 — `( id: Option<String>, dependencies: Option<Vec<PyObject>>, retry_attempts: Optio...`
-  `PyTaskHandle` type L32-79 — `= PyTaskHandle`
-  `WORKFLOW_CONTEXT_STACK` variable L88 — `: Mutex<Vec<WorkflowBuilderRef>>` — Global context stack for workflow-scoped task registration
-  `PythonTaskWrapper` type L129 — `impl Send for PythonTaskWrapper`
-  `PythonTaskWrapper` type L130 — `impl Sync for PythonTaskWrapper`
-  `PythonTaskWrapper` type L133-285 — `= PythonTaskWrapper`
-  `execute` function L134-253 — `( &self, context: crate::Context<serde_json::Value>, ) -> Result<crate::Context<...`
-  `id` function L255-257 — `(&self) -> &str`
-  `dependencies` function L259-261 — `(&self) -> &[crate::TaskNamespace]`
-  `retry_policy` function L263-265 — `(&self) -> crate::retry::RetryPolicy`
-  `requires_handle` function L267-269 — `(&self) -> bool`
-  `checkpoint` function L271-276 — `( &self, _context: &crate::Context<serde_json::Value>, ) -> Result<(), crate::Ch...`
-  `trigger_rules` function L278-280 — `(&self) -> serde_json::Value`
-  `code_fingerprint` function L282-284 — `(&self) -> Option<String>`
-  `build_retry_policy` function L288-341 — `( retry_attempts: Option<usize>, retry_backoff: Option<String>, retry_delay_ms: ...` — Build retry policy from Python decorator parameters
-  `TaskDecorator` type L354-425 — `= TaskDecorator`
-  `TaskDecorator` type L427-484 — `= TaskDecorator`
-  `convert_dependencies_to_namespaces` function L429-483 — `( &self, py: Python, context: &PyWorkflowContext, ) -> PyResult<Vec<crate::TaskN...` — Convert mixed dependencies (strings and function objects) to TaskNamespace objects

#### crates/cloacina/src/python/trigger.rs

- pub `PythonTriggerDef` struct L40-45 — `{ name: String, poll_interval: Duration, allow_concurrent: bool, python_function...` — A collected Python trigger definition.
- pub `drain_python_triggers` function L48-51 — `() -> Vec<PythonTriggerDef>` — Collect all registered Python triggers and clear the registry.
- pub `PyTriggerResult` struct L66-71 — `{ should_fire: bool, context: Option<PyObject> }` — Python-side trigger result returned from poll functions.
- pub `TriggerDecorator` struct L102-106 — `{ name: Option<String>, poll_interval: Duration, allow_concurrent: bool }` — Decorator for defining Python triggers.
- pub `__call__` function L110-130 — `(&self, py: Python, func: PyObject) -> PyResult<PyObject>` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
- pub `trigger` function L136-150 — `( name: Option<String>, poll_interval: String, allow_concurrent: bool, ) -> PyRe...` — `@cloaca.trigger(...)` decorator factory.
- pub `PythonTriggerWrapper` struct L153-158 — `{ name: String, poll_interval: Duration, allow_concurrent: bool, python_function...` — Rust wrapper that implements the `Trigger` trait by calling a Python function.
- pub `new` function L167-175 — `(def: &PythonTriggerDef) -> Self` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `PYTHON_TRIGGER_REGISTRY` variable L37 — `: Mutex<Vec<PythonTriggerDef>>` — Global registry of Python trigger definitions collected during module import.
-  `PyTriggerResult` type L74-91 — `= PyTriggerResult` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `new` function L77-82 — `(should_fire: bool, context: Option<PyObject>) -> Self` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `__repr__` function L84-90 — `(&self) -> String` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `TriggerDecorator` type L109-131 — `= TriggerDecorator` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `PythonTriggerWrapper` type L163 — `impl Send for PythonTriggerWrapper` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `PythonTriggerWrapper` type L164 — `impl Sync for PythonTriggerWrapper` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `PythonTriggerWrapper` type L166-176 — `= PythonTriggerWrapper` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `PythonTriggerWrapper` type L178-185 — `= PythonTriggerWrapper` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `fmt` function L179-184 — `(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `PythonTriggerWrapper` type L188-274 — `impl Trigger for PythonTriggerWrapper` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `name` function L189-191 — `(&self) -> &str` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `poll_interval` function L193-195 — `(&self) -> Duration` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `allow_concurrent` function L197-199 — `(&self) -> bool` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `poll` function L201-273 — `(&self) -> Result<RustTriggerResult, TriggerError>` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `tests` module L277-400 — `-` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `test_trigger_decorator_registers` function L282-300 — `()` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `test_trigger_decorator_uses_function_name` function L303-320 — `()` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `test_py_trigger_result_creation` function L323-333 — `()` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `test_python_trigger_wrapper_skip` function L336-354 — `()` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `test_python_trigger_wrapper_fire` function L357-373 — `()` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait
-  `test_python_trigger_wrapper_exception_handled` function L376-399 — `()` — - `PythonTriggerWrapper` implementing the Rust `Trigger` trait

#### crates/cloacina/src/python/workflow.rs

- pub `PyWorkflowBuilder` struct L24-27 — `{ inner: crate::WorkflowBuilder, context: PyWorkflowContext }` — Python wrapper for WorkflowBuilder
- pub `new` function L34-53 — `( name: &str, tenant: Option<&str>, package: Option<&str>, workflow: Option<&str...` — Create a new WorkflowBuilder with namespace context
- pub `description` function L56-58 — `(&mut self, description: &str)` — Set the workflow description
- pub `tag` function L61-63 — `(&mut self, key: &str, value: &str)` — Add a tag to the workflow
- pub `add_task` function L66-147 — `(&mut self, py: Python, task: PyObject) -> PyResult<()>` — Add a task to the workflow by ID or function reference
- pub `build` function L150-157 — `(&self) -> PyResult<PyWorkflow>` — Build the workflow
- pub `__enter__` function L160-163 — `(slf: PyRef<Self>) -> PyRef<Self>` — Context manager entry - establish workflow context for task decorators
- pub `__exit__` function L166-217 — `( &mut self, _py: Python, _exc_type: Option<&Bound<PyAny>>, _exc_value: Option<&...` — Context manager exit - clean up context and build workflow
- pub `__repr__` function L220-222 — `(&self) -> String` — String representation
- pub `PyWorkflow` struct L228-230 — `{ inner: crate::Workflow }` — Python wrapper for Workflow
- pub `name` function L236-238 — `(&self) -> &str` — Get workflow name
- pub `description` function L242-248 — `(&self) -> String` — Get workflow description
- pub `version` function L252-254 — `(&self) -> &str` — Get workflow version
- pub `topological_sort` function L257-262 — `(&self) -> PyResult<Vec<String>>` — Get topological sort of tasks
- pub `get_execution_levels` function L265-275 — `(&self) -> PyResult<Vec<Vec<String>>>` — Get execution levels (tasks that can run in parallel)
- pub `get_roots` function L278-284 — `(&self) -> Vec<String>` — Get root tasks (no dependencies)
- pub `get_leaves` function L287-293 — `(&self) -> Vec<String>` — Get leaf tasks (no dependents)
- pub `validate` function L296-300 — `(&self) -> PyResult<()>` — Validate the workflow
- pub `__repr__` function L303-309 — `(&self) -> String` — String representation
- pub `register_workflow_constructor` function L398-416 — `(name: String, constructor: PyObject) -> PyResult<()>` — Register a workflow constructor function
-  `PyWorkflowBuilder` type L30-223 — `= PyWorkflowBuilder`
-  `PyWorkflow` type L233-310 — `= PyWorkflow`
-  `tests` module L313-394 — `-`
-  `test_workflow_builder_new_defaults` function L317-322 — `()`
-  `test_workflow_builder_new_with_custom_namespace` function L325-335 — `()`
-  `test_workflow_builder_description_and_tag` function L338-346 — `()`
-  `test_workflow_builder_build_empty_returns_error` function L349-354 — `()`
-  `test_workflow_builder_build_with_task` function L357-393 — `()`

#### crates/cloacina/src/python/workflow_context.rs

- pub `PyWorkflowContext` struct L23-27 — `{ tenant_id: String, package_name: String, workflow_id: String }` — WorkflowContext provides namespace management for Python workflows
- pub `new` function L33-39 — `(tenant_id: &str, package_name: &str, workflow_id: &str) -> Self` — Create a new WorkflowContext
- pub `tenant_id` function L43-45 — `(&self) -> &str` — Get tenant ID
- pub `package_name` function L49-51 — `(&self) -> &str` — Get package name
- pub `workflow_id` function L55-57 — `(&self) -> &str` — Get workflow ID
- pub `task_namespace` function L60-67 — `(&self, task_id: &str) -> PyTaskNamespace` — Generate a TaskNamespace for a task within this workflow context
- pub `resolve_dependency` function L70-72 — `(&self, task_name: &str) -> PyTaskNamespace` — Resolve a dependency task name to a full TaskNamespace within this context
- pub `workflow_namespace` function L75-82 — `(&self) -> PyTaskNamespace` — Get the workflow namespace (without task_id)
- pub `contains_namespace` function L85-89 — `(&self, namespace: &PyTaskNamespace) -> bool` — Check if a namespace belongs to this workflow context
- pub `__str__` function L92-97 — `(&self) -> String` — String representation
- pub `__repr__` function L100-105 — `(&self) -> String` — String representation
- pub `__eq__` function L108-112 — `(&self, other: &PyWorkflowContext) -> bool` — Equality comparison
- pub `default` function L118-124 — `() -> Self` — Get the default workflow context (for backward compatibility)
- pub `as_components` function L127-129 — `(&self) -> (&str, &str, &str)` — Convert to namespace components
-  `PyWorkflowContext` type L30-113 — `= PyWorkflowContext`
-  `PyWorkflowContext` type L115-130 — `= PyWorkflowContext`

### crates/cloacina/src/registry

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/registry/error.rs

- pub `RegistryError` enum L30-115 — `PackageExists | PackageNotFound | PackageInUse | ValidationError | MetadataExtra...` — Main error type for registry operations.
- pub `StorageError` enum L122-169 — `ConnectionFailed | Timeout | QuotaExceeded | DataCorruption | InvalidId | Backen...` — Error type for storage backend operations.
- pub `LoaderError` enum L188-271 — `TempDirectory | LibraryLoad | SymbolNotFound | MetadataExtraction | FileSystem |...` — Error type for package loading and metadata extraction operations.
-  `RegistryError` type L171-175 — `= RegistryError` — and user feedback.
-  `from` function L172-174 — `(s: String) -> Self` — and user feedback.
-  `StorageError` type L177-181 — `= StorageError` — and user feedback.
-  `from` function L178-180 — `(s: String) -> Self` — and user feedback.

#### crates/cloacina/src/registry/mod.rs

- pub `error` module L66 — `-` — # Workflow Registry
- pub `loader` module L67 — `-` — ```
- pub `reconciler` module L68 — `-` — ```
- pub `storage` module L69 — `-` — ```
- pub `traits` module L70 — `-` — ```
- pub `types` module L71 — `-` — ```
- pub `workflow_registry` module L72 — `-` — ```

#### crates/cloacina/src/registry/traits.rs

- pub `WorkflowRegistry` interface L64-160 — `{ fn register_workflow(), fn get_workflow(), fn list_workflows(), fn unregister_...` — Main trait for workflow registry operations.
- pub `RegistryStorage` interface L195-253 — `{ fn store_binary(), fn retrieve_binary(), fn delete_binary(), fn storage_type()...` — Trait for binary storage backends.

#### crates/cloacina/src/registry/types.rs

- pub `WorkflowPackageId` type L30 — `= Uuid` — Unique identifier for a workflow package.
- pub `WorkflowMetadata` struct L59-89 — `{ id: WorkflowPackageId, registry_id: Uuid, package_name: String, version: Strin...` — Metadata for a registered workflow package.
- pub `PackageMetadata` struct L96-117 — `{ package: String, version: String, description: Option<String>, author: Option<...` — Package metadata extracted from a .cloacina file.
- pub `BuildInfo` struct L121-133 — `{ rustc_version: String, cloacina_version: String, build_timestamp: DateTime<Utc...` — Build information embedded in the package.
- pub `TaskInfo` struct L137-146 — `{ id: String, dependencies: Vec<String>, description: Option<String> }` — Basic task information from package metadata.
- pub `ScheduleInfo` struct L150-159 — `{ name: String, cron: String, workflow: String }` — Schedule information from package metadata.
- pub `WorkflowPackage` struct L166-172 — `{ metadata: PackageMetadata, package_data: Vec<u8> }` — A workflow package ready for registration.
- pub `new` function L176-181 — `(metadata: PackageMetadata, package_data: Vec<u8>) -> Self` — Create a new workflow package from metadata and binary data.
- pub `from_file` function L208-211 — `(_path: impl AsRef<std::path::Path>) -> Result<Self, std::io::Error>` — Load a workflow package from a .cloacina file.
- pub `LoadedWorkflow` struct L219-225 — `{ metadata: WorkflowMetadata, package_data: Vec<u8> }` — A loaded workflow with both metadata and binary data.
- pub `new` function L229-234 — `(metadata: WorkflowMetadata, package_data: Vec<u8>) -> Self` — Create a new loaded workflow.
-  `WorkflowPackage` type L174-212 — `= WorkflowPackage` — including workflow metadata, package information, and identifiers.
-  `LoadedWorkflow` type L227-235 — `= LoadedWorkflow` — including workflow metadata, package information, and identifiers.

### crates/cloacina/src/registry/loader

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/registry/loader/mod.rs

- pub `package_loader` module L23 — `-` — Package loader module for workflow registry.
- pub `python_loader` module L24 — `-` — global task registry.
- pub `task_registrar` module L25 — `-` — global task registry.
- pub `validator` module L26 — `-` — global task registry.

#### crates/cloacina/src/registry/loader/package_loader.rs

- pub `get_library_extension` function L30-38 — `() -> &'static str` — Get the platform-specific dynamic library extension.
- pub `PackageMetadata` struct L42-59 — `{ package_name: String, version: String, description: Option<String>, author: Op...` — Metadata extracted from a workflow package.
- pub `TaskMetadata` struct L63-76 — `{ index: u32, local_id: String, namespaced_id_template: String, dependencies: Ve...` — Individual task metadata.
- pub `PluginHandleCache` type L93 — `= std::sync::Arc<std::sync::Mutex<Vec<fidius_host::PluginHandle>>>` — Package loader for extracting metadata from workflow library files.
- pub `PackageLoader` struct L95-99 — `{ temp_dir: TempDir, handle_cache: PluginHandleCache }` — via the fidius-host plugin API and extract package metadata.
- pub `new` function L103-112 — `() -> Result<Self, LoaderError>` — Create a new package loader with a temporary directory for safe operations.
- pub `with_handle_cache` function L115-124 — `(cache: PluginHandleCache) -> Result<Self, LoaderError>` — Create a package loader with a shared handle cache.
- pub `handle_cache` function L127-129 — `(&self) -> PluginHandleCache` — Get the shared handle cache (for passing to TaskRegistrar).
- pub `extract_metadata` function L180-198 — `( &self, package_data: &[u8], ) -> Result<PackageMetadata, LoaderError>` — Extract metadata from compiled library bytes.
- pub `extract_graph_metadata` function L312-365 — `( &self, package_data: &[u8], ) -> Result<Option<cloacina_workflow_plugin::Graph...` — Extract computation graph metadata from compiled library bytes.
- pub `temp_dir` function L368-370 — `(&self) -> &Path` — Get the temporary directory path for manual file operations.
- pub `validate_package_symbols` function L376-405 — `( &self, package_data: &[u8], ) -> Result<Vec<String>, LoaderError>` — Validate that a package has the required symbols by loading it via fidius-host.
-  `PackageLoader` type L101-406 — `= PackageLoader` — via the fidius-host plugin API and extract package metadata.
-  `generate_graph_data_from_tasks` function L132-166 — `( &self, tasks: &[TaskMetadata], ) -> Result<serde_json::Value, LoaderError>` — Generate graph data from task dependencies.
-  `extract_metadata_from_so` function L203-244 — `( &self, library_path: &Path, ) -> Result<PackageMetadata, LoaderError>` — Extract metadata from a library file using the fidius-host plugin API.
-  `convert_plugin_metadata_to_rust` function L248-306 — `( &self, meta: cloacina_workflow_plugin::PackageTasksMetadata, ) -> Result<Packa...` — Convert `PackageTasksMetadata` from the fidius plugin into the `PackageMetadata`
-  `PackageLoader` type L408-412 — `impl Default for PackageLoader` — via the fidius-host plugin API and extract package metadata.
-  `default` function L409-411 — `() -> Self` — via the fidius-host plugin API and extract package metadata.
-  `tests` module L415-636 — `-` — via the fidius-host plugin API and extract package metadata.
-  `create_invalid_binary_data` function L419-421 — `() -> Vec<u8>` — Helper to create invalid binary data
-  `create_mock_elf_data` function L424-440 — `(size: usize) -> Vec<u8>` — Helper to create a mock ELF-like binary for testing
-  `test_package_loader_creation` function L443-447 — `()` — via the fidius-host plugin API and extract package metadata.
-  `test_package_loader_default` function L450-453 — `()` — via the fidius-host plugin API and extract package metadata.
-  `test_extract_metadata_with_invalid_elf` function L456-472 — `()` — via the fidius-host plugin API and extract package metadata.
-  `test_extract_metadata_with_empty_data` function L475-486 — `()` — via the fidius-host plugin API and extract package metadata.
-  `test_extract_metadata_with_large_invalid_data` function L489-500 — `()` — via the fidius-host plugin API and extract package metadata.
-  `test_validate_package_symbols_with_invalid_data` function L503-514 — `()` — via the fidius-host plugin API and extract package metadata.
-  `test_validate_package_symbols_with_empty_data` function L517-524 — `()` — via the fidius-host plugin API and extract package metadata.
-  `test_temp_dir_isolation` function L527-534 — `()` — via the fidius-host plugin API and extract package metadata.
-  `test_concurrent_package_loading` function L537-561 — `()` — via the fidius-host plugin API and extract package metadata.
-  `test_file_system_operations` function L564-573 — `()` — via the fidius-host plugin API and extract package metadata.
-  `test_error_types_and_messages` function L576-594 — `()` — via the fidius-host plugin API and extract package metadata.
-  `test_package_loader_memory_safety` function L597-603 — `()` — via the fidius-host plugin API and extract package metadata.
-  `test_temp_directory_cleanup` function L606-613 — `()` — via the fidius-host plugin API and extract package metadata.
-  `test_package_loader_sync_creation` function L616-622 — `()` — via the fidius-host plugin API and extract package metadata.
-  `test_get_library_extension` function L625-635 — `()` — via the fidius-host plugin API and extract package metadata.

#### crates/cloacina/src/registry/loader/python_loader.rs

- pub `ExtractedPythonPackage` struct L29-44 — `{ root_dir: PathBuf, vendor_dir: PathBuf, workflow_dir: PathBuf, entry_module: S...` — An extracted Python package ready for task execution.
- pub `PackageKind` enum L47-60 — `Python | Rust` — Result of detecting the package language from a source archive.
- pub `detect_package_kind` function L66-119 — `(archive_data: &[u8]) -> Result<PackageKind, LoaderError>` — Detect the package kind (Python or Rust) from a `.cloacina` source archive.
- pub `extract_python_package` function L126-200 — `( archive_data: &[u8], staging_dir: &Path, ) -> Result<ExtractedPythonPackage, L...` — Extract a Python workflow package from a `.cloacina` source archive.
-  `tests` module L203-325 — `-` — for task execution via PyO3.
-  `create_python_source_package` function L208-250 — `( dir: &Path, name: &str, include_workflow: bool, ) -> std::path::PathBuf` — Create a fidius source package directory for a Python workflow.
-  `test_detect_package_kind_python` function L253-262 — `()` — for task execution via PyO3.
-  `test_extract_python_package` function L265-280 — `()` — for task execution via PyO3.
-  `test_extract_missing_workflow_dir` function L283-293 — `()` — for task execution via PyO3.
-  `test_wrong_language_rejected` function L296-324 — `()` — for task execution via PyO3.

### crates/cloacina/src/registry/loader/task_registrar

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/registry/loader/task_registrar/dynamic_task.rs

-  `LoadedWorkflowPlugin` struct L35-39 — `{ handle: std::sync::Mutex<fidius_host::PluginHandle>, _temp_dir: tempfile::Temp...` — A persistent handle to a loaded workflow plugin library.
-  `LoadedWorkflowPlugin` type L43 — `impl Send for LoadedWorkflowPlugin` — temp files or dlopen/dlclose cycles.
-  `LoadedWorkflowPlugin` type L44 — `impl Sync for LoadedWorkflowPlugin` — temp files or dlopen/dlclose cycles.
-  `LoadedWorkflowPlugin` type L46-102 — `= LoadedWorkflowPlugin` — temp files or dlopen/dlclose cycles.
-  `load` function L48-90 — `(library_data: &[u8], package_name: &str) -> Result<Self, TaskError>` — Load a workflow plugin from library bytes.
-  `execute_task` function L93-101 — `(&self, request: TaskExecutionRequest) -> Result<TaskExecutionResult, String>` — Call execute_task (method index 1) on the loaded plugin.
-  `LoadedWorkflowPlugin` type L104-108 — `= LoadedWorkflowPlugin` — temp files or dlopen/dlclose cycles.
-  `fmt` function L105-107 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — temp files or dlopen/dlclose cycles.
-  `DynamicLibraryTask` struct L115-124 — `{ plugin: Arc<LoadedWorkflowPlugin>, task_name: String, package_name: String, de...` — A task implementation that executes via the fidius plugin API.
-  `DynamicLibraryTask` type L126-149 — `= DynamicLibraryTask` — temp files or dlopen/dlclose cycles.
-  `load_plugin` function L128-133 — `( library_data: &[u8], package_name: &str, ) -> Result<LoadedWorkflowPlugin, Tas...` — Load a plugin library from bytes.
-  `new` function L136-148 — `( plugin: Arc<LoadedWorkflowPlugin>, task_name: String, package_name: String, de...` — Create a new dynamic library task with a shared plugin handle.
-  `DynamicLibraryTask` type L152-251 — `impl Task for DynamicLibraryTask` — temp files or dlopen/dlclose cycles.
-  `execute` function L154-242 — `( &self, context: Context<serde_json::Value>, ) -> Result<Context<serde_json::Va...` — Execute the task using the pre-loaded plugin handle.
-  `id` function L244-246 — `(&self) -> &str` — temp files or dlopen/dlclose cycles.
-  `dependencies` function L248-250 — `(&self) -> &[TaskNamespace]` — temp files or dlopen/dlclose cycles.
-  `tests` module L254-263 — `-` — temp files or dlopen/dlclose cycles.
-  `test_loaded_workflow_plugin_debug` function L258-262 — `()` — temp files or dlopen/dlclose cycles.

#### crates/cloacina/src/registry/loader/task_registrar/extraction.rs

-  `TaskRegistrar` type L26-109 — `= TaskRegistrar` — Task metadata extraction from dynamic libraries via fidius-host.
-  `extract_task_metadata_from_library` function L34-108 — `( &self, package_data: &[u8], ) -> Result<OwnedTaskMetadataCollection, LoaderErr...` — Extract task metadata from a library using the fidius-host plugin API.

#### crates/cloacina/src/registry/loader/task_registrar/mod.rs

- pub `TaskRegistrar` struct L46-56 — `{ temp_dir: TempDir, registered_tasks: Arc<RwLock<HashMap<String, Vec<TaskNamesp...` — Task registrar for managing dynamically loaded package tasks.
- pub `new` function L60-71 — `() -> Result<Self, LoaderError>` — Create a new task registrar with a temporary directory for operations.
- pub `with_handle_cache` function L74-87 — `( cache: crate::registry::loader::package_loader::PluginHandleCache, ) -> Result...` — Create a task registrar with a shared handle cache.
- pub `register_package_tasks` function L102-209 — `( &self, package_id: &str, package_data: &[u8], _metadata: &PackageMetadata, ten...` — Register package tasks with the global task registry using new host-managed approach.
- pub `unregister_package_tasks` function L221-246 — `(&self, package_id: &str) -> Result<(), LoaderError>` — Unregister package tasks from the global registry.
- pub `get_registered_namespaces` function L249-252 — `(&self, package_id: &str) -> Vec<TaskNamespace>` — Get the list of task namespaces registered for a package.
- pub `loaded_package_count` function L255-258 — `(&self) -> usize` — Get the number of currently loaded packages.
- pub `total_registered_tasks` function L261-264 — `(&self) -> usize` — Get the total number of registered tasks across all packages.
- pub `temp_dir` function L267-269 — `(&self) -> &Path` — Get the temporary directory path for manual operations.
-  `dynamic_task` module L23 — `-` — Task registrar for integrating packaged workflow tasks with the global registry.
-  `extraction` module L24 — `-` — isolation and task lifecycle management.
-  `types` module L25 — `-` — isolation and task lifecycle management.
-  `TaskRegistrar` type L58-270 — `= TaskRegistrar` — isolation and task lifecycle management.
-  `TaskRegistrar` type L272-276 — `impl Default for TaskRegistrar` — isolation and task lifecycle management.
-  `default` function L273-275 — `() -> Self` — isolation and task lifecycle management.
-  `tests` module L279-571 — `-` — isolation and task lifecycle management.
-  `create_mock_package_metadata` function L284-306 — `(package_name: &str, task_count: usize) -> PackageMetadata` — Helper to create mock package metadata for testing
-  `create_mock_binary_data` function L309-312 — `() -> Vec<u8>` — Helper to create mock binary data (not a real .so file)
-  `test_task_registrar_creation` function L315-322 — `()` — isolation and task lifecycle management.
-  `test_task_registrar_default` function L325-329 — `()` — isolation and task lifecycle management.
-  `test_register_package_tasks_with_invalid_binary` function L332-349 — `()` — isolation and task lifecycle management.
-  `test_register_package_tasks_with_missing_symbols` function L352-372 — `()` — isolation and task lifecycle management.
-  `test_register_package_tasks_empty_metadata` function L375-386 — `()` — isolation and task lifecycle management.
-  `test_unregister_nonexistent_package` function L389-396 — `()` — isolation and task lifecycle management.
-  `test_get_registered_namespaces_empty` function L399-405 — `()` — isolation and task lifecycle management.
-  `test_registrar_metrics` function L408-424 — `()` — isolation and task lifecycle management.
-  `test_concurrent_registrar_operations` function L427-467 — `()` — isolation and task lifecycle management.
-  `test_temp_directory_isolation` function L470-478 — `()` — isolation and task lifecycle management.
-  `test_package_id_tracking` function L481-492 — `()` — isolation and task lifecycle management.
-  `test_tenant_isolation` function L495-511 — `()` — isolation and task lifecycle management.
-  `test_default_tenant` function L514-525 — `()` — isolation and task lifecycle management.
-  `test_large_package_metadata` function L528-541 — `()` — isolation and task lifecycle management.
-  `test_error_message_quality` function L544-560 — `()` — isolation and task lifecycle management.
-  `test_registrar_sync_creation` function L563-570 — `()` — isolation and task lifecycle management.

#### crates/cloacina/src/registry/loader/task_registrar/types.rs

- pub `OwnedTaskMetadata` struct L26-31 — `{ local_id: String, dependencies_json: String }` — Owned task metadata — safe to use after library is unloaded.
- pub `OwnedTaskMetadataCollection` struct L37-44 — `{ workflow_name: String, package_name: String, tasks: Vec<OwnedTaskMetadata> }` — Owned collection of task metadata — safe to use after library is unloaded.

### crates/cloacina/src/registry/loader/validator

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/registry/loader/validator/format.rs

-  `PackageValidator` type L26-90 — `= PackageValidator` — File format validation for dynamic libraries.
-  `validate_file_format` function L28-89 — `( &self, package_path: &Path, result: &mut ValidationResult, )` — Validate file format and basic structure.

#### crates/cloacina/src/registry/loader/validator/metadata.rs

-  `PackageValidator` type L26-93 — `= PackageValidator` — Package metadata validation.
-  `validate_metadata` function L28-92 — `( &self, metadata: &PackageMetadata, result: &mut ValidationResult, )` — Validate package metadata for consistency and safety.

#### crates/cloacina/src/registry/loader/validator/mod.rs

- pub `PackageValidator` struct L41-50 — `{ temp_dir: TempDir, strict_mode: bool, max_package_size: u64, required_symbols:...` — Comprehensive package validator
- pub `new` function L54-68 — `() -> Result<Self, LoaderError>` — Create a new package validator with default settings.
- pub `strict` function L71-75 — `() -> Result<Self, LoaderError>` — Create a validator with strict validation mode enabled.
- pub `with_max_size` function L78-81 — `(mut self, max_bytes: u64) -> Self` — Set the maximum allowed package size.
- pub `with_required_symbols` function L84-93 — `(mut self, symbols: I) -> Self` — Add additional required symbols for validation.
- pub `validate_package` function L106-160 — `( &self, package_data: &[u8], metadata: Option<&PackageMetadata>, ) -> Result<Va...` — Validate a package comprehensively.
- pub `temp_dir` function L163-165 — `(&self) -> &Path` — Get the temporary directory path.
- pub `is_strict_mode` function L168-170 — `(&self) -> bool` — Check if strict mode is enabled.
- pub `max_package_size` function L173-175 — `(&self) -> u64` — Get the maximum package size limit.
-  `format` module L23 — `-` — Package validator for ensuring workflow package safety and compatibility.
-  `metadata` module L24 — `-` — metadata verification, and compatibility testing.
-  `security` module L25 — `-` — metadata verification, and compatibility testing.
-  `size` module L26 — `-` — metadata verification, and compatibility testing.
-  `symbols` module L27 — `-` — metadata verification, and compatibility testing.
-  `types` module L28 — `-` — metadata verification, and compatibility testing.
-  `PackageValidator` type L52-176 — `= PackageValidator` — metadata verification, and compatibility testing.
-  `PackageValidator` type L178-183 — `impl Default for PackageValidator` — metadata verification, and compatibility testing.
-  `default` function L179-182 — `() -> Self` — metadata verification, and compatibility testing.
-  `tests` module L186-652 — `-` — metadata verification, and compatibility testing.
-  `create_valid_elf_header` function L191-219 — `() -> Vec<u8>` — Helper to create a valid ELF header for testing
-  `create_invalid_binary` function L222-224 — `() -> Vec<u8>` — Helper to create invalid binary data
-  `create_suspicious_binary` function L227-235 — `() -> Vec<u8>` — Helper to create binary with suspicious content
-  `create_mock_metadata` function L238-260 — `(package_name: &str, task_count: usize) -> PackageMetadata` — Helper to create mock package metadata
-  `test_validator_creation` function L263-269 — `()` — metadata verification, and compatibility testing.
-  `test_validator_default` function L272-276 — `()` — metadata verification, and compatibility testing.
-  `test_strict_validator` function L279-282 — `()` — metadata verification, and compatibility testing.
-  `test_validator_with_custom_max_size` function L285-289 — `()` — metadata verification, and compatibility testing.
-  `test_validator_with_required_symbols` function L292-299 — `()` — metadata verification, and compatibility testing.
-  `test_validate_empty_package` function L302-311 — `()` — metadata verification, and compatibility testing.
-  `test_validate_oversized_package` function L314-323 — `()` — metadata verification, and compatibility testing.
-  `test_validate_invalid_elf` function L326-340 — `()` — metadata verification, and compatibility testing.
-  `test_validate_valid_elf_header` function L343-356 — `()` — metadata verification, and compatibility testing.
-  `test_validate_suspicious_content` function L359-374 — `()` — metadata verification, and compatibility testing.
-  `test_validate_with_metadata` function L377-397 — `()` — metadata verification, and compatibility testing.
-  `test_validate_metadata_with_invalid_package_name` function L400-416 — `()` — metadata verification, and compatibility testing.
-  `test_validate_metadata_with_special_characters` function L419-434 — `()` — metadata verification, and compatibility testing.
-  `test_validate_metadata_with_duplicate_task_ids` function L437-455 — `()` — metadata verification, and compatibility testing.
-  `test_validate_metadata_with_no_tasks` function L458-473 — `()` — metadata verification, and compatibility testing.
-  `test_strict_mode_validation` function L476-488 — `()` — metadata verification, and compatibility testing.
-  `test_permissive_mode_with_warnings` function L491-503 — `()` — metadata verification, and compatibility testing.
-  `test_security_assessment_levels` function L506-524 — `()` — metadata verification, and compatibility testing.
-  `test_compatibility_info` function L527-541 — `()` — metadata verification, and compatibility testing.
-  `test_concurrent_validation` function L544-571 — `()` — metadata verification, and compatibility testing.
-  `test_memory_safety_with_large_packages` function L574-589 — `()` — metadata verification, and compatibility testing.
-  `test_temp_directory_isolation` function L592-600 — `()` — metadata verification, and compatibility testing.
-  `test_validation_result_serialization` function L603-613 — `()` — metadata verification, and compatibility testing.
-  `test_error_message_quality` function L616-633 — `()` — metadata verification, and compatibility testing.
-  `test_security_level_equality` function L636-641 — `()` — metadata verification, and compatibility testing.
-  `test_validator_sync_creation` function L644-651 — `()` — metadata verification, and compatibility testing.

#### crates/cloacina/src/registry/loader/validator/security.rs

-  `PackageValidator` type L25-94 — `= PackageValidator` — Security assessment for packages.
-  `assess_security` function L27-93 — `(&self, package_path: &Path, result: &mut ValidationResult)` — Perform security assessment of the package.

#### crates/cloacina/src/registry/loader/validator/size.rs

-  `PackageValidator` type L22-44 — `= PackageValidator` — Package size validation.
-  `validate_package_size` function L24-43 — `(&self, package_data: &[u8], result: &mut ValidationResult)` — Validate package size constraints.

#### crates/cloacina/src/registry/loader/validator/symbols.rs

-  `PackageValidator` type L25-71 — `= PackageValidator` — Symbol validation for dynamic libraries.
-  `validate_symbols` function L27-70 — `( &self, package_path: &Path, result: &mut ValidationResult, )` — Validate required symbols are present.

#### crates/cloacina/src/registry/loader/validator/types.rs

- pub `ValidationResult` struct L21-32 — `{ is_valid: bool, errors: Vec<String>, warnings: Vec<String>, security_level: Se...` — Package validation results
- pub `SecurityLevel` enum L36-45 — `Safe | Warning | Dangerous | Unknown` — Security assessment levels for packages
- pub `CompatibilityInfo` struct L49-58 — `{ architecture: String, required_symbols: Vec<String>, missing_symbols: Vec<Stri...` — Compatibility information for packages

### crates/cloacina/src/registry/reconciler

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/registry/reconciler/extraction.rs

-  `HOST_CRATES` variable L29-42 — `: &[(&str, &str)]` — Cloacina crates whose path dependencies should be rewritten to host paths
-  `host_workspace_root` function L47-54 — `() -> PathBuf` — Returns the host workspace root, derived from `CARGO_MANIFEST_DIR` at compile time.
-  `rewrite_host_dependencies` function L64-144 — `(source_dir: &Path) -> Result<(), RegistryError>` — Rewrite path dependencies in an extracted source package's Cargo.toml
-  `RegistryReconciler` type L146-253 — `= RegistryReconciler` — it to a cdylib using `cargo build`.
-  `compile_source_package` function L156-202 — `( source_dir: &Path, ) -> Result<PathBuf, RegistryError>` — Compile a Rust source package directory to a cdylib.
-  `find_compiled_library` function L209-252 — `(target_dir: &Path) -> Result<PathBuf, RegistryError>` — Search `target_dir` for the cdylib produced by `cargo build --lib`.
-  `tests` module L256-580 — `-` — it to a cdylib using `cargo build`.
-  `find_compiled_library_finds_dylib_on_macos` function L265-282 — `()` — it to a cdylib using `cargo build`.
-  `find_compiled_library_ignores_hash_suffixed_artifacts` function L285-307 — `()` — it to a cdylib using `cargo build`.
-  `find_compiled_library_ignores_wrong_extension` function L310-321 — `()` — it to a cdylib using `cargo build`.
-  `find_compiled_library_ignores_non_lib_prefix` function L324-343 — `()` — it to a cdylib using `cargo build`.
-  `find_compiled_library_empty_directory` function L346-350 — `()` — it to a cdylib using `cargo build`.
-  `find_compiled_library_nonexistent_directory` function L353-359 — `()` — it to a cdylib using `cargo build`.
-  `find_compiled_library_prefers_first_matching` function L362-382 — `()` — it to a cdylib using `cargo build`.
-  `rewrite_host_dependencies_adds_path_to_string_dep` function L390-431 — `()` — it to a cdylib using `cargo build`.
-  `rewrite_host_dependencies_adds_path_to_table_dep` function L435-457 — `()` — it to a cdylib using `cargo build`.
-  `rewrite_host_dependencies_preserves_existing_workspace` function L461-484 — `()` — it to a cdylib using `cargo build`.
-  `rewrite_host_dependencies_no_cloacina_deps_is_noop` function L488-510 — `()` — it to a cdylib using `cargo build`.
-  `rewrite_host_dependencies_missing_cargo_toml_errors` function L514-520 — `()` — it to a cdylib using `cargo build`.
-  `rewrite_host_dependencies_invalid_toml_errors` function L524-531 — `()` — it to a cdylib using `cargo build`.
-  `rewrite_host_dependencies_handles_dev_and_build_deps` function L535-567 — `()` — it to a cdylib using `cargo build`.
-  `host_workspace_root_returns_valid_path` function L571-579 — `()` — it to a cdylib using `cargo build`.

#### crates/cloacina/src/registry/reconciler/loading.rs

-  `RegistryReconciler` type L27-911 — `= RegistryReconciler` — Package loading, unloading, and task/workflow registration.
-  `load_package` function L38-453 — `( &self, metadata: WorkflowMetadata, ) -> Result<(), RegistryError>` — Load a package into the global registries.
-  `unload_package` function L456-520 — `( &self, package_id: WorkflowPackageId, ) -> Result<(), RegistryError>` — Unload a package from the global registries
-  `register_package_tasks` function L523-564 — `( &self, metadata: &WorkflowMetadata, package_data: &[u8], ) -> Result<Vec<TaskN...` — Register tasks from a package into the global task registry
-  `register_package_workflows` function L567-708 — `( &self, metadata: &WorkflowMetadata, package_data: &[u8], ) -> Result<Option<St...` — Register workflows from a package into the global workflow registry
-  `create_workflow_from_host_registry` function L711-759 — `( &self, package_name: &str, workflow_name: &str, tenant_id: &str, ) -> Result<c...` — Create a workflow using the host's global task registry (avoiding FFI isolation)
-  `create_workflow_from_host_registry_static` function L762-809 — `( package_name: &str, workflow_name: &str, tenant_id: &str, ) -> Result<crate::w...` — Static version of create_workflow_from_host_registry for use in closures
-  `unregister_package_tasks` function L812-835 — `( &self, package_id: WorkflowPackageId, task_namespaces: &[TaskNamespace], ) -> ...` — Unregister tasks from the global task registry
-  `unregister_package_workflow` function L838-849 — `( &self, workflow_name: &str, ) -> Result<(), RegistryError>` — Unregister a workflow from the global workflow registry
-  `register_package_triggers` function L857-899 — `( &self, metadata: &WorkflowMetadata, cloacina_metadata: &cloacina_workflow_plug...` — Verify and track triggers declared in a package's `CloacinaMetadata`.
-  `unregister_package_triggers` function L902-910 — `(&self, trigger_names: &[String])` — Unregister triggers from the global trigger registry.
-  `tests` module L914-1211 — `-` — Package loading, unloading, and task/workflow registration.
-  `make_test_reconciler` function L923-928 — `() -> RegistryReconciler` — Create a minimal RegistryReconciler for testing.
-  `make_test_metadata` function L930-943 — `() -> WorkflowMetadata` — Package loading, unloading, and task/workflow registration.
-  `make_cloacina_metadata_with_triggers` function L945-962 — `( triggers: Vec<cloacina_workflow_plugin::TriggerDefinition>, ) -> cloacina_work...` — Package loading, unloading, and task/workflow registration.
-  `register_triggers_with_no_triggers_returns_empty` function L970-979 — `()` — Package loading, unloading, and task/workflow registration.
-  `register_triggers_tracks_registered_triggers` function L983-1014 — `()` — Package loading, unloading, and task/workflow registration.
-  `register_triggers_skips_unregistered_triggers` function L1018-1038 — `()` — Package loading, unloading, and task/workflow registration.
-  `register_triggers_mixed_registered_and_missing` function L1042-1081 — `()` — Package loading, unloading, and task/workflow registration.
-  `unregister_triggers_removes_from_global_registry` function L1089-1108 — `()` — Package loading, unloading, and task/workflow registration.
-  `unregister_triggers_handles_already_removed` function L1112-1119 — `()` — Package loading, unloading, and task/workflow registration.
-  `unregister_triggers_empty_list_is_noop` function L1123-1126 — `()` — Package loading, unloading, and task/workflow registration.
-  `unregister_workflow_removes_from_global_registry` function L1134-1169 — `()` — Package loading, unloading, and task/workflow registration.
-  `unregister_workflow_nonexistent_is_ok` function L1173-1180 — `()` — Package loading, unloading, and task/workflow registration.
-  `DummyTrigger` struct L1187-1189 — `{ name: String }` — Package loading, unloading, and task/workflow registration.
-  `DummyTrigger` type L1192-1210 — `= DummyTrigger` — Package loading, unloading, and task/workflow registration.
-  `name` function L1193-1195 — `(&self) -> &str` — Package loading, unloading, and task/workflow registration.
-  `poll_interval` function L1197-1199 — `(&self) -> std::time::Duration` — Package loading, unloading, and task/workflow registration.
-  `allow_concurrent` function L1201-1203 — `(&self) -> bool` — Package loading, unloading, and task/workflow registration.
-  `poll` function L1205-1209 — `( &self, ) -> Result<crate::trigger::TriggerResult, crate::trigger::TriggerError...` — Package loading, unloading, and task/workflow registration.

#### crates/cloacina/src/registry/reconciler/mod.rs

- pub `ReconcilerConfig` struct L54-69 — `{ reconcile_interval: Duration, enable_startup_reconciliation: bool, package_ope...` — Configuration for the Registry Reconciler
- pub `ReconcileResult` struct L85-100 — `{ packages_loaded: Vec<WorkflowPackageId>, packages_unloaded: Vec<WorkflowPackag...` — Result of a reconciliation operation
- pub `has_changes` function L104-106 — `(&self) -> bool` — Check if the reconciliation had any changes
- pub `has_failures` function L109-111 — `(&self) -> bool` — Check if the reconciliation had any failures
- pub `ReconcilerStatus` struct L135-141 — `{ packages_loaded: usize, package_details: Vec<PackageStatusDetail> }` — Status information about the reconciler
- pub `PackageStatusDetail` struct L145-157 — `{ package_name: String, version: String, task_count: usize, has_workflow: bool }` — Detailed status information about a loaded package
- pub `RegistryReconciler` struct L160-191 — `{ registry: Arc<dyn WorkflowRegistry>, config: ReconcilerConfig, runtime: Option...` — Registry Reconciler for synchronizing database state with in-memory registries
- pub `new` function L195-219 — `( registry: Arc<dyn WorkflowRegistry>, config: ReconcilerConfig, shutdown_rx: wa...` — Create a new Registry Reconciler
- pub `with_runtime` function L224-227 — `(mut self, runtime: Arc<crate::Runtime>) -> Self` — Attach a Runtime to this reconciler.
- pub `with_reactive_scheduler` function L230-236 — `(self, scheduler: Arc<ReactiveScheduler>) -> Self` — Set the reactive scheduler for computation graph package routing.
- pub `set_reactive_scheduler_slot` function L240-245 — `( &mut self, slot: Arc<tokio::sync::RwLock<Option<Arc<ReactiveScheduler>>>>, )` — Replace the reactive scheduler slot with a shared reference from the runner.
- pub `start_reconciliation_loop` function L248-321 — `(mut self) -> Result<(), RegistryError>` — Start the background reconciliation loop
- pub `reconcile` function L324-436 — `(&self) -> Result<ReconcileResult, RegistryError>` — Perform a single reconciliation operation
- pub `get_status` function L462-477 — `(&self) -> ReconcilerStatus` — Get the current reconciliation status
-  `extraction` module L34 — `-` — # Registry Reconciler
-  `loading` module L35 — `-` — - `PackageState`: Tracking loaded package state
-  `ReconcilerConfig` type L71-81 — `impl Default for ReconcilerConfig` — - `PackageState`: Tracking loaded package state
-  `default` function L72-80 — `() -> Self` — - `PackageState`: Tracking loaded package state
-  `ReconcileResult` type L102-112 — `= ReconcileResult` — - `PackageState`: Tracking loaded package state
-  `PackageState` struct L116-131 — `{ metadata: WorkflowMetadata, task_namespaces: Vec<TaskNamespace>, workflow_name...` — Tracks the state of loaded packages
-  `RegistryReconciler` type L193-478 — `= RegistryReconciler` — - `PackageState`: Tracking loaded package state
-  `shutdown_cleanup` function L439-459 — `(&self) -> Result<(), RegistryError>` — Perform cleanup operations during shutdown
-  `tests` module L481-664 — `-` — - `PackageState`: Tracking loaded package state
-  `test_reconciler_config_default` function L487-494 — `()` — - `PackageState`: Tracking loaded package state
-  `test_reconcile_result_methods` function L497-519 — `()` — - `PackageState`: Tracking loaded package state
-  `test_reconciler_status` function L522-546 — `()` — - `PackageState`: Tracking loaded package state
-  `test_reconciler_config_custom_values` function L549-563 — `()` — - `PackageState`: Tracking loaded package state
-  `test_reconcile_result_no_changes_no_failures` function L566-578 — `()` — - `PackageState`: Tracking loaded package state
-  `test_reconcile_result_unloaded_counts_as_change` function L581-592 — `()` — - `PackageState`: Tracking loaded package state
-  `test_reconcile_result_both_loaded_and_unloaded` function L595-609 — `()` — - `PackageState`: Tracking loaded package state
-  `test_package_status_detail_fields` function L612-624 — `()` — - `PackageState`: Tracking loaded package state
-  `test_reconciler_status_empty` function L627-635 — `()` — - `PackageState`: Tracking loaded package state
-  `test_reconciler_config_clone` function L638-647 — `()` — - `PackageState`: Tracking loaded package state
-  `test_reconcile_result_clone` function L650-663 — `()` — - `PackageState`: Tracking loaded package state

### crates/cloacina/src/registry/workflow_registry

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/registry/workflow_registry/database.rs

- pub `claim_next_build` function L873-968 — `(&self) -> Result<Option<ClaimedBuild>, RegistryError>` — A pending build claimed by the compiler.
- pub `mark_build_success` function L972-1032 — `( &self, package_id: Uuid, compiled: Vec<u8>, ) -> Result<(), RegistryError>` — Record a successful build.
- pub `mark_build_failed` function L1035-1094 — `( &self, package_id: Uuid, error: &str, ) -> Result<(), RegistryError>` — Record a failed build.
- pub `heartbeat_build` function L1098-1149 — `(&self, package_id: Uuid) -> Result<(), RegistryError>` — Refresh `build_claimed_at` so the stale-build sweeper doesn't reset us.
- pub `sweep_stale_builds` function L1153-1224 — `( &self, stale_threshold: std::time::Duration, ) -> Result<usize, RegistryError>` — Reset rows stuck in `building` whose last heartbeat is older than
- pub `ClaimedBuild` struct L1230-1236 — `{ id: Uuid, registry_id: Uuid, package_name: String, version: String, metadata: ...` — A build row claimed by the compiler.
-  `store_package_metadata` function L29-56 — `( &self, registry_id: &str, package_metadata: &crate::registry::loader::package_...` — Store package metadata in the database.
-  `store_package_metadata_postgres` function L59-121 — `( &self, registry_uuid: Uuid, package_metadata: &crate::registry::loader::packag...` — Database operations for workflow registry metadata storage.
-  `store_package_metadata_sqlite` function L124-184 — `( &self, registry_uuid: Uuid, package_metadata: &crate::registry::loader::packag...` — Database operations for workflow registry metadata storage.
-  `get_package_metadata` function L187-205 — `( &self, package_name: &str, version: &str, ) -> Result< Option<( String, crate:...` — Retrieve package metadata from the database.
-  `get_package_metadata_postgres` function L208-254 — `( &self, package_name: &str, version: &str, ) -> Result< Option<( String, crate:...` — Database operations for workflow registry metadata storage.
-  `get_package_metadata_sqlite` function L257-303 — `( &self, package_name: &str, version: &str, ) -> Result< Option<( String, crate:...` — Database operations for workflow registry metadata storage.
-  `list_all_packages` function L306-312 — `(&self) -> Result<Vec<WorkflowMetadata>, RegistryError>` — List all packages in the registry.
-  `list_all_packages_postgres` function L315-362 — `(&self) -> Result<Vec<WorkflowMetadata>, RegistryError>` — Database operations for workflow registry metadata storage.
-  `list_all_packages_sqlite` function L365-412 — `(&self) -> Result<Vec<WorkflowMetadata>, RegistryError>` — Database operations for workflow registry metadata storage.
-  `delete_package_metadata` function L415-427 — `( &self, package_name: &str, version: &str, ) -> Result<(), RegistryError>` — Delete package metadata from the database.
-  `delete_package_metadata_postgres` function L430-459 — `( &self, package_name: &str, version: &str, ) -> Result<(), RegistryError>` — Database operations for workflow registry metadata storage.
-  `delete_package_metadata_sqlite` function L462-491 — `( &self, package_name: &str, version: &str, ) -> Result<(), RegistryError>` — Database operations for workflow registry metadata storage.
-  `get_package_metadata_by_id` function L494-503 — `( &self, package_id: Uuid, ) -> Result<Option<(String, WorkflowMetadata)>, Regis...` — Get package metadata by ID.
-  `get_package_metadata_by_id_postgres` function L506-561 — `( &self, package_id: Uuid, ) -> Result<Option<(String, WorkflowMetadata)>, Regis...` — Database operations for workflow registry metadata storage.
-  `get_package_metadata_by_id_sqlite` function L564-620 — `( &self, package_id: Uuid, ) -> Result<Option<(String, WorkflowMetadata)>, Regis...` — Database operations for workflow registry metadata storage.
-  `get_active_package_by_name` function L625-676 — `( &self, package_name: &str, ) -> Result<Option<(Uuid, String, String)>, Registr...` — Look up the active package row for `name`, returning (id, registry_id, content_hash).
-  `supersede_and_insert` function L685-799 — `( &self, old_id: Option<Uuid>, registry_id: &str, package_metadata: &crate::regi...` — Supersede the current active row for `old_id` (if provided) and insert a new
-  `delete_package_metadata_by_id` function L802-812 — `( &self, package_id: Uuid, ) -> Result<(), RegistryError>` — Delete package metadata by ID.
-  `delete_package_metadata_by_id_postgres` function L815-838 — `( &self, package_id: Uuid, ) -> Result<(), RegistryError>` — Database operations for workflow registry metadata storage.
-  `delete_package_metadata_by_id_sqlite` function L841-865 — `( &self, package_id: Uuid, ) -> Result<(), RegistryError>` — Database operations for workflow registry metadata storage.
-  `MAX_ERR` variable L1043 — `: usize` — Database operations for workflow registry metadata storage.
-  `ClaimedBuild` type L1238-1248 — `= ClaimedBuild` — Database operations for workflow registry metadata storage.
-  `from` function L1239-1247 — `(u: crate::dal::unified::models::UnifiedWorkflowPackage) -> Self` — Database operations for workflow registry metadata storage.
-  `tests` module L1251-1682 — `-` — Database operations for workflow registry metadata storage.
-  `create_test_registry` function L1258-1269 — `() -> WorkflowRegistryImpl<UnifiedRegistryStorage>` — Database operations for workflow registry metadata storage.
-  `sample_metadata` function L1272-1290 — `(name: &str, version: &str) -> PackageMetadata` — Database operations for workflow registry metadata storage.
-  `test_store_and_get_package_metadata` function L1294-1314 — `()` — Database operations for workflow registry metadata storage.
-  `test_get_package_metadata_not_found` function L1318-1326 — `()` — Database operations for workflow registry metadata storage.
-  `test_list_all_packages` function L1330-1356 — `()` — Database operations for workflow registry metadata storage.
-  `test_delete_package_metadata` function L1360-1389 — `()` — Database operations for workflow registry metadata storage.
-  `test_get_package_metadata_by_id` function L1393-1409 — `()` — Database operations for workflow registry metadata storage.
-  `test_get_package_metadata_by_id_not_found` function L1413-1421 — `()` — Database operations for workflow registry metadata storage.
-  `test_delete_package_metadata_by_id` function L1425-1445 — `()` — Database operations for workflow registry metadata storage.
-  `test_delete_nonexistent_does_not_error` function L1449-1461 — `()` — Database operations for workflow registry metadata storage.
-  `test_supersede_and_insert_fresh_name` function L1469-1486 — `()` — Database operations for workflow registry metadata storage.
-  `test_supersede_and_insert_replaces_old_active` function L1490-1541 — `()` — Database operations for workflow registry metadata storage.
-  `test_partial_unique_rejects_second_active_for_same_name` function L1545-1568 — `()` — Database operations for workflow registry metadata storage.
-  `test_claim_next_build_returns_pending_row` function L1576-1591 — `()` — Database operations for workflow registry metadata storage.
-  `test_mark_build_success_flips_state_and_writes_bytes` function L1595-1617 — `()` — Database operations for workflow registry metadata storage.
-  `test_mark_build_failed_writes_error` function L1621-1634 — `()` — Database operations for workflow registry metadata storage.
-  `test_heartbeat_updates_claim_timestamp_only_while_building` function L1638-1657 — `()` — Database operations for workflow registry metadata storage.
-  `test_sweep_stale_builds_resets_old_rows` function L1661-1681 — `()` — Database operations for workflow registry metadata storage.

#### crates/cloacina/src/registry/workflow_registry/filesystem.rs

- pub `FilesystemWorkflowRegistry` struct L42-45 — `{ watch_dirs: Vec<PathBuf> }` — A `WorkflowRegistry` implementation backed by directories of `.cloacina` files.
- pub `new` function L52-62 — `(watch_dirs: Vec<PathBuf>) -> Self` — Create a new filesystem registry watching the given directories.
-  `FilesystemWorkflowRegistry` type L47-173 — `= FilesystemWorkflowRegistry` — handles operational state (schedules, executions) separately.
-  `scan_packages` function L68-164 — `(&self) -> HashMap<(String, String), (PathBuf, WorkflowMetadata)>` — Scan all watch directories for `.cloacina` files.
-  `find_package_path` function L167-172 — `(&self, package_name: &str, version: &str) -> Option<PathBuf>` — Find the file path for a package by name and version.
-  `FilesystemWorkflowRegistry` type L176-317 — `impl WorkflowRegistry for FilesystemWorkflowRegistry` — handles operational state (schedules, executions) separately.
-  `register_workflow` function L177-251 — `( &mut self, package_data: Vec<u8>, ) -> Result<WorkflowPackageId, RegistryError...` — handles operational state (schedules, executions) separately.
-  `get_workflow` function L253-277 — `( &self, package_name: &str, version: &str, ) -> Result<Option<LoadedWorkflow>, ...` — handles operational state (schedules, executions) separately.
-  `list_workflows` function L279-285 — `(&self) -> Result<Vec<WorkflowMetadata>, RegistryError>` — handles operational state (schedules, executions) separately.
-  `unregister_workflow` function L287-316 — `( &mut self, package_name: &str, version: &str, ) -> Result<(), RegistryError>` — handles operational state (schedules, executions) separately.
-  `uuid_from_fingerprint` function L323-326 — `(fingerprint: &str) -> Uuid` — Derive a deterministic UUID from a string fingerprint.
-  `tests` module L329-600 — `-` — handles operational state (schedules, executions) separately.
-  `build_test_archive` function L334-360 — `(name: &str, version: &str) -> Vec<u8>` — Build a minimal `.cloacina` source archive via fidius pack_package.
-  `test_list_empty_directory` function L363-368 — `()` — handles operational state (schedules, executions) separately.
-  `test_list_discovers_packages` function L371-386 — `()` — handles operational state (schedules, executions) separately.
-  `test_list_multiple_directories` function L389-410 — `()` — handles operational state (schedules, executions) separately.
-  `test_get_workflow_returns_archive_bytes` function L413-426 — `()` — handles operational state (schedules, executions) separately.
-  `test_get_workflow_not_found` function L429-434 — `()` — handles operational state (schedules, executions) separately.
-  `test_register_writes_file` function L437-457 — `()` — handles operational state (schedules, executions) separately.
-  `test_register_duplicate_rejected` function L460-469 — `()` — handles operational state (schedules, executions) separately.
-  `test_unregister_removes_file` function L472-496 — `()` — handles operational state (schedules, executions) separately.
-  `test_unregister_not_found` function L499-505 — `()` — handles operational state (schedules, executions) separately.
-  `test_corrupt_file_skipped` function L508-530 — `()` — handles operational state (schedules, executions) separately.
-  `test_nonexistent_directory_handled` function L533-539 — `()` — handles operational state (schedules, executions) separately.
-  `test_register_creates_directory` function L542-552 — `()` — handles operational state (schedules, executions) separately.
-  `test_deterministic_package_id` function L555-562 — `()` — handles operational state (schedules, executions) separately.
-  `test_package_with_triggers_in_manifest` function L565-599 — `()` — handles operational state (schedules, executions) separately.

#### crates/cloacina/src/registry/workflow_registry/mod.rs

- pub `filesystem` module L24 — `-` — cohesive system for managing packaged workflows.
- pub `WorkflowRegistryImpl` struct L43-58 — `{ storage: S, database: Database, loader: PackageLoader, registrar: TaskRegistra...` — Complete implementation of the workflow registry.
- pub `new` function L72-85 — `(storage: S, database: Database) -> Result<Self, RegistryError>` — Create a new workflow registry implementation.
- pub `with_strict_validation` function L88-101 — `(storage: S, database: Database) -> Result<Self, RegistryError>` — Create a registry with strict validation enabled.
- pub `loaded_package_count` function L104-106 — `(&self) -> usize` — Get the number of currently loaded packages.
- pub `total_registered_tasks` function L109-111 — `(&self) -> usize` — Get the total number of registered tasks across all packages.
- pub `register_workflow_package` function L121-127 — `( &mut self, package_data: Vec<u8>, ) -> Result<Uuid, RegistryError>` — Register a workflow package (alias for register_workflow via the trait).
- pub `get_workflow_package_by_id` function L132-153 — `( &self, package_id: Uuid, ) -> Result<Option<(WorkflowMetadata, Vec<u8>)>, Regi...` — Get a workflow package by its UUID.
- pub `get_workflow_package_by_name` function L158-168 — `( &self, package_name: &str, version: &str, ) -> Result<Option<(WorkflowMetadata...` — Get a workflow package by name and version.
- pub `exists_by_id` function L171-173 — `(&self, package_id: Uuid) -> Result<bool, RegistryError>` — Check if a package exists by ID.
- pub `exists_by_name` function L176-185 — `( &self, package_name: &str, version: &str, ) -> Result<bool, RegistryError>` — Check if a package exists by name and version.
- pub `list_packages` function L190-192 — `(&self) -> Result<Vec<WorkflowMetadata>, RegistryError>` — List all packages in the registry.
- pub `unregister_workflow_package_by_id` function L195-219 — `( &mut self, package_id: Uuid, ) -> Result<(), RegistryError>` — Unregister a workflow package by ID.
- pub `unregister_workflow_package_by_name` function L222-238 — `( &mut self, package_name: &str, version: &str, ) -> Result<(), RegistryError>` — Unregister a workflow package by name and version.
-  `database` module L23 — `-` — Complete implementation of the workflow registry.
-  `package` module L25 — `-` — cohesive system for managing packaged workflows.
-  `register_workflow` function L243-320 — `( &mut self, package_data: Vec<u8>, ) -> Result<WorkflowPackageId, RegistryError...` — cohesive system for managing packaged workflows.
-  `get_workflow` function L322-366 — `( &self, package_name: &str, version: &str, ) -> Result<Option<LoadedWorkflow>, ...` — cohesive system for managing packaged workflows.
-  `list_workflows` function L368-370 — `(&self) -> Result<Vec<WorkflowMetadata>, RegistryError>` — cohesive system for managing packaged workflows.
-  `unregister_workflow` function L372-403 — `( &mut self, package_name: &str, version: &str, ) -> Result<(), RegistryError>` — cohesive system for managing packaged workflows.
-  `tests` module L407-430 — `-` — cohesive system for managing packaged workflows.
-  `test_registry_creation` function L412-419 — `()` — cohesive system for managing packaged workflows.
-  `test_registry_metrics` function L422-429 — `()` — cohesive system for managing packaged workflows.

#### crates/cloacina/src/registry/workflow_registry/package.rs

-  `is_cloacina_package` function L24-27 — `(data: &[u8]) -> bool` — Check if package data is a bzip2-compressed `.cloacina` source archive.

### crates/cloacina/src/runner/default_runner

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/runner/default_runner/config.rs

- pub `ConfigError` enum L38-41 — `Invalid` — Errors that can occur during configuration validation.
- pub `DefaultRunnerConfig` struct L67-97 — `{ max_concurrent_tasks: usize, scheduler_poll_interval: Duration, task_timeout: ...` — Configuration for the default runner
- pub `builder` function L101-103 — `() -> DefaultRunnerConfigBuilder` — Creates a new configuration builder with default values.
- pub `max_concurrent_tasks` function L106-108 — `(&self) -> usize` — Maximum number of concurrent task executions allowed.
- pub `scheduler_poll_interval` function L111-113 — `(&self) -> Duration` — How often the scheduler checks for ready tasks.
- pub `task_timeout` function L116-118 — `(&self) -> Duration` — Maximum time allowed for a single task to execute.
- pub `workflow_timeout` function L121-123 — `(&self) -> Option<Duration>` — Optional maximum time for an entire workflow execution.
- pub `db_pool_size` function L126-128 — `(&self) -> u32` — Number of database connections in the pool.
- pub `enable_recovery` function L131-133 — `(&self) -> bool` — Whether automatic recovery is enabled.
- pub `enable_cron_scheduling` function L136-138 — `(&self) -> bool` — Whether cron scheduling is enabled.
- pub `cron_poll_interval` function L141-143 — `(&self) -> Duration` — Poll interval for cron schedules.
- pub `cron_max_catchup_executions` function L146-148 — `(&self) -> usize` — Maximum catchup executions for missed cron runs.
- pub `cron_enable_recovery` function L151-153 — `(&self) -> bool` — Whether cron recovery is enabled.
- pub `cron_recovery_interval` function L156-158 — `(&self) -> Duration` — How often to check for lost cron executions.
- pub `cron_lost_threshold_minutes` function L161-163 — `(&self) -> i32` — Minutes before an execution is considered lost.
- pub `cron_max_recovery_age` function L166-168 — `(&self) -> Duration` — Maximum age of executions to recover.
- pub `cron_max_recovery_attempts` function L171-173 — `(&self) -> usize` — Maximum recovery attempts per execution.
- pub `enable_trigger_scheduling` function L176-178 — `(&self) -> bool` — Whether trigger scheduling is enabled.
- pub `trigger_base_poll_interval` function L181-183 — `(&self) -> Duration` — Base poll interval for trigger readiness checks.
- pub `trigger_poll_timeout` function L186-188 — `(&self) -> Duration` — Timeout for trigger poll operations.
- pub `enable_registry_reconciler` function L191-193 — `(&self) -> bool` — Whether the registry reconciler is enabled.
- pub `registry_reconcile_interval` function L196-198 — `(&self) -> Duration` — How often to run registry reconciliation.
- pub `registry_enable_startup_reconciliation` function L201-203 — `(&self) -> bool` — Whether startup reconciliation is enabled.
- pub `registry_storage_path` function L206-208 — `(&self) -> Option<&std::path::Path>` — Path for registry storage (filesystem backend).
- pub `registry_storage_backend` function L211-213 — `(&self) -> &str` — Registry storage backend type.
- pub `enable_claiming` function L216-218 — `(&self) -> bool` — Whether task claiming is enabled for horizontal scaling.
- pub `heartbeat_interval` function L221-223 — `(&self) -> Duration` — Heartbeat interval for claimed tasks.
- pub `stale_claim_sweep_interval` function L226-228 — `(&self) -> Duration` — Interval for stale claim sweep (only when claiming is enabled).
- pub `stale_claim_threshold` function L231-233 — `(&self) -> Duration` — How old a heartbeat must be to consider a claim stale.
- pub `runner_id` function L236-238 — `(&self) -> Option<&str>` — Optional runner identifier for logging.
- pub `runner_name` function L241-243 — `(&self) -> Option<&str>` — Optional runner name for logging.
- pub `routing_config` function L246-248 — `(&self) -> Option<&RoutingConfig>` — Routing configuration for task dispatch.
- pub `DefaultRunnerConfigBuilder` struct L262-264 — `{ config: DefaultRunnerConfig }` — Builder for [`DefaultRunnerConfig`].
- pub `max_concurrent_tasks` function L306-309 — `(mut self, value: usize) -> Self` — Sets the maximum number of concurrent task executions.
- pub `scheduler_poll_interval` function L312-315 — `(mut self, value: Duration) -> Self` — Sets the scheduler poll interval.
- pub `task_timeout` function L318-321 — `(mut self, value: Duration) -> Self` — Sets the task timeout.
- pub `workflow_timeout` function L324-327 — `(mut self, value: Option<Duration>) -> Self` — Sets the workflow timeout.
- pub `db_pool_size` function L330-333 — `(mut self, value: u32) -> Self` — Sets the database pool size.
- pub `enable_recovery` function L336-339 — `(mut self, value: bool) -> Self` — Enables or disables automatic recovery.
- pub `enable_cron_scheduling` function L342-345 — `(mut self, value: bool) -> Self` — Enables or disables cron scheduling.
- pub `cron_poll_interval` function L348-351 — `(mut self, value: Duration) -> Self` — Sets the cron poll interval.
- pub `cron_max_catchup_executions` function L354-357 — `(mut self, value: usize) -> Self` — Sets the maximum catchup executions for cron.
- pub `cron_enable_recovery` function L360-363 — `(mut self, value: bool) -> Self` — Enables or disables cron recovery.
- pub `cron_recovery_interval` function L366-369 — `(mut self, value: Duration) -> Self` — Sets the cron recovery interval.
- pub `cron_lost_threshold_minutes` function L372-375 — `(mut self, value: i32) -> Self` — Sets the cron lost threshold in minutes.
- pub `cron_max_recovery_age` function L378-381 — `(mut self, value: Duration) -> Self` — Sets the maximum cron recovery age.
- pub `cron_max_recovery_attempts` function L384-387 — `(mut self, value: usize) -> Self` — Sets the maximum cron recovery attempts.
- pub `enable_trigger_scheduling` function L390-393 — `(mut self, value: bool) -> Self` — Enables or disables trigger scheduling.
- pub `trigger_base_poll_interval` function L396-399 — `(mut self, value: Duration) -> Self` — Sets the trigger base poll interval.
- pub `trigger_poll_timeout` function L402-405 — `(mut self, value: Duration) -> Self` — Sets the trigger poll timeout.
- pub `enable_registry_reconciler` function L408-411 — `(mut self, value: bool) -> Self` — Enables or disables the registry reconciler.
- pub `registry_reconcile_interval` function L414-417 — `(mut self, value: Duration) -> Self` — Sets the registry reconcile interval.
- pub `registry_enable_startup_reconciliation` function L420-423 — `(mut self, value: bool) -> Self` — Enables or disables startup reconciliation.
- pub `registry_storage_path` function L426-429 — `(mut self, value: Option<std::path::PathBuf>) -> Self` — Sets the registry storage path.
- pub `registry_storage_backend` function L432-435 — `(mut self, value: impl Into<String>) -> Self` — Sets the registry storage backend.
- pub `runner_id` function L438-441 — `(mut self, value: Option<String>) -> Self` — Sets the runner identifier.
- pub `runner_name` function L444-447 — `(mut self, value: Option<String>) -> Self` — Sets the runner name.
- pub `routing_config` function L450-453 — `(mut self, value: Option<RoutingConfig>) -> Self` — Sets the routing configuration.
- pub `enable_claiming` function L456-459 — `(mut self, value: bool) -> Self` — Enables or disables task claiming for horizontal scaling.
- pub `heartbeat_interval` function L462-465 — `(mut self, value: Duration) -> Self` — Sets the heartbeat interval for claimed tasks.
- pub `build` function L470-497 — `(self) -> Result<DefaultRunnerConfig, ConfigError>` — Builds and validates the configuration.
- pub `DefaultRunnerBuilder` struct L534-539 — `{ database_url: Option<String>, schema: Option<String>, config: DefaultRunnerCon...` — Builder for creating a DefaultRunner with PostgreSQL schema-based multi-tenancy
- pub `new` function L549-556 — `() -> Self` — Creates a new builder with default configuration
- pub `database_url` function L559-562 — `(mut self, url: &str) -> Self` — Sets the database URL
- pub `schema` function L568-571 — `(mut self, schema: &str) -> Self` — Sets the PostgreSQL schema for multi-tenant isolation
- pub `with_config` function L574-577 — `(mut self, config: DefaultRunnerConfig) -> Self` — Sets the full configuration
- pub `runtime` function L584-587 — `(mut self, runtime: Runtime) -> Self` — Sets a scoped [`Runtime`] for this runner.
- pub `build` function L601-725 — `(self) -> Result<DefaultRunner, WorkflowExecutionError>` — Builds the DefaultRunner
- pub `routing_config` function L743-746 — `(mut self, config: RoutingConfig) -> Self` — Sets custom routing configuration for task dispatch.
-  `DefaultRunnerConfig` type L99-249 — `= DefaultRunnerConfig` — configuring the DefaultRunner's behavior.
-  `DefaultRunnerConfigBuilder` type L266-302 — `impl Default for DefaultRunnerConfigBuilder` — configuring the DefaultRunner's behavior.
-  `default` function L267-301 — `() -> Self` — configuring the DefaultRunner's behavior.
-  `DefaultRunnerConfigBuilder` type L304-498 — `= DefaultRunnerConfigBuilder` — configuring the DefaultRunner's behavior.
-  `DefaultRunnerConfig` type L500-506 — `impl Default for DefaultRunnerConfig` — configuring the DefaultRunner's behavior.
-  `default` function L501-505 — `() -> Self` — configuring the DefaultRunner's behavior.
-  `DefaultRunnerBuilder` type L541-545 — `impl Default for DefaultRunnerBuilder` — configuring the DefaultRunner's behavior.
-  `default` function L542-544 — `() -> Self` — configuring the DefaultRunner's behavior.
-  `DefaultRunnerBuilder` type L547-747 — `= DefaultRunnerBuilder` — configuring the DefaultRunner's behavior.
-  `validate_schema_name` function L590-598 — `(schema: &str) -> Result<(), WorkflowExecutionError>` — Validates the schema name contains only alphanumeric characters and underscores
-  `tests` module L750-925 — `-` — configuring the DefaultRunner's behavior.
-  `test_default_runner_config` function L754-769 — `()` — configuring the DefaultRunner's behavior.
-  `test_registry_storage_backend_configuration` function L772-798 — `()` — configuring the DefaultRunner's behavior.
-  `test_runner_identification` function L801-810 — `()` — configuring the DefaultRunner's behavior.
-  `test_registry_configuration_options` function L813-837 — `()` — configuring the DefaultRunner's behavior.
-  `test_cron_configuration` function L840-856 — `()` — configuring the DefaultRunner's behavior.
-  `test_db_pool_size_default` function L859-862 — `()` — configuring the DefaultRunner's behavior.
-  `test_config_clone` function L865-878 — `()` — configuring the DefaultRunner's behavior.
-  `test_config_debug` function L881-889 — `()` — configuring the DefaultRunner's behavior.
-  `test_builder_all_fields` function L892-924 — `()` — configuring the DefaultRunner's behavior.

#### crates/cloacina/src/runner/default_runner/cron_api.rs

- pub `register_cron_workflow` function L40-93 — `( &self, workflow_name: &str, cron_expression: &str, timezone: &str, ) -> Result...` — Register a workflow to run on a cron schedule
- pub `list_cron_schedules` function L104-123 — `( &self, enabled_only: bool, limit: i64, offset: i64, ) -> Result<Vec<crate::mod...` — List all registered cron schedules
- pub `set_cron_schedule_enabled` function L133-154 — `( &self, schedule_id: UniversalUuid, enabled: bool, ) -> Result<(), WorkflowExec...` — Enable or disable a cron schedule
- pub `delete_cron_schedule` function L163-179 — `( &self, schedule_id: UniversalUuid, ) -> Result<(), WorkflowExecutionError>` — Delete a cron schedule
- pub `get_cron_schedule` function L188-204 — `( &self, schedule_id: UniversalUuid, ) -> Result<crate::models::schedule::Schedu...` — Get a specific cron schedule by ID
- pub `update_cron_schedule` function L215-275 — `( &self, schedule_id: UniversalUuid, cron_expression: Option<&str>, timezone: Op...` — Update a cron schedule's expression and/or timezone
- pub `get_cron_execution_history` function L286-305 — `( &self, schedule_id: UniversalUuid, limit: i64, offset: i64, ) -> Result<Vec<cr...` — Get execution history for a cron schedule
- pub `get_cron_execution_stats` function L314-331 — `( &self, since: chrono::DateTime<chrono::Utc>, ) -> Result<crate::dal::ScheduleE...` — Get cron execution statistics
- pub `get_workflow_registry` function L338-341 — `(&self) -> Option<Arc<dyn WorkflowRegistry>>` — Get access to the workflow registry (if enabled)
- pub `get_registry_reconciler_status` function L348-357 — `( &self, ) -> Option<crate::registry::ReconcilerStatus>` — Get the current status of the registry reconciler (if enabled)
- pub `is_registry_reconciler_enabled` function L360-362 — `(&self) -> bool` — Check if the registry reconciler is enabled in the configuration
-  `DefaultRunner` type L30-363 — `= DefaultRunner` — This module provides methods for managing cron-scheduled workflow executions.

#### crates/cloacina/src/runner/default_runner/mod.rs

- pub `DefaultRunner` struct L69-91 — `{ runtime: Arc<Runtime>, database: Database, config: DefaultRunnerConfig, schedu...` — Default runner that coordinates workflow scheduling and task execution
- pub `new` function L125-127 — `(database_url: &str) -> Result<Self, WorkflowExecutionError>` — Creates a new default runner with default configuration
- pub `builder` function L141-143 — `() -> DefaultRunnerBuilder` — Creates a builder for configuring the executor
- pub `with_schema` function L161-170 — `( database_url: &str, schema: &str, ) -> Result<Self, WorkflowExecutionError>` — Creates a new executor with PostgreSQL schema-based multi-tenancy
- pub `with_config` function L187-264 — `( database_url: &str, config: DefaultRunnerConfig, ) -> Result<Self, WorkflowExe...` — Creates a new unified executor with custom configuration
- pub `database` function L267-269 — `(&self) -> &Database` — Returns a reference to the database.
- pub `dal` function L272-274 — `(&self) -> DAL` — Returns the DAL for database operations.
- pub `unified_scheduler` function L280-282 — `(&self) -> Option<Arc<Scheduler>>` — Returns the unified scheduler if enabled.
- pub `set_reactive_scheduler` function L286-292 — `( &self, scheduler: Arc<crate::computation_graph::scheduler::ReactiveScheduler>,...` — Set the reactive scheduler for computation graph package routing.
- pub `shutdown` function L304-341 — `(&self) -> Result<(), WorkflowExecutionError>` — Gracefully shuts down the executor and its background services
-  `config` module L29 — `-` — Default runner for workflow execution.
-  `cron_api` module L30 — `-` — - `DefaultRunnerBuilder`: Builder for creating runners with custom settings
-  `services` module L31 — `-` — - `DefaultRunnerBuilder`: Builder for creating runners with custom settings
-  `workflow_executor_impl` module L32 — `-` — - `DefaultRunnerBuilder`: Builder for creating runners with custom settings
-  `workflow_result` module L33 — `-` — - `DefaultRunnerBuilder`: Builder for creating runners with custom settings
-  `RuntimeHandles` struct L97-110 — `{ scheduler_handle: Option<tokio::task::JoinHandle<()>>, executor_handle: Option...` — Internal structure for managing runtime handles of background services
-  `DefaultRunner` type L112-342 — `= DefaultRunner` — - `DefaultRunnerBuilder`: Builder for creating runners with custom settings
-  `DefaultRunner` type L344-359 — `impl Clone for DefaultRunner` — - `DefaultRunnerBuilder`: Builder for creating runners with custom settings
-  `clone` function L345-358 — `(&self) -> Self` — - `DefaultRunnerBuilder`: Builder for creating runners with custom settings
-  `DefaultRunner` type L362-368 — `impl Drop for DefaultRunner` — - `DefaultRunnerBuilder`: Builder for creating runners with custom settings
-  `drop` function L363-367 — `(&mut self)` — - `DefaultRunnerBuilder`: Builder for creating runners with custom settings

#### crates/cloacina/src/runner/default_runner/services.rs

-  `DefaultRunner` type L37-409 — `= DefaultRunner` — the scheduler, executor, cron scheduler, cron recovery, and registry reconciler.
-  `create_runner_span` function L39-57 — `(&self, operation: &str) -> tracing::Span` — Creates a tracing span for this runner instance with proper context
-  `start_background_services` function L69-135 — `(&self) -> Result<(), WorkflowExecutionError>` — Starts the background scheduler and executor services
-  `start_unified_scheduler` function L138-195 — `( &self, handles: &mut super::RuntimeHandles, shutdown_tx: &broadcast::Sender<()...` — Starts the unified scheduler that handles both cron and trigger schedules.
-  `start_cron_recovery` function L198-255 — `( &self, handles: &mut super::RuntimeHandles, shutdown_tx: &broadcast::Sender<()...` — Starts the cron recovery service
-  `start_registry_reconciler` function L258-361 — `( &self, handles: &mut super::RuntimeHandles, shutdown_tx: &broadcast::Sender<()...` — Starts the registry reconciler service
-  `start_stale_claim_sweeper` function L364-408 — `( &self, _handles: &mut super::RuntimeHandles, shutdown_tx: &broadcast::Sender<(...` — Starts the stale claim sweeper background service.

#### crates/cloacina/src/runner/default_runner/workflow_executor_impl.rs

-  `DefaultRunner` type L44-371 — `impl WorkflowExecutor for DefaultRunner` — Implementation of WorkflowExecutor trait for DefaultRunner
-  `execute` function L55-101 — `( &self, workflow_name: &str, context: Context<serde_json::Value>, ) -> Result<W...` — Executes a workflow synchronously and waits for completion
-  `execute_async` function L114-133 — `( &self, workflow_name: &str, context: Context<serde_json::Value>, ) -> Result<W...` — Executes a workflow asynchronously
-  `execute_with_callback` function L147-175 — `( &self, workflow_name: &str, context: Context<serde_json::Value>, callback: Box...` — Executes a workflow with status callbacks
-  `get_execution_status` function L184-208 — `( &self, execution_id: Uuid, ) -> Result<WorkflowStatus, WorkflowExecutionError>` — Gets the current status of a workflow execution
-  `get_execution_result` function L217-222 — `( &self, execution_id: Uuid, ) -> Result<WorkflowExecutionResult, WorkflowExecut...` — Gets the complete result of a workflow execution
-  `cancel_execution` function L231-244 — `(&self, execution_id: Uuid) -> Result<(), WorkflowExecutionError>` — Cancels an in-progress workflow execution
-  `pause_execution` function L257-292 — `( &self, execution_id: Uuid, reason: Option<&str>, ) -> Result<(), WorkflowExecu...` — Pauses a running workflow execution
-  `resume_execution` function L304-333 — `(&self, execution_id: Uuid) -> Result<(), WorkflowExecutionError>` — Resumes a paused workflow execution
-  `list_executions` function L341-362 — `( &self, ) -> Result<Vec<WorkflowExecutionResult>, WorkflowExecutionError>` — Lists recent workflow executions
-  `shutdown` function L368-370 — `(&self) -> Result<(), WorkflowExecutionError>` — Shuts down the executor

#### crates/cloacina/src/runner/default_runner/workflow_result.rs

-  `DefaultRunner` type L35-176 — `= DefaultRunner` — from database records.
-  `build_workflow_result` function L50-175 — `( &self, execution_id: Uuid, ) -> Result<WorkflowExecutionResult, WorkflowExecut...` — Builds a workflow execution result from an execution ID

### crates/cloacina/src/runner

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/runner/mod.rs

- pub `default_runner` module L23 — `-` — Workflow runners for executing complete workflows.

### crates/cloacina/src/security

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/security/api_keys.rs

- pub `generate_api_key` function L29-37 — `() -> (String, String)` — Generates a new API key, returning `(plaintext, hash)`.
- pub `hash_api_key` function L40-44 — `(key: &str) -> String` — Returns the lowercase hex SHA-256 hash of an API key string.
-  `tests` module L47-72 — `-` — API key generation and hashing utilities.
-  `test_generate_api_key_format` function L51-58 — `()` — API key generation and hashing utilities.
-  `test_hash_api_key_deterministic` function L61-64 — `()` — API key generation and hashing utilities.
-  `test_generate_api_key_uniqueness` function L67-71 — `()` — API key generation and hashing utilities.

#### crates/cloacina/src/security/audit.rs

- pub `events` module L30-63 — `-` — Event types for package operations.
- pub `PACKAGE_LOAD_SUCCESS` variable L32 — `: &str` — Package load success event type.
- pub `PACKAGE_LOAD_FAILURE` variable L34 — `: &str` — Package load failure event type.
- pub `PACKAGE_SIGNED` variable L36 — `: &str` — Package signed event type.
- pub `PACKAGE_SIGN_FAILURE` variable L38 — `: &str` — Package sign failure event type.
- pub `KEY_SIGNING_CREATED` variable L41 — `: &str` — Signing key created event type.
- pub `KEY_SIGNING_CREATE_FAILED` variable L43 — `: &str` — Signing key create failure event type.
- pub `KEY_SIGNING_REVOKED` variable L45 — `: &str` — Signing key revoked event type.
- pub `KEY_EXPORTED` variable L47 — `: &str` — Signing key exported event type.
- pub `KEY_TRUSTED_ADDED` variable L50 — `: &str` — Trusted key added event type.
- pub `KEY_TRUSTED_REVOKED` variable L52 — `: &str` — Trusted key revoked event type.
- pub `KEY_TRUST_ACL_GRANTED` variable L55 — `: &str` — Trust ACL granted event type.
- pub `KEY_TRUST_ACL_REVOKED` variable L57 — `: &str` — Trust ACL revoked event type.
- pub `VERIFICATION_SUCCESS` variable L60 — `: &str` — Verification success event type.
- pub `VERIFICATION_FAILURE` variable L62 — `: &str` — Verification failure event type.
- pub `log_signing_key_created` function L66-80 — `( org_id: UniversalUuid, key_id: UniversalUuid, key_fingerprint: &str, key_name:...` — Log a signing key creation event.
- pub `log_signing_key_create_failed` function L83-91 — `(org_id: UniversalUuid, key_name: &str, error: &str)` — Log a signing key creation failure.
- pub `log_signing_key_revoked` function L94-108 — `( org_id: UniversalUuid, key_id: UniversalUuid, key_fingerprint: &str, key_name:...` — Log a signing key revocation event.
- pub `log_key_exported` function L111-118 — `(key_id: UniversalUuid, key_fingerprint: &str)` — Log a public key export event.
- pub `log_trusted_key_added` function L121-135 — `( org_id: UniversalUuid, key_id: UniversalUuid, key_fingerprint: &str, key_name:...` — Log a trusted key addition event.
- pub `log_trusted_key_revoked` function L138-144 — `(key_id: UniversalUuid)` — Log a trusted key revocation event.
- pub `log_trust_acl_granted` function L147-154 — `(parent_org: UniversalUuid, child_org: UniversalUuid)` — Log a trust ACL grant event.
- pub `log_trust_acl_revoked` function L157-164 — `(parent_org: UniversalUuid, child_org: UniversalUuid)` — Log a trust ACL revocation event.
- pub `log_package_signed` function L167-175 — `(package_path: &str, package_hash: &str, key_fingerprint: &str)` — Log a package signing event.
- pub `log_package_sign_failed` function L178-185 — `(package_path: &str, error: &str)` — Log a package signing failure.
- pub `log_package_load_success` function L188-204 — `( org_id: UniversalUuid, package_path: &str, package_hash: &str, signer_fingerpr...` — Log a package load success event.
- pub `log_package_load_failure` function L207-221 — `( org_id: UniversalUuid, package_path: &str, error: &str, failure_reason: &str, ...` — Log a package load failure event.
- pub `log_verification_success` function L224-238 — `( org_id: UniversalUuid, package_hash: &str, signer_fingerprint: &str, signer_na...` — Log a verification success event.
- pub `log_verification_failure` function L241-255 — `( org_id: UniversalUuid, package_hash: &str, failure_reason: &str, signer_finger...` — Log a verification failure event.
-  `tests` module L258-537 — `-` — Events are logged using the `tracing` crate at appropriate levels.
-  `StringWriter` struct L264 — `-` — Events are logged using the `tracing` crate at appropriate levels.
-  `StringWriter` type L266-275 — `= StringWriter` — Events are logged using the `tracing` crate at appropriate levels.
-  `write` function L267-270 — `(&mut self, buf: &[u8]) -> std::io::Result<usize>` — Events are logged using the `tracing` crate at appropriate levels.
-  `flush` function L272-274 — `(&mut self) -> std::io::Result<()>` — Events are logged using the `tracing` crate at appropriate levels.
-  `StringWriter` type L277-283 — `= StringWriter` — Events are logged using the `tracing` crate at appropriate levels.
-  `Writer` type L278 — `= StringWriter` — Events are logged using the `tracing` crate at appropriate levels.
-  `make_writer` function L280-282 — `(&'a self) -> Self::Writer` — Events are logged using the `tracing` crate at appropriate levels.
-  `with_captured_logs` function L286-302 — `(f: F) -> String` — Events are logged using the `tracing` crate at appropriate levels.
-  `test_log_signing_key_created` function L305-318 — `()` — Events are logged using the `tracing` crate at appropriate levels.
-  `test_log_verification_failure` function L321-334 — `()` — Events are logged using the `tracing` crate at appropriate levels.
-  `test_log_package_load_success` function L337-351 — `()` — Events are logged using the `tracing` crate at appropriate levels.
-  `test_log_trust_acl_granted` function L354-362 — `()` — Events are logged using the `tracing` crate at appropriate levels.
-  `test_event_type_constants` function L365-370 — `()` — Events are logged using the `tracing` crate at appropriate levels.
-  `test_log_signing_key_create_failed` function L373-380 — `()` — Events are logged using the `tracing` crate at appropriate levels.
-  `test_log_signing_key_revoked` function L383-395 — `()` — Events are logged using the `tracing` crate at appropriate levels.
-  `test_log_signing_key_revoked_no_name` function L398-409 — `()` — Events are logged using the `tracing` crate at appropriate levels.
-  `test_log_key_exported` function L412-418 — `()` — Events are logged using the `tracing` crate at appropriate levels.
-  `test_log_trusted_key_added` function L421-433 — `()` — Events are logged using the `tracing` crate at appropriate levels.
-  `test_log_trusted_key_added_no_name` function L436-447 — `()` — Events are logged using the `tracing` crate at appropriate levels.
-  `test_log_trusted_key_revoked` function L450-455 — `()` — Events are logged using the `tracing` crate at appropriate levels.
-  `test_log_trust_acl_revoked` function L458-465 — `()` — Events are logged using the `tracing` crate at appropriate levels.
-  `test_log_package_signed` function L468-476 — `()` — Events are logged using the `tracing` crate at appropriate levels.
-  `test_log_package_sign_failed` function L479-486 — `()` — Events are logged using the `tracing` crate at appropriate levels.
-  `test_log_package_load_failure` function L489-502 — `()` — Events are logged using the `tracing` crate at appropriate levels.
-  `test_log_verification_success` function L505-518 — `()` — Events are logged using the `tracing` crate at appropriate levels.
-  `test_log_verification_success_no_name` function L521-527 — `()` — Events are logged using the `tracing` crate at appropriate levels.
-  `test_log_verification_failure_no_fingerprint` function L530-536 — `()` — Events are logged using the `tracing` crate at appropriate levels.

#### crates/cloacina/src/security/db_key_manager.rs

- pub `DbKeyManager` struct L59-61 — `{ dal: DAL }` — Database-backed implementation of the [`KeyManager`] trait.
- pub `new` function L65-67 — `(dal: DAL) -> Self` — Creates a new database-backed key manager.
-  `ED25519_PEM_TAG` variable L39 — `: &str` — PEM tag for Ed25519 public keys.
-  `ED25519_DER_PREFIX` variable L43-50 — `: [u8; 12]` — ASN.1 DER prefix for Ed25519 public keys (SubjectPublicKeyInfo).
-  `DbKeyManager` type L63-139 — `= DbKeyManager` — AES-256-GCM.
-  `encode_public_key_pem` function L70-79 — `(public_key: &[u8]) -> String` — Encodes a raw Ed25519 public key to PEM format.
-  `decode_public_key_pem` function L82-112 — `(pem_str: &str) -> Result<Vec<u8>, KeyError>` — Decodes a PEM-encoded Ed25519 public key to raw bytes.
-  `to_signing_key_info` function L115-125 — `(key: UnifiedSigningKey) -> SigningKeyInfo` — Convert database model to SigningKeyInfo.
-  `to_trusted_key_info` function L128-138 — `(key: UnifiedTrustedKey) -> TrustedKeyInfo` — Convert database model to TrustedKeyInfo.
-  `DbKeyManager` type L142-502 — `impl KeyManager for DbKeyManager` — AES-256-GCM.
-  `create_signing_key` function L143-204 — `( &self, org_id: UniversalUuid, name: &str, master_key: &[u8], ) -> Result<Signi...` — AES-256-GCM.
-  `get_signing_key_info` function L206-215 — `( &self, key_id: UniversalUuid, ) -> Result<SigningKeyInfo, KeyError>` — AES-256-GCM.
-  `get_signing_key` function L217-237 — `( &self, key_id: UniversalUuid, master_key: &[u8], ) -> Result<(Vec<u8>, Vec<u8>...` — AES-256-GCM.
-  `export_public_key` function L239-250 — `(&self, key_id: UniversalUuid) -> Result<PublicKeyExport, KeyError>` — AES-256-GCM.
-  `trust_public_key` function L252-310 — `( &self, org_id: UniversalUuid, public_key: &[u8], name: Option<&str>, ) -> Resu...` — AES-256-GCM.
-  `trust_public_key_pem` function L312-320 — `( &self, org_id: UniversalUuid, pem: &str, name: Option<&str>, ) -> Result<Trust...` — AES-256-GCM.
-  `revoke_signing_key` function L322-341 — `(&self, key_id: UniversalUuid) -> Result<(), KeyError>` — AES-256-GCM.
-  `revoke_trusted_key` function L343-354 — `(&self, key_id: UniversalUuid) -> Result<(), KeyError>` — AES-256-GCM.
-  `grant_trust` function L356-393 — `( &self, parent_org: UniversalUuid, child_org: UniversalUuid, ) -> Result<(), Ke...` — AES-256-GCM.
-  `revoke_trust` function L395-410 — `( &self, parent_org: UniversalUuid, child_org: UniversalUuid, ) -> Result<(), Ke...` — AES-256-GCM.
-  `list_signing_keys` function L412-421 — `( &self, org_id: UniversalUuid, ) -> Result<Vec<SigningKeyInfo>, KeyError>` — AES-256-GCM.
-  `list_trusted_keys` function L423-459 — `( &self, org_id: UniversalUuid, ) -> Result<Vec<TrustedKeyInfo>, KeyError>` — AES-256-GCM.
-  `find_trusted_key` function L461-501 — `( &self, org_id: UniversalUuid, fingerprint: &str, ) -> Result<Option<TrustedKey...` — AES-256-GCM.
-  `DbKeyManager` type L506-837 — `= DbKeyManager` — AES-256-GCM.
-  `create_signing_key_postgres` function L507-536 — `( &self, new_key: NewUnifiedSigningKey, ) -> Result<(), KeyError>` — AES-256-GCM.
-  `get_signing_key_info_postgres` function L538-566 — `( &self, key_id: UniversalUuid, ) -> Result<SigningKeyInfo, KeyError>` — AES-256-GCM.
-  `get_signing_key_raw_postgres` function L568-593 — `( &self, key_id: UniversalUuid, ) -> Result<UnifiedSigningKey, KeyError>` — AES-256-GCM.
-  `create_trusted_key_postgres` function L595-616 — `( &self, new_key: NewUnifiedTrustedKey, ) -> Result<(), KeyError>` — AES-256-GCM.
-  `revoke_signing_key_postgres` function L618-643 — `(&self, key_id: UniversalUuid) -> Result<(), KeyError>` — AES-256-GCM.
-  `revoke_trusted_key_postgres` function L645-670 — `(&self, key_id: UniversalUuid) -> Result<(), KeyError>` — AES-256-GCM.
-  `grant_trust_postgres` function L672-696 — `(&self, new_acl: NewUnifiedKeyTrustAcl) -> Result<(), KeyError>` — AES-256-GCM.
-  `revoke_trust_postgres` function L698-732 — `( &self, parent_org: UniversalUuid, child_org: UniversalUuid, ) -> Result<(), Ke...` — AES-256-GCM.
-  `list_signing_keys_postgres` function L734-756 — `( &self, org_id: UniversalUuid, ) -> Result<Vec<SigningKeyInfo>, KeyError>` — AES-256-GCM.
-  `list_direct_trusted_keys_postgres` function L758-781 — `( &self, org_id: UniversalUuid, ) -> Result<Vec<TrustedKeyInfo>, KeyError>` — AES-256-GCM.
-  `get_trusted_child_orgs_postgres` function L783-806 — `( &self, org_id: UniversalUuid, ) -> Result<Vec<UniversalUuid>, KeyError>` — AES-256-GCM.
-  `find_direct_trusted_key_postgres` function L808-836 — `( &self, org_id: UniversalUuid, fingerprint: &str, ) -> Result<Option<TrustedKey...` — AES-256-GCM.
-  `DbKeyManager` type L841-1172 — `= DbKeyManager` — AES-256-GCM.
-  `create_signing_key_sqlite` function L842-871 — `( &self, new_key: NewUnifiedSigningKey, ) -> Result<(), KeyError>` — AES-256-GCM.
-  `get_signing_key_info_sqlite` function L873-901 — `( &self, key_id: UniversalUuid, ) -> Result<SigningKeyInfo, KeyError>` — AES-256-GCM.
-  `get_signing_key_raw_sqlite` function L903-928 — `( &self, key_id: UniversalUuid, ) -> Result<UnifiedSigningKey, KeyError>` — AES-256-GCM.
-  `create_trusted_key_sqlite` function L930-951 — `( &self, new_key: NewUnifiedTrustedKey, ) -> Result<(), KeyError>` — AES-256-GCM.
-  `revoke_signing_key_sqlite` function L953-978 — `(&self, key_id: UniversalUuid) -> Result<(), KeyError>` — AES-256-GCM.
-  `revoke_trusted_key_sqlite` function L980-1005 — `(&self, key_id: UniversalUuid) -> Result<(), KeyError>` — AES-256-GCM.
-  `grant_trust_sqlite` function L1007-1031 — `(&self, new_acl: NewUnifiedKeyTrustAcl) -> Result<(), KeyError>` — AES-256-GCM.
-  `revoke_trust_sqlite` function L1033-1067 — `( &self, parent_org: UniversalUuid, child_org: UniversalUuid, ) -> Result<(), Ke...` — AES-256-GCM.
-  `list_signing_keys_sqlite` function L1069-1091 — `( &self, org_id: UniversalUuid, ) -> Result<Vec<SigningKeyInfo>, KeyError>` — AES-256-GCM.
-  `list_direct_trusted_keys_sqlite` function L1093-1116 — `( &self, org_id: UniversalUuid, ) -> Result<Vec<TrustedKeyInfo>, KeyError>` — AES-256-GCM.
-  `get_trusted_child_orgs_sqlite` function L1118-1141 — `( &self, org_id: UniversalUuid, ) -> Result<Vec<UniversalUuid>, KeyError>` — AES-256-GCM.
-  `find_direct_trusted_key_sqlite` function L1143-1171 — `( &self, org_id: UniversalUuid, fingerprint: &str, ) -> Result<Option<TrustedKey...` — AES-256-GCM.
-  `tests` module L1175-1835 — `-` — AES-256-GCM.
-  `test_pem_roundtrip` function L1182-1191 — `()` — AES-256-GCM.
-  `test_pem_roundtrip_all_zeros` function L1194-1199 — `()` — AES-256-GCM.
-  `test_pem_roundtrip_all_ones` function L1202-1207 — `()` — AES-256-GCM.
-  `test_pem_roundtrip_random_key` function L1210-1215 — `()` — AES-256-GCM.
-  `test_invalid_pem` function L1218-1226 — `()` — AES-256-GCM.
-  `test_decode_pem_wrong_length` function L1229-1239 — `()` — AES-256-GCM.
-  `test_decode_pem_wrong_der_prefix` function L1242-1253 — `()` — AES-256-GCM.
-  `test_encode_pem_contains_expected_header_footer` function L1256-1260 — `()` — AES-256-GCM.
-  `unique_dal` function L1265-1275 — `() -> DAL` — AES-256-GCM.
-  `master_key` function L1278-1280 — `() -> [u8; 32]` — AES-256-GCM.
-  `test_create_signing_key` function L1286-1301 — `()` — AES-256-GCM.
-  `test_get_signing_key_info` function L1305-1320 — `()` — AES-256-GCM.
-  `test_get_signing_key_info_not_found` function L1324-1330 — `()` — AES-256-GCM.
-  `test_get_signing_key_decrypt` function L1334-1347 — `()` — AES-256-GCM.
-  `test_get_signing_key_wrong_master_key` function L1351-1364 — `()` — AES-256-GCM.
-  `test_get_signing_key_revoked_fails` function L1368-1382 — `()` — AES-256-GCM.
-  `test_list_signing_keys` function L1386-1411 — `()` — AES-256-GCM.
-  `test_revoke_signing_key` function L1415-1431 — `()` — AES-256-GCM.
-  `test_revoke_signing_key_not_found` function L1435-1441 — `()` — AES-256-GCM.
-  `test_export_public_key` function L1445-1463 — `()` — AES-256-GCM.
-  `test_trust_public_key` function L1467-1482 — `()` — AES-256-GCM.
-  `test_trust_public_key_invalid_length` function L1486-1493 — `()` — AES-256-GCM.
-  `test_trust_public_key_pem` function L1497-1512 — `()` — AES-256-GCM.
-  `test_trust_public_key_pem_invalid` function L1516-1523 — `()` — AES-256-GCM.
-  `test_list_trusted_keys` function L1527-1548 — `()` — AES-256-GCM.
-  `test_revoke_trusted_key` function L1552-1568 — `()` — AES-256-GCM.
-  `test_revoke_trusted_key_not_found` function L1572-1578 — `()` — AES-256-GCM.
-  `test_find_trusted_key_direct` function L1582-1599 — `()` — AES-256-GCM.
-  `test_find_trusted_key_not_found` function L1603-1613 — `()` — AES-256-GCM.
-  `test_find_trusted_key_revoked_not_found` function L1617-1635 — `()` — AES-256-GCM.
-  `test_grant_trust` function L1641-1658 — `()` — AES-256-GCM.
-  `test_grant_trust_find_inherited_key` function L1662-1678 — `()` — AES-256-GCM.
-  `test_revoke_trust` function L1682-1700 — `()` — AES-256-GCM.
-  `test_revoke_trust_not_found` function L1704-1712 — `()` — AES-256-GCM.
-  `test_create_key_sign_and_verify_roundtrip` function L1718-1740 — `()` — AES-256-GCM.
-  `test_export_and_import_roundtrip` function L1744-1766 — `()` — AES-256-GCM.
-  `test_list_signing_keys_includes_revoked` function L1770-1785 — `()` — AES-256-GCM.
-  `test_list_trusted_keys_deduplicates_across_acl` function L1789-1810 — `()` — AES-256-GCM.
-  `test_multiple_orgs_isolated` function L1814-1834 — `()` — AES-256-GCM.

#### crates/cloacina/src/security/key_manager.rs

- pub `KeyError` enum L28-58 — `NotFound | Revoked | DuplicateName | InvalidFormat | InvalidPem | Encryption | D...` — Errors that can occur during key management operations.
- pub `SigningKeyInfo` struct L62-72 — `{ id: UniversalUuid, org_id: UniversalUuid, key_name: String, fingerprint: Strin...` — Information about a signing key (excludes private key material).
- pub `is_active` function L76-78 — `(&self) -> bool` — Check if this key is currently active (not revoked).
- pub `TrustedKeyInfo` struct L83-94 — `{ id: UniversalUuid, org_id: UniversalUuid, fingerprint: String, public_key: Vec...` — Information about a trusted public key for verification.
- pub `is_active` function L98-100 — `(&self) -> bool` — Check if this key is currently trusted (not revoked).
- pub `PublicKeyExport` struct L105-112 — `{ fingerprint: String, public_key_pem: String, public_key_raw: Vec<u8> }` — Public key export in multiple formats.
- pub `KeyManager` interface L119-226 — `{ fn create_signing_key(), fn get_signing_key_info(), fn get_signing_key(), fn e...` — Trait for managing signing keys, trusted keys, and trust relationships.
-  `SigningKeyInfo` type L74-79 — `= SigningKeyInfo` — trusted public keys, and trust relationships between organizations.
-  `TrustedKeyInfo` type L96-101 — `= TrustedKeyInfo` — trusted public keys, and trust relationships between organizations.

#### crates/cloacina/src/security/mod.rs

- pub `api_keys` module L25 — `-` — Security module for package signing and key management.
- pub `audit` module L26 — `-` — - Security audit logging for SIEM integration
-  `db_key_manager` module L27 — `-` — - Security audit logging for SIEM integration
-  `key_manager` module L28 — `-` — - Security audit logging for SIEM integration
-  `package_signer` module L29 — `-` — - Security audit logging for SIEM integration
-  `verification` module L30 — `-` — - Security audit logging for SIEM integration

#### crates/cloacina/src/security/package_signer.rs

- pub `PackageSignError` enum L40-64 — `FileReadError | SigningFailed | KeyNotFound | KeyRevoked | Database | SignatureN...` — Errors that can occur during package signing operations.
- pub `PackageSignatureInfo` struct L68-77 — `{ package_hash: String, key_fingerprint: String, signature: Vec<u8>, signed_at: ...` — A package signature with all metadata.
- pub `DetachedSignature` struct L84-97 — `{ version: u32, algorithm: String, package_hash: String, key_fingerprint: String...` — Detached signature file format.
- pub `VERSION` variable L101 — `: u32` — Current signature format version.
- pub `ALGORITHM` variable L104 — `: &'static str` — Algorithm identifier for Ed25519.
- pub `from_signature_info` function L107-116 — `(info: &PackageSignatureInfo) -> Self` — Create a detached signature from signature info.
- pub `from_json` function L119-122 — `(json: &str) -> Result<Self, PackageSignError>` — Parse a detached signature from JSON.
- pub `to_json` function L125-128 — `(&self) -> Result<String, PackageSignError>` — Serialize to JSON.
- pub `signature_bytes` function L131-135 — `(&self) -> Result<Vec<u8>, PackageSignError>` — Get the raw signature bytes.
- pub `write_to_file` function L138-142 — `(&self, path: &Path) -> Result<(), PackageSignError>` — Write the detached signature to a file.
- pub `read_from_file` function L145-148 — `(path: &Path) -> Result<Self, PackageSignError>` — Read a detached signature from a file.
- pub `PackageSigner` interface L153-241 — `{ fn sign_package_with_db_key(), fn sign_package_with_raw_key(), fn sign_package...` — Trait for signing packages and managing signatures.
- pub `DbPackageSigner` struct L245-247 — `{ dal: DAL }` — Database-backed package signer implementation.
- pub `new` function L251-253 — `(dal: DAL) -> Self` — Create a new database-backed package signer.
-  `DetachedSignature` type L99-149 — `= DetachedSignature` — - [`DetachedSignature`] format for standalone signature files
-  `DbPackageSigner` type L249-277 — `= DbPackageSigner` — - [`DetachedSignature`] format for standalone signature files
-  `compute_file_hash` function L256-259 — `(path: &Path) -> Result<String, PackageSignError>` — Compute the SHA256 hash of a file.
-  `compute_data_hash` function L262-266 — `(data: &[u8]) -> Result<String, PackageSignError>` — Compute the SHA256 hash of data.
-  `to_signature_info` function L269-276 — `(sig: UnifiedPackageSignature) -> PackageSignatureInfo` — Convert database model to SignatureInfo.
-  `DbPackageSigner` type L280-509 — `impl PackageSigner for DbPackageSigner` — - [`DetachedSignature`] format for standalone signature files
-  `sign_package_with_db_key` function L281-329 — `( &self, package_path: &Path, key_id: UniversalUuid, master_key: &[u8], store_si...` — - [`DetachedSignature`] format for standalone signature files
-  `sign_package_with_raw_key` function L331-339 — `( &self, package_path: &Path, private_key: &[u8], public_key: &[u8], ) -> Result...` — - [`DetachedSignature`] format for standalone signature files
-  `sign_package_data` function L341-366 — `( &self, package_data: &[u8], private_key: &[u8], public_key: &[u8], ) -> Result...` — - [`DetachedSignature`] format for standalone signature files
-  `store_signature` function L368-403 — `( &self, signature: &PackageSignatureInfo, ) -> Result<UniversalUuid, PackageSig...` — - [`DetachedSignature`] format for standalone signature files
-  `find_signature` function L405-414 — `( &self, package_hash: &str, ) -> Result<Option<PackageSignatureInfo>, PackageSi...` — - [`DetachedSignature`] format for standalone signature files
-  `find_signatures` function L416-425 — `( &self, package_hash: &str, ) -> Result<Vec<PackageSignatureInfo>, PackageSignE...` — - [`DetachedSignature`] format for standalone signature files
-  `verify_package` function L427-465 — `( &self, package_path: &Path, org_id: UniversalUuid, ) -> Result<PackageSignatur...` — - [`DetachedSignature`] format for standalone signature files
-  `verify_package_with_detached_signature` function L467-508 — `( &self, package_path: &Path, signature: &DetachedSignature, public_key: &[u8], ...` — - [`DetachedSignature`] format for standalone signature files
-  `DbPackageSigner` type L513-589 — `= DbPackageSigner` — - [`DetachedSignature`] format for standalone signature files
-  `store_signature_postgres` function L514-535 — `( &self, new_sig: NewUnifiedPackageSignature, ) -> Result<(), PackageSignError>` — - [`DetachedSignature`] format for standalone signature files
-  `find_signature_postgres` function L537-562 — `( &self, package_hash: &str, ) -> Result<Option<PackageSignatureInfo>, PackageSi...` — - [`DetachedSignature`] format for standalone signature files
-  `find_signatures_postgres` function L564-588 — `( &self, package_hash: &str, ) -> Result<Vec<PackageSignatureInfo>, PackageSignE...` — - [`DetachedSignature`] format for standalone signature files
-  `DbPackageSigner` type L593-669 — `= DbPackageSigner` — - [`DetachedSignature`] format for standalone signature files
-  `store_signature_sqlite` function L594-615 — `( &self, new_sig: NewUnifiedPackageSignature, ) -> Result<(), PackageSignError>` — - [`DetachedSignature`] format for standalone signature files
-  `find_signature_sqlite` function L617-642 — `( &self, package_hash: &str, ) -> Result<Option<PackageSignatureInfo>, PackageSi...` — - [`DetachedSignature`] format for standalone signature files
-  `find_signatures_sqlite` function L644-668 — `( &self, package_hash: &str, ) -> Result<Vec<PackageSignatureInfo>, PackageSignE...` — - [`DetachedSignature`] format for standalone signature files
-  `tests` module L672-1253 — `-` — - [`DetachedSignature`] format for standalone signature files
-  `test_sign_and_verify_with_raw_key` function L678-698 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_detached_signature_roundtrip` function L701-720 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_detached_signature_file_io` function L723-740 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_compute_data_hash_deterministic` function L743-748 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_compute_data_hash_different_inputs` function L751-755 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_compute_data_hash_empty_input` function L758-762 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_compute_data_hash_large_payload` function L765-769 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_compute_file_hash_matches_data_hash` function L772-780 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_compute_file_hash_nonexistent_file` function L783-786 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_detached_signature_invalid_json` function L789-792 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_detached_signature_version_and_algorithm` function L795-805 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_detached_signature_corrupted_base64` function L808-819 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_sign_verify_roundtrip_different_data` function L822-841 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_sign_verify_wrong_key_fails` function L844-856 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_sign_verify_tampered_data_fails` function L859-873 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `db_tests` module L878-1252 — `-` — - [`DetachedSignature`] format for standalone signature files
-  `unique_dal` function L885-895 — `() -> DAL` — - [`DetachedSignature`] format for standalone signature files
-  `master_key` function L897-899 — `() -> [u8; 32]` — - [`DetachedSignature`] format for standalone signature files
-  `test_sign_package_data_with_raw_key` function L902-915 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_sign_package_with_raw_key_file` function L918-936 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_store_and_find_signature` function L939-956 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_find_signature_not_found` function L959-965 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_find_signatures_multiple` function L968-987 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_sign_package_with_db_key` function L990-1012 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_sign_package_with_db_key_and_store` function L1015-1038 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_sign_package_with_db_key_revoked_fails` function L1041-1061 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_sign_package_with_db_key_not_found` function L1064-1080 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_verify_package_with_detached_signature` function L1083-1108 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_verify_package_detached_tampered_fails` function L1111-1141 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_verify_package_detached_wrong_key_fails` function L1144-1171 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_verify_package_detached_wrong_algorithm` function L1174-1201 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_verify_package_trusted_key` function L1204-1235 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_verify_package_no_signature_fails` function L1238-1251 — `()` — - [`DetachedSignature`] format for standalone signature files

#### crates/cloacina/src/security/verification.rs

- pub `SecurityConfig` struct L36-50 — `{ require_signatures: bool, key_encryption_key: Option<[u8; 32]> }` — Security configuration for package verification.
- pub `require_signatures` function L54-59 — `() -> Self` — Create a security config that requires signatures.
- pub `development` function L62-64 — `() -> Self` — Create a security config with no signature requirements (for development).
- pub `with_encryption_key` function L67-70 — `(mut self, key: [u8; 32]) -> Self` — Set the key encryption key for signing operations.
- pub `VerificationError` enum L77-130 — `TamperedPackage | UntrustedSigner | InvalidSignature | SignatureNotFound | Malfo...` — Errors that occur during package verification.
- pub `SignatureSource` enum L134-147 — `Database | DetachedFile | Auto` — Where to find the signature for a package.
- pub `VerificationResult` struct L151-158 — `{ package_hash: String, signer_fingerprint: String, signer_name: Option<String> ...` — Result of successful verification.
- pub `verify_package` function L179-291 — `( package_path: P, org_id: UniversalUuid, signature_source: SignatureSource, pac...` — Verify a package signature.
- pub `verify_package_offline` function L306-365 — `( package_path: P, signature_path: S, public_key: &[u8], ) -> Result<Verificatio...` — Verify a package using only a detached signature and public key (offline mode).
-  `SecurityConfig` type L52-71 — `= SecurityConfig` — - [`verify_and_load_package`] for verified package loading
-  `compute_package_hash` function L368-374 — `(data: &[u8]) -> Result<String, VerificationError>` — Compute SHA256 hash of package data.
-  `load_signature_from_db` function L377-392 — `( package_hash: &str, package_signer: &DbPackageSigner, ) -> Result<DetachedSign...` — Load signature from database.
-  `load_signature_from_file` function L395-399 — `(path: &Path) -> Result<DetachedSignature, VerificationError>` — Load signature from file.
-  `tests` module L402-648 — `-` — - [`verify_and_load_package`] for verified package loading
-  `test_security_config_default` function L409-413 — `()` — - [`verify_and_load_package`] for verified package loading
-  `test_security_config_require_signatures` function L416-419 — `()` — - [`verify_and_load_package`] for verified package loading
-  `test_security_config_with_encryption_key` function L422-426 — `()` — - [`verify_and_load_package`] for verified package loading
-  `test_verify_package_offline_with_invalid_signature` function L429-458 — `()` — - [`verify_and_load_package`] for verified package loading
-  `test_signature_source_default` function L461-464 — `()` — - [`verify_and_load_package`] for verified package loading
-  `test_verify_package_offline_valid_signature` function L467-502 — `()` — - [`verify_and_load_package`] for verified package loading
-  `test_verify_package_offline_tampered_content` function L505-542 — `()` — - [`verify_and_load_package`] for verified package loading
-  `test_verify_package_offline_wrong_key` function L545-580 — `()` — - [`verify_and_load_package`] for verified package loading
-  `test_verify_package_offline_nonexistent_package` function L583-603 — `()` — - [`verify_and_load_package`] for verified package loading
-  `test_verify_package_offline_nonexistent_signature` function L606-617 — `()` — - [`verify_and_load_package`] for verified package loading
-  `test_load_signature_from_file_valid` function L620-635 — `()` — - [`verify_and_load_package`] for verified package loading
-  `test_load_signature_from_file_invalid` function L638-647 — `()` — - [`verify_and_load_package`] for verified package loading

### crates/cloacina/src/trigger

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/trigger/mod.rs

- pub `registry` module L51 — `-` — # Trigger System
- pub `TriggerError` enum L65-89 — `PollError | ContextError | TriggerNotFound | Database | ConnectionPool | Workflo...` — Errors that can occur during trigger operations.
- pub `TriggerResult` enum L115-124 — `Skip | Fire` — Result of a trigger poll operation.
- pub `should_fire` function L137-139 — `(&self) -> bool` — Returns true if this result indicates the workflow should fire.
- pub `into_context` function L142-147 — `(self) -> Option<Context<serde_json::Value>>` — Extracts the context if this is a Fire result.
- pub `context_hash` function L153-166 — `(&self) -> String` — Computes a hash of the context for deduplication purposes.
- pub `TriggerConfig` struct L174-189 — `{ name: String, workflow_name: String, poll_interval: Duration, allow_concurrent...` — Configuration for a trigger.
- pub `new` function L193-201 — `(name: &str, workflow_name: &str, poll_interval: Duration) -> Self` — Creates a new trigger configuration.
- pub `with_allow_concurrent` function L204-207 — `(mut self, allow: bool) -> Self` — Sets whether concurrent executions are allowed.
- pub `with_enabled` function L210-213 — `(mut self, enabled: bool) -> Self` — Sets whether the trigger is enabled.
- pub `Trigger` interface L275-296 — `{ fn name(), fn poll_interval(), fn allow_concurrent(), fn poll() }` — Core trait for user-defined triggers.
-  `TriggerError` type L91-95 — `= TriggerError` — ```
-  `from` function L92-94 — `(err: deadpool::managed::PoolError<deadpool_diesel::Error>) -> Self` — ```
-  `TriggerError` type L97-108 — `= TriggerError` — ```
-  `from` function L98-107 — `(err: cloacina_workflow::TriggerError) -> Self` — ```
-  `TriggerResult` type L126-133 — `= TriggerResult` — ```
-  `from` function L127-132 — `(r: cloacina_workflow::TriggerResult) -> Self` — ```
-  `TriggerResult` type L135-167 — `= TriggerResult` — ```
-  `TriggerConfig` type L191-214 — `= TriggerConfig` — ```
-  `tests` module L305-420 — `-` — ```
-  `TestTrigger` struct L309-312 — `{ name: String, should_fire: bool }` — ```
-  `TestTrigger` type L315-335 — `impl Trigger for TestTrigger` — ```
-  `name` function L316-318 — `(&self) -> &str` — ```
-  `poll_interval` function L320-322 — `(&self) -> Duration` — ```
-  `allow_concurrent` function L324-326 — `(&self) -> bool` — ```
-  `poll` function L328-334 — `(&self) -> Result<TriggerResult, TriggerError>` — ```
-  `test_trigger_result_should_fire` function L338-342 — `()` — ```
-  `test_trigger_result_into_context` function L345-352 — `()` — ```
-  `test_trigger_result_context_hash` function L355-379 — `()` — ```
-  `test_trigger_config` function L382-393 — `()` — ```
-  `test_trigger_trait` function L396-408 — `()` — ```
-  `test_trigger_fires` function L411-419 — `()` — ```

#### crates/cloacina/src/trigger/registry.rs

- pub `register_trigger_constructor` function L59-67 — `(name: impl Into<String>, constructor: F)` — Register a trigger constructor function globally.
- pub `register_trigger` function L76-79 — `(trigger: T)` — Register a trigger instance directly.
- pub `get_trigger` function L91-94 — `(name: &str) -> Option<Arc<dyn Trigger>>` — Get a trigger instance from the global registry by name.
- pub `global_trigger_registry` function L100-102 — `() -> GlobalTriggerRegistry` — Get the global trigger registry.
- pub `list_triggers` function L109-112 — `() -> Vec<String>` — Get all registered trigger names.
- pub `get_all_triggers` function L119-122 — `() -> Vec<Arc<dyn Trigger>>` — Get all registered triggers.
- pub `deregister_trigger` function L133-136 — `(name: &str) -> bool` — Deregister a trigger by name.
- pub `is_trigger_registered` function L147-150 — `(name: &str) -> bool` — Check if a trigger is registered.
- pub `clear_triggers` function L156-159 — `()` — Clear all registered triggers.
-  `TriggerConstructor` type L30 — `= Box<dyn Fn() -> Arc<dyn Trigger> + Send + Sync>` — Type alias for the trigger constructor function stored in the global registry
-  `GlobalTriggerRegistry` type L33 — `= Arc<RwLock<HashMap<String, TriggerConstructor>>>` — Type alias for the global trigger registry
-  `GLOBAL_TRIGGER_REGISTRY` variable L36-37 — `: Lazy<GlobalTriggerRegistry>` — Global registry for automatically registering triggers created with the `#[trigger]` macro
-  `tests` module L162-328 — `-` — Triggers registered here are available for use by the TriggerScheduler.
-  `TestTrigger` struct L170-172 — `{ name: String }` — Triggers registered here are available for use by the TriggerScheduler.
-  `TestTrigger` type L174-180 — `= TestTrigger` — Triggers registered here are available for use by the TriggerScheduler.
-  `new` function L175-179 — `(name: &str) -> Self` — Triggers registered here are available for use by the TriggerScheduler.
-  `TestTrigger` type L183-199 — `impl Trigger for TestTrigger` — Triggers registered here are available for use by the TriggerScheduler.
-  `name` function L184-186 — `(&self) -> &str` — Triggers registered here are available for use by the TriggerScheduler.
-  `poll_interval` function L188-190 — `(&self) -> Duration` — Triggers registered here are available for use by the TriggerScheduler.
-  `allow_concurrent` function L192-194 — `(&self) -> bool` — Triggers registered here are available for use by the TriggerScheduler.
-  `poll` function L196-198 — `(&self) -> Result<TriggerResult, TriggerError>` — Triggers registered here are available for use by the TriggerScheduler.
-  `test_register_and_get_trigger` function L206-218 — `()` — Triggers registered here are available for use by the TriggerScheduler.
-  `test_register_constructor` function L222-229 — `()` — Triggers registered here are available for use by the TriggerScheduler.
-  `test_list_triggers` function L233-245 — `()` — Triggers registered here are available for use by the TriggerScheduler.
-  `test_get_all_triggers` function L249-262 — `()` — Triggers registered here are available for use by the TriggerScheduler.
-  `test_deregister_trigger` function L266-278 — `()` — Triggers registered here are available for use by the TriggerScheduler.
-  `test_register_deregister_roundtrip` function L282-301 — `()` — Triggers registered here are available for use by the TriggerScheduler.
-  `test_clear_triggers` function L305-327 — `()` — Triggers registered here are available for use by the TriggerScheduler.

### crates/cloacina/src/workflow

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/workflow/builder.rs

- pub `WorkflowBuilder` struct L75-77 — `{ workflow: Workflow }` — Builder pattern for convenient and fluent Workflow construction.
- pub `new` function L81-85 — `(name: &str) -> Self` — Create a new workflow builder
- pub `name` function L88-90 — `(&self) -> &str` — Get the workflow name
- pub `get_description` function L93-95 — `(&self) -> Option<&str>` — Get the workflow description (if set).
- pub `get_tags` function L98-100 — `(&self) -> &std::collections::HashMap<String, String>` — Get the workflow tags.
- pub `description` function L103-106 — `(mut self, description: &str) -> Self` — Set the workflow description
- pub `tenant` function L109-112 — `(mut self, tenant: &str) -> Self` — Set the workflow tenant
- pub `tag` function L115-118 — `(mut self, key: &str, value: &str) -> Self` — Add a tag to the workflow metadata
- pub `add_task` function L121-124 — `(mut self, task: Arc<dyn Task>) -> Result<Self, WorkflowError>` — Add a task to the workflow
- pub `validate` function L127-130 — `(self) -> Result<Self, ValidationError>` — Validate the workflow structure
- pub `build` function L133-137 — `(self) -> Result<Workflow, ValidationError>` — Build the final workflow with automatic version calculation
-  `WorkflowBuilder` type L79-138 — `= WorkflowBuilder` — workflows using a chainable, fluent API.

#### crates/cloacina/src/workflow/graph.rs

- pub `DependencyGraph` struct L61-64 — `{ nodes: HashSet<TaskNamespace>, edges: HashMap<TaskNamespace, Vec<TaskNamespace...` — Low-level representation of task dependencies.
- pub `new` function L68-73 — `() -> Self` — Create a new empty dependency graph
- pub `add_node` function L76-79 — `(&mut self, node_id: TaskNamespace)` — Add a node (task) to the graph
- pub `add_edge` function L82-86 — `(&mut self, from: TaskNamespace, to: TaskNamespace)` — Add an edge (dependency) to the graph
- pub `remove_node` function L90-98 — `(&mut self, node_id: &TaskNamespace)` — Remove a node (task) from the graph
- pub `remove_edge` function L101-105 — `(&mut self, from: &TaskNamespace, to: &TaskNamespace)` — Remove a specific edge (dependency) from the graph
- pub `get_dependencies` function L108-110 — `(&self, node_id: &TaskNamespace) -> Option<&Vec<TaskNamespace>>` — Get dependencies for a task
- pub `get_dependents` function L113-124 — `(&self, node_id: &TaskNamespace) -> Vec<TaskNamespace>` — Get tasks that depend on the given task
- pub `has_cycles` function L127-149 — `(&self) -> bool` — Check if the graph contains cycles
- pub `topological_sort` function L152-198 — `(&self) -> Result<Vec<TaskNamespace>, ValidationError>` — Get tasks in topological order
-  `DependencyGraph` type L66-247 — `= DependencyGraph` — task dependencies, cycle detection, and topological sorting.
-  `find_cycle` function L200-214 — `(&self) -> Option<Vec<TaskNamespace>>` — task dependencies, cycle detection, and topological sorting.
-  `dfs_cycle` function L216-246 — `( &self, node: &TaskNamespace, visited: &mut HashSet<TaskNamespace>, rec_stack: ...` — task dependencies, cycle detection, and topological sorting.
-  `DependencyGraph` type L249-253 — `impl Default for DependencyGraph` — task dependencies, cycle detection, and topological sorting.
-  `default` function L250-252 — `() -> Self` — task dependencies, cycle detection, and topological sorting.
-  `tests` module L256-503 — `-` — task dependencies, cycle detection, and topological sorting.
-  `ns` function L259-261 — `(id: &str) -> TaskNamespace` — task dependencies, cycle detection, and topological sorting.
-  `test_add_node_and_get_dependencies` function L264-273 — `()` — task dependencies, cycle detection, and topological sorting.
-  `test_add_edge_and_get_dependencies` function L276-285 — `()` — task dependencies, cycle detection, and topological sorting.
-  `test_get_dependencies_nonexistent_node` function L288-292 — `()` — task dependencies, cycle detection, and topological sorting.
-  `test_get_dependents` function L295-307 — `()` — task dependencies, cycle detection, and topological sorting.
-  `test_get_dependents_no_dependents` function L310-317 — `()` — task dependencies, cycle detection, and topological sorting.
-  `test_remove_node` function L320-331 — `()` — task dependencies, cycle detection, and topological sorting.
-  `test_remove_edge` function L334-343 — `()` — task dependencies, cycle detection, and topological sorting.
-  `test_remove_edge_nonexistent` function L346-352 — `()` — task dependencies, cycle detection, and topological sorting.
-  `test_has_cycles_no_cycle` function L355-364 — `()` — task dependencies, cycle detection, and topological sorting.
-  `test_has_cycles_with_cycle` function L367-375 — `()` — task dependencies, cycle detection, and topological sorting.
-  `test_has_cycles_three_node_cycle` function L378-388 — `()` — task dependencies, cycle detection, and topological sorting.
-  `test_find_cycle_returns_some_when_cyclic` function L391-402 — `()` — task dependencies, cycle detection, and topological sorting.
-  `test_find_cycle_returns_none_when_acyclic` function L405-412 — `()` — task dependencies, cycle detection, and topological sorting.
-  `test_topological_sort_linear_chain` function L415-430 — `()` — task dependencies, cycle detection, and topological sorting.
-  `test_topological_sort_diamond` function L433-455 — `()` — task dependencies, cycle detection, and topological sorting.
-  `test_topological_sort_single_node` function L458-466 — `()` — task dependencies, cycle detection, and topological sorting.
-  `test_topological_sort_independent_nodes` function L469-480 — `()` — task dependencies, cycle detection, and topological sorting.
-  `test_topological_sort_cyclic_returns_error` function L483-496 — `()` — task dependencies, cycle detection, and topological sorting.
-  `test_default_creates_empty_graph` function L499-502 — `()` — task dependencies, cycle detection, and topological sorting.

#### crates/cloacina/src/workflow/metadata.rs

- pub `WorkflowMetadata` struct L56-65 — `{ created_at: DateTime<Utc>, version: String, description: Option<String>, tags:...` — Metadata information for a Workflow.
-  `WorkflowMetadata` type L67-76 — `impl Default for WorkflowMetadata` — workflow versioning, timestamps, and organizational tags.
-  `default` function L68-75 — `() -> Self` — workflow versioning, timestamps, and organizational tags.

#### crates/cloacina/src/workflow/mod.rs

- pub `Workflow` struct L147-154 — `{ name: String, tenant: String, package: String, tasks: HashMap<TaskNamespace, A...` — Main Workflow structure for representing and managing task graphs.
- pub `new` function L186-195 — `(name: &str) -> Self` — Create a new Workflow with the given name
- pub `builder` function L211-213 — `(name: &str) -> WorkflowBuilder` — Create a Workflow builder for programmatic construction
- pub `name` function L216-218 — `(&self) -> &str` — Get the Workflow name
- pub `tenant` function L221-223 — `(&self) -> &str` — Get the Workflow tenant
- pub `set_tenant` function L226-228 — `(&mut self, tenant: &str)` — Set the Workflow tenant
- pub `package` function L231-233 — `(&self) -> &str` — Get the Workflow package
- pub `set_package` function L236-238 — `(&mut self, package: &str)` — Set the Workflow package
- pub `metadata` function L251-253 — `(&self) -> &WorkflowMetadata` — Get the Workflow metadata
- pub `set_version` function L259-261 — `(&mut self, version: &str)` — Set the Workflow version manually
- pub `set_description` function L264-266 — `(&mut self, description: &str)` — Set the Workflow description
- pub `add_tag` function L283-287 — `(&mut self, key: &str, value: &str)` — Add a metadata tag
- pub `remove_tag` function L308-310 — `(&mut self, key: &str) -> Option<String>` — Remove a tag from the workflow metadata
- pub `add_task` function L342-363 — `(&mut self, task: Arc<dyn Task>) -> Result<(), WorkflowError>` — Add a task to the Workflow
- pub `remove_task` function L391-397 — `(&mut self, namespace: &TaskNamespace) -> Option<Arc<dyn Task>>` — Remove a task from the workflow
- pub `remove_dependency` function L421-423 — `(&mut self, from_task: &TaskNamespace, to_task: &TaskNamespace)` — Remove a dependency between two tasks
- pub `validate` function L447-478 — `(&self) -> Result<(), ValidationError>` — Validate the Workflow structure
- pub `topological_sort` function L498-501 — `(&self) -> Result<Vec<TaskNamespace>, ValidationError>` — Get topological ordering of tasks
- pub `get_task` function L513-518 — `(&self, namespace: &TaskNamespace) -> Result<Arc<dyn Task>, WorkflowError>` — Get a task by namespace
- pub `get_dependencies` function L530-538 — `( &self, namespace: &TaskNamespace, ) -> Result<&[TaskNamespace], WorkflowError>` — Get dependencies for a task
- pub `get_dependents` function L563-574 — `( &self, namespace: &TaskNamespace, ) -> Result<Vec<TaskNamespace>, WorkflowErro...` — Get dependents of a task
- pub `subgraph` function L586-621 — `(&self, task_namespaces: &[&TaskNamespace]) -> Result<Workflow, SubgraphError>` — Create a subgraph containing only specified tasks and their dependencies
- pub `get_execution_levels` function L665-698 — `(&self) -> Result<Vec<Vec<TaskNamespace>>, ValidationError>` — Get execution levels (tasks that can run in parallel)
- pub `get_roots` function L714-725 — `(&self) -> Vec<TaskNamespace>` — Get root tasks (tasks with no dependencies)
- pub `get_leaves` function L741-753 — `(&self) -> Vec<TaskNamespace>` — Get leaf tasks (tasks with no dependents)
- pub `can_run_parallel` function L775-778 — `(&self, task_a: &TaskNamespace, task_b: &TaskNamespace) -> bool` — Check if two tasks can run in parallel
- pub `calculate_version` function L826-840 — `(&self) -> String` — Calculate content-based version hash from Workflow structure and tasks.
- pub `get_task_ids` function L908-910 — `(&self) -> Vec<TaskNamespace>` — Get all task namespaces in the workflow
- pub `recreate_from_registry` function L935-976 — `(&self) -> Result<Workflow, WorkflowError>` — Create a new workflow instance from the same data as this workflow
- pub `finalize` function L1000-1005 — `(mut self) -> Self` — Finalize Workflow and calculate version.
-  `builder` module L77 — `-` — # Workflow Management
-  `graph` module L78 — `-` — - `get_all_workflows`: Get all registered workflows
-  `metadata` module L79 — `-` — - `get_all_workflows`: Get all registered workflows
-  `registry` module L80 — `-` — - `get_all_workflows`: Get all registered workflows
-  `Workflow` type L156-167 — `= Workflow` — - `get_all_workflows`: Get all registered workflows
-  `fmt` function L157-166 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — - `get_all_workflows`: Get all registered workflows
-  `Workflow` type L169-1006 — `= Workflow` — - `get_all_workflows`: Get all registered workflows
-  `collect_dependencies` function L623-639 — `( &self, task_namespace: &TaskNamespace, collected: &mut HashSet<TaskNamespace>,...` — - `get_all_workflows`: Get all registered workflows
-  `has_path` function L780-805 — `(&self, from: &TaskNamespace, to: &TaskNamespace) -> bool` — - `get_all_workflows`: Get all registered workflows
-  `hash_topology` function L842-855 — `(&self, hasher: &mut DefaultHasher)` — - `get_all_workflows`: Get all registered workflows
-  `hash_task_definitions` function L857-874 — `(&self, hasher: &mut DefaultHasher)` — - `get_all_workflows`: Get all registered workflows
-  `hash_configuration` function L876-886 — `(&self, hasher: &mut DefaultHasher)` — - `get_all_workflows`: Get all registered workflows
-  `get_task_code_hash` function L888-892 — `(&self, task_namespace: &TaskNamespace) -> Option<String>` — - `get_all_workflows`: Get all registered workflows
-  `tests` module L1009-1718 — `-` — - `get_all_workflows`: Get all registered workflows
-  `TestTask` struct L1017-1021 — `{ id: String, dependencies: Vec<TaskNamespace>, fingerprint: Option<String> }` — - `get_all_workflows`: Get all registered workflows
-  `TestTask` type L1023-1036 — `= TestTask` — - `get_all_workflows`: Get all registered workflows
-  `new` function L1024-1030 — `(id: &str, dependencies: Vec<TaskNamespace>) -> Self` — - `get_all_workflows`: Get all registered workflows
-  `with_fingerprint` function L1032-1035 — `(mut self, fingerprint: &str) -> Self` — - `get_all_workflows`: Get all registered workflows
-  `TestTask` type L1039-1058 — `impl Task for TestTask` — - `get_all_workflows`: Get all registered workflows
-  `execute` function L1040-1045 — `( &self, context: Context<serde_json::Value>, ) -> Result<Context<serde_json::Va...` — - `get_all_workflows`: Get all registered workflows
-  `id` function L1047-1049 — `(&self) -> &str` — - `get_all_workflows`: Get all registered workflows
-  `dependencies` function L1051-1053 — `(&self) -> &[TaskNamespace]` — - `get_all_workflows`: Get all registered workflows
-  `code_fingerprint` function L1055-1057 — `(&self) -> Option<String>` — - `get_all_workflows`: Get all registered workflows
-  `test_workflow_creation` function L1061-1068 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_workflow_add_task` function L1071-1080 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_workflow_validation` function L1083-1096 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_workflow_cycle_detection` function L1099-1116 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_workflow_topological_sort` function L1119-1145 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_workflow_builder_auto_versioning` function L1148-1179 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_execution_levels` function L1182-1216 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_workflow_version_consistency` function L1219-1251 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_workflow_version_changes` function L1254-1285 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_workflow_finalize` function L1288-1302 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_workflow_version_with_code_fingerprints` function L1305-1337 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_workflow_removal_methods` function L1340-1376 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_get_task_found` function L1379-1390 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_get_task_not_found` function L1393-1401 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_get_dependencies_with_deps` function L1404-1418 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_get_dependencies_no_deps` function L1421-1431 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_get_dependencies_task_not_found` function L1434-1442 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_remove_task_returns_task` function L1445-1459 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_remove_task_nonexistent_returns_none` function L1462-1470 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_remove_task_cleans_up_edges` function L1473-1488 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_remove_dependency` function L1491-1507 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_get_roots` function L1510-1527 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_get_leaves` function L1530-1548 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_get_roots_single_task` function L1551-1562 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_get_leaves_single_task` function L1565-1576 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_validate_success` function L1579-1590 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_validate_empty_workflow` function L1593-1599 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_validate_missing_dependency` function L1602-1615 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_get_dependents` function L1618-1636 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_get_dependents_task_not_found` function L1639-1647 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_can_run_parallel` function L1650-1670 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_duplicate_task_rejected` function L1673-1683 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_subgraph` function L1686-1706 — `()` — - `get_all_workflows`: Get all registered workflows
-  `test_subgraph_task_not_found` function L1709-1717 — `()` — - `get_all_workflows`: Get all registered workflows

#### crates/cloacina/src/workflow/registry.rs

- pub `WorkflowConstructor` type L30 — `= Box<dyn Fn() -> Workflow + Send + Sync>` — Type alias for the workflow constructor function stored in the global registry
- pub `GlobalWorkflowRegistry` type L33 — `= Arc<RwLock<HashMap<String, WorkflowConstructor>>>` — Type alias for the global workflow registry containing workflow constructors
- pub `GLOBAL_WORKFLOW_REGISTRY` variable L36-37 — `: Lazy<GlobalWorkflowRegistry>` — Global registry for automatically registering workflows created with the `workflow!` macro
- pub `register_workflow_constructor` function L43-50 — `(workflow_name: String, constructor: F)` — Register a workflow constructor function globally
- pub `global_workflow_registry` function L56-58 — `() -> GlobalWorkflowRegistry` — Get the global workflow registry
- pub `get_all_workflows` function L74-77 — `() -> Vec<Workflow>` — Get all workflows from the global registry

### crates/cloacina/tests

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/tests/fixtures.rs

- pub `get_or_init_postgres_fixture` function L80-103 — `() -> Arc<Mutex<TestFixture>>` — Gets or initializes the PostgreSQL test fixture singleton
- pub `get_or_init_sqlite_fixture` function L116-127 — `() -> Arc<Mutex<TestFixture>>` — Gets or initializes the SQLite test fixture singleton
- pub `get_or_init_fixture` function L132-134 — `() -> Arc<Mutex<TestFixture>>` — Get the default fixture for the current backend configuration.
- pub `get_or_init_fixture` function L139-141 — `() -> Arc<Mutex<TestFixture>>` — Get the default fixture for the current backend configuration.
- pub `get_all_fixtures` function L160-170 — `() -> Vec<(&'static str, Arc<Mutex<TestFixture>>)>` — Returns all enabled backend fixtures for parameterized testing.
- pub `TestFixture` struct L216-225 — `{ initialized: bool, db: Database, db_url: String, schema: String }` — Represents a test fixture for the Cloacina project.
- pub `new_postgres` function L233-249 — `(db: Database, db_url: String, schema: String) -> Self` — Creates a new TestFixture instance for PostgreSQL
- pub `new_sqlite` function L255-268 — `(db: Database, db_url: String) -> Self` — Creates a new TestFixture instance for SQLite
- pub `get_dal` function L271-273 — `(&self) -> cloacina::dal::DAL` — Get a DAL instance using the database
- pub `get_database` function L276-278 — `(&self) -> Database` — Get a clone of the database instance
- pub `get_database_url` function L281-283 — `(&self) -> String` — Get the database URL for this fixture
- pub `get_schema` function L286-288 — `(&self) -> String` — Get the schema name for this fixture
- pub `get_current_backend` function L291-307 — `(&self) -> &'static str` — Get the name of the current backend (postgres or sqlite)
- pub `create_storage` function L310-312 — `(&self) -> cloacina::dal::UnifiedRegistryStorage` — Create a unified storage backend using this fixture's database (primary storage method)
- pub `create_backend_storage` function L316-318 — `(&self) -> Box<dyn cloacina::registry::traits::RegistryStorage>` — Create storage backend matching the current database backend
- pub `create_unified_storage` function L321-323 — `(&self) -> cloacina::dal::UnifiedRegistryStorage` — Create a unified storage backend using this fixture's database
- pub `create_filesystem_storage` function L326-331 — `(&self) -> cloacina::dal::FilesystemRegistryStorage` — Create a filesystem storage backend for testing
- pub `initialize` function L334-363 — `(&mut self)` — Initialize the fixture with additional setup
- pub `reset_database` function L366-452 — `(&mut self)` — Reset the database by truncating all tables in the test schema
- pub `poll_until` function L472-491 — `( timeout: std::time::Duration, interval: std::time::Duration, msg: &str, condit...` — Poll a condition until it returns true, or timeout.
- pub `fixtures` module L508-574 — `-` — for integration tests.
-  `INIT` variable L40 — `: Once` — for integration tests.
-  `POSTGRES_FIXTURE` variable L42 — `: OnceCell<Arc<Mutex<TestFixture>>>` — for integration tests.
-  `SQLITE_FIXTURE` variable L44 — `: OnceCell<Arc<Mutex<TestFixture>>>` — for integration tests.
-  `DEFAULT_POSTGRES_URL` variable L48 — `: &str` — Default PostgreSQL connection URL
-  `get_test_schema` function L53-60 — `() -> String` — Get the test schema name from environment variable or generate a unique one
-  `DEFAULT_SQLITE_URL` variable L64 — `: &str` — Default SQLite connection URL (in-memory with shared cache for testing)
-  `backend_test` macro L186-206 — `-` — Macro for defining tests that run on all enabled backends.
-  `TestFixture` type L227-453 — `= TestFixture` — for integration tests.
-  `TableName` struct L384-387 — `{ tablename: String }` — for integration tests.
-  `TableName` struct L428-431 — `{ name: String }` — for integration tests.
-  `TestFixture` type L493-498 — `impl Drop for TestFixture` — for integration tests.
-  `drop` function L494-497 — `(&mut self)` — for integration tests.
-  `TableCount` struct L501-504 — `{ count: i64 }` — for integration tests.
-  `test_migration_function_postgres` function L515-542 — `()` — for integration tests.
-  `test_migration_function_sqlite` function L547-573 — `()` — for integration tests.

### crates/cloacina/tests/integration

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/tests/integration/computation_graph.rs

- pub `AlphaData` struct L28-30 — `{ value: f64 }` — graph, and generates a callable async function that routes data correctly.
- pub `ProcessedData` struct L33-35 — `{ result: f64 }` — graph, and generates a callable async function that routes data correctly.
- pub `OutputConfirmation` struct L38-41 — `{ published: bool, value: f64 }` — graph, and generates a callable async function that routes data correctly.
- pub `linear_chain` module L54-76 — `-` — graph, and generates a callable async function that routes data correctly.
- pub `entry` function L57-62 — `(alpha: Option<&AlphaData>) -> ProcessedData` — graph, and generates a callable async function that routes data correctly.
- pub `process` function L64-68 — `(input: &ProcessedData) -> ProcessedData` — graph, and generates a callable async function that routes data correctly.
- pub `output` function L70-75 — `(input: &ProcessedData) -> OutputConfirmation` — graph, and generates a callable async function that routes data correctly.
- pub `BetaData` struct L95-97 — `{ estimate: f64 }` — graph, and generates a callable async function that routes data correctly.
- pub `routing_graph` module L108-156 — `-` — graph, and generates a callable async function that routes data correctly.
- pub `DecisionOutcome` enum L112-115 — `Signal | NoAction` — graph, and generates a callable async function that routes data correctly.
- pub `SignalData` struct L118-120 — `{ output: f64 }` — graph, and generates a callable async function that routes data correctly.
- pub `NoActionReason` struct L123-125 — `{ reason: String }` — graph, and generates a callable async function that routes data correctly.
- pub `AuditRecord` struct L128-130 — `{ logged: bool }` — graph, and generates a callable async function that routes data correctly.
- pub `decision` function L132-142 — `(alpha: Option<&AlphaData>, beta: Option<&BetaData>) -> DecisionOutcome` — graph, and generates a callable async function that routes data correctly.
- pub `signal_handler` function L144-149 — `(signal: &SignalData) -> OutputConfirmation` — graph, and generates a callable async function that routes data correctly.
- pub `audit_logger` function L151-155 — `(reason: &NoActionReason) -> AuditRecord` — graph, and generates a callable async function that routes data correctly.
- pub `when_all_graph` module L693-708 — `-` — graph, and generates a callable async function that routes data correctly.
- pub `combine` function L696-700 — `(alpha: Option<&AlphaData>, beta: Option<&BetaData>) -> ProcessedData` — graph, and generates a callable async function that routes data correctly.
- pub `output` function L702-707 — `(input: &ProcessedData) -> OutputConfirmation` — graph, and generates a callable async function that routes data correctly.
-  `test_linear_chain` function L79-88 — `()` — graph, and generates a callable async function that routes data correctly.
-  `test_routing_signal_path` function L159-172 — `()` — graph, and generates a callable async function that routes data correctly.
-  `test_routing_no_action_path` function L175-188 — `()` — graph, and generates a callable async function that routes data correctly.
-  `TestPassthroughAccumulator` struct L203 — `-` — graph, and generates a callable async function that routes data correctly.
-  `TestPassthroughAccumulator` type L206-212 — `= TestPassthroughAccumulator` — graph, and generates a callable async function that routes data correctly.
-  `Output` type L207 — `= AlphaData` — graph, and generates a callable async function that routes data correctly.
-  `process` function L209-211 — `(&mut self, event: Vec<u8>) -> Option<AlphaData>` — graph, and generates a callable async function that routes data correctly.
-  `test_end_to_end_accumulator_reactor_graph` function L215-325 — `()` — graph, and generates a callable async function that routes data correctly.
-  `TestAccumulatorFactory` struct L339 — `-` — graph, and generates a callable async function that routes data correctly.
-  `TestAccumulatorFactory` type L341-379 — `impl AccumulatorFactory for TestAccumulatorFactory` — graph, and generates a callable async function that routes data correctly.
-  `spawn` function L342-378 — `( &self, name: String, boundary_tx: tokio_mpsc::Sender<(SourceName, Vec<u8>)>, s...` — graph, and generates a callable async function that routes data correctly.
-  `Passthrough` struct L351 — `-` — graph, and generates a callable async function that routes data correctly.
-  `Passthrough` type L354-359 — `= Passthrough` — graph, and generates a callable async function that routes data correctly.
-  `Output` type L355 — `= AlphaData` — graph, and generates a callable async function that routes data correctly.
-  `process` function L356-358 — `(&mut self, event: Vec<u8>) -> Option<AlphaData>` — graph, and generates a callable async function that routes data correctly.
-  `test_reactive_scheduler_end_to_end` function L382-478 — `()` — graph, and generates a callable async function that routes data correctly.
-  `TestPoller` struct L487-489 — `{ value: f64 }` — graph, and generates a callable async function that routes data correctly.
-  `TestPoller` type L492-507 — `impl PollingAccumulator for TestPoller` — graph, and generates a callable async function that routes data correctly.
-  `Output` type L493 — `= AlphaData` — graph, and generates a callable async function that routes data correctly.
-  `poll` function L495-502 — `(&mut self) -> Option<AlphaData>` — graph, and generates a callable async function that routes data correctly.
-  `interval` function L504-506 — `(&self) -> std::time::Duration` — graph, and generates a callable async function that routes data correctly.
-  `test_polling_accumulator_to_reactor` function L510-562 — `()` — graph, and generates a callable async function that routes data correctly.
-  `TestBatcher` struct L573 — `-` — graph, and generates a callable async function that routes data correctly.
-  `TestBatcher` type L576-587 — `impl BatchAccumulator for TestBatcher` — graph, and generates a callable async function that routes data correctly.
-  `Output` type L577 — `= AlphaData` — graph, and generates a callable async function that routes data correctly.
-  `process_batch` function L579-586 — `(&mut self, events: Vec<Vec<u8>>) -> Option<AlphaData>` — graph, and generates a callable async function that routes data correctly.
-  `test_batch_accumulator_to_reactor` function L590-681 — `()` — graph, and generates a callable async function that routes data correctly.
-  `test_when_all_waits_for_both_sources` function L711-830 — `()` — graph, and generates a callable async function that routes data correctly.
-  `BetaPassthrough` struct L734 — `-` — graph, and generates a callable async function that routes data correctly.
-  `BetaPassthrough` type L736-741 — `= BetaPassthrough` — graph, and generates a callable async function that routes data correctly.
-  `Output` type L737 — `= BetaData` — graph, and generates a callable async function that routes data correctly.
-  `process` function L738-740 — `(&mut self, event: Vec<u8>) -> Option<BetaData>` — graph, and generates a callable async function that routes data correctly.
-  `test_sequential_input_strategy` function L837-920 — `()` — graph, and generates a callable async function that routes data correctly.
-  `resilience_tests` module L927-1921 — `-` — graph, and generates a callable async function that routes data correctly.
-  `test_dal` function L933-943 — `() -> cloacina::dal::unified::DAL` — Helper: create an in-memory SQLite DAL for testing.
-  `test_boundary_sender_sequence_numbers` function L946-964 — `()` — graph, and generates a callable async function that routes data correctly.
-  `test_boundary_sender_with_sequence_recovery` function L967-981 — `()` — graph, and generates a callable async function that routes data correctly.
-  `test_accumulator_health_channel` function L984-1003 — `()` — graph, and generates a callable async function that routes data correctly.
-  `test_checkpoint_dal_round_trip` function L1006-1030 — `()` — graph, and generates a callable async function that routes data correctly.
-  `test_checkpoint_dal_upsert` function L1033-1051 — `()` — graph, and generates a callable async function that routes data correctly.
-  `test_boundary_dal_with_sequence` function L1054-1074 — `()` — graph, and generates a callable async function that routes data correctly.
-  `test_reactor_state_dal_round_trip` function L1077-1096 — `()` — graph, and generates a callable async function that routes data correctly.
-  `test_reactor_state_dal_with_sequential_queue` function L1099-1115 — `()` — graph, and generates a callable async function that routes data correctly.
-  `test_state_buffer_dal_round_trip` function L1118-1132 — `()` — graph, and generates a callable async function that routes data correctly.
-  `test_delete_graph_state` function L1135-1176 — `()` — graph, and generates a callable async function that routes data correctly.
-  `test_checkpoint_handle_typed_round_trip` function L1179-1195 — `()` — graph, and generates a callable async function that routes data correctly.
-  `test_checkpoint_handle_load_empty` function L1198-1209 — `()` — graph, and generates a callable async function that routes data correctly.
-  `test_reactor_cache_recovery_across_restart` function L1226-1366 — `()` — Test: Reactor cache persists to DAL and survives restart.
-  `test_reactor_health_warming_to_live` function L1374-1442 — `()` — Test: Health state machine transitions — Starting → Warming → Live.
-  `test_boundary_sequence_continuity_across_restart` function L1450-1527 — `()` — Test: Boundary sequence continuity across restart.
-  `test_state_accumulator_survives_restart` function L1534-1644 — `()` — Test: State accumulator persists VecDeque to DAL and restores on restart.
-  `test_batch_buffer_crash_recovery` function L1652-1767 — `()` — Test: Batch buffer survives crash via checkpoint.
-  `SumBatcher` struct L1675 — `-` — graph, and generates a callable async function that routes data correctly.
-  `SumBatcher` type L1677-1687 — `= SumBatcher` — graph, and generates a callable async function that routes data correctly.
-  `Output` type L1678 — `= AlphaData` — graph, and generates a callable async function that routes data correctly.
-  `process_batch` function L1679-1686 — `(&mut self, events: Vec<Vec<u8>>) -> Option<AlphaData>` — graph, and generates a callable async function that routes data correctly.
-  `test_supervisor_individual_accumulator_restart` function L1775-1920 — `()` — Test: Supervisor restarts crashed accumulator individually.
-  `PanicAfterTwoFactory` struct L1794-1796 — `{ spawn_count: std::sync::atomic::AtomicU32 }` — Factory that produces accumulators that panic after 2 events on first spawn,
-  `PanicAfterTwoFactory` type L1798-1849 — `impl AccumulatorFactory for PanicAfterTwoFactory` — graph, and generates a callable async function that routes data correctly.
-  `spawn` function L1799-1848 — `( &self, name: String, boundary_tx: tokio_mpsc::Sender<(SourceName, Vec<u8>)>, s...` — graph, and generates a callable async function that routes data correctly.
-  `MaybePanicAccumulator` struct L1811-1814 — `{ count: u32, should_panic: bool }` — graph, and generates a callable async function that routes data correctly.
-  `MaybePanicAccumulator` type L1817-1826 — `= MaybePanicAccumulator` — graph, and generates a callable async function that routes data correctly.
-  `Output` type L1818 — `= AlphaData` — graph, and generates a callable async function that routes data correctly.
-  `process` function L1819-1825 — `(&mut self, event: Vec<u8>) -> Option<AlphaData>` — graph, and generates a callable async function that routes data correctly.

#### crates/cloacina/tests/integration/context.rs

-  `postgres_tests` module L21-81 — `-`
-  `test_context_db_operations` function L30-80 — `()`
-  `sqlite_tests` module L84-144 — `-`
-  `test_context_db_operations` function L93-143 — `()`

#### crates/cloacina/tests/integration/error.rs

-  `test_context_error_display` function L20-35 — `()`
-  `test_task_error_display` function L38-58 — `()`
-  `test_validation_error_display` function L61-83 — `()`
-  `test_workflow_error_display` function L86-103 — `()`
-  `test_subgraph_error_display` function L106-118 — `()`
-  `test_error_source_chains` function L121-132 — `()`
-  `test_error_debug_formatting` function L135-146 — `()`

#### crates/cloacina/tests/integration/error_paths.rs

-  `MockTask` struct L29-32 — `{ id: String, deps: Vec<TaskNamespace> }` — Tests that invalid inputs produce the correct errors (not panics).
-  `MockTask` type L35-50 — `impl Task for MockTask` — Tests that invalid inputs produce the correct errors (not panics).
-  `execute` function L36-41 — `( &self, context: Context<serde_json::Value>, ) -> Result<Context<serde_json::Va...` — Tests that invalid inputs produce the correct errors (not panics).
-  `id` function L43-45 — `(&self) -> &str` — Tests that invalid inputs produce the correct errors (not panics).
-  `dependencies` function L47-49 — `(&self) -> &[TaskNamespace]` — Tests that invalid inputs produce the correct errors (not panics).
-  `test_empty_workflow_returns_error` function L55-63 — `()` — Tests that invalid inputs produce the correct errors (not panics).
-  `test_duplicate_task_returns_error` function L66-88 — `()` — Tests that invalid inputs produce the correct errors (not panics).
-  `test_missing_dependency_returns_error` function L91-112 — `()` — Tests that invalid inputs produce the correct errors (not panics).
-  `test_cyclic_dependency_returns_error` function L115-139 — `()` — Tests that invalid inputs produce the correct errors (not panics).
-  `test_invalid_trigger_rule_json` function L144-147 — `()` — Tests that invalid inputs produce the correct errors (not panics).
-  `test_unknown_trigger_rule_type` function L150-153 — `()` — Tests that invalid inputs produce the correct errors (not panics).
-  `test_trigger_rule_all_missing_conditions` function L156-159 — `()` — Tests that invalid inputs produce the correct errors (not panics).
-  `test_trigger_rule_conditions_wrong_type` function L162-166 — `()` — Tests that invalid inputs produce the correct errors (not panics).
-  `test_unknown_condition_type` function L169-173 — `()` — Tests that invalid inputs produce the correct errors (not panics).
-  `test_context_value_condition_missing_fields` function L176-179 — `()` — Tests that invalid inputs produce the correct errors (not panics).
-  `test_unknown_value_operator` function L182-185 — `()` — Tests that invalid inputs produce the correct errors (not panics).
-  `test_context_duplicate_insert_returns_error` function L190-196 — `()` — Tests that invalid inputs produce the correct errors (not panics).
-  `test_context_update_missing_key_returns_error` function L199-203 — `()` — Tests that invalid inputs produce the correct errors (not panics).
-  `test_context_get_missing_key_returns_none` function L206-209 — `()` — Tests that invalid inputs produce the correct errors (not panics).
-  `test_cron_invalid_expression_error` function L214-218 — `()` — Tests that invalid inputs produce the correct errors (not panics).
-  `test_cron_invalid_timezone_error` function L221-225 — `()` — Tests that invalid inputs produce the correct errors (not panics).
-  `test_cron_empty_expression_error` function L228-232 — `()` — Tests that invalid inputs produce the correct errors (not panics).
-  `test_manifest_parse_duration_invalid` function L237-243 — `()` — Tests that invalid inputs produce the correct errors (not panics).
-  `test_manifest_parse_duration_valid` function L246-264 — `()` — Tests that invalid inputs produce the correct errors (not panics).

#### crates/cloacina/tests/integration/event_dedup.rs

- pub `event_dedup_test_workflow` module L36-50 — `-` — execution events.
- pub `first` function L40-43 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — execution events.
- pub `second` function L46-49 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — execution events.
-  `test_no_duplicate_completion_events` function L56-122 — `()` — Execute a 2-task workflow and verify exactly one TaskCompleted event per task

#### crates/cloacina/tests/integration/fidius_validation.rs

-  `find_packaged_workflow_dylib` function L26-54 — `() -> Option<std::path::PathBuf>` — Find the pre-built debug dylib for the packaged-workflows example.
-  `create_non_fidius_dylib` function L57-72 — `() -> tempfile::NamedTempFile` — Create a temporary file that is NOT a fidius plugin.
-  `test_non_fidius_dylib_rejected_gracefully` function L75-88 — `()` — correctly in the cloacina context.
-  `test_metadata_fidelity` function L91-145 — `()` — correctly in the cloacina context.
-  `test_task_execution_fidelity` function L148-188 — `()` — correctly in the cloacina context.
-  `test_unknown_task_returns_error` function L191-225 — `()` — correctly in the cloacina context.
-  `test_plugin_info_populated` function L228-259 — `()` — correctly in the cloacina context.

#### crates/cloacina/tests/integration/logging.rs

-  `test_structured_logging` function L20-32 — `()`
-  `test_logging_with_context` function L35-50 — `()`
-  `test_span_creation` function L53-66 — `()`
-  `test_event_creation` function L69-81 — `()`

#### crates/cloacina/tests/integration/main.rs

- pub `computation_graph` module L27 — `-`
- pub `context` module L28 — `-`
- pub `dal` module L29 — `-`
- pub `database` module L30 — `-`
- pub `error` module L31 — `-`
- pub `error_paths` module L32 — `-`
- pub `event_dedup` module L33 — `-`
- pub `executor` module L34 — `-`
- pub `fidius_validation` module L35 — `-`
- pub `logging` module L36 — `-`
- pub `models` module L37 — `-`
- pub `packaging` module L38 — `-`
- pub `packaging_inspection` module L39 — `-`
- pub `python_package` module L40 — `-`
- pub `registry_simple_functional_test` module L41 — `-`
- pub `registry_storage_tests` module L42 — `-`
- pub `registry_workflow_registry_tests` module L43 — `-`
- pub `runner_configurable_registry_tests` module L44 — `-`
- pub `scheduler` module L45 — `-`
- pub `signing` module L46 — `-`
- pub `task` module L47 — `-`
- pub `trigger_packaging` module L48 — `-`
- pub `unified_workflow` module L49 — `-`
- pub `workflow` module L50 — `-`
-  `fixtures` module L53 — `-`

#### crates/cloacina/tests/integration/packaging.rs

-  `write_package_toml` function L30-46 — `(project_path: &Path)` — Write a minimal `package.toml` into a project directory for testing.
-  `PackagingFixture` struct L49-54 — `{ temp_dir: TempDir, project_path: PathBuf, output_path: PathBuf }` — Test fixture for managing temporary projects and packages
-  `PackagingFixture` type L56-113 — `= PackagingFixture` — manifest generation, and archive creation.
-  `new` function L58-104 — `() -> Result<Self>` — Create a new packaging fixture with a test project
-  `get_project_path` function L106-108 — `(&self) -> &Path` — manifest generation, and archive creation.
-  `get_output_path` function L110-112 — `(&self) -> &Path` — manifest generation, and archive creation.
-  `test_package_workflow_full_pipeline` function L117-147 — `()` — manifest generation, and archive creation.
-  `test_compile_options_default` function L150-157 — `()` — manifest generation, and archive creation.
-  `test_compile_options_custom` function L160-172 — `()` — manifest generation, and archive creation.
-  `test_packaging_with_package_toml` function L176-199 — `()` — manifest generation, and archive creation.
-  `test_packaging_invalid_project` function L203-212 — `()` — manifest generation, and archive creation.
-  `test_packaging_missing_cargo_toml` function L216-227 — `()` — manifest generation, and archive creation.
-  `test_packaging_missing_package_toml` function L231-247 — `()` — manifest generation, and archive creation.
-  `test_package_manifest_schema_serialization` function L250-291 — `()` — manifest generation, and archive creation.
-  `test_package_constants` function L294-299 — `()` — manifest generation, and archive creation.
-  `create_test_cargo_toml` function L302-317 — `() -> cloacina::packaging::types::CargoToml` — Helper function to create a minimal valid Cargo.toml for testing
-  `test_cargo_toml_parsing` function L320-334 — `()` — manifest generation, and archive creation.

#### crates/cloacina/tests/integration/packaging_inspection.rs

-  `PackageInspectionFixture` struct L30-35 — `{ temp_dir: TempDir, project_path: PathBuf, package_path: PathBuf }` — Test fixture for packaging and inspecting existing example projects.
-  `PackageInspectionFixture` type L37-79 — `= PackageInspectionFixture` — package (bzip2 tar archive containing source files and `package.toml`).
-  `new` function L39-56 — `() -> Result<Self>` — Create a new fixture using an existing example project.
-  `get_project_path` function L59-61 — `(&self) -> &Path` — package (bzip2 tar archive containing source files and `package.toml`).
-  `get_package_path` function L64-66 — `(&self) -> &Path` — package (bzip2 tar archive containing source files and `package.toml`).
-  `package_workflow` function L69-71 — `(&self) -> Result<()>` — Package the workflow using the cloacina library.
-  `verify_bzip2_magic` function L74-78 — `(&self) -> Result<bool>` — Verify the package is a valid bzip2 archive (fidius format).
-  `test_package_produces_bzip2_archive` function L83-126 — `()` — package (bzip2 tar archive containing source files and `package.toml`).
-  `test_package_inspection_error_handling` function L130-140 — `()` — package (bzip2 tar archive containing source files and `package.toml`).
-  `test_packaging_constants_integration` function L143-153 — `()` — package (bzip2 tar archive containing source files and `package.toml`).

#### crates/cloacina/tests/integration/python_package.rs

-  `create_python_source_dir` function L35-72 — `( dir: &std::path::Path, name: &str, version: &str, entry_module: &str, include_...` — Create a fidius source package directory for a Python workflow.
-  `create_rust_source_dir` function L75-92 — `(dir: &std::path::Path, name: &str, version: &str)` — Create a fidius source package directory for a Rust workflow.
-  `pack_to_bytes` function L95-102 — `(source_dir: &std::path::Path, output_dir: &std::path::Path) -> Vec<u8>` — Pack a source directory into a `.cloacina` archive and return the bytes.
-  `detect_package_kind_identifies_python` function L109-118 — `()` — full round-trip: pack → detect → extract → validate.
-  `detect_package_kind_identifies_rust` function L121-130 — `()` — full round-trip: pack → detect → extract → validate.
-  `extract_python_package_full_roundtrip` function L137-159 — `()` — full round-trip: pack → detect → extract → validate.
-  `extract_rejects_rust_archive` function L162-175 — `()` — full round-trip: pack → detect → extract → validate.
-  `make_python_manifest` function L181-219 — `() -> Manifest` — full round-trip: pack → detect → extract → validate.
-  `manifest_validates_task_dependency_references` function L222-231 — `()` — full round-trip: pack → detect → extract → validate.
-  `manifest_validates_duplicate_task_ids` function L234-243 — `()` — full round-trip: pack → detect → extract → validate.
-  `manifest_validates_python_function_path_format` function L246-255 — `()` — full round-trip: pack → detect → extract → validate.
-  `create_python_e2e_source_dir` function L262-301 — `(dir: &std::path::Path, name: &str)` — Create a Python workflow source dir with a task that sets a context key.
-  `python_e2e_pack_extract_load_register` function L304-357 — `()` — full round-trip: pack → detect → extract → validate.
-  `postgres_bindings` module L364-436 — `-` — full round-trip: pack → detect → extract → validate.
-  `TEST_PG_URL` variable L370 — `: &str` — full round-trip: pack → detect → extract → validate.
-  `test_runner_postgres_construction_and_shutdown` function L374-380 — `()` — full round-trip: pack → detect → extract → validate.
-  `test_with_schema_postgres_creates_and_shuts_down` function L384-399 — `()` — full round-trip: pack → detect → extract → validate.
-  `test_with_schema_register_and_list_cron` function L403-427 — `()` — full round-trip: pack → detect → extract → validate.
-  `test_database_admin_creates_with_postgres_url` function L431-435 — `()` — full round-trip: pack → detect → extract → validate.

#### crates/cloacina/tests/integration/registry_simple_functional_test.rs

-  `create_test_database` function L34-39 — `() -> Database` — Helper to create a test database using the fixture pattern
-  `create_test_storage` function L42-49 — `() -> FilesystemRegistryStorage` — Helper to create a test filesystem storage
-  `test_registry_with_simple_binary_data` function L53-75 — `()` — and demonstrates the new streamlined API.
-  `test_registry_with_real_package_if_available` function L79-140 — `()` — and demonstrates the new streamlined API.
-  `test_registry_api_simplification` function L144-175 — `()` — and demonstrates the new streamlined API.

#### crates/cloacina/tests/integration/registry_storage_tests.rs

- pub `test_store_and_retrieve_impl` function L53-66 — `(mut storage: S)` — Test store and retrieve operations
- pub `test_retrieve_nonexistent_impl` function L69-77 — `(storage: S)` — Test retrieving non-existent data
- pub `test_delete_impl` function L80-97 — `(mut storage: S)` — Test delete operations
- pub `test_invalid_uuid_impl` function L100-106 — `(mut storage: S)` — Test invalid UUID handling
- pub `test_empty_data_impl` function L109-115 — `(mut storage: S)` — Test empty data storage
- pub `test_large_data_impl` function L118-125 — `(mut storage: S)` — Test large data storage
- pub `test_uuid_format_impl` function L128-139 — `(mut storage: S)` — Test UUID format validation
- pub `test_binary_data_integrity_impl` function L142-153 — `(mut storage: S)` — Test binary data integrity
-  `create_test_workflow_data` function L34-46 — `(size: usize) -> Vec<u8>` — Helper to create test data that simulates a compiled .so file
-  `storage_tests` module L49-154 — `-` — Unified storage test implementations that work with any storage backend
-  `filesystem_tests` module L157-214 — `-` — The same test suite runs against all backends.
-  `create_filesystem_storage` function L160-165 — `() -> (FilesystemRegistryStorage, TempDir)` — The same test suite runs against all backends.
-  `test_store_and_retrieve` function L168-171 — `()` — The same test suite runs against all backends.
-  `test_retrieve_nonexistent` function L174-177 — `()` — The same test suite runs against all backends.
-  `test_delete` function L180-183 — `()` — The same test suite runs against all backends.
-  `test_invalid_uuid` function L186-189 — `()` — The same test suite runs against all backends.
-  `test_empty_data` function L192-195 — `()` — The same test suite runs against all backends.
-  `test_large_data` function L198-201 — `()` — The same test suite runs against all backends.
-  `test_uuid_format` function L204-207 — `()` — The same test suite runs against all backends.
-  `test_binary_data_integrity` function L210-213 — `()` — The same test suite runs against all backends.
-  `database_tests` module L217-283 — `-` — The same test suite runs against all backends.
-  `create_database_storage` function L221-226 — `() -> UnifiedRegistryStorage` — The same test suite runs against all backends.
-  `test_store_and_retrieve` function L230-233 — `()` — The same test suite runs against all backends.
-  `test_retrieve_nonexistent` function L237-240 — `()` — The same test suite runs against all backends.
-  `test_delete` function L244-247 — `()` — The same test suite runs against all backends.
-  `test_invalid_uuid` function L251-254 — `()` — The same test suite runs against all backends.
-  `test_empty_data` function L258-261 — `()` — The same test suite runs against all backends.
-  `test_large_data` function L265-268 — `()` — The same test suite runs against all backends.
-  `test_uuid_format` function L272-275 — `()` — The same test suite runs against all backends.
-  `test_binary_data_integrity` function L279-282 — `()` — The same test suite runs against all backends.

#### crates/cloacina/tests/integration/registry_workflow_registry_tests.rs

-  `PackageFixture` struct L36-40 — `{ temp_dir: tempfile::TempDir, package_path: std::path::PathBuf }` — Test fixture for managing package files.
-  `PackageFixture` type L42-101 — `= PackageFixture` — including storage, metadata extraction, validation, and task registration.
-  `new` function L47-89 — `() -> Self` — Create a new package fixture by packing the example source directory.
-  `get_package_data` function L92-94 — `(&self) -> Vec<u8>` — Get the package data as bytes
-  `get_package_path` function L98-100 — `(&self) -> &std::path::Path` — Get the path to the package file
-  `create_test_storage` function L104-109 — `( database: cloacina::Database, ) -> impl cloacina::registry::traits::RegistrySt...` — Helper to create a test storage backend appropriate for the current database
-  `create_test_filesystem_storage` function L113-120 — `() -> FilesystemRegistryStorage` — Helper to create a test filesystem storage (for tests that specifically need filesystem)
-  `test_workflow_registry_creation` function L124-140 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_register_workflow_with_invalid_package` function L144-165 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_register_real_workflow_package` function L169-210 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_get_workflow_nonexistent` function L214-225 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_unregister_nonexistent_workflow` function L229-242 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_list_workflows_empty` function L246-258 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_workflow_registry_with_multiple_packages` function L262-293 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_concurrent_registry_operations` function L297-347 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_registry_error_handling` function L351-374 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_storage_integration` function L378-398 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_database_integration` function L402-423 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_registry_memory_safety` function L427-445 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_package_lifecycle` function L449-477 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_validation_integration` function L481-503 — `()` — including storage, metadata extraction, validation, and task registration.

#### crates/cloacina/tests/integration/runner_configurable_registry_tests.rs

- pub `test_runner_creation_impl` function L82-92 — `(runner: DefaultRunner)` — Test that a runner can be created with a specific storage backend
- pub `test_workflow_registration_impl` function L95-115 — `(runner: DefaultRunner)` — Test that workflows can be registered and listed
- pub `test_registry_configuration_impl` function L118-136 — `(runner: DefaultRunner, expected_backend: &str)` — Test that the registry configuration is applied correctly
- pub `test_runner_shutdown_impl` function L139-143 — `(runner: DefaultRunner)` — Test that the runner can be shut down cleanly
-  `create_test_package` function L35-51 — `() -> Vec<u8>` — Helper to create a minimal test package (.cloacina file)
-  `create_test_config` function L54-66 — `(storage_backend: &str, temp_dir: Option<&TempDir>) -> DefaultRunnerConfig` — Helper to create a test runner config with the specified storage backend
-  `get_database_url_for_test` function L70-75 — `() -> String` — Helper to get the appropriate database URL for testing
-  `registry_tests` module L78-144 — `-` — Unified test implementations that work with any storage backend
-  `filesystem_tests` module L147-219 — `-` — correctly in end-to-end scenarios.
-  `create_filesystem_runner` function L150-161 — `() -> (DefaultRunner, TempDir)` — correctly in end-to-end scenarios.
-  `test_filesystem_runner_creation` function L164-167 — `()` — correctly in end-to-end scenarios.
-  `test_filesystem_workflow_registration` function L170-173 — `()` — correctly in end-to-end scenarios.
-  `test_filesystem_registry_configuration` function L176-179 — `()` — correctly in end-to-end scenarios.
-  `test_filesystem_runner_shutdown` function L182-185 — `()` — correctly in end-to-end scenarios.
-  `test_filesystem_custom_path` function L188-218 — `()` — correctly in end-to-end scenarios.
-  `current_backend_tests` module L222-306 — `-` — correctly in end-to-end scenarios.
-  `create_current_backend_runner` function L225-237 — `() -> DefaultRunner` — correctly in end-to-end scenarios.
-  `get_current_backend` function L239-243 — `() -> String` — correctly in end-to-end scenarios.
-  `test_current_backend_runner_creation` function L247-250 — `()` — correctly in end-to-end scenarios.
-  `test_current_backend_workflow_registration` function L254-257 — `()` — correctly in end-to-end scenarios.
-  `test_current_backend_registry_configuration` function L261-265 — `()` — correctly in end-to-end scenarios.
-  `test_current_backend_runner_shutdown` function L269-272 — `()` — correctly in end-to-end scenarios.
-  `test_current_backend_registry_uses_same_database` function L276-305 — `()` — correctly in end-to-end scenarios.
-  `error_tests` module L309-373 — `-` — correctly in end-to-end scenarios.
-  `test_invalid_storage_backend` function L313-341 — `()` — correctly in end-to-end scenarios.
-  `test_registry_disabled` function L344-372 — `()` — correctly in end-to-end scenarios.
-  `integration_tests` module L376-451 — `-` — correctly in end-to-end scenarios.
-  `test_filesystem_and_current_backend_runners` function L381-450 — `()` — correctly in end-to-end scenarios.

#### crates/cloacina/tests/integration/test_dlopen_packaged.rs

-  `test_dlopen_packaged_workflow_library` function L21-82 — `()` — Minimal test: load a packaged .dylib/.so via dlopen within the test process.

#### crates/cloacina/tests/integration/test_registry_dynamic_loading.rs

-  `test_reconciler_creation_with_loaders` function L35-67 — `()` — Test that the reconciler can be created with dynamic loading components
-  `test_package_loader_creation` function L71-84 — `()` — Test that PackageLoader can be created and used for metadata extraction
-  `test_task_registrar_creation` function L88-107 — `()` — Test that TaskRegistrar can be created and used for task registration
-  `test_reconciler_status` function L112-138 — `()` — Test reconciler status functionality
-  `test_reconciler_config` function L142-166 — `()` — Test reconciler configuration options
-  `test_loader_error_handling` function L170-221 — `()` — Test that loader components handle errors gracefully
-  `test_reconcile_result_methods` function L225-264 — `()` — Test reconciler result types

#### crates/cloacina/tests/integration/test_registry_dynamic_loading_simple.rs

-  `test_reconciler_with_dynamic_loading` function L38-79 — `()` — Test that verifies the reconciler can be created with dynamic loading enabled

#### crates/cloacina/tests/integration/trigger_packaging.rs

-  `rust_manifest_with_triggers` function L36-80 — `() -> Manifest` — - Discovered for Python packages via `@cloaca.trigger`
-  `rust_manifest_no_triggers` function L83-110 — `() -> Manifest` — - Discovered for Python packages via `@cloaca.trigger`
-  `python_manifest_with_trigger` function L113-148 — `() -> Manifest` — - Discovered for Python packages via `@cloaca.trigger`
-  `TestTrigger` struct L152-154 — `{ name: String }` — A simple test trigger for registry round-trip tests.
-  `TestTrigger` type L157-170 — `impl Trigger for TestTrigger` — - Discovered for Python packages via `@cloaca.trigger`
-  `name` function L158-160 — `(&self) -> &str` — - Discovered for Python packages via `@cloaca.trigger`
-  `poll_interval` function L161-163 — `(&self) -> std::time::Duration` — - Discovered for Python packages via `@cloaca.trigger`
-  `allow_concurrent` function L164-166 — `(&self) -> bool` — - Discovered for Python packages via `@cloaca.trigger`
-  `poll` function L167-169 — `(&self) -> Result<TriggerResult, TriggerError>` — - Discovered for Python packages via `@cloaca.trigger`
-  `trigger_register_verify_deregister_roundtrip` function L178-199 — `()` — - Discovered for Python packages via `@cloaca.trigger`
-  `multiple_triggers_register_and_deregister_independently` function L203-239 — `()` — - Discovered for Python packages via `@cloaca.trigger`
-  `python_trigger_decorator_registers_and_wraps` function L247-294 — `()` — - Discovered for Python packages via `@cloaca.trigger`
-  `python_trigger_poll_returns_result` function L298-328 — `()` — - Discovered for Python packages via `@cloaca.trigger`
-  `manifest_with_triggers_validates_successfully` function L335-338 — `()` — - Discovered for Python packages via `@cloaca.trigger`
-  `manifest_trigger_referencing_package_name_is_valid` function L341-345 — `()` — - Discovered for Python packages via `@cloaca.trigger`
-  `manifest_trigger_referencing_task_id_is_valid` function L348-352 — `()` — - Discovered for Python packages via `@cloaca.trigger`
-  `manifest_trigger_referencing_unknown_workflow_fails` function L355-359 — `()` — - Discovered for Python packages via `@cloaca.trigger`
-  `manifest_duplicate_trigger_names_fails` function L362-366 — `()` — - Discovered for Python packages via `@cloaca.trigger`
-  `manifest_trigger_invalid_poll_interval_fails` function L369-373 — `()` — - Discovered for Python packages via `@cloaca.trigger`

#### crates/cloacina/tests/integration/unified_workflow.rs

- pub `unified_test_workflow` module L29-48 — `-` — Integration test for the unified #[workflow] macro (embedded mode).
- pub `step_one` function L33-36 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Integration test for the unified #[workflow] macro (embedded mode).
- pub `step_two` function L39-47 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Integration test for the unified #[workflow] macro (embedded mode).
- pub `test_trigger` function L86-88 — `() -> Result<TriggerResult, TriggerError>` — Integration test for the unified #[workflow] macro (embedded mode).
- pub `my_trigger_fn` function L105-107 — `() -> Result<TriggerResult, TriggerError>` — Integration test for the unified #[workflow] macro (embedded mode).
-  `test_workflow_executes_sqlite` function L52-79 — `()` — Integration test for the unified #[workflow] macro (embedded mode).
-  `test_trigger_registered` function L91-97 — `()` — Integration test for the unified #[workflow] macro (embedded mode).
-  `test_trigger_custom_name` function L110-115 — `()` — Integration test for the unified #[workflow] macro (embedded mode).
-  `nightly_job` function L120 — `()` — Integration test for the unified #[workflow] macro (embedded mode).
-  `test_cron_trigger_registered` function L123-128 — `()` — Integration test for the unified #[workflow] macro (embedded mode).
-  `frequent_check` function L135 — `()` — Integration test for the unified #[workflow] macro (embedded mode).
-  `test_cron_trigger_custom_name` function L138-143 — `()` — Integration test for the unified #[workflow] macro (embedded mode).
-  `test_cron_trigger_poll_returns_result` function L146-156 — `()` — Integration test for the unified #[workflow] macro (embedded mode).

### crates/cloacina/tests/integration/dal

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/tests/integration/dal/api_keys.rs

-  `postgres_tests` module L20-156 — `-` — Integration tests for the API key DAL (Postgres only).
-  `test_create_and_validate_key` function L27-55 — `()` — Integration tests for the API key DAL (Postgres only).
-  `test_validate_unknown_hash_returns_none` function L59-70 — `()` — Integration tests for the API key DAL (Postgres only).
-  `test_list_keys` function L74-94 — `()` — Integration tests for the API key DAL (Postgres only).
-  `test_revoke_key` function L98-133 — `()` — Integration tests for the API key DAL (Postgres only).
-  `test_has_any_keys` function L137-155 — `()` — Integration tests for the API key DAL (Postgres only).

#### crates/cloacina/tests/integration/dal/context.rs

-  `test_save_and_load_context` function L21-46 — `()`
-  `test_update_context` function L49-81 — `()`
-  `test_delete_context` function L84-112 — `()`
-  `test_empty_context_handling` function L115-132 — `()`
-  `test_list_contexts_pagination` function L135-187 — `()`

#### crates/cloacina/tests/integration/dal/execution_events.rs

-  `test_dal_emits_events_on_state_transitions` function L46-198 — `()` — Test that DAL operations automatically emit execution events.
-  `test_events_queryable_by_pipeline` function L202-305 — `()` — Test that events can be queried by pipeline_id.
-  `test_events_queryable_by_task` function L309-404 — `()` — Test that events can be queried by task_id.
-  `test_events_queryable_by_type` function L408-488 — `()` — Test that events can be queried by event type.
-  `test_outbox_empty_after_claiming` function L496-582 — `()` — Test that the outbox is empty after all tasks are claimed.
-  `NUM_TASKS` variable L519 — `: usize` — Tests run on all enabled backends (SQLite, PostgreSQL) using `get_all_fixtures()`.
-  `test_concurrent_claiming_no_duplicates` function L592-721 — `()` — Test that concurrent workers don't cause duplicate claims.
-  `NUM_TASKS` variable L618 — `: usize` — Tests run on all enabled backends (SQLite, PostgreSQL) using `get_all_fixtures()`.
-  `NUM_WORKERS` variable L644 — `: usize` — Tests run on all enabled backends (SQLite, PostgreSQL) using `get_all_fixtures()`.
-  `test_event_count_and_deletion` function L729-816 — `()` — Test count_by_pipeline and delete_older_than for retention policy.
-  `test_get_recent_events` function L820-883 — `()` — Test get_recent returns events in correct order.
-  `test_manual_event_with_data` function L891-974 — `()` — Test that manually created events with event_data are correctly stored.

#### crates/cloacina/tests/integration/dal/mod.rs

- pub `api_keys` module L17 — `-`
- pub `context` module L18 — `-`
- pub `execution_events` module L19 — `-`
- pub `sub_status` module L20 — `-`
- pub `task_claiming` module L21 — `-`
- pub `workflow_packages` module L22 — `-`
- pub `workflow_registry` module L23 — `-`
- pub `workflow_registry_reconciler_integration` module L24 — `-`

#### crates/cloacina/tests/integration/dal/sub_status.rs

-  `test_sub_status_crud_operations` function L39-161 — `()` — Tests all sub_status operations in a single test to avoid fixture contention.

#### crates/cloacina/tests/integration/dal/task_claiming.rs

-  `test_concurrent_task_claiming_no_duplicates` function L45-200 — `()` — Test that concurrent task claiming doesn't produce duplicate claims.
-  `NUM_TASKS` variable L72 — `: usize` — Tests run on all enabled backends (SQLite, PostgreSQL) using `get_all_fixtures()`.
-  `NUM_WORKERS` variable L115 — `: usize` — Tests run on all enabled backends (SQLite, PostgreSQL) using `get_all_fixtures()`.
-  `test_claimed_tasks_marked_running` function L204-287 — `()` — Test that claimed tasks have their status properly updated to Running.
-  `test_running_tasks_not_claimable` function L291-344 — `()` — Test that already-running tasks cannot be claimed again.
-  `create_running_task` function L351-378 — `(dal: &DAL) -> (UniversalUuid, UniversalUuid)` — Helper: create a workflow execution and a Running task for runner claiming tests.
-  `test_runner_double_claim_prevention` function L382-441 — `()` — Double-claim prevention: two runners claim the same task — exactly one wins.
-  `test_heartbeat_ownership_guard` function L445-492 — `()` — Heartbeat succeeds when runner owns the claim, fails when claim is lost.
-  `test_release_claim_clears_fields` function L496-538 — `()` — Release claim clears claimed_by and heartbeat_at.
-  `test_reclaim_after_release` function L542-592 — `()` — After release, another runner can claim the task.
-  `test_find_stale_claims` function L596-641 — `()` — Find stale claims returns tasks with old heartbeats.

#### crates/cloacina/tests/integration/dal/workflow_packages.rs

-  `test_store_and_get_package_metadata` function L24-78 — `()`
-  `test_store_duplicate_package_metadata` function L81-136 — `()`
-  `test_list_all_packages` function L139-202 — `()`
-  `test_delete_package_metadata` function L205-263 — `()`
-  `test_delete_nonexistent_package` function L266-286 — `()`
-  `test_get_nonexistent_package` function L289-307 — `()`
-  `test_store_package_with_complex_metadata` function L310-403 — `()`
-  `test_store_package_with_invalid_uuid` function L406-445 — `()`
-  `test_package_versioning` function L448-520 — `()`

#### crates/cloacina/tests/integration/dal/workflow_registry.rs

-  `MOCK_PACKAGE` variable L28 — `: OnceLock<Vec<u8>>` — Cached mock package data.
-  `get_mock_package` function L35-37 — `() -> Vec<u8>` — Get the cached mock package, packing it from the example source directory.
-  `create_source_package` function L43-78 — `() -> Vec<u8>` — Create a fidius source package from the packaged-workflows example directory.
-  `test_register_and_get_workflow_package` function L82-86 — `()`
-  `test_register_and_get_workflow_package_with_db_storage` function L88-121 — `()`
-  `test_register_and_get_workflow_package_with_fs_storage` function L124-156 — `()`
-  `test_get_workflow_package_by_name` function L160-165 — `()`
-  `test_get_workflow_package_by_name_with_db_storage` function L167-207 — `()`
-  `test_get_workflow_package_by_name_with_fs_storage` function L209-249 — `()`
-  `test_unregister_workflow_package_by_id` function L253-258 — `()`
-  `test_unregister_workflow_package_by_id_with_db_storage` function L260-298 — `()`
-  `test_unregister_workflow_package_by_id_with_fs_storage` function L300-338 — `()`
-  `test_unregister_workflow_package_by_name` function L342-347 — `()`
-  `test_unregister_workflow_package_by_name_with_db_storage` function L349-396 — `()`
-  `test_unregister_workflow_package_by_name_with_fs_storage` function L398-445 — `()`
-  `test_list_packages` function L449-454 — `()`
-  `test_list_packages_with_db_storage` function L456-496 — `()`
-  `test_list_packages_with_fs_storage` function L498-538 — `()`
-  `test_register_duplicate_package_is_idempotent` function L542-548 — `()`
-  `test_register_duplicate_package_idempotent_with_db_storage` function L550-578 — `()`
-  `test_register_duplicate_package_idempotent_with_fs_storage` function L580-605 — `()`
-  `test_exists_operations` function L609-614 — `()`
-  `test_exists_operations_with_db_storage` function L616-664 — `()`
-  `test_exists_operations_with_fs_storage` function L666-714 — `()`
-  `test_get_nonexistent_package` function L718-723 — `()`
-  `test_get_nonexistent_package_with_db_storage` function L725-752 — `()`
-  `test_get_nonexistent_package_with_fs_storage` function L754-781 — `()`
-  `test_unregister_nonexistent_package` function L785-790 — `()`
-  `test_unregister_nonexistent_package_with_db_storage` function L792-823 — `()`
-  `test_unregister_nonexistent_package_with_fs_storage` function L825-856 — `()`

#### crates/cloacina/tests/integration/dal/workflow_registry_reconciler_integration.rs

-  `TEST_PACKAGE` variable L29 — `: OnceLock<Vec<u8>>` — Cached test package data.
-  `get_test_package` function L36-38 — `() -> Vec<u8>` — Get the cached test package, packing it from the example source directory.
-  `create_source_package` function L44-80 — `() -> Vec<u8>` — Create a fidius source package from the simple-packaged example directory.
-  `test_dal_register_then_reconciler_load` function L84-176 — `()` — Integration tests for the end-to-end workflow: register package via DAL → load via reconciler
-  `test_dal_register_then_get_workflow_package_by_id_failure_case` function L180-222 — `()` — Integration tests for the end-to-end workflow: register package via DAL → load via reconciler

### crates/cloacina/tests/integration/database

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/tests/integration/database/connection.rs

-  `test_url_parsing_basic` function L20-32 — `()`
-  `test_url_parsing_without_password` function L35-43 — `()`
-  `test_url_parsing_with_default_port` function L46-55 — `()`
-  `test_invalid_database_urls` function L58-71 — `()`
-  `test_database_connection_construction` function L74-85 — `()`
-  `test_database_url_modification` function L88-99 — `()`

#### crates/cloacina/tests/integration/database/mod.rs

- pub `connection` module L17 — `-`
- pub `migrations` module L18 — `-`

### crates/cloacina/tests/integration/executor

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/tests/integration/executor/context_merging.rs

-  `WorkflowTask` struct L28-31 — `{ id: String, dependencies: Vec<TaskNamespace> }`
-  `WorkflowTask` type L33-43 — `= WorkflowTask`
-  `new` function L34-42 — `(id: &str, deps: Vec<&str>) -> Self`
-  `WorkflowTask` type L46-61 — `impl Task for WorkflowTask`
-  `execute` function L47-52 — `( &self, context: Context<serde_json::Value>, ) -> Result<Context<serde_json::Va...`
-  `id` function L54-56 — `(&self) -> &str`
-  `dependencies` function L58-60 — `(&self) -> &[TaskNamespace]`
-  `early_producer_task` function L67-72 — `(context: &mut Context<Value>) -> Result<(), TaskError>`
-  `late_producer_task` function L78-83 — `(context: &mut Context<Value>) -> Result<(), TaskError>`
-  `merger_task` function L89-117 — `(context: &mut Context<Value>) -> Result<(), TaskError>`
-  `test_context_merging_latest_wins` function L120-261 — `()`
-  `scope_inspector_task` function L267-277 — `(context: &mut Context<Value>) -> Result<(), TaskError>`
-  `test_execution_scope_context_setup` function L280-392 — `()`

#### crates/cloacina/tests/integration/executor/defer_until.rs

-  `deferred_flag_task` function L41-82 — `( context: &mut Context<Value>, handle: &mut TaskHandle, ) -> Result<(), TaskErr...` — A task that defers until an external flag is set, then writes to context.
-  `after_deferred_task` function L86-91 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — A simple task that runs after the deferred task to verify chaining works.
-  `slow_deferred_task` function L95-123 — `( context: &mut Context<Value>, handle: &mut TaskHandle, ) -> Result<(), TaskErr...` — A task that defers with a longer interval so we can observe "Deferred" sub_status.
-  `SimpleTask` struct L131-134 — `{ id: String, dependencies: Vec<TaskNamespace> }` — once a condition is met.
-  `SimpleTask` type L136-158 — `= SimpleTask` — once a condition is met.
-  `new` function L137-145 — `(id: &str, deps: Vec<&str>) -> Self` — once a condition is met.
-  `with_workflow` function L149-157 — `(id: &str, deps: Vec<&str>, workflow_name: &str) -> Self` — Create a SimpleTask with dependencies specified as simple task names.
-  `SimpleTask` type L161-171 — `impl Task for SimpleTask` — once a condition is met.
-  `execute` function L162-164 — `(&self, context: Context<Value>) -> Result<Context<Value>, TaskError>` — once a condition is met.
-  `id` function L165-167 — `(&self) -> &str` — once a condition is met.
-  `dependencies` function L168-170 — `(&self) -> &[TaskNamespace]` — once a condition is met.
-  `test_defer_until_full_workflow` function L180-268 — `()` — Verifies that a task using `defer_until` via TaskHandle completes
-  `test_defer_until_with_downstream_dependency` function L272-371 — `()` — Verifies that a deferred task correctly chains with a downstream task.
-  `test_sub_status_transitions_during_deferral` function L376-481 — `()` — Verifies that sub_status transitions through "Deferred" while the task is

#### crates/cloacina/tests/integration/executor/mod.rs

- pub `context_merging` module L17 — `-`
- pub `defer_until` module L18 — `-`
- pub `multi_tenant` module L19 — `-`
- pub `pause_resume` module L20 — `-`
- pub `task_execution` module L21 — `-`

#### crates/cloacina/tests/integration/executor/multi_tenant.rs

-  `postgres_multi_tenant_tests` module L19-315 — `-` — Integration tests for multi-tenant functionality
-  `tenant_marker_task` function L32-36 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — Simple task that marks its tenant in the context
-  `setup_tenant_workflow` function L39-67 — `(tenant_schema: &str, runtime: &cloacina::Runtime) -> Workflow` — Helper to create a workflow and register it on a scoped runtime
-  `test_schema_isolation` function L71-170 — `() -> Result<(), Box<dyn std::error::Error>>` — Test that schema-based multi-tenancy provides complete data isolation
-  `test_independent_execution` function L174-257 — `() -> Result<(), Box<dyn std::error::Error>>` — Test that the same workflow can execute independently in different tenants
-  `test_invalid_schema_names` function L261-282 — `()` — Test that invalid schema names are rejected
-  `test_sqlite_schema_rejection` function L286-297 — `()` — Test that schema isolation is only supported for PostgreSQL
-  `test_builder_pattern` function L301-314 — `() -> Result<(), Box<dyn std::error::Error>>` — Test builder pattern for multi-tenant setup
-  `sqlite_multi_tenant_tests` module L317-469 — `-` — Integration tests for multi-tenant functionality
-  `sqlite_tenant_task` function L329-332 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — Simple task for SQLite tests
-  `setup_sqlite_workflow` function L335-360 — `(db_name: &str, runtime: &cloacina::Runtime) -> Workflow` — Helper to create a workflow and register it on a scoped runtime
-  `test_sqlite_file_isolation` function L364-451 — `() -> Result<(), Box<dyn std::error::Error>>` — Test that SQLite multi-tenancy works with separate database files
-  `test_sqlite_separate_files` function L455-468 — `() -> Result<(), Box<dyn std::error::Error>>` — Test that SQLite creates separate database files

#### crates/cloacina/tests/integration/executor/pause_resume.rs

-  `wait_for_status` function L33-55 — `( execution: &WorkflowExecution, target: impl Fn(&WorkflowStatus) -> bool, timeo...` — Helper to wait for a specific workflow execution status without consuming the execution handle.
-  `wait_for_terminal` function L58-63 — `( execution: &WorkflowExecution, timeout: Duration, ) -> Result<WorkflowStatus, ...` — Wait for the workflow execution to reach a terminal state (Completed, Failed, or Cancelled)
-  `WorkflowTask` struct L68-71 — `{ id: String, dependencies: Vec<TaskNamespace> }` — Integration tests for workflow pause/resume functionality.
-  `WorkflowTask` type L73-84 — `= WorkflowTask` — Integration tests for workflow pause/resume functionality.
-  `new` function L75-83 — `(id: &str, deps: Vec<&str>) -> Self` — Integration tests for workflow pause/resume functionality.
-  `WorkflowTask` type L87-102 — `impl Task for WorkflowTask` — Integration tests for workflow pause/resume functionality.
-  `execute` function L88-93 — `( &self, context: Context<serde_json::Value>, ) -> Result<Context<serde_json::Va...` — Integration tests for workflow pause/resume functionality.
-  `id` function L95-97 — `(&self) -> &str` — Integration tests for workflow pause/resume functionality.
-  `dependencies` function L99-101 — `(&self) -> &[TaskNamespace]` — Integration tests for workflow pause/resume functionality.
-  `quick_task` function L108-111 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — Integration tests for workflow pause/resume functionality.
-  `slow_first_task` function L117-122 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — Integration tests for workflow pause/resume functionality.
-  `slow_second_task` function L128-133 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — Integration tests for workflow pause/resume functionality.
-  `test_pause_running_workflow` function L136-243 — `()` — Integration tests for workflow pause/resume functionality.
-  `test_resume_paused_workflow` function L246-367 — `()` — Integration tests for workflow pause/resume functionality.
-  `test_pause_non_running_workflow_fails` function L370-440 — `()` — Integration tests for workflow pause/resume functionality.
-  `test_resume_non_paused_workflow_fails` function L443-518 — `()` — Integration tests for workflow pause/resume functionality.

#### crates/cloacina/tests/integration/executor/task_execution.rs

-  `WorkflowTask` struct L30-33 — `{ id: String, dependencies: Vec<TaskNamespace> }`
-  `WorkflowTask` type L35-45 — `= WorkflowTask`
-  `new` function L36-44 — `(id: &str, deps: Vec<&str>) -> Self`
-  `WorkflowTask` type L48-63 — `impl Task for WorkflowTask`
-  `execute` function L49-54 — `( &self, context: Context<serde_json::Value>, ) -> Result<Context<serde_json::Va...`
-  `id` function L56-58 — `(&self) -> &str`
-  `dependencies` function L60-62 — `(&self) -> &[TaskNamespace]`
-  `test_task` function L69-73 — `(context: &mut Context<Value>) -> Result<(), TaskError>`
-  `producer_task` function L79-83 — `(context: &mut Context<Value>) -> Result<(), TaskError>`
-  `consumer_task` function L89-105 — `(context: &mut Context<Value>) -> Result<(), TaskError>`
-  `timeout_task_test` function L112-116 — `(_context: &mut Context<Value>) -> Result<(), TaskError>`
-  `test_task_executor_basic_execution` function L119-211 — `()`
-  `test_task_executor_dependency_loading` function L214-360 — `()`
-  `test_task_executor_timeout_handling` function L363-501 — `()`
-  `unified_task_test` function L507-511 — `(context: &mut Context<Value>) -> Result<(), TaskError>`
-  `test_default_runner_execution` function L514-636 — `()`
-  `initial_context_task_test` function L642-657 — `(context: &mut Context<Value>) -> Result<(), TaskError>`
-  `test_task_executor_context_loading_no_dependencies` function L660-806 — `()`
-  `producer_context_task` function L812-827 — `(context: &mut Context<Value>) -> Result<(), TaskError>`
-  `consumer_context_task` function L833-856 — `(context: &mut Context<Value>) -> Result<(), TaskError>`
-  `test_task_executor_context_loading_with_dependencies` function L859-1053 — `()`
-  `always_fails_task` function L1061-1066 — `(_context: &mut Context<Value>) -> Result<(), TaskError>` — A task that always fails immediately.
-  `always_succeeds_task` function L1070-1073 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — A task that always succeeds.
-  `downstream_of_failure` function L1077-1080 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — A task that depends on always_fails_task (will be skipped when dep fails).
-  `run_workflow_and_get_status` function L1084-1197 — `( workflow_name: &str, task_defs: Vec<(&str, Box<dyn Fn() -> Arc<dyn Task> + Sen...` — Helper to set up a runner with registered tasks and workflow, execute, and
-  `test_workflow_all_tasks_succeed_marked_completed` function L1202-1217 — `()` — COR-01: Workflow where all tasks succeed must be marked "Completed".
-  `test_workflow_task_fails_marked_failed` function L1222-1237 — `()` — COR-01: Workflow where a task fails must be marked "Failed".
-  `test_workflow_mixed_results_marked_failed` function L1242-1266 — `()` — COR-01: Workflow with mixed results (one succeeds, one fails) must be "Failed".
-  `test_workflow_skipped_downstream_marked_failed` function L1271-1295 — `()` — COR-01: Workflow where a task fails and downstream tasks are skipped must be "Failed".

### crates/cloacina/tests/integration/models

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/tests/integration/models/mod.rs

- pub `context` module L17 — `-`

### crates/cloacina/tests/integration/scheduler

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/tests/integration/scheduler/basic_scheduling.rs

-  `SimpleTask` struct L27-29 — `{ id: String }`
-  `SimpleTask` type L32-47 — `impl Task for SimpleTask`
-  `execute` function L33-38 — `( &self, context: Context<serde_json::Value>, ) -> Result<Context<serde_json::Va...`
-  `id` function L40-42 — `(&self) -> &str`
-  `dependencies` function L44-46 — `(&self) -> &[TaskNamespace]`
-  `test_schedule_workflow_execution` function L51-96 — `()`
-  `test_schedule_nonexistent_workflow` function L100-124 — `()`
-  `test_workflow_version_tracking` function L128-172 — `()`

#### crates/cloacina/tests/integration/scheduler/cron_basic.rs

-  `test_cron_evaluator_basic` function L28-40 — `()`
-  `test_cron_schedule_creation` function L44-58 — `()`
-  `test_default_runner_cron_integration` function L62-105 — `()`
-  `test_cron_scheduler_startup_shutdown` function L109-130 — `()`
-  `test_cron_missed_executions_catchup_count` function L134-149 — `()`
-  `test_cron_catchup_respects_max_limit` function L153-162 — `()`
-  `test_cron_schedule_with_recovery_config` function L166-202 — `()`

#### crates/cloacina/tests/integration/scheduler/dependency_resolution.rs

-  `MockTask` struct L26-29 — `{ id: String, dependencies: Vec<TaskNamespace> }`
-  `MockTask` type L32-48 — `impl Task for MockTask`
-  `execute` function L33-39 — `( &self, context: Context<serde_json::Value>, ) -> Result<Context<serde_json::Va...`
-  `id` function L41-43 — `(&self) -> &str`
-  `dependencies` function L45-47 — `(&self) -> &[TaskNamespace]`
-  `test_task_dependency_initialization` function L52-130 — `()`
-  `test_dependency_satisfaction_check` function L134-212 — `()`

#### crates/cloacina/tests/integration/scheduler/mod.rs

-  `basic_scheduling` module L17 — `-`
-  `cron_basic` module L18 — `-`
-  `dependency_resolution` module L20 — `-`
-  `recovery` module L21 — `-`
-  `stale_claims` module L22 — `-`
-  `trigger_rules` module L23 — `-`

#### crates/cloacina/tests/integration/scheduler/recovery.rs

-  `postgres_tests` module L21-602 — `-`
-  `test_orphaned_task_recovery` function L35-109 — `()`
-  `test_task_abandonment_after_max_retries` function L113-193 — `()`
-  `test_no_recovery_needed` function L197-273 — `()`
-  `test_multiple_orphaned_tasks_recovery` function L277-413 — `()`
-  `test_recovery_event_details` function L417-478 — `()`
-  `test_graceful_recovery_for_unknown_workflow` function L482-601 — `()`
-  `sqlite_tests` module L605-1194 — `-`
-  `test_orphaned_task_recovery` function L619-693 — `()`
-  `test_task_abandonment_after_max_retries` function L697-781 — `()`
-  `test_no_recovery_needed` function L785-861 — `()`
-  `test_multiple_orphaned_tasks_recovery` function L865-1005 — `()`
-  `test_recovery_event_details` function L1009-1070 — `()`
-  `test_graceful_recovery_for_unknown_workflow` function L1074-1193 — `()`

#### crates/cloacina/tests/integration/scheduler/stale_claims.rs

-  `test_sweeper` function L32-39 — `(dal: Arc<DAL>, threshold: Duration) -> StaleClaimSweeper` — Create a sweeper with a very short stale threshold for testing.
-  `create_claimed_task` function L45-84 — `( dal: &DAL, wf_name: &str, task_name: &str, ) -> (UniversalUuid, UniversalUuid)` — Helper: create a workflow execution + task in "Running" state with a runner claim.
-  `test_sweep_during_grace_period_is_noop` function L87-121 — `()` — Integration tests for the stale claim sweeper.
-  `test_sweep_after_grace_period_no_stale_claims` function L124-148 — `()` — Integration tests for the stale claim sweeper.
-  `test_sweep_resets_stale_task_to_ready` function L151-186 — `()` — Integration tests for the stale claim sweeper.
-  `test_sweep_multiple_stale_tasks` function L189-227 — `()` — Integration tests for the stale claim sweeper.
-  `test_sweeper_run_loop_stops_on_shutdown` function L230-266 — `()` — Integration tests for the stale claim sweeper.

#### crates/cloacina/tests/integration/scheduler/trigger_rules.rs

-  `SimpleTask` struct L27-29 — `{ id: String }`
-  `SimpleTask` type L32-47 — `impl Task for SimpleTask`
-  `execute` function L33-38 — `( &self, context: Context<serde_json::Value>, ) -> Result<Context<serde_json::Va...`
-  `id` function L40-42 — `(&self) -> &str`
-  `dependencies` function L44-46 — `(&self) -> &[TaskNamespace]`
-  `TriggerTask` struct L51-55 — `{ id: String, deps: Vec<TaskNamespace>, rules: serde_json::Value }` — Mock task with configurable trigger rules and dependencies.
-  `TriggerTask` type L58-80 — `impl Task for TriggerTask`
-  `execute` function L59-67 — `( &self, mut context: Context<serde_json::Value>, ) -> Result<Context<serde_json...`
-  `id` function L69-71 — `(&self) -> &str`
-  `dependencies` function L73-75 — `(&self) -> &[TaskNamespace]`
-  `trigger_rules` function L77-79 — `(&self) -> serde_json::Value`
-  `test_always_trigger_rule` function L84-134 — `()`
-  `test_trigger_rule_serialization` function L138-175 — `()`
-  `test_context_value_operators` function L179-205 — `()`
-  `test_trigger_condition_types` function L209-236 — `()`
-  `test_complex_trigger_rule` function L240-266 — `()`
-  `schedule_and_process` function L272-315 — `( workflow_name: &str, workflow: Workflow, input: Context<serde_json::Value>, ) ...` — Helper: schedule a workflow and run one round of execution processing.
-  `test_runtime_all_conditions_met_task_becomes_ready` function L319-364 — `()`
-  `test_runtime_always_rule_no_deps_becomes_ready` function L368-392 — `()`
-  `test_runtime_none_rule_no_conditions_becomes_ready` function L396-421 — `()`
-  `test_runtime_all_empty_conditions_becomes_ready` function L425-450 — `()`
-  `test_runtime_any_empty_conditions_gets_skipped` function L454-477 — `()`
-  `test_runtime_context_value_exists_passes` function L481-512 — `()`
-  `test_runtime_context_value_exists_fails_skipped` function L516-544 — `()`
-  `test_runtime_context_value_equals_passes` function L548-578 — `()`
-  `test_runtime_context_value_equals_fails_skipped` function L582-610 — `()`

### crates/cloacina/tests/integration/signing

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/tests/integration/signing/key_rotation.rs

-  `test_multiple_keys_sign_different_packages` function L33-80 — `()` — Test that multiple keys can sign different packages.
-  `test_resign_package_with_new_key` function L84-119 — `()` — Test that re-signing a package with a new key works.
-  `test_key_rotation_database_workflow` function L126-145 — `()` — Test that database-based key rotation workflow works.
-  `sign_package_helper` function L148-174 — `( package_path: &std::path::Path, keypair: &cloacina::crypto::GeneratedKeypair, ...` — Helper function to sign a package and create a DetachedSignature.

#### crates/cloacina/tests/integration/signing/mod.rs

-  `key_rotation` module L25 — `-` — Integration tests for package signing and verification.
-  `security_failures` module L26 — `-` — - Security failure cases (tampered packages, untrusted signers, revoked keys)
-  `sign_and_verify` module L27 — `-` — - Security failure cases (tampered packages, untrusted signers, revoked keys)
-  `trust_chain` module L28 — `-` — - Security failure cases (tampered packages, untrusted signers, revoked keys)

#### crates/cloacina/tests/integration/signing/security_failures.rs

-  `test_tampered_package_rejected` function L31-57 — `()` — Test that a tampered package is rejected.
-  `test_untrusted_signer_rejected` function L61-88 — `()` — Test that a package signed by untrusted key is rejected.
-  `test_invalid_signature_rejected` function L92-128 — `()` — Test that an invalid signature (wrong bytes) is rejected.
-  `test_wrong_hash_in_signature_rejected` function L132-160 — `()` — Test that a signature with wrong hash is rejected.
-  `test_malformed_signature_file_rejected` function L164-180 — `()` — Test that malformed signature JSON is rejected.
-  `test_missing_signature_file` function L184-193 — `()` — Test that missing signature file is handled.
-  `test_empty_package` function L197-210 — `()` — Test that empty package is handled correctly.
-  `test_revoked_key_rejected` function L217-226 — `()` — Database-based tests for revoked key rejection.
-  `sign_package_helper` function L229-255 — `( package_path: &std::path::Path, keypair: &cloacina::crypto::GeneratedKeypair, ...` — Helper function to sign a package.

#### crates/cloacina/tests/integration/signing/sign_and_verify.rs

-  `test_sign_and_verify_offline` function L25-71 — `()` — Test signing and verifying a package with raw keys (offline mode).
-  `test_detached_signature_json_roundtrip` function L75-93 — `()` — Test that detached signature roundtrip works correctly.
-  `test_detached_signature_file_roundtrip` function L97-113 — `()` — Test that detached signature file I/O works correctly.
-  `test_signature_source_default` function L117-120 — `()` — Test signature source default is Auto.

#### crates/cloacina/tests/integration/signing/trust_chain.rs

-  `test_direct_trust` function L30-40 — `()` — Test that trust chain resolution includes directly trusted keys.
-  `test_trust_chain_acl` function L47-57 — `()` — Test that trust chain ACL allows parent org to trust child org's keys.
-  `test_trust_chain_isolation` function L64-73 — `()` — Test that trust chain does not leak to unrelated orgs.
-  `test_revoke_trust_acl` function L80-90 — `()` — Test that revoking trust ACL removes inherited keys.
-  `test_key_fingerprint_computation` function L94-104 — `()` — unless running with --include-ignored flag.
-  `test_different_keys_have_different_fingerprints` function L107-113 — `()` — unless running with --include-ignored flag.

### crates/cloacina/tests/integration/task

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/tests/integration/task/checkpoint.rs

-  `CheckpointableTask` struct L21-25 — `{ id: String, dependencies: Vec<TaskNamespace>, checkpoint_data: Arc<Mutex<Optio...`
-  `CheckpointableTask` type L27-42 — `= CheckpointableTask`
-  `new` function L28-37 — `(id: &str, dependencies: Vec<&str>) -> Self`
-  `get_checkpoint_data` function L39-41 — `(&self) -> Option<String>`
-  `CheckpointableTask` type L45-88 — `impl Task for CheckpointableTask`
-  `execute` function L46-67 — `( &self, mut context: Context<serde_json::Value>, ) -> Result<Context<serde_json...`
-  `id` function L69-71 — `(&self) -> &str`
-  `dependencies` function L73-75 — `(&self) -> &[TaskNamespace]`
-  `checkpoint` function L77-87 — `(&self, context: &Context<serde_json::Value>) -> Result<(), CheckpointError>`
-  `test_default_checkpoint_implementation` function L91-104 — `()`
-  `simple_task` function L94-96 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
-  `test_custom_checkpoint_save` function L107-128 — `()`
-  `test_checkpoint_restore` function L131-159 — `()`
-  `test_checkpoint_serialization_error` function L162-205 — `()`
-  `FailingCheckpointTask` struct L164 — `-`
-  `FailingCheckpointTask` type L167-190 — `impl Task for FailingCheckpointTask`
-  `execute` function L168-173 — `( &self, context: Context<serde_json::Value>, ) -> Result<Context<serde_json::Va...`
-  `id` function L175-177 — `(&self) -> &str`
-  `dependencies` function L179-181 — `(&self) -> &[TaskNamespace]`
-  `checkpoint` function L183-189 — `(&self, _context: &Context<serde_json::Value>) -> Result<(), CheckpointError>`
-  `test_checkpoint_validation` function L208-225 — `()`

#### crates/cloacina/tests/integration/task/debug_macro.rs

-  `test_task` function L20-22 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
-  `test_task_generation` function L25-30 — `()`

#### crates/cloacina/tests/integration/task/handle_macro.rs

-  `no_handle_task` function L31-40 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — - Handle-aware tasks can still execute (context-only path via `Task::execute`)
-  `with_handle_task` function L45-59 — `( context: &mut Context<Value>, handle: &mut TaskHandle, ) -> Result<(), TaskErr...` — - Handle-aware tasks can still execute (context-only path via `Task::execute`)
-  `with_task_handle_task` function L64-77 — `( context: &mut Context<Value>, task_handle: &mut TaskHandle, ) -> Result<(), Ta...` — - Handle-aware tasks can still execute (context-only path via `Task::execute`)
-  `test_no_handle_task_does_not_require_handle` function L80-86 — `()` — - Handle-aware tasks can still execute (context-only path via `Task::execute`)
-  `test_handle_param_requires_handle` function L89-95 — `()` — - Handle-aware tasks can still execute (context-only path via `Task::execute`)
-  `test_task_handle_param_requires_handle` function L98-104 — `()` — - Handle-aware tasks can still execute (context-only path via `Task::execute`)
-  `test_no_handle_task_executes_normally` function L107-114 — `()` — - Handle-aware tasks can still execute (context-only path via `Task::execute`)

#### crates/cloacina/tests/integration/task/macro_test.rs

-  `simple_task` function L21-30 — `(context: &mut Context<Value>) -> Result<(), TaskError>`
-  `dependent_task` function L33-50 — `(context: &mut Context<Value>) -> Result<(), TaskError>`
-  `test_macro_generated_task` function L53-67 — `()`
-  `test_macro_with_dependencies` function L70-81 — `()`
-  `test_task_registry_with_macro_tasks` function L84-123 — `()`
-  `test_task_execution_flow` function L126-160 — `()`
-  `test_original_function_available` function L164-169 — `()`

#### crates/cloacina/tests/integration/task/mod.rs

- pub `checkpoint` module L17 — `-`
- pub `debug_macro` module L18 — `-`
- pub `handle_macro` module L19 — `-`
- pub `macro_test` module L20 — `-`
- pub `simple_macro` module L21 — `-`

#### crates/cloacina/tests/integration/task/simple_macro.rs

-  `test_task` function L20-24 — `( _context: &mut cloacina::Context<serde_json::Value>, ) -> Result<(), cloacina:...`
-  `test_macro_expansion` function L27-30 — `()`

### crates/cloacina/tests/integration/workflow

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/tests/integration/workflow/basic.rs

- pub `basic_test_pipeline` module L20-29 — `-`
- pub `simple_task` function L24-28 — `( _context: &mut cloacina::Context<serde_json::Value>, ) -> Result<(), cloacina:...`
-  `test_simple_workflow_creation` function L32-40 — `()`

#### crates/cloacina/tests/integration/workflow/callback_test.rs

-  `TEST1_SUCCESS_COUNT` variable L22 — `: AtomicU32`
-  `TEST2_FAILURE_COUNT` variable L23 — `: AtomicU32`
-  `TEST3_SUCCESS_COUNT` variable L24 — `: AtomicU32`
-  `TEST3_FAILURE_COUNT` variable L25 — `: AtomicU32`
-  `TEST4_SUCCESS_COUNT` variable L26 — `: AtomicU32`
-  `TEST4_FAILURE_COUNT` variable L27 — `: AtomicU32`
-  `test1_success_callback` function L30-36 — `( _task_id: &str, _context: &Context<serde_json::Value>, ) -> Result<(), Box<dyn...`
-  `test2_failure_callback` function L39-46 — `( _task_id: &str, _error: &cloacina::cloacina_workflow::TaskError, _context: &Co...`
-  `test3_success_callback` function L49-55 — `( _task_id: &str, _context: &Context<serde_json::Value>, ) -> Result<(), Box<dyn...`
-  `test3_failure_callback` function L57-64 — `( _task_id: &str, _error: &cloacina::cloacina_workflow::TaskError, _context: &Co...`
-  `test4_success_callback` function L67-73 — `( _task_id: &str, _context: &Context<serde_json::Value>, ) -> Result<(), Box<dyn...`
-  `test4_failure_callback` function L75-82 — `( _task_id: &str, _error: &cloacina::cloacina_workflow::TaskError, _context: &Co...`
-  `test1_task` function L86-88 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
-  `test2_task` function L92-98 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
-  `test3_task` function L107-109 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
-  `test4_task` function L118-124 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
-  `test_on_success_callback_invoked` function L127-140 — `()`
-  `test_on_failure_callback_invoked` function L143-156 — `()`
-  `test_both_callbacks_success_path` function L159-178 — `()`
-  `test_both_callbacks_failure_path` function L181-200 — `()`

#### crates/cloacina/tests/integration/workflow/macro_test.rs

- pub `document_processing` module L25-53 — `-`
- pub `fetch_document` function L29-33 — `( _context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>`
- pub `extract_text` function L36-38 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
- pub `generate_embeddings` function L41-45 — `( _context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>`
- pub `store_embeddings` function L48-52 — `( _context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>`
- pub `parallel_execution` module L79-96 — `-`
- pub `task_a` function L83-85 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
- pub `task_b` function L88-90 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
- pub `task_c` function L93-95 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
-  `test_workflow_macro_basic` function L56-76 — `()`
-  `test_workflow_macro_emits_inventory_entries` function L99-141 — `()`
-  `test_workflow_execution_levels` function L144-159 — `()`

#### crates/cloacina/tests/integration/workflow/mod.rs

- pub `basic` module L17 — `-`
- pub `callback_test` module L18 — `-`
- pub `macro_test` module L19 — `-`
- pub `subgraph` module L20 — `-`

### crates/cloacina-build/src

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina-build/src/lib.rs

- pub `configure` function L47-66 — `()` — Configures the Python rpath and PyO3 cfg flags for the current binary crate.

### crates/cloacina-compiler/src

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina-compiler/src/config.rs

- pub `CompilerConfig` struct L25-50 — `{ home: PathBuf, bind: SocketAddr, database_url: String, verbose: bool, poll_int...` — Runtime configuration for the compiler service.
- pub `tmp_root_or_default` function L54-58 — `(&self) -> PathBuf` — Resolve the effective tmp-root — uses `$home/build-tmp` when unset.
-  `CompilerConfig` type L52-59 — `= CompilerConfig` — Configuration for cloacina-compiler.

#### crates/cloacina-compiler/src/health.rs

-  `serve` function L26-50 — `(bind: SocketAddr, shutdown: CancellationToken)` — `cloacinactl compiler status` / `health` (T-0525).
-  `health` function L52-54 — `() -> Json<serde_json::Value>` — `cloacinactl compiler status` / `health` (T-0525).
-  `status` function L56-66 — `() -> Json<serde_json::Value>` — `cloacinactl compiler status` / `health` (T-0525).

#### crates/cloacina-compiler/src/lib.rs

- pub `run` function L33-68 — `(config: CompilerConfig) -> Result<()>` — Start the compiler service.
-  `config` module L20 — `-` — cloacina-compiler library — entrypoint `run()` exposed so integration tests
-  `health` module L21 — `-` — and the binary main both share the same code path.
-  `loopp` module L22 — `-` — and the binary main both share the same code path.
-  `install_logging` function L70-94 — `(config: &CompilerConfig) -> Result<tracing_appender::non_blocking::WorkerGuard>` — and the binary main both share the same code path.

#### crates/cloacina-compiler/src/loopp.rs

-  `run` function L32-86 — `(config: CompilerConfig, shutdown: CancellationToken) -> Result<()>` — with real cargo invocation; T-0522 layers heartbeats + a sweeper on top.

#### crates/cloacina-compiler/src/main.rs

-  `Cli` struct L37-74 — `{ verbose: bool, home: PathBuf, bind: SocketAddr, database_url: String, poll_int...` — cloacina-compiler — DB-queue-driven build service.
-  `default_home` function L76-80 — `() -> PathBuf` — directly — no runtime toolchain required.
-  `main` function L83-110 — `() -> Result<()>` — directly — no runtime toolchain required.

### crates/cloacina-computation-graph/src

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina-computation-graph/src/lib.rs

- pub `SourceName` struct L41 — `-` — Identifies an accumulator source by name.
- pub `new` function L44-46 — `(name: impl Into<String>) -> Self` — this crate.
- pub `as_str` function L48-50 — `(&self) -> &str` — this crate.
- pub `serialize` function L79-81 — `(value: &T) -> Result<Vec<u8>, GraphError>` — Serialize a value to bincode bytes.
- pub `deserialize` function L84-86 — `(bytes: &[u8]) -> Result<T, GraphError>` — Deserialize bincode bytes to a value.
- pub `json_to_wire` function L93-99 — `( json_str: &str, ) -> Result<Vec<u8>, GraphError>` — Convert a JSON string to bincode bytes for a given type.
- pub `InputCache` struct L113-115 — `{ entries: HashMap<SourceName, Vec<u8>> }` — The input cache holds the last-seen serialized boundary per source.
- pub `new` function L118-122 — `() -> Self` — this crate.
- pub `update` function L125-127 — `(&mut self, source: SourceName, bytes: Vec<u8>)` — Update the cached value for a source.
- pub `get` function L130-133 — `(&self, name: &str) -> Option<Result<T, GraphError>>` — Get and deserialize a cached value by source name.
- pub `has` function L136-138 — `(&self, name: &str) -> bool` — Check if a source has an entry in the cache.
- pub `get_raw` function L141-145 — `(&self, name: &str) -> Option<&[u8]>` — Get the raw bytes for a source.
- pub `snapshot` function L148-150 — `(&self) -> InputCache` — Create a snapshot (clone) of the cache.
- pub `len` function L153-155 — `(&self) -> usize` — Number of sources in the cache.
- pub `is_empty` function L158-160 — `(&self) -> bool` — Whether the cache is empty.
- pub `replace_all` function L163-165 — `(&mut self, other: InputCache)` — Replace all entries.
- pub `sources` function L168-170 — `(&self) -> Vec<&SourceName>` — List all source names in the cache.
- pub `entries_raw` function L173-175 — `(&self) -> &HashMap<SourceName, Vec<u8>>` — Get a reference to the raw entries map.
- pub `entries_as_json` function L178-192 — `(&self) -> HashMap<String, String>` — Return entries as a JSON-friendly map.
- pub `GraphResult` enum L211-216 — `Completed | Error` — Result of executing a compiled computation graph.
- pub `completed` function L219-221 — `(outputs: Vec<Box<dyn Any + Send>>) -> Self` — this crate.
- pub `completed_empty` function L223-227 — `() -> Self` — this crate.
- pub `error` function L229-231 — `(err: GraphError) -> Self` — this crate.
- pub `is_completed` function L233-235 — `(&self) -> bool` — this crate.
- pub `is_error` function L237-239 — `(&self) -> bool` — this crate.
- pub `GraphError` enum L244-259 — `Serialization | Deserialization | MissingInput | NodeExecution | Execution` — Errors that can occur during graph execution.
- pub `CompiledGraphFn` type L266-267 — `= Arc<dyn Fn(InputCache) -> Pin<Box<dyn Future<Output = GraphResult> + Send>> + ...` — Type alias for the compiled graph function.
- pub `ComputationGraphRegistration` struct L274-281 — `{ graph_fn: CompiledGraphFn, accumulator_names: Vec<String>, reaction_mode: Stri...` — Metadata about a registered computation graph.
- pub `ComputationGraphConstructor` type L283 — `= Box<dyn Fn() -> ComputationGraphRegistration + Send + Sync>` — this crate.
- pub `GlobalComputationGraphRegistry` type L284-285 — `= Arc<parking_lot::RwLock<HashMap<String, ComputationGraphConstructor>>>` — this crate.
- pub `register_computation_graph_constructor` function L291-298 — `(graph_name: String, constructor: F)` — Register a computation graph constructor in the global registry.
- pub `global_computation_graph_registry` function L301-303 — `() -> GlobalComputationGraphRegistry` — Get the global computation graph registry.
- pub `list_registered_graphs` function L306-309 — `() -> Vec<String>` — List all registered computation graph names.
- pub `deregister_computation_graph` function L312-316 — `(graph_name: &str)` — Remove a computation graph from the global registry.
- pub `types` module L319-321 — `-` — this crate.
-  `SourceName` type L43-51 — `= SourceName` — this crate.
-  `SourceName` type L53-57 — `= SourceName` — this crate.
-  `fmt` function L54-56 — `(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result` — this crate.
-  `SourceName` type L59-63 — `= SourceName` — this crate.
-  `from` function L60-62 — `(s: &str) -> Self` — this crate.
-  `SourceName` type L65-69 — `= SourceName` — this crate.
-  `from` function L66-68 — `(s: String) -> Self` — this crate.
-  `InputCache` type L117-193 — `= InputCache` — this crate.
-  `InputCache` type L195-199 — `impl Default for InputCache` — this crate.
-  `default` function L196-198 — `() -> Self` — this crate.
-  `hex_encode` function L201-203 — `(bytes: &[u8]) -> String` — this crate.
-  `GraphResult` type L218-240 — `= GraphResult` — this crate.
-  `GLOBAL_COMPUTATION_GRAPH_REGISTRY` variable L287-288 — `: once_cell::sync::Lazy<GlobalComputationGraphRegistry>` — this crate.

### crates/cloacina-macros/src/computation_graph

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina-macros/src/computation_graph/accumulator_macros.rs

- pub `passthrough_accumulator_impl` function L90-127 — `( _args: TokenStream, input: TokenStream, ) -> syn::Result<TokenStream>` — Generate code for `#[passthrough_accumulator]`.
- pub `stream_accumulator_impl` function L133-224 — `(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream>` — Generate code for `#[stream_accumulator(type = "...", topic = "...")]`.
- pub `polling_accumulator_impl` function L292-324 — `(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream>` — Generate code for `#[polling_accumulator(interval = "5s")]`.
- pub `batch_accumulator_impl` function L377-428 — `(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream>` — Generate code for `#[batch_accumulator(flush_interval = "5s")]`.
- pub `state_accumulator_impl` function L524-554 — `(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream>` — Generate code for `#[state_accumulator(capacity = N)]`.
-  `StreamAccumulatorArgs` struct L27-32 — `{ backend_type: String, topic: String, group: Option<String>, state_type: Option...` — Parsed args for `#[stream_accumulator(type = "...", topic = "...", ...)]`
-  `StreamAccumulatorArgs` type L34-84 — `impl Parse for StreamAccumulatorArgs` — These generate structs implementing the `Accumulator` trait.
-  `parse` function L35-83 — `(input: ParseStream) -> syn::Result<Self>` — These generate structs implementing the `Accumulator` trait.
-  `PollingAccumulatorArgs` struct L227-229 — `{ interval_str: String }` — Parsed args for `#[polling_accumulator(interval = "...")]`
-  `PollingAccumulatorArgs` type L231-264 — `impl Parse for PollingAccumulatorArgs` — These generate structs implementing the `Accumulator` trait.
-  `parse` function L232-263 — `(input: ParseStream) -> syn::Result<Self>` — These generate structs implementing the `Accumulator` trait.
-  `parse_duration_ms` function L267-286 — `(s: &str) -> syn::Result<u64>` — Parse a duration string like "5s", "100ms", "1m" into milliseconds.
-  `BatchAccumulatorArgs` struct L327-330 — `{ flush_interval_str: String, max_buffer_size: Option<usize> }` — Parsed args for `#[batch_accumulator(flush_interval = "...")]`
-  `BatchAccumulatorArgs` type L332-371 — `impl Parse for BatchAccumulatorArgs` — These generate structs implementing the `Accumulator` trait.
-  `parse` function L333-370 — `(input: ParseStream) -> syn::Result<Self>` — These generate structs implementing the `Accumulator` trait.
-  `extract_vec_inner` function L431-447 — `(ty: &Type) -> syn::Result<Type>` — Extract the inner type T from Vec<T>.
-  `extract_option_inner` function L450-466 — `(ty: &Type) -> syn::Result<Type>` — Extract the inner type T from Option<T>.
-  `StateAccumulatorArgs` struct L469-471 — `{ capacity: i32 }` — Parsed args for `#[state_accumulator(capacity = N)]`
-  `StateAccumulatorArgs` type L473-513 — `impl Parse for StateAccumulatorArgs` — These generate structs implementing the `Accumulator` trait.
-  `parse` function L474-512 — `(input: ParseStream) -> syn::Result<Self>` — These generate structs implementing the `Accumulator` trait.
-  `extract_vecdeque_inner` function L557-573 — `(ty: &Type) -> syn::Result<Type>` — Extract the inner type T from VecDeque<T>.
-  `pascal_case` function L576-586 — `(s: &str) -> String` — Convert snake_case to PascalCase.
-  `extract_first_param_type` function L589-606 — `( inputs: &syn::punctuated::Punctuated<syn::FnArg, Token![,]>, ) -> syn::Result<...` — Extract the type of the first function parameter.
-  `extract_return_type` function L609-617 — `(output: &syn::ReturnType) -> syn::Result<Type>` — Extract the return type from a function signature.
-  `tests` module L620-629 — `-` — These generate structs implementing the `Accumulator` trait.
-  `test_pascal_case` function L624-628 — `()` — These generate structs implementing the `Accumulator` trait.

#### crates/cloacina-macros/src/computation_graph/codegen.rs

- pub `generate` function L49-381 — `(ir: &GraphIR, module: &ItemMod) -> syn::Result<TokenStream>` — Validate the graph against the module's functions and generate the compiled output.
-  `pascal_case_ident` function L33-46 — `(ident: &Ident) -> Ident` — Convert a snake_case Ident to PascalCase string for struct naming.
-  `extract_functions` function L384-402 — `(module: &ItemMod) -> syn::Result<HashMap<String, ItemFn>>` — Extract named async functions from a module.
-  `has_blocking_attr` function L405-414 — `(func: &ItemFn) -> bool` — Check if a function has `#[node(blocking)]` attribute.
-  `generate_compiled_function` function L420-472 — `( ir: &GraphIR, functions: &HashMap<String, ItemFn>, blocking_nodes: &HashSet<St...` — Generate the body of the compiled async function.
-  `generate_cache_reads` function L475-492 — `(ir: &GraphIR) -> TokenStream` — Generate `let` bindings for cache reads.
-  `generate_node_execution` function L495-572 — `( ir: &GraphIR, node: &GraphNode, functions: &HashMap<String, ItemFn>, blocking_...` — Generate execution code for a single node.
-  `generate_call_args` function L575-602 — `(ir: &GraphIR, node: &GraphNode) -> TokenStream` — Generate the argument list for a node function call.
-  `generate_routing_match` function L605-653 — `( ir: &GraphIR, from_name: &str, variants: &[super::graph_ir::GraphRoutingVarian...` — Generate match arms for a routing node.
-  `generate_routing_use_stmts` function L657-685 — `( ir: &GraphIR, functions: &HashMap<String, ItemFn>, mod_name: &Ident, ) -> Vec<...` — Generate `use ModName::ReturnType::*;` for routing nodes so enum variant

#### crates/cloacina-macros/src/computation_graph/graph_ir.rs

- pub `GraphIR` struct L28-35 — `{ react: ReactionCriteria, sorted_nodes: Vec<String>, nodes: HashMap<String, Gra...` — The complete validated graph, ready for code generation.
- pub `GraphNode` struct L39-50 — `{ name: String, cache_inputs: Vec<String>, edges_out: Vec<GraphEdge>, edges_in: ...` — A node in the graph IR.
- pub `GraphEdge` enum L54-59 — `Linear | Routing` — An outgoing edge from a node.
- pub `GraphRoutingVariant` struct L63-66 — `{ variant_name: String, target: String }` — A single variant -> target mapping.
- pub `IncomingEdge` struct L70-75 — `{ from: String, variant: Option<String> }` — An incoming edge to a node (who feeds this node).
- pub `GraphIRError` enum L79-88 — `Cycle | DanglingReference | DuplicateEdge` — Errors during graph IR construction.
- pub `from_parsed` function L95-217 — `(parsed: ParsedTopology) -> Result<Self, GraphIRError>` — Build a GraphIR from a ParsedTopology.
- pub `terminal_nodes` function L220-222 — `(&self) -> Vec<&GraphNode>` — Get all terminal nodes (leaves of the graph).
- pub `entry_nodes` function L225-230 — `(&self) -> Vec<&GraphNode>` — Get all entry nodes (nodes with no incoming edges).
- pub `get_node` function L233-235 — `(&self, name: &str) -> Option<&GraphNode>` — Get a node by name.
- pub `incoming_sources` function L238-243 — `(&self, name: &str) -> Vec<&IncomingEdge>` — Get all node names that feed into a given node.
-  `GraphIR` type L90-244 — `= GraphIR` — suitable for code generation.
-  `topological_sort` function L247-327 — `(nodes: &HashMap<String, GraphNode>) -> Result<Vec<String>, GraphIRError>` — Kahn's algorithm for topological sorting with cycle detection.
-  `tests` module L330-570 — `-` — suitable for code generation.
-  `ident` function L335-337 — `(name: &str) -> Ident` — suitable for code generation.
-  `make_topology` function L339-347 — `(edges: Vec<ParsedEdge>) -> ParsedTopology` — suitable for code generation.
-  `test_linear_chain` function L350-369 — `()` — suitable for code generation.
-  `test_routing` function L372-394 — `()` — suitable for code generation.
-  `test_diamond_graph` function L397-438 — `()` — suitable for code generation.
-  `test_cycle_detection` function L441-460 — `()` — suitable for code generation.
-  `test_terminal_nodes` function L463-484 — `()` — suitable for code generation.
-  `test_entry_nodes` function L487-507 — `()` — suitable for code generation.
-  `test_cache_inputs_preserved` function L510-520 — `()` — suitable for code generation.
-  `test_incoming_edges_with_variants` function L523-538 — `()` — suitable for code generation.
-  `test_mixed_routing_and_linear` function L541-569 — `()` — suitable for code generation.

#### crates/cloacina-macros/src/computation_graph/mod.rs

- pub `computation_graph_attr` function L34-42 — `(args: TokenStream, input: TokenStream) -> TokenStream` — The `#[computation_graph]` attribute macro entry point.
-  `accumulator_macros` module L22 — `-` — `#[computation_graph]` attribute macro implementation.
-  `codegen` module L23 — `-` — validates it, and generates a compiled async function.
-  `graph_ir` module L24 — `-` — validates it, and generates a compiled async function.
-  `parser` module L25 — `-` — validates it, and generates a compiled async function.
-  `computation_graph_impl` function L44-60 — `( args: proc_macro2::TokenStream, input: proc_macro2::TokenStream, ) -> syn::Res...` — validates it, and generates a compiled async function.

#### crates/cloacina-macros/src/computation_graph/parser.rs

- pub `ParsedTopology` struct L42-45 — `{ react: ReactionCriteria, edges: Vec<ParsedEdge> }` — The full parsed topology from the macro attribute.
- pub `ReactionCriteria` struct L49-52 — `{ mode: ReactionMode, accumulators: Vec<Ident> }` — Reaction criteria: when_any or when_all with accumulator names.
- pub `ReactionMode` enum L55-58 — `WhenAny | WhenAll` — ```
- pub `ParsedEdge` enum L62-75 — `Linear | Routing` — A parsed edge in the topology.
- pub `RoutingVariant` struct L79-82 — `{ variant_name: Ident, target: Ident }` — A single variant -> downstream mapping in a routing edge.
- pub `from_name` function L85-90 — `(&self) -> &Ident` — ```
- pub `from_inputs` function L92-97 — `(&self) -> &[Ident]` — ```
-  `ParsedEdge` type L84-98 — `= ParsedEdge` — ```
-  `ParsedTopology` type L102-145 — `impl Parse for ParsedTopology` — ```
-  `parse` function L103-144 — `(input: ParseStream) -> syn::Result<Self>` — ```
-  `ReactionCriteria` type L147-174 — `impl Parse for ReactionCriteria` — ```
-  `parse` function L148-173 — `(input: ParseStream) -> syn::Result<Self>` — ```
-  `parse_graph_block` function L177-190 — `(input: ParseStream) -> syn::Result<Vec<ParsedEdge>>` — Parse the `graph = { ...
-  `parse_edge` function L199-267 — `(input: ParseStream) -> syn::Result<ParsedEdge>` — Parse a single edge declaration.
-  `tests` module L270-550 — `-` — ```
-  `parse_topology` function L274-276 — `(tokens: proc_macro2::TokenStream) -> syn::Result<ParsedTopology>` — ```
-  `test_parse_when_any` function L279-292 — `()` — ```
-  `test_parse_when_all` function L295-305 — `()` — ```
-  `test_parse_linear_edge` function L308-345 — `()` — ```
-  `test_parse_routing_edge` function L348-377 — `()` — ```
-  `test_parse_mixed_edges` function L380-422 — `()` — ```
-  `test_parse_fan_in` function L425-445 — `()` — ```
-  `test_parse_fan_out` function L448-473 — `()` — ```
-  `test_error_missing_react` function L476-486 — `()` — ```
-  `test_error_missing_graph` function L489-497 — `()` — ```
-  `test_error_unknown_field` function L500-510 — `()` — ```
-  `test_error_unknown_reaction_mode` function L513-522 — `()` — ```
-  `test_error_empty_routing` function L525-536 — `()` — ```
-  `test_error_duplicate_react` function L539-549 — `()` — ```

### crates/cloacina-macros/src

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina-macros/src/lib.rs

- pub `task` function L58-60 — `(args: TokenStream, input: TokenStream) -> TokenStream` — Define a task with retry policies and trigger rules.
- pub `workflow` function L85-87 — `(args: TokenStream, input: TokenStream) -> TokenStream` — Define a workflow as a module containing `#[task]` functions.
- pub `trigger` function L106-108 — `(args: TokenStream, input: TokenStream) -> TokenStream` — Define a trigger that fires a workflow on a schedule or condition.
- pub `computation_graph` function L135-137 — `(args: TokenStream, input: TokenStream) -> TokenStream` — Define a computation graph as a module containing async node functions.
- pub `passthrough_accumulator` function L148-156 — `(args: TokenStream, input: TokenStream) -> TokenStream` — Define a passthrough accumulator (socket-only, no event loop).
- pub `stream_accumulator` function L167-173 — `(args: TokenStream, input: TokenStream) -> TokenStream` — Define a stream-backed accumulator.
- pub `batch_accumulator` function L185-190 — `(args: TokenStream, input: TokenStream) -> TokenStream` — Define a batch accumulator (buffers events, flushes on timer or size threshold).
- pub `polling_accumulator` function L202-208 — `(args: TokenStream, input: TokenStream) -> TokenStream` — Define a polling accumulator (timer-based, queries pull-based sources).
- pub `state_accumulator` function L217-222 — `(args: TokenStream, input: TokenStream) -> TokenStream` — Define a state accumulator (bounded history buffer with DAL persistence).
-  `computation_graph` module L47 — `-` — # Cloacina Macros
-  `packaged_workflow` module L48 — `-` — ```
-  `registry` module L49 — `-` — ```
-  `tasks` module L50 — `-` — ```
-  `trigger_attr` module L51 — `-` — ```
-  `workflow_attr` module L52 — `-` — ```

#### crates/cloacina-macros/src/packaged_workflow.rs

- pub `TaskMetadata` struct L34-45 — `{ local_id: *const std::os::raw::c_char, namespaced_id_template: *const std::os:...` — C-compatible task metadata structure for FFI
- pub `TaskMetadataCollection` struct L55-64 — `{ task_count: u32, tasks: *const TaskMetadata, workflow_name: *const std::os::ra...` — C-compatible collection of task metadata for FFI
- pub `PackagedWorkflowAttributes` struct L80-86 — `{ name: String, package: String, tenant: String, description: Option<String>, au...` — Attributes for the packaged_workflow macro
- pub `detect_package_cycles` function L172-204 — `( task_dependencies: &HashMap<String, Vec<String>>, ) -> Result<(), String>` — Detect circular dependencies within a package's task dependencies
- pub `calculate_levenshtein_distance` function L274-309 — `(a: &str, b: &str) -> usize`
- pub `find_similar_package_task_names` function L321-334 — `(target: &str, available: &[String]) -> Vec<String>` — Find task names similar to the given name for typo suggestions in packaged workflows
- pub `build_package_graph_data` function L348-424 — `( detected_tasks: &HashMap<String, syn::Ident>, task_dependencies: &HashMap<Stri...` — Build graph data structure for a packaged workflow
- pub `generate_packaged_workflow_impl` function L499-1219 — `( attrs: PackagedWorkflowAttributes, input: ItemMod, ) -> TokenStream2` — Generate packaged workflow implementation
- pub `packaged_workflow` function L1260-1292 — `(args: TokenStream, input: TokenStream) -> TokenStream` — The packaged_workflow macro for creating distributable workflow packages
-  `TaskMetadata` type L48 — `impl Send for TaskMetadata`
-  `TaskMetadata` type L49 — `impl Sync for TaskMetadata`
-  `TaskMetadataCollection` type L67 — `impl Send for TaskMetadataCollection`
-  `TaskMetadataCollection` type L68 — `impl Sync for TaskMetadataCollection`
-  `PackagedWorkflowAttributes` type L88-156 — `impl Parse for PackagedWorkflowAttributes`
-  `parse` function L89-155 — `(input: ParseStream) -> SynResult<Self>`
-  `dfs_package_cycle_detection` function L220-258 — `( task_id: &str, task_dependencies: &HashMap<String, Vec<String>>, visited: &mut...` — Depth-first search implementation for package-level cycle detection
-  `calculate_max_depth` function L433-442 — `(task_dependencies: &HashMap<String, Vec<String>>) -> usize` — Calculate the maximum depth in the task dependency graph
-  `calculate_task_depth` function L453-478 — `( task_id: &str, task_dependencies: &HashMap<String, Vec<String>>, visited: &mut...` — Calculate the depth of a specific task in the dependency graph

#### crates/cloacina-macros/src/registry.rs

- pub `TaskInfo` struct L41-48 — `{ id: String, dependencies: Vec<String>, file_path: String }` — Information about a registered task
- pub `CompileTimeTaskRegistry` struct L53-58 — `{ tasks: HashMap<String, TaskInfo>, dependency_graph: HashMap<String, Vec<String...` — Registry that maintains task information and dependency relationships
- pub `new` function L63-68 — `() -> Self` — Creates a new empty task registry
- pub `register_task` function L78-98 — `(&mut self, task_info: TaskInfo) -> Result<(), CompileTimeError>` — Register a task in the compile-time registry
- pub `validate_dependencies` function L109-144 — `(&self, task_id: &str) -> Result<(), CompileTimeError>` — Validate that all dependencies for a task exist in the registry
- pub `validate_single_dependency` function L155-164 — `(&self, dependency: &str) -> Result<(), CompileTimeError>` — Validate that a single dependency exists in the registry
- pub `detect_cycles` function L171-195 — `(&self) -> Result<(), CompileTimeError>` — Detect circular dependencies in the task graph using Tarjan's algorithm
- pub `get_all_task_ids` function L251-253 — `(&self) -> Vec<String>` — Get all registered task IDs
- pub `clear` function L259-262 — `(&mut self)` — Clear the registry
- pub `size` function L266-268 — `(&self) -> usize` — Get the current number of registered tasks
- pub `CompileTimeError` enum L274-302 — `DuplicateTaskId | MissingDependency | CircularDependency | TaskNotFound` — Errors that can occur during compile-time task validation
- pub `to_compile_error` function L309-373 — `(&self) -> TokenStream` — Convert the error into a compile-time error token stream
- pub `get_registry` function L379-381 — `() -> &'static Lazy<Mutex<CompileTimeTaskRegistry>>` — Get the global compile-time registry instance
-  `COMPILE_TIME_TASK_REGISTRY` variable L36-37 — `: Lazy<Mutex<CompileTimeTaskRegistry>>` — Global compile-time registry instance for task tracking
-  `CompileTimeTaskRegistry` type L61-269 — `= CompileTimeTaskRegistry` — for thread-safe access during compilation.
-  `dfs_cycle_detection` function L208-243 — `( &self, task_id: &str, visited: &mut HashMap<String, bool>, rec_stack: &mut Has...` — Depth-first search implementation for cycle detection
-  `CompileTimeError` type L304-374 — `= CompileTimeError` — for thread-safe access during compilation.
-  `find_similar_task_names` function L393-406 — `(target: &str, available: &[String]) -> Vec<String>` — Find task names similar to the given name for typo suggestions
-  `levenshtein_distance` function L419-454 — `(a: &str, b: &str) -> usize` — Calculate the Levenshtein distance between two strings

#### crates/cloacina-macros/src/tasks.rs

- pub `TaskAttributes` struct L44-56 — `{ id: String, dependencies: Vec<String>, retry_attempts: Option<i32>, retry_back...` — Attributes for the task macro that define task behavior and configuration
- pub `calculate_function_fingerprint` function L176-199 — `(func: &ItemFn) -> String` — Calculate code fingerprint from function
- pub `generate_retry_policy_code` function L210-269 — `(attrs: &TaskAttributes) -> TokenStream2` — Generate retry policy creation code based on task attributes
- pub `generate_trigger_rules_code` function L280-303 — `(attrs: &TaskAttributes) -> TokenStream2` — Generate trigger rules JSON code based on task attributes
- pub `parse_trigger_rules_expr` function L321-408 — `(expr: &Expr) -> Result<serde_json::Value, String>` — Parse trigger rule expressions into JSON at compile time
- pub `to_pascal_case` function L554-564 — `(s: &str) -> String` — Convert snake_case to PascalCase
- pub `generate_task_impl` function L579-785 — `(attrs: TaskAttributes, input: ItemFn) -> TokenStream2` — Generate the task implementation
- pub `task` function L807-868 — `(args: TokenStream, input: TokenStream) -> TokenStream` — The main task proc macro
-  `TaskAttributes` type L58-162 — `impl Parse for TaskAttributes`
-  `parse` function L59-161 — `(input: ParseStream) -> SynResult<Self>`
-  `parse_condition_list` function L411-419 — `( args: &syn::punctuated::Punctuated<Expr, syn::Token![,]>, ) -> Result<Vec<serd...` — Parse a list of trigger conditions from function arguments
-  `parse_trigger_condition_expr` function L422-478 — `(expr: &Expr) -> Result<serde_json::Value, String>` — Parse a single trigger condition (not wrapped in a rule)
-  `extract_string_literal` function L481-492 — `(expr: &Expr) -> Result<String, String>` — Extract a string literal from an expression
-  `parse_value_operator` function L495-516 — `(expr: &Expr) -> Result<String, String>` — Parse value operators like equals, greater_than, etc.
-  `parse_json_value` function L519-543 — `(expr: &Expr) -> Result<serde_json::Value, String>` — Parse JSON values from expressions

#### crates/cloacina-macros/src/trigger_attr.rs

- pub `TriggerAttributes` struct L37-44 — `{ on: String, poll_interval: Option<String>, cron: Option<String>, timezone: Opt...` — Attributes for the `#[trigger]` macro.
- pub `trigger_attr` function L130-168 — `(args: TokenStream, input: TokenStream) -> TokenStream` — Entry point for the `#[trigger]` attribute macro.
-  `TriggerAttributes` type L46-127 — `impl Parse for TriggerAttributes` — - **Cron**: `cron` parameter, no function body — framework provides poll logic (T-0305)
-  `parse` function L47-126 — `(input: ParseStream) -> SynResult<Self>` — - **Cron**: `cron` parameter, no function body — framework provides poll logic (T-0305)
-  `parse_duration_ms` function L171-194 — `(s: &str) -> Result<u64, String>` — Parse a duration string like "100ms", "5s", "2m", "1h" into milliseconds.
-  `generate_custom_trigger` function L197-293 — `(attrs: TriggerAttributes, input_fn: ItemFn) -> TokenStream2` — Generate a custom poll trigger (function body provides poll logic).
-  `generate_cron_trigger` function L296-409 — `(attrs: TriggerAttributes, input_fn: ItemFn) -> TokenStream2` — Generate a cron trigger (schedule expression provides the poll logic).
-  `validate_cron_expression` function L412-436 — `(expr: &str) -> Result<(), String>` — Validate a cron expression at compile time.

#### crates/cloacina-macros/src/workflow_attr.rs

- pub `UnifiedWorkflowAttributes` struct L49-54 — `{ name: String, tenant: String, description: Option<String>, author: Option<Stri...` — Attributes for the unified `#[workflow]` macro.
- pub `workflow_attr` function L114-133 — `(args: TokenStream, input: TokenStream) -> TokenStream` — Entry point for the `#[workflow]` attribute macro.
-  `UnifiedWorkflowAttributes` type L56-111 — `impl Parse for UnifiedWorkflowAttributes` — - With `packaged` feature: generates FFI exports (packaged mode) — added in T-0303
-  `parse` function L57-110 — `(input: ParseStream) -> SynResult<Self>` — - With `packaged` feature: generates FFI exports (packaged mode) — added in T-0303
-  `generate_workflow_attr` function L141-268 — `(attrs: UnifiedWorkflowAttributes, input: ItemMod) -> TokenStream2` — Generate the unified workflow implementation.
-  `validate_dependencies` function L271-325 — `( workflow_name: &str, detected_tasks: &HashMap<String, syn::Ident>, task_depend...` — Validate task dependencies within the module.
-  `generate_embedded_registration` function L332-676 — `( mod_name: &syn::Ident, workflow_name: &str, tenant: &str, description: &str, a...` — Generate embedded mode registration code.
-  `generate_trigger_rules_rewrite` function L679-722 — `(tenant: &str, workflow_name: &str) -> TokenStream2` — Generate trigger rules rewrite code (namespace task names in trigger conditions).
-  `generate_packaged_registration` function L729-877 — `( mod_name: &syn::Ident, workflow_name: &str, description: &str, author: &str, f...` — Generate packaged mode FFI exports.

### crates/cloacina-server/src

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina-server/src/lib.rs

- pub `routes` module L23 — `-` — Cloacina HTTP API server library.
- pub `TenantDatabaseCache` struct L43-46 — `{ databases: tokio::sync::RwLock<std::collections::HashMap<String, Database>>, d...` — Cached per-tenant database connections for schema isolation.
- pub `new` function L49-54 — `(database_url: String) -> Self` — management, workflow upload, and execution APIs.
- pub `resolve` function L59-91 — `( &self, tenant_id: &str, admin_db: &Database, ) -> Result<Database, cloacina::d...` — Get or create a schema-scoped Database for the given tenant.
- pub `AppState` struct L96-109 — `{ database: Database, runner: Arc<DefaultRunner>, key_cache: Arc<crate::routes::...` — Shared application state accessible from all route handlers.
- pub `run` function L112-325 — `( home: std::path::PathBuf, bind: SocketAddr, database_url: String, verbose: boo...` — Run the API server.
- pub `RequestId` struct L333 — `-` — Build the axum router with all routes.
-  `TenantDatabaseCache` type L48-92 — `= TenantDatabaseCache` — management, workflow upload, and execution APIs.
-  `request_id_middleware` function L337-358 — `( mut request: axum::extract::Request, next: axum::middleware::Next, ) -> axum::...` — Middleware that generates a UUID request ID, creates a tracing span,
-  `build_router` function L360-481 — `(state: AppState) -> Router` — management, workflow upload, and execution APIs.
-  `api_request_metrics` function L484-494 — `( request: axum::extract::Request, next: axum::middleware::Next, ) -> axum::resp...` — Middleware that counts API requests by method and status code.
-  `health` function L497-499 — `() -> impl IntoResponse` — GET /health — liveness check (no auth, no DB)
-  `ready` function L502-531 — `(State(state): State<AppState>) -> impl IntoResponse` — GET /ready — readiness check (verifies DB connection pool is healthy)
-  `metrics` function L534-544 — `(State(state): State<AppState>) -> impl IntoResponse` — GET /metrics — Prometheus metrics (placeholder for now)
-  `fallback_404` function L547-552 — `() -> impl IntoResponse` — Fallback for unmatched routes — returns 404 JSON
-  `shutdown_signal` function L555-577 — `()` — Wait for shutdown signal (SIGINT or SIGTERM)
-  `bootstrap_admin_key` function L583-631 — `( state: &AppState, home: &std::path::Path, provided_key: Option<&str>, ) -> Res...` — Bootstrap: create an admin API key on first startup if none exist.
-  `mask_db_url` function L635-637 — `(url: &str) -> String` — Mask password in database URL for logging
-  `tests` module L640-1545 — `-` — management, workflow upload, and execution APIs.
-  `TEST_DB_URL` variable L648 — `: &str` — management, workflow upload, and execution APIs.
-  `test_state` function L651-684 — `() -> AppState` — Create a test AppState with a real Postgres connection.
-  `create_test_api_key` function L687-695 — `(state: &AppState) -> String` — Create a bootstrap API key and return the plaintext token.
-  `send_request` function L698-713 — `( app: Router, request: axum::http::Request<Body>, ) -> (StatusCode, serde_json:...` — Send a request to the router and return (status, body as serde_json::Value).
-  `test_request_id_header_present` function L719-745 — `()` — management, workflow upload, and execution APIs.
-  `test_health_returns_200` function L751-763 — `()` — management, workflow upload, and execution APIs.
-  `test_ready_returns_200_with_db` function L767-779 — `()` — management, workflow upload, and execution APIs.
-  `test_metrics_returns_prometheus_format` function L783-836 — `()` — management, workflow upload, and execution APIs.
-  `test_auth_no_token_returns_401` function L842-854 — `()` — management, workflow upload, and execution APIs.
-  `test_auth_invalid_token_returns_401` function L858-871 — `()` — management, workflow upload, and execution APIs.
-  `test_auth_valid_token_passes` function L875-888 — `()` — management, workflow upload, and execution APIs.
-  `test_auth_malformed_header_returns_401` function L892-905 — `()` — management, workflow upload, and execution APIs.
-  `test_create_key_returns_201` function L911-929 — `()` — management, workflow upload, and execution APIs.
-  `test_create_key_missing_name_returns_422` function L933-949 — `()` — management, workflow upload, and execution APIs.
-  `test_list_keys_returns_list` function L953-968 — `()` — management, workflow upload, and execution APIs.
-  `test_revoke_key_valid` function L972-997 — `()` — management, workflow upload, and execution APIs.
-  `test_revoke_key_nonexistent_returns_404` function L1001-1016 — `()` — management, workflow upload, and execution APIs.
-  `test_revoke_key_invalid_uuid_returns_400` function L1020-1034 — `()` — management, workflow upload, and execution APIs.
-  `test_create_tenant_returns_201` function L1040-1066 — `()` — management, workflow upload, and execution APIs.
-  `test_list_tenants` function L1070-1084 — `()` — management, workflow upload, and execution APIs.
-  `test_remove_tenant_nonexistent_succeeds` function L1088-1104 — `()` — management, workflow upload, and execution APIs.
-  `test_create_then_delete_tenant` function L1108-1145 — `()` — management, workflow upload, and execution APIs.
-  `test_create_tenant_missing_fields_returns_422` function L1149-1164 — `()` — management, workflow upload, and execution APIs.
-  `test_list_workflows_returns_list` function L1170-1184 — `()` — management, workflow upload, and execution APIs.
-  `test_get_workflow_nonexistent_returns_404` function L1188-1201 — `()` — management, workflow upload, and execution APIs.
-  `test_upload_workflow_empty_file_returns_400` function L1205-1229 — `()` — management, workflow upload, and execution APIs.
-  `test_upload_workflow_no_file_field_returns_400` function L1233-1257 — `()` — management, workflow upload, and execution APIs.
-  `fixture_path` function L1260-1265 — `(name: &str) -> std::path::PathBuf` — Path to test fixture directory (relative to workspace root).
-  `multipart_file_body` function L1268-1279 — `(data: &[u8]) -> (String, Vec<u8>)` — Build a multipart request body with a file field.
-  `delete_workflow_if_exists` function L1282-1292 — `(state: &AppState, token: &str, name: &str, version: &str)` — Delete a workflow by name/version if it exists (cleanup for idempotent tests).
-  `test_upload_valid_python_workflow_returns_201` function L1296-1322 — `()` — management, workflow upload, and execution APIs.
-  `test_upload_valid_rust_workflow_returns_201` function L1326-1352 — `()` — management, workflow upload, and execution APIs.
-  `test_upload_corrupt_package_returns_400` function L1356-1376 — `()` — management, workflow upload, and execution APIs.
-  `test_list_executions_returns_list` function L1382-1396 — `()` — management, workflow upload, and execution APIs.
-  `test_get_execution_invalid_uuid_returns_400` function L1400-1413 — `()` — management, workflow upload, and execution APIs.
-  `test_get_execution_nonexistent_returns_404` function L1417-1431 — `()` — management, workflow upload, and execution APIs.
-  `test_get_execution_events_invalid_uuid_returns_400` function L1435-1448 — `()` — management, workflow upload, and execution APIs.
-  `test_execute_nonexistent_workflow_returns_error` function L1452-1467 — `()` — management, workflow upload, and execution APIs.
-  `test_get_execution_events_valid_uuid_no_events` function L1471-1489 — `()` — management, workflow upload, and execution APIs.
-  `test_list_triggers_returns_list` function L1495-1509 — `()` — management, workflow upload, and execution APIs.
-  `test_get_trigger_nonexistent_returns_404` function L1513-1526 — `()` — management, workflow upload, and execution APIs.
-  `test_unknown_route_returns_404` function L1532-1544 — `()` — management, workflow upload, and execution APIs.

#### crates/cloacina-server/src/main.rs

-  `Cli` struct L29-53 — `{ verbose: bool, home: PathBuf, bind: SocketAddr, database_url: String, bootstra...` — cloacina-server — HTTP API for Cloacina, backed by Postgres.
-  `default_home` function L55-59 — `() -> PathBuf` — command in T-0510 (CLOACI-I-0098).
-  `main` function L62-73 — `() -> Result<()>` — command in T-0510 (CLOACI-I-0098).

### crates/cloacina-server/src/routes

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina-server/src/routes/auth.rs

- pub `AuthenticatedKey` struct L43-49 — `{ key_id: uuid::Uuid, name: String, permissions: String, tenant_id: Option<Strin...` — Authenticated key info inserted into request extensions.
- pub `KeyCache` struct L58-61 — `{ cache: Mutex<LruCache<String, CachedEntry>>, ttl: Duration }` — LRU cache for validated API key hashes with TTL expiry.
- pub `new` function L66-73 — `(capacity: usize, ttl: Duration) -> Self` — Create a new key cache.
- pub `default_cache` function L76-78 — `() -> Self` — Create with default settings (256 entries, 30s TTL).
- pub `get` function L81-91 — `(&self, hash: &str) -> Option<ApiKeyInfo>` — Look up a key hash.
- pub `insert` function L94-103 — `(&self, hash: String, info: ApiKeyInfo)` — Insert a validated key into the cache.
- pub `evict` function L107-110 — `(&self, hash: &str)` — Evict a specific key (used after revocation).
- pub `clear` function L113-116 — `(&self)` — Clear all entries.
- pub `validate_token` function L123-169 — `( state: &AppState, token: &str, ) -> Result<AuthenticatedKey, (StatusCode, Json...` — Validate a bearer token and return the authenticated key info.
- pub `require_auth` function L175-195 — `( State(state): State<AppState>, mut request: Request, next: Next, ) -> Response` — Auth middleware — validates Bearer token against cache then DAL.
- pub `can_access_tenant` function L217-225 — `(&self, tenant_id: &str) -> bool` — Check if this key can access the given tenant's resources.
- pub `forbidden_response` function L228-230 — `() -> ApiError` — Returns a 403 response for tenant access denied.
- pub `admin_required_response` function L233-235 — `() -> ApiError` — Returns a 403 response for admin-only operations.
- pub `can_write` function L240-242 — `(&self) -> bool` — Check if this key has at least write permission.
- pub `can_admin` function L246-248 — `(&self) -> bool` — Check if this key has admin role within its tenant.
- pub `insufficient_role_response` function L251-253 — `() -> ApiError` — Returns a 403 response for insufficient role.
- pub `WsTicketStore` struct L273-277 — `{ tickets: Mutex<HashMap<String, WsTicket>>, ttl: Duration, max_capacity: usize ...` — Thread-safe store for WebSocket auth tickets.
- pub `new` function L281-287 — `(ttl: Duration) -> Self` — Create a new ticket store with the given TTL (e.g., 60 seconds).
- pub `issue` function L294-319 — `(&self, auth: AuthenticatedKey) -> String` — Issue a new ticket for the given authenticated key.
- pub `consume` function L323-331 — `(&self, ticket: &str) -> Option<AuthenticatedKey>` — Consume a ticket — returns the authenticated key if valid and not expired.
-  `CachedEntry` struct L52-55 — `{ info: ApiKeyInfo, inserted_at: Instant }` — A cached entry with TTL tracking.
-  `KeyCache` type L63-117 — `= KeyCache` — Applied via `route_layer` so unauthenticated routes still 404 correctly.
-  `extract_bearer_token` function L198-205 — `(request: &Request) -> Option<&str>` — Extract the Bearer token from the Authorization header.
-  `AuthenticatedKey` type L211-254 — `= AuthenticatedKey` — Applied via `route_layer` so unauthenticated routes still 404 correctly.
-  `WsTicket` struct L264-267 — `{ auth: AuthenticatedKey, expires_at: Instant }` — A single-use, time-limited ticket for WebSocket authentication.
-  `WsTicketStore` type L279-332 — `= WsTicketStore` — Applied via `route_layer` so unauthenticated routes still 404 correctly.
-  `tests` module L335-447 — `-` — Applied via `route_layer` so unauthenticated routes still 404 correctly.
-  `make_auth` function L338-346 — `(name: &str) -> AuthenticatedKey` — Applied via `route_layer` so unauthenticated routes still 404 correctly.
-  `test_ticket_issue_and_consume` function L349-357 — `()` — Applied via `route_layer` so unauthenticated routes still 404 correctly.
-  `test_ticket_single_use` function L360-369 — `()` — Applied via `route_layer` so unauthenticated routes still 404 correctly.
-  `test_ticket_invalid_rejected` function L372-378 — `()` — Applied via `route_layer` so unauthenticated routes still 404 correctly.
-  `test_ticket_expired_rejected` function L381-389 — `()` — Applied via `route_layer` so unauthenticated routes still 404 correctly.
-  `test_ticket_store_bounded` function L392-420 — `()` — Applied via `route_layer` so unauthenticated routes still 404 correctly.
-  `test_ticket_store_evicts_expired_on_issue` function L423-446 — `()` — Applied via `route_layer` so unauthenticated routes still 404 correctly.

#### crates/cloacina-server/src/routes/error.rs

- pub `ApiError` struct L39-43 — `{ status: StatusCode, code: &'static str, message: String }` — Standardized API error response.
- pub `new` function L47-53 — `(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self` — Create a new API error.
- pub `bad_request` function L57-59 — `(code: &'static str, message: impl Into<String>) -> Self` — error responses with request correlation IDs.
- pub `not_found` function L61-63 — `(code: &'static str, message: impl Into<String>) -> Self` — error responses with request correlation IDs.
- pub `forbidden` function L65-67 — `(code: &'static str, message: impl Into<String>) -> Self` — error responses with request correlation IDs.
- pub `unauthorized` function L69-71 — `(message: impl Into<String>) -> Self` — error responses with request correlation IDs.
- pub `internal` function L73-75 — `(message: impl Into<String>) -> Self` — error responses with request correlation IDs.
-  `ApiError` type L45-76 — `= ApiError` — error responses with request correlation IDs.
-  `ApiError` type L78-86 — `impl IntoResponse for ApiError` — error responses with request correlation IDs.
-  `into_response` function L79-85 — `(self) -> Response` — error responses with request correlation IDs.

#### crates/cloacina-server/src/routes/executions.rs

- pub `ExecuteRequest` struct L37-41 — `{ context: Option<serde_json::Value> }` — Request body for executing a workflow.
- pub `execute_workflow` function L50-99 — `( State(state): State<AppState>, Extension(auth): Extension<AuthenticatedKey>, P...` — POST /tenants/:tenant_id/workflows/:name/execute — execute a workflow.
- pub `list_executions` function L102-151 — `( State(state): State<AppState>, Extension(auth): Extension<AuthenticatedKey>, P...` — GET /tenants/:tenant_id/executions — list workflow executions.
- pub `get_execution` function L154-203 — `( State(state): State<AppState>, Extension(auth): Extension<AuthenticatedKey>, P...` — GET /tenants/:tenant_id/executions/:id — get execution details.
- pub `get_execution_events` function L206-258 — `( State(state): State<AppState>, Extension(auth): Extension<AuthenticatedKey>, P...` — GET /tenants/:tenant_id/executions/:id/events — execution event log.

#### crates/cloacina-server/src/routes/health_reactive.rs

- pub `list_accumulators` function L33-50 — `(State(state): State<AppState>) -> impl IntoResponse` — GET /v1/health/accumulators — list all registered accumulators with health status.
- pub `list_reactors` function L53-76 — `(State(state): State<AppState>) -> impl IntoResponse` — GET /v1/health/reactors — list all reactors with status.
- pub `get_reactor` function L79-111 — `( State(state): State<AppState>, Path(name): Path<String>, ) -> impl IntoRespons...` — GET /v1/health/reactors/{name} — single reactor health.

#### crates/cloacina-server/src/routes/keys.rs

- pub `KeyRole` enum L38-42 — `Admin | Write | Read` — Allowed roles for API keys.
- pub `as_str` function L45-51 — `(&self) -> &'static str` — The bootstrap key is created automatically on first server startup.
- pub `CreateKeyRequest` struct L62-66 — `{ name: String, role: KeyRole }` — Request body for creating a new API key.
- pub `create_key` function L73-116 — `( State(state): State<AppState>, Extension(auth): Extension<AuthenticatedKey>, J...` — POST /auth/keys — create a new API key.
- pub `list_keys` function L120-151 — `( State(state): State<AppState>, Extension(auth): Extension<AuthenticatedKey>, )...` — GET /auth/keys — list all API keys (no hashes or plaintext).
- pub `revoke_key` function L155-185 — `( State(state): State<AppState>, Extension(auth): Extension<AuthenticatedKey>, P...` — DELETE /auth/keys/:key_id — revoke an API key.
- pub `create_tenant_key` function L189-237 — `( State(state): State<AppState>, Extension(auth): Extension<AuthenticatedKey>, P...` — POST /tenants/:tenant_id/keys — create a key scoped to a tenant.
- pub `create_ws_ticket` function L243-253 — `( State(state): State<AppState>, Extension(auth): Extension<AuthenticatedKey>, )...` — POST /auth/ws-ticket — exchange a Bearer token for a single-use WebSocket ticket.
-  `KeyRole` type L44-52 — `= KeyRole` — The bootstrap key is created automatically on first server startup.
-  `KeyRole` type L54-58 — `impl Default for KeyRole` — The bootstrap key is created automatically on first server startup.
-  `default` function L55-57 — `() -> Self` — The bootstrap key is created automatically on first server startup.

#### crates/cloacina-server/src/routes/mod.rs

- pub `auth` module L19 — `-` — API server route handlers and middleware.
- pub `error` module L20 — `-` — API server route handlers and middleware.
- pub `executions` module L21 — `-` — API server route handlers and middleware.
- pub `health_reactive` module L22 — `-` — API server route handlers and middleware.
- pub `keys` module L23 — `-` — API server route handlers and middleware.
- pub `tenants` module L24 — `-` — API server route handlers and middleware.
- pub `triggers` module L25 — `-` — API server route handlers and middleware.
- pub `workflows` module L26 — `-` — API server route handlers and middleware.
- pub `ws` module L27 — `-` — API server route handlers and middleware.

#### crates/cloacina-server/src/routes/tenants.rs

- pub `CreateTenantRequest` struct L39-47 — `{ schema_name: String, username: String, password: String }` — Request body for creating a tenant.
- pub `create_tenant` function L51-88 — `( State(state): State<AppState>, Extension(auth): Extension<AuthenticatedKey>, J...` — POST /tenants — create a new tenant (Postgres schema + user + migrations).
- pub `remove_tenant` function L92-115 — `( State(state): State<AppState>, Extension(auth): Extension<AuthenticatedKey>, P...` — DELETE /tenants/:schema_name — remove a tenant (drop schema + user).
- pub `list_tenants` function L119-141 — `( State(state): State<AppState>, Extension(auth): Extension<AuthenticatedKey>, )...` — GET /tenants — list tenant schemas.

#### crates/cloacina-server/src/routes/triggers.rs

- pub `list_triggers` function L31-72 — `( State(state): State<AppState>, Extension(auth): Extension<AuthenticatedKey>, P...` — GET /tenants/:tenant_id/triggers — list all schedules (cron + trigger).
- pub `get_trigger` function L75-134 — `( State(state): State<AppState>, Extension(auth): Extension<AuthenticatedKey>, P...` — GET /tenants/:tenant_id/triggers/:name — trigger details + recent executions.

#### crates/cloacina-server/src/routes/workflows.rs

- pub `upload_workflow` function L36-120 — `( State(state): State<AppState>, Extension(auth): Extension<AuthenticatedKey>, P...` — POST /tenants/:tenant_id/workflows — multipart upload of .cloacina source package.
- pub `list_workflows` function L123-174 — `( State(state): State<AppState>, Extension(auth): Extension<AuthenticatedKey>, P...` — GET /tenants/:tenant_id/workflows — list registered workflows.
- pub `get_workflow` function L177-225 — `( State(state): State<AppState>, Extension(auth): Extension<AuthenticatedKey>, P...` — GET /tenants/:tenant_id/workflows/:name — get workflow details.
- pub `delete_workflow` function L228-280 — `( State(state): State<AppState>, Extension(auth): Extension<AuthenticatedKey>, P...` — DELETE /tenants/:tenant_id/workflows/:name/:version — unregister workflow.
-  `extract_file_field` function L283-294 — `(multipart: &mut Multipart) -> Result<Vec<u8>, String>` — Extract the first file field from a multipart request.

#### crates/cloacina-server/src/routes/ws.rs

- pub `WsAuthQuery` struct L50-52 — `{ token: Option<String> }` — Query parameter for passing a single-use ticket on WebSocket upgrade.
- pub `accumulator_ws` function L101-146 — `( State(state): State<AppState>, Path(name): Path<String>, Query(query): Query<W...` — WebSocket handler for accumulator endpoints.
- pub `reactor_ws` function L153-198 — `( State(state): State<AppState>, Path(name): Path<String>, Query(query): Query<W...` — WebSocket handler for reactor endpoints.
-  `WsTokenSource` enum L55-60 — `Header | QueryTicket` — Where the auth credential came from — determines validation strategy.
-  `extract_ws_token` function L63-77 — `(headers: &axum::http::HeaderMap, query: &WsAuthQuery) -> Option<WsTokenSource>` — Extract the auth token from either the Authorization header or query param.
-  `authenticate_ws` function L80-94 — `( state: &AppState, source: WsTokenSource, ) -> Result<AuthenticatedKey, ApiErro...` — Authenticate a WebSocket upgrade request using the appropriate strategy.
-  `handle_accumulator_socket` function L205-252 — `( mut socket: axum::extract::ws::WebSocket, name: String, auth: AuthenticatedKey...` — Handle an accepted accumulator WebSocket connection.
-  `handle_reactor_socket` function L259-318 — `( mut socket: axum::extract::ws::WebSocket, name: String, auth: AuthenticatedKey...` — Handle an accepted reactor WebSocket connection.
-  `command_to_op` function L321-330 — `(cmd: &ReactorCommand) -> cloacina::computation_graph::registry::ReactorOp` — Map a ReactorCommand to its corresponding ReactorOp for authZ checks.
-  `process_reactor_command` function L333-410 — `( name: &str, cmd: ReactorCommand, registry: &EndpointRegistry, handle: &Option<...` — Process a single reactor command and return the response.

### crates/cloacina-testing/src

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina-testing/src/assertions.rs

- pub `assert_all_completed` function L27-41 — `(&self)` — Asserts that all tasks completed successfully.
- pub `assert_task_completed` function L48-63 — `(&self, task_id: &str)` — Asserts that a specific task completed successfully.
- pub `assert_task_failed` function L70-85 — `(&self, task_id: &str)` — Asserts that a specific task failed.
- pub `assert_task_skipped` function L92-107 — `(&self, task_id: &str)` — Asserts that a specific task was skipped.
-  `TestResult` type L21-108 — `= TestResult` — Assertion helpers for test results.

#### crates/cloacina-testing/src/boundary.rs

- pub `ComputationBoundary` enum L36-44 — `TimeRange | OffsetRange` — A computation boundary representing a slice of data to process.
- pub `BoundaryEmitter` struct L61-63 — `{ boundaries: Vec<ComputationBoundary> }` — Simulates detector output for testing continuous tasks.
- pub `new` function L67-71 — `() -> Self` — Create a new empty emitter.
- pub `emit` function L74-77 — `(mut self, boundary: ComputationBoundary) -> Self` — Emit a raw boundary.
- pub `emit_time_range` function L80-82 — `(self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self` — Emit a time-range boundary.
- pub `emit_offset_range` function L85-87 — `(self, start: i64, end: i64) -> Self` — Emit an offset-range boundary.
- pub `into_context` function L90-110 — `(self) -> Context<serde_json::Value>` — Convert emitted boundaries into a context matching accumulator drain output.
-  `BoundaryEmitter` type L65-111 — `= BoundaryEmitter` — lands, these will be replaced with the real types.
-  `BoundaryEmitter` type L113-117 — `impl Default for BoundaryEmitter` — lands, these will be replaced with the real types.
-  `default` function L114-116 — `() -> Self` — lands, these will be replaced with the real types.
-  `tests` module L120-172 — `-` — lands, these will be replaced with the real types.
-  `test_empty_emitter` function L124-128 — `()` — lands, these will be replaced with the real types.
-  `test_time_range_boundary` function L131-142 — `()` — lands, these will be replaced with the real types.
-  `test_offset_range_boundary` function L145-156 — `()` — lands, these will be replaced with the real types.
-  `test_multiple_boundaries` function L159-171 — `()` — lands, these will be replaced with the real types.

#### crates/cloacina-testing/src/lib.rs

- pub `assertions` module L56 — `-` — # cloacina-testing
- pub `result` module L57 — `-` — ## Feature Flags
- pub `runner` module L58 — `-` — ## Feature Flags
- pub `boundary` module L62 — `-` — ## Feature Flags
- pub `mock` module L64 — `-` — ## Feature Flags

#### crates/cloacina-testing/src/mock.rs

- pub `ConnectionDescriptor` struct L32-37 — `{ system_type: String, location: String }` — Descriptor for a mock data connection.
- pub `MockDataConnection` struct L59-62 — `{ handle: T, descriptor: ConnectionDescriptor }` — A mock data connection that returns a user-provided handle.
- pub `new` function L66-68 — `(handle: T, descriptor: ConnectionDescriptor) -> Self` — Create a new mock connection with the given handle and descriptor.
- pub `connect` function L71-73 — `(&self) -> T` — Get a clone of the underlying handle.
- pub `descriptor` function L76-78 — `(&self) -> &ConnectionDescriptor` — Get the connection descriptor.
- pub `system_metadata` function L81-83 — `(&self) -> Value` — Get system metadata (returns empty JSON object for mocks).
-  `tests` module L87-129 — `-` — once CLOACI-I-0023 lands.
-  `test_mock_connection_connect` function L91-101 — `()` — once CLOACI-I-0023 lands.
-  `test_mock_connection_descriptor` function L104-115 — `()` — once CLOACI-I-0023 lands.
-  `test_mock_connection_metadata` function L118-128 — `()` — once CLOACI-I-0023 lands.

#### crates/cloacina-testing/src/result.rs

- pub `TestResult` struct L27-32 — `{ context: Context<serde_json::Value>, task_outcomes: IndexMap<String, TaskOutco...` — The result of running tasks through a [`TestRunner`](crate::TestRunner).
- pub `TaskOutcome` enum L36-43 — `Completed | Failed | Skipped` — The outcome of a single task execution.
- pub `is_completed` function L47-49 — `(&self) -> bool` — Returns `true` if the task completed successfully.
- pub `is_failed` function L52-54 — `(&self) -> bool` — Returns `true` if the task failed.
- pub `is_skipped` function L57-59 — `(&self) -> bool` — Returns `true` if the task was skipped.
- pub `unwrap_error` function L66-74 — `(&self) -> &TaskError` — Returns the error if the task failed, panics otherwise.
-  `TaskOutcome` type L45-75 — `= TaskOutcome` — Test result types for capturing task execution outcomes.
-  `TaskOutcome` type L77-85 — `= TaskOutcome` — Test result types for capturing task execution outcomes.
-  `fmt` function L78-84 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — Test result types for capturing task execution outcomes.
-  `TestResult` type L87-99 — `= TestResult` — Test result types for capturing task execution outcomes.
-  `Output` type L88 — `= TaskOutcome` — Test result types for capturing task execution outcomes.
-  `index` function L90-98 — `(&self, task_id: &str) -> &Self::Output` — Test result types for capturing task execution outcomes.
-  `outcome_name` function L101-107 — `(outcome: &TaskOutcome) -> &'static str` — Test result types for capturing task execution outcomes.

#### crates/cloacina-testing/src/runner.rs

- pub `TestRunner` struct L50-52 — `{ tasks: IndexMap<String, Arc<dyn Task>> }` — A no-DB, in-process task executor for unit tests.
- pub `new` function L56-60 — `() -> Self` — Create a new empty test runner.
- pub `register` function L63-67 — `(mut self, task: Arc<dyn Task>) -> Self` — Register a task with the runner.
- pub `run` function L79-130 — `( &self, initial_context: cloacina_workflow::Context<serde_json::Value>, ) -> Re...` — Execute all registered tasks in topological order.
- pub `TestRunnerError` enum L233-237 — `CyclicDependency` — Errors that can occur when running the test runner.
-  `TestRunner` type L54-223 — `= TestRunner` — In-process test runner for Cloacina tasks.
-  `topological_sort` function L133-172 — `(&self) -> Result<Vec<String>, TestRunnerError>` — Build a petgraph from registered tasks and return topological order.
-  `find_cycle` function L175-190 — `(&self) -> Vec<String>` — Find a cycle in the dependency graph (for error reporting).
-  `dfs_cycle` function L192-222 — `( &self, node: &str, visited: &mut HashSet<String>, rec_stack: &mut HashSet<Stri...` — In-process test runner for Cloacina tasks.
-  `TestRunner` type L225-229 — `impl Default for TestRunner` — In-process test runner for Cloacina tasks.
-  `default` function L226-228 — `() -> Self` — In-process test runner for Cloacina tasks.
-  `tests` module L241-556 — `-` — In-process test runner for Cloacina tasks.
-  `PassTask` struct L250-255 — `{ id: String, deps: Vec<TaskNamespace>, key: String, value: serde_json::Value }` — A task that inserts a key into the context.
-  `PassTask` type L257-272 — `= PassTask` — In-process test runner for Cloacina tasks.
-  `new` function L258-265 — `(id: &str, key: &str, value: serde_json::Value) -> Self` — In-process test runner for Cloacina tasks.
-  `with_dep` function L267-271 — `(mut self, dep_id: &str) -> Self` — In-process test runner for Cloacina tasks.
-  `PassTask` type L275-289 — `impl Task for PassTask` — In-process test runner for Cloacina tasks.
-  `execute` function L276-282 — `( &self, mut context: Context<serde_json::Value>, ) -> Result<Context<serde_json...` — In-process test runner for Cloacina tasks.
-  `id` function L283-285 — `(&self) -> &str` — In-process test runner for Cloacina tasks.
-  `dependencies` function L286-288 — `(&self) -> &[TaskNamespace]` — In-process test runner for Cloacina tasks.
-  `FailTask` struct L292-296 — `{ id: String, deps: Vec<TaskNamespace>, message: String }` — A task that always fails.
-  `FailTask` type L298-312 — `= FailTask` — In-process test runner for Cloacina tasks.
-  `new` function L299-305 — `(id: &str, message: &str) -> Self` — In-process test runner for Cloacina tasks.
-  `with_dep` function L307-311 — `(mut self, dep_id: &str) -> Self` — In-process test runner for Cloacina tasks.
-  `FailTask` type L315-332 — `impl Task for FailTask` — In-process test runner for Cloacina tasks.
-  `execute` function L316-325 — `( &self, _context: Context<serde_json::Value>, ) -> Result<Context<serde_json::V...` — In-process test runner for Cloacina tasks.
-  `id` function L326-328 — `(&self) -> &str` — In-process test runner for Cloacina tasks.
-  `dependencies` function L329-331 — `(&self) -> &[TaskNamespace]` — In-process test runner for Cloacina tasks.
-  `ContextCheckTask` struct L335-339 — `{ id: String, deps: Vec<TaskNamespace>, expected_key: String }` — A task that checks a key exists in context.
-  `ContextCheckTask` type L341-355 — `= ContextCheckTask` — In-process test runner for Cloacina tasks.
-  `new` function L342-348 — `(id: &str, expected_key: &str) -> Self` — In-process test runner for Cloacina tasks.
-  `with_dep` function L350-354 — `(mut self, dep_id: &str) -> Self` — In-process test runner for Cloacina tasks.
-  `ContextCheckTask` type L358-379 — `impl Task for ContextCheckTask` — In-process test runner for Cloacina tasks.
-  `execute` function L359-372 — `( &self, mut context: Context<serde_json::Value>, ) -> Result<Context<serde_json...` — In-process test runner for Cloacina tasks.
-  `id` function L373-375 — `(&self) -> &str` — In-process test runner for Cloacina tasks.
-  `dependencies` function L376-378 — `(&self) -> &[TaskNamespace]` — In-process test runner for Cloacina tasks.
-  `test_single_task_completes` function L384-393 — `()` — In-process test runner for Cloacina tasks.
-  `test_multiple_independent_tasks` function L396-407 — `()` — In-process test runner for Cloacina tasks.
-  `test_linear_dependency_chain` function L410-428 — `()` — In-process test runner for Cloacina tasks.
-  `test_diamond_dependency` function L431-452 — `()` — In-process test runner for Cloacina tasks.
-  `test_task_failure_skips_dependents` function L455-472 — `()` — In-process test runner for Cloacina tasks.
-  `test_partial_failure_independent_branches_continue` function L475-491 — `()` — In-process test runner for Cloacina tasks.
-  `test_cycle_detection` function L494-508 — `()` — In-process test runner for Cloacina tasks.
-  `test_empty_runner` function L511-515 — `()` — In-process test runner for Cloacina tasks.
-  `test_context_propagation` function L518-532 — `()` — In-process test runner for Cloacina tasks.
-  `test_index_access` function L535-543 — `()` — In-process test runner for Cloacina tasks.
-  `test_index_missing_task_panics` function L547-555 — `()` — In-process test runner for Cloacina tasks.

### crates/cloacina-workflow/src

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina-workflow/src/context.rs

- pub `Context` struct L53-58 — `{ data: HashMap<String, T> }` — A context that holds data for pipeline execution.
- pub `new` function L74-79 — `() -> Self` — Creates a new empty context.
- pub `clone_data` function L87-95 — `(&self) -> Self` — Creates a clone of this context's data.
- pub `insert` function L122-131 — `(&mut self, key: impl Into<String>, value: T) -> Result<(), ContextError>` — Inserts a value into the context.
- pub `update` function L160-169 — `(&mut self, key: impl Into<String>, value: T) -> Result<(), ContextError>` — Updates an existing value in the context.
- pub `get` function L193-196 — `(&self, key: &str) -> Option<&T>` — Gets a reference to a value from the context.
- pub `remove` function L221-224 — `(&mut self, key: &str) -> Option<T>` — Removes and returns a value from the context.
- pub `data` function L248-250 — `(&self) -> &HashMap<String, T>` — Gets a reference to the underlying data HashMap.
- pub `into_data` function L257-259 — `(self) -> HashMap<String, T>` — Consumes the context and returns the underlying data HashMap.
- pub `from_data` function L270-272 — `(data: HashMap<String, T>) -> Self` — Creates a Context from a HashMap.
- pub `to_json` function L280-285 — `(&self) -> Result<String, ContextError>` — Serializes the context to a JSON string.
- pub `from_json` function L297-302 — `(json: String) -> Result<Self, ContextError>` — Deserializes a context from a JSON string.
-  `default` function L309-311 — `() -> Self` — like database persistence or dependency loading.
-  `tests` module L315-389 — `-` — like database persistence or dependency loading.
-  `setup_test_context` function L318-320 — `() -> Context<i32>` — like database persistence or dependency loading.
-  `test_context_operations` function L323-348 — `()` — like database persistence or dependency loading.
-  `test_context_serialization` function L351-359 — `()` — like database persistence or dependency loading.
-  `test_context_clone_data` function L362-370 — `()` — like database persistence or dependency loading.
-  `test_context_from_data` function L373-379 — `()` — like database persistence or dependency loading.
-  `test_context_into_data` function L382-388 — `()` — like database persistence or dependency loading.

#### crates/cloacina-workflow/src/error.rs

- pub `ContextError` enum L37-61 — `Serialization | KeyNotFound | TypeMismatch | KeyExists | Database | ConnectionPo...` — Errors that can occur during context operations.
- pub `TaskError` enum L68-110 — `ExecutionFailed | DependencyNotSatisfied | Timeout | ContextError | ValidationFa...` — Errors that can occur during task execution.
- pub `CheckpointError` enum L126-146 — `SaveFailed | LoadFailed | Serialization | StorageError | ValidationFailed` — Errors that can occur during task checkpointing.
-  `TaskError` type L112-119 — `= TaskError` — - [`CheckpointError`]: Errors in task checkpointing
-  `from` function L113-118 — `(error: ContextError) -> Self` — - [`CheckpointError`]: Errors in task checkpointing

#### crates/cloacina-workflow/src/lib.rs

- pub `context` module L68 — `-` — # Cloacina Workflow - Minimal Types for Workflow Authoring
- pub `error` module L69 — `-` — ```
- pub `namespace` module L70 — `-` — ```
- pub `retry` module L71 — `-` — ```
- pub `task` module L72 — `-` — ```
- pub `trigger` module L73 — `-` — ```
- pub `__private` module L90-92 — `-` — Private re-exports used by generated macro code.

#### crates/cloacina-workflow/src/namespace.rs

- pub `TaskNamespace` struct L62-79 — `{ tenant_id: String, package_name: String, workflow_id: String, task_id: String ...` — Hierarchical namespace for task identification and isolation.
- pub `new` function L93-100 — `(tenant_id: &str, package_name: &str, workflow_id: &str, task_id: &str) -> Self` — Create a complete namespace from all components.
- pub `from_string` function L127-129 — `(namespace_str: &str) -> Result<Self, String>` — Create a TaskNamespace from a string representation.
- pub `is_public` function L136-138 — `(&self) -> bool` — Check if this is a public (non-tenant-specific) namespace.
- pub `is_embedded` function L145-147 — `(&self) -> bool` — Check if this is an embedded (non-packaged) namespace.
- pub `parse_namespace` function L201-212 — `(namespace_str: &str) -> Result<TaskNamespace, String>` — Parse a namespace string back into a TaskNamespace.
-  `TaskNamespace` type L81-148 — `= TaskNamespace` — ```
-  `TaskNamespace` type L150-173 — `impl Display for TaskNamespace` — ```
-  `fmt` function L166-172 — `(&self, f: &mut Formatter) -> FmtResult` — Format the namespace as a string using the standard format.
-  `tests` module L215-312 — `-` — ```
-  `test_embedded_namespace` function L219-229 — `()` — ```
-  `test_packaged_namespace` function L232-242 — `()` — ```
-  `test_tenant_namespace` function L245-260 — `()` — ```
-  `test_namespace_display` function L263-269 — `()` — ```
-  `test_namespace_equality_and_hashing` function L272-288 — `()` — ```
-  `test_parse_namespace` function L291-302 — `()` — ```
-  `test_from_string` function L305-311 — `()` — ```

#### crates/cloacina-workflow/src/retry.rs

- pub `RetryPolicy` struct L61-79 — `{ max_attempts: i32, backoff_strategy: BackoffStrategy, initial_delay: Duration,...` — Comprehensive retry policy configuration for tasks.
- pub `BackoffStrategy` enum L87-112 — `Fixed | Linear | Exponential | Custom` — Different backoff strategies for calculating retry delays.
- pub `RetryCondition` enum L120-132 — `AllErrors | Never | TransientOnly | ErrorPattern` — Conditions that determine whether a failed task should be retried.
- pub `builder` function L161-163 — `() -> RetryPolicyBuilder` — Creates a new RetryPolicyBuilder for fluent configuration.
- pub `calculate_delay` function L174-205 — `(&self, attempt: i32) -> Duration` — Calculates the delay before the next retry attempt.
- pub `should_retry` function L217-237 — `(&self, error: &TaskError, attempt: i32) -> bool` — Determines whether a retry should be attempted based on the error and retry conditions.
- pub `calculate_retry_at` function L249-252 — `(&self, attempt: i32, now: NaiveDateTime) -> NaiveDateTime` — Calculates the absolute timestamp when the next retry should occur.
- pub `RetryPolicyBuilder` struct L296-298 — `{ policy: RetryPolicy }` — Builder for creating RetryPolicy instances with a fluent API.
- pub `new` function L302-306 — `() -> Self` — Creates a new RetryPolicyBuilder with default values.
- pub `max_attempts` function L309-312 — `(mut self, max_attempts: i32) -> Self` — Sets the maximum number of retry attempts.
- pub `backoff_strategy` function L315-318 — `(mut self, strategy: BackoffStrategy) -> Self` — Sets the backoff strategy.
- pub `initial_delay` function L321-324 — `(mut self, delay: Duration) -> Self` — Sets the initial delay before the first retry.
- pub `max_delay` function L327-330 — `(mut self, delay: Duration) -> Self` — Sets the maximum delay between retries.
- pub `with_jitter` function L333-336 — `(mut self, jitter: bool) -> Self` — Enables or disables jitter.
- pub `retry_condition` function L339-342 — `(mut self, condition: RetryCondition) -> Self` — Adds a retry condition.
- pub `retry_conditions` function L345-348 — `(mut self, conditions: Vec<RetryCondition>) -> Self` — Adds multiple retry conditions.
- pub `build` function L351-353 — `(self) -> RetryPolicy` — Builds the RetryPolicy.
-  `RetryPolicy` type L134-157 — `impl Default for RetryPolicy` — ```
-  `default` function L144-156 — `() -> Self` — Creates a default retry policy with reasonable production settings.
-  `RetryPolicy` type L159-292 — `= RetryPolicy` — ```
-  `add_jitter` function L257-262 — `(&self, delay: Duration) -> Duration` — Adds random jitter to a delay to prevent thundering herd problems.
-  `is_transient_error` function L265-273 — `(&self, error: &TaskError) -> bool` — Determines if an error is transient (network, timeout, temporary failures).
-  `message_matches_transient_patterns` function L276-291 — `(message: &str) -> bool` — Checks whether an error message contains any known transient error patterns.
-  `TRANSIENT_PATTERNS` variable L277-286 — `: &[&str]` — ```
-  `RetryPolicyBuilder` type L300-354 — `= RetryPolicyBuilder` — ```
-  `RetryPolicyBuilder` type L356-360 — `impl Default for RetryPolicyBuilder` — ```
-  `default` function L357-359 — `() -> Self` — ```
-  `tests` module L363-650 — `-` — ```
-  `test_default_retry_policy` function L367-377 — `()` — ```
-  `test_retry_policy_builder` function L380-395 — `()` — ```
-  `test_fixed_backoff_calculation` function L398-408 — `()` — ```
-  `test_linear_backoff_calculation` function L411-421 — `()` — ```
-  `test_exponential_backoff_calculation` function L424-438 — `()` — ```
-  `test_max_delay_capping` function L441-455 — `()` — ```
-  `make_execution_error` function L459-465 — `(msg: &str) -> TaskError` — ```
-  `make_unknown_error` function L467-472 — `(msg: &str) -> TaskError` — ```
-  `test_timeout_is_transient` function L475-482 — `()` — ```
-  `test_connection_error_is_transient` function L485-493 — `()` — ```
-  `test_unknown_error_with_transient_message_is_transient` function L496-500 — `()` — ```
-  `test_permanent_errors_are_not_transient` function L503-508 — `()` — ```
-  `test_non_retryable_error_variants_are_not_transient` function L511-534 — `()` — ```
-  `test_transient_pattern_matching_is_case_insensitive` function L537-542 — `()` — ```
-  `test_should_retry_all_errors_within_limit` function L547-558 — `()` — ```
-  `test_should_retry_never_condition` function L561-568 — `()` — ```
-  `test_should_retry_transient_only` function L571-579 — `()` — ```
-  `test_should_retry_error_pattern` function L582-593 — `()` — ```
-  `test_should_retry_zero_max_attempts` function L596-603 — `()` — ```
-  `test_custom_backoff_falls_back_to_exponential` function L606-618 — `()` — ```
-  `test_jitter_stays_within_bounds` function L621-635 — `()` — ```
-  `test_message_matches_transient_patterns_directly` function L638-649 — `()` — ```

#### crates/cloacina-workflow/src/task.rs

- pub `TaskState` enum L45-62 — `Pending | Running | Completed | Failed | Skipped` — Represents the execution state of a task throughout its lifecycle.
- pub `is_completed` function L66-68 — `(&self) -> bool` — Returns true if the task is in the completed state
- pub `is_failed` function L71-73 — `(&self) -> bool` — Returns true if the task is in the failed state
- pub `is_running` function L76-78 — `(&self) -> bool` — Returns true if the task is currently running
- pub `is_pending` function L81-83 — `(&self) -> bool` — Returns true if the task is pending execution
- pub `is_skipped` function L86-88 — `(&self) -> bool` — Returns true if the task was skipped
- pub `Task` interface L118-222 — `{ fn execute(), fn id(), fn dependencies(), fn checkpoint(), fn retry_policy(), ...` — Core trait that defines an executable task in a pipeline.
-  `TaskState` type L64-89 — `= TaskState` — executable tasks in Cloacina workflows.
-  `checkpoint` function L164-167 — `(&self, _context: &Context<serde_json::Value>) -> Result<(), CheckpointError>` — Saves a checkpoint for this task.
-  `retry_policy` function L177-179 — `(&self) -> RetryPolicy` — Returns the retry policy for this task.
-  `trigger_rules` function L191-193 — `(&self) -> serde_json::Value` — Returns the trigger rules for this task.
-  `code_fingerprint` function L208-210 — `(&self) -> Option<String>` — Returns a code fingerprint for content-based versioning.
-  `requires_handle` function L219-221 — `(&self) -> bool` — Returns whether this task requires a `TaskHandle` for execution control.

#### crates/cloacina-workflow/src/trigger.rs

- pub `TriggerResult` enum L26-31 — `Skip | Fire` — Result of a trigger poll operation.
- pub `TriggerError` enum L35-42 — `PollError | ContextError` — Errors that can occur during trigger polling.

### crates/cloacina-workflow-plugin/src

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina-workflow-plugin/src/lib.rs

- pub `types` module L35 — `-` — Cloacina plugin interface for the fidius plugin system.
- pub `CloacinaPlugin` interface L77-101 — `{ fn get_task_metadata(), fn execute_task(), fn get_graph_metadata(), fn execute...` — The plugin interface for cloacina workflow packages.

#### crates/cloacina-workflow-plugin/src/types.rs

- pub `TaskMetadataEntry` struct L30-43 — `{ index: u32, id: String, namespaced_id_template: String, dependencies: Vec<Stri...` — Metadata for a single task within a workflow package.
- pub `PackageTasksMetadata` struct L47-62 — `{ workflow_name: String, package_name: String, package_description: Option<Strin...` — Complete metadata for a workflow package, returned by `get_task_metadata()`.
- pub `TaskExecutionRequest` struct L66-71 — `{ task_name: String, context_json: String }` — Request to execute a task within a workflow package.
- pub `TaskExecutionResult` struct L75-82 — `{ success: bool, context_json: Option<String>, error: Option<String> }` — Result of a task execution.
- pub `GraphPackageMetadata` struct L90-102 — `{ graph_name: String, package_name: String, reaction_mode: String, input_strateg...` — Metadata for a computation graph package, returned by `get_graph_metadata()`.
- pub `AccumulatorDeclarationEntry` struct L110-118 — `{ name: String, accumulator_type: String, config: std::collections::HashMap<Stri...` — Declaration of an accumulator within a computation graph package.
- pub `GraphExecutionRequest` struct L122-125 — `{ cache: std::collections::HashMap<String, String> }` — Request to execute a computation graph.
- pub `GraphExecutionResult` struct L129-136 — `{ success: bool, terminal_outputs_json: Option<Vec<String>>, error: Option<Strin...` — Result of a computation graph execution.
- pub `CloacinaMetadata` struct L148-185 — `{ package_type: Vec<String>, workflow_name: Option<String>, graph_name: Option<S...` — Host-defined metadata schema for cloacina packages.
- pub `AccumulatorConfig` struct L189-198 — `{ name: String, accumulator_type: String, config: std::collections::HashMap<Stri...` — Accumulator configuration from package.toml metadata.
- pub `has_workflow` function L210-212 — `(&self) -> bool` — Check if this package contains a workflow.
- pub `has_computation_graph` function L215-217 — `(&self) -> bool` — Check if this package contains a computation graph.
- pub `effective_workflow_name` function L222-224 — `(&self) -> Option<&str>` — Get the workflow name, falling back for backward compatibility.
- pub `TriggerDefinition` struct L229-242 — `{ name: String, workflow: String, poll_interval: String, cron_expression: Option...` — A trigger definition within a workflow package manifest.
-  `default_input_strategy` function L104-106 — `() -> String` — no manual `#[repr(C)]` structs or `CStr` handling needed.
-  `default_accumulator_type` function L200-202 — `() -> String` — no manual `#[repr(C)]` structs or `CStr` handling needed.
-  `default_package_type` function L204-206 — `() -> Vec<String>` — no manual `#[repr(C)]` structs or `CStr` handling needed.
-  `CloacinaMetadata` type L208-225 — `= CloacinaMetadata` — no manual `#[repr(C)]` structs or `CStr` handling needed.
-  `tests` module L245-511 — `-` — no manual `#[repr(C)]` structs or `CStr` handling needed.
-  `test_task_metadata_serde_round_trip` function L249-263 — `()` — no manual `#[repr(C)]` structs or `CStr` handling needed.
-  `test_package_tasks_metadata_serde_round_trip` function L266-288 — `()` — no manual `#[repr(C)]` structs or `CStr` handling needed.
-  `test_task_execution_request_round_trip` function L291-300 — `()` — no manual `#[repr(C)]` structs or `CStr` handling needed.
-  `test_task_execution_result_success` function L303-315 — `()` — no manual `#[repr(C)]` structs or `CStr` handling needed.
-  `test_task_execution_result_failure` function L318-329 — `()` — no manual `#[repr(C)]` structs or `CStr` handling needed.
-  `test_cloacina_metadata_rust_from_toml` function L332-361 — `()` — no manual `#[repr(C)]` structs or `CStr` handling needed.
-  `test_cloacina_metadata_python_from_toml` function L364-379 — `()` — no manual `#[repr(C)]` structs or `CStr` handling needed.
-  `test_cloacina_metadata_minimal_rust` function L382-393 — `()` — no manual `#[repr(C)]` structs or `CStr` handling needed.
-  `test_cloacina_metadata_missing_language_fails` function L396-403 — `()` — no manual `#[repr(C)]` structs or `CStr` handling needed.
-  `test_cloacina_metadata_defaults_to_workflow_package_type` function L406-416 — `()` — no manual `#[repr(C)]` structs or `CStr` handling needed.
-  `test_cloacina_metadata_computation_graph_from_toml` function L419-435 — `()` — no manual `#[repr(C)]` structs or `CStr` handling needed.
-  `test_cloacina_metadata_both_types` function L438-449 — `()` — no manual `#[repr(C)]` structs or `CStr` handling needed.
-  `test_graph_package_metadata_round_trip` function L452-483 — `()` — no manual `#[repr(C)]` structs or `CStr` handling needed.
-  `test_graph_execution_request_round_trip` function L486-496 — `()` — no manual `#[repr(C)]` structs or `CStr` handling needed.
-  `test_graph_execution_result_round_trip` function L499-510 — `()` — no manual `#[repr(C)]` structs or `CStr` handling needed.

### crates/cloacinactl

> *Semantic summary to be generated by AI agent.*

#### crates/cloacinactl/build.rs

-  `main` function L17-19 — `()`

### crates/cloacinactl/src/commands

> *Semantic summary to be generated by AI agent.*

#### crates/cloacinactl/src/commands/cleanup_events.rs

- pub `run` function L99-151 — `(database_url: &str, older_than: &str, dry_run: bool) -> Result<()>` — Run the cleanup-events command.
-  `parse_duration` function L40-90 — `(s: &str) -> Result<Duration>` — Parse a duration string like "90d", "30d", "7d", "24h", "1h30m" into a chrono::Duration.
-  `tests` module L154-221 — `-` — Cleans up old execution events from the database based on a retention policy.
-  `test_parse_duration_days` function L158-161 — `()` — Cleans up old execution events from the database based on a retention policy.
-  `test_parse_duration_hours` function L164-167 — `()` — Cleans up old execution events from the database based on a retention policy.
-  `test_parse_duration_minutes` function L170-173 — `()` — Cleans up old execution events from the database based on a retention policy.
-  `test_parse_duration_seconds` function L176-179 — `()` — Cleans up old execution events from the database based on a retention policy.
-  `test_parse_duration_combined` function L182-185 — `()` — Cleans up old execution events from the database based on a retention policy.
-  `test_parse_duration_complex` function L188-194 — `()` — Cleans up old execution events from the database based on a retention policy.
-  `test_parse_duration_case_insensitive` function L197-200 — `()` — Cleans up old execution events from the database based on a retention policy.
-  `test_parse_duration_empty` function L203-205 — `()` — Cleans up old execution events from the database based on a retention policy.
-  `test_parse_duration_missing_unit` function L208-210 — `()` — Cleans up old execution events from the database based on a retention policy.
-  `test_parse_duration_invalid_unit` function L213-215 — `()` — Cleans up old execution events from the database based on a retention policy.
-  `test_parse_duration_zero` function L218-220 — `()` — Cleans up old execution events from the database based on a retention policy.

#### crates/cloacinactl/src/commands/config.rs

- pub `CloacinaConfig` struct L33-53 — `{ database_url: Option<String>, default_profile: Option<String>, profiles: BTree...` — Full configuration file structure.
- pub `Profile` struct L58-64 — `{ server: String, api_key: String }` — A named server-targeting profile.
- pub `DaemonSection` struct L68-86 — `{ poll_interval_ms: u64, log_level: String, shutdown_timeout_s: u64, watcher_deb...` — - Config value lookup for commands that need database_url etc.
- pub `WatchSection` struct L105-107 — `{ directories: Vec<String> }` — - Config value lookup for commands that need database_url etc.
- pub `load` function L112-141 — `(path: &Path) -> Self` — Load config from a TOML file.
- pub `save` function L144-154 — `(&self, path: &Path) -> Result<()>` — Save config to a TOML file.
- pub `resolve_watch_dirs` function L157-170 — `(&self) -> Vec<PathBuf>` — Resolve watch directories from config, expanding `~` to home dir.
- pub `get` function L173-177 — `(&self, key: &str) -> Option<String>` — Get a config value by dotted key path (e.g., "daemon.poll_interval_ms").
- pub `set` function L180-192 — `(&mut self, key: &str, value: &str) -> Result<()>` — Set a config value by dotted key path.
- pub `list` function L195-203 — `(&self) -> Vec<(String, String)>` — List all config key-value pairs.
- pub `run_get` function L301-312 — `(config_path: &Path, key: &str) -> Result<()>` — Run `cloacinactl config get <key>`.
- pub `run_set` function L315-321 — `(config_path: &Path, key: &str, value: &str) -> Result<()>` — Run `cloacinactl config set <key> <value>`.
- pub `run_list` function L324-335 — `(config_path: &Path) -> Result<()>` — Run `cloacinactl config list`.
- pub `run_profile_set` function L338-360 — `( config_path: &Path, name: &str, server: &str, api_key: &str, default: bool, ) ...` — Run `cloacinactl config profile set <NAME> <URL> --api-key <K> [--default]`.
- pub `run_profile_list` function L363-380 — `(config_path: &Path) -> Result<()>` — Run `cloacinactl config profile list`.
- pub `run_profile_use` function L383-392 — `(config_path: &Path, name: &str) -> Result<()>` — Run `cloacinactl config profile use <NAME>`.
- pub `run_profile_delete` function L395-406 — `(config_path: &Path, name: &str) -> Result<()>` — Run `cloacinactl config profile delete <NAME>`.
- pub `resolve_database_url` function L423-437 — `(cli_url: Option<&str>, config_path: &Path) -> Result<String>` — Resolve database_url from CLI arg or config file.
-  `DaemonSection` type L88-101 — `impl Default for DaemonSection` — - Config value lookup for commands that need database_url etc.
-  `default` function L89-100 — `() -> Self` — - Config value lookup for commands that need database_url etc.
-  `CloacinaConfig` type L109-204 — `= CloacinaConfig` — - Config value lookup for commands that need database_url etc.
-  `resolve_key` function L207-214 — `(value: &'a toml::Value, key: &str) -> Option<&'a toml::Value>` — Resolve a dotted key path in a TOML value tree.
-  `set_key` function L217-263 — `(root: &mut toml::Value, key: &str, value: &str) -> Result<()>` — Set a value at a dotted key path in a TOML value tree.
-  `collect_pairs` function L266-282 — `(value: &toml::Value, prefix: &str, pairs: &mut Vec<(String, String)>)` — Collect all leaf key-value pairs with dotted paths.
-  `format_value` function L285-298 — `(value: &toml::Value) -> String` — Format a TOML value for display.
-  `redact_secret` function L412-420 — `(raw: &str) -> String` — Short redacted form of a secret for display.
-  `tests` module L440-588 — `-` — - Config value lookup for commands that need database_url etc.
-  `config_defaults_are_sensible` function L445-457 — `()` — - Config value lookup for commands that need database_url etc.
-  `config_load_missing_file_returns_defaults` function L460-464 — `()` — - Config value lookup for commands that need database_url etc.
-  `config_load_valid_toml` function L467-495 — `()` — - Config value lookup for commands that need database_url etc.
-  `config_load_invalid_toml_returns_defaults` function L498-507 — `()` — - Config value lookup for commands that need database_url etc.
-  `config_load_partial_toml_fills_defaults` function L510-520 — `()` — - Config value lookup for commands that need database_url etc.
-  `config_resolve_watch_dirs_expands_tilde` function L523-534 — `()` — - Config value lookup for commands that need database_url etc.
-  `config_resolve_watch_dirs_empty` function L537-540 — `()` — - Config value lookup for commands that need database_url etc.
-  `config_save_and_reload_roundtrip` function L543-559 — `()` — - Config value lookup for commands that need database_url etc.
-  `config_get_dotted_key` function L562-570 — `()` — - Config value lookup for commands that need database_url etc.
-  `config_set_dotted_key` function L573-577 — `()` — - Config value lookup for commands that need database_url etc.
-  `config_list_returns_all_keys` function L580-587 — `()` — - Config value lookup for commands that need database_url etc.

#### crates/cloacinactl/src/commands/daemon.rs

- pub `run` function L118-398 — `( home: PathBuf, watch_dirs: Vec<PathBuf>, poll_interval_ms: u64, verbose: bool,...` — Run the daemon.
-  `collect_watch_dirs` function L43-55 — `( packages_dir: &Path, cli_dirs: &[PathBuf], config_dirs: &[PathBuf], ) -> Vec<P...` — Merge watch directories from multiple sources, deduplicating.
-  `apply_watch_dir_changes` function L61-84 — `( watcher: &mut PackageWatcher, current: &[PathBuf], new: &[PathBuf], )` — Diff watch directories and apply changes to the watcher.
-  `handle_reconcile` function L87-107 — `( runner: &DefaultRunner, registry: &Arc<FilesystemWorkflowRegistry>, result: &R...` — Handle a reconciliation result: log changes/failures and register triggers.
-  `register_triggers_from_reconcile` function L402-520 — `( runner: &DefaultRunner, registry: &Arc<FilesystemWorkflowRegistry>, result: &R...` — After reconciliation loads new packages, register their triggers with the
-  `tests` module L523-583 — `-` — package storage.
-  `collect_watch_dirs_deduplicates` function L528-546 — `()` — package storage.
-  `collect_watch_dirs_packages_dir_always_first` function L549-557 — `()` — package storage.
-  `collect_watch_dirs_empty_sources` function L560-564 — `()` — package storage.
-  `collect_watch_dirs_preserves_order` function L567-582 — `()` — package storage.

#### crates/cloacinactl/src/commands/health.rs

- pub `DaemonHealth` struct L31-38 — `{ status: String, pid: u32, uptime_seconds: u64, database: DatabaseHealth, recon...` — Health response served over the Unix socket.
- pub `DatabaseHealth` struct L41-44 — `{ connected: bool, backend: String }` — Daemon health observability — shared state, Unix socket listener, and log pulse.
- pub `ReconcilerHealth` struct L47-50 — `{ packages_loaded: usize, last_run_at: Option<String> }` — Daemon health observability — shared state, Unix socket listener, and log pulse.
- pub `SharedDaemonState` struct L53-57 — `{ start_time: Instant, packages_loaded: AtomicUsize, last_reconciliation: Mutex<...` — Mutable state updated by the daemon's main loop.
- pub `new` function L60-66 — `() -> Self` — Daemon health observability — shared state, Unix socket listener, and log pulse.
- pub `set_packages_loaded` function L68-70 — `(&self, count: usize)` — Daemon health observability — shared state, Unix socket listener, and log pulse.
- pub `set_last_reconciliation` function L72-74 — `(&self, time: chrono::DateTime<chrono::Utc>)` — Daemon health observability — shared state, Unix socket listener, and log pulse.
- pub `build_health` function L78-122 — `( dal: &cloacina::dal::DAL, state: &SharedDaemonState, db_backend: &str, ) -> Da...` — Build a health snapshot by querying DB and reading shared state.
- pub `run_health_socket` function L128-187 — `( socket_path: PathBuf, dal: cloacina::dal::DAL, state: Arc<SharedDaemonState>, ...` — Accept connections on a Unix domain socket and serve health JSON.
- pub `run_health_pulse` function L190-218 — `( dal: cloacina::dal::DAL, state: Arc<SharedDaemonState>, db_backend: String, in...` — Emit a periodic structured health log line.
-  `SharedDaemonState` type L59-75 — `= SharedDaemonState` — Daemon health observability — shared state, Unix socket listener, and log pulse.

#### crates/cloacinactl/src/commands/mod.rs

- pub `cleanup_events` module L19 — `-` — CLI command implementations.
- pub `config` module L20 — `-` — CLI command implementations.
- pub `daemon` module L21 — `-` — CLI command implementations.
- pub `health` module L22 — `-` — CLI command implementations.
- pub `status` module L23 — `-` — CLI command implementations.
- pub `watcher` module L24 — `-` — CLI command implementations.

#### crates/cloacinactl/src/commands/status.rs

- pub `run` function L28-54 — `(home: PathBuf) -> Result<()>` — Connect to the daemon's Unix socket and display health status.
-  `display_health` function L56-81 — `(health: &DaemonHealth)` — `cloacinactl status` — queries the daemon health socket and displays status.
-  `format_duration` function L83-98 — `(seconds: u64) -> String` — `cloacinactl status` — queries the daemon health socket and displays status.

#### crates/cloacinactl/src/commands/watcher.rs

- pub `ReconcileSignal` struct L31 — `-` — Signal sent when the watcher detects a relevant filesystem change.
- pub `PackageWatcher` struct L35-37 — `{ _watcher: RecommendedWatcher }` — Watches directories for `.cloacina` file changes and signals the daemon
- pub `new` function L47-128 — `( watch_dirs: &[PathBuf], debounce: Duration, ) -> Result<(Self, mpsc::Receiver<...` — Create a new watcher monitoring the given directories.
- pub `watch_dir` function L131-135 — `(&mut self, dir: &Path) -> Result<(), notify::Error>` — Add a new directory to the watcher.
- pub `unwatch_dir` function L138-142 — `(&mut self, dir: &Path) -> Result<(), notify::Error>` — Remove a directory from the watcher.
-  `PackageWatcher` type L39-143 — `= PackageWatcher` — modified, or removed.
-  `tests` module L146-337 — `-` — modified, or removed.
-  `watcher_creates_on_valid_directory` function L152-157 — `()` — modified, or removed.
-  `settle` function L160-162 — `()` — kqueue (macOS) needs time to register the watch before events fire.
-  `watcher_signals_on_cloacina_file_create` function L165-182 — `()` — modified, or removed.
-  `watcher_ignores_non_cloacina_files` function L185-199 — `()` — modified, or removed.
-  `watcher_signals_on_cloacina_file_modify` function L202-225 — `()` — modified, or removed.
-  `watcher_signals_on_cloacina_file_remove` function L228-251 — `()` — modified, or removed.
-  `watcher_debounces_rapid_changes` function L254-279 — `()` — modified, or removed.
-  `watcher_watch_dir_adds_directory` function L282-301 — `()` — modified, or removed.
-  `watcher_unwatch_dir_removes_directory` function L304-324 — `()` — modified, or removed.
-  `watcher_skips_nonexistent_directories` function L327-336 — `()` — modified, or removed.

### crates/cloacinactl/src

> *Semantic summary to be generated by AI agent.*

#### crates/cloacinactl/src/main.rs

- pub `GlobalOpts` struct L51-87 — `{ verbose: bool, home: PathBuf, profile: Option<String>, server: Option<String>,...` — is a documented exception — a composite view over daemon + server.
- pub `OutputFormat` enum L90-97 — `Table | Json | Yaml | Id` — is a documented exception — a composite view over daemon + server.
- pub `effective_output` function L100-106 — `(&self) -> OutputFormat` — is a documented exception — a composite view over daemon + server.
-  `commands` module L30 — `-` — is a documented exception — a composite view over daemon + server.
-  `nouns` module L31 — `-` — is a documented exception — a composite view over daemon + server.
-  `shared` module L32 — `-` — is a documented exception — a composite view over daemon + server.
-  `Cli` struct L42-48 — `{ globals: GlobalOpts, command: Commands }` — cloacinactl — Cloacina task orchestration engine
-  `GlobalOpts` type L99-107 — `= GlobalOpts` — is a documented exception — a composite view over daemon + server.
-  `Commands` enum L110-158 — `Daemon | Server | Package | Workflow | Graph | Execution | Tenant | Key | Trigge...` — is a documented exception — a composite view over daemon + server.
-  `ConfigCommands` enum L161-179 — `Get | Set | List | Profile` — is a documented exception — a composite view over daemon + server.
-  `ProfileCommands` enum L182-202 — `Set | List | Use | Delete` — is a documented exception — a composite view over daemon + server.
-  `AdminCommands` enum L205-217 — `CleanupEvents` — is a documented exception — a composite view over daemon + server.
-  `default_home` function L219-223 — `() -> PathBuf` — is a documented exception — a composite view over daemon + server.
-  `main` function L226-234 — `() -> ExitCode` — is a documented exception — a composite view over daemon + server.
-  `run` function L236-316 — `() -> std::result::Result<(), CliError>` — is a documented exception — a composite view over daemon + server.

### crates/cloacinactl/src/nouns/daemon

> *Semantic summary to be generated by AI agent.*

#### crates/cloacinactl/src/nouns/daemon/health.rs

- pub `run` function L23-45 — `(globals: &GlobalOpts) -> Result<()>`

#### crates/cloacinactl/src/nouns/daemon/mod.rs

- pub `health` module L25 — `-` — `cloacinactl daemon <verb>` — local scheduler verbs.
- pub `start` module L26 — `-` — `cloacinactl daemon <verb>` — local scheduler verbs.
- pub `status` module L27 — `-` — `cloacinactl daemon <verb>` — local scheduler verbs.
- pub `stop` module L28 — `-` — `cloacinactl daemon <verb>` — local scheduler verbs.
- pub `DaemonCmd` struct L31-34 — `{ verb: DaemonVerb }` — `cloacinactl daemon <verb>` — local scheduler verbs.
- pub `run` function L61-71 — `(self, globals: &GlobalOpts) -> Result<()>` — `cloacinactl daemon <verb>` — local scheduler verbs.
-  `DaemonVerb` enum L37-58 — `Start | Stop | Status | Health` — `cloacinactl daemon <verb>` — local scheduler verbs.
-  `DaemonCmd` type L60-72 — `= DaemonCmd` — `cloacinactl daemon <verb>` — local scheduler verbs.

#### crates/cloacinactl/src/nouns/daemon/start.rs

- pub `run` function L23-38 — `(globals: &GlobalOpts, watch_dirs: Vec<PathBuf>, poll_interval: u64) -> Result<(...`

#### crates/cloacinactl/src/nouns/daemon/status.rs

- pub `run` function L21-23 — `(globals: &GlobalOpts) -> Result<()>`

#### crates/cloacinactl/src/nouns/daemon/stop.rs

- pub `run` function L22-30 — `(globals: &GlobalOpts, force: bool) -> Result<()>`

### crates/cloacinactl/src/nouns/execution

> *Semantic summary to be generated by AI agent.*

#### crates/cloacinactl/src/nouns/execution/mod.rs

- pub `ExecutionCmd` struct L29-32 — `{ verb: ExecutionVerb }` — `cloacinactl execution <verb>`.
- pub `run` function L62-111 — `(self, globals: &GlobalOpts) -> Result<(), CliError>` — `cloacinactl execution <verb>`.
-  `ExecutionVerb` enum L35-59 — `List | Status | Events | Cancel` — `cloacinactl execution <verb>`.
-  `ExecutionCmd` type L61-112 — `= ExecutionCmd` — `cloacinactl execution <verb>`.

### crates/cloacinactl/src/nouns/graph

> *Semantic summary to be generated by AI agent.*

#### crates/cloacinactl/src/nouns/graph/mod.rs

- pub `GraphCmd` struct L29-32 — `{ verb: GraphVerb }` — `cloacinactl graph <verb>` — computation graphs.
- pub `run` function L43-72 — `(self, globals: &GlobalOpts) -> Result<(), CliError>` — `cloacinactl graph <verb>` — computation graphs.
-  `GraphVerb` enum L35-40 — `List | Status | Pause | Resume` — `cloacinactl graph <verb>` — computation graphs.
-  `GraphCmd` type L42-73 — `= GraphCmd` — `cloacinactl graph <verb>` — computation graphs.

### crates/cloacinactl/src/nouns/key

> *Semantic summary to be generated by AI agent.*

#### crates/cloacinactl/src/nouns/key/mod.rs

- pub `KeyCmd` struct L27-30 — `{ verb: KeyVerb }`
- pub `Role` enum L33-37 — `Admin | Write | Read`
- pub `run` function L71-118 — `(self, globals: &GlobalOpts) -> Result<(), CliError>`
-  `KeyVerb` enum L40-58 — `Create | List | Revoke`
-  `KeyVerb` type L60-68 — `= KeyVerb`
-  `role_str` function L61-67 — `(r: Role) -> &'static str`
-  `KeyCmd` type L70-119 — `= KeyCmd`

### crates/cloacinactl/src/nouns

> *Semantic summary to be generated by AI agent.*

#### crates/cloacinactl/src/nouns/mod.rs

- pub `daemon` module L24 — `-` — methods on the noun's `Cmd` struct.
- pub `execution` module L25 — `-` — methods on the noun's `Cmd` struct.
- pub `graph` module L26 — `-` — methods on the noun's `Cmd` struct.
- pub `key` module L27 — `-` — methods on the noun's `Cmd` struct.
- pub `package` module L28 — `-` — methods on the noun's `Cmd` struct.
- pub `server` module L29 — `-` — methods on the noun's `Cmd` struct.
- pub `tenant` module L30 — `-` — methods on the noun's `Cmd` struct.
- pub `trigger` module L31 — `-` — methods on the noun's `Cmd` struct.
- pub `workflow` module L32 — `-` — methods on the noun's `Cmd` struct.
- pub `top_level_status` function L36-49 — `(globals: &GlobalOpts) -> Result<()>` — Composite status — runs daemon status + server status and prints both.

### crates/cloacinactl/src/nouns/package

> *Semantic summary to be generated by AI agent.*

#### crates/cloacinactl/src/nouns/package/build.rs

- pub `run` function L22-55 — `(dir: &Path, release: bool) -> Result<(), CliError>`

#### crates/cloacinactl/src/nouns/package/delete.rs

- pub `run` function L25-45 — `(globals: &GlobalOpts, id: &str, force: bool) -> Result<(), CliError>`

#### crates/cloacinactl/src/nouns/package/inspect.rs

- pub `run` function L25-62 — `(globals: &GlobalOpts, id: &str) -> Result<(), CliError>`
-  `json_str` function L64-69 — `(v: &Value, key: &str) -> String`

#### crates/cloacinactl/src/nouns/package/list.rs

- pub `run` function L25-48 — `(globals: &GlobalOpts, filter: Option<&str>) -> Result<(), CliError>`
-  `render_list` function L50-101 — `(items: &[Value], format: OutputFormat) -> Result<(), CliError>`
-  `truncate_id` function L103-109 — `(id: &str) -> String`

#### crates/cloacinactl/src/nouns/package/mod.rs

- pub `build` module L26 — `-` — inspect / delete.
- pub `delete` module L27 — `-` — inspect / delete.
- pub `inspect` module L28 — `-` — inspect / delete.
- pub `list` module L29 — `-` — inspect / delete.
- pub `pack` module L30 — `-` — inspect / delete.
- pub `publish` module L31 — `-` — inspect / delete.
- pub `upload` module L32 — `-` — inspect / delete.
- pub `PackageCmd` struct L35-38 — `{ verb: PackageVerb }` — inspect / delete.
- pub `run` function L85-99 — `(self, globals: &GlobalOpts) -> Result<(), CliError>` — inspect / delete.
-  `PackageVerb` enum L41-82 — `Build | Pack | Publish | Upload | List | Inspect | Delete` — inspect / delete.
-  `PackageCmd` type L84-100 — `= PackageCmd` — inspect / delete.

#### crates/cloacinactl/src/nouns/package/pack.rs

- pub `run` function L21-44 — `(dir: &Path, out: Option<&Path>, sign: Option<&Path>) -> Result<(), CliError>`

#### crates/cloacinactl/src/nouns/package/publish.rs

- pub `run` function L23-44 — `( globals: &GlobalOpts, dir: &Path, release: bool, sign: Option<&Path>, ) -> Res...`

#### crates/cloacinactl/src/nouns/package/upload.rs

- pub `run` function L26-68 — `(globals: &GlobalOpts, file: &Path) -> Result<(), CliError>`

### crates/cloacinactl/src/nouns/server

> *Semantic summary to be generated by AI agent.*

#### crates/cloacinactl/src/nouns/server/health.rs

- pub `run` function L24-56 — `(globals: &GlobalOpts) -> Result<()>`

#### crates/cloacinactl/src/nouns/server/mod.rs

- pub `health` module L25 — `-` — `cloacinactl server <verb>` — cloacina-server HTTP API verbs.
- pub `start` module L26 — `-` — `cloacinactl server <verb>` — cloacina-server HTTP API verbs.
- pub `status` module L27 — `-` — `cloacinactl server <verb>` — cloacina-server HTTP API verbs.
- pub `stop` module L28 — `-` — `cloacinactl server <verb>` — cloacina-server HTTP API verbs.
- pub `ServerCmd` struct L31-34 — `{ verb: ServerVerb }` — `cloacinactl server <verb>` — cloacina-server HTTP API verbs.
- pub `run` function L65-86 — `(self, globals: &GlobalOpts) -> Result<()>` — `cloacinactl server <verb>` — cloacina-server HTTP API verbs.
-  `ServerVerb` enum L37-62 — `Start | Stop | Status | Health` — `cloacinactl server <verb>` — cloacina-server HTTP API verbs.
-  `ServerCmd` type L64-87 — `= ServerCmd` — `cloacinactl server <verb>` — cloacina-server HTTP API verbs.

#### crates/cloacinactl/src/nouns/server/start.rs

- pub `run` function L26-59 — `( globals: &GlobalOpts, bind: SocketAddr, database_url: Option<String>, bootstra...`
-  `_config_type_check` function L63 — `(_: CloacinaConfig)`

#### crates/cloacinactl/src/nouns/server/status.rs

- pub `run` function L24-64 — `(globals: &GlobalOpts) -> Result<()>`

#### crates/cloacinactl/src/nouns/server/stop.rs

- pub `run` function L22-37 — `(globals: &GlobalOpts, force: bool) -> Result<()>`

### crates/cloacinactl/src/nouns/tenant

> *Semantic summary to be generated by AI agent.*

#### crates/cloacinactl/src/nouns/tenant/mod.rs

- pub `TenantCmd` struct L27-30 — `{ verb: TenantVerb }`
- pub `run` function L48-75 — `(self, globals: &GlobalOpts) -> Result<(), CliError>`
-  `TenantVerb` enum L33-45 — `Create | List | Delete`
-  `TenantCmd` type L47-76 — `= TenantCmd`

### crates/cloacinactl/src/nouns/trigger

> *Semantic summary to be generated by AI agent.*

#### crates/cloacinactl/src/nouns/trigger/mod.rs

- pub `TriggerCmd` struct L27-30 — `{ verb: TriggerVerb }`
- pub `run` function L39-54 — `(self, globals: &GlobalOpts) -> Result<(), CliError>`
-  `TriggerVerb` enum L33-36 — `List | Inspect`
-  `TriggerCmd` type L38-55 — `= TriggerCmd`

### crates/cloacinactl/src/nouns/workflow

> *Semantic summary to be generated by AI agent.*

#### crates/cloacinactl/src/nouns/workflow/mod.rs

- pub `WorkflowCmd` struct L30-33 — `{ verb: WorkflowVerb }` — `cloacinactl workflow <verb>`.
- pub `run` function L58-110 — `(self, globals: &GlobalOpts) -> Result<(), CliError>` — `cloacinactl workflow <verb>`.
-  `WorkflowVerb` enum L36-55 — `List | Inspect | Run | Enable | Disable` — `cloacinactl workflow <verb>`.
-  `WorkflowCmd` type L57-111 — `= WorkflowCmd` — `cloacinactl workflow <verb>`.
-  `load_context` function L113-126 — `(source: Option<&str>) -> Result<serde_json::Value, CliError>` — `cloacinactl workflow <verb>`.

### crates/cloacinactl/src/shared

> *Semantic summary to be generated by AI agent.*

#### crates/cloacinactl/src/shared/client.rs

- pub `KeyScope` enum L36-41 — `Admin | Tenant` — Scope of the caller's API key as reported by `GET /v1/keys/self`.
- pub `WhoAmI` struct L45-50 — `{ scope: KeyScope, role: Option<String> }` — What `whoami` returns.
- pub `CliClient` struct L53-57 — `{ ctx: ClientContext, http: reqwest::Client, whoami_cache: OnceLock<WhoAmI> }` — Shared HTTP client used by every verb handler.
- pub `confirm_destructive` function L61-80 — `(action: &str) -> Result<(), CliError>` — Prompt the user for destructive-op confirmation unless stdin isn't a TTY
- pub `new` function L83-94 — `(ctx: ClientContext) -> Result<Arc<Self>, CliError>` — rule from ADR-0003 §4.
- pub `ctx` function L96-98 — `(&self) -> &ClientContext` — rule from ADR-0003 §4.
- pub `get` function L133-140 — `(&self, path: &str) -> Result<T, CliError>` — Typed GET.
- pub `post` function L143-156 — `( &self, path: &str, body: &B, ) -> Result<T, CliError>` — Typed POST (JSON body).
- pub `delete` function L159-172 — `(&self, path: &str) -> Result<(), CliError>` — DELETE without a response body.
- pub `whoami` function L175-183 — `(&self) -> Result<&WhoAmI, CliError>` — Cache-aware `GET /v1/keys/self`.
- pub `require_tenant` function L191-212 — `( &self, tenant_scoped_command: bool, ) -> Result<Option<String>, CliError>` — Resolve the tenant to use for the current command per ADR §4.
-  `CliClient` type L82-213 — `= CliClient` — rule from ADR-0003 §4.
-  `url` function L100-104 — `(&self, path: &str) -> String` — rule from ADR-0003 §4.
-  `apply_auth` function L106-116 — `( &self, req: reqwest::RequestBuilder, tenant: Option<&str>, ) -> reqwest::Reque...` — rule from ADR-0003 §4.
-  `send` function L118-121 — `(&self, req: reqwest::RequestBuilder) -> Result<Response, CliError>` — rule from ADR-0003 §4.
-  `parse_response` function L123-130 — `(response: Response) -> Result<T, CliError>` — rule from ADR-0003 §4.

#### crates/cloacinactl/src/shared/client_ctx.rs

- pub `ClientContext` struct L29-35 — `{ server: String, api_key: String, tenant: Option<String>, output: OutputFormat,...` — Resolved client context — everything a client command needs to talk to the
- pub `resolve` function L40-78 — `(opts: &GlobalOpts, config: &CloacinaConfig) -> Result<Self>` — Resolve against the precedence rule from ADR-0003 §3:
- pub `resolve_api_key_scheme` function L82-97 — `(raw: &str) -> Result<String>` — Resolve an api-key value that may carry a scheme prefix.
-  `ClientContext` type L37-79 — `= ClientContext` — that client-side commands use to hit the server.
-  `read_key_file` function L99-108 — `(path: &Path) -> Result<String>` — that client-side commands use to hit the server.
-  `tests` module L111-203 — `-` — that client-side commands use to hit the server.
-  `opts` function L116-130 — `(overrides: impl FnOnce(&mut GlobalOpts)) -> GlobalOpts` — that client-side commands use to hit the server.
-  `explicit_flag_wins` function L133-150 — `()` — that client-side commands use to hit the server.
-  `named_profile_wins_over_default` function L153-173 — `()` — that client-side commands use to hit the server.
-  `no_config_errors` function L176-180 — `()` — that client-side commands use to hit the server.
-  `env_scheme` function L183-188 — `()` — that client-side commands use to hit the server.
-  `file_scheme` function L191-196 — `()` — that client-side commands use to hit the server.
-  `keyring_scheme_deferred` function L199-202 — `()` — that client-side commands use to hit the server.

#### crates/cloacinactl/src/shared/error.rs

- pub `CliError` enum L23-41 — `UserError | Network | NotFound | Auth | ServerReject | Io | Other` — Typed CLI errors.
- pub `exit_code` function L45-55 — `(&self) -> i32` — Exit code for this error, per ADR-0003 §6.
- pub `from_reqwest` function L58-60 — `(err: reqwest::Error) -> Self` — Build a `CliError` from a reqwest error.
- pub `from_status` function L63-73 — `(status: u16, body: serde_json::Value) -> Self` — Build a `CliError` from an HTTP response status + body.
-  `CliError` type L43-74 — `= CliError` — Error types and exit-code mapping per ADR-0003.
-  `extract_message` function L76-87 — `(body: &serde_json::Value) -> String` — Error types and exit-code mapping per ADR-0003.
-  `CliError` type L89-109 — `= CliError` — Error types and exit-code mapping per ADR-0003.
-  `fmt` function L90-108 — `(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result` — Error types and exit-code mapping per ADR-0003.
-  `CliError` type L111 — `= CliError` — Error types and exit-code mapping per ADR-0003.
-  `CliError` type L113-117 — `= CliError` — Error types and exit-code mapping per ADR-0003.
-  `from` function L114-116 — `(e: std::io::Error) -> Self` — Error types and exit-code mapping per ADR-0003.
-  `CliError` type L119-123 — `= CliError` — Error types and exit-code mapping per ADR-0003.
-  `from` function L120-122 — `(e: reqwest::Error) -> Self` — Error types and exit-code mapping per ADR-0003.
-  `CliError` type L125-129 — `= CliError` — Error types and exit-code mapping per ADR-0003.
-  `from` function L126-128 — `(e: anyhow::Error) -> Self` — Error types and exit-code mapping per ADR-0003.
-  `tests` module L132-184 — `-` — Error types and exit-code mapping per ADR-0003.
-  `exit_codes_match_adr` function L136-156 — `()` — Error types and exit-code mapping per ADR-0003.
-  `from_status_maps_correctly` function L159-176 — `()` — Error types and exit-code mapping per ADR-0003.
-  `message_extraction_prefers_structured_error` function L179-183 — `()` — Error types and exit-code mapping per ADR-0003.

#### crates/cloacinactl/src/shared/mod.rs

- pub `client` module L20 — `-` — Helpers shared across nouns: PID-file management, Unix socket client,
- pub `client_ctx` module L21 — `-` — exec helpers, etc.
- pub `error` module L22 — `-` — exec helpers, etc.
- pub `output` module L23 — `-` — exec helpers, etc.
- pub `pid` module L24 — `-` — exec helpers, etc.
- pub `render` module L25 — `-` — exec helpers, etc.

#### crates/cloacinactl/src/shared/output.rs

- pub `Renderable` interface L27-30 — `{ fn render() }` — Something the CLI can render in any supported `OutputFormat`.
- pub `emit` function L34-39 — `(value: &T, format: OutputFormat) -> io::Result<()>` — Convenience: render any serializable + table-renderable type using `format`,
- pub `render_serialized` function L45-68 — `( value: &T, format: OutputFormat, out: &mut dyn Write, ) -> io::Result<()>` — Generic serde-based rendering for `Json` and `Yaml` formats.
- pub `Redacted` struct L75 — `-` — A string redacted to its first/last 4 chars for human display.
- pub `short` function L78-87 — `(&self) -> String` — secrets.
- pub `raw` function L89-91 — `(&self) -> &str` — secrets.
-  `Redacted` type L77-92 — `= Redacted` — secrets.
-  `Redacted` type L94-98 — `= Redacted` — secrets.
-  `fmt` function L95-97 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — secrets.
-  `Redacted` type L100-106 — `impl Serialize for Redacted` — secrets.
-  `serialize` function L101-105 — `(&self, s: S) -> Result<S::Ok, S::Error>` — secrets.
-  `tests` module L109-129 — `-` — secrets.
-  `redacted_short_form` function L113-121 — `()` — secrets.
-  `redacted_json_is_raw` function L124-128 — `()` — secrets.

#### crates/cloacinactl/src/shared/pid.rs

- pub `write` function L26-34 — `(path: &Path) -> Result<()>` — Write the current process PID into `path`, creating the parent directory
- pub `read` function L37-43 — `(path: &Path) -> Result<u32>` — Read a PID from `path`.
- pub `try_read` function L46-48 — `(path: &Path) -> Option<u32>` — Non-erroring variant — `None` when the file is absent or unreadable.
- pub `remove` function L51-60 — `(path: &Path) -> Result<()>` — Remove the PID file, ignoring "not found" errors.
- pub `signal_and_wait` function L64-97 — `(pid: u32, force: bool, timeout: Duration) -> Result<()>` — Send SIGTERM (or SIGKILL if `force`) to `pid` and wait up to `timeout` for
-  `libc_signal` module L99-101 — `-` — PID-file read/write/signal helpers used by `daemon stop` and `server stop`.

#### crates/cloacinactl/src/shared/render.rs

- pub `list` function L26-56 — `(body: &Value, format: OutputFormat) -> Result<(), CliError>` — catalog-style listings; can be replaced with per-type renderers later.
- pub `object` function L58-98 — `(body: &Value, format: OutputFormat) -> Result<(), CliError>` — catalog-style listings; can be replaced with per-type renderers later.
-  `table` function L100-130 — `(items: &[Value]) -> Result<(), CliError>` — catalog-style listings; can be replaced with per-type renderers later.
-  `truncate` function L132-138 — `(s: &str, max: usize) -> String` — catalog-style listings; can be replaced with per-type renderers later.

### docs/themes/hugo-geekdoc/static/js

> *Semantic summary to be generated by AI agent.*

#### docs/themes/hugo-geekdoc/static/js/130-3b252fb9.chunk.min.js

- pub `_getExpansion` method L1 — `_getExpansion(e)`
- pub `baseSizingClasses` method L1 — `baseSizingClasses()`
- pub `beginGroup` method L1 — `beginGroup()`
- pub `callFunction` method L1 — `callFunction(e,t,r,a,n)`
- pub `constructor` method L1 — `constructor(e,t,r)`
- pub `consume` method L1 — `consume()`
- pub `consumeArg` method L1 — `consumeArg(e)`
- pub `consumeArgs` method L1 — `consumeArgs(e,t)`
- pub `consumeSpaces` method L1 — `consumeSpaces()`
- pub `countExpansion` method L1 — `countExpansion(e)`
- pub `cramp` method L1 — `cramp()`
- pub `endGroup` method L1 — `endGroup()`
- pub `endGroups` method L1 — `endGroups()`
- pub `expandAfterFuture` method L1 — `expandAfterFuture()`
- pub `expandMacro` method L1 — `expandMacro(e)`
- pub `expandMacroAsText` method L1 — `expandMacroAsText(e)`
- pub `expandNextToken` method L1 — `expandNextToken()`
- pub `expandOnce` method L1 — `expandOnce(e)`
- pub `expandTokens` method L1 — `expandTokens(e)`
- pub `expect` method L1 — `expect(e,t)`
- pub `extend` method L1 — `extend(e)`
- pub `feed` method L1 — `feed(e)`
- pub `fetch` method L1 — `fetch()`
- pub `fontMetrics` method L1 — `fontMetrics()`
- pub `formLigatures` method L1 — `formLigatures(e)`
- pub `formatUnsupportedCmd` method L1 — `formatUnsupportedCmd(e)`
- pub `fracDen` method L1 — `fracDen()`
- pub `fracNum` method L1 — `fracNum()`
- pub `future` method L1 — `future()`
- pub `get` method L1 — `get(e)`
- pub `getAttribute` method L1 — `getAttribute(e)`
- pub `getColor` method L1 — `getColor()`
- pub `handleInfixNodes` method L1 — `handleInfixNodes(e)`
- pub `handleSupSubscript` method L1 — `handleSupSubscript(e)`
- pub `has` method L1 — `has(e)`
- pub `hasClass` method L1 — `hasClass(e)`
- pub `havingBaseSizing` method L1 — `havingBaseSizing()`
- pub `havingBaseStyle` method L1 — `havingBaseStyle(e)`
- pub `havingCrampedStyle` method L1 — `havingCrampedStyle()`
- pub `havingSize` method L1 — `havingSize(e)`
- pub `havingStyle` method L1 — `havingStyle(e)`
- pub `isDefined` method L1 — `isDefined(e)`
- pub `isExpandable` method L1 — `isExpandable(e)`
- pub `isTight` method L1 — `isTight()`
- pub `isTrusted` method L1 — `isTrusted(e)`
- pub `lex` method L1 — `lex()`
- pub `parse` method L1 — `parse()`
- pub `parseArgumentGroup` method L1 — `parseArgumentGroup(e,t)`
- pub `parseArguments` method L1 — `parseArguments(e,t)`
- pub `parseAtom` method L1 — `parseAtom(e)`
- pub `parseColorGroup` method L1 — `parseColorGroup(e)`
- pub `parseExpression` method L1 — `parseExpression(e,t)`
- pub `parseFunction` method L1 — `parseFunction(e,t)`
- pub `parseGroup` method L1 — `parseGroup(e,t)`
- pub `parseGroupOfType` method L1 — `parseGroupOfType(e,t,r)`
- pub `parseRegexGroup` method L1 — `parseRegexGroup(e,t)`
- pub `parseSizeGroup` method L1 — `parseSizeGroup(e)`
- pub `parseStringGroup` method L1 — `parseStringGroup(e,t)`
- pub `parseSymbol` method L1 — `parseSymbol()`
- pub `parseUrlGroup` method L1 — `parseUrlGroup(e)`
- pub `popToken` method L1 — `popToken()`
- pub `pushToken` method L1 — `pushToken(e)`
- pub `pushTokens` method L1 — `pushTokens(e)`
- pub `range` method L1 — `range(e,t)`
- pub `reportNonstrict` method L1 — `reportNonstrict(e,t,r)`
- pub `scanArgument` method L1 — `scanArgument(e)`
- pub `set` method L1 — `set(e,t,r)`
- pub `setAttribute` method L1 — `setAttribute(e,t)`
- pub `setCatcode` method L1 — `setCatcode(e,t)`
- pub `sizingClasses` method L1 — `sizingClasses(e)`
- pub `sub` method L1 — `sub()`
- pub `subparse` method L1 — `subparse(e)`
- pub `sup` method L1 — `sup()`
- pub `switchMode` method L1 — `switchMode(e)`
- pub `text` method L1 — `text()`
- pub `toMarkup` method L1 — `toMarkup()`
- pub `toNode` method L1 — `toNode()`
- pub `toText` method L1 — `toText()`
- pub `useStrictBehavior` method L1 — `useStrictBehavior(e,t,r)`
- pub `withColor` method L1 — `withColor(e)`
- pub `withFont` method L1 — `withFont(e)`
- pub `withPhantom` method L1 — `withPhantom()`
- pub `withTextFontFamily` method L1 — `withTextFontFamily(e)`
- pub `withTextFontShape` method L1 — `withTextFontShape(e)`
- pub `withTextFontWeight` method L1 — `withTextFontWeight(e)`
-  `At` function L1 — `function At(e,t)`
-  `Bt` class L1 — `-`
-  `C` function L1 — `function C(e)`
-  `Dt` function L1 — `function Dt(e,t,r,a,n)`
-  `Ea` class L1 — `-`
-  `Er` function L1 — `function Er(e)`
-  `G` class L1 — `-`
-  `Gr` function L1 — `function Gr(e,t)`
-  `Ht` function L1 — `function Ht(e)`
-  `L` function L1 — `function L(e,t,r)`
-  `Nt` class L1 — `-`
-  `Oa` class L1 — `-`
-  `Or` function L1 — `function Or(e,t)`
-  `Pr` function L1 — `function Pr(e)`
-  `Q` class L1 — `-`
-  `Qt` function L1 — `function Qt(e,t)`
-  `R` class L1 — `-`
-  `Tt` function L1 — `function Tt(e)`
-  `Ur` function L1 — `function Ur(e)`
-  `Wr` function L1 — `function Wr(e,t,r)`
-  `Wt` function L1 — `function Wt(e)`
-  `Xa` class L1 — `-`
-  `Xr` function L1 — `function Xr(e)`
-  `Xt` function L1 — `function Xt(e,t)`
-  `Za` class L1 — `-`
-  `_r` function L1 — `function _r(e)`
-  `_t` function L1 — `function _t(e)`
-  `a` class L1 — `-`
-  `ae` class L1 — `-`
-  `b` function L1 — `function b(e)`
-  `ce` function L1 — `function ce(e,t,r,a,n,i)`
-  `ee` class L1 — `-`
-  `er` function L1 — `function er(e,t)`
-  `ht` function L1 — `function ht(e)`
-  `i` class L1 — `-`
-  `ie` class L1 — `-`
-  `k` function L1 — `function k()`
-  `mt` function L1 — `function mt(e)`
-  `n` class L1 — `-`
-  `ne` class L1 — `-`
-  `nr` function L1 — `function nr(e,t,r)`
-  `oe` class L1 — `-`
-  `se` function L1 — `function se(e)`
-  `te` class L1 — `-`
-  `va` function L1 — `function va(e,t,r)`
-  `w` function L1 — `function w()`
-  `x` class L1 — `-`
-  `x` function L1 — `function x(e)`
-  `y` class L1 — `-`
-  `zt` function L1 — `function zt(e,t)`

#### docs/themes/hugo-geekdoc/static/js/164-f339d58d.chunk.min.js

-  `o` function L1 — `function o(t)`

#### docs/themes/hugo-geekdoc/static/js/165-d20df99c.chunk.min.js

-  `$c` function L2 — `function $c(e,t,n)`
-  `Ac` function L2 — `function Ac(e,t,n)`
-  `Ai` function L2 — `function Ai()`
-  `Ao` function L2 — `function Ao(e,t,n)`
-  `As` function L2 — `function As(e,t)`
-  `Be` function L2 — `function Be()`
-  `Bi` function L2 — `function Bi()`
-  `Bo` function L2 — `function Bo(e,t,n,r)`
-  `Ce` function L2 — `function Ce()`
-  `Ci` function L2 — `function Ci()`
-  `Dc` function L2 — `function Dc(e,t)`
-  `De` function L2 — `function De()`
-  `Di` function L2 — `function Di()`
-  `Do` function L2 — `function Do(e,t,n)`
-  `E` function L2 — `function E()`
-  `Ed` function L2 — `function Ed(e,t,n)`
-  `Ei` function L2 — `function Ei()`
-  `Gs` function L2 — `function Gs(e)`
-  `Gu` function L2 — `function Gu(e,t)`
-  `Hl` function L2 — `function Hl(e)`
-  `Hs` function L2 — `function Hs(e)`
-  `Is` function L2 — `function Is(e,t)`
-  `Jc` function L2 — `function Jc(e,t,n)`
-  `Ki` function L2 — `function Ki()`
-  `Kl` function L2 — `function Kl(e)`
-  `Ld` function L2 — `function Ld(e)`
-  `Lo` function L2 — `function Lo(e)`
-  `Mc` function L2 — `function Mc(e,t,n,r,a)`
-  `Md` function L2 — `function Md(e)`
-  `Mi` function L2 — `function Mi()`
-  `Ms` function L2 — `function Ms(e,t)`
-  `Ns` function L2 — `function Ns(e)`
-  `P` function L2 — `function P(e,n,r,a,i)`
-  `Pe` function L2 — `function Pe()`
-  `Pi` function L2 — `function Pi()`
-  `Qc` function L2 — `function Qc(e)`
-  `Qn` function L2 — `function Qn(e,t,n,r,a,i)`
-  `Qu` function L2 — `function Qu(e)`
-  `Rc` function L2 — `function Rc(e,t,n,r)`
-  `Rd` function L2 — `function Rd(e,t,n)`
-  `S` function L2 — `function S(e,n)`
-  `Se` function L2 — `function Se()`
-  `Si` function L2 — `function Si()`
-  `Te` function L2 — `function Te(e)`
-  `Ti` function L2 — `function Ti()`
-  `Wc` function L2 — `function Wc(e,t,n,r,a)`
-  `Xd` function L2 — `function Xd(e,t)`
-  `Xs` function L2 — `function Xs(e)`
-  `Yd` function L2 — `function Yd(e,t,n,r,a)`
-  `Yl` function L2 — `function Yl(e)`
-  `Ys` function L2 — `function Ys(e)`
-  `Zc` function L2 — `function Zc(e,t,n)`
-  `Zs` function L2 — `function Zs(e)`
-  `Zu` function L2 — `function Zu(e,t,n,r)`
-  `_c` function L2 — `function _c(e,t,n)`
-  `_i` function L2 — `function _i()`
-  `_o` function L2 — `function _o(e,t,n)`
-  `_s` function L2 — `function _s(e,t,n)`
-  `a` function L2 — `function a(e,t)`
-  `ad` function L2 — `function ad(e,t,n,r)`
-  `al` function L2 — `function al(e,t,n,r,a)`
-  `b` function L2 — `function b(e)`
-  `bi` function L2 — `function bi()`
-  `bu` function L2 — `function bu(e)`
-  `c` function L2 — `function c(e)`
-  `cd` function L2 — `function cd(e,t,n)`
-  `cl` function L2 — `function cl(e,t)`
-  `cs` function L2 — `function cs()`
-  `d` function L2 — `function d(e)`
-  `ds` function L2 — `function ds()`
-  `e` function L2 — `function e(e)`
-  `ed` function L2 — `function ed(e,t)`
-  `el` function L2 — `function el(e,t,n,r)`
-  `gu` function L2 — `function gu(e)`
-  `h` function L2 — `function h(e,t)`
-  `i` function L2 — `function i(e,t,n)`
-  `il` function L2 — `function il(e,t)`
-  `jd` function L2 — `function jd(e,t,n)`
-  `jl` function L2 — `function jl(e)`
-  `kd` function L2 — `function kd(e,t,n)`
-  `ki` function L2 — `function ki()`
-  `ku` function L2 — `function ku(e)`
-  `l` function L2 — `function l(e,t)`
-  `ld` function L2 — `function ld(e,t,n)`
-  `ll` function L2 — `function ll(e,t)`
-  `m` function L2 — `function m(e)`
-  `md` function L2 — `function md(e,t)`
-  `n` function L2 — `function n(e)`
-  `nd` function L2 — `function nd(e,t,n)`
-  `o` function L2 — `function o(e,t)`
-  `od` function L2 — `function od()`
-  `ol` function L2 — `function ol(e,t,n,r,a)`
-  `qd` function L2 — `function qd(e,t,n,r)`
-  `r` function L2 — `function r(e,t)`
-  `rd` function L2 — `function rd(e,t,n,r,a,i)`
-  `s` function L2 — `function s(e,t,n)`
-  `sd` function L2 — `function sd(e)`
-  `sl` function L2 — `function sl(e,t,n,r)`
-  `t` function L2 — `function t(n,r)`
-  `tc` function L2 — `function tc(e,t,n)`
-  `td` function L2 — `function td(e,t)`
-  `u` function L2 — `function u(e)`
-  `ud` function L2 — `function ud(e,t,n)`
-  `ul` function L2 — `function ul(e,t,n,r)`
-  `v` function L2 — `function v(e,t)`
-  `vs` function L2 — `function vs()`
-  `w` function L2 — `function w(e)`
-  `wd` function L2 — `function wd(e,t)`
-  `wi` function L2 — `function wi()`
-  `wu` function L2 — `function wu(e)`
-  `x` function L2 — `function x()`
-  `xi` function L2 — `function xi()`
-  `y` function L2 — `function y(e,t,n)`
-  `yu` function L2 — `function yu(e)`
-  `zo` function L2 — `function zo(e,t)`

#### docs/themes/hugo-geekdoc/static/js/248-d3b4979c.chunk.min.js

-  `$` function L1 — `function $()`
-  `B` function L1 — `function B(t)`
-  `E` function L1 — `function E()`
-  `F` function L1 — `function F(t,i)`
-  `G` function L1 — `function G(t,i)`
-  `H` function L1 — `function H(t)`
-  `I` function L1 — `function I()`
-  `M` function L1 — `function M()`
-  `N` function L1 — `function N(t)`
-  `O` function L1 — `function O(t)`
-  `Q` function L1 — `function Q()`
-  `U` function L1 — `function U(t)`
-  `V` function L1 — `function V(t)`
-  `W` function L1 — `function W(t)`
-  `X` function L1 — `function X(t,i)`
-  `Y` function L1 — `function Y(t)`
-  `Z` function L1 — `function Z()`
-  `b` function L1 — `function b(t,i,e,s)`
-  `c` function L1 — `function c(t)`
-  `g` function L1 — `function g(t)`
-  `j` function L1 — `function j(t,i)`
-  `l` function L1 — `function l(t)`
-  `m` function L1 — `function m()`
-  `q` function L1 — `function q()`
-  `u` function L1 — `function u(t)`
-  `w` function L1 — `function w(t,i,e)`
-  `y` function L1 — `function y(t,i,e,s)`
-  `z` function L1 — `function z(t)`

#### docs/themes/hugo-geekdoc/static/js/295-8a201dad.chunk.min.js

-  `a` function L1 — `function a(t)`
-  `c` function L1 — `function c(t)`
-  `e` function L1 — `function e(t,e,n,i,r,a,o,c,l)`
-  `j` function L1 — `function j(t,e)`
-  `n` function L1 — `function n(t,e,n,i,s)`
-  `o` function L1 — `function o(t)`
-  `s` function L1 — `function s(t,e)`
-  `t` function L1 — `function t(t,e,n,i,r,a,o,c)`
-  `u` function L1 — `function u()`
-  `x` function L1 — `function x()`

#### docs/themes/hugo-geekdoc/static/js/297-baccf39c.chunk.min.js

-  `m` function L1 — `function m()`
-  `ut` function L1 — `function ut()`

#### docs/themes/hugo-geekdoc/static/js/343-07706d94.chunk.min.js

-  `k` function L1 — `function k()`
-  `ne` function L1 — `function ne()`
-  `r` function L1 — `const r = (t,e)`

#### docs/themes/hugo-geekdoc/static/js/370-0e626739.chunk.min.js

-  `$` function L1 — `function $(t)`
-  `D` function L1 — `function D(t,e,n,s)`
-  `K` function L1 — `function K()`
-  `Kt` function L1 — `function Kt(t,e,n)`
-  `T` function L1 — `function T(t,e,n)`
-  `_` function L1 — `function _(t,e)`
-  `b` function L1 — `function b(t,e)`
-  `f` function L1 — `function f(n)`
-  `g` function L1 — `function g()`
-  `v` function L1 — `function v(t,e,n,o,c,l,d,u)`
-  `w` function L1 — `function w(t,e,n,s)`
-  `x` function L1 — `function x(t,n,a,o,c,l,u)`

#### docs/themes/hugo-geekdoc/static/js/388-0f08b415.chunk.min.js

-  `F` function L1 — `function F(t,e,i,n)`
-  `P` function L1 — `function P(t,e)`
-  `R` function L1 — `function R(t,e,i,n,r)`
-  `S` function L1 — `function S(t,e,i,n,r)`
-  `U` function L1 — `function U(t,e)`
-  `_` function L1 — `function _()`
-  `b` function L1 — `function b(t,e)`
-  `g` function L1 — `function g(t)`
-  `h` function L1 — `function h()`
-  `i` function L1 — `function i(n)`
-  `l` function L1 — `function l(t,e,i,s)`
-  `n` function L1 — `function n()`
-  `o` function L1 — `function o(t)`
-  `r` function L1 — `function r()`
-  `s` function L1 — `function s(t,e,i)`
-  `t` function L1 — `function t(t,e)`
-  `u` function L1 — `function u(t,e,i)`
-  `v` function L1 — `function v()`

#### docs/themes/hugo-geekdoc/static/js/391-a0aaa95e.chunk.min.js

- pub `_removeFromParentsChildList` method L1 — `_removeFromParentsChildList(t)`
- pub `children` method L1 — `children(t)`
- pub `constructor` method L1 — `constructor(t={})`
- pub `edge` method L1 — `edge(t,e,r)`
- pub `edgeCount` method L1 — `edgeCount()`
- pub `edges` method L1 — `edges()`
- pub `filterNodes` method L1 — `filterNodes(t)`
- pub `graph` method L1 — `graph()`
- pub `hasEdge` method L1 — `hasEdge(t,e,r)`
- pub `hasNode` method L1 — `hasNode(t)`
- pub `inEdges` method L1 — `inEdges(t,e)`
- pub `isCompound` method L1 — `isCompound()`
- pub `isDirected` method L1 — `isDirected()`
- pub `isLeaf` method L1 — `isLeaf(t)`
- pub `isMultigraph` method L1 — `isMultigraph()`
- pub `neighbors` method L1 — `neighbors(t)`
- pub `node` method L1 — `node(t)`
- pub `nodeCount` method L1 — `nodeCount()`
- pub `nodeEdges` method L1 — `nodeEdges(t,e)`
- pub `nodes` method L1 — `nodes()`
- pub `outEdges` method L1 — `outEdges(t,e)`
- pub `parent` method L1 — `parent(t)`
- pub `predecessors` method L1 — `predecessors(t)`
- pub `removeEdge` method L1 — `removeEdge(t,e,r)`
- pub `removeNode` method L1 — `removeNode(t)`
- pub `setDefaultEdgeLabel` method L1 — `setDefaultEdgeLabel(t)`
- pub `setDefaultNodeLabel` method L1 — `setDefaultNodeLabel(t)`
- pub `setEdge` method L1 — `setEdge()`
- pub `setGraph` method L1 — `setGraph(t)`
- pub `setNode` method L1 — `setNode(t,e)`
- pub `setNodes` method L1 — `setNodes(t,e)`
- pub `setParent` method L1 — `setParent(t,e)`
- pub `setPath` method L1 — `setPath(t,e)`
- pub `sinks` method L1 — `sinks()`
- pub `sources` method L1 — `sources()`
- pub `successors` method L1 — `successors(t)`
-  `At` function L1 — `function At(t,e,r,s)`
-  `Dt` function L1 — `function Dt(t,e)`
-  `Et` function L1 — `function Et(t,e,r,s)`
-  `J` function L1 — `function J(t,e)`
-  `K` function L1 — `function K(t)`
-  `Kt` function L1 — `function Kt(t,e,r)`
-  `L` function L1 — `function L(t,e)`
-  `Lt` function L1 — `function Lt(t,e,r,s)`
-  `N` function L1 — `function N(t)`
-  `Q` function L1 — `function Q(t,e,r=0,s=0)`
-  `St` function L1 — `function St(t,e,r)`
-  `T` function L1 — `function T(t)`
-  `Vt` function L1 — `function Vt(t,e,r,s)`
-  `_` function L1 — `function _(t,e,r,s)`
-  `a` function L1 — `function a(t)`
-  `at` function L1 — `function at(t)`
-  `be` function L1 — `function be(t,e,r,s,a)`
-  `de` function L1 — `function de(t,e,r=!1)`
-  `et` function L1 — `function et(t,{minX:e,minY:r,maxX:s,maxY:a}={minX:0,minY:0,maxX:0,maxY:0})`
-  `f` class L1 — `-`
-  `f` function L1 — `function f()`
-  `ge` function L1 — `function ge(t,e,r)`
-  `gt` function L1 — `function gt(t,e)`
-  `he` function L1 — `function he(t,e,r)`
-  `i` function L1 — `const i = (t,e)`
-  `m` function L1 — `function m(t,e)`
-  `pe` function L1 — `function pe(t,e,r)`
-  `rt` function L1 — `function rt(t)`
-  `s` function L1 — `function s()`
-  `st` function L1 — `function st(t,e)`
-  `tt` function L1 — `function tt(t,e)`
-  `ue` function L1 — `function ue(t,e,r,s)`
-  `w` function L1 — `function w(t,e)`
-  `wt` function L1 — `function wt(t,e)`
-  `ye` function L1 — `function ye(t,e,r)`

#### docs/themes/hugo-geekdoc/static/js/420-35785222.chunk.min.js

-  `I` function L1 — `function I(t,e)`
-  `Q` function L1 — `function Q(t,e)`
-  `a` function L1 — `function a(t,e,a,s,r)`
-  `b` function L1 — `function b()`
-  `c` function L1 — `function c(a,s)`
-  `ct` function L1 — `function ct(t,e,a,s,r,i,o)`
-  `dt` function L1 — `function dt(t,e,a)`
-  `e` function L1 — `function e(t,e,a,s,o,c,l,d)`
-  `l` function L1 — `function l(a,s)`
-  `o` function L1 — `function o(o)`
-  `ot` function L1 — `function ot(t,e,a,s,r)`
-  `pt` function L1 — `function pt(t,e,a)`
-  `r` function L1 — `function r(t,e)`
-  `s` function L1 — `function s(t,a,s,i,o,c,l,d)`
-  `t` function L1 — `function t(t,e,a,s,i,n,o)`
-  `z` function L1 — `function z()`

#### docs/themes/hugo-geekdoc/static/js/428-1733cd76.chunk.min.js

-  `B` function L1 — `function B(t="",e=0,s="",i=L)`
-  `G` function L1 — `function G(t)`
-  `J` function L1 — `function J()`
-  `O` function L1 — `function O()`
-  `P` function L1 — `function P(t,e,s)`
-  `f` function L1 — `function f()`
-  `j` function L1 — `function j(t)`

#### docs/themes/hugo-geekdoc/static/js/440-00a1e1fb.chunk.min.js

-  `f` function L1 — `function f()`
-  `ue` function L1 — `function ue()`

#### docs/themes/hugo-geekdoc/static/js/475-5c92875f.chunk.min.js

-  `f` function L1 — `function f(e)`
-  `h` function L1 — `function h(e)`
-  `l` function L1 — `function l(e)`

#### docs/themes/hugo-geekdoc/static/js/567-6c3220fd.chunk.min.js

- pub `_removeFromParentsChildList` method L1 — `_removeFromParentsChildList(e)`
- pub `children` method L1 — `children(e)`
- pub `constructor` method L1 — `constructor()`
- pub `dequeue` method L1 — `dequeue()`
- pub `edge` method L1 — `edge(e,n,t)`
- pub `edgeCount` method L1 — `edgeCount()`
- pub `edges` method L1 — `edges()`
- pub `enqueue` method L1 — `enqueue(e)`
- pub `filterNodes` method L1 — `filterNodes(e)`
- pub `graph` method L1 — `graph()`
- pub `hasEdge` method L1 — `hasEdge(e,n,t)`
- pub `hasNode` method L1 — `hasNode(e)`
- pub `inEdges` method L1 — `inEdges(e,n)`
- pub `isCompound` method L1 — `isCompound()`
- pub `isDirected` method L1 — `isDirected()`
- pub `isLeaf` method L1 — `isLeaf(e)`
- pub `isMultigraph` method L1 — `isMultigraph()`
- pub `neighbors` method L1 — `neighbors(e)`
- pub `node` method L1 — `node(e)`
- pub `nodeCount` method L1 — `nodeCount()`
- pub `nodeEdges` method L1 — `nodeEdges(e,n)`
- pub `nodes` method L1 — `nodes()`
- pub `outEdges` method L1 — `outEdges(e,n)`
- pub `parent` method L1 — `parent(e)`
- pub `predecessors` method L1 — `predecessors(e)`
- pub `removeEdge` method L1 — `removeEdge(e,n,t)`
- pub `removeNode` method L1 — `removeNode(e)`
- pub `setDefaultEdgeLabel` method L1 — `setDefaultEdgeLabel(e)`
- pub `setDefaultNodeLabel` method L1 — `setDefaultNodeLabel(e)`
- pub `setEdge` method L1 — `setEdge()`
- pub `setGraph` method L1 — `setGraph(e)`
- pub `setNode` method L1 — `setNode(e,n)`
- pub `setNodes` method L1 — `setNodes(e,n)`
- pub `setParent` method L1 — `setParent(e,n)`
- pub `setPath` method L1 — `setPath(e,n)`
- pub `sinks` method L1 — `sinks()`
- pub `sources` method L1 — `sources()`
- pub `successors` method L1 — `successors(e)`
- pub `toString` method L1 — `toString()`
-  `$` function L1 — `function $(e,n,t,r)`
-  `A` function L1 — `function A(e)`
-  `An` function L1 — `function An(e,n,t)`
-  `Be` function L1 — `function Be(e)`
-  `Ce` function L1 — `function Ce(e,n,t)`
-  `De` function L1 — `function De(e)`
-  `Fe` function L1 — `function Fe(e,n,t)`
-  `H` function L1 — `function H(e)`
-  `Ie` function L1 — `function Ie(e,n)`
-  `J` function L1 — `function J(e)`
-  `K` function L1 — `function K(e,n,t,r)`
-  `Le` function L1 — `function Le(e,n,t,o,i)`
-  `Me` function L1 — `function Me(e,n,t)`
-  `Ne` function L1 — `function Ne(e,n,t,o,i,u)`
-  `Oe` function L1 — `function Oe(e,n,t)`
-  `Pe` function L1 — `function Pe(e)`
-  `Pn` function L1 — `function Pn(e,n)`
-  `Q` function L1 — `function Q(e)`
-  `Re` function L1 — `function Re(e,n,t,o)`
-  `Te` function L1 — `function Te(e)`
-  `U` function L1 — `function U(e,n)`
-  `W` function L1 — `function W(e,n)`
-  `X` function L1 — `function X(e,n,t,r,o,i)`
-  `Z` function L1 — `function Z(e,n)`
-  `_` function L1 — `function _(e)`
-  `ae` function L1 — `function ae(e,n)`
-  `an` function L1 — `function an(e,n,t,o)`
-  `b` function L1 — `function b(e,n)`
-  `bn` function L1 — `function bn(e,n)`
-  `ce` function L1 — `function ce(e,n)`
-  `cn` function L1 — `function cn(e,n)`
-  `d` function L1 — `function d(e,n)`
-  `de` function L1 — `function de(e,n)`
-  `dn` function L1 — `function dn(e,n)`
-  `ee` function L1 — `function ee(e)`
-  `he` function L1 — `function he(e,n,t)`
-  `je` function L1 — `function je(e,n)`
-  `jn` function L1 — `function jn(e)`
-  `m` function L1 — `function m(e,n,t,o,i)`
-  `ne` function L1 — `function ne(e)`
-  `o` function L1 — `function o(n)`
-  `on` function L1 — `function on(e,n)`
-  `p` class L1 — `-`
-  `pn` function L1 — `function pn(e,n,t)`
-  `qe` function L1 — `function qe(e,n,t,o,i,u,a)`
-  `re` function L1 — `function re(e)`
-  `rn` function L1 — `function rn(e,n,t)`
-  `se` function L1 — `function se(e)`
-  `sn` function L1 — `function sn(e,n,t)`
-  `t` function L1 — `function t(o,i)`
-  `te` function L1 — `function te(e)`
-  `tn` function L1 — `function tn(e,n)`
-  `ue` function L1 — `function ue(e)`
-  `un` function L1 — `function un(e,n,t)`
-  `w` function L1 — `function w(e,n)`
-  `w` class L1 — `-`
-  `wn` function L1 — `function wn(e)`
-  `y` function L1 — `function y(e,n,t)`

#### docs/themes/hugo-geekdoc/static/js/623-da9b1ffc.chunk.min.js

-  `A` function L1 — `function A(t)`
-  `C` function L1 — `function C(t)`
-  `F` function L1 — `function F(t,e)`
-  `L` function L1 — `function L()`
-  `P` function L1 — `function P(t)`
-  `S` function L1 — `function S(t)`
-  `T` function L1 — `function T()`
-  `_` function L1 — `function _(t)`
-  `b` function L1 — `function b(t)`
-  `c` function L1 — `function c(t)`
-  `d` function L1 — `function d(t)`
-  `g` function L1 — `function g(t)`
-  `gt` function L1 — `function gt()`
-  `k` function L1 — `function k(t,e,i,a,n)`
-  `l` function L1 — `function l(t)`
-  `m` function L1 — `function m(t)`
-  `o` function L1 — `function o(t)`
-  `p` function L1 — `function p(t)`
-  `q` function L1 — `function q(t)`
-  `r` function L1 — `function r(t)`
-  `u` function L1 — `function u(t)`
-  `y` function L1 — `function y(t)`

#### docs/themes/hugo-geekdoc/static/js/687-3d36056d.chunk.min.js

-  `At` function L1 — `function At(t,e,a,n,s)`
-  `Rt` function L1 — `function Rt(t,e,a,n,i)`
-  `a` function L1 — `function a(t,a,i,r,s,l,o,c)`
-  `e` function L1 — `function e(t,e,a,i,s,l,o,c)`
-  `n` function L1 — `function n(t,e)`
-  `st` function L1 — `function st()`
-  `t` function L1 — `function t(t,e,a,i,r,s,l)`
-  `x` function L1 — `function x()`

#### docs/themes/hugo-geekdoc/static/js/704-ed584c37.chunk.min.js

-  `$` function L1 — `function $(n,e)`
-  `A` function L1 — `function A(t)`
-  `C` function L1 — `function C(t)`
-  `D` function L1 — `function D(t)`
-  `I` function L1 — `function I(t)`
-  `L` function L1 — `function L()`
-  `M` function L1 — `function M(t)`
-  `N` function L1 — `function N(t,n,e,i,s)`
-  `O` function L1 — `function O()`
-  `P` function L1 — `function P(t)`
-  `S` function L1 — `function S()`
-  `T` function L1 — `function T(t)`
-  `_` function L1 — `function _(t)`
-  `a` function L1 — `function a(t,n)`
-  `c` function L1 — `function c(t,n)`
-  `d` function L1 — `function d(t)`
-  `f` function L1 — `function f(t,n)`
-  `g` function L1 — `function g(t)`
-  `h` function L1 — `function h(t)`
-  `k` function L1 — `function k(t,n)`
-  `l` function L1 — `function l(t,n)`
-  `o` function L1 — `function o(t)`
-  `p` function L1 — `function p(t)`
-  `r` function L1 — `function r(t,n)`
-  `u` function L1 — `function u(t,n)`
-  `x` function L1 — `function x({nodes:t})`
-  `y` function L1 — `function y(t,n)`

#### docs/themes/hugo-geekdoc/static/js/719-e4d0dfca.chunk.min.js

-  `X` function L1 — `function X()`
-  `m` function L1 — `function m()`
-  `r` function L1 — `const r = (t,e)`

#### docs/themes/hugo-geekdoc/static/js/731-7d3aeec3.chunk.min.js

- pub `DEFINE_RULE` method L1 — `DEFINE_RULE(e,t)`
- pub `IS_RECORDING` method L1 — `IS_RECORDING()`
- pub `accept` method L1 — `accept(e)`
- pub `action` method L1 — `action(e,t)`
- pub `add` method L1 — `add(e,t=null,n)`
- pub `addAll` method L1 — `addAll(e,t)`
- pub `addAstNodeRegionWithAssignmentsTo` method L1 — `addAstNodeRegionWithAssignmentsTo(e)`
- pub `addDocument` method L1 — `addDocument(e)`
- pub `addEntry` method L1 — `addEntry(e,t)`
- pub `addHiddenToken` method L1 — `addHiddenToken(e,t)`
- pub `addHiddenTokens` method L1 — `addHiddenTokens(e)`
- pub `addParents` method L1 — `addParents(e)`
- pub `addTokenUsingMemberAccess` method L1 — `addTokenUsingMemberAccess(e,t,n)`
- pub `addTokenUsingPush` method L1 — `addTokenUsingPush(e,t,n)`
- pub `after` method L1 — `after(e)`
- pub `all` method L1 — `all()`
- pub `allElements` method L1 — `allElements(e,t)`
- pub `alternative` method L1 — `alternative()`
- pub `alternatives` method L1 — `alternatives(e,t)`
- pub `alts` method L1 — `alts()`
- pub `assertion` method L1 — `assertion()`
- pub `assign` method L1 — `assign(e,t,n,r,i)`
- pub `assignWithoutOverride` method L1 — `assignWithoutOverride(e,t)`
- pub `astNode` method L1 — `astNode()`
- pub `atLeastOne` method L1 — `atLeastOne(e,t)`
- pub `atom` method L1 — `atom()`
- pub `atomEscape` method L1 — `atomEscape()`
- pub `before` method L1 — `before(e)`
- pub `build` method L1 — `build(e,t={},n=yc.XO.None)`
- pub `buildCompositeNode` method L1 — `buildCompositeNode(e)`
- pub `buildDocuments` method L1 — `buildDocuments(e,t,n)`
- pub `buildEarlyExitMessage` method L1 — `buildEarlyExitMessage(e)`
- pub `buildKeywordPattern` method L1 — `buildKeywordPattern(e,t)`
- pub `buildKeywordToken` method L1 — `buildKeywordToken(e,t,n)`
- pub `buildKeywordTokens` method L1 — `buildKeywordTokens(e,t,n)`
- pub `buildLeafNode` method L1 — `buildLeafNode(e,t)`
- pub `buildLookaheadForAlternation` method L1 — `buildLookaheadForAlternation(e)`
- pub `buildLookaheadForOptional` method L1 — `buildLookaheadForOptional(e)`
- pub `buildMismatchTokenMessage` method L1 — `buildMismatchTokenMessage(e)`
- pub `buildNoViableAltMessage` method L1 — `buildNoViableAltMessage(e)`
- pub `buildNotAllInputParsedMessage` method L1 — `buildNotAllInputParsedMessage(e)`
- pub `buildReference` method L1 — `buildReference(e,t,n,i)`
- pub `buildRootNode` method L1 — `buildRootNode(e)`
- pub `buildTerminalToken` method L1 — `buildTerminalToken(e)`
- pub `buildTerminalTokens` method L1 — `buildTerminalTokens(e)`
- pub `buildTokens` method L1 — `buildTokens(e,t)`
- pub `cacheForContext` method L1 — `cacheForContext(e)`
- pub `cancel` method L1 — `cancel()`
- pub `cancelWrite` method L1 — `cancelWrite()`
- pub `characterClass` method L1 — `characterClass()`
- pub `characterClassEscape` method L1 — `characterClassEscape()`
- pub `checkIsTarget` method L1 — `checkIsTarget(e,t,n,r)`
- pub `children` method L1 — `children()`
- pub `chopInput` method L1 — `chopInput(e,t)`
- pub `classAtom` method L1 — `classAtom()`
- pub `classEscape` method L1 — `classEscape()`
- pub `classPatternCharacterAtom` method L1 — `classPatternCharacterAtom()`
- pub `clear` method L1 — `clear()`
- pub `computeExports` method L1 — `computeExports(e,t=yc.XO.None)`
- pub `computeExportsForNode` method L1 — `computeExportsForNode(e,t,n=ke,r=yc.XO.None)`
- pub `computeIsSubtype` method L1 — `computeIsSubtype(e,t)`
- pub `computeLocalScopes` method L1 — `computeLocalScopes(e,t=yc.XO.None)`
- pub `computeNewColumn` method L1 — `computeNewColumn(e,t)`
- pub `concat` method L1 — `concat(e)`
- pub `construct` method L1 — `construct(e)`
- pub `constructor` method L1 — `constructor(e)`
- pub `consume` method L1 — `consume(e,t,n)`
- pub `consumeChar` method L1 — `consumeChar(e)`
- pub `controlEscapeAtom` method L1 — `controlEscapeAtom()`
- pub `controlLetterEscapeAtom` method L1 — `controlLetterEscapeAtom()`
- pub `convert` method L1 — `convert(e,t)`
- pub `count` method L1 — `count()`
- pub `create` method L1 — `create(e,t)`
- pub `createAsync` method L1 — `createAsync(e,t,n)`
- pub `createDehyrationContext` method L1 — `createDehyrationContext(e)`
- pub `createDescription` method L1 — `createDescription(e,t,n=Ee(e))`
- pub `createDescriptions` method L1 — `createDescriptions(e,t=yc.XO.None)`
- pub `createDocument` method L1 — `createDocument(e,t,n)`
- pub `createFullToken` method L1 — `createFullToken(e,t,n,r,i,s,a)`
- pub `createGrammarElementIdMap` method L1 — `createGrammarElementIdMap()`
- pub `createHydrationContext` method L1 — `createHydrationContext(e)`
- pub `createLangiumDocument` method L1 — `createLangiumDocument(e,t,n,r)`
- pub `createLinkingError` method L1 — `createLinkingError(e,t)`
- pub `createOffsetOnlyToken` method L1 — `createOffsetOnlyToken(e,t,n,r)`
- pub `createScope` method L1 — `createScope(e,t,n)`
- pub `createScopeForNodes` method L1 — `createScopeForNodes(e,t,n)`
- pub `createStartOnlyToken` method L1 — `createStartOnlyToken(e,t,n,r,i,s)`
- pub `createTextDocumentGetter` method L1 — `createTextDocumentGetter(e,t)`
- pub `currIdx` method L1 — `currIdx()`
- pub `current` method L1 — `current()`
- pub `decimalEscapeAtom` method L1 — `decimalEscapeAtom()`
- pub `definition` method L1 — `definition()`
- pub `definitionErrors` method L1 — `definitionErrors()`
- pub `dehydrate` method L1 — `dehydrate(e)`
- pub `dehydrateAstNode` method L1 — `dehydrateAstNode(e,t)`
- pub `dehydrateCstNode` method L1 — `dehydrateCstNode(e,t)`
- pub `dehydrateReference` method L1 — `dehydrateReference(e,t)`
- pub `delete` method L1 — `delete(e,t)`
- pub `deleteDocument` method L1 — `deleteDocument(e)`
- pub `deserialize` method L1 — `deserialize(e,t={})`
- pub `disjunction` method L1 — `disjunction()`
- pub `dispose` method L1 — `dispose()`
- pub `distinct` method L1 — `distinct(e)`
- pub `doLink` method L1 — `doLink(e,t)`
- pub `documentationLinkRenderer` method L1 — `documentationLinkRenderer(e,t,n)`
- pub `documentationTagRenderer` method L1 — `documentationTagRenderer(e,t)`
- pub `dotAll` method L1 — `dotAll()`
- pub `element` method L1 — `element()`
- pub `elements` method L1 — `elements()`
- pub `emitUpdate` method L1 — `emitUpdate(e,t)`
- pub `end` method L1 — `end()`
- pub `enqueue` method L1 — `enqueue(e,t,n)`
- pub `ensureBeforeEOL` method L1 — `ensureBeforeEOL(e,t)`
- pub `entries` method L1 — `entries()`
- pub `entriesGroupedByKey` method L1 — `entriesGroupedByKey()`
- pub `event` method L1 — `event()`
- pub `every` method L1 — `every(e)`
- pub `exclude` method L1 — `exclude(e,t)`
- pub `exportNode` method L1 — `exportNode(e,t,n)`
- pub `feature` method L1 — `feature()`
- pub `file` method L1 — `file(t)`
- pub `filter` method L1 — `filter(e)`
- pub `finalize` method L1 — `finalize()`
- pub `find` method L1 — `find(e)`
- pub `findAllReferences` method L1 — `findAllReferences(e,t)`
- pub `findDeclaration` method L1 — `findDeclaration(e)`
- pub `findDeclarationNode` method L1 — `findDeclarationNode(e)`
- pub `findIndex` method L1 — `findIndex(e)`
- pub `findLongerAlt` method L1 — `findLongerAlt(e,t)`
- pub `findNameInGlobalScope` method L1 — `findNameInGlobalScope(e,t)`
- pub `findNameInPrecomputedScopes` method L1 — `findNameInPrecomputedScopes(e,t)`
- pub `findReferences` method L1 — `findReferences(e,t)`
- pub `fire` method L1 — `fire(e)`
- pub `firstNonHiddenNode` method L1 — `firstNonHiddenNode()`
- pub `flat` method L1 — `flat(e)`
- pub `flatMap` method L1 — `flatMap(e)`
- pub `forEach` method L1 — `forEach(e)`
- pub `from` method L1 — `from(e)`
- pub `fromModel` method L1 — `fromModel(e,t)`
- pub `fromString` method L1 — `fromString(e,t,n)`
- pub `fromTextDocument` method L1 — `fromTextDocument(e,t,n)`
- pub `fromUri` method L1 — `fromUri(e,t=yc.XO.None)`
- pub `fsPath` method L1 — `fsPath()`
- pub `fullText` method L1 — `fullText()`
- pub `get` method L1 — `get(e)`
- pub `getAllElements` method L1 — `getAllElements()`
- pub `getAllSubTypes` method L1 — `getAllSubTypes(e)`
- pub `getAllTags` method L1 — `getAllTags()`
- pub `getAllTypes` method L1 — `getAllTypes()`
- pub `getAssignment` method L1 — `getAssignment(e)`
- pub `getAstNode` method L1 — `getAstNode(e,t)`
- pub `getAstNodePath` method L1 — `getAstNodePath(e)`
- pub `getBuildOptions` method L1 — `getBuildOptions(e)`
- pub `getCandidate` method L1 — `getCandidate(e)`
- pub `getChecks` method L1 — `getChecks(e,t)`
- pub `getComment` method L1 — `getComment(e)`
- pub `getConfiguration` method L1 — `getConfiguration(e,t)`
- pub `getDocument` method L1 — `getDocument(e)`
- pub `getDocumentation` method L1 — `getDocumentation(e)`
- pub `getElement` method L1 — `getElement(e)`
- pub `getFileDescriptions` method L1 — `getFileDescriptions(e,t)`
- pub `getGlobalScope` method L1 — `getGlobalScope(e,t)`
- pub `getGrammarElement` method L1 — `getGrammarElement(e)`
- pub `getGrammarElementId` method L1 — `getGrammarElementId(e)`
- pub `getKey` method L1 — `getKey(e)`
- pub `getLineOffsets` method L1 — `getLineOffsets()`
- pub `getLinkedNode` method L1 — `getLinkedNode(e)`
- pub `getName` method L1 — `getName(e)`
- pub `getNameNode` method L1 — `getNameNode(e)`
- pub `getOrCreateDocument` method L1 — `getOrCreateDocument(e,t)`
- pub `getPathSegment` method L1 — `getPathSegment({$containerProperty:e,$containerIndex:t})`
- pub `getRefNode` method L1 — `getRefNode(e,t,n)`
- pub `getReferenceToSelf` method L1 — `getReferenceToSelf(e)`
- pub `getReferenceType` method L1 — `getReferenceType(e)`
- pub `getRootFolder` method L1 — `getRootFolder(e)`
- pub `getRuleStack` method L1 — `getRuleStack()`
- pub `getScope` method L1 — `getScope(e)`
- pub `getServices` method L1 — `getServices(e)`
- pub `getSource` method L1 — `getSource()`
- pub `getTag` method L1 — `getTag(e)`
- pub `getTags` method L1 — `getTags(e)`
- pub `getText` method L1 — `getText(e)`
- pub `getTokenType` method L1 — `getTokenType(e)`
- pub `getTypeMetaData` method L1 — `getTypeMetaData(e)`
- pub `group` method L1 — `group()`
- pub `handleModes` method L1 — `handleModes(e,t,n,r)`
- pub `handlePayloadNoCustom` method L1 — `handlePayloadNoCustom(e,t)`
- pub `handlePayloadWithCustom` method L1 — `handlePayloadWithCustom(e,t)`
- pub `has` method L1 — `has(e,t)`
- pub `hasDocument` method L1 — `hasDocument(e)`
- pub `head` method L1 — `head()`
- pub `hexEscapeSequenceAtom` method L1 — `hexEscapeSequenceAtom()`
- pub `hidden` method L1 — `hidden()`
- pub `hydrate` method L1 — `hydrate(e)`
- pub `hydrateAstNode` method L1 — `hydrateAstNode(e,t)`
- pub `hydrateCstLeafNode` method L1 — `hydrateCstLeafNode(e)`
- pub `hydrateCstNode` method L1 — `hydrateCstNode(e,t,n=0)`
- pub `hydrateReference` method L1 — `hydrateReference(e,t,n,r)`
- pub `identityEscapeAtom` method L1 — `identityEscapeAtom()`
- pub `includeEntry` method L1 — `includeEntry(e,t,n)`
- pub `includes` method L1 — `includes(e)`
- pub `indexOf` method L1 — `indexOf(e,t=0)`
- pub `initialize` method L1 — `initialize(e)`
- pub `initializeWorkspace` method L1 — `initializeWorkspace(e,t=yc.XO.None)`
- pub `initialized` method L1 — `initialized(e)`
- pub `integerIncludingZero` method L1 — `integerIncludingZero()`
- pub `invalidateDocument` method L1 — `invalidateDocument(e)`
- pub `invoke` method L1 — `invoke(...e)`
- pub `is` method L1 — `is(e)`
- pub `isAffected` method L1 — `isAffected(e,t)`
- pub `isAssertion` method L1 — `isAssertion()`
- pub `isAtom` method L1 — `isAtom()`
- pub `isCancellationRequested` method L1 — `isCancellationRequested()`
- pub `isClassAtom` method L1 — `isClassAtom(e=0)`
- pub `isDigit` method L1 — `isDigit()`
- pub `isEmpty` method L1 — `isEmpty()`
- pub `isEpsilon` method L1 — `isEpsilon()`
- pub `isFull` method L1 — `isFull(e)`
- pub `isIncremental` method L1 — `isIncremental(e)`
- pub `isInstance` method L1 — `isInstance(e,t)`
- pub `isPatternCharacter` method L1 — `isPatternCharacter()`
- pub `isQuantifier` method L1 — `isQuantifier()`
- pub `isRangeDash` method L1 — `isRangeDash()`
- pub `isRecording` method L1 — `isRecording()`
- pub `isRegExpFlag` method L1 — `isRegExpFlag()`
- pub `isSubtype` method L1 — `isSubtype(e,t)`
- pub `isTerm` method L1 — `isTerm()`
- pub `isUri` method L1 — `isUri(e)`
- pub `isValidToken` method L1 — `isValidToken(e)`
- pub `iterator` method L1 — `iterator()`
- pub `join` method L1 — `join(e=",")`
- pub `keepStackSize` method L1 — `keepStackSize()`
- pub `key` method L1 — `key()`
- pub `keys` method L1 — `keys()`
- pub `languageId` method L1 — `languageId()`
- pub `lastNonHiddenNode` method L1 — `lastNonHiddenNode()`
- pub `length` method L1 — `length()`
- pub `limit` method L1 — `limit(e)`
- pub `lineCount` method L1 — `lineCount()`
- pub `link` method L1 — `link(e,t=yc.XO.None)`
- pub `linkNode` method L1 — `linkNode(e,t,n,i,s,a)`
- pub `loadAdditionalDocuments` method L1 — `loadAdditionalDocuments(e,t)`
- pub `loadAstNode` method L1 — `loadAstNode(e)`
- pub `loc` method L1 — `loc(e)`
- pub `many` method L1 — `many(e,t)`
- pub `map` method L1 — `map(e)`
- pub `matchWithExec` method L1 — `matchWithExec(e,t)`
- pub `matchWithTest` method L1 — `matchWithTest(e,t,n)`
- pub `nonNullable` method L1 — `nonNullable()`
- pub `notifyBuildPhase` method L1 — `notifyBuildPhase(e,t,n)`
- pub `nulCharacterAtom` method L1 — `nulCharacterAtom()`
- pub `offset` method L1 — `offset()`
- pub `offsetAt` method L1 — `offsetAt(e)`
- pub `onBuildPhase` method L1 — `onBuildPhase(e,t)`
- pub `onCancellationRequested` method L1 — `onCancellationRequested()`
- pub `onDispose` method L1 — `onDispose(e)`
- pub `onUpdate` method L1 — `onUpdate(e)`
- pub `optional` method L1 — `optional(e,t)`
- pub `parent` method L1 — `parent()`
- pub `parse` method L1 — `parse(e)`
- pub `parseAsync` method L1 — `parseAsync(e,t,n)`
- pub `parseHexDigits` method L1 — `parseHexDigits(e)`
- pub `pattern` method L1 — `pattern(e)`
- pub `patternCharacter` method L1 — `patternCharacter()`
- pub `peekChar` method L1 — `peekChar(e=0)`
- pub `performNextOperation` method L1 — `performNextOperation()`
- pub `performSelfAnalysis` method L1 — `performSelfAnalysis(e)`
- pub `performStartup` method L1 — `performStartup(e)`
- pub `performSubruleAssignment` method L1 — `performSubruleAssignment(e,t,n)`
- pub `popChar` method L1 — `popChar()`
- pub `positionAt` method L1 — `positionAt(e)`
- pub `positiveInteger` method L1 — `positiveInteger()`
- pub `prepareBuild` method L1 — `prepareBuild(e,t)`
- pub `processLexingErrors` method L1 — `processLexingErrors(e,t,n)`
- pub `processLinkingErrors` method L1 — `processLinkingErrors(e,t,n)`
- pub `processNode` method L1 — `processNode(e,t,n)`
- pub `processParsingErrors` method L1 — `processParsingErrors(e,t,n)`
- pub `push` method L1 — `push(...e)`
- pub `quantifier` method L1 — `quantifier(e=!1)`
- pub `range` method L1 — `range()`
- pub `read` method L1 — `read(e)`
- pub `readDirectory` method L1 — `readDirectory()`
- pub `readFile` method L1 — `readFile()`
- pub `ready` method L1 — `ready()`
- pub `recursiveReduce` method L1 — `recursiveReduce(e,t,n)`
- pub `reduce` method L1 — `reduce(e,t)`
- pub `reduceRight` method L1 — `reduceRight(e,t)`
- pub `regExpUnicodeEscapeSequenceAtom` method L1 — `regExpUnicodeEscapeSequenceAtom()`
- pub `regexPatternFunction` method L1 — `regexPatternFunction(e)`
- pub `register` method L1 — `register(e)`
- pub `remove` method L1 — `remove(e,t=null)`
- pub `removeNode` method L1 — `removeNode(e)`
- pub `removeUnexpectedElements` method L1 — `removeUnexpectedElements()`
- pub `replacer` method L1 — `replacer(e,t,{refText:n,sourceText:s,textRegions:a,comments:o,uriConverter:c})`
- pub `requiresCustomPattern` method L1 — `requiresCustomPattern(e)`
- pub `resetStackSize` method L1 — `resetStackSize(e)`
- pub `resetState` method L1 — `resetState()`
- pub `resolveRefs` method L1 — `resolveRefs()`
- pub `restoreState` method L1 — `restoreState(e)`
- pub `revive` method L1 — `revive(e)`
- pub `reviveReference` method L1 — `reviveReference(e,t,n,i,s)`
- pub `rule` method L1 — `rule(e,t)`
- pub `runCancelable` method L1 — `runCancelable(e,t,n,r)`
- pub `runConverter` method L1 — `runConverter(e,t,n)`
- pub `saveState` method L1 — `saveState()`
- pub `serialize` method L1 — `serialize(e,t={})`
- pub `set` method L1 — `set(e,t)`
- pub `setParent` method L1 — `setParent(e,t)`
- pub `shouldRelink` method L1 — `shouldRelink(e,t)`
- pub `shouldValidate` method L1 — `shouldValidate(e)`
- pub `size` method L1 — `size()`
- pub `some` method L1 — `some(e)`
- pub `splice` method L1 — `splice(e,t,...n)`
- pub `startImplementation` method L1 — `startImplementation(e,t)`
- pub `startWalking` method L1 — `startWalking()`
- pub `subrule` method L1 — `subrule(e,t,n,r)`
- pub `tail` method L1 — `tail(e=1)`
- pub `term` method L1 — `term()`
- pub `text` method L1 — `text()`
- pub `throwIfDisposed` method L1 — `throwIfDisposed()`
- pub `toArray` method L1 — `toArray()`
- pub `toDiagnostic` method L1 — `toDiagnostic(e,t,n)`
- pub `toJSON` method L1 — `toJSON()`
- pub `toMap` method L1 — `toMap(e,t)`
- pub `toMarkdown` method L1 — `toMarkdown(e)`
- pub `toMarkdownDefault` method L1 — `toMarkdownDefault(e)`
- pub `toSectionName` method L1 — `toSectionName(e)`
- pub `toSet` method L1 — `toSet()`
- pub `toString` method L1 — `toString()`
- pub `toTokenTypeDictionary` method L1 — `toTokenTypeDictionary(e)`
- pub `tokenType` method L1 — `tokenType()`
- pub `tokenize` method L1 — `tokenize(e,t=this.defaultMode)`
- pub `tokenizeInternal` method L1 — `tokenizeInternal(e,t)`
- pub `traverseFolder` method L1 — `traverseFolder(e,t,n,r)`
- pub `unlink` method L1 — `unlink(e)`
- pub `unorderedGroups` method L1 — `unorderedGroups()`
- pub `unshift` method L1 — `unshift(...e)`
- pub `update` method L1 — `update(e,t)`
- pub `updateConfiguration` method L1 — `updateConfiguration(e)`
- pub `updateContent` method L1 — `updateContent(e,t=yc.XO.None)`
- pub `updateExpectedNext` method L1 — `updateExpectedNext()`
- pub `updateLastIndex` method L1 — `updateLastIndex(e,t)`
- pub `updateReferences` method L1 — `updateReferences(e,t=yc.XO.None)`
- pub `updateSectionConfiguration` method L1 — `updateSectionConfiguration(e,t)`
- pub `updateTokenEndLineColumnLocation` method L1 — `updateTokenEndLineColumnLocation(e,t,n,r,i,s,a)`
- pub `uri` method L1 — `uri()`
- pub `validate` method L1 — `validate(e)`
- pub `validateAmbiguousAlternationAlternatives` method L1 — `validateAmbiguousAlternationAlternatives(e,t)`
- pub `validateAst` method L1 — `validateAst(e,t,n=yc.XO.None)`
- pub `validateDocument` method L1 — `validateDocument(e,t={},n=yc.XO.None)`
- pub `validateEmptyOrAlternatives` method L1 — `validateEmptyOrAlternatives(e)`
- pub `validateNoLeftRecursion` method L1 — `validateNoLeftRecursion(e)`
- pub `validateSomeNonEmptyLookaheadPath` method L1 — `validateSomeNonEmptyLookaheadPath(e,t)`
- pub `values` method L1 — `values()`
- pub `version` method L1 — `version()`
- pub `visit` method L1 — `visit(e)`
- pub `visitAlternation` method L1 — `visitAlternation(e)`
- pub `visitAlternative` method L1 — `visitAlternative(e)`
- pub `visitCharacter` method L1 — `visitCharacter(e)`
- pub `visitChildren` method L1 — `visitChildren(e)`
- pub `visitDisjunction` method L1 — `visitDisjunction(e)`
- pub `visitEndAnchor` method L1 — `visitEndAnchor(e)`
- pub `visitFlags` method L1 — `visitFlags(e)`
- pub `visitGroup` method L1 — `visitGroup(e)`
- pub `visitGroupBackReference` method L1 — `visitGroupBackReference(e)`
- pub `visitLookahead` method L1 — `visitLookahead(e)`
- pub `visitNegativeLookahead` method L1 — `visitNegativeLookahead(e)`
- pub `visitNonTerminal` method L1 — `visitNonTerminal(e)`
- pub `visitNonWordBoundary` method L1 — `visitNonWordBoundary(e)`
- pub `visitOption` method L1 — `visitOption(e)`
- pub `visitPattern` method L1 — `visitPattern(e)`
- pub `visitQuantifier` method L1 — `visitQuantifier(e)`
- pub `visitRepetition` method L1 — `visitRepetition(e)`
- pub `visitRepetitionMandatory` method L1 — `visitRepetitionMandatory(e)`
- pub `visitRepetitionMandatoryWithSeparator` method L1 — `visitRepetitionMandatoryWithSeparator(e)`
- pub `visitRepetitionWithSeparator` method L1 — `visitRepetitionWithSeparator(e)`
- pub `visitRule` method L1 — `visitRule(e)`
- pub `visitSet` method L1 — `visitSet(e)`
- pub `visitStartAnchor` method L1 — `visitStartAnchor(e)`
- pub `visitTerminal` method L1 — `visitTerminal(e)`
- pub `visitWordBoundary` method L1 — `visitWordBoundary(e)`
- pub `waitUntil` method L1 — `waitUntil(e,t,n)`
- pub `walk` method L1 — `walk(e,t=[])`
- pub `walkAtLeastOne` method L1 — `walkAtLeastOne(e,t,n)`
- pub `walkAtLeastOneSep` method L1 — `walkAtLeastOneSep(e,t,n)`
- pub `walkFlat` method L1 — `walkFlat(e,t,n)`
- pub `walkMany` method L1 — `walkMany(e,t,n)`
- pub `walkManySep` method L1 — `walkManySep(e,t,n)`
- pub `walkOption` method L1 — `walkOption(e,t,n)`
- pub `walkOr` method L1 — `walkOr(e,t,n)`
- pub `walkProdRef` method L1 — `walkProdRef(e,t,n)`
- pub `walkTerminal` method L1 — `walkTerminal(e,t,n)`
- pub `with` method L1 — `with(e)`
- pub `wrapAtLeastOne` method L1 — `wrapAtLeastOne(e,t)`
- pub `wrapConsume` method L1 — `wrapConsume(e,t)`
- pub `wrapMany` method L1 — `wrapMany(e,t)`
- pub `wrapOption` method L1 — `wrapOption(e,t)`
- pub `wrapOr` method L1 — `wrapOr(e,t)`
- pub `wrapSelfAnalysis` method L1 — `wrapSelfAnalysis()`
- pub `wrapSubrule` method L1 — `wrapSubrule(e,t,n)`
- pub `wrapValidationException` method L1 — `wrapValidationException(e,t)`
- pub `write` method L1 — `write(e)`
-  `$c` function L1 — `function $c(e)`
-  `$e` function L1 — `function $e(e)`
-  `$i` function L1 — `function $i(e,t)`
-  `$l` function L1 — `function $l(e)`
-  `$r` function L1 — `function $r(e,t)`
-  `A` function L1 — `function A(e)`
-  `Ai` function L1 — `function Ai(e,t,n,r)`
-  `Bc` class L1 — `-`
-  `Be` class L1 — `-`
-  `Bl` class L1 — `-`
-  `Bs` function L1 — `function Bs(e,t)`
-  `Bt` class L1 — `-`
-  `Cc` function L1 — `function Cc(e)`
-  `Ce` function L1 — `function Ce(e)`
-  `Ci` function L1 — `function Ci(e,t,n)`
-  `Cl` function L1 — `function Cl(e,t)`
-  `Cr` function L1 — `function Cr(e)`
-  `D` function L1 — `-`
-  `Di` function L1 — `function Di(e,t,n,r=[])`
-  `Dl` class L1 — `-`
-  `Dr` function L1 — `function Dr(e)`
-  `Ds` class L1 — `-`
-  `E` function L1 — `function E(e,t)`
-  `Ec` class L1 — `-`
-  `Ee` function L1 — `function Ee(e)`
-  `Ei` function L1 — `function Ei(e,t,n,r)`
-  `Es` function L1 — `function Es(e)`
-  `Fc` class L1 — `-`
-  `Fi` class L1 — `-`
-  `Fs` function L1 — `function Fs(e,t,n)`
-  `G` function L1 — `function G(e)`
-  `Gc` class L1 — `-`
-  `Gi` class L1 — `-`
-  `Gl` class L1 — `-`
-  `Gs` function L1 — `function Gs(e,t,n,r,i)`
-  `Gt` class L1 — `-`
-  `Hc` class L1 — `-`
-  `Hl` function L1 — `function Hl(e,t,n,r,i,s,a,o,c)`
-  `Ho` class L1 — `-`
-  `Hs` function L1 — `function Hs(e,t,n)`
-  `Ht` class L1 — `-`
-  `Ie` function L1 — `function Ie(e,t)`
-  `Ii` class L1 — `-`
-  `Il` function L1 — `function Il(e,t)`
-  `Ir` function L1 — `function Ir(e,t)`
-  `J` function L1 — `function J(e)`
-  `Jc` class L1 — `-`
-  `Je` function L1 — `function Je(e,t,n,r)`
-  `Jn` function L1 — `function Jn(e,t,n)`
-  `Jo` function L1 — `-`
-  `Js` function L1 — `function Js(e,t=!0)`
-  `Jt` class L1 — `-`
-  `Kc` class L1 — `-`
-  `Ke` class L1 — `-`
-  `Kl` class L1 — `-`
-  `Ks` function L1 — `function Ks(e,t,n,r,i)`
-  `Kt` class L1 — `-`
-  `Le` function L1 — `function Le(e,t)`
-  `Li` function L1 — `function Li(e,t,n,r)`
-  `Ll` function L1 — `function Ll(e,t)`
-  `Lr` function L1 — `function Lr(e)`
-  `Ls` class L1 — `-`
-  `M` function L1 — `function M(e)`
-  `Mi` class L1 — `-`
-  `Ml` function L1 — `function Ml(e)`
-  `Mr` function L1 — `function Mr(e)`
-  `Ms` class L1 — `-`
-  `Nc` function L1 — `function Nc(e,t,n=0)`
-  `Ne` function L1 — `function Ne(e)`
-  `Ni` function L1 — `function Ni(e)`
-  `Nl` function L1 — `function Nl(e)`
-  `Oe` function L1 — `function Oe(e)`
-  `Oi` function L1 — `function Oi(e,t)`
-  `Ol` class L1 — `-`
-  `Or` function L1 — `function Or(e)`
-  `Os` class L1 — `-`
-  `P` function L1 — `function P()`
-  `Pi` function L1 — `function Pi(e)`
-  `Pl` class L1 — `-`
-  `Pr` class L1 — `-`
-  `Ps` class L1 — `-`
-  `Qc` class L1 — `-`
-  `Qe` function L1 — `function Qe(e,t,n)`
-  `Qi` class L1 — `-`
-  `Ql` class L1 — `-`
-  `Qn` function L1 — `function Qn(e,t,n)`
-  `Qo` function L1 — `function Qo(e)`
-  `Qs` class L1 — `-`
-  `Qt` function L1 — `function Qt(e)`
-  `Rc` function L1 — `function Rc(e)`
-  `Re` function L1 — `function Re(e,t)`
-  `Ri` function L1 — `function Ri(e)`
-  `Rn` class L1 — `-`
-  `Rs` function L1 — `function Rs(e,t)`
-  `Sc` function L1 — `function Sc(e,t)`
-  `Se` function L1 — `function Se(e,t)`
-  `Si` function L1 — `function Si(e)`
-  `Sl` function L1 — `function Sl(e)`
-  `T` function L1 — `function T(e)`
-  `Te` class L1 — `-`
-  `Tn` function L1 — `function Tn(e)`
-  `Tr` function L1 — `function Tr(e)`
-  `U` function L1 — `function U(e)`
-  `Uc` class L1 — `-`
-  `Ui` function L1 — `function Ui(e)`
-  `Ul` class L1 — `-`
-  `Us` function L1 — `function Us(e,t,n)`
-  `V` function L1 — `function V(e)`
-  `Vc` class L1 — `-`
-  `Vl` function L1 — `function Vl(e)`
-  `Vn` function L1 — `function Vn(e)`
-  `Vo` class L1 — `-`
-  `Vs` function L1 — `function Vs(e,t,n,r)`
-  `Vt` class L1 — `-`
-  `W` function L1 — `function W(e)`
-  `Wc` class L1 — `-`
-  `We` function L1 — `function We(e)`
-  `Wi` function L1 — `function Wi(e)`
-  `Wo` class L1 — `-`
-  `Wr` function L1 — `function Wr(e)`
-  `Ws` function L1 — `function Ws(e,t)`
-  `Wt` class L1 — `-`
-  `Xc` class L1 — `-`
-  `Xe` function L1 — `function Xe(e,t)`
-  `Xi` class L1 — `-`
-  `Xl` function L1 — `function Xl(e,t,n,r)`
-  `Xo` class L1 — `-`
-  `Xr` function L1 — `function Xr(e,t)`
-  `Xs` function L1 — `function Xs(e,t)`
-  `Xt` class L1 — `-`
-  `Y` function L1 — `function Y(e)`
-  `Yc` class L1 — `-`
-  `Ye` function L1 — `function Ye(e)`
-  `Yi` class L1 — `-`
-  `Yo` class L1 — `-`
-  `Yr` function L1 — `function Yr(e,t,n,r,i,s,a,o)`
-  `Ys` function L1 — `function Ys(e,t)`
-  `Yt` class L1 — `-`
-  `Zc` function L1 — `function Zc(e)`
-  `Ze` function L1 — `function Ze(e)`
-  `Zn` function L1 — `function Zn(e,t)`
-  `Zo` class L1 — `-`
-  `Zr` class L1 — `-`
-  `_` function L1 — `function _(e)`
-  `_e` function L1 — `function _e(e)`
-  `_i` function L1 — `function _i(e)`
-  `_l` class L1 — `-`
-  `_s` class L1 — `-`
-  `a` class L1 — `-`
-  `a` function L1 — `function a(e)`
-  `aa` function L1 — `function aa(e,t,n,r)`
-  `ac` function L1 — `function ac(e,t,n)`
-  `ae` function L1 — `function ae(e)`
-  `al` function L1 — `function al(e)`
-  `as` class L1 — `-`
-  `be` function L1 — `function be()`
-  `bi` function L1 — `function bi(e)`
-  `bl` class L1 — `-`
-  `bs` function L1 — `function bs(e,t,n)`
-  `c` function L1 — `function c(e)`
-  `c` class L1 — `-`
-  `c` function L1 — `function c(e=i.DD)`
-  `ca` function L1 — `function ca(e,t,n,r)`
-  `cc` function L1 — `function cc(e)`
-  `cl` class L1 — `-`
-  `cs` function L1 — `function cs(e,t)`
-  `ct` function L1 — `function ct(e)`
-  `d` function L1 — `function d(e)`
-  `da` function L1 — `function da(e)`
-  `dc` function L1 — `function dc(e,t,n,r)`
-  `di` class L1 — `-`
-  `dl` class L1 — `-`
-  `dr` function L1 — `function dr(e)`
-  `ds` function L1 — `function ds(e,t)`
-  `ea` function L1 — `function ea(e,t)`
-  `ec` class L1 — `-`
-  `el` class L1 — `-`
-  `er` function L1 — `function er(e)`
-  `es` class L1 — `-`
-  `et` function L1 — `function et(e,t,n)`
-  `fa` function L1 — `function fa(e,t)`
-  `fc` function L1 — `function fc(e,t)`
-  `fi` class L1 — `-`
-  `fl` class L1 — `-`
-  `fn` function L1 — `function fn(e)`
-  `g` function L1 — `function g(t,n)`
-  `ge` function L1 — `function ge(e)`
-  `gi` function L1 — `function gi(e,t,n=[])`
-  `gl` function L1 — `function gl(e)`
-  `gr` function L1 — `function gr(e,t,n)`
-  `gt` function L1 — `function gt(e)`
-  `h` class L1 — `-`
-  `ha` function L1 — `function ha(e,t,n,r)`
-  `hc` function L1 — `function hc(e,t)`
-  `hi` class L1 — `-`
-  `hl` class L1 — `-`
-  `hn` function L1 — `function hn(e,t=[])`
-  `hr` function L1 — `function hr(e)`
-  `i` function L1 — `function i(e)`
-  `ia` function L1 — `function ia(e,t=!0)`
-  `ie` function L1 — `function ie(e)`
-  `il` class L1 — `-`
-  `it` function L1 — `function it(e)`
-  `jc` class L1 — `-`
-  `jl` function L1 — `function jl(e)`
-  `jn` function L1 — `function jn(e)`
-  `jo` class L1 — `-`
-  `js` function L1 — `function js(e,t,n,r,...i)`
-  `jt` class L1 — `-`
-  `k` class L1 — `-`
-  `kc` class L1 — `-`
-  `ke` function L1 — `function ke(e,t)`
-  `ki` function L1 — `function ki(e,t,n)`
-  `kl` function L1 — `function kl(e,t)`
-  `kr` function L1 — `function kr(e)`
-  `ks` function L1 — `function ks(e)`
-  `l` function L1 — `function l(e)`
-  `la` function L1 — `function la(e,t,n)`
-  `lc` function L1 — `function lc(e)`
-  `li` class L1 — `-`
-  `ll` class L1 — `-`
-  `lr` function L1 — `function lr(e)`
-  `ls` function L1 — `function ls(e,t)`
-  `lt` function L1 — `function lt(e,t)`
-  `m` class L1 — `-`
-  `m` function L1 — `function m(e)`
-  `ma` function L1 — `function ma(e,t)`
-  `mc` class L1 — `-`
-  `mi` class L1 — `-`
-  `ml` function L1 — `function ml(e)`
-  `mn` function L1 — `function mn(e,t,n)`
-  `mr` function L1 — `function mr(e)`
-  `n` function L1 — `function n(e,t)`
-  `nc` class L1 — `-`
-  `nl` function L1 — `function nl(e)`
-  `nr` function L1 — `function nr(e,t)`
-  `nt` function L1 — `function nt(e,t)`
-  `o` function L1 — `function o(e=i.DD)`
-  `oa` function L1 — `function oa(e,t,n,r,i,s)`
-  `oc` function L1 — `function oc(e,t,n=!1)`
-  `ol` class L1 — `-`
-  `ot` function L1 — `function ot(e,t)`
-  `p` function L1 — `function p(...e)`
-  `pa` function L1 — `function pa(e,t)`
-  `pc` class L1 — `-`
-  `pe` function L1 — `function pe(e)`
-  `pi` class L1 — `-`
-  `pl` class L1 — `-`
-  `pn` class L1 — `-`
-  `pr` function L1 — `function pr(e,t)`
-  `q` function L1 — `function q(e)`
-  `qc` class L1 — `-`
-  `qe` function L1 — `function qe(e,t,n)`
-  `qi` class L1 — `-`
-  `ql` function L1 — `function ql(e,t)`
-  `qn` function L1 — `function qn(e,t=!1)`
-  `qt` class L1 — `-`
-  `r` function L1 — `function r(e)`
-  `ra` class L1 — `-`
-  `rc` class L1 — `-`
-  `rl` class L1 — `-`
-  `rt` function L1 — `function rt(e)`
-  `s` class L1 — `-`
-  `s` function L1 — `function s(e)`
-  `sa` function L1 — `function sa(e,t,n,r)`
-  `sc` class L1 — `-`
-  `sl` function L1 — `function sl(e)`
-  `ss` function L1 — `function ss(e,t,n)`
-  `st` function L1 — `function st(e)`
-  `t` function L1 — `function t()`
-  `t` class L1 — `-`
-  `t` function L1 — `const t = ()`
-  `ta` class L1 — `-`
-  `tc` class L1 — `-`
-  `te` function L1 — `function te(e)`
-  `tl` class L1 — `-`
-  `tr` class L1 — `-`
-  `ts` function L1 — `function ts(e,t,n,r,i,s,a)`
-  `tt` function L1 — `function tt(e)`
-  `tu` function L1 — `function tu(e)`
-  `u` class L1 — `-`
-  `u` function L1 — `const u = ()`
-  `u` class L1 — `-`
-  `ua` function L1 — `function ua(e,t)`
-  `uc` function L1 — `function uc(e,t,n=t.terminal)`
-  `ue` function L1 — `function ue(e)`
-  `ui` class L1 — `-`
-  `ul` class L1 — `-`
-  `ur` function L1 — `function ur(e)`
-  `us` function L1 — `function us(e,t)`
-  `v` function L1 — `function v(e)`
-  `vc` function L1 — `function vc(e)`
-  `vi` function L1 — `function vi(e)`
-  `vl` function L1 — `function vl(e,t,n,r)`
-  `vs` function L1 — `function vs(e,t,n,r=!1)`
-  `wc` function L1 — `function wc(e)`
-  `we` function L1 — `function we(e,t)`
-  `wi` function L1 — `function wi(e,t,n,r)`
-  `wl` function L1 — `function wl(e)`
-  `wr` function L1 — `function wr(e)`
-  `ws` function L1 — `function ws(e=void 0)`
-  `x` function L1 — `function x(e)`
-  `xe` function L1 — `function xe(e,t)`
-  `xi` class L1 — `-`
-  `xl` function L1 — `function xl(e)`
-  `xr` function L1 — `function xr(e,t)`
-  `y` function L1 — `function y(e,t)`
-  `yi` function L1 — `function yi(e,t,n,r)`
-  `yl` function L1 — `function yl(e)`
-  `zc` class L1 — `-`
-  `ze` function L1 — `function ze(e)`
-  `zi` class L1 — `-`
-  `zl` function L1 — `function zl(e,t)`
-  `zn` function L1 — `function zn(e)`
-  `zo` class L1 — `-`
-  `zs` function L1 — `function zs(e,t,n,r)`
-  `zt` class L1 — `-`

#### docs/themes/hugo-geekdoc/static/js/768-19f4d0a4.chunk.min.js

-  `N` function L1 — `function N()`
-  `b` function L1 — `function b()`

#### docs/themes/hugo-geekdoc/static/js/846-699d57b4.chunk.min.js

-  `E` function L1 — `function E(t,r,e)`
-  `g` function L1 — `function g(t,r)`
-  `k` function L1 — `function k(t)`
-  `l` function L1 — `function l()`
-  `n` function L1 — `function n(t,r)`

#### docs/themes/hugo-geekdoc/static/js/848-160cde0b.chunk.min.js

-  `i` function L1 — `function i(e,t)`

#### docs/themes/hugo-geekdoc/static/js/906-5e2ec84c.chunk.min.js

-  `r` function L1 — `function r(t,e)`

#### docs/themes/hugo-geekdoc/static/js/938-e8554e58.chunk.min.js

-  `T` function L1 — `function T()`
-  `at` function L1 — `function at(t,e)`
-  `ct` function L1 — `function ct(t)`
-  `dt` function L1 — `function dt(t,e,i,n,r,{spatialMaps:o,groupAlignments:s})`
-  `g` function L1 — `function g(t,e,i)`
-  `h` function L1 — `function h()`
-  `ht` function L1 — `function ht(t,e)`
-  `i` function L1 — `function i(n)`
-  `l` function L1 — `function l(t,e,i,s)`
-  `lt` function L1 — `function lt(t,e,i)`
-  `n` function L1 — `function n(t,e)`
-  `o` function L1 — `function o(t,e,i,r)`
-  `ot` function L1 — `function ot(t,e)`
-  `q` function L1 — `function q(t)`
-  `r` function L1 — `function r()`
-  `rt` function L1 — `function rt(t,e)`
-  `s` function L1 — `function s(t,e,i)`
-  `st` function L1 — `function st(t,e)`
-  `t` function L1 — `function t(t,e)`

#### docs/themes/hugo-geekdoc/static/js/975-7b2dc052.chunk.min.js

-  `L` function L1 — `function L(t)`
-  `a` function L1 — `function a(t)`
-  `c` function L1 — `function c(t)`
-  `e` function L1 — `function e(t,e,n,s,r,a,o,c,l)`
-  `i` function L1 — `function i(t,e)`
-  `m` function L1 — `function m()`
-  `n` function L1 — `function n(t,e,n,i,s)`
-  `o` function L1 — `function o(t)`
-  `t` function L1 — `function t(t,e,n,s,r,a,o,c)`
-  `u` function L1 — `function u()`

#### docs/themes/hugo-geekdoc/static/js/colortheme-05deda6f.bundle.min.js

-  `a` function L1 — `function a()`
-  `n` function L1 — `function n(r)`
-  `s` function L1 — `function s(n=!0)`

#### docs/themes/hugo-geekdoc/static/js/katex-13a419d8.bundle.min.js

- pub `_getExpansion` method L1 — `_getExpansion(e)`
- pub `baseSizingClasses` method L1 — `baseSizingClasses()`
- pub `beginGroup` method L1 — `beginGroup()`
- pub `callFunction` method L1 — `callFunction(e,t,r,n,i)`
- pub `constructor` method L1 — `constructor(e,t,r)`
- pub `consume` method L1 — `consume()`
- pub `consumeArg` method L1 — `consumeArg(e)`
- pub `consumeArgs` method L1 — `consumeArgs(e,t)`
- pub `consumeSpaces` method L1 — `consumeSpaces()`
- pub `countExpansion` method L1 — `countExpansion(e)`
- pub `cramp` method L1 — `cramp()`
- pub `endGroup` method L1 — `endGroup()`
- pub `endGroups` method L1 — `endGroups()`
- pub `expandAfterFuture` method L1 — `expandAfterFuture()`
- pub `expandMacro` method L1 — `expandMacro(e)`
- pub `expandMacroAsText` method L1 — `expandMacroAsText(e)`
- pub `expandNextToken` method L1 — `expandNextToken()`
- pub `expandOnce` method L1 — `expandOnce(e)`
- pub `expandTokens` method L1 — `expandTokens(e)`
- pub `expect` method L1 — `expect(e,t)`
- pub `extend` method L1 — `extend(e)`
- pub `feed` method L1 — `feed(e)`
- pub `fetch` method L1 — `fetch()`
- pub `fontMetrics` method L1 — `fontMetrics()`
- pub `formLigatures` method L1 — `formLigatures(e)`
- pub `formatUnsupportedCmd` method L1 — `formatUnsupportedCmd(e)`
- pub `fracDen` method L1 — `fracDen()`
- pub `fracNum` method L1 — `fracNum()`
- pub `future` method L1 — `future()`
- pub `get` method L1 — `get(e)`
- pub `getAttribute` method L1 — `getAttribute(e)`
- pub `getColor` method L1 — `getColor()`
- pub `handleInfixNodes` method L1 — `handleInfixNodes(e)`
- pub `handleSupSubscript` method L1 — `handleSupSubscript(e)`
- pub `has` method L1 — `has(e)`
- pub `hasClass` method L1 — `hasClass(e)`
- pub `havingBaseSizing` method L1 — `havingBaseSizing()`
- pub `havingBaseStyle` method L1 — `havingBaseStyle(e)`
- pub `havingCrampedStyle` method L1 — `havingCrampedStyle()`
- pub `havingSize` method L1 — `havingSize(e)`
- pub `havingStyle` method L1 — `havingStyle(e)`
- pub `isDefined` method L1 — `isDefined(e)`
- pub `isExpandable` method L1 — `isExpandable(e)`
- pub `isTight` method L1 — `isTight()`
- pub `isTrusted` method L1 — `isTrusted(e)`
- pub `lex` method L1 — `lex()`
- pub `parse` method L1 — `parse()`
- pub `parseArgumentGroup` method L1 — `parseArgumentGroup(e,t)`
- pub `parseArguments` method L1 — `parseArguments(e,t)`
- pub `parseAtom` method L1 — `parseAtom(e)`
- pub `parseColorGroup` method L1 — `parseColorGroup(e)`
- pub `parseExpression` method L1 — `parseExpression(e,t)`
- pub `parseFunction` method L1 — `parseFunction(e,t)`
- pub `parseGroup` method L1 — `parseGroup(e,r)`
- pub `parseGroupOfType` method L1 — `parseGroupOfType(e,t,r)`
- pub `parseRegexGroup` method L1 — `parseRegexGroup(e,t)`
- pub `parseSizeGroup` method L1 — `parseSizeGroup(e)`
- pub `parseStringGroup` method L1 — `parseStringGroup(e,t)`
- pub `parseSymbol` method L1 — `parseSymbol()`
- pub `parseUrlGroup` method L1 — `parseUrlGroup(e)`
- pub `popToken` method L1 — `popToken()`
- pub `pushToken` method L1 — `pushToken(e)`
- pub `pushTokens` method L1 — `pushTokens(e)`
- pub `range` method L1 — `range(e,r)`
- pub `reportNonstrict` method L1 — `reportNonstrict(e,t,r)`
- pub `scanArgument` method L1 — `scanArgument(e)`
- pub `set` method L1 — `set(e,t,r)`
- pub `setAttribute` method L1 — `setAttribute(e,t)`
- pub `setCatcode` method L1 — `setCatcode(e,t)`
- pub `sizingClasses` method L1 — `sizingClasses(e)`
- pub `sub` method L1 — `sub()`
- pub `subparse` method L1 — `subparse(e)`
- pub `sup` method L1 — `sup()`
- pub `switchMode` method L1 — `switchMode(e)`
- pub `text` method L1 — `text()`
- pub `toMarkup` method L1 — `toMarkup()`
- pub `toNode` method L1 — `toNode()`
- pub `toText` method L1 — `toText()`
- pub `useStrictBehavior` method L1 — `useStrictBehavior(e,t,r)`
- pub `withColor` method L1 — `withColor(e)`
- pub `withFont` method L1 — `withFont(e)`
- pub `withPhantom` method L1 — `withPhantom()`
- pub `withTextFontFamily` method L1 — `withTextFontFamily(e)`
- pub `withTextFontShape` method L1 — `withTextFontShape(e)`
- pub `withTextFontWeight` method L1 — `withTextFontWeight(e)`
-  `$a` class L1 — `-`
-  `At` class L1 — `-`
-  `Dr` function L1 — `function Dr(e)`
-  `Et` function L1 — `function Et(e,t,r,a,n)`
-  `Fr` function L1 — `function Fr(e)`
-  `Ha` class L1 — `-`
-  `Hr` function L1 — `function Hr(e)`
-  `It` function L1 — `function It(e)`
-  `J` class L1 — `-`
-  `Jt` function L1 — `function Jt(e,t)`
-  `K` class L1 — `-`
-  `Kt` function L1 — `function Kt(e,t)`
-  `Mt` function L1 — `function Mt(e,t)`
-  `N` function L1 — `function N(e)`
-  `O` function L1 — `function O(e,t,r)`
-  `Q` class L1 — `-`
-  `Ra` class L1 — `-`
-  `Rr` function L1 — `function Rr(e,t)`
-  `St` function L1 — `function St(e,t)`
-  `Tt` class L1 — `-`
-  `Ua` class L1 — `-`
-  `Ur` function L1 — `function Ur(e)`
-  `Ut` function L1 — `function Ut(e,t)`
-  `V` class L1 — `-`
-  `Vr` function L1 — `function Vr(e,t)`
-  `Xr` function L1 — `function Xr(e)`
-  `Xt` function L1 — `function Xt(e)`
-  `Yr` function L1 — `function Yr(e,t,n)`
-  `Yt` function L1 — `function Yt(e)`
-  `a` class L1 — `-`
-  `ae` class L1 — `-`
-  `b` class L1 — `-`
-  `f` function L1 — `function f(e)`
-  `ga` function L1 — `function ga(e,t,r)`
-  `he` function L1 — `function he(e,t,r,a,n,i)`
-  `ie` function L1 — `function ie(e)`
-  `k` function L1 — `function k()`
-  `lt` function L1 — `function lt(e)`
-  `ne` class L1 — `-`
-  `q` class L1 — `-`
-  `r` class L1 — `-`
-  `re` class L1 — `-`
-  `rr` function L1 — `function rr(e,t,r)`
-  `st` function L1 — `function st(e)`
-  `t` class L1 — `-`
-  `te` class L1 — `-`
-  `v` class L1 — `-`
-  `w` function L1 — `function w()`
-  `x` function L1 — `function x(e)`
-  `zt` function L1 — `function zt(e)`

#### docs/themes/hugo-geekdoc/static/js/main-c5dd8165.bundle.min.js

-  `a` function L2 — `function a(t,e)`
-  `c` function L2 — `function c(t,e,n,r)`
-  `e` function L2 — `function e()`
-  `g` function L2 — `function g(t)`
-  `h` function L2 — `function h(t,e)`
-  `m` function L2 — `function m(t,e)`
-  `n` function L2 — `function n(o)`
-  `o` function L2 — `function o()`
-  `p` function L2 — `function p(t)`
-  `r` function L2 — `function r(t,e,n,o,r)`
-  `s` function L2 — `function s(t)`
-  `v` function L2 — `function v(t,e)`
-  `y` function L2 — `function y(t)`

#### docs/themes/hugo-geekdoc/static/js/mermaid-6735100e.bundle.min.js

- pub `_d` method L2 — `_d(t,e,r)`
- pub `_drawToContext` method L2 — `_drawToContext(t,e,r,i="nonzero")`
- pub `_fillPolygons` method L2 — `_fillPolygons(t,e)`
- pub `_mergedShape` method L2 — `_mergedShape(t)`
- pub `_o` method L2 — `_o(t)`
- pub `arc` method L2 — `arc(t,e,r,i,n,a,o=!1,s)`
- pub `arcTo` method L2 — `arcTo(t,e,r,i,n)`
- pub `areaEnd` method L2 — `areaEnd()`
- pub `areaStart` method L2 — `areaStart()`
- pub `autolink` method L2 — `autolink(t)`
- pub `bezierCurveTo` method L2 — `bezierCurveTo(t,e,r,i,n,a)`
- pub `blockTokens` method L2 — `blockTokens(t,e=[],r=!1)`
- pub `blockquote` method L2 — `blockquote(t)`
- pub `br` method L2 — `br(t)`
- pub `checkbox` method L2 — `checkbox({checked:t})`
- pub `circle` method L2 — `circle(t,e,r,i)`
- pub `closePath` method L2 — `closePath()`
- pub `code` method L2 — `code(t)`
- pub `codespan` method L2 — `codespan(t)`
- pub `constructor` method L2 — `constructor(t)`
- pub `curve` method L2 — `curve(t,e)`
- pub `dashedLine` method L2 — `dashedLine(t,e)`
- pub `def` method L2 — `def(t)`
- pub `del` method L2 — `del(t)`
- pub `delete` method L2 — `delete(t)`
- pub `dotsOnLines` method L2 — `dotsOnLines(t,e)`
- pub `draw` method L2 — `draw(t)`
- pub `ellipse` method L2 — `ellipse(t,e,r,i,n)`
- pub `em` method L2 — `em({tokens:t})`
- pub `emStrong` method L2 — `emStrong(t,e,r="")`
- pub `escape` method L2 — `escape(t)`
- pub `fences` method L2 — `fences(t)`
- pub `fillPolygons` method L2 — `fillPolygons(t,e)`
- pub `fillSketch` method L2 — `fillSketch(t,e)`
- pub `generator` method L2 — `generator()`
- pub `get` method L2 — `get(t)`
- pub `getDefaultOptions` method L2 — `getDefaultOptions()`
- pub `has` method L2 — `has(t)`
- pub `heading` method L2 — `heading(t)`
- pub `hr` method L2 — `hr(t)`
- pub `html` method L2 — `html(t)`
- pub `image` method L2 — `image({href:t,title:e,text:r})`
- pub `inline` method L2 — `inline(t,e=[])`
- pub `inlineText` method L2 — `inlineText(t)`
- pub `inlineTokens` method L2 — `inlineTokens(t,e=[])`
- pub `lex` method L2 — `lex(t,e)`
- pub `lexInline` method L2 — `lexInline(t,e)`
- pub `lheading` method L2 — `lheading(t)`
- pub `line` method L2 — `line(t,e,r,i,n)`
- pub `lineEnd` method L2 — `lineEnd()`
- pub `lineStart` method L2 — `lineStart()`
- pub `lineTo` method L2 — `lineTo(t,e)`
- pub `linearPath` method L2 — `linearPath(t,e)`
- pub `link` method L2 — `link(t)`
- pub `list` method L2 — `list(t)`
- pub `listitem` method L2 — `listitem(t)`
- pub `moveTo` method L2 — `moveTo(t,e)`
- pub `newSeed` method L2 — `newSeed()`
- pub `next` method L2 — `next()`
- pub `opsToPath` method L2 — `opsToPath(t,e)`
- pub `paragraph` method L2 — `paragraph(t)`
- pub `parse` method L2 — `parse(t,e)`
- pub `parseInline` method L2 — `parseInline(t,e)`
- pub `path` method L2 — `path(t,e)`
- pub `point` method L2 — `point(t,e)`
- pub `polygon` method L2 — `polygon(t,e)`
- pub `postprocess` method L2 — `postprocess(t)`
- pub `preprocess` method L2 — `preprocess(t)`
- pub `processAllTokens` method L2 — `processAllTokens(t)`
- pub `provideLexer` method L2 — `provideLexer()`
- pub `provideParser` method L2 — `provideParser()`
- pub `quadraticCurveTo` method L2 — `quadraticCurveTo(t,e,r,i)`
- pub `rect` method L2 — `rect(t,e,r,i)`
- pub `rectangle` method L2 — `rectangle(t,e,r,i,n)`
- pub `reflink` method L2 — `reflink(t,e)`
- pub `renderLines` method L2 — `renderLines(t,e)`
- pub `rules` method L2 — `rules()`
- pub `set` method L2 — `set(t,e)`
- pub `space` method L2 — `space(t)`
- pub `strong` method L2 — `strong({tokens:t})`
- pub `table` method L2 — `table(t)`
- pub `tablecell` method L2 — `tablecell(t)`
- pub `tablerow` method L2 — `tablerow({text:t})`
- pub `tag` method L2 — `tag(t)`
- pub `text` method L2 — `text(t)`
- pub `toPaths` method L2 — `toPaths(t)`
- pub `toString` method L2 — `toString()`
- pub `url` method L2 — `url(t)`
- pub `zigzagLines` method L2 — `zigzagLines(t,e,r)`
-  `$` function L2 — `function $(t,e)`
-  `$a` function L2 — `function $a(t,e)`
-  `$e` function L2 — `function $e(t)`
-  `$o` function L2 — `function $o(t)`
-  `$r` function L2 — `function $r(t)`
-  `$s` function L2 — `function $s(t)`
-  `$t` function L2 — `function $t(t,e)`
-  `A` function L2 — `function A(t,e,r)`
-  `Aa` function L2 — `function Aa(t,e)`
-  `Ae` function L2 — `function Ae(t,e)`
-  `Ai` function L2 — `function Ai(t,e)`
-  `As` function L2 — `function As(t,e,r)`
-  `At` function L2 — `function At(t,e)`
-  `B` function L2 — `function B(t,e,r,i,n,a,o,s,l)`
-  `Ba` function L2 — `function Ba(t,e)`
-  `Be` function L2 — `function Be(t,e)`
-  `Bi` function L2 — `function Bi(t,e,r)`
-  `Bs` function L2 — `function Bs(t)`
-  `Bt` function L2 — `function Bt(t,e)`
-  `C` function L2 — `function C(t,e,r,i,n,a,o,s,l,c)`
-  `Ca` function L2 — `function Ca(t,e)`
-  `Ci` function L2 — `function Ci(t,e)`
-  `Cr` function L2 — `function Cr(t)`
-  `Cs` function L2 — `function Cs(t)`
-  `Ct` function L2 — `function Ct(t,e,{config:{themeVariables:r,flowchart:n}})`
-  `D` function L2 — `function D(t)`
-  `Da` function L2 — `function Da(t,e)`
-  `De` function L2 — `function De()`
-  `Di` function L2 — `function Di(t)`
-  `Do` function L2 — `function Do(t)`
-  `Ds` function L2 — `function Ds(t)`
-  `Dt` function L2 — `function Dt(t,e)`
-  `E` function L2 — `function E(t)`
-  `Ea` function L2 — `function Ea(t,e)`
-  `Ee` function L2 — `function Ee(t,e,r)`
-  `En` function L2 — `function En(t,e,r,i,n,a)`
-  `Eo` function L2 — `function Eo(t)`
-  `Es` function L2 — `function Es(t)`
-  `Et` function L2 — `function Et(t)`
-  `F` function L2 — `function F(t,e)`
-  `Fa` function L2 — `function Fa(t,e)`
-  `Fe` function L2 — `function Fe(t)`
-  `Fi` function L2 — `function Fi()`
-  `Fo` class L2 — `-`
-  `Fs` function L2 — `function Fs(t)`
-  `Ft` function L2 — `function Ft(t,e)`
-  `G` function L2 — `function G(t,e,r,i,n,a,o)`
-  `Ga` function L2 — `function Ga(t)`
-  `Ge` function L2 — `function Ge(t)`
-  `Gn` function L2 — `function Gn(t)`
-  `Go` function L2 — `function Go()`
-  `Gr` function L2 — `function Gr(t,e,r,i)`
-  `Gt` function L2 — `function Gt(t,e)`
-  `H` function L2 — `function H(t,e,r)`
-  `Ha` function L2 — `function Ha(t,e)`
-  `He` function L2 — `function He(t)`
-  `Ho` function L2 — `function Ho(t,e)`
-  `Hr` function L2 — `function Hr(t)`
-  `Ht` function L2 — `function Ht(t,e)`
-  `I` function L2 — `function I(t,e)`
-  `Ia` function L2 — `function Ia(t)`
-  `Ie` function L2 — `function Ie(t)`
-  `Ii` function L2 — `function Ii(t,e)`
-  `Io` function L2 — `function Io(t)`
-  `Is` function L2 — `function Is(t,e)`
-  `It` function L2 — `function It(t,e)`
-  `J` function L2 — `function J(t,e)`
-  `Ja` function L2 — `function Ja(t,e)`
-  `Je` function L2 — `function Je(t,e,r,i,n)`
-  `Jn` function L2 — `function Jn(t,e,r)`
-  `Jo` function L2 — `function Jo(t)`
-  `Jr` class L2 — `-`
-  `Jt` function L2 — `function Jt(t,e)`
-  `K` function L2 — `function K(t,e,r,i,n,a,o,s)`
-  `Ka` function L2 — `function Ka(t,e)`
-  `Ke` function L2 — `function Ke(t,e,r,i)`
-  `Ko` function L2 — `function Ko(t)`
-  `Kr` function L2 — `function Kr(t)`
-  `L` function L2 — `function L(t,e)`
-  `La` function L2 — `function La(t)`
-  `Le` function L2 — `function Le(t)`
-  `Li` function L2 — `function Li(t,e)`
-  `Lo` function L2 — `function Lo(t)`
-  `Ls` function L2 — `function Ls(t)`
-  `Lt` function L2 — `function Lt(t,e)`
-  `M` function L2 — `function M(t,e,r,i)`
-  `Ma` function L2 — `function Ma(t)`
-  `Me` function L2 — `function Me(t,e)`
-  `Mi` function L2 — `function Mi(t,e,r)`
-  `Mr` function L2 — `function Mr(t,e)`
-  `Ms` function L2 — `function Ms(t)`
-  `Mt` function L2 — `function Mt(t,e,r)`
-  `N` function L2 — `function N(t,e,r,i)`
-  `Na` function L2 — `function Na(t,e)`
-  `Ne` function L2 — `function Ne(t,e,r,i)`
-  `Ni` function L2 — `function Ni(t,e)`
-  `Nn` function L2 — `function Nn(t)`
-  `No` function L2 — `function No(t,e,r,i,n,a,o)`
-  `Ns` function L2 — `function Ns(t)`
-  `Nt` function L2 — `function Nt(t,e)`
-  `O` function L2 — `function O(t,e,r,i=1)`
-  `Oa` function L2 — `function Oa(t,e)`
-  `Oe` function L2 — `function Oe()`
-  `Oi` function L2 — `function Oi(t)`
-  `Oo` function L2 — `function Oo(t)`
-  `Os` function L2 — `function Os(t)`
-  `Ot` function L2 — `function Ot(t,e,r)`
-  `P` function L2 — `function P(t,e,r)`
-  `Pa` function L2 — `function Pa(t,e)`
-  `Pe` function L2 — `function Pe(t)`
-  `Pn` function L2 — `function Pn(t)`
-  `Po` function L2 — `function Po()`
-  `Ps` function L2 — `function Ps(t)`
-  `Pt` function L2 — `function Pt(t,e)`
-  `Q` function L2 — `function Q(t,e)`
-  `Qa` function L2 — `function Qa(t,e)`
-  `Qe` function L2 — `function Qe(t,e,r)`
-  `Qn` function L2 — `function Qn(t,e,r)`
-  `Qo` function L2 — `function Qo(t)`
-  `Qr` function L2 — `function Qr(t,e)`
-  `Qt` function L2 — `function Qt(t,e,{config:{themeVariables:r}})`
-  `R` function L2 — `function R(t,e,r,i,n,a=!1)`
-  `Ra` function L2 — `function Ra(t,e)`
-  `Re` function L2 — `function Re(t)`
-  `Ri` function L2 — `function Ri(t)`
-  `Ro` function L2 — `function Ro(t)`
-  `Rs` function L2 — `function Rs(t)`
-  `Rt` function L2 — `function Rt(t,e)`
-  `S` function L2 — `function S(t,e,r)`
-  `Sa` function L2 — `function Sa(t,e)`
-  `Se` function L2 — `function Se(t)`
-  `So` function L2 — `function So(t)`
-  `Sr` function L2 — `function Sr()`
-  `Ss` function L2 — `function Ss(t,e,r)`
-  `St` function L2 — `function St(t,e,{config:{themeVariables:r,flowchart:n}})`
-  `T` function L2 — `function T(t,e)`
-  `Ta` function L2 — `function Ta(t)`
-  `Te` function L2 — `function Te(t,e,r)`
-  `Ti` function L2 — `function Ti(t)`
-  `Ts` function L2 — `function Ts(t,e)`
-  `Tt` function L2 — `function Tt(t,e,{config:{flowchart:r}})`
-  `U` function L2 — `function U(t,e)`
-  `Ua` function L2 — `function Ua(t)`
-  `Ue` function L2 — `function Ue(t)`
-  `Ui` function L2 — `function Ui(t)`
-  `Uo` function L2 — `function Uo(t,e)`
-  `Ur` function L2 — `function Ur(t)`
-  `Ut` function L2 — `function Ut(t,e)`
-  `V` function L2 — `function V(t,e)`
-  `Va` function L2 — `function Va(t,e)`
-  `Ve` function L2 — `function Ve(t,e,r,i)`
-  `Vn` function L2 — `function Vn(t)`
-  `Vo` function L2 — `function Vo()`
-  `Vr` function L2 — `function Vr(t)`
-  `Vt` function L2 — `function Vt(t,e)`
-  `W` function L2 — `function W(t)`
-  `Wa` function L2 — `function Wa(t,e)`
-  `We` function L2 — `function We(t)`
-  `Wo` function L2 — `function Wo(t)`
-  `Wr` function L2 — `function Wr(t)`
-  `Wt` function L2 — `function Wt(t,e)`
-  `X` function L2 — `function X(t,e,r,i)`
-  `Xa` function L2 — `function Xa(t)`
-  `Xe` function L2 — `function Xe(t)`
-  `Xn` function L2 — `function Xn(t)`
-  `Xo` function L2 — `function Xo(t,e,r)`
-  `Xr` function L2 — `function Xr(t)`
-  `Xt` function L2 — `function Xt(t,e)`
-  `Y` function L2 — `function Y(t,e)`
-  `Ya` function L2 — `function Ya(t,e)`
-  `Ye` function L2 — `function Ye(t,e,r,i)`
-  `Yi` function L2 — `function Yi()`
-  `Yn` function L2 — `function Yn(t,e,r)`
-  `Yo` function L2 — `function Yo(t)`
-  `Yr` function L2 — `function Yr(t,e,r,i)`
-  `Yt` function L2 — `function Yt(t,e)`
-  `Z` function L2 — `function Z(t,e)`
-  `Za` function L2 — `function Za(t,e)`
-  `Ze` function L2 — `function Ze(t)`
-  `Zn` function L2 — `function Zn(t,e,r)`
-  `Zo` function L2 — `function Zo(t)`
-  `Zt` function L2 — `function Zt(t,e,{config:{themeVariables:r}})`
-  `_` function L2 — `function _(u)`
-  `_a` function L2 — `function _a(t,e)`
-  `_i` function L2 — `function _i(t,e)`
-  `_s` function L2 — `function _s(t)`
-  `_t` function L2 — `function _t(t,e,{config:{themeVariables:r,flowchart:n}})`
-  `a` function L2 — `function a(t,e,r,a=1)`
-  `aa` function L2 — `function aa(t,e,r)`
-  `ae` function L2 — `function ae(t,e)`
-  `ao` function L2 — `function ao(t)`
-  `as` function L2 — `function as(t)`
-  `at` function L2 — `function at(t,e)`
-  `b` function L2 — `function b(t)`
-  `ba` function L2 — `function ba(t,e)`
-  `be` function L2 — `function be(t,e,r,i="")`
-  `bi` function L2 — `function bi(t,e)`
-  `br` function L2 — `function br(t,e)`
-  `bt` function L2 — `function bt()`
-  `c` class L2 — `-`
-  `c` function L2 — `const c = ()`
-  `ca` function L2 — `function ca(t,e,r)`
-  `ce` function L2 — `function ce(t,e)`
-  `ci` function L2 — `function ci(t,e,r)`
-  `co` function L2 — `function co()`
-  `cr` function L2 — `function cr(t)`
-  `cs` function L2 — `function cs(t,e)`
-  `ct` function L2 — `function ct(t)`
-  `ct` class L2 — `-`
-  `ct` function L2 — `function ct(t)`
-  `d` class L2 — `-`
-  `d` function L2 — `function d(t)`
-  `da` function L2 — `function da(t,e,r)`
-  `de` function L2 — `function de(t,e)`
-  `di` function L2 — `function di(t,e)`
-  `dr` function L2 — `function dr(t,e,r)`
-  `dt` function L2 — `function dt(t,e)`
-  `dt` class L2 — `-`
-  `e` function L2 — `function e(e,r)`
-  `ea` function L2 — `function ea(t,e,r)`
-  `ee` function L2 — `function ee(t,e)`
-  `ei` function L2 — `function ei(t)`
-  `eo` function L2 — `function eo(t,e)`
-  `er` function L2 — `function er(t,e)`
-  `es` function L2 — `function es(t)`
-  `et` function L2 — `function et(t,e)`
-  `et` class L2 — `-`
-  `et` function L2 — `function et(t)`
-  `f` function L2 — `function f(t)`
-  `fa` function L2 — `function fa(t,e,r)`
-  `fe` function L2 — `function fe(t,e,r,i=0,n=0,c=[],h="")`
-  `fi` function L2 — `function fi()`
-  `fr` function L2 — `function fr(t,e,r)`
-  `ft` function L2 — `function ft(t,e)`
-  `g` function L2 — `function g(t,e,r,i,n,a)`
-  `g` class L2 — `-`
-  `g` function L2 — `function g(t)`
-  `ga` function L2 — `function ga(t,e,r)`
-  `ge` function L2 — `function ge(t,e,r,i,n=r.class.padding??12)`
-  `gr` function L2 — `function gr(t,e)`
-  `gs` function L2 — `function gs(t,e,r)`
-  `gt` function L2 — `function gt(t,e,{config:{themeVariables:r}})`
-  `h` class L2 — `-`
-  `h` function L2 — `function h()`
-  `ha` function L2 — `function ha(t,e,r)`
-  `he` function L2 — `function he(t,e)`
-  `hi` function L2 — `function hi(t,e,r)`
-  `hr` function L2 — `function hr(t)`
-  `ht` function L2 — `function ht()`
-  `ht` class L2 — `-`
-  `i` function L2 — `function i()`
-  `ia` function L2 — `function ia(t,e,r)`
-  `ie` function L2 — `function ie(t,e,r,i,n,a)`
-  `ii` function L2 — `function ii()`
-  `io` function L2 — `function io()`
-  `is` class L2 — `-`
-  `it` function L2 — `function it(t,e)`
-  `it` class L2 — `-`
-  `it` function L2 — `function it(t,e)`
-  `j` function L2 — `function j(t,e)`
-  `ja` function L2 — `function ja(t,e)`
-  `je` function L2 — `function je()`
-  `jo` function L2 — `function jo(t)`
-  `jr` function L2 — `function jr(t)`
-  `jt` function L2 — `function jt(t,e)`
-  `k` function L2 — `function k(t)`
-  `ka` function L2 — `function ka(t,e)`
-  `ke` function L2 — `function ke()`
-  `ki` function L2 — `function ki(t,e)`
-  `kr` function L2 — `function kr(t,e)`
-  `ks` function L2 — `function ks(t,e)`
-  `kt` function L2 — `function kt(t,e)`
-  `l` function L2 — `function l(t)`
-  `la` function L2 — `function la(t,e,r)`
-  `le` function L2 — `function le(t,e)`
-  `li` function L2 — `function li(t,e,r)`
-  `lo` function L2 — `function lo(t,e,r,i,n,a,o,s,l,c)`
-  `lr` function L2 — `function lr(t,e)`
-  `ls` function L2 — `function ls(t,e,r)`
-  `lt` function L2 — `function lt(t,e)`
-  `lt` class L2 — `-`
-  `lt` function L2 — `function lt(t)`
-  `m` function L2 — `function m(t,e)`
-  `ma` function L2 — `function ma(t,e)`
-  `me` function L2 — `function me(t,e)`
-  `mr` function L2 — `function mr(t,e)`
-  `mt` function L2 — `function mt(t,e,{dir:r,config:{state:i,themeVariables:n}})`
-  `n` function L2 — `function n(t,e)`
-  `na` function L2 — `function na(t,e,r)`
-  `ne` function L2 — `function ne(t,e)`
-  `ni` function L2 — `function ni()`
-  `no` function L2 — `function no(t)`
-  `nr` function L2 — `function nr(t)`
-  `ns` function L2 — `function ns(t)`
-  `nt` function L2 — `function nt(t,e,r,i=100,n=0,a=180)`
-  `o` function L2 — `function o(t)`
-  `oa` function L2 — `function oa(t,e,r)`
-  `oe` function L2 — `function oe(t,e)`
-  `oo` function L2 — `function oo(t)`
-  `os` function L2 — `function os(t,e)`
-  `ot` function L2 — `function ot(t,e,r,i=100,n=0,a=180)`
-  `p` class L2 — `-`
-  `p` function L2 — `function p(t)`
-  `pa` function L2 — `function pa(t,e,r)`
-  `pe` function L2 — `function pe(t,e)`
-  `pi` function L2 — `function pi(t)`
-  `pn` function L2 — `function pn(t)`
-  `po` function L2 — `function po(t)`
-  `pr` function L2 — `function pr(t,e,r)`
-  `ps` function L2 — `function ps(t,e)`
-  `pt` function L2 — `function pt(t,e)`
-  `pt` class L2 — `-`
-  `pt` function L2 — `function pt(t)`
-  `q` function L2 — `function q(t,e,r,i)`
-  `qa` function L2 — `function qa(t,e)`
-  `qe` function L2 — `function qe()`
-  `qo` function L2 — `function qo(t)`
-  `qr` function L2 — `function qr(t,e,r,i)`
-  `qt` function L2 — `function qt(t,e)`
-  `r` function L2 — `function r(t)`
-  `ra` function L2 — `function ra(t,e,r)`
-  `re` function L2 — `function re(t,e)`
-  `rn` function L2 — `function rn(t,e,r,i)`
-  `ro` function L2 — `function ro()`
-  `rr` function L2 — `function rr(t,e)`
-  `rs` function L2 — `function rs(t)`
-  `rt` function L2 — `function rt(t,e,r,i=100,n=0,a=180)`
-  `s` class L2 — `-`
-  `s` function L2 — `function s()`
-  `sa` function L2 — `function sa(t,e,r)`
-  `se` function L2 — `function se(t,e)`
-  `so` function L2 — `function so(t)`
-  `sr` function L2 — `function sr(t,e)`
-  `st` function L2 — `function st(t,e)`
-  `t` function L2 — `function t()`
-  `ta` function L2 — `function ta(t,e,r)`
-  `te` function L2 — `function te(t,e)`
-  `ti` function L2 — `function ti({_intern:t,_key:e},r)`
-  `to` function L2 — `function to(t,e)`
-  `tr` function L2 — `-`
-  `ts` function L2 — `function ts(t)`
-  `tt` function L2 — `function tt(t)`
-  `tt` class L2 — `-`
-  `tt` function L2 — `function tt(t)`
-  `u` function L2 — `function u(t)`
-  `u` class L2 — `-`
-  `u` function L2 — `function u(t)`
-  `ua` function L2 — `function ua(t,e,r)`
-  `ue` function L2 — `function ue(t,e)`
-  `ui` function L2 — `function ui(t,e)`
-  `uo` function L2 — `function uo(t)`
-  `ur` function L2 — `function ur(t,e,r)`
-  `us` function L2 — `function us(t,e)`
-  `ut` function L2 — `function ut(t)`
-  `ut` class L2 — `-`
-  `v` function L2 — `function v(t,e,r,i,n)`
-  `va` function L2 — `function va(t,e)`
-  `ve` function L2 — `function ve(t)`
-  `vi` function L2 — `function vi(t)`
-  `vr` function L2 — `function vr(t,e,r,i)`
-  `vs` function L2 — `function vs(t)`
-  `vt` function L2 — `function vt(t,e,{config:{themeVariables:r,flowchart:n}})`
-  `w` function L2 — `function w(t,e,r)`
-  `wa` function L2 — `function wa(t,e)`
-  `we` function L2 — `function we(t,e,{config:r})`
-  `wi` function L2 — `function wi(t,e)`
-  `wn` function L2 — `function wn(t)`
-  `wt` function L2 — `function wt(t,e)`
-  `x` function L2 — `function x(t,e)`
-  `xa` function L2 — `function xa(t,e)`
-  `xe` function L2 — `function xe(t,e)`
-  `xi` function L2 — `function xi(t,e)`
-  `xr` function L2 — `function xr(t,e)`
-  `xs` function L2 — `function xs(t,e)`
-  `xt` function L2 — `function xt(t,e)`
-  `y` function L2 — `function y(t,e,r,i,n,a)`
-  `ya` function L2 — `function ya(t,e,r)`
-  `ye` function L2 — `function ye(t,e,r,i=[])`
-  `yr` function L2 — `function yr(t,e)`
-  `ys` function L2 — `function ys(t,e)`
-  `yt` function L2 — `function yt(t,e)`
-  `z` function L2 — `function z(t,e,r)`
-  `za` function L2 — `function za(t,e)`
-  `ze` function L2 — `function ze(t,e,r,i)`
-  `zi` function L2 — `function zi(t)`
-  `zn` function L2 — `function zn(t,e,r)`
-  `zo` function L2 — `function zo(t)`
-  `zs` function L2 — `function zs(t,e,r)`
-  `zt` function L2 — `function zt(t,e,{config:{themeVariables:r}})`

#### docs/themes/hugo-geekdoc/static/js/search-16a110ff.bundle.min.js

- pub `addSchema` method L2 — `addSchema(e,t)`
- pub `constructor` method L2 — `constructor(e,t="2019-09",r=!0)`
- pub `validate` method L2 — `validate(e)`
-  `A` function L2 — `function A(e)`
-  `D` class L2 — `-`
-  `F` function L2 — `function F(e)`
-  `O` function L2 — `function O(e)`
-  `R` function L2 — `function R(e,t,r="2019-09",n=k(t),o=!0,i=null,s="#",a="#",c=Object.create(null))`
-  `S` function L2 — `function S(e,t)`
-  `T` function L2 — `function T(e,t)`
-  `U` function L2 — `function U(e,r,n)`
-  `W` function L2 — `function W(e,t)`
-  `WorkerIndex` function L2 — `function WorkerIndex(e)`
-  `_` function L2 — `function _(e,t,r,n,i,s,a,c)`
-  `__webpack_require__` function L2 — `function __webpack_require__(e)`
-  `a` function L2 — `function a(e)`
-  `b` function L2 — `function b(e,t,r,n)`
-  `c` function L2 — `function c(e)`
-  `create` function L2 — `function create(factory,is_node_js,worker_path)`
-  `d` function L2 — `function d(e)`
-  `f` function L2 — `function f(e)`
-  `g` function L2 — `function g(e,t,r,n,o)`
-  `h` function L2 — `function h(e,t)`
-  `i` function L2 — `function i(e)`
-  `k` function L2 — `function k(e,t=Object.create(null),r=w,n="")`
-  `l` function L2 — `function l(e)`
-  `m` function L2 — `function m(e,t)`
-  `n` function L2 — `function n(e,t)`
-  `o` function L2 — `function o(e,t)`
-  `p` function L2 — `function p(e,t,r,n,i)`
-  `register` function L2 — `function register(e)`
-  `s` function L2 — `function s()`
-  `u` function L2 — `function u(e)`
-  `v` function L2 — `function v(e,t,r)`
-  `w` function L2 — `function w(e,t,r,o,i)`
-  `x` function L2 — `function x(e,t,r)`
-  `y` function L2 — `function y(e)`
-  `z` function L2 — `function z(e,t)`

### examples/features/computation-graphs/continuous-scheduling/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/computation-graphs/continuous-scheduling/src/main.rs

-  `AggregateHourlyTask` struct L42 — `-` — The actual continuous task that processes aggregated data.
-  `AggregateHourlyTask` type L45-83 — `impl Task for AggregateHourlyTask` — 4.
-  `execute` function L46-74 — `( &self, mut context: Context<serde_json::Value>, ) -> Result<Context<serde_json...` — 4.
-  `id` function L76-78 — `(&self) -> &str` — 4.
-  `dependencies` function L80-82 — `(&self) -> &[TaskNamespace]` — 4.
-  `SimulatedDbConnection` struct L86-88 — `{ table: String }` — Simulated database connection for the example.
-  `SimulatedDbConnection` type L90-105 — `impl DataConnection for SimulatedDbConnection` — 4.
-  `connect` function L91-93 — `(&self) -> Result<Box<dyn Any>, DataConnectionError>` — 4.
-  `descriptor` function L95-100 — `(&self) -> ConnectionDescriptor` — 4.
-  `system_metadata` function L102-104 — `(&self) -> serde_json::Value` — 4.
-  `main` function L108-268 — `()` — 4.

### examples/features/computation-graphs/packaged-graph

> *Semantic summary to be generated by AI agent.*

#### examples/features/computation-graphs/packaged-graph/build.rs

-  `main` function L17-19 — `()`

### examples/features/computation-graphs/packaged-graph/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/computation-graphs/packaged-graph/src/lib.rs

- pub `OrderBookData` struct L27-30 — `{ best_bid: f64, best_ask: f64 }` — that can be loaded by the reconciler and executed via FFI.
- pub `PricingData` struct L33-35 — `{ mid_price: f64 }` — that can be loaded by the reconciler and executed via FFI.
- pub `TradeSignal` struct L38-42 — `{ direction: String, price: f64, confidence: f64 }` — that can be loaded by the reconciler and executed via FFI.
- pub `NoActionReason` struct L45-47 — `{ reason: String }` — that can be loaded by the reconciler and executed via FFI.
- pub `TradeConfirmation` struct L50-53 — `{ executed: bool, message: String }` — that can be loaded by the reconciler and executed via FFI.
- pub `AuditRecord` struct L56-59 — `{ logged: bool, reason: String }` — that can be loaded by the reconciler and executed via FFI.
- pub `market_maker` module L72-135 — `-` — that can be loaded by the reconciler and executed via FFI.
- pub `DecisionOutcome` enum L76-79 — `Trade | NoAction` — that can be loaded by the reconciler and executed via FFI.
- pub `decision` function L81-117 — `( orderbook: Option<&OrderBookData>, pricing: Option<&PricingData>, ) -> Decisio...` — that can be loaded by the reconciler and executed via FFI.
- pub `signal_handler` function L119-127 — `(signal: &TradeSignal) -> TradeConfirmation` — that can be loaded by the reconciler and executed via FFI.
- pub `audit_logger` function L129-134 — `(reason: &NoActionReason) -> AuditRecord` — that can be loaded by the reconciler and executed via FFI.

### examples/features/workflows/complex-dag

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/complex-dag/build.rs

-  `main` function L17-19 — `()`

### examples/features/workflows/complex-dag/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/complex-dag/src/lib.rs

-  `complex_dag_workflow` module L34-212 — `-` — - Complex branching and merging
-  `init_config` function L42-46 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `init_database` function L49-53 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `init_logging` function L56-60 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `load_schema` function L67-71 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `setup_security` function L74-78 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `configure_monitoring` function L81-87 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — - Complex branching and merging
-  `create_tables` function L94-98 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `setup_cache` function L101-105 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `load_raw_data` function L112-116 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `validate_data` function L119-123 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `clean_data` function L126-130 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `transform_customers` function L137-143 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — - Complex branching and merging
-  `transform_orders` function L146-150 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `transform_products` function L153-157 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `calculate_metrics` function L164-168 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `generate_insights` function L171-175 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `build_dashboard` function L182-186 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `generate_reports` function L189-193 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `send_notifications` function L200-204 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `cleanup_staging` function L207-211 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging

### examples/features/workflows/cron-scheduling

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/cron-scheduling/build.rs

-  `main` function L17-19 — `()`

### examples/features/workflows/cron-scheduling/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/cron-scheduling/src/main.rs

- pub `data_backup_workflow` module L56-165 — `-` — - Recovery service for missed executions
- pub `check_backup_prerequisites` function L67-80 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — - Recovery service for missed executions
- pub `create_backup_snapshot` function L90-105 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — - Recovery service for missed executions
- pub `verify_backup_integrity` function L115-143 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — - Recovery service for missed executions
- pub `cleanup_old_backups` function L153-164 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — - Recovery service for missed executions
- pub `health_check_workflow` module L175-345 — `-` — - Recovery service for missed executions
- pub `check_system_resources` function L186-213 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — - Recovery service for missed executions
- pub `check_database_connectivity` function L223-249 — `( context: &mut Context<Value>, ) -> Result<(), TaskError>` — - Recovery service for missed executions
- pub `check_external_services` function L259-290 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — - Recovery service for missed executions
- pub `update_health_metrics` function L300-344 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — - Recovery service for missed executions
- pub `daily_report_workflow` module L355-468 — `-` — - Recovery service for missed executions
- pub `collect_daily_metrics` function L366-385 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — - Recovery service for missed executions
- pub `generate_usage_report` function L395-427 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — - Recovery service for missed executions
- pub `send_report_notification` function L437-467 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — - Recovery service for missed executions
-  `main` function L471-533 — `() -> Result<(), Box<dyn std::error::Error>>` — - Recovery service for missed executions
-  `create_cron_schedules` function L536-577 — `(runner: &DefaultRunner) -> Result<(), Box<dyn std::error::Error>>` — Create cron schedules for our workflows
-  `show_execution_stats` function L580-592 — `(runner: &DefaultRunner) -> Result<(), Box<dyn std::error::Error>>` — Display execution statistics

### examples/features/workflows/deferred-tasks

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/deferred-tasks/build.rs

-  `main` function L17-19 — `()`

### examples/features/workflows/deferred-tasks/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/deferred-tasks/src/main.rs

- pub `deferred_pipeline` module L54-128 — `-` — ```
- pub `wait_for_data` function L65-104 — `( context: &mut Context<serde_json::Value>, handle: &mut TaskHandle, ) -> Result...` — Simulates waiting for external data to become available.
- pub `process_data` function L108-127 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Processes data that was fetched by the deferred task.
-  `main` function L131-166 — `() -> Result<(), Box<dyn std::error::Error>>` — ```

### examples/features/workflows/event-triggers

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/event-triggers/build.rs

-  `main` function L17-19 — `()`

### examples/features/workflows/event-triggers/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/event-triggers/src/main.rs

- pub `file_processing_workflow` module L62-133 — `-` — ```
- pub `validate_file` function L67-86 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Validates and parses an incoming file.
- pub `process_file` function L90-111 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Processes the validated file data.
- pub `archive_file` function L115-132 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Archives the processed file.
- pub `queue_processing_workflow` module L143-216 — `-` — ```
- pub `drain_queue` function L148-171 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Drains messages from the queue.
- pub `process_messages` function L175-193 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — Processes the drained messages.
- pub `ack_messages` function L197-215 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Acknowledges processed messages.
- pub `service_recovery_workflow` module L226-337 — `-` — ```
- pub `diagnose_failure` function L231-257 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — Diagnoses the service failure.
- pub `restart_service` function L261-280 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — Attempts to restart the service.
- pub `verify_recovery` function L284-306 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — Verifies service health after restart.
- pub `notify_incident` function L310-336 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — Sends notification about the incident.
-  `triggers` module L50 — `-` — ```
-  `main` function L340-412 — `() -> Result<(), Box<dyn std::error::Error>>` — ```
-  `register_triggers` function L415-430 — `()` — Register triggers in the global trigger registry.
-  `register_trigger_schedules` function L433-497 — `( runner: &DefaultRunner, ) -> Result<(), Box<dyn std::error::Error>>` — Register trigger schedules with the runner (persists configuration to DB).

#### examples/features/workflows/event-triggers/src/triggers.rs

- pub `FileWatcherTrigger` struct L54-58 — `{ name: String, poll_interval: Duration, watch_path: String }` — A trigger that polls for new files in a simulated directory.
- pub `new` function L62-68 — `(name: &str, watch_path: &str, poll_interval: Duration) -> Self` — Creates a new file watcher trigger.
- pub `QueueDepthTrigger` struct L144-149 — `{ name: String, poll_interval: Duration, queue_name: String, threshold: usize }` — A trigger that fires when a queue exceeds a depth threshold.
- pub `new` function L153-160 — `(name: &str, queue_name: &str, threshold: usize, poll_interval: Duration) -> Sel...` — Creates a new queue depth trigger.
- pub `HealthCheckTrigger` struct L231-237 — `{ name: String, poll_interval: Duration, service_name: String, consecutive_failu...` — A trigger that fires when a service becomes unhealthy.
- pub `new` function L241-254 — `( name: &str, service_name: &str, failure_threshold: usize, poll_interval: Durat...` — Creates a new health check trigger.
- pub `create_file_watcher_trigger` function L340-346 — `() -> FileWatcherTrigger` — Creates the file watcher trigger for the file processing workflow.
- pub `create_queue_depth_trigger` function L349-356 — `() -> QueueDepthTrigger` — Creates the queue depth trigger for the queue processing workflow.
- pub `create_health_check_trigger` function L359-366 — `() -> HealthCheckTrigger` — Creates the health check trigger for the recovery workflow.
-  `FILE_COUNTER` variable L37 — `: AtomicUsize` — Counter for simulating file arrivals
-  `QUEUE_DEPTH` variable L40 — `: AtomicUsize` — Counter for simulating queue depth
-  `SERVICE_HEALTHY` variable L43 — `: std::sync::atomic::AtomicBool` — Flag for simulating service health
-  `FileWatcherTrigger` type L60-91 — `= FileWatcherTrigger` — 3.
-  `check_for_new_files` function L72-90 — `(&self) -> Option<String>` — Simulates checking for new files.
-  `FileWatcherTrigger` type L94-133 — `impl Trigger for FileWatcherTrigger` — 3.
-  `name` function L95-97 — `(&self) -> &str` — 3.
-  `poll_interval` function L99-101 — `(&self) -> Duration` — 3.
-  `allow_concurrent` function L103-106 — `(&self) -> bool` — 3.
-  `poll` function L108-132 — `(&self) -> Result<TriggerResult, TriggerError>` — 3.
-  `QueueDepthTrigger` type L151-175 — `= QueueDepthTrigger` — 3.
-  `get_queue_depth` function L164-174 — `(&self) -> usize` — Simulates checking queue depth.
-  `QueueDepthTrigger` type L178-220 — `impl Trigger for QueueDepthTrigger` — 3.
-  `name` function L179-181 — `(&self) -> &str` — 3.
-  `poll_interval` function L183-185 — `(&self) -> Duration` — 3.
-  `allow_concurrent` function L187-190 — `(&self) -> bool` — 3.
-  `poll` function L192-219 — `(&self) -> Result<TriggerResult, TriggerError>` — 3.
-  `HealthCheckTrigger` type L239-265 — `= HealthCheckTrigger` — 3.
-  `check_service_health` function L258-264 — `(&self) -> bool` — Simulates checking service health.
-  `HealthCheckTrigger` type L268-333 — `impl Trigger for HealthCheckTrigger` — 3.
-  `name` function L269-271 — `(&self) -> &str` — 3.
-  `poll_interval` function L273-275 — `(&self) -> Duration` — 3.
-  `allow_concurrent` function L277-280 — `(&self) -> bool` — 3.
-  `poll` function L282-332 — `(&self) -> Result<TriggerResult, TriggerError>` — 3.

### examples/features/workflows/multi-tenant

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/multi-tenant/build.rs

-  `main` function L17-19 — `()`

### examples/features/workflows/multi-tenant/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/multi-tenant/src/main.rs

-  `main` function L28-50 — `() -> Result<(), Box<dyn std::error::Error>>` — with PostgreSQL schema-based isolation.
-  `demonstrate_multi_tenant_setup` function L52-82 — `(database_url: &str) -> Result<(), WorkflowExecutionError>` — with PostgreSQL schema-based isolation.
-  `demonstrate_recovery_scenarios` function L85-123 — `(database_url: &str) -> Result<(), WorkflowExecutionError>` — Demonstrates recovery scenarios for multi-tenant systems

### examples/features/workflows/packaged-triggers

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/packaged-triggers/build.rs

-  `main` function L17-19 — `()`

### examples/features/workflows/packaged-triggers/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/packaged-triggers/src/lib.rs

- pub `file_processing` module L88-166 — `-`
- pub `validate` function L100-118 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
- pub `transform` function L127-144 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
- pub `archive` function L153-165 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`

### examples/features/workflows/packaged-workflows

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/packaged-workflows/build.rs

-  `main` function L17-19 — `()`

### examples/features/workflows/packaged-workflows/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/packaged-workflows/src/lib.rs

- pub `analytics_workflow` module L54-284 — `-`
- pub `extract_data` function L67-94 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
- pub `validate_data` function L106-150 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
- pub `transform_data` function L162-216 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
- pub `generate_reports` function L228-283 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>`

### examples/features/workflows/per-tenant-credentials

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/per-tenant-credentials/build.rs

-  `main` function L17-19 — `()`

### examples/features/workflows/per-tenant-credentials/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/per-tenant-credentials/src/main.rs

-  `main` function L28-50 — `() -> Result<(), Box<dyn std::error::Error>>` — isolated tenant users with their own database credentials and schemas.
-  `demonstrate_admin_tenant_creation` function L52-122 — `( admin_database_url: &str, ) -> Result<(), Box<dyn std::error::Error>>` — isolated tenant users with their own database credentials and schemas.
-  `demonstrate_tenant_isolation` function L124-182 — `( admin_database_url: &str, ) -> Result<(), Box<dyn std::error::Error>>` — isolated tenant users with their own database credentials and schemas.
-  `mask_password` function L185-196 — `(connection_string: &str) -> String` — Masks passwords in connection strings for safe logging

### examples/features/workflows/python-workflow

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/python-workflow/run_pipeline.py

- pub `check` function L34-40 — `def check(condition: bool, msg: str) -> None`

### examples/features/workflows/registry-execution

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/registry-execution/build.rs

-  `main` function L17-19 — `()`

### examples/features/workflows/registry-execution/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/registry-execution/src/main.rs

-  `main` function L53-273 — `() -> Result<(), Box<dyn std::error::Error>>`
-  `build_package` function L275-287 — `() -> Result<Vec<u8>, Box<dyn std::error::Error>>`
-  `find_workspace_root` function L289-302 — `() -> Result<PathBuf, Box<dyn std::error::Error>>`

### examples/features/workflows/simple-packaged

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/simple-packaged/build.rs

-  `main` function L17-19 — `()`

### examples/features/workflows/simple-packaged/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/simple-packaged/src/lib.rs

- pub `data_processing` module L53-146 — `-`
- pub `collect_data` function L62-77 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
- pub `process_data` function L85-108 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
- pub `generate_report` function L116-145 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>`
-  `tests` module L149-168 — `-`
-  `test_workflow_execution` function L153-167 — `()`

### examples/features/workflows/simple-packaged/tests

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/simple-packaged/tests/ffi_tests.rs

-  `test_workflow_creation_directly` function L25-38 — `()` — Tests for the FFI functions generated by the packaged_workflow macro.
-  `test_get_task_metadata_integration` function L41-64 — `()` — Tests for the FFI functions generated by the packaged_workflow macro.
-  `test_metadata_functions` function L67-82 — `()` — Tests for the FFI functions generated by the packaged_workflow macro.

#### examples/features/workflows/simple-packaged/tests/host_managed_registry_tests.rs

-  `test_get_task_metadata_basic` function L27-56 — `()` — Tests for the new host-managed registry approach using the get_task_metadata() FFI function.
-  `test_get_task_metadata_task_details` function L59-126 — `()` — Tests for the new host-managed registry approach using the get_task_metadata() FFI function.
-  `test_task_metadata_memory_safety` function L129-148 — `()` — Tests for the new host-managed registry approach using the get_task_metadata() FFI function.

### examples/features/workflows/validation-failures

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/validation-failures/build.rs

-  `main` function L17-19 — `()`

### examples/features/workflows/validation-failures/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/workflows/validation-failures/src/circular_dependency.rs

- pub `circular_pipeline` module L25-41 — `-`
- pub `task_a` function L30-33 — `(_context: &mut Context<Value>) -> Result<(), TaskError>`
- pub `task_b` function L37-40 — `(_context: &mut Context<Value>) -> Result<(), TaskError>`
-  `main` function L44-48 — `() -> Result<(), Box<dyn std::error::Error>>`

#### examples/features/workflows/validation-failures/src/duplicate_task_ids.rs

- pub `duplicate_pipeline` module L25-41 — `-`
- pub `task_one` function L30-33 — `(_context: &mut Context<Value>) -> Result<(), TaskError>`
- pub `task_two` function L37-40 — `(_context: &mut Context<Value>) -> Result<(), TaskError>`
-  `main` function L44-48 — `() -> Result<(), Box<dyn std::error::Error>>`

#### examples/features/workflows/validation-failures/src/missing_dependency.rs

- pub `missing_dep_pipeline` module L25-40 — `-`
- pub `valid_task` function L29-32 — `(_context: &mut Context<Value>) -> Result<(), TaskError>`
- pub `invalid_task` function L36-39 — `(_context: &mut Context<Value>) -> Result<(), TaskError>`
-  `main` function L43-47 — `() -> Result<(), Box<dyn std::error::Error>>`

#### examples/features/workflows/validation-failures/src/missing_workflow_task.rs

- pub `failing_pipeline` module L25-40 — `-`
- pub `existing_task` function L29-32 — `(_context: &mut Context<Value>) -> Result<(), TaskError>`
- pub `depends_on_missing` function L36-39 — `(_context: &mut Context<Value>) -> Result<(), TaskError>`
-  `main` function L42-44 — `()`

### examples/performance/computation-graph

> *Semantic summary to be generated by AI agent.*

#### examples/performance/computation-graph/build.rs

-  `main` function L17-19 — `()`

### examples/performance/computation-graph/src

> *Semantic summary to be generated by AI agent.*

#### examples/performance/computation-graph/src/bench.rs

- pub `BenchEvent` struct L44-47 — `{ sequence: u64, value: f64 }` — - Maximum sustained throughput: events/sec before channel backup
- pub `BenchOutput` struct L50-52 — `{ result: f64 }` — - Maximum sustained throughput: events/sec before channel backup
- pub `bench_graph` module L64-74 — `-` — - Maximum sustained throughput: events/sec before channel backup
- pub `process` function L67-69 — `(source: Option<&BenchEvent>) -> f64` — - Maximum sustained throughput: events/sec before channel backup
- pub `output` function L71-73 — `(value: &f64) -> BenchOutput` — - Maximum sustained throughput: events/sec before channel backup
-  `BenchAccumulator` struct L80 — `-` — - Maximum sustained throughput: events/sec before channel backup
-  `BenchAccumulator` type L83-88 — `= BenchAccumulator` — - Maximum sustained throughput: events/sec before channel backup
-  `Output` type L84 — `= BenchEvent` — - Maximum sustained throughput: events/sec before channel backup
-  `process` function L85-87 — `(&mut self, event: Vec<u8>) -> Option<BenchEvent>` — - Maximum sustained throughput: events/sec before channel backup
-  `Args` struct L97-117 — `{ latency_duration: u64, latency_interval_us: u64, throughput_duration: u64, thr...` — - Maximum sustained throughput: events/sec before channel backup
-  `percentile` function L123-129 — `(sorted: &[f64], p: f64) -> f64` — - Maximum sustained throughput: events/sec before channel backup
-  `main` function L136-374 — `()` — - Maximum sustained throughput: events/sec before channel backup

#### examples/performance/computation-graph/src/main.rs

- pub `OrderBookData` struct L80-83 — `{ best_bid: f64, best_ask: f64 }` — bounded memory growth, no persistent channel backup.
- pub `PricingData` struct L86-88 — `{ mid_price: f64 }` — bounded memory growth, no persistent channel backup.
- pub `TradeSignal` struct L91-95 — `{ direction: String, price: f64, confidence: f64 }` — bounded memory growth, no persistent channel backup.
- pub `NoActionReason` struct L98-100 — `{ reason: String }` — bounded memory growth, no persistent channel backup.
- pub `TradeConfirmation` struct L103-106 — `{ executed: bool, message: String }` — bounded memory growth, no persistent channel backup.
- pub `AuditRecord` struct L109-112 — `{ logged: bool, reason: String }` — bounded memory growth, no persistent channel backup.
- pub `market_maker` module L127-190 — `-` — bounded memory growth, no persistent channel backup.
- pub `DecisionOutcome` enum L131-134 — `Trade | NoAction` — bounded memory growth, no persistent channel backup.
- pub `decision` function L136-172 — `( orderbook: Option<&OrderBookData>, pricing: Option<&PricingData>, ) -> Decisio...` — bounded memory growth, no persistent channel backup.
- pub `signal_handler` function L174-182 — `(signal: &TradeSignal) -> TradeConfirmation` — bounded memory growth, no persistent channel backup.
- pub `audit_logger` function L184-189 — `(reason: &NoActionReason) -> AuditRecord` — bounded memory growth, no persistent channel backup.
-  `ALLOCATED` variable L45 — `: AtomicUsize` — bounded memory growth, no persistent channel backup.
-  `TrackingAllocator` struct L47 — `-` — bounded memory growth, no persistent channel backup.
-  `TrackingAllocator` type L49-66 — `impl GlobalAlloc for TrackingAllocator` — bounded memory growth, no persistent channel backup.
-  `alloc` function L50-53 — `(&self, layout: Layout) -> *mut u8` — bounded memory growth, no persistent channel backup.
-  `dealloc` function L55-58 — `(&self, ptr: *mut u8, layout: Layout)` — bounded memory growth, no persistent channel backup.
-  `realloc` function L60-65 — `(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8` — bounded memory growth, no persistent channel backup.
-  `GLOBAL` variable L69 — `: TrackingAllocator` — bounded memory growth, no persistent channel backup.
-  `current_allocated_bytes` function L71-73 — `() -> usize` — bounded memory growth, no persistent channel backup.
-  `OrderBookAccumulator` struct L196 — `-` — bounded memory growth, no persistent channel backup.
-  `OrderBookAccumulator` type L199-204 — `= OrderBookAccumulator` — bounded memory growth, no persistent channel backup.
-  `Output` type L200 — `= OrderBookData` — bounded memory growth, no persistent channel backup.
-  `process` function L201-203 — `(&mut self, event: Vec<u8>) -> Option<OrderBookData>` — bounded memory growth, no persistent channel backup.
-  `PricingAccumulator` struct L206 — `-` — bounded memory growth, no persistent channel backup.
-  `PricingAccumulator` type L209-214 — `= PricingAccumulator` — bounded memory growth, no persistent channel backup.
-  `Output` type L210 — `= PricingData` — bounded memory growth, no persistent channel backup.
-  `process` function L211-213 — `(&mut self, event: Vec<u8>) -> Option<PricingData>` — bounded memory growth, no persistent channel backup.
-  `Args` struct L223-243 — `{ duration: u64, fast_interval_ms: u64, slow_interval_ms: u64, mem_threshold_pct...` — bounded memory growth, no persistent channel backup.
-  `main` function L250-510 — `()` — bounded memory growth, no persistent channel backup.

### examples/performance/parallel

> *Semantic summary to be generated by AI agent.*

#### examples/performance/parallel/build.rs

-  `main` function L17-19 — `()`

### examples/performance/parallel/src

> *Semantic summary to be generated by AI agent.*

#### examples/performance/parallel/src/main.rs

- pub `parallel_workflow` module L45-156 — `-` — Based on tutorial-03, this measures throughput of parallel 5-task fan-out/fan-in workflows.
- pub `setup_data` function L53-59 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Based on tutorial-03, this measures throughput of parallel 5-task fan-out/fan-in workflows.
- pub `process_batch_1` function L66-81 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — Based on tutorial-03, this measures throughput of parallel 5-task fan-out/fan-in workflows.
- pub `process_batch_2` function L88-103 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — Based on tutorial-03, this measures throughput of parallel 5-task fan-out/fan-in workflows.
- pub `process_batch_3` function L110-125 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — Based on tutorial-03, this measures throughput of parallel 5-task fan-out/fan-in workflows.
- pub `merge_results` function L132-155 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Based on tutorial-03, this measures throughput of parallel 5-task fan-out/fan-in workflows.
-  `Args` struct L31-39 — `{ iterations: usize, concurrency: usize }` — Based on tutorial-03, this measures throughput of parallel 5-task fan-out/fan-in workflows.
-  `main` function L159-246 — `() -> Result<(), Box<dyn std::error::Error>>` — Based on tutorial-03, this measures throughput of parallel 5-task fan-out/fan-in workflows.

### examples/performance/pipeline

> *Semantic summary to be generated by AI agent.*

#### examples/performance/pipeline/build.rs

-  `main` function L17-19 — `()`

### examples/performance/pipeline/src

> *Semantic summary to be generated by AI agent.*

#### examples/performance/pipeline/src/main.rs

- pub `etl_workflow` module L45-98 — `-` — Based on tutorial-02, this measures throughput of sequential 3-task pipelines.
- pub `extract_numbers` function L53-59 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — Based on tutorial-02, this measures throughput of sequential 3-task pipelines.
- pub `transform_numbers` function L66-80 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — Based on tutorial-02, this measures throughput of sequential 3-task pipelines.
- pub `load_numbers` function L87-97 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Based on tutorial-02, this measures throughput of sequential 3-task pipelines.
-  `Args` struct L31-39 — `{ iterations: usize, concurrency: usize }` — Based on tutorial-02, this measures throughput of sequential 3-task pipelines.
-  `main` function L101-188 — `() -> Result<(), Box<dyn std::error::Error>>` — Based on tutorial-02, this measures throughput of sequential 3-task pipelines.

### examples/performance/simple

> *Semantic summary to be generated by AI agent.*

#### examples/performance/simple/build.rs

-  `main` function L17-19 — `()`

### examples/performance/simple/src

> *Semantic summary to be generated by AI agent.*

#### examples/performance/simple/src/main.rs

- pub `simple_workflow` module L45-58 — `-` — Based on tutorial-01, this measures throughput of simple single-task workflows.
- pub `hello_world` function L53-57 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Based on tutorial-01, this measures throughput of simple single-task workflows.
-  `Args` struct L31-39 — `{ iterations: usize, concurrency: usize }` — Based on tutorial-01, this measures throughput of simple single-task workflows.
-  `main` function L61-145 — `() -> Result<(), Box<dyn std::error::Error>>` — Based on tutorial-01, this measures throughput of simple single-task workflows.

### examples/tutorials/computation-graphs/library/07-computation-graph

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/computation-graphs/library/07-computation-graph/build.rs

-  `main` function L17-19 — `()`

### examples/tutorials/computation-graphs/library/07-computation-graph/src

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/computation-graphs/library/07-computation-graph/src/main.rs

- pub `OrderBookSnapshot` struct L42-46 — `{ best_bid: f64, best_ask: f64, timestamp: u64 }` — Raw order book snapshot — our input data.
- pub `SpreadSignal` struct L50-53 — `{ spread: f64, mid_price: f64 }` — Computed spread signal — intermediate result.
- pub `FormattedOutput` struct L57-61 — `{ message: String, mid_price: f64, spread_bps: f64 }` — Final formatted output — terminal node result.
- pub `pricing_pipeline` module L82-114 — `-` — - Calling `{module}_compiled(&cache)` and inspecting `GraphResult`
- pub `ingest` function L86-91 — `(orderbook: Option<&OrderBookSnapshot>) -> SpreadSignal` — Entry node: reads the order book from the cache and extracts key fields.
- pub `compute_spread` function L94-101 — `(input: &SpreadSignal) -> SpreadSignal` — Processing node: computes spread in basis points.
- pub `format_output` function L104-113 — `(input: &SpreadSignal) -> FormattedOutput` — Terminal node: formats the result for display.
-  `main` function L124-169 — `()` — - Calling `{module}_compiled(&cache)` and inspecting `GraphResult`

### examples/tutorials/computation-graphs/library/08-accumulators

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/computation-graphs/library/08-accumulators/build.rs

-  `main` function L17-19 — `()`

### examples/tutorials/computation-graphs/library/08-accumulators/src

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/computation-graphs/library/08-accumulators/src/main.rs

- pub `PricingUpdate` struct L51-54 — `{ mid_price: f64, timestamp: u64 }` — - Pushing serialized events and watching the graph fire
- pub `PricingSignal` struct L57-60 — `{ price: f64, change_pct: f64 }` — - Pushing serialized events and watching the graph fire
- pub `SignalOutput` struct L63-65 — `{ message: String }` — - Pushing serialized events and watching the graph fire
- pub `pricing_graph` module L78-106 — `-` — - Pushing serialized events and watching the graph fire
- pub `ingest` function L81-83 — `(pricing: Option<&PricingSignal>) -> PricingSignal` — - Pushing serialized events and watching the graph fire
- pub `analyze` function L85-96 — `(input: &PricingSignal) -> PricingSignal` — - Pushing serialized events and watching the graph fire
- pub `format_signal` function L98-105 — `(input: &PricingSignal) -> SignalOutput` — - Pushing serialized events and watching the graph fire
-  `PricingAccumulator` struct L119 — `-` — - Pushing serialized events and watching the graph fire
-  `PricingAccumulator` type L122-134 — `= PricingAccumulator` — - Pushing serialized events and watching the graph fire
-  `Output` type L123 — `= PricingSignal` — - Pushing serialized events and watching the graph fire
-  `process` function L125-133 — `(&mut self, event: Vec<u8>) -> Option<PricingSignal>` — - Pushing serialized events and watching the graph fire
-  `main` function L150-259 — `()` — - Pushing serialized events and watching the graph fire

### examples/tutorials/computation-graphs/library/09-full-pipeline

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/computation-graphs/library/09-full-pipeline/build.rs

-  `main` function L17-19 — `()`

### examples/tutorials/computation-graphs/library/09-full-pipeline/src

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/computation-graphs/library/09-full-pipeline/src/main.rs

- pub `OrderBookUpdate` struct L50-53 — `{ best_bid: f64, best_ask: f64 }` — - Pushing to different sources and watching the reactor fire each time
- pub `PricingUpdate` struct L56-58 — `{ mid_price: f64 }` — - Pushing to different sources and watching the reactor fire each time
- pub `MarketView` struct L61-65 — `{ spread: f64, mid_price: f64, pricing_mid: f64 }` — - Pushing to different sources and watching the reactor fire each time
- pub `TradingSignal` struct L68-71 — `{ action: String, confidence: f64 }` — - Pushing to different sources and watching the reactor fire each time
- pub `market_pipeline` module L84-130 — `-` — - Pushing to different sources and watching the reactor fire each time
- pub `combine` function L89-104 — `( orderbook: Option<&OrderBookUpdate>, pricing: Option<&PricingUpdate>, ) -> Mar...` — Entry node: combines data from both sources.
- pub `evaluate` function L107-124 — `(view: &MarketView) -> TradingSignal` — Evaluate the combined market view.
- pub `signal` function L127-129 — `(input: &TradingSignal) -> TradingSignal` — Terminal node: formats the signal.
-  `OrderBookAccumulator` struct L136 — `-` — - Pushing to different sources and watching the reactor fire each time
-  `OrderBookAccumulator` type L139-145 — `= OrderBookAccumulator` — - Pushing to different sources and watching the reactor fire each time
-  `Output` type L140 — `= OrderBookUpdate` — - Pushing to different sources and watching the reactor fire each time
-  `process` function L142-144 — `(&mut self, event: Vec<u8>) -> Option<OrderBookUpdate>` — - Pushing to different sources and watching the reactor fire each time
-  `PricingAccumulator` struct L147 — `-` — - Pushing to different sources and watching the reactor fire each time
-  `PricingAccumulator` type L150-156 — `= PricingAccumulator` — - Pushing to different sources and watching the reactor fire each time
-  `Output` type L151 — `= PricingUpdate` — - Pushing to different sources and watching the reactor fire each time
-  `process` function L153-155 — `(&mut self, event: Vec<u8>) -> Option<PricingUpdate>` — - Pushing to different sources and watching the reactor fire each time
-  `main` function L163-302 — `()` — - Pushing to different sources and watching the reactor fire each time

### examples/tutorials/computation-graphs/library/10-routing

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/computation-graphs/library/10-routing/build.rs

-  `main` function L17-19 — `()`

### examples/tutorials/computation-graphs/library/10-routing/src

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/computation-graphs/library/10-routing/src/main.rs

- pub `OrderBookData` struct L49-52 — `{ best_bid: f64, best_ask: f64 }` — - How input values determine which path executes
- pub `PricingData` struct L55-57 — `{ mid_price: f64 }` — - How input values determine which path executes
- pub `TradeSignal` struct L65-69 — `{ direction: String, price: f64, confidence: f64 }` — Data carried when the decision is to trade.
- pub `NoActionReason` struct L73-75 — `{ reason: String }` — Data carried when the decision is no action.
- pub `TradeConfirmation` struct L79-82 — `{ executed: bool, message: String }` — Terminal output from the signal handler.
- pub `AuditRecord` struct L86-89 — `{ logged: bool, reason: String }` — Terminal output from the audit logger.
- pub `market_maker` module L113-183 — `-` — - How input values determine which path executes
- pub `DecisionOutcome` enum L118-121 — `Trade | NoAction` — The routing enum.
- pub `decision` function L126-163 — `( orderbook: Option<&OrderBookData>, pricing: Option<&PricingData>, ) -> Decisio...` — Decision engine: evaluates market data and decides whether to trade.
- pub `signal_handler` function L166-174 — `(signal: &TradeSignal) -> TradeConfirmation` — Signal handler: executes the trade (terminal node on Trade path).
- pub `audit_logger` function L177-182 — `(reason: &NoActionReason) -> AuditRecord` — Audit logger: records why no action was taken (terminal on NoAction path).
-  `OrderBookAccumulator` struct L189 — `-` — - How input values determine which path executes
-  `OrderBookAccumulator` type L192-197 — `= OrderBookAccumulator` — - How input values determine which path executes
-  `Output` type L193 — `= OrderBookData` — - How input values determine which path executes
-  `process` function L194-196 — `(&mut self, event: Vec<u8>) -> Option<OrderBookData>` — - How input values determine which path executes
-  `PricingAccumulator` struct L199 — `-` — - How input values determine which path executes
-  `PricingAccumulator` type L202-207 — `= PricingAccumulator` — - How input values determine which path executes
-  `Output` type L203 — `= PricingData` — - How input values determine which path executes
-  `process` function L204-206 — `(&mut self, event: Vec<u8>) -> Option<PricingData>` — - How input values determine which path executes
-  `main` function L210-364 — `()` — - How input values determine which path executes

### examples/tutorials/python/workflows

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/python/workflows/04_error_handling.py

- pub `UnreliableExternalService` class L31-56 — `{ __init__, fetch_data }` — Simulates an external service with configurable failure rates.
- pub `__init__` method L34-36 — `def __init__(self, failure_rate=0.3)`
- pub `fetch_data` method L38-56 — `def fetch_data(self, data_id)` — Fetch data with potential for failure.

#### examples/tutorials/python/workflows/05_cron_scheduling.py

- pub `get_workflow_names` function L112-116 — `def get_workflow_names()` — Get all registered workflow names.
- pub `cron_demo` function L118-169 — `def cron_demo()` — Demonstrate advanced cron scheduling patterns.
- pub `main` function L171-190 — `def main()` — Main tutorial demonstration.

#### examples/tutorials/python/workflows/06_multi_tenancy.py

- pub `TenantManager` class L165-325 — `{ __init__, provision_tenant, create_tenant_runner, get_tenant_runner, execute_f...` — Manages multi-tenant workflow execution.
- pub `__init__` method L168-175 — `def __init__(self, admin_postgres_url: str)` — Initialize with PostgreSQL admin connection URL.
- pub `provision_tenant` method L177-199 — `def provision_tenant(self, tenant_id: str) -> cloaca.TenantCredentials` — Provision a new tenant with dedicated schema and credentials.
- pub `create_tenant_runner` method L201-217 — `def create_tenant_runner(self, tenant_id: str) -> cloaca.DefaultRunner` — Create a tenant-specific runner with schema isolation.
- pub `get_tenant_runner` method L219-221 — `def get_tenant_runner(self, tenant_id: str) -> Optional[cloaca.DefaultRunner]` — Get existing runner for tenant.
- pub `execute_for_tenant` method L223-230 — `def execute_for_tenant(self, tenant_id: str, workflow_name: str, context: cloaca...` — Execute workflow for specific tenant.
- pub `onboard_new_tenant` method L232-262 — `def onboard_new_tenant(self, tenant_id: str, tenant_info: Dict) -> Dict` — Complete onboarding workflow for new tenant.
- pub `process_tenant_data` method L264-292 — `def process_tenant_data(self, tenant_id: str) -> Dict` — Process data for specific tenant.
- pub `remove_tenant` method L294-309 — `def remove_tenant(self, tenant_id: str)` — Remove tenant completely including schema and credentials.
- pub `cleanup_tenant_resources` method L311-317 — `def cleanup_tenant_resources(self, tenant_id: str)` — Clean up runtime resources for tenant (keeps schema).
- pub `shutdown_all` method L319-325 — `def shutdown_all(self)` — Shutdown all tenant runners.
- pub `simulate_multi_tenant_operations` function L328-436 — `def simulate_multi_tenant_operations()` — Simulate multi-tenant SaaS operations.

#### examples/tutorials/python/workflows/07_event_triggers.py

- pub `on_task_success` function L23-25 — `def on_task_success(task_id, context)` — Callback called when a task completes successfully.
- pub `on_task_failure` function L28-30 — `def on_task_failure(task_id, error, context)` — Callback called when a task fails.
- pub `demo_callbacks` function L136-155 — `def demo_callbacks()` — Demonstrate task callbacks.
- pub `demo_trigger_definition` function L158-191 — `def demo_trigger_definition()` — Demonstrate trigger definition and TriggerResult usage.
- pub `demo_trigger_management` function L194-219 — `def demo_trigger_management()` — Demonstrate trigger management through Python API.
- pub `demo_concepts` function L222-254 — `def demo_concepts()` — Explain key concepts.
- pub `main` function L257-284 — `def main()` — Main tutorial demonstration.

#### examples/tutorials/python/workflows/08_packaged_triggers.py

- pub `demo_trigger_polls` function L98-112 — `def demo_trigger_polls()` — Show how trigger polling works.
- pub `demo_workflow_execution` function L115-139 — `def demo_workflow_execution()` — Run the workflow as if triggered.
- pub `demo_manifest_explanation` function L142-183 — `def demo_manifest_explanation()` — Explain the ManifestV2 trigger fields.
- pub `main` function L186-205 — `def main()` — Main tutorial.

### examples/tutorials/workflows/library/01-basic-workflow

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/workflows/library/01-basic-workflow/build.rs

-  `main` function L17-19 — `()`

### examples/tutorials/workflows/library/01-basic-workflow/src

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/workflows/library/01-basic-workflow/src/main.rs

- pub `simple_workflow` module L32-47 — `-` — This example demonstrates the most basic usage of Cloacina with a single task.
- pub `hello_world` function L40-46 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — This example demonstrates the most basic usage of Cloacina with a single task.
-  `main` function L50-91 — `() -> Result<(), Box<dyn std::error::Error>>` — This example demonstrates the most basic usage of Cloacina with a single task.

### examples/tutorials/workflows/library/02-multi-task

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/workflows/library/02-multi-task/build.rs

-  `main` function L17-19 — `()`

### examples/tutorials/workflows/library/02-multi-task/src

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/workflows/library/02-multi-task/src/main.rs

-  `tasks` module L49 — `-` — - Different retry policies for different task types
-  `main` function L52-100 — `() -> Result<(), Box<dyn std::error::Error>>` — - Different retry policies for different task types

#### examples/tutorials/workflows/library/02-multi-task/src/tasks.rs

- pub `etl_workflow` module L32-130 — `-` — - Load: Store the transformed numbers
- pub `extract_numbers` function L43-62 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — - Load: Store the transformed numbers
- pub `transform_numbers` function L72-98 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — - Load: Store the transformed numbers
- pub `load_numbers` function L108-129 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — - Load: Store the transformed numbers

### examples/tutorials/workflows/library/03-dependencies

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/workflows/library/03-dependencies/build.rs

-  `main` function L17-19 — `()`

### examples/tutorials/workflows/library/03-dependencies/src

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/workflows/library/03-dependencies/src/main.rs

- pub `parallel_processing` module L76-551 — `-` — - **Final Convergence**: All processing completes before cleanup
- pub `generate_data` function L85-107 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - **Final Convergence**: All processing completes before cleanup
- pub `partition_data` function L115-148 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - **Final Convergence**: All processing completes before cleanup
- pub `process_partition_1` function L157-206 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — - **Final Convergence**: All processing completes before cleanup
- pub `process_partition_2` function L215-264 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — - **Final Convergence**: All processing completes before cleanup
- pub `process_partition_3` function L273-322 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — - **Final Convergence**: All processing completes before cleanup
- pub `combine_results` function L330-458 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — - **Final Convergence**: All processing completes before cleanup
- pub `generate_report` function L466-501 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — - **Final Convergence**: All processing completes before cleanup
- pub `send_notifications` function L509-539 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — - **Final Convergence**: All processing completes before cleanup
- pub `cleanup` function L547-550 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - **Final Convergence**: All processing completes before cleanup
-  `Product` struct L57-63 — `{ id: u32, name: String, category: String, price: f64, stock: u32 }` — - **Final Convergence**: All processing completes before cleanup
-  `CategoryStats` struct L66-70 — `{ total_value: f64, total_stock: u32, product_count: u32 }` — - **Final Convergence**: All processing completes before cleanup
-  `main` function L554-584 — `() -> Result<(), Box<dyn std::error::Error>>` — - **Final Convergence**: All processing completes before cleanup

### examples/tutorials/workflows/library/04-error-handling

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/workflows/library/04-error-handling/build.rs

-  `main` function L17-19 — `()`

### examples/tutorials/workflows/library/04-error-handling/src

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/workflows/library/04-error-handling/src/main.rs

- pub `resilient_pipeline` module L92-352 — `-` — - Monitoring task execution outcomes
- pub `fetch_data` function L105-138 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Monitoring task execution outcomes
- pub `cached_data` function L146-166 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Monitoring task execution outcomes
- pub `process_data` function L176-210 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Monitoring task execution outcomes
- pub `high_quality_processing` function L221-250 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — - Monitoring task execution outcomes
- pub `low_quality_processing` function L261-290 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — - Monitoring task execution outcomes
- pub `failure_notification` function L301-317 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — - Monitoring task execution outcomes
- pub `final_report` function L330-351 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Monitoring task execution outcomes
-  `on_task_success` function L44-54 — `( task_id: &str, _context: &Context<serde_json::Value>, ) -> Result<(), Box<dyn ...` — Called when a task completes successfully.
-  `on_task_failure` function L58-72 — `( task_id: &str, error: &cloacina::cloacina_workflow::TaskError, _context: &Cont...` — Called when a task fails (after all retries are exhausted).
-  `on_data_fetch_failure` function L75-86 — `( task_id: &str, error: &cloacina::cloacina_workflow::TaskError, _context: &Cont...` — Specific callback for critical data operations
-  `main` function L355-424 — `() -> Result<(), Box<dyn std::error::Error>>` — - Monitoring task execution outcomes

### examples/tutorials/workflows/library/05-advanced

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/workflows/library/05-advanced/build.rs

-  `main` function L17-19 — `()`

### examples/tutorials/workflows/library/05-advanced/src

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/workflows/library/05-advanced/src/main.rs

-  `tasks` module L46 — `-` — - Recovery service for missed executions
-  `main` function L49-109 — `() -> Result<(), Box<dyn std::error::Error>>` — - Recovery service for missed executions
-  `create_cron_schedules` function L112-153 — `(runner: &DefaultRunner) -> Result<(), Box<dyn std::error::Error>>` — Create cron schedules for our workflows
-  `show_execution_stats` function L156-168 — `(runner: &DefaultRunner) -> Result<(), Box<dyn std::error::Error>>` — Display execution statistics

#### examples/tutorials/workflows/library/05-advanced/src/tasks.rs

- pub `data_backup_workflow` module L34-143 — `-` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `check_backup_prerequisites` function L45-58 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `create_backup_snapshot` function L68-83 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `verify_backup_integrity` function L93-121 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `cleanup_old_backups` function L131-142 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `health_check_workflow` module L153-323 — `-` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `check_system_resources` function L164-191 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `check_database_connectivity` function L201-227 — `( context: &mut Context<Value>, ) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `check_external_services` function L237-268 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `update_health_metrics` function L278-322 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `daily_report_workflow` module L333-446 — `-` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `collect_daily_metrics` function L344-363 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `generate_usage_report` function L373-405 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `send_report_notification` function L415-445 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.

### examples/tutorials/workflows/library/06-multi-tenancy

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/workflows/library/06-multi-tenancy/build.rs

-  `main` function L17-19 — `()`

### examples/tutorials/workflows/library/06-multi-tenancy/src

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/workflows/library/06-multi-tenancy/src/main.rs

- pub `customer_processing` module L35-79 — `-` — using PostgreSQL schema-based multi-tenancy and the Database Admin API.
- pub `process_customer_data` function L42-78 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — using PostgreSQL schema-based multi-tenancy and the Database Admin API.
- pub `tenant_onboarding_workflow` module L85-144 — `-` — using PostgreSQL schema-based multi-tenancy and the Database Admin API.
- pub `tenant_onboarding` function L92-143 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — using PostgreSQL schema-based multi-tenancy and the Database Admin API.
-  `main` function L147-175 — `() -> Result<(), Box<dyn std::error::Error>>` — using PostgreSQL schema-based multi-tenancy and the Database Admin API.
-  `basic_multi_tenant_demo` function L177-229 — `(database_url: &str) -> Result<(), Box<dyn std::error::Error>>` — using PostgreSQL schema-based multi-tenancy and the Database Admin API.
-  `advanced_admin_demo` function L231-291 — `(admin_database_url: &str) -> Result<(), Box<dyn std::error::Error>>` — using PostgreSQL schema-based multi-tenancy and the Database Admin API.

### tests/python

> *Semantic summary to be generated by AI agent.*

#### tests/python/conftest.py

- pub `get_test_db_url` function L29-42 — `def get_test_db_url()` — Get appropriate database URL based on CLOACA_BACKEND env var.
- pub `pytest_sessionfinish` function L186-198 — `def pytest_sessionfinish(session, exitstatus)` — Final cleanup at session end.

#### tests/python/test_scenario_01_basic_api.py

- pub `TestBasicImports` class L13-44 — `{ test_import_cloaca_successfully, test_hello_world_function, test_core_classes_...` — Test that we can import and use basic Cloaca functionality.
- pub `test_import_cloaca_successfully` method L16-22 — `def test_import_cloaca_successfully(self)` — Test that cloaca module imports without errors.
- pub `test_hello_world_function` method L24-30 — `def test_hello_world_function(self)` — Test the hello_world function returns expected output.
- pub `test_core_classes_available` method L32-44 — `def test_core_classes_available(self)` — Test that core classes are importable.
- pub `TestContextOperations` class L47-222 — `{ test_empty_context_creation, test_context_creation_with_data, test_context_bas...` — Test Context class functionality without database operations.
- pub `test_empty_context_creation` method L50-57 — `def test_empty_context_creation(self)` — Test creating empty context.
- pub `test_context_creation_with_data` method L59-83 — `def test_context_creation_with_data(self)` — Test creating context with initial data.
- pub `test_context_basic_operations` method L85-105 — `def test_context_basic_operations(self)` — Test basic get/set/contains operations.
- pub `test_context_insert_and_update` method L107-127 — `def test_context_insert_and_update(self)` — Test insert and update operations with error handling.
- pub `test_context_remove_and_delete` method L129-155 — `def test_context_remove_and_delete(self)` — Test remove and delete operations.
- pub `test_context_serialization` method L157-191 — `def test_context_serialization(self)` — Test JSON serialization and deserialization.
- pub `test_context_dict_conversion` method L193-212 — `def test_context_dict_conversion(self)` — Test to_dict and update_from_dict operations.
- pub `test_context_string_representation` method L214-222 — `def test_context_string_representation(self)` — Test context string representation.
- pub `TestTaskDecorator` class L225-365 — `{ test_basic_task_decorator, test_task_decorator_with_dependencies, test_task_de...` — Test @task decorator functionality without execution.
- pub `test_basic_task_decorator` method L228-245 — `def test_basic_task_decorator(self)` — Test basic task decorator usage.
- pub `test_task_decorator_with_dependencies` method L247-274 — `def test_task_decorator_with_dependencies(self)` — Test task decorator with dependency specification.
- pub `test_task_decorator_with_retry_policy` method L276-300 — `def test_task_decorator_with_retry_policy(self)` — Test task decorator with comprehensive retry configuration.
- pub `test_task_decorator_auto_id` method L302-318 — `def test_task_decorator_auto_id(self)` — Test task decorator with automatic ID generation.
- pub `test_task_decorator_function_references` method L320-347 — `def test_task_decorator_function_references(self)` — Test using function references in dependencies.
- pub `test_task_decorator_return_none` method L349-365 — `def test_task_decorator_return_none(self)` — Test task that returns None (success case).
- pub `TestWorkflowBuilder` class L368-567 — `{ test_basic_workflow_builder_creation, test_workflow_builder_with_tasks, test_w...` — Test WorkflowBuilder functionality without execution.
- pub `test_basic_workflow_builder_creation` method L371-397 — `def test_basic_workflow_builder_creation(self)` — Test creating WorkflowBuilder with basic configuration.
- pub `test_workflow_builder_with_tasks` method L399-431 — `def test_workflow_builder_with_tasks(self)` — Test building workflow with registered tasks.
- pub `test_workflow_builder_function_references` method L433-457 — `def test_workflow_builder_function_references(self)` — Test adding tasks using function references.
- pub `test_workflow_builder_error_handling` method L459-473 — `def test_workflow_builder_error_handling(self)` — Test error handling in WorkflowBuilder.
- pub `test_workflow_validation` method L475-495 — `def test_workflow_validation(self)` — Test workflow validation functionality.
- pub `test_workflow_properties` method L497-531 — `def test_workflow_properties(self)` — Test workflow property access and methods.
- pub `test_workflow_version_consistency` method L533-567 — `def test_workflow_version_consistency(self)` — Test that identical workflows have identical versions.
- pub `TestDefaultRunnerConfig` class L570-676 — `{ test_config_creation_with_defaults, test_config_creation_with_custom_values, t...` — Test DefaultRunnerConfig functionality.
- pub `test_config_creation_with_defaults` method L573-589 — `def test_config_creation_with_defaults(self)` — Test creating config with default values.
- pub `test_config_creation_with_custom_values` method L591-608 — `def test_config_creation_with_custom_values(self)` — Test creating config with custom values.
- pub `test_config_property_access` method L610-637 — `def test_config_property_access(self)` — Test all config property getters and setters.
- pub `test_config_to_dict` method L639-653 — `def test_config_to_dict(self)` — Test config dictionary conversion.
- pub `test_config_static_default_method` method L655-665 — `def test_config_static_default_method(self)` — Test static default method.
- pub `test_config_string_representation` method L667-676 — `def test_config_string_representation(self)` — Test config string representation.
- pub `TestWorkflowContextManager` class L679-729 — `{ test_basic_workflow_context_manager, test_register_workflow_constructor }` — Test workflow context manager functionality.
- pub `test_basic_workflow_context_manager` method L682-706 — `def test_basic_workflow_context_manager(self)` — Test basic workflow context manager usage.
- pub `test_register_workflow_constructor` method L708-729 — `def test_register_workflow_constructor(self)` — Test manual workflow constructor registration.
- pub `TestHelloClass` class L732-749 — `{ test_hello_class_creation }` — Test HelloClass functionality.
- pub `test_hello_class_creation` method L735-749 — `def test_hello_class_creation(self)` — Test HelloClass creation and basic functionality.

#### tests/python/test_scenario_02_single_task_workflow_execution.py

- pub `TestSingleTaskWorkflowExecution` class L12-40 — `{ test_task_with_context_manipulation }` — Test basic single task workflow execution.
- pub `test_task_with_context_manipulation` method L15-40 — `def test_task_with_context_manipulation(self, shared_runner)` — Test task that manipulates context data.

#### tests/python/test_scenario_03_function_based_dag_topology.py

- pub `TestFunctionBasedDAGTopology` class L12-181 — `{ test_comprehensive_dag_topology_patterns }` — Test function-based DAG topology features.
- pub `test_comprehensive_dag_topology_patterns` method L15-181 — `def test_comprehensive_dag_topology_patterns(self, shared_runner)` — Test comprehensive DAG topology patterns and task relationship approaches.

#### tests/python/test_scenario_08_multi_task_workflow_execution.py

- pub `TestMultiTaskWorkflowExecution` class L13-90 — `{ test_comprehensive_multi_pattern_workflow }` — Test comprehensive multi-task workflow with complex dependencies.
- pub `test_comprehensive_multi_pattern_workflow` method L16-90 — `def test_comprehensive_multi_pattern_workflow(self, shared_runner)` — Test a comprehensive workflow combining sequential, parallel, and diamond patterns.

#### tests/python/test_scenario_09_context_propagation.py

- pub `TestContextPropagation` class L12-50 — `{ test_data_flow_through_pipeline }` — Test context data flow between tasks.
- pub `test_data_flow_through_pipeline` method L15-50 — `def test_data_flow_through_pipeline(self, shared_runner)` — Test data flowing through a pipeline of tasks.

#### tests/python/test_scenario_10_workflow_error_handling.py

- pub `TestErrorHandling` class L12-35 — `{ test_task_success_workflow_completion }` — Test error handling and recovery mechanisms.
- pub `test_task_success_workflow_completion` method L15-35 — `def test_task_success_workflow_completion(self, shared_runner)` — Test successful task execution leads to workflow completion.

#### tests/python/test_scenario_11_retry_mechanisms.py

- pub `TestRetryMechanisms` class L12-38 — `{ test_task_with_retry_policy }` — Test configurable retry policies.
- pub `test_task_with_retry_policy` method L15-38 — `def test_task_with_retry_policy(self, shared_runner)` — Test task with retry configuration executes successfully.

#### tests/python/test_scenario_12_workflow_performance.py

- pub `TestPerformanceCharacteristics` class L13-80 — `{ test_comprehensive_workflow_performance }` — Test comprehensive performance and timing characteristics.
- pub `test_comprehensive_workflow_performance` method L16-80 — `def test_comprehensive_workflow_performance(self, shared_runner)` — Test comprehensive performance including timing and multiple executions.

#### tests/python/test_scenario_13_complex_dependency_chains.py

- pub `TestComplexDependencyChains` class L12-292 — `{ test_comprehensive_complex_dependency_patterns }` — Test complex dependency chain patterns.
- pub `test_comprehensive_complex_dependency_patterns` method L15-292 — `def test_comprehensive_complex_dependency_patterns(self, shared_runner)` — Test comprehensive complex dependency chain patterns including diamond, fan-out, fan-in, and multi-level chains.

#### tests/python/test_scenario_14_trigger_rules.py

- pub `TestTriggerRules` class L12-233 — `{ test_comprehensive_trigger_rule_patterns }` — Test various trigger rule configurations.
- pub `test_comprehensive_trigger_rule_patterns` method L15-233 — `def test_comprehensive_trigger_rule_patterns(self, shared_runner)` — Test comprehensive trigger rule patterns including all_success, all_failed, one_success, one_failed, and none_failed.

#### tests/python/test_scenario_15_workflow_versioning.py

- pub `TestWorkflowVersioning` class L12-112 — `{ test_comprehensive_workflow_versioning }` — Test workflow versioning functionality.
- pub `test_comprehensive_workflow_versioning` method L15-112 — `def test_comprehensive_workflow_versioning(self, shared_runner)` — Test comprehensive workflow versioning including content-based hashing and version stability.

#### tests/python/test_scenario_16_registry_management.py

- pub `TestRegistryManagement` class L12-180 — `{ test_comprehensive_registry_management }` — Test registry management and isolation.
- pub `test_comprehensive_registry_management` method L15-180 — `def test_comprehensive_registry_management(self, shared_runner)` — Test comprehensive registry management including isolation, cleanup, and state verification.

#### tests/python/test_scenario_17_advanced_error_handling.py

- pub `TestAdvancedErrorHandling` class L13-161 — `{ test_comprehensive_error_validation }` — Test advanced error handling scenarios.
- pub `test_comprehensive_error_validation` method L16-161 — `def test_comprehensive_error_validation(self, shared_runner)` — Test comprehensive error handling including validation and execution errors.

#### tests/python/test_scenario_18_basic_shared_runner_functionality.py

- pub `TestBasicSharedRunnerFunctionality` class L12-35 — `{ test_basic_shared_runner_execution }` — Test basic shared runner functionality.
- pub `test_basic_shared_runner_execution` method L15-35 — `def test_basic_shared_runner_execution(self, shared_runner)` — Verify runner can execute a simple workflow.

#### tests/python/test_scenario_19_context_passing_runner.py

- pub `TestContextPassingRunner` class L12-56 — `{ test_context_data_flow_through_runner }` — Test context passing through shared runner.
- pub `test_context_data_flow_through_runner` method L15-56 — `def test_context_data_flow_through_runner(self, shared_runner)` — Ensure context data flows correctly through execution.

#### tests/python/test_scenario_20_multiple_workflow_execution_runner.py

- pub `TestMultipleWorkflowExecutionRunner` class L12-69 — `{ test_sequential_workflow_runs }` — Test multiple workflow execution in sequence.
- pub `test_sequential_workflow_runs` method L15-69 — `def test_sequential_workflow_runs(self, shared_runner)` — Run several workflows in sequence with shared runner.

#### tests/python/test_scenario_21_success_validation_runner.py

- pub `TestSuccessValidationRunner` class L12-62 — `{ test_workflow_success_status_reporting }` — Test success validation and status reporting.
- pub `test_workflow_success_status_reporting` method L15-62 — `def test_workflow_success_status_reporting(self, shared_runner)` — Verify expected outcomes and status reporting for successful workflows.

#### tests/python/test_scenario_22_simple_workflow_context_manager.py

- pub `TestSimpleWorkflowContextManager` class L12-38 — `{ test_workflow_context_manager_pattern }` — Test simple workflow creation with context manager.
- pub `test_workflow_context_manager_pattern` method L15-38 — `def test_workflow_context_manager_pattern(self, shared_runner)` — Test basic workflow creation and registration with context manager.

#### tests/python/test_scenario_23_multi_task_workflow_dependencies_builder.py

- pub `TestMultiTaskWorkflowDependenciesBuilder` class L12-55 — `{ test_complex_workflow_builder_pattern }` — Test multi-task workflow construction with dependencies.
- pub `test_complex_workflow_builder_pattern` method L15-55 — `def test_complex_workflow_builder_pattern(self, shared_runner)` — Test complex workflow construction with builder pattern.

#### tests/python/test_scenario_24_parameterized_workflows.py

- pub `TestParameterizedWorkflows` class L12-46 — `{ test_parameterized_workflow_construction }` — Test workflows with configurable parameters.
- pub `test_parameterized_workflow_construction` method L15-46 — `def test_parameterized_workflow_construction(self, shared_runner)` — Test workflows with configurable parameters.

#### tests/python/test_scenario_25_async_task_support.py

- pub `TestAsyncTaskSupport` class L12-47 — `{ test_async_task_workflow }` — Test workflows with asynchronous task functions.
- pub `test_async_task_workflow` method L15-47 — `def test_async_task_workflow(self, shared_runner)` — Test workflows with asynchronous task functions.

#### tests/python/test_scenario_26_simple_workflow_execution.py

- pub `TestSimpleWorkflowExecution` class L12-40 — `{ test_simple_workflow_execution }` — Test the simplest possible workflow execution.
- pub `test_simple_workflow_execution` method L15-40 — `def test_simple_workflow_execution(self, shared_runner)` — Test executing a simple workflow with one task.

#### tests/python/test_scenario_27_cron_scheduling.py

- pub `TestCronScheduling` class L13-196 — `{ test_comprehensive_cron_scheduling }` — Test comprehensive cron scheduling functionality.
- pub `test_comprehensive_cron_scheduling` method L16-196 — `def test_comprehensive_cron_scheduling(self, shared_runner)` — Test comprehensive cron scheduling including CRUD operations and monitoring.

#### tests/python/test_scenario_28_multi_tenancy.py

- pub `TestMultiTenancyBasics` class L28-68 — `{ test_with_schema_method_exists, test_schema_validation_empty_name, test_schema...` — Test basic multi-tenancy functionality.
- pub `test_with_schema_method_exists` method L31-35 — `def test_with_schema_method_exists(self)` — Test that with_schema method is available.
- pub `test_schema_validation_empty_name` method L37-40 — `def test_schema_validation_empty_name(self)` — Test that empty schema names are rejected.
- pub `test_schema_validation_invalid_characters` method L42-54 — `def test_schema_validation_invalid_characters(self)` — Test that invalid schema names are rejected.
- pub `test_schema_validation_valid_names_with_connection_error` method L56-68 — `def test_schema_validation_valid_names_with_connection_error(self)` — Test that valid schema names are accepted but connection fails gracefully.
- pub `TestPostgreSQLMultiTenancy` class L71-96 — `{ test_create_tenant_runners_with_connection_error, test_different_schema_names ...` — Test PostgreSQL-specific multi-tenancy features.
- pub `test_create_tenant_runners_with_connection_error` method L74-84 — `def test_create_tenant_runners_with_connection_error(self)` — Test creating multiple tenant runners fails gracefully with bad connection.
- pub `test_different_schema_names` method L86-96 — `def test_different_schema_names(self)` — Test that different schema names are accepted.
- pub `TestMultiTenancyAPI` class L99-136 — `{ test_api_signature, test_method_is_static, test_basic_usage_pattern }` — Test multi-tenancy API patterns.
- pub `test_api_signature` method L102-111 — `def test_api_signature(self)` — Test that the API follows expected patterns.
- pub `test_method_is_static` method L113-118 — `def test_method_is_static(self)` — Test that method is properly static.
- pub `test_basic_usage_pattern` method L120-136 — `def test_basic_usage_pattern(self)` — Test that usage examples work as expected.
- pub `TestMultiTenancyIntegration` class L139-165 — `{ test_tenant_workflow_concepts, test_tenant_cron_concepts }` — Test multi-tenancy integration concepts.
- pub `test_tenant_workflow_concepts` method L142-156 — `def test_tenant_workflow_concepts(self)` — Test that multi-tenant concepts work with workflow system.
- pub `test_tenant_cron_concepts` method L158-165 — `def test_tenant_cron_concepts(self)` — Test that multi-tenant concepts work with cron system.
- pub `TestMultiTenancyDocumentation` class L168-212 — `{ test_documented_patterns, test_error_messages_are_helpful }` — Verify multi-tenancy usage patterns work as documented.
- pub `test_documented_patterns` method L171-192 — `def test_documented_patterns(self)` — Test patterns that would be shown in documentation.
- pub `test_error_messages_are_helpful` method L194-212 — `def test_error_messages_are_helpful(self)` — Test that error messages provide useful information.

#### tests/python/test_scenario_29_event_triggers.py

- pub `TestEventTriggers` class L11-147 — `{ test_trigger_result_skip, test_trigger_result_fire_no_context, test_trigger_re...` — Test event trigger functionality.
- pub `test_trigger_result_skip` method L14-22 — `def test_trigger_result_skip(self, shared_runner)` — Test TriggerResult.skip() creation.
- pub `test_trigger_result_fire_no_context` method L24-32 — `def test_trigger_result_fire_no_context(self, shared_runner)` — Test TriggerResult.fire() without context.
- pub `test_trigger_result_fire_with_context` method L34-42 — `def test_trigger_result_fire_with_context(self, shared_runner)` — Test TriggerResult.fire() with context.
- pub `test_trigger_decorator_registration` method L44-78 — `def test_trigger_decorator_registration(self, shared_runner)` — Test that @trigger decorator registers triggers correctly.
- pub `test_trigger_with_counter` method L80-114 — `def test_trigger_with_counter(self, shared_runner)` — Test trigger that fires after N polls.
- pub `test_list_trigger_schedules` method L116-121 — `def test_list_trigger_schedules(self, shared_runner)` — Test listing trigger schedules.
- pub `test_list_trigger_schedules_with_filters` method L123-133 — `def test_list_trigger_schedules_with_filters(self, shared_runner)` — Test listing trigger schedules with filtering options.
- pub `test_get_nonexistent_trigger_schedule` method L135-140 — `def test_get_nonexistent_trigger_schedule(self, shared_runner)` — Test getting a trigger schedule that doesn't exist.
- pub `test_get_trigger_execution_history` method L142-147 — `def test_get_trigger_execution_history(self, shared_runner)` — Test getting execution history for a trigger.

#### tests/python/test_scenario_30_task_callbacks.py

- pub `TestTaskCallbacks` class L9-180 — `{ test_on_success_callback_called, test_on_failure_callback_called, test_both_ca...` — Test task callback functionality.
- pub `test_on_success_callback_called` method L12-35 — `def test_on_success_callback_called(self, shared_runner)` — Test that on_success callback is called on successful task completion.
- pub `test_on_failure_callback_called` method L37-68 — `def test_on_failure_callback_called(self, shared_runner)` — Test that on_failure callback is called on task failure.
- pub `test_both_callbacks_on_same_task` method L70-99 — `def test_both_callbacks_on_same_task(self, shared_runner)` — Test that both callbacks can be set on the same task.
- pub `test_callback_error_isolation` method L101-121 — `def test_callback_error_isolation(self, shared_runner)` — Test that errors in callbacks don't fail the task.
- pub `test_callback_receives_correct_context` method L123-147 — `def test_callback_receives_correct_context(self, shared_runner)` — Test that callbacks receive the correct context data.
- pub `test_callbacks_with_dependencies` method L149-180 — `def test_callbacks_with_dependencies(self, shared_runner)` — Test callbacks work correctly with task dependencies.

#### tests/python/test_scenario_31_task_handle.py

- pub `TestTaskHandleDetection` class L14-61 — `{ test_task_without_handle_is_callable, test_task_with_handle_param_is_callable,...` — Test that the @task decorator correctly detects handle parameters.
- pub `test_task_without_handle_is_callable` method L17-30 — `def test_task_without_handle_is_callable(self)` — A normal task (no handle param) should work as before.
- pub `test_task_with_handle_param_is_callable` method L32-46 — `def test_task_with_handle_param_is_callable(self)` — A task with handle param should still be callable as a plain function.
- pub `test_task_with_task_handle_param` method L48-61 — `def test_task_with_task_handle_param(self)` — A task with task_handle param (alternate name) should be detected.
- pub `TestTaskHandleClass` class L64-80 — `{ test_task_handle_is_importable, test_task_handle_has_defer_until, test_task_ha...` — Test that TaskHandle is importable and has expected attributes.
- pub `test_task_handle_is_importable` method L67-70 — `def test_task_handle_is_importable(self)` — TaskHandle class should be importable from cloaca.
- pub `test_task_handle_has_defer_until` method L72-75 — `def test_task_handle_has_defer_until(self)` — TaskHandle should have a defer_until method.
- pub `test_task_handle_has_is_slot_held` method L77-80 — `def test_task_handle_has_is_slot_held(self)` — TaskHandle should have an is_slot_held method.
- pub `TestTaskHandleExecution` class L83-174 — `{ test_deferred_task_completes, test_deferred_task_chains_with_downstream, test_...` — Test TaskHandle.defer_until through the executor pipeline.
- pub `test_deferred_task_completes` method L86-113 — `def test_deferred_task_completes(self, shared_runner)` — A task using defer_until should complete successfully.
- pub `test_deferred_task_chains_with_downstream` method L115-147 — `def test_deferred_task_chains_with_downstream(self, shared_runner)` — A deferred task should correctly chain with a downstream task.
- pub `test_non_handle_task_alongside_handle_task` method L149-174 — `def test_non_handle_task_alongside_handle_task(self, shared_runner)` — Normal tasks and handle tasks should work together in a workflow.

#### tests/python/utilities.py

- pub `FailureRecord` class L14-20 — `-` — Represents a single test failure.
- pub `SectionRecord` class L24-29 — `-` — Represents a section of tests within a scenario.
- pub `ResultsAggregator` class L32-155 — `{ __init__, add_section, add_failure, run_test_section, assert_with_context, sof...` — Aggregates test results and failures for end-of-test reporting.
- pub `__init__` method L35-40 — `def __init__(self, test_name: str)`
- pub `add_section` method L42-55 — `def add_section(self, section_name: str, passed: bool = True, error_message: Opt...` — Add a test section result.
- pub `add_failure` method L57-67 — `def add_failure(self, test_name: str, failure_type: str, error_message: str, con...` — Add a test failure.
- pub `run_test_section` method L69-82 — `def run_test_section(self, section_name: str, test_func, *args, **kwargs)` — Run a test section and capture any failures.
- pub `assert_with_context` method L84-89 — `def assert_with_context(self, condition: bool, message: str, context: Optional[D...` — Assert with context information for better failure reporting.
- pub `soft_assert` method L91-97 — `def soft_assert(self, condition: bool, message: str, context: Optional[Dict[str,...` — Soft assertion that doesn't raise but records failure.
- pub `report_results` method L99-139 — `def report_results(self) -> None` — Report aggregated test results at the end.
- pub `get_success_rate` method L141-145 — `def get_success_rate(self) -> float` — Get the success rate as a percentage.
- pub `has_failures` method L147-149 — `def has_failures(self) -> bool` — Check if there are any failures.
- pub `raise_if_failures` method L151-155 — `def raise_if_failures(self) -> None` — Raise an exception if there are failures (for pytest compatibility).
- pub `create_test_aggregator` function L158-160 — `def create_test_aggregator(test_name: str) -> ResultsAggregator` — Factory function to create a test aggregator.
