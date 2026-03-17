# Code Index

> Generated: 2026-03-17T01:32:07Z | 399 files | JavaScript, Python, Rust

## Project Structure

```
├── bindings/
│   └── cloaca-backend/
│       ├── python/
│       │   └── cloaca/
│       │       ├── __init__.py
│       │       ├── cli/
│       │       │   ├── __init__.py
│       │       │   └── build.py
│       │       ├── discovery.py
│       │       ├── manifest.py
│       │       ├── pyproject.py
│       │       └── vendoring.py
│       └── src/
│           ├── admin.rs
│           ├── context.rs
│           ├── lib.rs
│           ├── runner.rs
│           ├── task.rs
│           ├── trigger.rs
│           ├── value_objects/
│           │   ├── context.rs
│           │   ├── mod.rs
│           │   ├── namespace.rs
│           │   └── retry.rs
│           └── workflow.rs
├── crates/
│   ├── cloacina/
│   │   ├── src/
│   │   │   ├── context.rs
│   │   │   ├── continuous/
│   │   │   │   ├── accumulator.rs
│   │   │   │   ├── boundary.rs
│   │   │   │   ├── connections/
│   │   │   │   │   ├── kafka.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── postgres.rs
│   │   │   │   │   └── s3.rs
│   │   │   │   ├── datasource.rs
│   │   │   │   ├── detector.rs
│   │   │   │   ├── detector_state_store.rs
│   │   │   │   ├── graph.rs
│   │   │   │   ├── ledger.rs
│   │   │   │   ├── ledger_trigger.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── scheduler.rs
│   │   │   │   ├── state_management.rs
│   │   │   │   ├── trigger_policy.rs
│   │   │   │   └── watermark.rs
│   │   │   ├── cron_evaluator.rs
│   │   │   ├── cron_recovery.rs
│   │   │   ├── cron_scheduler.rs
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
│   │   │   │       ├── accumulator_state.rs
│   │   │   │       ├── api_key_dal.rs
│   │   │   │       ├── context.rs
│   │   │   │       ├── cron_execution/
│   │   │   │       │   ├── crud.rs
│   │   │   │       │   ├── mod.rs
│   │   │   │       │   ├── queries.rs
│   │   │   │       │   └── tracking.rs
│   │   │   │       ├── cron_schedule/
│   │   │   │       │   ├── crud.rs
│   │   │   │       │   ├── mod.rs
│   │   │   │       │   ├── queries.rs
│   │   │   │       │   └── state.rs
│   │   │   │       ├── detector_state_dal.rs
│   │   │   │       ├── execution_event.rs
│   │   │   │       ├── mod.rs
│   │   │   │       ├── models.rs
│   │   │   │       ├── pending_boundary_dal.rs
│   │   │   │       ├── pipeline_execution.rs
│   │   │   │       ├── recovery_event.rs
│   │   │   │       ├── task_execution/
│   │   │   │       │   ├── claiming.rs
│   │   │   │       │   ├── crud.rs
│   │   │   │       │   ├── mod.rs
│   │   │   │       │   ├── queries.rs
│   │   │   │       │   ├── recovery.rs
│   │   │   │       │   └── state.rs
│   │   │   │       ├── task_execution_metadata.rs
│   │   │   │       ├── task_outbox.rs
│   │   │   │       ├── tenant_dal.rs
│   │   │   │       ├── trigger_execution/
│   │   │   │       │   ├── crud.rs
│   │   │   │       │   └── mod.rs
│   │   │   │       ├── trigger_schedule/
│   │   │   │       │   ├── crud.rs
│   │   │   │       │   └── mod.rs
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
│   │   │   ├── executor/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── pipeline_executor.rs
│   │   │   │   ├── slot_token.rs
│   │   │   │   ├── task_handle.rs
│   │   │   │   ├── thread_task_executor.rs
│   │   │   │   └── types.rs
│   │   │   ├── graph.rs
│   │   │   ├── lib.rs
│   │   │   ├── logging.rs
│   │   │   ├── models/
│   │   │   │   ├── context.rs
│   │   │   │   ├── cron_execution.rs
│   │   │   │   ├── cron_schedule.rs
│   │   │   │   ├── execution_event.rs
│   │   │   │   ├── key_trust_acl.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── package_signature.rs
│   │   │   │   ├── pipeline_execution.rs
│   │   │   │   ├── recovery_event.rs
│   │   │   │   ├── signing_key.rs
│   │   │   │   ├── task_execution.rs
│   │   │   │   ├── task_execution_metadata.rs
│   │   │   │   ├── task_outbox.rs
│   │   │   │   ├── trigger_execution.rs
│   │   │   │   ├── trigger_schedule.rs
│   │   │   │   ├── trusted_key.rs
│   │   │   │   ├── workflow_packages.rs
│   │   │   │   └── workflow_registry.rs
│   │   │   ├── packaging/
│   │   │   │   ├── archive.rs
│   │   │   │   ├── compile.rs
│   │   │   │   ├── debug.rs
│   │   │   │   ├── manifest.rs
│   │   │   │   ├── manifest_v2.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── platform.rs
│   │   │   │   ├── tests.rs
│   │   │   │   ├── types.rs
│   │   │   │   └── validation.rs
│   │   │   ├── python/
│   │   │   │   ├── executor.rs
│   │   │   │   └── mod.rs
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
│   │   │   │       ├── mod.rs
│   │   │   │       └── package.rs
│   │   │   ├── retry.rs
│   │   │   ├── runner/
│   │   │   │   ├── default_runner/
│   │   │   │   │   ├── config.rs
│   │   │   │   │   ├── cron_api.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── pipeline_executor_impl.rs
│   │   │   │   │   ├── pipeline_result.rs
│   │   │   │   │   └── services.rs
│   │   │   │   └── mod.rs
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
│   │   │   ├── task_scheduler/
│   │   │   │   ├── context_manager.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── recovery.rs
│   │   │   │   ├── scheduler_loop.rs
│   │   │   │   ├── state_manager.rs
│   │   │   │   └── trigger_rules.rs
│   │   │   ├── trigger/
│   │   │   │   ├── mod.rs
│   │   │   │   └── registry.rs
│   │   │   ├── trigger_scheduler.rs
│   │   │   └── workflow/
│   │   │       ├── builder.rs
│   │   │       ├── graph.rs
│   │   │       ├── metadata.rs
│   │   │       ├── mod.rs
│   │   │       └── registry.rs
│   │   └── tests/
│   │       ├── fixtures.rs
│   │       └── integration/
│   │           ├── context.rs
│   │           ├── continuous/
│   │           │   ├── accumulator_persistence.rs
│   │           │   ├── mod.rs
│   │           │   ├── recovery_e2e.rs
│   │           │   └── runner_lifecycle.rs
│   │           ├── dal/
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
│   │           ├── executor/
│   │           │   ├── context_merging.rs
│   │           │   ├── defer_until.rs
│   │           │   ├── mod.rs
│   │           │   ├── multi_tenant.rs
│   │           │   ├── pause_resume.rs
│   │           │   └── task_execution.rs
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
│   │           │   └── trigger_rules.rs
│   │           ├── signing/
│   │           │   ├── key_rotation.rs
│   │           │   ├── mod.rs
│   │           │   ├── security_failures.rs
│   │           │   ├── sign_and_verify.rs
│   │           │   └── trust_chain.rs
│   │           ├── task/
│   │           │   ├── checkpoint.rs
│   │           │   ├── continuous_macro.rs
│   │           │   ├── debug_macro.rs
│   │           │   ├── handle_macro.rs
│   │           │   ├── macro_test.rs
│   │           │   ├── mod.rs
│   │           │   └── simple_macro.rs
│   │           ├── test_registry_dynamic_loading.rs
│   │           ├── test_registry_dynamic_loading_simple.rs
│   │           └── workflow/
│   │               ├── basic.rs
│   │               ├── callback_test.rs
│   │               ├── macro_test.rs
│   │               ├── mod.rs
│   │               └── subgraph.rs
│   ├── cloacina-macros/
│   │   └── src/
│   │       ├── continuous_task.rs
│   │       ├── lib.rs
│   │       ├── packaged_workflow.rs
│   │       ├── registry.rs
│   │       ├── tasks.rs
│   │       └── workflow.rs
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
│   │       └── task.rs
│   └── cloacinactl/
│       ├── build.rs
│       └── src/
│           ├── auth/
│           │   ├── cache.rs
│           │   ├── context.rs
│           │   ├── middleware.rs
│           │   ├── mod.rs
│           │   └── pattern.rs
│           ├── commands/
│           │   ├── api_key.rs
│           │   ├── cleanup_events.rs
│           │   ├── key.rs
│           │   ├── key_trust.rs
│           │   ├── mod.rs
│           │   ├── package.rs
│           │   └── serve.rs
│           ├── config.rs
│           ├── main.rs
│           └── routes/
│               ├── auth_test.rs
│               ├── error.rs
│               ├── executions.rs
│               ├── health.rs
│               ├── mod.rs
│               ├── tenants.rs
│               └── workflows.rs
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
│   │   ├── complex-dag/
│   │   │   └── src/
│   │   │       └── lib.rs
│   │   ├── continuous-scheduling/
│   │   │   └── src/
│   │   │       └── main.rs
│   │   ├── cron-scheduling/
│   │   │   └── src/
│   │   │       ├── main.rs
│   │   │       └── tasks.rs
│   │   ├── deferred-tasks/
│   │   │   └── src/
│   │   │       └── main.rs
│   │   ├── event-triggers/
│   │   │   └── src/
│   │   │       ├── main.rs
│   │   │       ├── tasks.rs
│   │   │       └── triggers.rs
│   │   ├── multi-tenant/
│   │   │   └── src/
│   │   │       └── main.rs
│   │   ├── packaged-workflows/
│   │   │   └── src/
│   │   │       └── lib.rs
│   │   ├── per-tenant-credentials/
│   │   │   └── src/
│   │   │       └── main.rs
│   │   ├── python-workflow/
│   │   │   ├── data_pipeline/
│   │   │   │   ├── __init__.py
│   │   │   │   └── tasks.py
│   │   │   └── run_pipeline.py
│   │   ├── registry-execution/
│   │   │   └── src/
│   │   │       └── main.rs
│   │   ├── simple-packaged/
│   │   │   ├── src/
│   │   │   │   └── lib.rs
│   │   │   └── tests/
│   │   │       ├── ffi_tests.rs
│   │   │       └── host_managed_registry_tests.rs
│   │   └── validation-failures/
│   │       └── src/
│   │           ├── circular_dependency.rs
│   │           ├── duplicate_task_ids.rs
│   │           ├── missing_dependency.rs
│   │           └── missing_workflow_task.rs
│   ├── performance/
│   │   ├── parallel/
│   │   │   └── src/
│   │   │       └── main.rs
│   │   ├── pipeline/
│   │   │   └── src/
│   │   │       └── main.rs
│   │   └── simple/
│   │       └── src/
│   │           └── main.rs
│   └── tutorials/
│       ├── 01-basic-workflow/
│       │   └── src/
│       │       └── main.rs
│       ├── 02-multi-task/
│       │   └── src/
│       │       ├── main.rs
│       │       └── tasks.rs
│       ├── 03-dependencies/
│       │   └── src/
│       │       └── main.rs
│       ├── 04-error-handling/
│       │   └── src/
│       │       └── main.rs
│       ├── 05-advanced/
│       │   └── src/
│       │       ├── main.rs
│       │       └── tasks.rs
│       ├── 06-multi-tenancy/
│       │   └── src/
│       │       └── main.rs
│       └── python/
│           ├── 01_first_workflow.py
│           ├── 02_context_handling.py
│           ├── 03_complex_workflows.py
│           ├── 04_error_handling.py
│           ├── 05_cron_scheduling.py
│           ├── 06_multi_tenancy.py
│           └── 07_event_triggers.py
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

### bindings/cloaca-backend/python/cloaca

> *Semantic summary to be generated by AI agent.*

#### bindings/cloaca-backend/python/cloaca/discovery.py

- pub `discover_tasks` function L29-73 — `def discover_tasks(entry_module: str, project_dir: Path) -> list[TaskDefinition]` — Discover tasks from *entry_module* under *project_dir*.
-  `_resolve_module_path` function L76-85 — `def _resolve_module_path(dotted: str, project_dir: Path) -> Path` — Convert a dotted module path to a filesystem path.
-  `_extract_task_decorator` function L88-111 — `def _extract_task_decorator(func: ast.FunctionDef) -> dict | None` — Return keyword args if *func* has a ``@task`` decorator, else ``None``.
-  `_eval_literal` function L114-119 — `def _eval_literal(node: ast.expr) -> object` — Safely evaluate an AST node as a literal value.

#### bindings/cloaca-backend/python/cloaca/manifest.py

- pub `detect_current_platform` function L46-59 — `def detect_current_platform() -> str` — Return the current platform as a target string.
- pub `TaskDefinition` class L67-75 — `(BaseModel)` — A single task within a workflow package.
- pub `PythonRuntime` class L78-82 — `(BaseModel)` — Python-specific runtime requirements.
- pub `RustRuntime` class L85-88 — `(BaseModel)` — Rust-specific runtime requirements.
- pub `PackageInfo` class L91-98 — `(BaseModel)` — Package metadata.
- pub `Manifest` class L101-172 — `(BaseModel) { validate_targets, validate_tasks, validate_all, to_json, write_to_...` — Unified package manifest (v2).
- pub `validate_targets` method L124-128 — `def validate_targets(self) -> None` — Raise ``ValueError`` if any target is unsupported.
- pub `validate_tasks` method L130-151 — `def validate_tasks(self) -> None` — Raise ``ValueError`` on duplicate IDs, bad deps, or bad function paths.
- pub `validate_all` method L153-156 — `def validate_all(self) -> None` — Run all validation checks.
- pub `to_json` method L160-162 — `def to_json(self, **kwargs: object) -> str` — Serialize to a JSON string.
- pub `write_to_file` method L164-166 — `def write_to_file(self, path: str | Path) -> None` — Write manifest JSON to *path*.

#### bindings/cloaca-backend/python/cloaca/pyproject.py

- pub `parse_pyproject` function L23-49 — `def parse_pyproject(path: Path) -> dict` — Parse *pyproject.toml* and validate required fields for cloaca.

#### bindings/cloaca-backend/python/cloaca/vendoring.py

- pub `VendoringError` class L31-32 — `(Exception)` — Base class for vendoring errors.
- pub `DependencyResolutionError` class L35-36 — `(VendoringError)` — Failed to resolve dependencies.
- pub `SdistOnlyError` class L39-40 — `(VendoringError)` — Package only has source distribution, no wheel.
- pub `DownloadError` class L43-44 — `(VendoringError)` — Failed to download wheel.
- pub `ResolvedDependency` class L52-57 — `-` — A single resolved dependency with version and hash.
- pub `VendorResult` class L61-66 — `-` — Result of the vendoring pipeline.
- pub `check_uv_available` function L87-101 — `def check_uv_available() -> str` — Verify ``uv`` is installed.
- pub `vendor_dependencies` function L104-165 — `def vendor_dependencies( project_dir: Path, vendor_dir: Path, targets: list[str]...` — Run the full vendoring pipeline.
-  `_resolve_dependencies` function L172-202 — `def _resolve_dependencies( pyproject_path: Path, target: str, python_version: st...` — Use ``uv pip compile`` to resolve pinned dependencies.
-  `_parse_lock_file` function L205-249 — `def _parse_lock_file(path: Path) -> list[ResolvedDependency]` — Parse a pip-compile style requirements file into resolved deps.
-  `_download_wheels` function L252-288 — `def _download_wheels( deps: list[ResolvedDependency], target: str, python_versio...` — Download wheels using ``uv pip download``.
-  `_extract_wheels` function L291-295 — `def _extract_wheels(wheels: list[Path], vendor_dir: Path) -> None` — Extract wheel contents into *vendor_dir*.
-  `_generate_lock_file` function L298-314 — `def _generate_lock_file( deps: list[ResolvedDependency], output_path: Path, targ...` — Write a ``requirements.lock`` with pinned versions and hashes.

### bindings/cloaca-backend/python/cloaca/cli

> *Semantic summary to be generated by AI agent.*

#### bindings/cloaca-backend/python/cloaca/cli/build.py

-  `_copy_workflow_source` function L186-200 — `def _copy_workflow_source( project_dir: Path, dest: Path, entry_module: str ) ->...` — Copy the entry module's package tree into *dest*.
-  `_compute_sha256` function L203-208 — `def _compute_sha256(path: Path) -> str`

### bindings/cloaca-backend/src

> *Semantic summary to be generated by AI agent.*

#### bindings/cloaca-backend/src/admin.rs

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

#### bindings/cloaca-backend/src/context.rs

- pub `PyContext` struct L37-39 — `{ inner: cloacina::Context<serde_json::Value> }` — PyContext - Python wrapper for Rust Context<serde_json::Value>
- pub `new` function L52-69 — `(data: Option<&Bound<'_, PyDict>>) -> PyResult<Self>` — Creates a new empty context
- pub `get` function L80-88 — `(&self, key: &str, default: Option<&Bound<'_, PyAny>>) -> PyResult<PyObject>` — Gets a value from the context
- pub `set` function L99-114 — `(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()>` — Sets a value in the context (insert or update)
- pub `update` function L124-129 — `(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()>` — Updates an existing value in the context
- pub `insert` function L139-144 — `(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()>` — Inserts a new value into the context
- pub `remove` function L153-158 — `(&mut self, key: &str) -> PyResult<Option<PyObject>>` — Removes and returns a value from the context
- pub `to_dict` function L164-166 — `(&self, py: Python<'_>) -> PyResult<PyObject>` — Returns the context as a Python dictionary
- pub `update_from_dict` function L172-191 — `(&mut self, data: &Bound<'_, PyDict>) -> PyResult<()>` — Updates the context with values from a Python dictionary
- pub `to_json` function L197-204 — `(&self) -> PyResult<String>` — Serializes the context to a JSON string
- pub `from_json` function L214-222 — `(json_str: &str) -> PyResult<Self>` — Creates a context from a JSON string
- pub `__len__` function L225-227 — `(&self) -> usize` — Returns the number of key-value pairs in the context
- pub `__contains__` function L230-232 — `(&self, key: &str) -> bool` — Checks if a key exists in the context
- pub `__repr__` function L235-240 — `(&self) -> String` — String representation of the context
- pub `__getitem__` function L243-255 — `(&self, key: &str) -> PyResult<PyObject>` — Dictionary-style item access
- pub `__setitem__` function L258-260 — `(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()>` — Dictionary-style item assignment
- pub `__delitem__` function L263-271 — `(&mut self, key: &str) -> PyResult<()>` — Dictionary-style item deletion
- pub `PyDefaultRunnerConfig` struct L333-335 — `{ inner: cloacina::runner::DefaultRunnerConfig }` — PyDefaultRunnerConfig - Python wrapper for Rust DefaultRunnerConfig
- pub `new` function L360-426 — `( max_concurrent_tasks: Option<usize>, scheduler_poll_interval_ms: Option<u64>, ...`
- pub `default` function L433-437 — `() -> Self` — Creates a DefaultRunnerConfig with all default values
- pub `max_concurrent_tasks` function L442-444 — `(&self) -> usize`
- pub `scheduler_poll_interval_ms` function L447-449 — `(&self) -> u64`
- pub `task_timeout_seconds` function L452-454 — `(&self) -> u64`
- pub `pipeline_timeout_seconds` function L457-459 — `(&self) -> Option<u64>`
- pub `db_pool_size` function L462-464 — `(&self) -> u32`
- pub `enable_recovery` function L467-469 — `(&self) -> bool`
- pub `enable_cron_scheduling` function L472-474 — `(&self) -> bool`
- pub `cron_poll_interval_seconds` function L477-479 — `(&self) -> u64`
- pub `cron_max_catchup_executions` function L482-484 — `(&self) -> usize`
- pub `cron_enable_recovery` function L487-489 — `(&self) -> bool`
- pub `cron_recovery_interval_seconds` function L492-494 — `(&self) -> u64`
- pub `cron_lost_threshold_minutes` function L497-499 — `(&self) -> i32`
- pub `cron_max_recovery_age_seconds` function L502-504 — `(&self) -> u64`
- pub `cron_max_recovery_attempts` function L507-509 — `(&self) -> usize`
- pub `set_max_concurrent_tasks` function L515-517 — `(&mut self, value: usize)`
- pub `set_scheduler_poll_interval_ms` function L520-523 — `(&mut self, value: u64)`
- pub `set_task_timeout_seconds` function L526-528 — `(&mut self, value: u64)`
- pub `set_pipeline_timeout_seconds` function L531-534 — `(&mut self, value: Option<u64>)`
- pub `set_db_pool_size` function L537-539 — `(&mut self, value: u32)`
- pub `set_enable_recovery` function L542-544 — `(&mut self, value: bool)`
- pub `set_enable_cron_scheduling` function L547-549 — `(&mut self, value: bool)`
- pub `set_cron_poll_interval_seconds` function L552-554 — `(&mut self, value: u64)`
- pub `set_cron_max_catchup_executions` function L557-559 — `(&mut self, value: usize)`
- pub `set_cron_enable_recovery` function L562-564 — `(&mut self, value: bool)`
- pub `set_cron_recovery_interval_seconds` function L567-570 — `(&mut self, value: u64)`
- pub `set_cron_lost_threshold_minutes` function L573-575 — `(&mut self, value: i32)`
- pub `set_cron_max_recovery_age_seconds` function L578-581 — `(&mut self, value: u64)`
- pub `set_cron_max_recovery_attempts` function L584-586 — `(&mut self, value: usize)`
- pub `to_dict` function L592-638 — `(&self, py: Python<'_>) -> PyResult<PyObject>` — Returns a dictionary representation of the configuration
- pub `__repr__` function L641-648 — `(&self) -> String` — String representation of the configuration
-  `PyContext` type L42-272 — `= PyContext`
-  `PyContext` type L274-294 — `= PyContext`
-  `from_rust_context` function L276-278 — `(context: cloacina::Context<serde_json::Value>) -> Self` — Create a PyContext from a Rust Context (for internal use)
-  `into_inner` function L281-283 — `(self) -> cloacina::Context<serde_json::Value>` — Extract the inner Rust Context (for internal use)
-  `clone_inner` function L286-288 — `(&self) -> cloacina::Context<serde_json::Value>` — Clone the inner Rust Context (for internal use)
-  `get_data_clone` function L291-293 — `(&self) -> std::collections::HashMap<String, serde_json::Value>` — Get a clone of the context data as a HashMap (for internal use)
-  `PyContext` type L298-312 — `impl Clone for PyContext` — Manual implementation of Clone since Context<T> doesn't implement Clone
-  `clone` function L299-311 — `(&self) -> Self`
-  `PyDefaultRunnerConfig` type L338-649 — `= PyDefaultRunnerConfig`
-  `PyDefaultRunnerConfig` type L651-683 — `= PyDefaultRunnerConfig`
-  `to_rust_config` function L653-655 — `(&self) -> cloacina::runner::DefaultRunnerConfig` — Get the inner Rust config (for internal use)
-  `rebuild` function L659-682 — `( &self, apply: impl FnOnce( cloacina::runner::DefaultRunnerConfigBuilder, ) -> ...` — Rebuild the inner config by snapshotting current values into a builder,

#### bindings/cloaca-backend/src/lib.rs

- pub `HelloClass` struct L40-42 — `{ message: String }` — A simple hello world class for testing
- pub `new` function L47-51 — `() -> Self`
- pub `get_message` function L53-55 — `(&self) -> String`
- pub `__repr__` function L57-59 — `(&self) -> String`
-  `admin` module L19 — `-`
-  `context` module L20 — `-`
-  `runner` module L21 — `-`
-  `task` module L22 — `-`
-  `trigger` module L23 — `-`
-  `value_objects` module L24 — `-`
-  `workflow` module L25 — `-`
-  `HelloClass` type L45-60 — `= HelloClass`
-  `hello_world` function L65-67 — `() -> String` — A simple hello world function for testing
-  `cloaca` function L71-115 — `(m: &Bound<'_, PyModule>) -> PyResult<()>` — A unified Python module supporting both PostgreSQL and SQLite backends.
-  `tests` module L118-278 — `-`
-  `test_task_registration` function L124-167 — `()`
-  `test_workflow_add_task_lookup` function L170-211 — `()`
-  `test_namespace_investigation` function L214-277 — `()`
-  `TestTask` struct L227 — `-`
-  `TestTask` type L229-246 — `= TestTask`
-  `execute` function L230-236 — `( &self, context: cloacina::Context<serde_json::Value>, ) -> Result<cloacina::Co...`
-  `id` function L237-239 — `(&self) -> &str`
-  `dependencies` function L240-242 — `(&self) -> &[cloacina::TaskNamespace]`
-  `retry_policy` function L243-245 — `(&self) -> cloacina::retry::RetryPolicy`

#### bindings/cloaca-backend/src/runner.rs

- pub `ShutdownError` enum L34-46 — `ChannelClosed | ThreadPanic | Timeout` — Errors that can occur during async runtime shutdown
- pub `PyPipelineResult` struct L233-235 — `{ inner: cloacina::executor::PipelineResult }` — Python wrapper for PipelineResult
- pub `status` function L241-243 — `(&self) -> String` — Get the execution status
- pub `start_time` function L247-249 — `(&self) -> String` — Get execution start time as ISO string
- pub `end_time` function L253-255 — `(&self) -> Option<String>` — Get execution end time as ISO string
- pub `final_context` function L259-263 — `(&self) -> PyContext` — Get the final context
- pub `error_message` function L267-269 — `(&self) -> Option<&str>` — Get error message if execution failed
- pub `__repr__` function L272-278 — `(&self) -> String` — String representation
- pub `PyDefaultRunner` struct L283-285 — `{ runtime_handle: Mutex<AsyncRuntimeHandle> }` — Python wrapper for DefaultRunner
- pub `new` function L291-682 — `(database_url: &str) -> PyResult<Self>` — Create a new DefaultRunner with database connection
- pub `with_config` function L686-1057 — `( database_url: &str, config: &crate::context::PyDefaultRunnerConfig, ) -> PyRes...` — Create a new DefaultRunner with custom configuration
- pub `with_schema` function L1088-1501 — `(database_url: &str, schema: &str) -> PyResult<PyDefaultRunner>` — Create a new DefaultRunner with PostgreSQL schema-based multi-tenancy
- pub `execute` function L1504-1550 — `( &self, workflow_name: &str, context: &PyContext, py: Python, ) -> PyResult<PyP...` — Execute a workflow by name with context
- pub `start` function L1553-1560 — `(&self) -> PyResult<()>` — Start the runner (task scheduler and executor)
- pub `stop` function L1563-1570 — `(&self) -> PyResult<()>` — Stop the runner
- pub `shutdown` function L1580-1599 — `(&self, py: Python) -> PyResult<()>` — Shutdown the runner and cleanup resources
- pub `register_cron_workflow` function L1619-1651 — `( &self, workflow_name: String, cron_expression: String, timezone: String, py: P...` — Register a cron workflow for automatic execution at scheduled times
- pub `list_cron_schedules` function L1662-1721 — `( &self, enabled_only: Option<bool>, limit: Option<i64>, offset: Option<i64>, py...` — List all cron schedules
- pub `set_cron_schedule_enabled` function L1728-1758 — `( &self, schedule_id: String, enabled: bool, py: Python, ) -> PyResult<()>` — Enable or disable a cron schedule
- pub `delete_cron_schedule` function L1764-1788 — `(&self, schedule_id: String, py: Python) -> PyResult<()>` — Delete a cron schedule
- pub `get_cron_schedule` function L1797-1837 — `(&self, schedule_id: String, py: Python) -> PyResult<PyObject>` — Get details of a specific cron schedule
- pub `update_cron_schedule` function L1845-1877 — `( &self, schedule_id: String, cron_expression: String, timezone: String, py: Pyt...` — Update a cron schedule's expression and timezone
- pub `get_cron_execution_history` function L1888-1946 — `( &self, schedule_id: String, limit: Option<i64>, offset: Option<i64>, py: Pytho...` — Get execution history for a specific cron schedule
- pub `get_cron_execution_stats` function L1955-1994 — `(&self, since: String, py: Python) -> PyResult<PyObject>` — Get execution statistics for cron schedules
- pub `list_trigger_schedules` function L2010-2071 — `( &self, enabled_only: Option<bool>, limit: Option<i64>, offset: Option<i64>, py...` — List all trigger schedules
- pub `get_trigger_schedule` function L2080-2126 — `( &self, trigger_name: String, py: Python, ) -> PyResult<Option<PyObject>>` — Get details of a specific trigger schedule
- pub `set_trigger_enabled` function L2133-2161 — `( &self, trigger_name: String, enabled: bool, py: Python, ) -> PyResult<()>` — Enable or disable a trigger
- pub `get_trigger_execution_history` function L2173-2234 — `( &self, trigger_name: String, limit: Option<i64>, offset: Option<i64>, py: Pyth...` — Get execution history for a specific trigger
- pub `__repr__` function L2237-2239 — `(&self) -> String` — String representation
- pub `__enter__` function L2242-2244 — `(slf: PyRef<Self>) -> PyRef<Self>` — Context manager entry
- pub `__exit__` function L2247-2256 — `( &self, py: Python, _exc_type: Option<&Bound<PyAny>>, _exc_value: Option<&Bound...` — Context manager exit - automatically shutdown
- pub `from_result` function L2260-2262 — `(result: cloacina::executor::PipelineResult) -> Self`
-  `SHUTDOWN_TIMEOUT` variable L30 — `: Duration` — Timeout for waiting on runtime thread shutdown
-  `RuntimeMessage` enum L49-153 — `Execute | RegisterCronWorkflow | ListCronSchedules | SetCronScheduleEnabled | De...` — Message types for communication with the async runtime thread
-  `AsyncRuntimeHandle` struct L156-159 — `{ tx: mpsc::UnboundedSender<RuntimeMessage>, thread_handle: Option<thread::JoinH...` — Handle to the background async runtime thread
-  `AsyncRuntimeHandle` type L161-220 — `= AsyncRuntimeHandle`
-  `shutdown` function L166-219 — `(&mut self) -> Result<(), ShutdownError>` — Shutdown the runtime thread and wait for it to complete
-  `AsyncRuntimeHandle` type L222-229 — `impl Drop for AsyncRuntimeHandle`
-  `drop` function L223-228 — `(&mut self)`
-  `PyPipelineResult` type L238-279 — `= PyPipelineResult`
-  `PyDefaultRunner` type L288-2257 — `= PyDefaultRunner`
-  `PyPipelineResult` type L2259-2263 — `= PyPipelineResult`

#### bindings/cloaca-backend/src/task.rs

- pub `PyTaskHandle` struct L38-40 — `{ inner: Option<cloacina::TaskHandle> }` — Python wrapper for TaskHandle providing defer_until capability.
- pub `defer_until` function L57-93 — `( &mut self, py: Python, condition: PyObject, poll_interval_ms: u64, ) -> PyResu...` — Release the concurrency slot while polling an external condition.
- pub `is_slot_held` function L96-102 — `(&self) -> PyResult<bool>` — Returns whether the handle currently holds a concurrency slot.
- pub `WorkflowBuilderRef` struct L107-109 — `{ context: PyWorkflowContext }` — Workflow builder reference for automatic task registration
- pub `push_workflow_context` function L115-120 — `(context: PyWorkflowContext)` — Push a workflow context onto the stack (called when entering workflow scope)
- pub `pop_workflow_context` function L123-125 — `() -> Option<WorkflowBuilderRef>` — Pop a workflow context from the stack (called when exiting workflow scope)
- pub `current_workflow_context` function L128-135 — `() -> PyResult<PyWorkflowContext>` — Get the current workflow context (used by task decorator)
- pub `PythonTaskWrapper` struct L141-149 — `{ id: String, dependencies: Vec<cloacina::TaskNamespace>, retry_policy: cloacina...` — Python task wrapper implementing Rust Task trait
- pub `TaskDecorator` struct L385-391 — `{ id: Option<String>, dependencies: Vec<PyObject>, retry_policy: cloacina::retry...` — Decorator class that holds task configuration
- pub `__call__` function L395-480 — `(&self, py: Python, func: PyObject) -> PyResult<PyObject>`
- pub `task` function L599-628 — `( id: Option<String>, dependencies: Option<Vec<PyObject>>, retry_attempts: Optio...`
-  `PyTaskHandle` type L43-103 — `= PyTaskHandle`
-  `WORKFLOW_CONTEXT_STACK` variable L112 — `: Mutex<Vec<WorkflowBuilderRef>>` — Global context stack for workflow-scoped task registration
-  `PythonTaskWrapper` type L153 — `impl Send for PythonTaskWrapper`
-  `PythonTaskWrapper` type L154 — `impl Sync for PythonTaskWrapper`
-  `PythonTaskWrapper` type L157-324 — `= PythonTaskWrapper`
-  `execute` function L158-290 — `( &self, context: cloacina::Context<serde_json::Value>, ) -> Result<cloacina::Co...`
-  `id` function L292-294 — `(&self) -> &str`
-  `dependencies` function L296-298 — `(&self) -> &[cloacina::TaskNamespace]`
-  `retry_policy` function L300-302 — `(&self) -> cloacina::retry::RetryPolicy`
-  `requires_handle` function L304-306 — `(&self) -> bool`
-  `checkpoint` function L308-313 — `( &self, _context: &cloacina::Context<serde_json::Value>, ) -> Result<(), cloaci...`
-  `trigger_rules` function L315-318 — `(&self) -> serde_json::Value`
-  `code_fingerprint` function L320-323 — `(&self) -> Option<String>`
-  `build_retry_policy` function L328-381 — `( retry_attempts: Option<usize>, retry_backoff: Option<String>, retry_delay_ms: ...` — Build retry policy from Python decorator parameters
-  `TaskDecorator` type L394-481 — `= TaskDecorator`
-  `TaskDecorator` type L483-543 — `= TaskDecorator`
-  `convert_dependencies_to_namespaces` function L485-542 — `( &self, py: Python, context: &PyWorkflowContext, ) -> PyResult<Vec<cloacina::Ta...` — Convert mixed dependencies (strings and function objects) to TaskNamespace objects

#### bindings/cloaca-backend/src/trigger.rs

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

#### bindings/cloaca-backend/src/workflow.rs

- pub `PyWorkflowBuilder` struct L24-27 — `{ inner: cloacina::WorkflowBuilder, context: PyWorkflowContext }` — Python wrapper for WorkflowBuilder
- pub `new` function L34-54 — `( name: &str, tenant: Option<&str>, package: Option<&str>, workflow: Option<&str...` — Create a new WorkflowBuilder with namespace context
- pub `description` function L57-59 — `(&mut self, description: &str)` — Set the workflow description
- pub `tag` function L62-64 — `(&mut self, key: &str, value: &str)` — Add a tag to the workflow
- pub `add_task` function L67-158 — `(&mut self, py: Python, task: PyObject) -> PyResult<()>` — Add a task to the workflow by ID or function reference
- pub `build` function L161-168 — `(&self) -> PyResult<PyWorkflow>` — Build the workflow
- pub `__enter__` function L171-174 — `(slf: PyRef<Self>) -> PyRef<Self>` — Context manager entry - establish workflow context for task decorators
- pub `__exit__` function L177-228 — `( &mut self, _py: Python, _exc_type: Option<&Bound<PyAny>>, _exc_value: Option<&...` — Context manager exit - clean up context and build workflow
- pub `__repr__` function L231-233 — `(&self) -> String` — String representation
- pub `PyWorkflow` struct L239-241 — `{ inner: cloacina::Workflow }` — Python wrapper for Workflow
- pub `name` function L247-249 — `(&self) -> &str` — Get workflow name
- pub `description` function L253-259 — `(&self) -> String` — Get workflow description
- pub `version` function L263-265 — `(&self) -> &str` — Get workflow version
- pub `topological_sort` function L268-273 — `(&self) -> PyResult<Vec<String>>` — Get topological sort of tasks
- pub `get_execution_levels` function L276-286 — `(&self) -> PyResult<Vec<Vec<String>>>` — Get execution levels (tasks that can run in parallel)
- pub `get_roots` function L289-295 — `(&self) -> Vec<String>` — Get root tasks (no dependencies)
- pub `get_leaves` function L298-304 — `(&self) -> Vec<String>` — Get leaf tasks (no dependents)
- pub `validate` function L309-313 — `(&self) -> PyResult<()>` — Validate the workflow
- pub `__repr__` function L316-322 — `(&self) -> String` — String representation
- pub `register_workflow_constructor` function L328-350 — `(name: String, constructor: PyObject) -> PyResult<()>` — Register a workflow constructor function
-  `PyWorkflowBuilder` type L30-234 — `= PyWorkflowBuilder`
-  `PyWorkflow` type L244-323 — `= PyWorkflow`

### bindings/cloaca-backend/src/value_objects

> *Semantic summary to be generated by AI agent.*

#### bindings/cloaca-backend/src/value_objects/context.rs

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
- pub `default` function L117-123 — `() -> Self` — Get the default workflow context (for backward compatibility)
- pub `as_components` function L126-128 — `(&self) -> (&str, &str, &str)` — Convert to namespace components
-  `PyWorkflowContext` type L30-113 — `= PyWorkflowContext`
-  `PyWorkflowContext` type L115-129 — `= PyWorkflowContext`

#### bindings/cloaca-backend/src/value_objects/mod.rs

- pub `context` module L17 — `-`
- pub `namespace` module L18 — `-`
- pub `retry` module L19 — `-`

#### bindings/cloaca-backend/src/value_objects/namespace.rs

- pub `PyTaskNamespace` struct L23-25 — `{ inner: cloacina::TaskNamespace }` — Python wrapper for TaskNamespace
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
- pub `from_rust` function L134-136 — `(namespace: cloacina::TaskNamespace) -> Self` — Convert from Rust TaskNamespace (for internal use)
- pub `to_rust` function L139-141 — `(&self) -> cloacina::TaskNamespace` — Convert to Rust TaskNamespace (for internal use)
-  `PyTaskNamespace` type L28-130 — `= PyTaskNamespace`
-  `PyTaskNamespace` type L132-142 — `= PyTaskNamespace`

#### bindings/cloaca-backend/src/value_objects/retry.rs

- pub `PyRetryPolicy` struct L23-25 — `{ inner: cloacina::retry::RetryPolicy }` — Python wrapper for RetryPolicy
- pub `PyBackoffStrategy` struct L30-32 — `{ inner: cloacina::retry::BackoffStrategy }` — Python wrapper for BackoffStrategy
- pub `PyRetryCondition` struct L37-39 — `{ inner: cloacina::retry::RetryCondition }` — Python wrapper for RetryCondition
- pub `PyRetryPolicyBuilder` struct L44-51 — `{ max_attempts: Option<i32>, backoff_strategy: Option<cloacina::retry::BackoffSt...` — Python wrapper for RetryPolicy::Builder
- pub `builder` function L57-66 — `() -> PyRetryPolicyBuilder` — Create a builder for constructing RetryPolicy
- pub `default` function L70-74 — `() -> Self` — Create a default RetryPolicy
- pub `should_retry` function L77-81 — `(&self, attempt: i32, _error_type: &str) -> bool` — Check if a retry should be attempted
- pub `calculate_delay` function L84-87 — `(&self, attempt: i32) -> f64` — Calculate delay for a given attempt
- pub `max_attempts` function L91-93 — `(&self) -> i32` — Get maximum number of attempts
- pub `initial_delay` function L97-99 — `(&self) -> f64` — Get initial delay in seconds
- pub `max_delay` function L103-105 — `(&self) -> f64` — Get maximum delay in seconds
- pub `with_jitter` function L109-111 — `(&self) -> bool` — Check if jitter is enabled
- pub `__repr__` function L114-122 — `(&self) -> String` — String representation
- pub `fixed` function L129-133 — `() -> Self` — Fixed delay strategy
- pub `linear` function L137-141 — `(multiplier: f64) -> Self` — Linear backoff strategy
- pub `exponential` function L145-152 — `(base: f64, multiplier: Option<f64>) -> Self` — Exponential backoff strategy
- pub `__repr__` function L155-171 — `(&self) -> String` — String representation
- pub `never` function L178-182 — `() -> Self` — Never retry
- pub `transient_only` function L186-190 — `() -> Self` — Retry only on transient errors
- pub `all_errors` function L194-198 — `() -> Self` — Retry on all errors
- pub `error_pattern` function L202-206 — `(patterns: Vec<String>) -> Self` — Retry on specific error patterns
- pub `__repr__` function L209-220 — `(&self) -> String` — String representation
- pub `max_attempts` function L226-230 — `(&self, attempts: i32) -> Self` — Set maximum number of retry attempts
- pub `initial_delay` function L233-237 — `(&self, delay_seconds: f64) -> Self` — Set initial delay
- pub `max_delay` function L240-244 — `(&self, delay_seconds: f64) -> Self` — Set maximum delay
- pub `backoff_strategy` function L247-251 — `(&self, strategy: PyBackoffStrategy) -> Self` — Set backoff strategy
- pub `retry_condition` function L254-258 — `(&self, condition: PyRetryCondition) -> Self` — Set retry condition
- pub `with_jitter` function L261-265 — `(&self, jitter: bool) -> Self` — Enable/disable jitter
- pub `build` function L268-293 — `(&self) -> PyRetryPolicy` — Build the RetryPolicy
- pub `from_rust` function L298-300 — `(policy: cloacina::retry::RetryPolicy) -> Self` — Convert from Rust RetryPolicy (for internal use)
- pub `to_rust` function L303-305 — `(&self) -> cloacina::retry::RetryPolicy` — Convert to Rust RetryPolicy (for internal use)
-  `PyRetryPolicy` type L54-123 — `= PyRetryPolicy`
-  `PyBackoffStrategy` type L126-172 — `= PyBackoffStrategy`
-  `PyRetryCondition` type L175-221 — `= PyRetryCondition`
-  `PyRetryPolicyBuilder` type L224-294 — `= PyRetryPolicyBuilder`
-  `PyRetryPolicy` type L296-306 — `= PyRetryPolicy`

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
-  `tests` module L381-483 — `-` — ```
-  `test_cron_evaluator_creation` function L386-390 — `()` — ```
-  `test_invalid_cron_expression` function L393-400 — `()` — ```
-  `test_invalid_timezone` function L403-407 — `()` — ```
-  `test_next_execution_utc` function L410-419 — `()` — ```
-  `test_next_execution_timezone` function L422-431 — `()` — ```
-  `test_next_executions` function L434-444 — `()` — ```
-  `test_executions_between` function L447-459 — `()` — ```
-  `test_validation_functions` function L462-472 — `()` — ```
-  `test_from_str` function L475-482 — `()` — ```

#### crates/cloacina/src/cron_recovery.rs

- pub `CronRecoveryConfig` struct L57-68 — `{ check_interval: Duration, lost_threshold_minutes: i32, max_recovery_age: Durat...` — Configuration for the cron recovery service.
- pub `CronRecoveryService` struct L87-94 — `{ dal: Arc<DAL>, executor: Arc<dyn PipelineExecutor>, config: CronRecoveryConfig...` — Recovery service for lost cron executions.
- pub `new` function L104-117 — `( dal: Arc<DAL>, executor: Arc<dyn PipelineExecutor>, config: CronRecoveryConfig...` — Creates a new cron recovery service.
- pub `with_defaults` function L120-126 — `( dal: Arc<DAL>, executor: Arc<dyn PipelineExecutor>, shutdown: watch::Receiver<...` — Creates a new recovery service with default configuration.
- pub `run_recovery_loop` function L132-160 — `(&mut self) -> Result<(), PipelineError>` — Runs the recovery service loop.
- pub `clear_recovery_attempts` function L358-362 — `(&self)` — Clears the recovery attempts cache.
- pub `get_recovery_attempts` function L365-371 — `( &self, execution_id: crate::database::UniversalUuid, ) -> usize` — Gets the current recovery attempts for an execution.
-  `CronRecoveryConfig` type L70-80 — `impl Default for CronRecoveryConfig` — - The execution is too old (beyond recovery window)
-  `default` function L71-79 — `() -> Self` — - The execution is too old (beyond recovery window)
-  `CronRecoveryService` type L96-372 — `= CronRecoveryService` — - The execution is too old (beyond recovery window)
-  `check_and_recover_lost_executions` function L163-195 — `(&self) -> Result<(), PipelineError>` — Checks for lost executions and attempts to recover them.
-  `recover_execution` function L198-352 — `(&self, execution: &CronExecution) -> Result<(), PipelineError>` — Attempts to recover a single lost execution.
-  `tests` module L375-405 — `-` — - The execution is too old (beyond recovery window)
-  `test_recovery_config_default` function L380-387 — `()` — - The execution is too old (beyond recovery window)
-  `test_recovery_attempts_tracking` function L390-404 — `()` — - The execution is too old (beyond recovery window)

#### crates/cloacina/src/cron_scheduler.rs

- pub `CronSchedulerConfig` struct L81-88 — `{ poll_interval: Duration, max_catchup_executions: usize, max_acceptable_delay: ...` — Configuration for the cron scheduler.
- pub `CronScheduler` struct L128-133 — `{ dal: Arc<DAL>, executor: Arc<dyn PipelineExecutor>, config: CronSchedulerConfi...` — Saga-based cron scheduler for time-based workflow execution.
- pub `new` function L143-155 — `( dal: Arc<DAL>, executor: Arc<dyn PipelineExecutor>, config: CronSchedulerConfi...` — Creates a new cron scheduler.
- pub `with_defaults` function L158-164 — `( dal: Arc<DAL>, executor: Arc<dyn PipelineExecutor>, shutdown: watch::Receiver<...` — Creates a new cron scheduler with default configuration.
- pub `run_polling_loop` function L177-205 — `(&mut self) -> Result<(), PipelineError>` — Runs the main polling loop for cron schedule processing.
-  `CronSchedulerConfig` type L90-98 — `impl Default for CronSchedulerConfig` — ```
-  `default` function L91-97 — `() -> Self` — ```
-  `CronScheduler` type L135-572 — `= CronScheduler` — ```
-  `check_and_execute_schedules` function L213-243 — `(&self) -> Result<(), PipelineError>` — Checks for due schedules and executes them.
-  `process_schedule` function L254-361 — `( &self, schedule: &CronSchedule, now: DateTime<Utc>, ) -> Result<(), PipelineEr...` — Processes a single cron schedule using the saga pattern.
-  `is_schedule_active` function L364-380 — `(&self, schedule: &CronSchedule, now: DateTime<Utc>) -> bool` — Checks if a schedule is within its active time window.
-  `calculate_execution_times` function L388-433 — `( &self, schedule: &CronSchedule, now: DateTime<Utc>, ) -> Result<Vec<DateTime<U...` — Calculates execution times based on the schedule's catchup policy.
-  `calculate_next_run` function L436-453 — `( &self, schedule: &CronSchedule, after: DateTime<Utc>, ) -> Result<DateTime<Utc...` — Calculates the next run time for a schedule.
-  `execute_workflow` function L460-513 — `( &self, schedule: &CronSchedule, scheduled_time: DateTime<Utc>, ) -> Result<cra...` — Executes a workflow by handing it off to the pipeline executor.
-  `create_execution_audit` function L527-545 — `( &self, schedule_id: crate::database::UniversalUuid, scheduled_time: DateTime<U...` — Creates audit record BEFORE workflow execution for guaranteed reliability.
-  `complete_execution_audit` function L555-571 — `( &self, audit_record_id: crate::database::UniversalUuid, pipeline_execution_id:...` — Updates audit record with pipeline execution ID after successful handoff.
-  `tests` module L575-640 — `-` — ```
-  `create_test_schedule` function L580-596 — `(cron_expr: &str, timezone: &str) -> CronSchedule` — ```
-  `test_cron_scheduler_config_default` function L599-607 — `()` — ```
-  `test_is_schedule_active` function L610-621 — `()` — ```
-  `test_calculate_execution_times_skip_policy` function L624-630 — `()` — ```
-  `test_calculate_execution_times_run_all_policy` function L633-639 — `()` — ```

#### crates/cloacina/src/error.rs

- pub `ContextError` enum L132-153 — `Serialization | KeyNotFound | TypeMismatch | KeyExists | Database | ConnectionPo...` — Errors that can occur during context operations.
- pub `RegistrationError` enum L171-180 — `DuplicateTaskId | InvalidTaskId | RegistrationFailed` — Errors that can occur during task registration.
- pub `ValidationError` enum L187-253 — `CyclicDependency | MissingDependency | MissingDependencyOld | CircularDependency...` — Errors that can occur during Workflow and dependency validation.
- pub `ExecutorError` enum L269-302 — `Database | ConnectionPool | TaskNotFound | TaskExecution | Context | TaskTimeout...` — Errors that can occur during task execution.
- pub `WorkflowError` enum L314-338 — `DuplicateTask | TaskNotFound | InvalidDependency | CyclicDependency | Unreachabl...` — Errors that can occur during workflow construction and management.
- pub `SubgraphError` enum L345-351 — `TaskNotFound | UnsupportedOperation` — Errors that can occur when creating Workflow subgraphs.
-  `ContextError` type L155-164 — `= ContextError` — relevant context information to aid in troubleshooting and recovery.
-  `from` function L156-163 — `(err: cloacina_workflow::ContextError) -> Self` — relevant context information to aid in troubleshooting and recovery.
-  `ValidationError` type L255-259 — `= ValidationError` — relevant context information to aid in troubleshooting and recovery.
-  `from` function L256-258 — `(err: deadpool::managed::PoolError<deadpool_diesel::Error>) -> Self` — relevant context information to aid in troubleshooting and recovery.
-  `ContextError` type L261-265 — `= ContextError` — relevant context information to aid in troubleshooting and recovery.
-  `from` function L262-264 — `(err: deadpool::managed::PoolError<deadpool_diesel::Error>) -> Self` — relevant context information to aid in troubleshooting and recovery.
-  `ExecutorError` type L304-308 — `= ExecutorError` — relevant context information to aid in troubleshooting and recovery.
-  `from` function L305-307 — `(err: deadpool::managed::PoolError<deadpool_diesel::Error>) -> Self` — relevant context information to aid in troubleshooting and recovery.
-  `TaskError` type L354-379 — `= TaskError` — relevant context information to aid in troubleshooting and recovery.
-  `from` function L355-378 — `(error: ContextError) -> Self` — relevant context information to aid in troubleshooting and recovery.

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

#### crates/cloacina/src/lib.rs

- pub `prelude` module L450-480 — `-` — Prelude module for convenient imports.
- pub `context` module L484 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `continuous` module L485 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `cron_evaluator` module L486 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `cron_recovery` module L487 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `cron_scheduler` module L488 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `crypto` module L489 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `dal` module L490 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `database` module L491 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `dispatcher` module L492 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `error` module L493 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `executor` module L494 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `graph` module L495 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `logging` module L496 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `models` module L497 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `packaging` module L498 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `python` module L499 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `registry` module L500 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `retry` module L501 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `runner` module L502 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `security` module L503 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `task` module L504 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `task_scheduler` module L505 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `trigger` module L506 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `trigger_scheduler` module L507 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `workflow` module L508 — `-` — - [`retry`]: Retry policies and backoff strategies
- pub `setup_test` function L516-518 — `()` — - [`retry`]: Retry policies and backoff strategies

#### crates/cloacina/src/logging.rs

- pub `init_logging` function L136-146 — `(level: Option<Level>)` — Initializes the logging system with the specified log level.
- pub `init_test_logging` function L170-175 — `()` — Initializes the logging system for test environments.
-  `tests` module L178-191 — `-` — - Test logging initialization is idempotent and safe to call multiple times
-  `test_logging_levels` function L183-190 — `()` — - Test logging initialization is idempotent and safe to call multiple times

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

#### crates/cloacina/src/trigger_scheduler.rs

- pub `TriggerSchedulerConfig` struct L80-85 — `{ base_poll_interval: Duration, poll_timeout: Duration }` — Configuration for the trigger scheduler.
- pub `TriggerScheduler` struct L120-127 — `{ dal: Arc<DAL>, executor: Arc<dyn PipelineExecutor>, config: TriggerSchedulerCo...` — Event-based trigger scheduler for workflow execution.
- pub `new` function L137-150 — `( dal: Arc<DAL>, executor: Arc<dyn PipelineExecutor>, config: TriggerSchedulerCo...` — Creates a new trigger scheduler.
- pub `with_defaults` function L153-159 — `( dal: Arc<DAL>, executor: Arc<dyn PipelineExecutor>, shutdown: watch::Receiver<...` — Creates a new trigger scheduler with default configuration.
- pub `run_polling_loop` function L172-200 — `(&mut self) -> Result<(), PipelineError>` — Runs the main polling loop for trigger processing.
- pub `register_trigger` function L457-468 — `( &self, trigger: &dyn Trigger, workflow_name: &str, ) -> Result<TriggerSchedule...` — Registers a trigger with the scheduler.
- pub `disable_trigger` function L471-482 — `(&self, trigger_name: &str) -> Result<(), ValidationError>` — Disables a trigger by name.
- pub `enable_trigger` function L485-496 — `(&self, trigger_name: &str) -> Result<(), ValidationError>` — Enables a trigger by name.
-  `TriggerSchedulerConfig` type L87-94 — `impl Default for TriggerSchedulerConfig` — ```
-  `default` function L88-93 — `() -> Self` — ```
-  `TriggerScheduler` type L129-497 — `= TriggerScheduler` — ```
-  `check_and_process_triggers` function L203-252 — `(&mut self) -> Result<(), PipelineError>` — Checks all registered triggers and processes those that are due.
-  `process_trigger` function L262-388 — `(&self, schedule: &TriggerSchedule) -> Result<(), TriggerError>` — Processes a single trigger schedule.
-  `create_execution_audit` function L391-411 — `( &self, trigger_name: &str, context_hash: &str, ) -> Result<crate::models::trig...` — Creates an audit record for a trigger execution.
-  `execute_workflow` function L414-446 — `( &self, schedule: &TriggerSchedule, mut context: Context<serde_json::Value>, ) ...` — Executes a workflow by handing it off to the pipeline executor.
-  `tests` module L500-509 — `-` — ```
-  `test_trigger_scheduler_config_default` function L504-508 — `()` — ```

### crates/cloacina/src/continuous

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/continuous/accumulator.rs

- pub `AccumulatorMetrics` struct L36-49 — `{ buffered_count: usize, oldest_boundary_emitted_at: Option<DateTime<Utc>>, newe...` — Observable state for monitoring and backpressure detection.
- pub `EdgeMetrics` struct L53-60 — `{ source: String, task: String, accumulator: AccumulatorMetrics }` — Per-edge metrics snapshot for the scheduler.
- pub `ReceiveResult` enum L64-69 — `Accepted | AcceptedWithDrop` — Result of receiving a boundary into an accumulator.
- pub `SignalAccumulator` interface L72-105 — `{ fn receive(), fn is_ready(), fn drain(), fn metrics(), fn consumer_watermark()...` — Per-edge stateful component that buffers boundaries and decides when to fire.
- pub `SimpleAccumulator` struct L114-126 — `{ buffer: Vec<BufferedBoundary>, policy: Box<dyn TriggerPolicy>, watermark: Opti...` — Simple accumulator with no watermark awareness.
- pub `new` function L130-143 — `(policy: Box<dyn TriggerPolicy>) -> Self` — Create a new SimpleAccumulator with the given trigger policy.
- pub `with_max_buffer` function L146-159 — `(policy: Box<dyn TriggerPolicy>, max_buffer_size: usize) -> Self` — Create a new SimpleAccumulator with a custom buffer size limit.
- pub `WatermarkMode` enum L267-272 — `WaitForWatermark | BestEffort` — How the accumulator uses source watermarks for readiness.
- pub `WindowedAccumulator` struct L279-293 — `{ buffer: Vec<BufferedBoundary>, policy: Box<dyn TriggerPolicy>, watermark: Opti...` — Windowed accumulator with source watermark awareness.
- pub `new` function L297-318 — `( policy: Box<dyn TriggerPolicy>, watermark_mode: WatermarkMode, boundary_ledger...` — Create a new WindowedAccumulator.
- pub `with_max_buffer` function L321-343 — `( policy: Box<dyn TriggerPolicy>, watermark_mode: WatermarkMode, boundary_ledger...` — Create a new WindowedAccumulator with a custom buffer size limit.
- pub `pending_boundary` function L346-350 — `(&self) -> Option<ComputationBoundary>` — Get the coalesced pending boundary without draining.
-  `try_drain` function L98-104 — `(&mut self) -> Option<Context<serde_json::Value>>` — Atomically check readiness and drain if ready.
-  `DEFAULT_MAX_BUFFER_SIZE` variable L108 — `: usize` — Default maximum buffer size for accumulators.
-  `SimpleAccumulator` type L128-160 — `= SimpleAccumulator` — See CLOACI-S-0005 for the full specification.
-  `SimpleAccumulator` type L162-263 — `impl SignalAccumulator for SimpleAccumulator` — See CLOACI-S-0005 for the full specification.
-  `receive` function L163-192 — `(&mut self, boundary: ComputationBoundary) -> ReceiveResult` — See CLOACI-S-0005 for the full specification.
-  `is_ready` function L194-196 — `(&self) -> bool` — See CLOACI-S-0005 for the full specification.
-  `drain` function L198-243 — `(&mut self) -> Context<serde_json::Value>` — See CLOACI-S-0005 for the full specification.
-  `metrics` function L245-254 — `(&self) -> AccumulatorMetrics` — See CLOACI-S-0005 for the full specification.
-  `consumer_watermark` function L256-258 — `(&self) -> Option<&ComputationBoundary>` — See CLOACI-S-0005 for the full specification.
-  `set_consumer_watermark` function L260-262 — `(&mut self, watermark: ComputationBoundary)` — See CLOACI-S-0005 for the full specification.
-  `WindowedAccumulator` type L295-351 — `= WindowedAccumulator` — See CLOACI-S-0005 for the full specification.
-  `WindowedAccumulator` type L353-457 — `impl SignalAccumulator for WindowedAccumulator` — See CLOACI-S-0005 for the full specification.
-  `receive` function L354-381 — `(&mut self, boundary: ComputationBoundary) -> ReceiveResult` — See CLOACI-S-0005 for the full specification.
-  `is_ready` function L383-398 — `(&self) -> bool` — See CLOACI-S-0005 for the full specification.
-  `drain` function L400-437 — `(&mut self) -> Context<serde_json::Value>` — See CLOACI-S-0005 for the full specification.
-  `metrics` function L439-448 — `(&self) -> AccumulatorMetrics` — See CLOACI-S-0005 for the full specification.
-  `consumer_watermark` function L450-452 — `(&self) -> Option<&ComputationBoundary>` — See CLOACI-S-0005 for the full specification.
-  `set_consumer_watermark` function L454-456 — `(&mut self, watermark: ComputationBoundary)` — See CLOACI-S-0005 for the full specification.
-  `tests` module L460-824 — `-` — See CLOACI-S-0005 for the full specification.
-  `make_offset_boundary` function L465-471 — `(start: i64, end: i64) -> ComputationBoundary` — See CLOACI-S-0005 for the full specification.
-  `make_cursor_boundary` function L473-481 — `(value: &str) -> ComputationBoundary` — See CLOACI-S-0005 for the full specification.
-  `test_simple_accumulator_receive_and_drain` function L484-498 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_simple_accumulator_coalesces_on_drain` function L501-513 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_simple_accumulator_updates_consumer_watermark` function L516-533 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_simple_accumulator_empty_drain` function L536-542 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_simple_accumulator_metrics` function L545-559 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_simple_accumulator_lag_tracking` function L562-578 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_simple_accumulator_multiple_drain_cycles` function L581-602 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_windowed_best_effort_fires_immediately` function L607-618 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_windowed_wait_for_watermark_blocks_without_watermark` function L621-633 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_windowed_wait_for_watermark_fires_when_covered` function L636-655 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_windowed_wait_for_watermark_blocks_when_not_covered` function L658-677 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_windowed_watermark_advance_unblocks` function L680-700 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_windowed_drain_produces_context` function L703-724 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_simple_accumulator_buffer_overflow_drops_oldest` function L729-747 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_simple_accumulator_buffer_within_limit` function L750-758 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_metrics_accurate_after_interleaved_receive_drain` function L761-790 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_set_consumer_watermark_enables_late_detection` function L793-807 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_try_drain_when_not_ready` function L810-814 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_try_drain_when_ready` function L817-823 — `()` — See CLOACI-S-0005 for the full specification.

#### crates/cloacina/src/continuous/boundary.rs

- pub `ComputationBoundary` struct L32-39 — `{ kind: BoundaryKind, metadata: Option<serde_json::Value>, emitted_at: DateTime<...` — A serializable message describing what slice of data a signal or execution covers.
- pub `BoundaryKind` enum L44-62 — `TimeRange | OffsetRange | Cursor | FullState | Custom` — The specific type and data of a computation boundary.
- pub `BufferedBoundary` struct L66-71 — `{ boundary: ComputationBoundary, received_at: DateTime<Utc> }` — A boundary buffered in an accumulator, with receipt timestamp for backpressure measurement.
- pub `new` function L75-80 — `(boundary: ComputationBoundary) -> Self` — Create a new buffered boundary with the current time as receipt time.
- pub `lag` function L83-86 — `(&self) -> chrono::Duration` — Calculate ingestion lag (received_at - emitted_at).
- pub `CustomBoundarySchema` struct L91-96 — `{ kind: String, schema: serde_json::Value }` — Schema definition for custom boundary types.
- pub `register_custom_boundary` function L124-147 — `(kind: &str, schema: serde_json::Value)` — Register a custom boundary schema.
- pub `validate_custom_boundary` function L153-169 — `(kind: &str, value: &serde_json::Value) -> Result<(), String>` — Validate a custom boundary payload against its registered schema.
- pub `validate_boundary` function L175-181 — `(boundary: &ComputationBoundary) -> Result<(), String>` — Validate a `ComputationBoundary`.
- pub `clear_custom_schemas` function L185-188 — `()` — Clear all registered custom boundary schemas (for testing).
- pub `coalesce` function L206-275 — `(boundaries: &[ComputationBoundary]) -> Option<ComputationBoundary>` — Coalesce a slice of computation boundaries into a single boundary.
-  `BufferedBoundary` type L73-87 — `= BufferedBoundary` — See CLOACI-S-0002 for the full specification.
-  `CompiledSchema` struct L99-104 — `{ definition: CustomBoundarySchema, validator: jsonschema::Validator }` — Compiled schema entry for the registry.
-  `CompiledSchema` type L107-113 — `= CompiledSchema` — See CLOACI-S-0002 for the full specification.
-  `fmt` function L108-112 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — See CLOACI-S-0002 for the full specification.
-  `CUSTOM_SCHEMAS` variable L116-117 — `: std::sync::LazyLock<RwLock<HashMap<String, CompiledSchema>>>` — Global registry for custom boundary schemas with compiled validators.
-  `tests` module L278-595 — `-` — See CLOACI-S-0002 for the full specification.
-  `make_time_boundary` function L282-292 — `(start_offset_hours: i64, end_offset_hours: i64) -> ComputationBoundary` — See CLOACI-S-0002 for the full specification.
-  `make_offset_boundary` function L294-300 — `(start: i64, end: i64) -> ComputationBoundary` — See CLOACI-S-0002 for the full specification.
-  `make_cursor_boundary` function L302-310 — `(value: &str, emitted_at: DateTime<Utc>) -> ComputationBoundary` — See CLOACI-S-0002 for the full specification.
-  `make_fullstate_boundary` function L312-320 — `(value: &str, emitted_at: DateTime<Utc>) -> ComputationBoundary` — See CLOACI-S-0002 for the full specification.
-  `test_coalesce_empty` function L325-327 — `()` — See CLOACI-S-0002 for the full specification.
-  `test_coalesce_single` function L330-334 — `()` — See CLOACI-S-0002 for the full specification.
-  `test_coalesce_time_ranges` function L337-356 — `()` — See CLOACI-S-0002 for the full specification.
-  `test_coalesce_offset_ranges` function L359-369 — `()` — See CLOACI-S-0002 for the full specification.
-  `test_coalesce_cursors_latest_wins` function L372-384 — `()` — See CLOACI-S-0002 for the full specification.
-  `test_coalesce_fullstate_latest_wins` function L387-398 — `()` — See CLOACI-S-0002 for the full specification.
-  `test_buffered_boundary_lag` function L403-413 — `()` — See CLOACI-S-0002 for the full specification.
-  `test_boundary_serialization_roundtrip` function L418-427 — `()` — See CLOACI-S-0002 for the full specification.
-  `test_boundary_kind_tagged_serialization` function L430-437 — `()` — See CLOACI-S-0002 for the full specification.
-  `test_custom_schema_validation_passes` function L442-465 — `()` — See CLOACI-S-0002 for the full specification.
-  `test_custom_schema_missing_required_field` function L468-486 — `()` — See CLOACI-S-0002 for the full specification.
-  `test_custom_schema_unregistered_kind` function L489-497 — `()` — See CLOACI-S-0002 for the full specification.
-  `test_custom_schema_wrong_type` function L500-520 — `()` — See CLOACI-S-0002 for the full specification.
-  `test_boundary_json_roundtrip_offset_range` function L525-546 — `()` — See CLOACI-S-0002 for the full specification.
-  `test_boundary_json_roundtrip_cursor` function L549-567 — `()` — See CLOACI-S-0002 for the full specification.
-  `test_boundary_json_roundtrip_time_range` function L570-594 — `()` — See CLOACI-S-0002 for the full specification.

#### crates/cloacina/src/continuous/datasource.rs

- pub `DataConnection` interface L32-41 — `{ fn connect(), fn descriptor(), fn system_metadata() }` — Trait for connecting to external data systems.
- pub `DataConnectionError` enum L45-50 — `ConnectionFailed | ConfigurationError` — Error type for data connection operations.
- pub `ConnectionDescriptor` struct L54-59 — `{ system_type: String, location: String }` — Generic lineage descriptor for a data connection.
- pub `DataSourceMetadata` struct L69-76 — `{ description: Option<String>, owner: Option<String>, tags: Vec<String> }` — Metadata for lineage tracking on a data source.
- pub `DataSource` struct L82-92 — `{ name: String, connection: Box<dyn DataConnection>, detector_workflow: String, ...` — A named handle to an external dataset.
- pub `GraphError` enum L107-122 — `SourceNotFound | ConnectionTypeMismatch | ConnectionError` — Errors that can occur during graph operations.
- pub `DataSourceMap` struct L128-130 — `{ sources: HashMap<String, DataSource> }` — A map of data sources provided to continuous tasks at execution time.
- pub `new` function L134-138 — `() -> Self` — Create a new empty data source map.
- pub `insert` function L141-143 — `(&mut self, source: DataSource)` — Insert a data source into the map.
- pub `get` function L146-148 — `(&self, name: &str) -> Option<&DataSource>` — Get a data source by name.
- pub `connection` function L155-169 — `(&self, name: &str) -> Result<Box<T>, GraphError>` — Get a typed connection handle, with a clear error on wiring mismatch.
- pub `len` function L172-174 — `(&self) -> usize` — Get the number of data sources in the map.
- pub `is_empty` function L177-179 — `(&self) -> bool` — Check if the map is empty.
- pub `names` function L182-184 — `(&self) -> impl Iterator<Item = &str>` — Iterate over all data source names.
-  `ConnectionDescriptor` type L61-65 — `= ConnectionDescriptor` — See CLOACI-S-0003 for the full specification.
-  `fmt` function L62-64 — `(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result` — See CLOACI-S-0003 for the full specification.
-  `DataSource` type L94-103 — `= DataSource` — See CLOACI-S-0003 for the full specification.
-  `fmt` function L95-102 — `(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result` — See CLOACI-S-0003 for the full specification.
-  `DataSourceMap` type L132-185 — `= DataSourceMap` — See CLOACI-S-0003 for the full specification.
-  `DataSourceMap` type L187-191 — `impl Default for DataSourceMap` — See CLOACI-S-0003 for the full specification.
-  `default` function L188-190 — `() -> Self` — See CLOACI-S-0003 for the full specification.
-  `DataSourceMap` type L193-199 — `= DataSourceMap` — See CLOACI-S-0003 for the full specification.
-  `fmt` function L194-198 — `(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result` — See CLOACI-S-0003 for the full specification.
-  `tests` module L202-460 — `-` — See CLOACI-S-0003 for the full specification.
-  `TestStringConnection` struct L206-208 — `{ value: String }` — Simple test connection that returns a String handle.
-  `TestStringConnection` type L210-225 — `impl DataConnection for TestStringConnection` — See CLOACI-S-0003 for the full specification.
-  `connect` function L211-213 — `(&self) -> Result<Box<dyn Any>, DataConnectionError>` — See CLOACI-S-0003 for the full specification.
-  `descriptor` function L215-220 — `(&self) -> ConnectionDescriptor` — See CLOACI-S-0003 for the full specification.
-  `system_metadata` function L222-224 — `(&self) -> serde_json::Value` — See CLOACI-S-0003 for the full specification.
-  `TestIntConnection` struct L228-230 — `{ value: i32 }` — Test connection that returns an i32 handle.
-  `TestIntConnection` type L232-247 — `impl DataConnection for TestIntConnection` — See CLOACI-S-0003 for the full specification.
-  `connect` function L233-235 — `(&self) -> Result<Box<dyn Any>, DataConnectionError>` — See CLOACI-S-0003 for the full specification.
-  `descriptor` function L237-242 — `(&self) -> ConnectionDescriptor` — See CLOACI-S-0003 for the full specification.
-  `system_metadata` function L244-246 — `(&self) -> serde_json::Value` — See CLOACI-S-0003 for the full specification.
-  `make_test_source` function L249-256 — `(name: &str, conn: impl DataConnection + 'static) -> DataSource` — See CLOACI-S-0003 for the full specification.
-  `test_datasource_map_typed_access` function L259-270 — `()` — See CLOACI-S-0003 for the full specification.
-  `test_datasource_map_type_mismatch` function L273-290 — `()` — See CLOACI-S-0003 for the full specification.
-  `test_datasource_map_missing_source` function L293-301 — `()` — See CLOACI-S-0003 for the full specification.
-  `test_datasource_map_multiple_sources` function L304-317 — `()` — See CLOACI-S-0003 for the full specification.
-  `test_connection_descriptor` function L320-328 — `()` — See CLOACI-S-0003 for the full specification.
-  `test_datasource_debug` function L331-340 — `()` — See CLOACI-S-0003 for the full specification.
-  `test_datasource_map_new_is_empty` function L343-347 — `()` — See CLOACI-S-0003 for the full specification.
-  `test_datasource_map_default_is_empty` function L350-354 — `()` — See CLOACI-S-0003 for the full specification.
-  `test_datasource_map_insert_and_len` function L357-372 — `()` — See CLOACI-S-0003 for the full specification.
-  `test_datasource_map_get_returns_source` function L375-389 — `()` — See CLOACI-S-0003 for the full specification.
-  `test_datasource_map_get_returns_none_for_missing` function L392-395 — `()` — See CLOACI-S-0003 for the full specification.
-  `test_datasource_map_names` function L398-417 — `()` — See CLOACI-S-0003 for the full specification.
-  `test_datasource_map_names_empty` function L420-424 — `()` — See CLOACI-S-0003 for the full specification.
-  `test_datasource_map_insert_overwrites_existing` function L427-445 — `()` — See CLOACI-S-0003 for the full specification.
-  `test_datasource_map_debug_format` function L448-459 — `()` — See CLOACI-S-0003 for the full specification.

#### crates/cloacina/src/continuous/detector.rs

- pub `DETECTOR_OUTPUT_KEY` variable L29 — `: &str` — Well-known context key for detector output.
- pub `DetectorOutput` enum L38-50 — `Change | WatermarkAdvance | Both` — Output produced by a detector workflow.
- pub `from_context` function L56-69 — `(context: &cloacina_workflow::Context<serde_json::Value>) -> Option<Self>` — Extract `DetectorOutput` from a task output context.
- pub `boundaries` function L72-78 — `(&self) -> &[ComputationBoundary]` — Get all change boundaries from this output (empty for WatermarkAdvance-only).
- pub `watermark` function L81-87 — `(&self) -> Option<&ComputationBoundary>` — Get the watermark boundary if present.
-  `DetectorOutput` type L52-88 — `= DetectorOutput` — See CLOACI-S-0004 for the full specification.
-  `tests` module L91-154 — `-` — See CLOACI-S-0004 for the full specification.
-  `make_boundary` function L96-102 — `() -> ComputationBoundary` — See CLOACI-S-0004 for the full specification.
-  `test_detector_output_change_serialization` function L105-115 — `()` — See CLOACI-S-0004 for the full specification.
-  `test_detector_output_watermark_advance` function L118-124 — `()` — See CLOACI-S-0004 for the full specification.
-  `test_detector_output_both` function L127-134 — `()` — See CLOACI-S-0004 for the full specification.
-  `test_detector_output_from_context` function L137-147 — `()` — See CLOACI-S-0004 for the full specification.
-  `test_detector_output_from_context_missing` function L150-153 — `()` — See CLOACI-S-0004 for the full specification.

#### crates/cloacina/src/continuous/detector_state_store.rs

- pub `DetectorCheckpoint` struct L25-32 — `{ latest: Option<serde_json::Value>, committed: Option<serde_json::Value>, edge_...` — Per-source detector checkpoint with latest/committed split.
- pub `DetectorStateStore` struct L39-41 — `{ inner: Arc<RwLock<HashMap<String, DetectorCheckpoint>>> }` — Thread-safe in-memory store for detector state checkpoints.
- pub `new` function L44-48 — `() -> Self` — In-memory detector state store with committed/latest checkpoint tracking.
- pub `load_committed` function L51-55 — `(&self, source_name: &str, committed_state: serde_json::Value)` — Load committed states from DB on startup.
- pub `get_committed` function L58-61 — `(&self, source_name: &str) -> Option<serde_json::Value>` — Get the committed (safe-to-resume) state for a source.
- pub `update_latest` function L64-68 — `(&self, source_name: &str, state: serde_json::Value)` — Update the latest (not yet committed) state from a detector completion.
- pub `record_edge_drain` function L73-87 — `(&self, source_name: &str, edge_id: &str) -> Option<serde_json::Value>` — Record that an edge drained, capturing the current latest state for that edge.
- pub `commit` function L90-102 — `(&self, source_name: &str) -> Option<serde_json::Value>` — Commit: promote latest -> committed.
- pub `get_latest` function L105-108 — `(&self, source_name: &str) -> Option<serde_json::Value>` — Get the latest (uncommitted) state for a source.
-  `DetectorStateStore` type L43-109 — `= DetectorStateStore` — In-memory detector state store with committed/latest checkpoint tracking.
-  `DetectorStateStore` type L111-115 — `impl Default for DetectorStateStore` — In-memory detector state store with committed/latest checkpoint tracking.
-  `default` function L112-114 — `() -> Self` — In-memory detector state store with committed/latest checkpoint tracking.
-  `tests` module L118-288 — `-` — In-memory detector state store with committed/latest checkpoint tracking.
-  `test_update_latest_and_get_committed` function L123-137 — `()` — In-memory detector state store with committed/latest checkpoint tracking.
-  `test_commit_promotes_latest` function L140-148 — `()` — In-memory detector state store with committed/latest checkpoint tracking.
-  `test_update_without_commit_preserves_old_committed` function L151-165 — `()` — In-memory detector state store with committed/latest checkpoint tracking.
-  `test_load_committed_from_db` function L168-176 — `()` — In-memory detector state store with committed/latest checkpoint tracking.
-  `test_record_edge_drain` function L179-191 — `()` — In-memory detector state store with committed/latest checkpoint tracking.
-  `test_commit_clears_edge_drain_states` function L194-204 — `()` — In-memory detector state store with committed/latest checkpoint tracking.
-  `test_multiple_sources_independent` function L207-216 — `()` — In-memory detector state store with committed/latest checkpoint tracking.
-  `test_commit_gate_multi_edge_slowest_wins` function L219-246 — `()` — In-memory detector state store with committed/latest checkpoint tracking.
-  `test_commit_preserves_state_across_cycles` function L249-266 — `()` — In-memory detector state store with committed/latest checkpoint tracking.
-  `test_record_edge_drain_captures_latest_at_drain_time` function L269-287 — `()` — In-memory detector state store with committed/latest checkpoint tracking.

#### crates/cloacina/src/continuous/graph.rs

- pub `JoinMode` enum L34-39 — `Any | All` — How to combine accumulator readiness for multi-input tasks.
- pub `LateArrivalPolicy` enum L46-56 — `Discard | AccumulateForward | Retrigger` — Late arrival policy for boundaries arriving after consumer watermark.
- pub `GraphEdge` struct L65-74 — `{ source: String, task: String, accumulator: Arc<Mutex<Box<dyn SignalAccumulator...` — An edge in the continuous graph: data source → accumulator → task.
- pub `ContinuousTaskConfig` struct L88-95 — `{ triggered_edges: Vec<usize>, referenced_sources: Vec<String>, join_mode: JoinM...` — Configuration for a continuous task within the graph.
- pub `DataSourceGraph` struct L101-108 — `{ data_sources: HashMap<String, DataSource>, tasks: HashMap<String, ContinuousTa...` — The continuous reactive graph.
- pub `new` function L112-118 — `() -> Self` — Create a new empty graph.
- pub `edges_for_task` function L121-132 — `(&self, task_id: &str) -> Vec<&GraphEdge>` — Get all edges for a given task.
- pub `edges_for_source` function L135-140 — `(&self, source_name: &str) -> Vec<&GraphEdge>` — Get all edges for a given data source.
- pub `task_ids` function L143-145 — `(&self) -> Vec<&str>` — Get all task IDs in the graph.
- pub `ContinuousTaskRegistration` struct L169-176 — `{ id: String, sources: Vec<String>, referenced: Vec<String> }` — Registration for a continuous task (used during graph assembly).
- pub `GraphAssemblyError` enum L180-201 — `UnknownSource | UnknownDetectorWorkflow | DuplicateTask | DuplicateDetectorWorkf...` — Errors during graph assembly.
- pub `assemble_graph` function L207-343 — `( data_sources: Vec<DataSource>, task_registrations: Vec<ContinuousTaskRegistrat...` — Assemble a `DataSourceGraph` from registered data sources and task declarations.
-  `LateArrivalPolicy` type L58-62 — `impl Default for LateArrivalPolicy` — See CLOACI-S-0008 for the full specification.
-  `default` function L59-61 — `() -> Self` — See CLOACI-S-0008 for the full specification.
-  `GraphEdge` type L76-84 — `= GraphEdge` — See CLOACI-S-0008 for the full specification.
-  `fmt` function L77-83 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — See CLOACI-S-0008 for the full specification.
-  `DataSourceGraph` type L110-146 — `= DataSourceGraph` — See CLOACI-S-0008 for the full specification.
-  `DataSourceGraph` type L148-152 — `impl Default for DataSourceGraph` — See CLOACI-S-0008 for the full specification.
-  `default` function L149-151 — `() -> Self` — See CLOACI-S-0008 for the full specification.
-  `DataSourceGraph` type L154-165 — `= DataSourceGraph` — See CLOACI-S-0008 for the full specification.
-  `fmt` function L155-164 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — See CLOACI-S-0008 for the full specification.
-  `tests` module L346-634 — `-` — See CLOACI-S-0008 for the full specification.
-  `MockConnection` struct L353 — `-` — See CLOACI-S-0008 for the full specification.
-  `MockConnection` type L354-367 — `impl DataConnection for MockConnection` — See CLOACI-S-0008 for the full specification.
-  `connect` function L355-357 — `(&self) -> Result<Box<dyn Any>, DataConnectionError>` — See CLOACI-S-0008 for the full specification.
-  `descriptor` function L358-363 — `(&self) -> ConnectionDescriptor` — See CLOACI-S-0008 for the full specification.
-  `system_metadata` function L364-366 — `(&self) -> serde_json::Value` — See CLOACI-S-0008 for the full specification.
-  `make_data_source` function L369-376 — `(name: &str) -> DataSource` — See CLOACI-S-0008 for the full specification.
-  `test_assemble_simple_graph` function L379-396 — `()` — See CLOACI-S-0008 for the full specification.
-  `test_assemble_multi_source_task` function L399-410 — `()` — See CLOACI-S-0008 for the full specification.
-  `test_assemble_unknown_source` function L413-429 — `()` — See CLOACI-S-0008 for the full specification.
-  `test_assemble_unknown_referenced_source` function L432-442 — `()` — See CLOACI-S-0008 for the full specification.
-  `test_assemble_duplicate_task` function L445-466 — `()` — See CLOACI-S-0008 for the full specification.
-  `test_edges_for_task` function L469-480 — `()` — See CLOACI-S-0008 for the full specification.
-  `test_edges_for_source` function L483-501 — `()` — See CLOACI-S-0008 for the full specification.
-  `test_empty_graph` function L504-509 — `()` — See CLOACI-S-0008 for the full specification.
-  `make_data_source_with_detector` function L513-520 — `(name: &str, detector: &str) -> DataSource` — See CLOACI-S-0008 for the full specification.
-  `test_cycle_detection_simple_cycle` function L523-552 — `()` — See CLOACI-S-0008 for the full specification.
-  `test_diamond_dependency_is_valid` function L555-589 — `()` — See CLOACI-S-0008 for the full specification.
-  `test_linear_chain_is_valid` function L592-613 — `()` — See CLOACI-S-0008 for the full specification.
-  `test_shared_detector_workflow_allowed` function L616-633 — `()` — See CLOACI-S-0008 for the full specification.

#### crates/cloacina/src/continuous/ledger.rs

- pub `LedgerEvent` enum L34-58 — `TaskCompleted | TaskFailed | BoundaryEmitted | AccumulatorDrained` — Events recorded in the execution ledger.
- pub `task_name` function L62-69 — `(&self) -> Option<&str>` — Get the task name if this event is task-related.
- pub `is_task_completed` function L72-74 — `(&self) -> bool` — Returns true if this is a TaskCompleted event.
- pub `is_task_failed` function L77-79 — `(&self) -> bool` — Returns true if this is a TaskFailed event.
- pub `LedgerConfig` struct L87-90 — `{ max_events: usize }` — Configuration for the execution ledger.
- pub `ExecutionLedger` struct L111-120 — `{ events: VecDeque<LedgerEvent>, base_offset: usize, config: LedgerConfig, notif...` — In-memory log of graph activity with bounded size.
- pub `new` function L130-137 — `() -> Self` — Create a new empty ledger with default configuration.
- pub `with_config` function L140-147 — `(config: LedgerConfig) -> Self` — Create a new empty ledger with the given configuration.
- pub `subscribe` function L151-153 — `(&self) -> Arc<Notify>` — Get a handle to the notification channel.
- pub `append` function L157-173 — `(&mut self, event: LedgerEvent)` — Append an event to the ledger, evicting the oldest if at capacity.
- pub `events_since` function L180-192 — `(&self, cursor: usize) -> Vec<&LedgerEvent>` — Get all events since the given cursor position.
- pub `len` function L196-198 — `(&self) -> usize` — Get the current length (absolute index of next append).
- pub `is_empty` function L201-203 — `(&self) -> bool` — Check if the ledger is empty.
- pub `get` function L206-211 — `(&self, index: usize) -> Option<&LedgerEvent>` — Get a specific event by absolute index.
- pub `base_offset` function L214-216 — `(&self) -> usize` — Get the base offset (number of evicted events).
- pub `retained_count` function L219-221 — `(&self) -> usize` — Get the number of events currently retained.
-  `LedgerEvent` type L60-80 — `= LedgerEvent` — See CLOACI-S-0007 for the full specification.
-  `DEFAULT_MAX_EVENTS` variable L83 — `: usize` — Default maximum number of events in the ledger.
-  `LedgerConfig` type L92-98 — `impl Default for LedgerConfig` — See CLOACI-S-0007 for the full specification.
-  `default` function L93-97 — `() -> Self` — See CLOACI-S-0007 for the full specification.
-  `ExecutionLedger` type L122-126 — `impl Default for ExecutionLedger` — See CLOACI-S-0007 for the full specification.
-  `default` function L123-125 — `() -> Self` — See CLOACI-S-0007 for the full specification.
-  `ExecutionLedger` type L128-222 — `= ExecutionLedger` — See CLOACI-S-0007 for the full specification.
-  `tests` module L225-469 — `-` — See CLOACI-S-0007 for the full specification.
-  `make_completed_event` function L229-235 — `(task: &str) -> LedgerEvent` — See CLOACI-S-0007 for the full specification.
-  `make_failed_event` function L237-243 — `(task: &str, error: &str) -> LedgerEvent` — See CLOACI-S-0007 for the full specification.
-  `make_boundary_event` function L245-256 — `(source: &str) -> LedgerEvent` — See CLOACI-S-0007 for the full specification.
-  `test_ledger_append_and_len` function L259-270 — `()` — See CLOACI-S-0007 for the full specification.
-  `test_ledger_events_since` function L273-291 — `()` — See CLOACI-S-0007 for the full specification.
-  `test_ledger_cursor_advancement` function L294-318 — `()` — See CLOACI-S-0007 for the full specification.
-  `test_ledger_event_helpers` function L321-334 — `()` — See CLOACI-S-0007 for the full specification.
-  `test_ledger_get` function L337-343 — `()` — See CLOACI-S-0007 for the full specification.
-  `test_ledger_eviction_on_overflow` function L348-369 — `()` — See CLOACI-S-0007 for the full specification.
-  `test_ledger_cursor_adjustment_after_eviction` function L372-389 — `()` — See CLOACI-S-0007 for the full specification.
-  `test_ledger_cursor_before_base_offset_returns_all_retained` function L392-405 — `()` — See CLOACI-S-0007 for the full specification.
-  `test_ledger_heavy_eviction_stress` function L410-430 — `()` — See CLOACI-S-0007 for the full specification.
-  `test_ledger_cursor_tracking_through_eviction` function L433-460 — `()` — See CLOACI-S-0007 for the full specification.
-  `test_ledger_notify_on_append` function L463-468 — `()` — See CLOACI-S-0007 for the full specification.

#### crates/cloacina/src/continuous/ledger_trigger.rs

- pub `LedgerMatchMode` enum L36-41 — `Any | All` — How to match watched task completions.
- pub `LedgerTrigger` struct L48-65 — `{ trigger_name: String, watch_tasks: Vec<String>, match_mode: LedgerMatchMode, l...` — A trigger that watches the `ExecutionLedger` for task completions.
- pub `new` function L69-89 — `( trigger_name: String, watch_tasks: Vec<String>, match_mode: LedgerMatchMode, l...` — Create a new LedgerTrigger.
- pub `notify_handle` function L93-95 — `(&self) -> Option<&Arc<Notify>>` — Get the notification handle for event-driven wake-up.
-  `LedgerTrigger` type L67-96 — `= LedgerTrigger` — See CLOACI-S-0007 for the full specification.
-  `LedgerTrigger` type L99-169 — `impl Trigger for LedgerTrigger` — See CLOACI-S-0007 for the full specification.
-  `name` function L100-102 — `(&self) -> &str` — See CLOACI-S-0007 for the full specification.
-  `poll_interval` function L104-108 — `(&self) -> Duration` — See CLOACI-S-0007 for the full specification.
-  `allow_concurrent` function L110-112 — `(&self) -> bool` — See CLOACI-S-0007 for the full specification.
-  `poll` function L114-168 — `(&self) -> Result<TriggerResult, TriggerError>` — See CLOACI-S-0007 for the full specification.
-  `tests` module L172-326 — `-` — See CLOACI-S-0007 for the full specification.
-  `make_ledger_with_completions` function L177-187 — `(tasks: &[&str]) -> Arc<RwLock<ExecutionLedger>>` — See CLOACI-S-0007 for the full specification.
-  `test_any_mode_fires_on_single_match` function L190-201 — `()` — See CLOACI-S-0007 for the full specification.
-  `test_any_mode_skips_on_no_match` function L204-215 — `()` — See CLOACI-S-0007 for the full specification.
-  `test_all_mode_waits_for_all` function L218-244 — `()` — See CLOACI-S-0007 for the full specification.
-  `test_all_mode_resets_after_fire` function L247-263 — `()` — See CLOACI-S-0007 for the full specification.
-  `test_cursor_idempotency` function L266-296 — `()` — See CLOACI-S-0007 for the full specification.
-  `test_empty_ledger_skips` function L299-310 — `()` — See CLOACI-S-0007 for the full specification.
-  `test_trigger_metadata` function L313-325 — `()` — See CLOACI-S-0007 for the full specification.

#### crates/cloacina/src/continuous/mod.rs

- pub `accumulator` module L26 — `-` — Continuous reactive scheduling for Cloacina.
- pub `boundary` module L27 — `-` — See CLOACI-S-0001 for the full specification.
- pub `connections` module L28 — `-` — See CLOACI-S-0001 for the full specification.
- pub `datasource` module L29 — `-` — See CLOACI-S-0001 for the full specification.
- pub `detector` module L30 — `-` — See CLOACI-S-0001 for the full specification.
- pub `detector_state_store` module L31 — `-` — See CLOACI-S-0001 for the full specification.
- pub `graph` module L32 — `-` — See CLOACI-S-0001 for the full specification.
- pub `ledger` module L33 — `-` — See CLOACI-S-0001 for the full specification.
- pub `ledger_trigger` module L34 — `-` — See CLOACI-S-0001 for the full specification.
- pub `scheduler` module L35 — `-` — See CLOACI-S-0001 for the full specification.
- pub `state_management` module L36 — `-` — See CLOACI-S-0001 for the full specification.
- pub `trigger_policy` module L37 — `-` — See CLOACI-S-0001 for the full specification.
- pub `watermark` module L38 — `-` — See CLOACI-S-0001 for the full specification.

#### crates/cloacina/src/continuous/scheduler.rs

- pub `ContinuousSchedulerConfig` struct L40-50 — `{ poll_interval: Duration, max_fired_tasks: usize, task_timeout: Option<Duration...` — Configuration for the continuous scheduler.
- pub `ContinuousScheduler` struct L66-85 — `{ graph: DataSourceGraph, ledger: Arc<RwLock<ExecutionLedger>>, boundary_ledger:...` — The continuous reactive scheduler.
- pub `new` function L89-112 — `( graph: DataSourceGraph, ledger: Arc<RwLock<ExecutionLedger>>, config: Continuo...` — Create a new continuous scheduler.
- pub `register_task` function L118-122 — `(&mut self, task: Arc<dyn cloacina_workflow::Task>) -> &mut Self` — Register a continuous task implementation.
- pub `with_dal` function L128-131 — `(mut self, dal: Arc<crate::dal::DAL>) -> Self` — Enable accumulator state persistence via DAL.
- pub `detector_state_store` function L134-136 — `(&self) -> &DetectorStateStore` — Get a reference to the detector state store.
- pub `restore_from_persisted_state` function L143-214 — `(&self)` — Restore accumulator consumer watermarks from persisted state.
- pub `boundary_ledger` function L217-219 — `(&self) -> &Arc<RwLock<BoundaryLedger>>` — Get a reference to the boundary ledger (for WindowedAccumulator integration).
- pub `graph_metrics` function L222-235 — `(&self) -> Vec<super::accumulator::EdgeMetrics>` — Get per-edge accumulator metrics for observability.
- pub `add_exit_edge` function L238-243 — `(&mut self, task_id: String, workflow_name: String)` — Register an exit edge: when `task_id` completes, fire `workflow_name`.
- pub `restore_pending_boundaries` function L247-295 — `(&self)` — Restore pending boundaries from WAL into accumulators on startup.
- pub `restore_detector_state` function L298-330 — `(&self)` — Restore detector states from DB into the detector state store.
- pub `init_drain_cursors` function L333-349 — `(&self)` — Initialize edge drain cursors for all edges in the graph.
- pub `run` function L359-681 — `(&self, mut shutdown: watch::Receiver<bool>) -> Vec<FiredTask>` — Run the continuous scheduling loop.
- pub `FiredTask` struct L847-858 — `{ task_id: String, fired_at: chrono::DateTime<Utc>, boundary_context: Vec<cloaci...` — A task that was fired by the scheduler.
-  `ContinuousSchedulerConfig` type L52-60 — `impl Default for ContinuousSchedulerConfig` — See CLOACI-S-0008 for the full specification.
-  `default` function L53-59 — `() -> Self` — See CLOACI-S-0008 for the full specification.
-  `ContinuousScheduler` type L87-843 — `= ContinuousScheduler` — See CLOACI-S-0008 for the full specification.
-  `process_detector_output` function L684-775 — `(&self, detector_task: &str, output: &DetectorOutput)` — Process a detector output: route watermarks and boundaries.
-  `check_readiness` function L778-842 — `(&self) -> Vec<(String, Vec<cloacina_workflow::Context<serde_json::Value>>)>` — Check all tasks for readiness based on their JoinMode.
-  `ContinuousScheduler` type L860-869 — `= ContinuousScheduler` — See CLOACI-S-0008 for the full specification.
-  `fmt` function L861-868 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — See CLOACI-S-0008 for the full specification.
-  `tests` module L872-1487 — `-` — See CLOACI-S-0008 for the full specification.
-  `MockConn` struct L881 — `-` — See CLOACI-S-0008 for the full specification.
-  `MockConn` type L882-895 — `impl DataConnection for MockConn` — See CLOACI-S-0008 for the full specification.
-  `connect` function L883-885 — `(&self) -> Result<Box<dyn Any>, DataConnectionError>` — See CLOACI-S-0008 for the full specification.
-  `descriptor` function L886-891 — `(&self) -> ConnectionDescriptor` — See CLOACI-S-0008 for the full specification.
-  `system_metadata` function L892-894 — `(&self) -> serde_json::Value` — See CLOACI-S-0008 for the full specification.
-  `make_source` function L897-904 — `(name: &str) -> DataSource` — See CLOACI-S-0008 for the full specification.
-  `make_boundary` function L906-912 — `(start: i64, end: i64) -> ComputationBoundary` — See CLOACI-S-0008 for the full specification.
-  `test_scheduler_processes_detector_output` function L915-947 — `()` — See CLOACI-S-0008 for the full specification.
-  `test_scheduler_run_loop_with_shutdown` function L950-1004 — `()` — See CLOACI-S-0008 for the full specification.
-  `test_scheduler_empty_graph_runs_cleanly` function L1007-1028 — `()` — See CLOACI-S-0008 for the full specification.
-  `test_watermark_advance_updates_boundary_ledger` function L1031-1062 — `()` — See CLOACI-S-0008 for the full specification.
-  `test_both_output_routes_watermark_and_boundaries` function L1065-1102 — `()` — See CLOACI-S-0008 for the full specification.
-  `setup_scheduler_with_watermark` function L1108-1151 — `( policy: super::super::graph::LateArrivalPolicy, ) -> (ContinuousScheduler, Arc...` — Helper: create a scheduler, drain once to set consumer watermark,
-  `test_late_arrival_discard_drops_boundary` function L1154-1169 — `()` — See CLOACI-S-0008 for the full specification.
-  `test_late_arrival_accumulate_forward` function L1172-1188 — `()` — See CLOACI-S-0008 for the full specification.
-  `test_late_arrival_retrigger` function L1191-1207 — `()` — See CLOACI-S-0008 for the full specification.
-  `test_non_late_boundary_passes_through_regardless_of_policy` function L1213-1230 — `()` — See CLOACI-S-0008 for the full specification.
-  `RealTask` struct L1235-1237 — `{ id: String }` — A test task that writes to context proving it ran.
-  `RealTask` type L1240-1256 — `= RealTask` — See CLOACI-S-0008 for the full specification.
-  `execute` function L1241-1249 — `( &self, mut context: cloacina_workflow::Context<serde_json::Value>, ) -> Result...` — See CLOACI-S-0008 for the full specification.
-  `id` function L1250-1252 — `(&self) -> &str` — See CLOACI-S-0008 for the full specification.
-  `dependencies` function L1253-1255 — `(&self) -> &[cloacina_workflow::TaskNamespace]` — See CLOACI-S-0008 for the full specification.
-  `test_scheduler_actually_executes_registered_task` function L1259-1335 — `()` — See CLOACI-S-0008 for the full specification.
-  `test_scheduler_handles_task_failure` function L1338-1429 — `()` — See CLOACI-S-0008 for the full specification.
-  `FailingTask` struct L1339 — `-` — See CLOACI-S-0008 for the full specification.
-  `FailingTask` type L1342-1360 — `= FailingTask` — See CLOACI-S-0008 for the full specification.
-  `execute` function L1343-1353 — `( &self, _context: cloacina_workflow::Context<serde_json::Value>, ) -> Result<cl...` — See CLOACI-S-0008 for the full specification.
-  `id` function L1354-1356 — `(&self) -> &str` — See CLOACI-S-0008 for the full specification.
-  `dependencies` function L1357-1359 — `(&self) -> &[cloacina_workflow::TaskNamespace]` — See CLOACI-S-0008 for the full specification.
-  `test_unregistered_task_records_not_executed` function L1432-1486 — `()` — See CLOACI-S-0008 for the full specification.

#### crates/cloacina/src/continuous/state_management.rs

- pub `list_orphaned_states` function L29-49 — `( graph: &DataSourceGraph, dal: &DAL, ) -> Result<Vec<String>, String>` — List orphaned accumulator state edge IDs.
- pub `prune_orphaned_states` function L54-62 — `(graph: &DataSourceGraph, dal: &DAL) -> Result<usize, String>` — Prune (delete) orphaned accumulator states.

#### crates/cloacina/src/continuous/trigger_policy.rs

- pub `TriggerPolicy` interface L32-40 — `{ fn should_fire(), fn mark_drained() }` — Trait controlling when an accumulator should fire.
- pub `Immediate` struct L43 — `-` — Fires on every boundary — as soon as the buffer is non-empty.
- pub `WallClockWindow` struct L56-61 — `{ duration: Duration, last_drain_at: Instant }` — Fires when wall clock time since last drain exceeds a configured duration.
- pub `new` function L65-70 — `(duration: Duration) -> Self` — Create a new WallClockWindow policy with the given duration.
- pub `AnyPolicy` struct L95 — `-` — Fires when ANY sub-policy returns true (OR combinator).
- pub `AllPolicy` struct L118 — `-` — Fires when ALL sub-policies return true (AND combinator).
- pub `BoundaryCount` struct L133-136 — `{ count: usize }` — Fires when N boundaries are buffered.
- pub `new` function L140-142 — `(count: usize) -> Self` — Create a new BoundaryCount policy.
- pub `WallClockDebounce` struct L156-159 — `{ chrono_duration: chrono::Duration }` — Fires when no new boundary has been received for `duration` (debounce).
- pub `try_new` function L167-171 — `(duration: Duration) -> Result<Self, String>` — Create a new WallClockDebounce policy.
- pub `new` function L177-179 — `(duration: Duration) -> Self` — Create a new WallClockDebounce policy (panics on invalid duration).
-  `mark_drained` function L39 — `(&mut self)` — Notify the policy that a drain occurred.
-  `Immediate` type L45-49 — `impl TriggerPolicy for Immediate` — See CLOACI-S-0005 for the full specification.
-  `should_fire` function L46-48 — `(&self, buffer: &[BufferedBoundary]) -> bool` — See CLOACI-S-0005 for the full specification.
-  `WallClockWindow` type L63-71 — `= WallClockWindow` — See CLOACI-S-0005 for the full specification.
-  `WallClockWindow` type L73-84 — `impl TriggerPolicy for WallClockWindow` — See CLOACI-S-0005 for the full specification.
-  `should_fire` function L74-79 — `(&self, buffer: &[BufferedBoundary]) -> bool` — See CLOACI-S-0005 for the full specification.
-  `mark_drained` function L81-83 — `(&mut self)` — See CLOACI-S-0005 for the full specification.
-  `AnyPolicy` type L97-107 — `impl TriggerPolicy for AnyPolicy` — See CLOACI-S-0005 for the full specification.
-  `should_fire` function L98-100 — `(&self, buffer: &[BufferedBoundary]) -> bool` — See CLOACI-S-0005 for the full specification.
-  `mark_drained` function L102-106 — `(&mut self)` — See CLOACI-S-0005 for the full specification.
-  `AllPolicy` type L120-130 — `impl TriggerPolicy for AllPolicy` — See CLOACI-S-0005 for the full specification.
-  `should_fire` function L121-123 — `(&self, buffer: &[BufferedBoundary]) -> bool` — See CLOACI-S-0005 for the full specification.
-  `mark_drained` function L125-129 — `(&mut self)` — See CLOACI-S-0005 for the full specification.
-  `BoundaryCount` type L138-143 — `= BoundaryCount` — See CLOACI-S-0005 for the full specification.
-  `BoundaryCount` type L145-149 — `impl TriggerPolicy for BoundaryCount` — See CLOACI-S-0005 for the full specification.
-  `should_fire` function L146-148 — `(&self, buffer: &[BufferedBoundary]) -> bool` — See CLOACI-S-0005 for the full specification.
-  `WallClockDebounce` type L161-180 — `= WallClockDebounce` — See CLOACI-S-0005 for the full specification.
-  `WallClockDebounce` type L182-191 — `impl TriggerPolicy for WallClockDebounce` — See CLOACI-S-0005 for the full specification.
-  `should_fire` function L183-190 — `(&self, buffer: &[BufferedBoundary]) -> bool` — See CLOACI-S-0005 for the full specification.
-  `tests` module L194-363 — `-` — See CLOACI-S-0005 for the full specification.
-  `make_buffered` function L199-207 — `() -> BufferedBoundary` — See CLOACI-S-0005 for the full specification.
-  `test_immediate_fires_on_non_empty` function L210-213 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_immediate_does_not_fire_on_empty` function L216-219 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_wall_clock_window_fires_after_duration` function L222-228 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_wall_clock_window_does_not_fire_early` function L231-234 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_wall_clock_window_does_not_fire_on_empty` function L237-243 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_wall_clock_window_mark_drained` function L246-256 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_boundary_count_fires_at_threshold` function L261-265 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_boundary_count_does_not_fire_below` function L268-272 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_boundary_count_fires_above` function L275-279 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_debounce_fires_after_silence` function L284-291 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_debounce_does_not_fire_during_burst` function L294-298 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_debounce_empty_buffer` function L301-304 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_any_fires_when_one_matches` function L309-315 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_any_does_not_fire_when_none_match` function L318-324 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_all_fires_when_all_match` function L329-332 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_all_does_not_fire_when_one_fails` function L335-341 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_all_empty_policies_does_not_fire` function L344-347 — `()` — See CLOACI-S-0005 for the full specification.
-  `test_nested_any_all` function L352-362 — `()` — See CLOACI-S-0005 for the full specification.

#### crates/cloacina/src/continuous/watermark.rs

- pub `WatermarkError` enum L30-37 — `BackwardMovement | IncompatibleKinds` — Error from watermark operations.
- pub `BoundaryLedger` struct L46-48 — `{ watermarks: HashMap<String, ComputationBoundary> }` — In-memory source watermark store.
- pub `new` function L52-56 — `() -> Self` — Create a new empty ledger.
- pub `advance` function L62-76 — `( &mut self, source_name: &str, watermark: ComputationBoundary, ) -> Result<(), ...` — Advance the watermark for a data source.
- pub `covers` function L81-86 — `(&self, source_name: &str, boundary: &ComputationBoundary) -> bool` — Does the watermark for this source cover the given boundary?
- pub `watermark` function L89-91 — `(&self, source_name: &str) -> Option<&ComputationBoundary>` — Get the current watermark for a data source.
- pub `sources` function L94-96 — `(&self) -> impl Iterator<Item = &str>` — Get all tracked source names.
-  `BoundaryLedger` type L50-97 — `= BoundaryLedger` — See CLOACI-S-0006 for the full specification.
-  `is_backward` function L106-146 — `( source_name: &str, existing: &ComputationBoundary, proposed: &ComputationBound...` — Check if a new watermark would be a backward movement.
-  `boundary_covered` function L154-174 — `(watermark: &ComputationBoundary, boundary: &ComputationBoundary) -> bool` — Check if a watermark covers a boundary.
-  `tests` module L177-503 — `-` — See CLOACI-S-0006 for the full specification.
-  `time_boundary` function L181-191 — `(end_offset_hours: i64) -> ComputationBoundary` — See CLOACI-S-0006 for the full specification.
-  `offset_boundary` function L193-199 — `(start: i64, end: i64) -> ComputationBoundary` — See CLOACI-S-0006 for the full specification.
-  `cursor_boundary` function L201-209 — `(value: &str) -> ComputationBoundary` — See CLOACI-S-0006 for the full specification.
-  `cursor_boundary_at` function L211-219 — `(value: &str, emitted_at: chrono::DateTime<Utc>) -> ComputationBoundary` — See CLOACI-S-0006 for the full specification.
-  `fullstate_boundary` function L221-229 — `(value: &str) -> ComputationBoundary` — See CLOACI-S-0006 for the full specification.
-  `fullstate_boundary_at` function L231-242 — `( value: &str, emitted_at: chrono::DateTime<Utc>, ) -> ComputationBoundary` — See CLOACI-S-0006 for the full specification.
-  `test_advance_first_watermark_succeeds` function L247-251 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_advance_forward_succeeds` function L254-258 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_advance_backward_rejected_offset` function L261-272 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_advance_same_value_accepted` function L275-279 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_advance_cursor_forward_accepted` function L282-295 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_advance_cursor_backward_rejected` function L298-310 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_advance_fullstate_forward_accepted` function L313-325 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_advance_fullstate_backward_rejected` function L328-339 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_advance_fullstate_same_emitted_at_accepted` function L342-352 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_covers_offset_within_watermark` function L357-362 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_covers_offset_beyond_watermark` function L365-369 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_covers_missing_source` function L372-375 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_covers_cursor_by_timestamp` function L378-397 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_covers_fullstate_by_timestamp` function L400-416 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_covers_different_kinds_returns_false` function L419-423 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_watermark_returns_none_for_unknown` function L428-431 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_sources_iterator` function L434-441 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_watermark_kind_mixing_rejected` function L446-459 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_watermark_same_kind_different_values_ok` function L462-470 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_watermark_backward_movement_rejected` function L473-482 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_watermark_monotonicity_many_advances` function L485-494 — `()` — See CLOACI-S-0006 for the full specification.
-  `test_covers_cross_kind_returns_false` function L497-502 — `()` — See CLOACI-S-0006 for the full specification.

### crates/cloacina/src/continuous/connections

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/continuous/connections/kafka.rs

- pub `KafkaConnectionConfig` struct L26-31 — `{ brokers: Vec<String>, topic: String, partition: Option<i32>, consumer_group: O...` — Kafka connection configuration returned by `connect()`.
- pub `KafkaConnection` struct L35-40 — `{ brokers: Vec<String>, topic: String, partition: Option<i32>, consumer_group: O...` — A Kafka data connection for continuous scheduling.
- pub `new` function L43-50 — `(brokers: Vec<String>, topic: &str) -> Self` — Kafka `DataConnection` implementation.
- pub `with_partition` function L52-55 — `(mut self, partition: i32) -> Self` — Kafka `DataConnection` implementation.
- pub `with_consumer_group` function L57-60 — `(mut self, group: &str) -> Self` — Kafka `DataConnection` implementation.
-  `KafkaConnection` type L42-61 — `= KafkaConnection` — Kafka `DataConnection` implementation.
-  `KafkaConnection` type L63-88 — `impl DataConnection for KafkaConnection` — Kafka `DataConnection` implementation.
-  `connect` function L64-71 — `(&self) -> Result<Box<dyn Any>, DataConnectionError>` — Kafka `DataConnection` implementation.
-  `descriptor` function L73-78 — `(&self) -> ConnectionDescriptor` — Kafka `DataConnection` implementation.
-  `system_metadata` function L80-87 — `(&self) -> serde_json::Value` — Kafka `DataConnection` implementation.
-  `tests` module L91-122 — `-` — Kafka `DataConnection` implementation.
-  `test_kafka_descriptor` function L95-101 — `()` — Kafka `DataConnection` implementation.
-  `test_kafka_metadata` function L104-112 — `()` — Kafka `DataConnection` implementation.
-  `test_kafka_connect_returns_config` function L115-121 — `()` — Kafka `DataConnection` implementation.

#### crates/cloacina/src/continuous/connections/mod.rs

- pub `kafka` module L19 — `-` — Framework-provided `DataConnection` implementations.
- pub `postgres` module L21 — `-` — Framework-provided `DataConnection` implementations.
- pub `s3` module L22 — `-` — Framework-provided `DataConnection` implementations.

#### crates/cloacina/src/continuous/connections/postgres.rs

- pub `PostgresPoolConfig` struct L29-36 — `{ url: String, max_connections: u32, min_connections: u32 }` — Pool configuration returned by `connect()`.
- pub `PostgresConnection` struct L44-61 — `{ host: String, port: u16, database: String, schema: String, table: String, user...` — A PostgreSQL data connection for continuous scheduling.
- pub `new` function L65-76 — `(host: &str, port: u16, database: &str, schema: &str, table: &str) -> Self` — Create a new PostgresConnection.
- pub `with_username` function L79-82 — `(mut self, username: &str) -> Self` — Set the username for the connection URL.
- pub `with_max_connections` function L85-88 — `(mut self, max: u32) -> Self` — Set the maximum number of connections in the pool.
- pub `with_min_connections` function L91-94 — `(mut self, min: u32) -> Self` — Set the minimum number of idle connections in the pool.
- pub `connection_url` function L97-105 — `(&self) -> String` — Build the connection URL.
-  `PostgresConnection` type L63-106 — `= PostgresConnection` — PostgreSQL `DataConnection` implementation.
-  `PostgresConnection` type L108-138 — `impl DataConnection for PostgresConnection` — PostgreSQL `DataConnection` implementation.
-  `connect` function L109-118 — `(&self) -> Result<Box<dyn Any>, DataConnectionError>` — PostgreSQL `DataConnection` implementation.
-  `descriptor` function L120-125 — `(&self) -> ConnectionDescriptor` — PostgreSQL `DataConnection` implementation.
-  `system_metadata` function L127-137 — `(&self) -> serde_json::Value` — PostgreSQL `DataConnection` implementation.
-  `tests` module L141-197 — `-` — PostgreSQL `DataConnection` implementation.
-  `test_postgres_connection_descriptor` function L145-150 — `()` — PostgreSQL `DataConnection` implementation.
-  `test_postgres_connection_metadata` function L153-160 — `()` — PostgreSQL `DataConnection` implementation.
-  `test_postgres_connection_returns_pool_config` function L163-172 — `()` — PostgreSQL `DataConnection` implementation.
-  `test_postgres_connection_with_username` function L175-181 — `()` — PostgreSQL `DataConnection` implementation.
-  `test_postgres_connection_url` function L184-187 — `()` — PostgreSQL `DataConnection` implementation.
-  `test_default_pool_settings` function L190-196 — `()` — PostgreSQL `DataConnection` implementation.

#### crates/cloacina/src/continuous/connections/s3.rs

- pub `S3ConnectionConfig` struct L26-30 — `{ bucket: String, prefix: String, region: String }` — S3 connection configuration returned by `connect()`.
- pub `S3Connection` struct L34-38 — `{ bucket: String, prefix: String, region: String }` — An S3 data connection for continuous scheduling.
- pub `new` function L41-47 — `(bucket: &str, prefix: &str, region: &str) -> Self` — S3 `DataConnection` implementation.
-  `S3Connection` type L40-48 — `= S3Connection` — S3 `DataConnection` implementation.
-  `S3Connection` type L50-73 — `impl DataConnection for S3Connection` — S3 `DataConnection` implementation.
-  `connect` function L51-57 — `(&self) -> Result<Box<dyn Any>, DataConnectionError>` — S3 `DataConnection` implementation.
-  `descriptor` function L59-64 — `(&self) -> ConnectionDescriptor` — S3 `DataConnection` implementation.
-  `system_metadata` function L66-72 — `(&self) -> serde_json::Value` — S3 `DataConnection` implementation.
-  `tests` module L76-104 — `-` — S3 `DataConnection` implementation.
-  `test_s3_descriptor` function L80-85 — `()` — S3 `DataConnection` implementation.
-  `test_s3_metadata` function L88-94 — `()` — S3 `DataConnection` implementation.
-  `test_s3_connect_returns_config` function L97-103 — `()` — S3 `DataConnection` implementation.

### crates/cloacina/src/crypto

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/crypto/key_encryption.rs

- pub `KeyEncryptionError` enum L36-48 — `EncryptionFailed | DecryptionFailed | InvalidKeyLength | InvalidEncryptedData` — Errors that can occur during key encryption/decryption.
- pub `encrypt_private_key` function L67-94 — `( private_key: &[u8], encryption_key: &[u8], ) -> Result<Vec<u8>, KeyEncryptionE...` — Encrypts an Ed25519 private key using AES-256-GCM.
- pub `decrypt_private_key` function L110-136 — `( encrypted_data: &[u8], encryption_key: &[u8], ) -> Result<Vec<u8>, KeyEncrypti...` — Decrypts an Ed25519 private key that was encrypted with AES-256-GCM.
-  `NONCE_SIZE` variable L51 — `: usize` — Size of the AES-256-GCM nonce in bytes.
-  `tests` module L139-206 — `-` — - A key management service (KMS)
-  `test_encrypt_decrypt_roundtrip` function L143-155 — `()` — - A key management service (KMS)
-  `test_wrong_key_fails` function L158-167 — `()` — - A key management service (KMS)
-  `test_invalid_key_length` function L170-179 — `()` — - A key management service (KMS)
-  `test_invalid_encrypted_data` function L182-191 — `()` — - A key management service (KMS)
-  `test_tampered_ciphertext_fails` function L194-205 — `()` — - A key management service (KMS)

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
- pub `compute_package_hash` function L180-185 — `(data: &[u8]) -> String` — Computes the SHA256 hash of package data.
-  `tests` module L188-285 — `-` — - Verifying signatures
-  `test_generate_keypair` function L192-198 — `()` — - Verifying signatures
-  `test_sign_and_verify` function L201-212 — `()` — - Verifying signatures
-  `test_verify_wrong_key_fails` function L215-225 — `()` — - Verifying signatures
-  `test_verify_tampered_data_fails` function L228-238 — `()` — - Verifying signatures
-  `test_fingerprint_is_deterministic` function L241-248 — `()` — - Verifying signatures
-  `test_invalid_key_lengths` function L251-271 — `()` — - Verifying signatures
-  `test_compute_package_hash` function L274-284 — `()` — - Verifying signatures

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

### crates/cloacina/src/dal/unified

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/dal/unified/accumulator_state.rs

- pub `AccumulatorStateDAL` struct L26-28 — `{ dal: &'a DAL }` — Data access layer for accumulator state operations.
- pub `new` function L31-33 — `(dal: &'a DAL) -> Self` — DAL for accumulator state persistence (continuous scheduling).
- pub `save` function L36-42 — `(&self, state: NewAccumulatorState) -> Result<(), String>` — Save or update accumulator state for an edge.
- pub `load` function L45-52 — `(&self, edge_id: &str) -> Result<Option<AccumulatorStateRow>, String>` — Load accumulator state for a specific edge.
- pub `load_all` function L55-61 — `(&self) -> Result<Vec<AccumulatorStateRow>, String>` — Load all persisted accumulator states.
- pub `delete_by_ids` function L64-70 — `(&self, edge_ids: Vec<String>) -> Result<usize, String>` — Delete accumulator states by edge IDs.
-  `save_postgres` function L75-98 — `(&self, state: NewAccumulatorState) -> Result<(), String>` — DAL for accumulator state persistence (continuous scheduling).
-  `load_postgres` function L101-118 — `(&self, edge_id: String) -> Result<Option<AccumulatorStateRow>, String>` — DAL for accumulator state persistence (continuous scheduling).
-  `load_all_postgres` function L121-136 — `(&self) -> Result<Vec<AccumulatorStateRow>, String>` — DAL for accumulator state persistence (continuous scheduling).
-  `delete_postgres` function L139-156 — `(&self, edge_ids: Vec<String>) -> Result<usize, String>` — DAL for accumulator state persistence (continuous scheduling).
-  `save_sqlite` function L161-178 — `(&self, state: NewAccumulatorState) -> Result<(), String>` — DAL for accumulator state persistence (continuous scheduling).
-  `load_sqlite` function L181-198 — `(&self, edge_id: String) -> Result<Option<AccumulatorStateRow>, String>` — DAL for accumulator state persistence (continuous scheduling).
-  `load_all_sqlite` function L201-216 — `(&self) -> Result<Vec<AccumulatorStateRow>, String>` — DAL for accumulator state persistence (continuous scheduling).
-  `delete_sqlite` function L219-236 — `(&self, edge_ids: Vec<String>) -> Result<usize, String>` — DAL for accumulator state persistence (continuous scheduling).

#### crates/cloacina/src/dal/unified/api_key_dal.rs

- pub `ApiKeyDAL` struct L27-29 — `{ dal: &'a DAL }` — Data access layer for API key operations.
- pub `new` function L32-34 — `(dal: &'a DAL) -> Self` — DAL for API key operations (auth system).
- pub `create` function L37-43 — `(&self, new_key: NewApiKey) -> Result<(), String>` — Create a new API key.
- pub `create_patterns` function L46-52 — `(&self, patterns: Vec<NewWorkflowPattern>) -> Result<(), String>` — Create workflow patterns in batch.
- pub `load_by_prefix` function L55-65 — `( &self, prefix: &str, ) -> Result<Vec<(ApiKeyRow, Vec<WorkflowPatternRow>)>, St...` — Load API keys by prefix, along with their workflow patterns.
- pub `list_by_tenant` function L68-74 — `(&self, tenant_id: UniversalUuid) -> Result<Vec<ApiKeyRow>, String>` — List API keys for a specific tenant.
- pub `list_all` function L77-83 — `(&self) -> Result<Vec<ApiKeyRow>, String>` — List all API keys (across all tenants).
- pub `revoke` function L86-92 — `(&self, key_id: UniversalUuid) -> Result<(), String>` — Revoke an API key by setting revoked_at to now.
-  `create_postgres` function L97-114 — `(&self, new_key: NewApiKey) -> Result<(), String>` — DAL for API key operations (auth system).
-  `create_patterns_postgres` function L117-137 — `( &self, patterns: Vec<NewWorkflowPattern>, ) -> Result<(), String>` — DAL for API key operations (auth system).
-  `load_by_prefix_postgres` function L140-180 — `( &self, prefix: String, ) -> Result<Vec<(ApiKeyRow, Vec<WorkflowPatternRow>)>, ...` — DAL for API key operations (auth system).
-  `list_by_tenant_postgres` function L183-202 — `( &self, tenant_id: UniversalUuid, ) -> Result<Vec<ApiKeyRow>, String>` — DAL for API key operations (auth system).
-  `list_all_postgres` function L205-221 — `(&self) -> Result<Vec<ApiKeyRow>, String>` — DAL for API key operations (auth system).
-  `revoke_postgres` function L224-244 — `(&self, key_id: UniversalUuid) -> Result<(), String>` — DAL for API key operations (auth system).
-  `create_sqlite` function L249-266 — `(&self, new_key: NewApiKey) -> Result<(), String>` — DAL for API key operations (auth system).
-  `create_patterns_sqlite` function L269-291 — `( &self, patterns: Vec<NewWorkflowPattern>, ) -> Result<(), String>` — DAL for API key operations (auth system).
-  `load_by_prefix_sqlite` function L294-326 — `( &self, prefix: String, ) -> Result<Vec<(ApiKeyRow, Vec<WorkflowPatternRow>)>, ...` — DAL for API key operations (auth system).
-  `list_by_tenant_sqlite` function L329-348 — `( &self, tenant_id: UniversalUuid, ) -> Result<Vec<ApiKeyRow>, String>` — DAL for API key operations (auth system).
-  `list_all_sqlite` function L351-367 — `(&self) -> Result<Vec<ApiKeyRow>, String>` — DAL for API key operations (auth system).
-  `revoke_sqlite` function L370-390 — `(&self, key_id: UniversalUuid) -> Result<(), String>` — DAL for API key operations (auth system).

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

#### crates/cloacina/src/dal/unified/detector_state_dal.rs

- pub `DetectorStateDAL` struct L26-28 — `{ dal: &'a DAL }` — Data access layer for detector state operations.
- pub `new` function L31-33 — `(dal: &'a DAL) -> Self` — DAL for detector state persistence (continuous scheduling).
- pub `save` function L36-42 — `(&self, state: NewDetectorState) -> Result<(), String>` — Save or update detector state for a source.
- pub `load` function L45-52 — `(&self, source_name: &str) -> Result<Option<DetectorStateRow>, String>` — Load detector state for a specific source.
- pub `load_all` function L55-61 — `(&self) -> Result<Vec<DetectorStateRow>, String>` — Load all persisted detector states.
-  `save_postgres` function L66-86 — `(&self, state: NewDetectorState) -> Result<(), String>` — DAL for detector state persistence (continuous scheduling).
-  `load_postgres` function L89-106 — `(&self, source_name: String) -> Result<Option<DetectorStateRow>, String>` — DAL for detector state persistence (continuous scheduling).
-  `load_all_postgres` function L109-124 — `(&self) -> Result<Vec<DetectorStateRow>, String>` — DAL for detector state persistence (continuous scheduling).
-  `save_sqlite` function L129-146 — `(&self, state: NewDetectorState) -> Result<(), String>` — DAL for detector state persistence (continuous scheduling).
-  `load_sqlite` function L149-166 — `(&self, source_name: String) -> Result<Option<DetectorStateRow>, String>` — DAL for detector state persistence (continuous scheduling).
-  `load_all_sqlite` function L169-184 — `(&self) -> Result<Vec<DetectorStateRow>, String>` — DAL for detector state persistence (continuous scheduling).

#### crates/cloacina/src/dal/unified/execution_event.rs

- pub `ExecutionEventDAL` struct L39-41 — `{ dal: &'a DAL }` — Data access layer for execution event operations with runtime backend selection.
- pub `new` function L45-47 — `(dal: &'a DAL) -> Self` — Creates a new ExecutionEventDAL instance.
- pub `create` function L53-62 — `( &self, new_event: NewExecutionEvent, ) -> Result<ExecutionEvent, ValidationErr...` — Creates a new execution event record.
- pub `list_by_pipeline` function L148-157 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<Vec<ExecutionEvent>, ...` — Gets all execution events for a specific pipeline execution, ordered by sequence.
- pub `list_by_task` function L210-219 — `( &self, task_execution_id: UniversalUuid, ) -> Result<Vec<ExecutionEvent>, Vali...` — Gets all execution events for a specific task execution, ordered by sequence.
- pub `list_by_type` function L272-282 — `( &self, event_type: ExecutionEventType, limit: i64, ) -> Result<Vec<ExecutionEv...` — Gets execution events by type for monitoring and analysis.
- pub `get_recent` function L341-347 — `(&self, limit: i64) -> Result<Vec<ExecutionEvent>, ValidationError>` — Gets recent execution events for monitoring purposes.
- pub `delete_older_than` function L400-409 — `( &self, cutoff: UniversalTimestamp, ) -> Result<usize, ValidationError>` — Deletes execution events older than the specified timestamp.
- pub `count_by_pipeline` function L462-471 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<i64, ValidationError>` — Counts total execution events for a pipeline.
- pub `count_older_than` function L526-535 — `( &self, cutoff: UniversalTimestamp, ) -> Result<i64, ValidationError>` — Counts execution events older than the specified timestamp.
-  `create_postgres` function L65-99 — `( &self, new_event: NewExecutionEvent, ) -> Result<ExecutionEvent, ValidationErr...` — state transitions for debugging, compliance, and replay capability.
-  `create_sqlite` function L102-145 — `( &self, new_event: NewExecutionEvent, ) -> Result<ExecutionEvent, ValidationErr...` — state transitions for debugging, compliance, and replay capability.
-  `list_by_pipeline_postgres` function L160-182 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<Vec<ExecutionEvent>, ...` — state transitions for debugging, compliance, and replay capability.
-  `list_by_pipeline_sqlite` function L185-207 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<Vec<ExecutionEvent>, ...` — state transitions for debugging, compliance, and replay capability.
-  `list_by_task_postgres` function L222-244 — `( &self, task_execution_id: UniversalUuid, ) -> Result<Vec<ExecutionEvent>, Vali...` — state transitions for debugging, compliance, and replay capability.
-  `list_by_task_sqlite` function L247-269 — `( &self, task_execution_id: UniversalUuid, ) -> Result<Vec<ExecutionEvent>, Vali...` — state transitions for debugging, compliance, and replay capability.
-  `list_by_type_postgres` function L285-310 — `( &self, event_type: ExecutionEventType, limit: i64, ) -> Result<Vec<ExecutionEv...` — state transitions for debugging, compliance, and replay capability.
-  `list_by_type_sqlite` function L313-338 — `( &self, event_type: ExecutionEventType, limit: i64, ) -> Result<Vec<ExecutionEv...` — state transitions for debugging, compliance, and replay capability.
-  `get_recent_postgres` function L350-372 — `( &self, limit: i64, ) -> Result<Vec<ExecutionEvent>, ValidationError>` — state transitions for debugging, compliance, and replay capability.
-  `get_recent_sqlite` function L375-394 — `(&self, limit: i64) -> Result<Vec<ExecutionEvent>, ValidationError>` — state transitions for debugging, compliance, and replay capability.
-  `delete_older_than_postgres` function L412-434 — `( &self, cutoff: UniversalTimestamp, ) -> Result<usize, ValidationError>` — state transitions for debugging, compliance, and replay capability.
-  `delete_older_than_sqlite` function L437-459 — `( &self, cutoff: UniversalTimestamp, ) -> Result<usize, ValidationError>` — state transitions for debugging, compliance, and replay capability.
-  `count_by_pipeline_postgres` function L474-496 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<i64, ValidationError>` — state transitions for debugging, compliance, and replay capability.
-  `count_by_pipeline_sqlite` function L499-521 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<i64, ValidationError>` — state transitions for debugging, compliance, and replay capability.
-  `count_older_than_postgres` function L538-560 — `( &self, cutoff: UniversalTimestamp, ) -> Result<i64, ValidationError>` — state transitions for debugging, compliance, and replay capability.
-  `count_older_than_sqlite` function L563-585 — `( &self, cutoff: UniversalTimestamp, ) -> Result<i64, ValidationError>` — state transitions for debugging, compliance, and replay capability.

#### crates/cloacina/src/dal/unified/mod.rs

- pub `accumulator_state` module L46 — `-` — ```
- pub `api_key_dal` module L47 — `-` — ```
- pub `context` module L48 — `-` — ```
- pub `cron_execution` module L49 — `-` — ```
- pub `cron_schedule` module L50 — `-` — ```
- pub `detector_state_dal` module L51 — `-` — ```
- pub `execution_event` module L52 — `-` — ```
- pub `models` module L53 — `-` — ```
- pub `pending_boundary_dal` module L54 — `-` — ```
- pub `pipeline_execution` module L55 — `-` — ```
- pub `recovery_event` module L56 — `-` — ```
- pub `task_execution` module L57 — `-` — ```
- pub `task_execution_metadata` module L58 — `-` — ```
- pub `task_outbox` module L59 — `-` — ```
- pub `tenant_dal` module L60 — `-` — ```
- pub `trigger_execution` module L61 — `-` — ```
- pub `trigger_schedule` module L62 — `-` — ```
- pub `workflow_packages` module L63 — `-` — ```
- pub `workflow_registry` module L64 — `-` — ```
- pub `workflow_registry_storage` module L65 — `-` — ```
- pub `DAL` struct L176-179 — `{ database: Database }` — The unified Data Access Layer struct.
- pub `new` function L191-193 — `(database: Database) -> Self` — Creates a new unified DAL instance.
- pub `backend` function L196-198 — `(&self) -> BackendType` — Returns the backend type for this DAL instance.
- pub `database` function L201-203 — `(&self) -> &Database` — Returns a reference to the underlying database.
- pub `pool` function L206-208 — `(&self) -> AnyPool` — Returns the connection pool.
- pub `context` function L211-213 — `(&self) -> ContextDAL<'_>` — Returns a context DAL for context operations.
- pub `pipeline_execution` function L216-218 — `(&self) -> PipelineExecutionDAL<'_>` — Returns a pipeline execution DAL for pipeline operations.
- pub `task_execution` function L221-223 — `(&self) -> TaskExecutionDAL<'_>` — Returns a task execution DAL for task operations.
- pub `task_execution_metadata` function L226-228 — `(&self) -> TaskExecutionMetadataDAL<'_>` — Returns a task execution metadata DAL for metadata operations.
- pub `task_outbox` function L231-233 — `(&self) -> TaskOutboxDAL<'_>` — Returns a task outbox DAL for work distribution operations.
- pub `recovery_event` function L236-238 — `(&self) -> RecoveryEventDAL<'_>` — Returns a recovery event DAL for recovery operations.
- pub `execution_event` function L241-243 — `(&self) -> ExecutionEventDAL<'_>` — Returns an execution event DAL for execution event operations.
- pub `cron_schedule` function L246-248 — `(&self) -> CronScheduleDAL<'_>` — Returns a cron schedule DAL for schedule operations.
- pub `cron_execution` function L251-253 — `(&self) -> CronExecutionDAL<'_>` — Returns a cron execution DAL for cron execution operations.
- pub `trigger_schedule` function L256-258 — `(&self) -> TriggerScheduleDAL<'_>` — Returns a trigger schedule DAL for trigger schedule operations.
- pub `trigger_execution` function L261-263 — `(&self) -> TriggerExecutionDAL<'_>` — Returns a trigger execution DAL for trigger execution operations.
- pub `workflow_packages` function L266-268 — `(&self) -> WorkflowPackagesDAL<'_>` — Returns a workflow packages DAL for package operations.
- pub `workflow_registry` function L280-286 — `( &self, storage: S, ) -> crate::registry::workflow_registry::WorkflowRegistryIm...` — Creates a workflow registry implementation with the given storage backend.
- pub `try_workflow_registry` function L299-310 — `( &self, storage: S, ) -> Result< crate::registry::workflow_registry::WorkflowRe...` — Creates a workflow registry implementation with the given storage backend.
-  `backend_dispatch` macro L105-125 — `-` — Helper macro for dispatching operations based on backend type.
-  `connection_match` macro L144-164 — `-` — Helper macro for matching on AnyConnection variants.
-  `DAL` type L181-311 — `= DAL` — ```

#### crates/cloacina/src/dal/unified/models.rs

- pub `UnifiedDbContext` struct L41-46 — `{ id: UniversalUuid, value: String, created_at: UniversalTimestamp, updated_at: ...` — Unified context model that works with both PostgreSQL and SQLite.
- pub `NewUnifiedDbContext` struct L51-56 — `{ id: UniversalUuid, value: String, created_at: UniversalTimestamp, updated_at: ...` — Insertable context with explicit ID and timestamps (for SQLite compatibility).
- pub `UnifiedPipelineExecution` struct L64-79 — `{ id: UniversalUuid, pipeline_name: String, pipeline_version: String, status: St...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedPipelineExecution` struct L83-92 — `{ id: UniversalUuid, pipeline_name: String, pipeline_version: String, status: St...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedTaskExecution` struct L100-119 — `{ id: UniversalUuid, pipeline_execution_id: UniversalUuid, task_name: String, st...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedTaskExecution` struct L123-134 — `{ id: UniversalUuid, pipeline_execution_id: UniversalUuid, task_name: String, st...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedTaskExecutionMetadata` struct L142-150 — `{ id: UniversalUuid, task_execution_id: UniversalUuid, pipeline_execution_id: Un...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedTaskExecutionMetadata` struct L154-162 — `{ id: UniversalUuid, task_execution_id: UniversalUuid, pipeline_execution_id: Un...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedRecoveryEvent` struct L170-179 — `{ id: UniversalUuid, pipeline_execution_id: UniversalUuid, task_execution_id: Op...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedRecoveryEvent` struct L183-192 — `{ id: UniversalUuid, pipeline_execution_id: UniversalUuid, task_execution_id: Op...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedExecutionEvent` struct L202-211 — `{ id: UniversalUuid, pipeline_execution_id: UniversalUuid, task_execution_id: Op...` — Unified execution event model for audit trail of state transitions.
- pub `NewUnifiedExecutionEvent` struct L215-223 — `{ id: UniversalUuid, pipeline_execution_id: UniversalUuid, task_execution_id: Op...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedTaskOutbox` struct L233-237 — `{ id: i64, task_execution_id: UniversalUuid, created_at: UniversalTimestamp }` — Unified task outbox model for work distribution.
- pub `NewUnifiedTaskOutbox` struct L241-244 — `{ task_execution_id: UniversalUuid, created_at: UniversalTimestamp }` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedCronSchedule` struct L252-265 — `{ id: UniversalUuid, workflow_name: String, cron_expression: String, timezone: S...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedCronSchedule` struct L269-281 — `{ id: UniversalUuid, workflow_name: String, cron_expression: String, timezone: S...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedCronExecution` struct L289-297 — `{ id: UniversalUuid, schedule_id: UniversalUuid, pipeline_execution_id: Option<U...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedCronExecution` struct L301-309 — `{ id: UniversalUuid, schedule_id: UniversalUuid, pipeline_execution_id: Option<U...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedTriggerSchedule` struct L317-327 — `{ id: UniversalUuid, trigger_name: String, workflow_name: String, poll_interval_...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedTriggerSchedule` struct L331-340 — `{ id: UniversalUuid, trigger_name: String, workflow_name: String, poll_interval_...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedTriggerExecution` struct L348-357 — `{ id: UniversalUuid, trigger_name: String, context_hash: String, pipeline_execut...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedTriggerExecution` struct L361-369 — `{ id: UniversalUuid, trigger_name: String, context_hash: String, pipeline_execut...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedWorkflowRegistryEntry` struct L377-381 — `{ id: UniversalUuid, created_at: UniversalTimestamp, data: UniversalBinary }` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedWorkflowRegistryEntry` struct L385-389 — `{ id: UniversalUuid, created_at: UniversalTimestamp, data: UniversalBinary }` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedWorkflowPackage` struct L397-408 — `{ id: UniversalUuid, registry_id: UniversalUuid, package_name: String, version: ...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedWorkflowPackage` struct L412-423 — `{ id: UniversalUuid, registry_id: UniversalUuid, package_name: String, version: ...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedSigningKey` struct L431-440 — `{ id: UniversalUuid, org_id: UniversalUuid, key_name: String, encrypted_private_...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedSigningKey` struct L444-452 — `{ id: UniversalUuid, org_id: UniversalUuid, key_name: String, encrypted_private_...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedTrustedKey` struct L460-468 — `{ id: UniversalUuid, org_id: UniversalUuid, key_fingerprint: String, public_key:...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedTrustedKey` struct L472-479 — `{ id: UniversalUuid, org_id: UniversalUuid, key_fingerprint: String, public_key:...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedKeyTrustAcl` struct L487-493 — `{ id: UniversalUuid, parent_org_id: UniversalUuid, child_org_id: UniversalUuid, ...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedKeyTrustAcl` struct L497-502 — `{ id: UniversalUuid, parent_org_id: UniversalUuid, child_org_id: UniversalUuid, ...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `UnifiedPackageSignature` struct L510-516 — `{ id: UniversalUuid, package_hash: String, key_fingerprint: String, signature: U...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewUnifiedPackageSignature` struct L520-526 — `{ id: UniversalUuid, package_hash: String, key_fingerprint: String, signature: U...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `AccumulatorStateRow` struct L803-810 — `{ edge_id: String, consumer_watermark: Option<String>, last_drain_at: UniversalT...` — Persisted accumulator state for crash recovery.
- pub `NewAccumulatorState` struct L815-819 — `{ edge_id: String, consumer_watermark: Option<String>, drain_metadata: String }` — New accumulator state for insertion.
- pub `DetectorStateRow` struct L828-832 — `{ source_name: String, committed_state: Option<String>, updated_at: UniversalTim...` — Persisted detector state row.
- pub `NewDetectorState` struct L837-840 — `{ source_name: String, committed_state: Option<String> }` — New detector state for insertion.
- pub `PendingBoundaryRow` struct L849-854 — `{ id: i64, source_name: String, boundary_json: String, received_at: UniversalTim...` — Pending boundary row from the WAL.
- pub `NewPendingBoundary` struct L859-862 — `{ source_name: String, boundary_json: String }` — New pending boundary for insertion.
- pub `EdgeDrainCursorRow` struct L871-875 — `{ edge_id: String, source_name: String, last_drain_id: i64 }` — Edge drain cursor row.
- pub `NewEdgeDrainCursor` struct L880-884 — `{ edge_id: String, source_name: String, last_drain_id: i64 }` — New edge drain cursor for insertion.
- pub `TenantRow` struct L892-899 — `{ id: UniversalUuid, name: String, schema_name: String, status: String, created_...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewTenant` struct L903-907 — `{ id: UniversalUuid, name: String, schema_name: String }` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `ApiKeyRow` struct L915-928 — `{ id: UniversalUuid, tenant_id: Option<UniversalUuid>, key_hash: String, key_pre...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewApiKey` struct L932-942 — `{ id: UniversalUuid, tenant_id: Option<UniversalUuid>, key_hash: String, key_pre...` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `WorkflowPatternRow` struct L950-954 — `{ id: UniversalUuid, api_key_id: UniversalUuid, pattern: String }` — SQL types that work with both PostgreSQL and SQLite backends.
- pub `NewWorkflowPattern` struct L958-962 — `{ id: UniversalUuid, api_key_id: UniversalUuid, pattern: String }` — SQL types that work with both PostgreSQL and SQLite backends.
-  `DbContext` type L551-560 — `= DbContext` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L552-559 — `(u: UnifiedDbContext) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `PipelineExecution` type L562-581 — `= PipelineExecution` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L563-580 — `(u: UnifiedPipelineExecution) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `TaskExecution` type L583-606 — `= TaskExecution` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L584-605 — `(u: UnifiedTaskExecution) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `TaskExecutionMetadata` type L608-620 — `= TaskExecutionMetadata` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L609-619 — `(u: UnifiedTaskExecutionMetadata) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `RecoveryEvent` type L622-635 — `= RecoveryEvent` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L623-634 — `(u: UnifiedRecoveryEvent) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `ExecutionEvent` type L637-650 — `= ExecutionEvent` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L638-649 — `(u: UnifiedExecutionEvent) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `CronSchedule` type L652-669 — `= CronSchedule` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L653-668 — `(u: UnifiedCronSchedule) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `CronExecution` type L671-683 — `= CronExecution` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L672-682 — `(u: UnifiedCronExecution) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `WorkflowRegistryEntry` type L685-693 — `= WorkflowRegistryEntry` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L686-692 — `(u: UnifiedWorkflowRegistryEntry) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `WorkflowPackage` type L695-710 — `= WorkflowPackage` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L696-709 — `(u: UnifiedWorkflowPackage) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `TriggerSchedule` type L712-726 — `= TriggerSchedule` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L713-725 — `(u: UnifiedTriggerSchedule) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `TriggerExecution` type L728-741 — `= TriggerExecution` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L729-740 — `(u: UnifiedTriggerExecution) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `SigningKey` type L743-756 — `= SigningKey` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L744-755 — `(u: UnifiedSigningKey) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `TrustedKey` type L758-770 — `= TrustedKey` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L759-769 — `(u: UnifiedTrustedKey) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `KeyTrustAcl` type L772-782 — `= KeyTrustAcl` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L773-781 — `(u: UnifiedKeyTrustAcl) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.
-  `PackageSignature` type L784-794 — `= PackageSignature` — SQL types that work with both PostgreSQL and SQLite backends.
-  `from` function L785-793 — `(u: UnifiedPackageSignature) -> Self` — SQL types that work with both PostgreSQL and SQLite backends.

#### crates/cloacina/src/dal/unified/pending_boundary_dal.rs

- pub `PendingBoundaryDAL` struct L28-30 — `{ dal: &'a DAL }` — Data access layer for pending boundary and edge drain cursor operations.
- pub `new` function L33-35 — `(dal: &'a DAL) -> Self` — DAL for pending boundary WAL and edge drain cursors (continuous scheduling).
- pub `append` function L38-48 — `(&self, source_name: String, boundary_json: String) -> Result<i64, String>` — Append a boundary to the WAL, returning the new row ID.
- pub `load_after_cursor` function L51-62 — `( &self, source_name: String, cursor_id: i64, ) -> Result<Vec<PendingBoundaryRow...` — Load all boundaries after a cursor position for a source.
- pub `advance_cursor` function L65-72 — `(&self, edge_id: String, drain_id: i64) -> Result<(), String>` — Advance the drain cursor for an edge.
- pub `load_cursor` function L75-81 — `(&self, edge_id: String) -> Result<i64, String>` — Load the last drain ID for an edge cursor.
- pub `init_cursor` function L84-95 — `(&self, edge_id: String, source_name: String) -> Result<(), String>` — Initialize a drain cursor for an edge (no-op if already exists).
- pub `min_cursor_for_source` function L98-105 — `(&self, source_name: String) -> Result<i64, String>` — Get the minimum cursor position across all edges for a source.
- pub `max_id_for_source` function L108-114 — `(&self, source_name: String) -> Result<Option<i64>, String>` — Get the maximum boundary ID for a source (most recent boundary).
- pub `cleanup` function L117-123 — `(&self, source_name: String, up_to_id: i64) -> Result<usize, String>` — Delete consumed boundaries up to (and including) a given ID for a source.
- pub `load_all_cursors` function L126-132 — `(&self) -> Result<Vec<EdgeDrainCursorRow>, String>` — Load all edge drain cursors.
-  `append_postgres` function L139-156 — `(&self, boundary: NewPendingBoundary) -> Result<i64, String>` — DAL for pending boundary WAL and edge drain cursors (continuous scheduling).
-  `load_after_cursor_postgres` function L159-181 — `( &self, source_name: String, cursor_id: i64, ) -> Result<Vec<PendingBoundaryRow...` — DAL for pending boundary WAL and edge drain cursors (continuous scheduling).
-  `advance_cursor_postgres` function L184-203 — `(&self, edge_id: String, drain_id: i64) -> Result<(), String>` — DAL for pending boundary WAL and edge drain cursors (continuous scheduling).
-  `load_cursor_postgres` function L206-225 — `(&self, edge_id: String) -> Result<i64, String>` — DAL for pending boundary WAL and edge drain cursors (continuous scheduling).
-  `init_cursor_postgres` function L228-247 — `(&self, cursor: NewEdgeDrainCursor) -> Result<(), String>` — DAL for pending boundary WAL and edge drain cursors (continuous scheduling).
-  `min_cursor_for_source_postgres` function L250-269 — `(&self, source_name: String) -> Result<i64, String>` — DAL for pending boundary WAL and edge drain cursors (continuous scheduling).
-  `max_id_for_source_postgres` function L272-290 — `(&self, source_name: String) -> Result<Option<i64>, String>` — DAL for pending boundary WAL and edge drain cursors (continuous scheduling).
-  `cleanup_postgres` function L293-312 — `(&self, source_name: String, up_to_id: i64) -> Result<usize, String>` — DAL for pending boundary WAL and edge drain cursors (continuous scheduling).
-  `load_all_cursors_postgres` function L315-330 — `(&self) -> Result<Vec<EdgeDrainCursorRow>, String>` — DAL for pending boundary WAL and edge drain cursors (continuous scheduling).
-  `append_sqlite` function L337-360 — `(&self, boundary: NewPendingBoundary) -> Result<i64, String>` — DAL for pending boundary WAL and edge drain cursors (continuous scheduling).
-  `load_after_cursor_sqlite` function L363-385 — `( &self, source_name: String, cursor_id: i64, ) -> Result<Vec<PendingBoundaryRow...` — DAL for pending boundary WAL and edge drain cursors (continuous scheduling).
-  `advance_cursor_sqlite` function L388-407 — `(&self, edge_id: String, drain_id: i64) -> Result<(), String>` — DAL for pending boundary WAL and edge drain cursors (continuous scheduling).
-  `load_cursor_sqlite` function L410-429 — `(&self, edge_id: String) -> Result<i64, String>` — DAL for pending boundary WAL and edge drain cursors (continuous scheduling).
-  `init_cursor_sqlite` function L432-449 — `(&self, cursor: NewEdgeDrainCursor) -> Result<(), String>` — DAL for pending boundary WAL and edge drain cursors (continuous scheduling).
-  `min_cursor_for_source_sqlite` function L452-471 — `(&self, source_name: String) -> Result<i64, String>` — DAL for pending boundary WAL and edge drain cursors (continuous scheduling).
-  `max_id_for_source_sqlite` function L474-492 — `(&self, source_name: String) -> Result<Option<i64>, String>` — DAL for pending boundary WAL and edge drain cursors (continuous scheduling).
-  `cleanup_sqlite` function L495-514 — `(&self, source_name: String, up_to_id: i64) -> Result<usize, String>` — DAL for pending boundary WAL and edge drain cursors (continuous scheduling).
-  `load_all_cursors_sqlite` function L517-532 — `(&self) -> Result<Vec<EdgeDrainCursorRow>, String>` — DAL for pending boundary WAL and edge drain cursors (continuous scheduling).

#### crates/cloacina/src/dal/unified/pipeline_execution.rs

- pub `PipelineExecutionDAL` struct L35-37 — `{ dal: &'a DAL }` — Data access layer for pipeline execution operations with compile-time backend selection.
- pub `new` function L40-42 — `(dal: &'a DAL) -> Self` — are written atomically.
- pub `create` function L48-57 — `( &self, new_execution: NewPipelineExecution, ) -> Result<PipelineExecution, Val...` — Creates a new pipeline execution record in the database.
- pub `get_by_id` function L185-191 — `(&self, id: UniversalUuid) -> Result<PipelineExecution, ValidationError>` — are written atomically.
- pub `get_active_executions` function L233-239 — `(&self) -> Result<Vec<PipelineExecution>, ValidationError>` — are written atomically.
- pub `update_status` function L287-297 — `( &self, id: UniversalUuid, status: &str, ) -> Result<(), ValidationError>` — are written atomically.
- pub `mark_completed` function L361-367 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — Marks a pipeline execution as completed.
- pub `get_last_version` function L463-472 — `( &self, pipeline_name: &str, ) -> Result<Option<String>, ValidationError>` — are written atomically.
- pub `mark_failed` function L534-544 — `( &self, id: UniversalUuid, reason: &str, ) -> Result<(), ValidationError>` — Marks a pipeline execution as failed with an error reason.
- pub `increment_recovery_attempts` function L654-663 — `( &self, id: UniversalUuid, ) -> Result<(), ValidationError>` — are written atomically.
- pub `cancel` function L723-729 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — are written atomically.
- pub `pause` function L738-748 — `( &self, id: UniversalUuid, reason: Option<&str>, ) -> Result<(), ValidationErro...` — Pauses a running pipeline execution.
- pub `resume` function L864-870 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — Resumes a paused pipeline execution.
- pub `update_final_context` function L1018-1029 — `( &self, id: UniversalUuid, final_context_id: UniversalUuid, ) -> Result<(), Val...` — are written atomically.
- pub `list_recent` function L1087-1093 — `(&self, limit: i64) -> Result<Vec<PipelineExecution>, ValidationError>` — are written atomically.
-  `create_postgres` function L60-120 — `( &self, new_execution: NewPipelineExecution, ) -> Result<PipelineExecution, Val...` — are written atomically.
-  `create_sqlite` function L123-183 — `( &self, new_execution: NewPipelineExecution, ) -> Result<PipelineExecution, Val...` — are written atomically.
-  `get_by_id_postgres` function L194-211 — `( &self, id: UniversalUuid, ) -> Result<PipelineExecution, ValidationError>` — are written atomically.
-  `get_by_id_sqlite` function L214-231 — `( &self, id: UniversalUuid, ) -> Result<PipelineExecution, ValidationError>` — are written atomically.
-  `get_active_executions_postgres` function L242-262 — `( &self, ) -> Result<Vec<PipelineExecution>, ValidationError>` — are written atomically.
-  `get_active_executions_sqlite` function L265-285 — `( &self, ) -> Result<Vec<PipelineExecution>, ValidationError>` — are written atomically.
-  `update_status_postgres` function L300-326 — `( &self, id: UniversalUuid, status: &str, ) -> Result<(), ValidationError>` — are written atomically.
-  `update_status_sqlite` function L329-355 — `( &self, id: UniversalUuid, status: &str, ) -> Result<(), ValidationError>` — are written atomically.
-  `mark_completed_postgres` function L370-414 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — are written atomically.
-  `mark_completed_sqlite` function L417-461 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — are written atomically.
-  `get_last_version_postgres` function L475-500 — `( &self, pipeline_name: &str, ) -> Result<Option<String>, ValidationError>` — are written atomically.
-  `get_last_version_sqlite` function L503-528 — `( &self, pipeline_name: &str, ) -> Result<Option<String>, ValidationError>` — are written atomically.
-  `mark_failed_postgres` function L547-598 — `( &self, id: UniversalUuid, reason: &str, ) -> Result<(), ValidationError>` — are written atomically.
-  `mark_failed_sqlite` function L601-652 — `( &self, id: UniversalUuid, reason: &str, ) -> Result<(), ValidationError>` — are written atomically.
-  `increment_recovery_attempts_postgres` function L666-692 — `( &self, id: UniversalUuid, ) -> Result<(), ValidationError>` — are written atomically.
-  `increment_recovery_attempts_sqlite` function L695-721 — `( &self, id: UniversalUuid, ) -> Result<(), ValidationError>` — are written atomically.
-  `pause_postgres` function L751-802 — `( &self, id: UniversalUuid, reason: Option<&str>, ) -> Result<(), ValidationErro...` — are written atomically.
-  `pause_sqlite` function L805-856 — `( &self, id: UniversalUuid, reason: Option<&str>, ) -> Result<(), ValidationErro...` — are written atomically.
-  `resume_postgres` function L873-918 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — are written atomically.
-  `resume_sqlite` function L921-966 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — are written atomically.
-  `cancel_postgres` function L969-991 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — are written atomically.
-  `cancel_sqlite` function L994-1016 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — are written atomically.
-  `update_final_context_postgres` function L1032-1057 — `( &self, id: UniversalUuid, final_context_id: UniversalUuid, ) -> Result<(), Val...` — are written atomically.
-  `update_final_context_sqlite` function L1060-1085 — `( &self, id: UniversalUuid, final_context_id: UniversalUuid, ) -> Result<(), Val...` — are written atomically.
-  `list_recent_postgres` function L1096-1118 — `( &self, limit: i64, ) -> Result<Vec<PipelineExecution>, ValidationError>` — are written atomically.
-  `list_recent_sqlite` function L1121-1143 — `( &self, limit: i64, ) -> Result<Vec<PipelineExecution>, ValidationError>` — are written atomically.

#### crates/cloacina/src/dal/unified/recovery_event.rs

- pub `RecoveryEventDAL` struct L36-38 — `{ dal: &'a DAL }` — Data access layer for recovery event operations with runtime backend selection.
- pub `new` function L42-44 — `(dal: &'a DAL) -> Self` — Creates a new RecoveryEventDAL instance.
- pub `create` function L47-56 — `( &self, new_event: NewRecoveryEvent, ) -> Result<RecoveryEvent, ValidationError...` — Creates a new recovery event record.
- pub `get_by_pipeline` function L143-152 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<Vec<RecoveryEvent>, V...` — Gets all recovery events for a specific pipeline execution.
- pub `get_by_task` function L205-214 — `( &self, task_execution_id: UniversalUuid, ) -> Result<Vec<RecoveryEvent>, Valid...` — Gets all recovery events for a specific task execution.
- pub `get_by_type` function L267-276 — `( &self, recovery_type: &str, ) -> Result<Vec<RecoveryEvent>, ValidationError>` — Gets recovery events by type for monitoring and analysis.
- pub `get_workflow_unavailable_events` function L331-336 — `( &self, ) -> Result<Vec<RecoveryEvent>, ValidationError>` — Gets all workflow unavailability events for monitoring unknown workflow cleanup.
- pub `get_recent` function L339-345 — `(&self, limit: i64) -> Result<Vec<RecoveryEvent>, ValidationError>` — Gets recent recovery events for monitoring purposes.
-  `create_postgres` function L59-98 — `( &self, new_event: NewRecoveryEvent, ) -> Result<RecoveryEvent, ValidationError...` — at runtime based on the database connection type.
-  `create_sqlite` function L101-140 — `( &self, new_event: NewRecoveryEvent, ) -> Result<RecoveryEvent, ValidationError...` — at runtime based on the database connection type.
-  `get_by_pipeline_postgres` function L155-177 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<Vec<RecoveryEvent>, V...` — at runtime based on the database connection type.
-  `get_by_pipeline_sqlite` function L180-202 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<Vec<RecoveryEvent>, V...` — at runtime based on the database connection type.
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
- pub `get_by_pipeline_and_task` function L139-151 — `( &self, pipeline_id: UniversalUuid, task_namespace: &TaskNamespace, ) -> Result...` — Retrieves task execution metadata for a specific pipeline and task.
- pub `get_by_task_execution` function L208-217 — `( &self, task_execution_id: UniversalUuid, ) -> Result<TaskExecutionMetadata, Va...` — Retrieves task execution metadata by task execution ID.
- pub `update_context_id` function L268-280 — `( &self, task_execution_id: UniversalUuid, context_id: Option<UniversalUuid>, ) ...` — Updates the context ID for a specific task execution.
- pub `upsert_task_execution_metadata` function L341-352 — `( &self, new_metadata: NewTaskExecutionMetadata, ) -> Result<TaskExecutionMetada...` — Creates or updates task execution metadata.
- pub `get_dependency_metadata` function L496-508 — `( &self, pipeline_id: UniversalUuid, dependency_task_names: &[String], ) -> Resu...` — Retrieves metadata for multiple dependency tasks within a pipeline.
- pub `get_dependency_metadata_with_contexts` function L565-587 — `( &self, pipeline_id: UniversalUuid, dependency_task_namespaces: &[TaskNamespace...` — Retrieves metadata and context data for multiple dependency tasks in a single query.
-  `create_postgres` function L57-95 — `( &self, new_metadata: NewTaskExecutionMetadata, ) -> Result<TaskExecutionMetada...` — at runtime based on the database connection type.
-  `create_sqlite` function L98-136 — `( &self, new_metadata: NewTaskExecutionMetadata, ) -> Result<TaskExecutionMetada...` — at runtime based on the database connection type.
-  `get_by_pipeline_and_task_postgres` function L154-178 — `( &self, pipeline_id: UniversalUuid, task_namespace: &TaskNamespace, ) -> Result...` — at runtime based on the database connection type.
-  `get_by_pipeline_and_task_sqlite` function L181-205 — `( &self, pipeline_id: UniversalUuid, task_namespace: &TaskNamespace, ) -> Result...` — at runtime based on the database connection type.
-  `get_by_task_execution_postgres` function L220-241 — `( &self, task_execution_id: UniversalUuid, ) -> Result<TaskExecutionMetadata, Va...` — at runtime based on the database connection type.
-  `get_by_task_execution_sqlite` function L244-265 — `( &self, task_execution_id: UniversalUuid, ) -> Result<TaskExecutionMetadata, Va...` — at runtime based on the database connection type.
-  `update_context_id_postgres` function L283-309 — `( &self, task_execution_id: UniversalUuid, context_id: Option<UniversalUuid>, ) ...` — at runtime based on the database connection type.
-  `update_context_id_sqlite` function L312-338 — `( &self, task_execution_id: UniversalUuid, context_id: Option<UniversalUuid>, ) ...` — at runtime based on the database connection type.
-  `upsert_task_execution_metadata_postgres` function L355-403 — `( &self, new_metadata: NewTaskExecutionMetadata, ) -> Result<TaskExecutionMetada...` — at runtime based on the database connection type.
-  `upsert_task_execution_metadata_sqlite` function L406-493 — `( &self, new_metadata: NewTaskExecutionMetadata, ) -> Result<TaskExecutionMetada...` — at runtime based on the database connection type.
-  `get_dependency_metadata_postgres` function L511-535 — `( &self, pipeline_id: UniversalUuid, dependency_task_names: &[String], ) -> Resu...` — at runtime based on the database connection type.
-  `get_dependency_metadata_sqlite` function L538-562 — `( &self, pipeline_id: UniversalUuid, dependency_task_names: &[String], ) -> Resu...` — at runtime based on the database connection type.
-  `get_dependency_metadata_with_contexts_postgres` function L590-626 — `( &self, pipeline_id: UniversalUuid, dependency_task_namespaces: &[TaskNamespace...` — at runtime based on the database connection type.
-  `get_dependency_metadata_with_contexts_sqlite` function L629-665 — `( &self, pipeline_id: UniversalUuid, dependency_task_namespaces: &[TaskNamespace...` — at runtime based on the database connection type.

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

#### crates/cloacina/src/dal/unified/tenant_dal.rs

- pub `TenantDAL` struct L27-29 — `{ dal: &'a DAL }` — Data access layer for tenant operations.
- pub `new` function L32-34 — `(dal: &'a DAL) -> Self` — DAL for tenant operations (auth system).
- pub `create` function L37-43 — `(&self, new_tenant: NewTenant) -> Result<(), String>` — Create a new tenant.
- pub `list` function L46-52 — `(&self) -> Result<Vec<TenantRow>, String>` — List all active tenants.
- pub `get` function L55-61 — `(&self, id: UniversalUuid) -> Result<Option<TenantRow>, String>` — Get a tenant by ID.
- pub `get_by_name` function L64-71 — `(&self, name: &str) -> Result<Option<TenantRow>, String>` — Get a tenant by name.
- pub `deactivate` function L74-80 — `(&self, id: UniversalUuid) -> Result<(), String>` — Deactivate a tenant (set status to 'deactivated').
-  `create_postgres` function L85-102 — `(&self, new_tenant: NewTenant) -> Result<(), String>` — DAL for tenant operations (auth system).
-  `list_postgres` function L105-121 — `(&self) -> Result<Vec<TenantRow>, String>` — DAL for tenant operations (auth system).
-  `get_postgres` function L124-141 — `(&self, id: UniversalUuid) -> Result<Option<TenantRow>, String>` — DAL for tenant operations (auth system).
-  `get_by_name_postgres` function L144-161 — `(&self, name: String) -> Result<Option<TenantRow>, String>` — DAL for tenant operations (auth system).
-  `deactivate_postgres` function L164-181 — `(&self, id: UniversalUuid) -> Result<(), String>` — DAL for tenant operations (auth system).
-  `create_sqlite` function L186-203 — `(&self, new_tenant: NewTenant) -> Result<(), String>` — DAL for tenant operations (auth system).
-  `list_sqlite` function L206-222 — `(&self) -> Result<Vec<TenantRow>, String>` — DAL for tenant operations (auth system).
-  `get_sqlite` function L225-242 — `(&self, id: UniversalUuid) -> Result<Option<TenantRow>, String>` — DAL for tenant operations (auth system).
-  `get_by_name_sqlite` function L245-262 — `(&self, name: String) -> Result<Option<TenantRow>, String>` — DAL for tenant operations (auth system).
-  `deactivate_sqlite` function L265-282 — `(&self, id: UniversalUuid) -> Result<(), String>` — DAL for tenant operations (auth system).

#### crates/cloacina/src/dal/unified/workflow_packages.rs

- pub `WorkflowPackagesDAL` struct L35-37 — `{ dal: &'a DAL }` — Data access layer for workflow package operations with runtime backend selection.
- pub `new` function L41-43 — `(dal: &'a DAL) -> Self` — Creates a new WorkflowPackagesDAL instance.
- pub `store_package_metadata` function L46-59 — `( &self, registry_id: &str, package_metadata: &PackageMetadata, storage_type: cr...` — Store package metadata in the database.
- pub `get_package_metadata` function L178-190 — `( &self, package_name: &str, version: &str, ) -> Result<Option<(String, PackageM...` — Retrieve package metadata from the database.
- pub `get_package_metadata_by_id` function L267-276 — `( &self, package_id: Uuid, ) -> Result<Option<(String, PackageMetadata)>, Regist...` — Retrieve package metadata by UUID from the database.
- pub `list_all_packages` function L345-351 — `(&self) -> Result<Vec<WorkflowPackage>, RegistryError>` — List all packages in the registry.
- pub `delete_package_metadata` function L390-402 — `( &self, package_name: &str, version: &str, ) -> Result<(), RegistryError>` — Delete package metadata from the database.
- pub `delete_package_metadata_by_id` function L467-477 — `( &self, package_id: Uuid, ) -> Result<(), RegistryError>` — Delete package metadata by UUID from the database.
-  `store_package_metadata_postgres` function L62-117 — `( &self, registry_id: &str, package_metadata: &PackageMetadata, storage_type: cr...` — at runtime based on the database connection type.
-  `store_package_metadata_sqlite` function L120-175 — `( &self, registry_id: &str, package_metadata: &PackageMetadata, storage_type: cr...` — at runtime based on the database connection type.
-  `get_package_metadata_postgres` function L193-227 — `( &self, package_name: &str, version: &str, ) -> Result<Option<(String, PackageM...` — at runtime based on the database connection type.
-  `get_package_metadata_sqlite` function L230-264 — `( &self, package_name: &str, version: &str, ) -> Result<Option<(String, PackageM...` — at runtime based on the database connection type.
-  `get_package_metadata_by_id_postgres` function L279-309 — `( &self, package_id: Uuid, ) -> Result<Option<(String, PackageMetadata)>, Regist...` — at runtime based on the database connection type.
-  `get_package_metadata_by_id_sqlite` function L312-342 — `( &self, package_id: Uuid, ) -> Result<Option<(String, PackageMetadata)>, Regist...` — at runtime based on the database connection type.
-  `list_all_packages_postgres` function L354-369 — `(&self) -> Result<Vec<WorkflowPackage>, RegistryError>` — at runtime based on the database connection type.
-  `list_all_packages_sqlite` function L372-387 — `(&self) -> Result<Vec<WorkflowPackage>, RegistryError>` — at runtime based on the database connection type.
-  `delete_package_metadata_postgres` function L405-433 — `( &self, package_name: &str, version: &str, ) -> Result<(), RegistryError>` — at runtime based on the database connection type.
-  `delete_package_metadata_sqlite` function L436-464 — `( &self, package_name: &str, version: &str, ) -> Result<(), RegistryError>` — at runtime based on the database connection type.
-  `delete_package_metadata_by_id_postgres` function L480-502 — `( &self, package_id: Uuid, ) -> Result<(), RegistryError>` — at runtime based on the database connection type.
-  `delete_package_metadata_by_id_sqlite` function L505-527 — `( &self, package_id: Uuid, ) -> Result<(), RegistryError>` — at runtime based on the database connection type.

#### crates/cloacina/src/dal/unified/workflow_registry.rs

- pub `WorkflowRegistryDAL` struct L23-25 — `{ dal: &'a DAL }` — Data access layer for workflow registry operations.
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

### crates/cloacina/src/dal/unified/cron_execution

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/dal/unified/cron_execution/crud.rs

-  `create_postgres` function L31-69 — `( &self, new_execution: NewCronExecution, ) -> Result<CronExecution, ValidationE...` — CRUD operations for cron executions.
-  `create_sqlite` function L72-110 — `( &self, new_execution: NewCronExecution, ) -> Result<CronExecution, ValidationE...` — CRUD operations for cron executions.
-  `update_pipeline_execution_id_postgres` function L113-139 — `( &self, cron_execution_id: UniversalUuid, pipeline_execution_id: UniversalUuid,...` — CRUD operations for cron executions.
-  `update_pipeline_execution_id_sqlite` function L142-168 — `( &self, cron_execution_id: UniversalUuid, pipeline_execution_id: UniversalUuid,...` — CRUD operations for cron executions.
-  `delete_older_than_postgres` function L171-193 — `( &self, older_than: DateTime<Utc>, ) -> Result<usize, ValidationError>` — CRUD operations for cron executions.
-  `delete_older_than_sqlite` function L196-218 — `( &self, older_than: DateTime<Utc>, ) -> Result<usize, ValidationError>` — CRUD operations for cron executions.

#### crates/cloacina/src/dal/unified/cron_execution/mod.rs

- pub `CronExecutionStats` struct L35-44 — `{ total_executions: i64, successful_executions: i64, lost_executions: i64, succe...` — Statistics about cron execution performance
- pub `CronExecutionDAL` struct L48-50 — `{ dal: &'a DAL }` — Data access layer for cron execution operations with runtime backend selection.
- pub `new` function L54-56 — `(dal: &'a DAL) -> Self` — Creates a new CronExecutionDAL instance.
- pub `create` function L59-68 — `( &self, new_execution: NewCronExecution, ) -> Result<CronExecution, ValidationE...` — Creates a new cron execution audit record in the database.
- pub `update_pipeline_execution_id` function L71-83 — `( &self, cron_execution_id: UniversalUuid, pipeline_execution_id: UniversalUuid,...` — Updates the pipeline execution ID for an existing cron execution record.
- pub `find_lost_executions` function L86-95 — `( &self, older_than_minutes: i32, ) -> Result<Vec<CronExecution>, ValidationErro...` — Finds "lost" executions that need recovery.
- pub `get_by_id` function L98-104 — `(&self, id: UniversalUuid) -> Result<CronExecution, ValidationError>` — Retrieves a cron execution record by its ID.
- pub `get_by_schedule_id` function L107-120 — `( &self, schedule_id: UniversalUuid, limit: i64, offset: i64, ) -> Result<Vec<Cr...` — Retrieves all cron execution records for a specific schedule.
- pub `get_by_pipeline_execution_id` function L123-134 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<Option<CronExecution>...` — Retrieves the cron execution record for a specific pipeline execution.
- pub `get_by_time_range` function L137-151 — `( &self, start_time: DateTime<Utc>, end_time: DateTime<Utc>, limit: i64, offset:...` — Retrieves cron execution records within a time range.
- pub `count_by_schedule` function L154-163 — `( &self, schedule_id: UniversalUuid, ) -> Result<i64, ValidationError>` — Counts the total number of executions for a specific schedule.
- pub `execution_exists` function L166-178 — `( &self, schedule_id: UniversalUuid, scheduled_time: DateTime<Utc>, ) -> Result<...` — Checks if an execution already exists for a specific schedule and time.
- pub `get_latest_by_schedule` function L181-190 — `( &self, schedule_id: UniversalUuid, ) -> Result<Option<CronExecution>, Validati...` — Retrieves the most recent execution for a specific schedule.
- pub `delete_older_than` function L193-202 — `( &self, older_than: DateTime<Utc>, ) -> Result<usize, ValidationError>` — Deletes old execution records beyond a certain age.
- pub `get_execution_stats` function L205-214 — `( &self, since: DateTime<Utc>, ) -> Result<CronExecutionStats, ValidationError>` — Gets execution statistics for monitoring and alerting.
-  `crud` module L23 — `-` — Unified Cron Execution DAL with runtime backend selection
-  `queries` module L24 — `-` — at runtime based on the database connection type.
-  `tracking` module L25 — `-` — at runtime based on the database connection type.

#### crates/cloacina/src/dal/unified/cron_execution/queries.rs

-  `get_by_id_postgres` function L31-48 — `( &self, id: UniversalUuid, ) -> Result<CronExecution, ValidationError>` — Query operations for cron executions.
-  `get_by_id_sqlite` function L51-68 — `( &self, id: UniversalUuid, ) -> Result<CronExecution, ValidationError>` — Query operations for cron executions.
-  `get_by_schedule_id_postgres` function L71-97 — `( &self, schedule_id: UniversalUuid, limit: i64, offset: i64, ) -> Result<Vec<Cr...` — Query operations for cron executions.
-  `get_by_schedule_id_sqlite` function L100-126 — `( &self, schedule_id: UniversalUuid, limit: i64, offset: i64, ) -> Result<Vec<Cr...` — Query operations for cron executions.
-  `get_by_pipeline_execution_id_postgres` function L129-151 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<Option<CronExecution>...` — Query operations for cron executions.
-  `get_by_pipeline_execution_id_sqlite` function L154-176 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<Option<CronExecution>...` — Query operations for cron executions.
-  `get_by_time_range_postgres` function L179-210 — `( &self, start_time: DateTime<Utc>, end_time: DateTime<Utc>, limit: i64, offset:...` — Query operations for cron executions.
-  `get_by_time_range_sqlite` function L213-244 — `( &self, start_time: DateTime<Utc>, end_time: DateTime<Utc>, limit: i64, offset:...` — Query operations for cron executions.
-  `get_latest_by_schedule_postgres` function L247-270 — `( &self, schedule_id: UniversalUuid, ) -> Result<Option<CronExecution>, Validati...` — Query operations for cron executions.
-  `get_latest_by_schedule_sqlite` function L273-296 — `( &self, schedule_id: UniversalUuid, ) -> Result<Option<CronExecution>, Validati...` — Query operations for cron executions.

#### crates/cloacina/src/dal/unified/cron_execution/tracking.rs

-  `find_lost_executions_postgres` function L31-62 — `( &self, older_than_minutes: i32, ) -> Result<Vec<CronExecution>, ValidationErro...` — Tracking and statistics operations for cron executions.
-  `find_lost_executions_sqlite` function L65-96 — `( &self, older_than_minutes: i32, ) -> Result<Vec<CronExecution>, ValidationErro...` — Tracking and statistics operations for cron executions.
-  `count_by_schedule_postgres` function L99-121 — `( &self, schedule_id: UniversalUuid, ) -> Result<i64, ValidationError>` — Tracking and statistics operations for cron executions.
-  `count_by_schedule_sqlite` function L124-146 — `( &self, schedule_id: UniversalUuid, ) -> Result<i64, ValidationError>` — Tracking and statistics operations for cron executions.
-  `execution_exists_postgres` function L149-175 — `( &self, schedule_id: UniversalUuid, scheduled_time: DateTime<Utc>, ) -> Result<...` — Tracking and statistics operations for cron executions.
-  `execution_exists_sqlite` function L178-204 — `( &self, schedule_id: UniversalUuid, scheduled_time: DateTime<Utc>, ) -> Result<...` — Tracking and statistics operations for cron executions.
-  `get_execution_stats_postgres` function L207-264 — `( &self, since: DateTime<Utc>, ) -> Result<CronExecutionStats, ValidationError>` — Tracking and statistics operations for cron executions.
-  `get_execution_stats_sqlite` function L267-330 — `( &self, since: DateTime<Utc>, ) -> Result<CronExecutionStats, ValidationError>` — Tracking and statistics operations for cron executions.

### crates/cloacina/src/dal/unified/cron_schedule

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/dal/unified/cron_schedule/crud.rs

-  `create_postgres` function L30-74 — `( &self, new_schedule: NewCronSchedule, ) -> Result<CronSchedule, ValidationErro...` — CRUD operations for cron schedules.
-  `create_sqlite` function L77-121 — `( &self, new_schedule: NewCronSchedule, ) -> Result<CronSchedule, ValidationErro...` — CRUD operations for cron schedules.
-  `get_by_id_postgres` function L124-141 — `( &self, id: UniversalUuid, ) -> Result<CronSchedule, ValidationError>` — CRUD operations for cron schedules.
-  `get_by_id_sqlite` function L144-161 — `( &self, id: UniversalUuid, ) -> Result<CronSchedule, ValidationError>` — CRUD operations for cron schedules.
-  `delete_postgres` function L164-177 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — CRUD operations for cron schedules.
-  `delete_sqlite` function L180-193 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — CRUD operations for cron schedules.

#### crates/cloacina/src/dal/unified/cron_schedule/mod.rs

- pub `CronScheduleDAL` struct L35-37 — `{ dal: &'a DAL }` — Data access layer for cron schedule operations with runtime backend selection.
- pub `new` function L41-43 — `(dal: &'a DAL) -> Self` — Creates a new CronScheduleDAL instance.
- pub `create` function L46-55 — `( &self, new_schedule: NewCronSchedule, ) -> Result<CronSchedule, ValidationErro...` — Creates a new cron schedule record in the database.
- pub `get_by_id` function L58-64 — `(&self, id: UniversalUuid) -> Result<CronSchedule, ValidationError>` — Retrieves a cron schedule by its ID.
- pub `get_due_schedules` function L67-76 — `( &self, now: DateTime<Utc>, ) -> Result<Vec<CronSchedule>, ValidationError>` — Retrieves all enabled cron schedules that are due for execution.
- pub `update_schedule_times` function L79-92 — `( &self, id: UniversalUuid, last_run: DateTime<Utc>, next_run: DateTime<Utc>, ) ...` — Updates the last run and next run times for a cron schedule.
- pub `enable` function L95-101 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — Enables a cron schedule.
- pub `disable` function L104-110 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — Disables a cron schedule.
- pub `delete` function L113-119 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — Deletes a cron schedule from the database.
- pub `list` function L122-133 — `( &self, enabled_only: bool, limit: i64, offset: i64, ) -> Result<Vec<CronSchedu...` — Lists all cron schedules with optional filtering.
- pub `find_by_workflow` function L136-145 — `( &self, workflow_name: &str, ) -> Result<Vec<CronSchedule>, ValidationError>` — Finds cron schedules by workflow name.
- pub `update_next_run` function L148-158 — `( &self, id: UniversalUuid, next_run: DateTime<Utc>, ) -> Result<(), ValidationE...` — Updates the next run time for a cron schedule.
- pub `claim_and_update` function L161-175 — `( &self, id: UniversalUuid, current_time: DateTime<Utc>, last_run: DateTime<Utc>...` — Atomically claims and updates a cron schedule's timing.
- pub `count` function L178-184 — `(&self, enabled_only: bool) -> Result<i64, ValidationError>` — Counts the total number of cron schedules.
- pub `update_expression_and_timezone` function L187-201 — `( &self, id: UniversalUuid, cron_expression: Option<&str>, timezone: Option<&str...` — Updates the cron expression, timezone, and next run time for a schedule.
-  `crud` module L23 — `-` — Unified Cron Schedule DAL with runtime backend selection
-  `queries` module L24 — `-` — at runtime based on the database connection type.
-  `state` module L25 — `-` — at runtime based on the database connection type.

#### crates/cloacina/src/dal/unified/cron_schedule/queries.rs

-  `get_due_schedules_postgres` function L31-70 — `( &self, now: DateTime<Utc>, ) -> Result<Vec<CronSchedule>, ValidationError>` — Query operations for cron schedules.
-  `get_due_schedules_sqlite` function L73-108 — `( &self, now: DateTime<Utc>, ) -> Result<Vec<CronSchedule>, ValidationError>` — Query operations for cron schedules.
-  `list_postgres` function L111-143 — `( &self, enabled_only: bool, limit: i64, offset: i64, ) -> Result<Vec<CronSchedu...` — Query operations for cron schedules.
-  `list_sqlite` function L146-178 — `( &self, enabled_only: bool, limit: i64, offset: i64, ) -> Result<Vec<CronSchedu...` — Query operations for cron schedules.
-  `find_by_workflow_postgres` function L181-204 — `( &self, workflow_name: &str, ) -> Result<Vec<CronSchedule>, ValidationError>` — Query operations for cron schedules.
-  `find_by_workflow_sqlite` function L207-230 — `( &self, workflow_name: &str, ) -> Result<Vec<CronSchedule>, ValidationError>` — Query operations for cron schedules.
-  `count_postgres` function L233-256 — `(&self, enabled_only: bool) -> Result<i64, ValidationError>` — Query operations for cron schedules.
-  `count_sqlite` function L259-282 — `(&self, enabled_only: bool) -> Result<i64, ValidationError>` — Query operations for cron schedules.

#### crates/cloacina/src/dal/unified/cron_schedule/state.rs

-  `update_schedule_times_postgres` function L29-59 — `( &self, id: UniversalUuid, last_run: DateTime<Utc>, next_run: DateTime<Utc>, ) ...` — State transition operations for cron schedules.
-  `update_schedule_times_sqlite` function L62-92 — `( &self, id: UniversalUuid, last_run: DateTime<Utc>, next_run: DateTime<Utc>, ) ...` — State transition operations for cron schedules.
-  `enable_postgres` function L95-118 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — State transition operations for cron schedules.
-  `enable_sqlite` function L121-144 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — State transition operations for cron schedules.
-  `disable_postgres` function L147-170 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — State transition operations for cron schedules.
-  `disable_sqlite` function L173-196 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — State transition operations for cron schedules.
-  `update_next_run_postgres` function L199-226 — `( &self, id: UniversalUuid, next_run: DateTime<Utc>, ) -> Result<(), ValidationE...` — State transition operations for cron schedules.
-  `update_next_run_sqlite` function L229-256 — `( &self, id: UniversalUuid, next_run: DateTime<Utc>, ) -> Result<(), ValidationE...` — State transition operations for cron schedules.
-  `claim_and_update_postgres` function L259-299 — `( &self, id: UniversalUuid, current_time: DateTime<Utc>, last_run: DateTime<Utc>...` — State transition operations for cron schedules.
-  `claim_and_update_sqlite` function L302-338 — `( &self, id: UniversalUuid, current_time: DateTime<Utc>, last_run: DateTime<Utc>...` — State transition operations for cron schedules.
-  `update_expression_and_timezone_postgres` function L341-401 — `( &self, id: UniversalUuid, cron_expression: Option<&str>, timezone: Option<&str...` — State transition operations for cron schedules.
-  `update_expression_and_timezone_sqlite` function L404-464 — `( &self, id: UniversalUuid, cron_expression: Option<&str>, timezone: Option<&str...` — State transition operations for cron schedules.

### crates/cloacina/src/dal/unified/task_execution

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/dal/unified/task_execution/claiming.rs

- pub `schedule_retry` function L37-50 — `( &self, task_id: UniversalUuid, retry_at: UniversalTimestamp, new_attempt: i32,...` — Updates a task's retry schedule with a new attempt count and retry time.
- pub `claim_ready_task` function L206-215 — `( &self, limit: usize, ) -> Result<Vec<ClaimResult>, ValidationError>` — Atomically claims up to `limit` ready tasks for execution.
- pub `get_ready_for_retry` function L417-423 — `(&self) -> Result<Vec<TaskExecution>, ValidationError>` — Retrieves tasks that are ready for retry (retry_at time has passed).
-  `schedule_retry_postgres` function L53-125 — `( &self, task_id: UniversalUuid, retry_at: UniversalTimestamp, new_attempt: i32,...` — are written atomically.
-  `schedule_retry_sqlite` function L128-200 — `( &self, task_id: UniversalUuid, retry_at: UniversalTimestamp, new_attempt: i32,...` — are written atomically.
-  `claim_ready_task_postgres` function L218-311 — `( &self, limit: usize, ) -> Result<Vec<ClaimResult>, ValidationError>` — are written atomically.
-  `PgClaimResult` struct L235-244 — `{ id: Uuid, pipeline_execution_id: Uuid, task_name: String, attempt: i32 }` — are written atomically.
-  `claim_ready_task_sqlite` function L314-414 — `( &self, limit: usize, ) -> Result<Vec<ClaimResult>, ValidationError>` — are written atomically.
-  `get_ready_for_retry_postgres` function L426-450 — `(&self) -> Result<Vec<TaskExecution>, ValidationError>` — are written atomically.
-  `get_ready_for_retry_sqlite` function L453-477 — `(&self) -> Result<Vec<TaskExecution>, ValidationError>` — are written atomically.

#### crates/cloacina/src/dal/unified/task_execution/crud.rs

- pub `create` function L38-47 — `( &self, new_task: NewTaskExecution, ) -> Result<TaskExecution, ValidationError>` — Creates a new task execution record in the database.
- pub `get_by_id` function L172-181 — `( &self, task_id: UniversalUuid, ) -> Result<TaskExecution, ValidationError>` — Retrieves a specific task execution by its ID.
- pub `get_all_tasks_for_pipeline` function L224-235 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<Vec<TaskExecution>, V...` — Retrieves all tasks associated with a pipeline execution.
-  `create_postgres` function L50-108 — `( &self, new_task: NewTaskExecution, ) -> Result<TaskExecution, ValidationError>` — are written atomically.
-  `create_sqlite` function L111-169 — `( &self, new_task: NewTaskExecution, ) -> Result<TaskExecution, ValidationError>` — are written atomically.
-  `get_by_id_postgres` function L184-201 — `( &self, task_id: UniversalUuid, ) -> Result<TaskExecution, ValidationError>` — are written atomically.
-  `get_by_id_sqlite` function L204-221 — `( &self, task_id: UniversalUuid, ) -> Result<TaskExecution, ValidationError>` — are written atomically.
-  `get_all_tasks_for_pipeline_postgres` function L238-259 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<Vec<TaskExecution>, V...` — are written atomically.
-  `get_all_tasks_for_pipeline_sqlite` function L262-283 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<Vec<TaskExecution>, V...` — are written atomically.

#### crates/cloacina/src/dal/unified/task_execution/mod.rs

- pub `RetryStats` struct L40-49 — `{ tasks_with_retries: i32, total_retries: i32, max_attempts_used: i32, tasks_exh...` — Statistics about retry behavior for a pipeline execution.
- pub `ClaimResult` struct L53-62 — `{ id: UniversalUuid, pipeline_execution_id: UniversalUuid, task_name: String, at...` — Result structure for atomic task claiming operations.
- pub `TaskExecutionDAL` struct L66-68 — `{ dal: &'a DAL }` — Data access layer for task execution operations with runtime backend selection.
- pub `new` function L72-74 — `(dal: &'a DAL) -> Self` — Creates a new TaskExecutionDAL instance.
-  `claiming` module L29 — `-` — Task Execution Data Access Layer for Unified Backend Support
-  `crud` module L30 — `-` — - Pipeline completion and failure detection
-  `queries` module L31 — `-` — - Pipeline completion and failure detection
-  `recovery` module L32 — `-` — - Pipeline completion and failure detection
-  `state` module L33 — `-` — - Pipeline completion and failure detection

#### crates/cloacina/src/dal/unified/task_execution/queries.rs

- pub `get_pending_tasks` function L29-38 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<Vec<TaskExecution>, V...` — Retrieves all pending (NotStarted) tasks for a specific pipeline execution.
- pub `get_pending_tasks_batch` function L91-102 — `( &self, pipeline_execution_ids: Vec<UniversalUuid>, ) -> Result<Vec<TaskExecuti...` — Gets all pending tasks for multiple pipelines in a single query.
- pub `check_pipeline_completion` function L163-174 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<bool, ValidationError...` — Checks if all tasks in a pipeline have reached a terminal state.
- pub `get_task_status` function L229-241 — `( &self, pipeline_execution_id: UniversalUuid, task_name: &str, ) -> Result<Stri...` — Gets the current status of a specific task in a pipeline.
- pub `get_task_statuses_batch` function L300-312 — `( &self, pipeline_execution_id: UniversalUuid, task_names: Vec<String>, ) -> Res...` — Gets the status of multiple tasks in a single database query.
-  `get_pending_tasks_postgres` function L41-63 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<Vec<TaskExecution>, V...` — Query operations for task executions.
-  `get_pending_tasks_sqlite` function L66-88 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<Vec<TaskExecution>, V...` — Query operations for task executions.
-  `get_pending_tasks_batch_postgres` function L105-131 — `( &self, pipeline_execution_ids: Vec<UniversalUuid>, ) -> Result<Vec<TaskExecuti...` — Query operations for task executions.
-  `get_pending_tasks_batch_sqlite` function L134-160 — `( &self, pipeline_execution_ids: Vec<UniversalUuid>, ) -> Result<Vec<TaskExecuti...` — Query operations for task executions.
-  `check_pipeline_completion_postgres` function L177-200 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<bool, ValidationError...` — Query operations for task executions.
-  `check_pipeline_completion_sqlite` function L203-226 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<bool, ValidationError...` — Query operations for task executions.
-  `get_task_status_postgres` function L244-269 — `( &self, pipeline_execution_id: UniversalUuid, task_name: &str, ) -> Result<Stri...` — Query operations for task executions.
-  `get_task_status_sqlite` function L272-297 — `( &self, pipeline_execution_id: UniversalUuid, task_name: &str, ) -> Result<Stri...` — Query operations for task executions.
-  `get_task_statuses_batch_postgres` function L315-345 — `( &self, pipeline_execution_id: UniversalUuid, task_names: Vec<String>, ) -> Res...` — Query operations for task executions.
-  `get_task_statuses_batch_sqlite` function L348-378 — `( &self, pipeline_execution_id: UniversalUuid, task_names: Vec<String>, ) -> Res...` — Query operations for task executions.

#### crates/cloacina/src/dal/unified/task_execution/recovery.rs

- pub `get_orphaned_tasks` function L29-35 — `(&self) -> Result<Vec<TaskExecution>, ValidationError>` — Retrieves tasks that are stuck in "Running" state (orphaned tasks).
- pub `reset_task_for_recovery` function L80-89 — `( &self, task_id: UniversalUuid, ) -> Result<(), ValidationError>` — Resets a task from "Running" to "Ready" state for recovery.
- pub `check_pipeline_failure` function L152-163 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<bool, ValidationError...` — Checks if a pipeline should be marked as failed due to abandoned tasks.
- pub `get_retry_stats` function L220-247 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<RetryStats, Validatio...` — Calculates retry statistics for a specific pipeline execution.
- pub `get_exhausted_retry_tasks` function L250-265 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<Vec<TaskExecution>, V...` — Retrieves tasks that have exceeded their retry limit.
-  `get_orphaned_tasks_postgres` function L38-56 — `(&self) -> Result<Vec<TaskExecution>, ValidationError>` — Recovery operations for orphaned and failed tasks.
-  `get_orphaned_tasks_sqlite` function L59-77 — `(&self) -> Result<Vec<TaskExecution>, ValidationError>` — Recovery operations for orphaned and failed tasks.
-  `reset_task_for_recovery_postgres` function L92-119 — `( &self, task_id: UniversalUuid, ) -> Result<(), ValidationError>` — Recovery operations for orphaned and failed tasks.
-  `reset_task_for_recovery_sqlite` function L122-149 — `( &self, task_id: UniversalUuid, ) -> Result<(), ValidationError>` — Recovery operations for orphaned and failed tasks.
-  `check_pipeline_failure_postgres` function L166-190 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<bool, ValidationError...` — Recovery operations for orphaned and failed tasks.
-  `check_pipeline_failure_sqlite` function L193-217 — `( &self, pipeline_execution_id: UniversalUuid, ) -> Result<bool, ValidationError...` — Recovery operations for orphaned and failed tasks.

#### crates/cloacina/src/dal/unified/task_execution/state.rs

- pub `mark_completed` function L37-43 — `(&self, task_id: UniversalUuid) -> Result<(), ValidationError>` — Marks a task execution as completed.
- pub `mark_failed` function L151-161 — `( &self, task_id: UniversalUuid, error_message: &str, ) -> Result<(), Validation...` — Marks a task execution as failed with an error message.
- pub `mark_ready` function L286-292 — `(&self, task_id: UniversalUuid) -> Result<(), ValidationError>` — Marks a task as ready for execution.
- pub `mark_skipped` function L418-428 — `( &self, task_id: UniversalUuid, reason: &str, ) -> Result<(), ValidationError>` — Marks a task as skipped with a provided reason.
- pub `mark_abandoned` function L552-562 — `( &self, task_id: UniversalUuid, reason: &str, ) -> Result<(), ValidationError>` — Marks a task as permanently abandoned after too many recovery attempts.
- pub `set_sub_status` function L685-695 — `( &self, task_id: UniversalUuid, sub_status: Option<&str>, ) -> Result<(), Valid...` — Updates the sub_status of a running task execution.
- pub `reset_retry_state` function L829-835 — `(&self, task_id: UniversalUuid) -> Result<(), ValidationError>` — Resets the retry state for a task to its initial state.
-  `mark_completed_postgres` function L46-94 — `(&self, task_id: UniversalUuid) -> Result<(), ValidationError>` — are written atomically.
-  `mark_completed_sqlite` function L97-145 — `(&self, task_id: UniversalUuid) -> Result<(), ValidationError>` — are written atomically.
-  `mark_failed_postgres` function L164-219 — `( &self, task_id: UniversalUuid, error_message: &str, ) -> Result<(), Validation...` — are written atomically.
-  `mark_failed_sqlite` function L222-277 — `( &self, task_id: UniversalUuid, error_message: &str, ) -> Result<(), Validation...` — are written atomically.
-  `mark_ready_postgres` function L295-352 — `(&self, task_id: UniversalUuid) -> Result<(), ValidationError>` — are written atomically.
-  `mark_ready_sqlite` function L355-412 — `(&self, task_id: UniversalUuid) -> Result<(), ValidationError>` — are written atomically.
-  `mark_skipped_postgres` function L431-487 — `( &self, task_id: UniversalUuid, reason: &str, ) -> Result<(), ValidationError>` — are written atomically.
-  `mark_skipped_sqlite` function L490-546 — `( &self, task_id: UniversalUuid, reason: &str, ) -> Result<(), ValidationError>` — are written atomically.
-  `mark_abandoned_postgres` function L565-620 — `( &self, task_id: UniversalUuid, reason: &str, ) -> Result<(), ValidationError>` — are written atomically.
-  `mark_abandoned_sqlite` function L623-678 — `( &self, task_id: UniversalUuid, reason: &str, ) -> Result<(), ValidationError>` — are written atomically.
-  `set_sub_status_postgres` function L698-759 — `( &self, task_id: UniversalUuid, sub_status: Option<&str>, ) -> Result<(), Valid...` — are written atomically.
-  `set_sub_status_sqlite` function L762-823 — `( &self, task_id: UniversalUuid, sub_status: Option<&str>, ) -> Result<(), Valid...` — are written atomically.
-  `reset_retry_state_postgres` function L838-893 — `( &self, task_id: UniversalUuid, ) -> Result<(), ValidationError>` — are written atomically.
-  `reset_retry_state_sqlite` function L896-951 — `( &self, task_id: UniversalUuid, ) -> Result<(), ValidationError>` — are written atomically.

### crates/cloacina/src/dal/unified/trigger_execution

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/dal/unified/trigger_execution/crud.rs

-  `create_postgres` function L31-69 — `( &self, new_execution: NewTriggerExecution, ) -> Result<TriggerExecution, Valid...` — CRUD operations for trigger executions.
-  `create_sqlite` function L72-110 — `( &self, new_execution: NewTriggerExecution, ) -> Result<TriggerExecution, Valid...` — CRUD operations for trigger executions.
-  `get_by_id_postgres` function L113-130 — `( &self, id: UniversalUuid, ) -> Result<TriggerExecution, ValidationError>` — CRUD operations for trigger executions.
-  `get_by_id_sqlite` function L133-150 — `( &self, id: UniversalUuid, ) -> Result<TriggerExecution, ValidationError>` — CRUD operations for trigger executions.
-  `has_active_execution_postgres` function L153-180 — `( &self, trigger_name: &str, context_hash: &str, ) -> Result<bool, ValidationErr...` — CRUD operations for trigger executions.
-  `has_active_execution_sqlite` function L183-210 — `( &self, trigger_name: &str, context_hash: &str, ) -> Result<bool, ValidationErr...` — CRUD operations for trigger executions.
-  `complete_postgres` function L213-235 — `( &self, id: UniversalUuid, completed_at: DateTime<Utc>, ) -> Result<(), Validat...` — CRUD operations for trigger executions.
-  `complete_sqlite` function L238-260 — `( &self, id: UniversalUuid, completed_at: DateTime<Utc>, ) -> Result<(), Validat...` — CRUD operations for trigger executions.
-  `link_pipeline_execution_postgres` function L263-284 — `( &self, id: UniversalUuid, pipeline_execution_id: UniversalUuid, ) -> Result<()...` — CRUD operations for trigger executions.
-  `link_pipeline_execution_sqlite` function L287-308 — `( &self, id: UniversalUuid, pipeline_execution_id: UniversalUuid, ) -> Result<()...` — CRUD operations for trigger executions.
-  `get_recent_postgres` function L311-336 — `( &self, trigger_name: &str, limit: i64, ) -> Result<Vec<TriggerExecution>, Vali...` — CRUD operations for trigger executions.
-  `get_recent_sqlite` function L339-364 — `( &self, trigger_name: &str, limit: i64, ) -> Result<Vec<TriggerExecution>, Vali...` — CRUD operations for trigger executions.
-  `list_by_trigger_postgres` function L367-394 — `( &self, trigger_name: &str, limit: i64, offset: i64, ) -> Result<Vec<TriggerExe...` — CRUD operations for trigger executions.
-  `list_by_trigger_sqlite` function L397-424 — `( &self, trigger_name: &str, limit: i64, offset: i64, ) -> Result<Vec<TriggerExe...` — CRUD operations for trigger executions.
-  `complete_by_pipeline_postgres` function L427-455 — `( &self, pipeline_execution_id: UniversalUuid, completed_at: DateTime<Utc>, ) ->...` — CRUD operations for trigger executions.
-  `complete_by_pipeline_sqlite` function L458-486 — `( &self, pipeline_execution_id: UniversalUuid, completed_at: DateTime<Utc>, ) ->...` — CRUD operations for trigger executions.

#### crates/cloacina/src/dal/unified/trigger_execution/mod.rs

- pub `TriggerExecutionDAL` struct L32-34 — `{ dal: &'a DAL }` — Data access layer for trigger execution operations with runtime backend selection.
- pub `new` function L38-40 — `(dal: &'a DAL) -> Self` — Creates a new TriggerExecutionDAL instance.
- pub `create` function L43-52 — `( &self, new_execution: NewTriggerExecution, ) -> Result<TriggerExecution, Valid...` — Creates a new trigger execution record in the database.
- pub `get_by_id` function L55-61 — `(&self, id: UniversalUuid) -> Result<TriggerExecution, ValidationError>` — Retrieves a trigger execution by its ID.
- pub `has_active_execution` function L65-77 — `( &self, trigger_name: &str, context_hash: &str, ) -> Result<bool, ValidationErr...` — Checks if there's an active (incomplete) execution for a trigger with the given context hash.
- pub `complete` function L80-90 — `( &self, id: UniversalUuid, completed_at: DateTime<Utc>, ) -> Result<(), Validat...` — Marks an execution as completed.
- pub `link_pipeline_execution` function L93-105 — `( &self, id: UniversalUuid, pipeline_execution_id: UniversalUuid, ) -> Result<()...` — Links a trigger execution to a pipeline execution.
- pub `get_recent` function L108-118 — `( &self, trigger_name: &str, limit: i64, ) -> Result<Vec<TriggerExecution>, Vali...` — Retrieves recent executions for a trigger.
- pub `list_by_trigger` function L121-134 — `( &self, trigger_name: &str, limit: i64, offset: i64, ) -> Result<Vec<TriggerExe...` — Lists executions for a trigger with pagination.
- pub `complete_by_pipeline` function L138-150 — `( &self, pipeline_execution_id: UniversalUuid, completed_at: DateTime<Utc>, ) ->...` — Marks all incomplete executions for a pipeline as completed.
-  `crud` module L22 — `-` — Unified Trigger Execution DAL with runtime backend selection

### crates/cloacina/src/dal/unified/trigger_schedule

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/dal/unified/trigger_schedule/crud.rs

-  `create_postgres` function L31-74 — `( &self, new_schedule: NewTriggerSchedule, ) -> Result<TriggerSchedule, Validati...` — CRUD operations for trigger schedules.
-  `create_sqlite` function L77-120 — `( &self, new_schedule: NewTriggerSchedule, ) -> Result<TriggerSchedule, Validati...` — CRUD operations for trigger schedules.
-  `get_by_id_postgres` function L123-140 — `( &self, id: UniversalUuid, ) -> Result<TriggerSchedule, ValidationError>` — CRUD operations for trigger schedules.
-  `get_by_id_sqlite` function L143-160 — `( &self, id: UniversalUuid, ) -> Result<TriggerSchedule, ValidationError>` — CRUD operations for trigger schedules.
-  `get_by_name_postgres` function L163-186 — `( &self, name: &str, ) -> Result<Option<TriggerSchedule>, ValidationError>` — CRUD operations for trigger schedules.
-  `get_by_name_sqlite` function L189-212 — `( &self, name: &str, ) -> Result<Option<TriggerSchedule>, ValidationError>` — CRUD operations for trigger schedules.
-  `get_enabled_postgres` function L215-235 — `( &self, ) -> Result<Vec<TriggerSchedule>, ValidationError>` — CRUD operations for trigger schedules.
-  `get_enabled_sqlite` function L238-256 — `(&self) -> Result<Vec<TriggerSchedule>, ValidationError>` — CRUD operations for trigger schedules.
-  `list_postgres` function L259-283 — `( &self, limit: i64, offset: i64, ) -> Result<Vec<TriggerSchedule>, ValidationEr...` — CRUD operations for trigger schedules.
-  `list_sqlite` function L286-310 — `( &self, limit: i64, offset: i64, ) -> Result<Vec<TriggerSchedule>, ValidationEr...` — CRUD operations for trigger schedules.
-  `update_last_poll_postgres` function L313-338 — `( &self, id: UniversalUuid, last_poll_at: DateTime<Utc>, ) -> Result<(), Validat...` — CRUD operations for trigger schedules.
-  `update_last_poll_sqlite` function L341-366 — `( &self, id: UniversalUuid, last_poll_at: DateTime<Utc>, ) -> Result<(), Validat...` — CRUD operations for trigger schedules.
-  `enable_postgres` function L369-390 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — CRUD operations for trigger schedules.
-  `enable_sqlite` function L393-414 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — CRUD operations for trigger schedules.
-  `disable_postgres` function L417-438 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — CRUD operations for trigger schedules.
-  `disable_sqlite` function L441-462 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — CRUD operations for trigger schedules.
-  `delete_postgres` function L465-478 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — CRUD operations for trigger schedules.
-  `delete_sqlite` function L481-494 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — CRUD operations for trigger schedules.
-  `upsert_postgres` function L497-544 — `( &self, new_schedule: NewTriggerSchedule, ) -> Result<TriggerSchedule, Validati...` — CRUD operations for trigger schedules.
-  `upsert_sqlite` function L547-591 — `( &self, new_schedule: NewTriggerSchedule, ) -> Result<TriggerSchedule, Validati...` — CRUD operations for trigger schedules.

#### crates/cloacina/src/dal/unified/trigger_schedule/mod.rs

- pub `TriggerScheduleDAL` struct L32-34 — `{ dal: &'a DAL }` — Data access layer for trigger schedule operations with runtime backend selection.
- pub `new` function L38-40 — `(dal: &'a DAL) -> Self` — Creates a new TriggerScheduleDAL instance.
- pub `create` function L43-52 — `( &self, new_schedule: NewTriggerSchedule, ) -> Result<TriggerSchedule, Validati...` — Creates a new trigger schedule record in the database.
- pub `get_by_id` function L55-61 — `(&self, id: UniversalUuid) -> Result<TriggerSchedule, ValidationError>` — Retrieves a trigger schedule by its ID.
- pub `get_by_name` function L64-73 — `( &self, name: &str, ) -> Result<Option<TriggerSchedule>, ValidationError>` — Retrieves a trigger schedule by its name.
- pub `get_enabled` function L76-82 — `(&self) -> Result<Vec<TriggerSchedule>, ValidationError>` — Retrieves all enabled trigger schedules.
- pub `list` function L85-95 — `( &self, limit: i64, offset: i64, ) -> Result<Vec<TriggerSchedule>, ValidationEr...` — Lists trigger schedules with pagination.
- pub `update_last_poll` function L98-108 — `( &self, id: UniversalUuid, last_poll_at: DateTime<Utc>, ) -> Result<(), Validat...` — Updates the last poll time for a trigger schedule.
- pub `enable` function L111-117 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — Enables a trigger schedule.
- pub `disable` function L120-126 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — Disables a trigger schedule.
- pub `delete` function L129-135 — `(&self, id: UniversalUuid) -> Result<(), ValidationError>` — Deletes a trigger schedule from the database.
- pub `upsert` function L138-147 — `( &self, new_schedule: NewTriggerSchedule, ) -> Result<TriggerSchedule, Validati...` — Creates or updates a trigger schedule by name.
-  `crud` module L22 — `-` — Unified Trigger Schedule DAL with runtime backend selection

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
-  `postgres_impl` module L26-434 — `-` — Note: This module is only available when using the PostgreSQL backend.
-  `AdminError` type L85-89 — `= AdminError` — Note: This module is only available when using the PostgreSQL backend.
-  `from` function L86-88 — `(err: deadpool::managed::PoolError<deadpool_diesel::postgres::Manager>) -> Self` — Note: This module is only available when using the PostgreSQL backend.
-  `AdminError` type L91-95 — `= AdminError` — Note: This module is only available when using the PostgreSQL backend.
-  `from` function L92-94 — `(err: deadpool::managed::PoolError<deadpool_diesel::Error>) -> Self` — Note: This module is only available when using the PostgreSQL backend.
-  `DatabaseAdmin` type L98-317 — `= DatabaseAdmin` — Note: This module is only available when using the PostgreSQL backend.
-  `build_connection_string` function L306-316 — `(&self, username: &str, password: &str) -> String` — Note: This module is only available when using the PostgreSQL backend.
-  `generate_secure_password` function L320-332 — `(length: usize) -> String` — Note: This module is only available when using the PostgreSQL backend.
-  `tests` module L335-433 — `-` — Note: This module is only available when using the PostgreSQL backend.
-  `test_generate_secure_password` function L339-349 — `()` — Note: This module is only available when using the PostgreSQL backend.
-  `test_tenant_config_validation` function L352-364 — `()` — Note: This module is only available when using the PostgreSQL backend.
-  `test_username_validation_rejects_sql_injection` function L367-387 — `()` — Note: This module is only available when using the PostgreSQL backend.
-  `test_schema_validation_rejects_sql_injection` function L390-404 — `()` — Note: This module is only available when using the PostgreSQL backend.
-  `test_reserved_usernames_rejected` function L407-419 — `()` — Note: This module is only available when using the PostgreSQL backend.
-  `test_password_escaping` function L422-432 — `()` — Note: This module is only available when using the PostgreSQL backend.

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

- pub `unified` module L1210-1212 — `-`
- pub `postgres` module L1217-1219 — `-`
- pub `sqlite` module L1222-1224 — `-`
-  `unified_schema` module L25-449 — `-`
-  `postgres_schema` module L456-864 — `-`
-  `sqlite_schema` module L867-1205 — `-`

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
- pub `run_migrations` function L374-451 — `(&self) -> Result<(), String>` — Runs pending database migrations for the appropriate backend.
- pub `setup_schema` function L463-516 — `(&self, schema: &str) -> Result<(), String>` — Sets up the PostgreSQL schema for multi-tenant isolation.
- pub `get_connection_with_schema` function L526-564 — `( &self, ) -> Result< deadpool::managed::Object<PgManager>, deadpool::managed::P...` — Gets a PostgreSQL connection with the schema search path set.
- pub `get_postgres_connection` function L570-577 — `( &self, ) -> Result< deadpool::managed::Object<PgManager>, deadpool::managed::P...` — Gets a PostgreSQL connection.
- pub `get_sqlite_connection` function L583-601 — `( &self, ) -> Result< deadpool::managed::Object<SqliteManager>, deadpool::manage...` — Gets a SQLite connection.
-  `backend` module L51 — `-` — Database connection management module supporting both PostgreSQL and SQLite.
-  `schema_validation` module L52 — `-` — ```
-  `Database` type L125-133 — `= Database` — ```
-  `fmt` function L126-132 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — ```
-  `Database` type L135-602 — `= Database` — ```
-  `build_postgres_url` function L355-359 — `(base_url: &str, database_name: &str) -> Result<String, url::ParseError>` — Builds a PostgreSQL connection URL.
-  `build_sqlite_url` function L362-369 — `(connection_string: &str) -> String` — Builds a SQLite connection URL.
-  `tests` module L605-702 — `-` — ```
-  `test_postgres_url_parsing_scenarios` function L609-633 — `()` — ```
-  `test_sqlite_connection_strings` function L636-652 — `()` — ```
-  `test_backend_type_detection` function L655-701 — `()` — ```

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
-  `DefaultDispatcher` type L61-131 — `= DefaultDispatcher` — configurable glob patterns.
-  `handle_result` function L87-130 — `( &self, event: &TaskReadyEvent, result: super::types::ExecutionResult, ) -> Res...` — Handles the execution result by updating database state.
-  `DefaultDispatcher` type L134-183 — `impl Dispatcher for DefaultDispatcher` — configurable glob patterns.
-  `dispatch` function L135-163 — `(&self, event: TaskReadyEvent) -> Result<(), DispatchError>` — configurable glob patterns.
-  `register_executor` function L165-173 — `(&self, key: &str, executor: Arc<dyn TaskExecutor>)` — configurable glob patterns.
-  `has_capacity` function L175-178 — `(&self) -> bool` — configurable glob patterns.
-  `resolve_executor_key` function L180-182 — `(&self, task_name: &str) -> String` — configurable glob patterns.
-  `tests` module L186-273 — `-` — configurable glob patterns.
-  `MockExecutor` struct L194-198 — `{ name: String, has_capacity: AtomicBool, execute_count: AtomicUsize }` — Mock executor for testing
-  `MockExecutor` type L200-213 — `= MockExecutor` — configurable glob patterns.
-  `new` function L201-207 — `(name: &str) -> Self` — configurable glob patterns.
-  `execution_count` function L210-212 — `(&self) -> usize` — configurable glob patterns.
-  `MockExecutor` type L216-242 — `impl TaskExecutor for MockExecutor` — configurable glob patterns.
-  `execute` function L217-223 — `(&self, event: TaskReadyEvent) -> Result<ExecutionResult, DispatchError>` — configurable glob patterns.
-  `has_capacity` function L225-227 — `(&self) -> bool` — configurable glob patterns.
-  `metrics` function L229-237 — `(&self) -> ExecutorMetrics` — configurable glob patterns.
-  `name` function L239-241 — `(&self) -> &str` — configurable glob patterns.
-  `create_test_event` function L245-252 — `(task_name: &str) -> TaskReadyEvent` — configurable glob patterns.
-  `test_register_executor` function L255-261 — `()` — configurable glob patterns.
-  `test_resolve_executor_key` function L264-272 — `()` — configurable glob patterns.

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

- pub `TaskReadyEvent` struct L31-40 — `{ task_execution_id: UniversalUuid, pipeline_execution_id: UniversalUuid, task_n...` — Event emitted when a task becomes ready for execution.
- pub `new` function L44-56 — `( task_execution_id: UniversalUuid, pipeline_execution_id: UniversalUuid, task_n...` — Creates a new TaskReadyEvent.
- pub `ExecutionStatus` enum L61-68 — `Completed | Failed | Retry` — Simplified status for execution results.
- pub `ExecutionResult` struct L75-84 — `{ task_execution_id: UniversalUuid, status: ExecutionStatus, error: Option<Strin...` — Result of task execution from an executor.
- pub `success` function L88-95 — `(task_execution_id: UniversalUuid, duration: Duration) -> Self` — Creates a successful execution result.
- pub `failure` function L98-109 — `( task_execution_id: UniversalUuid, error: impl Into<String>, duration: Duration...` — Creates a failed execution result.
- pub `retry` function L112-123 — `( task_execution_id: UniversalUuid, error: impl Into<String>, duration: Duration...` — Creates a retry execution result.
- pub `ExecutorMetrics` struct L128-139 — `{ active_tasks: usize, max_concurrent: usize, total_executed: u64, total_failed:...` — Metrics for monitoring executor performance.
- pub `available_capacity` function L143-145 — `(&self) -> usize` — Returns the current capacity (available slots).
- pub `RoutingConfig` struct L153-158 — `{ default_executor: String, rules: Vec<RoutingRule> }` — Configuration for task routing.
- pub `new` function L171-176 — `(default_executor: impl Into<String>) -> Self` — Creates a new routing configuration with a default executor.
- pub `with_rule` function L179-182 — `(mut self, rule: RoutingRule) -> Self` — Adds a routing rule.
- pub `with_rules` function L185-188 — `(mut self, rules: impl IntoIterator<Item = RoutingRule>) -> Self` — Adds multiple routing rules.
- pub `RoutingRule` struct L196-201 — `{ task_pattern: String, executor: String }` — A routing rule for directing tasks to specific executors.
- pub `new` function L205-210 — `(task_pattern: impl Into<String>, executor: impl Into<String>) -> Self` — Creates a new routing rule.
- pub `DispatchError` enum L215-243 — `ExecutorNotFound | ExecutionFailed | DatabaseError | ContextError | ValidationEr...` — Errors that can occur during dispatch operations.
-  `TaskReadyEvent` type L42-57 — `= TaskReadyEvent` — tasks from the scheduler to executors.
-  `ExecutionResult` type L86-124 — `= ExecutionResult` — tasks from the scheduler to executors.
-  `ExecutorMetrics` type L141-146 — `= ExecutorMetrics` — tasks from the scheduler to executors.
-  `RoutingConfig` type L160-167 — `impl Default for RoutingConfig` — tasks from the scheduler to executors.
-  `default` function L161-166 — `() -> Self` — tasks from the scheduler to executors.
-  `RoutingConfig` type L169-189 — `= RoutingConfig` — tasks from the scheduler to executors.
-  `RoutingRule` type L203-211 — `= RoutingRule` — tasks from the scheduler to executors.

#### crates/cloacina/src/dispatcher/work_distributor.rs

- pub `WorkDistributor` interface L56-71 — `{ fn wait_for_work(), fn shutdown() }` — Trait for abstracting work notification mechanisms.
- pub `PostgresDistributor` struct L85-94 — `{ database_url: String, notify: Arc<Notify>, shutdown: Arc<std::sync::atomic::At...` — PostgreSQL work distributor using LISTEN/NOTIFY.
- pub `new` function L113-128 — `(database_url: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>` — Creates a new PostgreSQL work distributor.
- pub `SqliteDistributor` struct L257-264 — `{ poll_interval: Duration, shutdown: Arc<std::sync::atomic::AtomicBool>, notify:...` — SQLite work distributor using periodic polling.
- pub `new` function L272-274 — `() -> Self` — Creates a new SQLite work distributor with default poll interval (500ms).
- pub `with_poll_interval` function L281-287 — `(poll_interval: Duration) -> Self` — Creates a new SQLite work distributor with custom poll interval.
- pub `create_work_distributor` function L331-346 — `( database: &crate::Database, ) -> Result<Box<dyn WorkDistributor>, Box<dyn std:...` — Creates the appropriate work distributor based on database backend.
-  `PostgresDistributor` type L97-218 — `= PostgresDistributor` — ```
-  `POLL_FALLBACK` variable L99 — `: Duration` — Fallback poll interval when no notifications received
-  `spawn_listener` function L131-217 — `( database_url: String, notify: Arc<Notify>, shutdown: Arc<std::sync::atomic::At...` — Spawns the background listener task.
-  `PostgresDistributor` type L222-240 — `impl WorkDistributor for PostgresDistributor` — ```
-  `wait_for_work` function L223-233 — `(&self)` — ```
-  `shutdown` function L235-239 — `(&self)` — ```
-  `PostgresDistributor` type L243-250 — `impl Drop for PostgresDistributor` — ```
-  `drop` function L244-249 — `(&mut self)` — ```
-  `SqliteDistributor` type L267-288 — `= SqliteDistributor` — ```
-  `DEFAULT_POLL_INTERVAL` variable L269 — `: Duration` — Default poll interval for SQLite
-  `SqliteDistributor` type L291-295 — `impl Default for SqliteDistributor` — ```
-  `default` function L292-294 — `() -> Self` — ```
-  `SqliteDistributor` type L299-320 — `impl WorkDistributor for SqliteDistributor` — ```
-  `wait_for_work` function L300-313 — `(&self)` — ```
-  `shutdown` function L315-319 — `(&self)` — ```
-  `tests` module L349-388 — `-` — ```
-  `test_sqlite_distributor_poll_interval` function L354-364 — `()` — ```
-  `test_sqlite_distributor_shutdown` function L368-387 — `()` — ```

### crates/cloacina/src/executor

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/executor/mod.rs

- pub `pipeline_executor` module L47 — `-` — # Task Executor
- pub `slot_token` module L48 — `-` — All components are thread-safe and can be used in concurrent environments.
- pub `task_handle` module L49 — `-` — All components are thread-safe and can be used in concurrent environments.
- pub `thread_task_executor` module L50 — `-` — All components are thread-safe and can be used in concurrent environments.
- pub `types` module L51 — `-` — All components are thread-safe and can be used in concurrent environments.

#### crates/cloacina/src/executor/pipeline_executor.rs

- pub `StatusCallback` interface L59-66 — `{ fn on_status_change() }` — Callback trait for receiving real-time status updates during pipeline execution.
- pub `TaskResult` struct L73-88 — `{ task_name: String, status: TaskState, start_time: Option<DateTime<Utc>>, end_t...` — Represents the outcome of a single task execution within a pipeline.
- pub `PipelineError` enum L96-120 — `DatabaseConnection | WorkflowNotFound | ExecutionFailed | Timeout | Validation |...` — Unified error type for pipeline execution operations.
- pub `PipelineStatus` enum L128-141 — `Pending | Running | Completed | Failed | Cancelled | Paused` — Represents the current state of a pipeline execution.
- pub `is_terminal` function L151-156 — `(&self) -> bool` — Determines if this status represents a terminal state.
- pub `PipelineResult` struct L164-183 — `{ execution_id: Uuid, workflow_name: String, status: PipelineStatus, start_time:...` — Contains the complete result of a pipeline execution.
- pub `PipelineExecution` struct L189-195 — `{ execution_id: Uuid, workflow_name: String, executor: crate::runner::DefaultRun...` — Handle for managing an asynchronous pipeline execution.
- pub `new` function L205-215 — `( execution_id: Uuid, workflow_name: String, executor: crate::runner::DefaultRun...` — Creates a new pipeline execution handle.
- pub `wait_for_completion` function L225-227 — `(self) -> Result<PipelineResult, PipelineError>` — Waits for the pipeline to complete execution.
- pub `wait_for_completion_with_timeout` function L239-269 — `( self, timeout: Option<Duration>, ) -> Result<PipelineResult, PipelineError>` — Waits for completion with a specified timeout.
- pub `get_status` function L277-279 — `(&self) -> Result<PipelineStatus, PipelineError>` — Gets the current status of the pipeline execution.
- pub `cancel` function L289-291 — `(&self) -> Result<(), PipelineError>` — Cancels the pipeline execution.
- pub `pause` function L306-310 — `(&self, reason: Option<&str>) -> Result<(), PipelineError>` — Pauses the pipeline execution.
- pub `resume` function L321-323 — `(&self) -> Result<(), PipelineError>` — Resumes a paused pipeline execution.
- pub `PipelineExecutor` interface L332-484 — `{ fn execute(), fn execute_async(), fn get_execution_status(), fn get_execution_...` — Core trait defining the interface for pipeline execution engines.
-  `PipelineStatus` type L143-157 — `= PipelineStatus` — ```
-  `PipelineExecution` type L197-324 — `= PipelineExecution` — ```
-  `PipelineStatus` type L486-519 — `= PipelineStatus` — ```
-  `from_str` function L508-518 — `(s: &str) -> Self` — Creates a PipelineStatus from a string representation.

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
- pub `defer_until` function L162-227 — `( &mut self, condition: F, poll_interval: Duration, ) -> Result<(), ExecutorErro...` — Release the concurrency slot while polling an external condition.
- pub `task_execution_id` function L230-232 — `(&self) -> UniversalUuid` — Returns the task execution ID associated with this handle.
- pub `is_slot_held` function L235-237 — `(&self) -> bool` — Returns whether the handle currently holds a concurrency slot.
-  `TaskHandle` type L116-246 — `= TaskHandle` — ```
-  `new` function L120-126 — `(slot_token: SlotToken, task_execution_id: UniversalUuid) -> Self` — Creates a new TaskHandle.
-  `with_dal` function L129-139 — `( slot_token: SlotToken, task_execution_id: UniversalUuid, dal: DAL, ) -> Self` — Creates a new TaskHandle with DAL for sub_status persistence.
-  `into_slot_token` function L243-245 — `(self) -> SlotToken` — Consumes the handle, returning the inner SlotToken.
-  `tests` module L249-410 — `-` — ```
-  `make_handle` function L255-262 — `(semaphore: &Arc<Semaphore>) -> TaskHandle` — ```
-  `test_defer_until_releases_and_reclaims_slot` function L265-293 — `()` — ```
-  `test_defer_until_immediate_condition` function L296-307 — `()` — ```
-  `test_defer_until_frees_slot_for_other_tasks` function L310-341 — `()` — ```
-  `test_task_local_round_trip` function L344-366 — `()` — ```
-  `test_task_local_not_returned_yields_none` function L369-384 — `()` — ```
-  `test_with_task_handle_preserves_handle_through_defer` function L387-409 — `()` — ```

#### crates/cloacina/src/executor/thread_task_executor.rs

- pub `ThreadTaskExecutor` struct L71-88 — `{ database: Database, dal: DAL, task_registry: Arc<TaskRegistry>, instance_id: U...` — ThreadTaskExecutor is a thread-based implementation of task execution.
- pub `new` function L100-118 — `( database: Database, task_registry: Arc<TaskRegistry>, config: ExecutorConfig, ...` — Creates a new ThreadTaskExecutor instance.
- pub `with_global_registry` function L131-145 — `( database: Database, config: ExecutorConfig, ) -> Result<Self, crate::error::Re...` — Creates a TaskExecutor using the global task registry.
- pub `semaphore` function L151-153 — `(&self) -> &Arc<Semaphore>` — Returns a reference to the concurrency semaphore.
-  `ThreadTaskExecutor` type L90-660 — `= ThreadTaskExecutor` — to the executor based on routing rules.
-  `build_task_context` function L163-284 — `( &self, claimed_task: &ClaimedTask, dependencies: &[crate::task::TaskNamespace]...` — Builds the execution context for a task by loading its dependencies.
-  `merge_context_values` function L298-333 — `( existing: &serde_json::Value, new: &serde_json::Value, ) -> serde_json::Value` — Merges two context values using smart merging strategy.
-  `execute_with_timeout` function L343-352 — `( &self, task: &dyn Task, context: Context<serde_json::Value>, ) -> Result<Conte...` — Executes a task with timeout protection.
-  `handle_task_result` function L368-414 — `( &self, claimed_task: ClaimedTask, result: Result<Context<serde_json::Value>, E...` — Handles the result of task execution.
-  `save_task_context` function L424-454 — `( &self, claimed_task: &ClaimedTask, context: Context<serde_json::Value>, ) -> R...` — Saves the task's execution context to the database.
-  `mark_task_completed` function L463-484 — `( &self, task_execution_id: UniversalUuid, ) -> Result<(), ExecutorError>` — Marks a task as completed in the database.
-  `complete_task_transaction` function L497-510 — `( &self, claimed_task: &ClaimedTask, context: Context<serde_json::Value>, ) -> R...` — Completes a task by saving its context and marking it as completed in a single transaction.
-  `mark_task_failed` function L520-543 — `( &self, task_execution_id: UniversalUuid, error: &ExecutorError, ) -> Result<()...` — Marks a task as failed in the database.
-  `should_retry_task` function L559-596 — `( &self, claimed_task: &ClaimedTask, error: &ExecutorError, retry_policy: &Retry...` — Determines if a failed task should be retried.
-  `is_transient_error` function L605-622 — `(&self, error: &ExecutorError) -> bool` — Determines if an error is transient and potentially retryable.
-  `schedule_task_retry` function L632-659 — `( &self, claimed_task: &ClaimedTask, retry_policy: &RetryPolicy, ) -> Result<(),...` — Schedules a task for retry execution.
-  `ThreadTaskExecutor` type L662-676 — `impl Clone for ThreadTaskExecutor` — to the executor based on routing rules.
-  `clone` function L663-675 — `(&self) -> Self` — to the executor based on routing rules.
-  `ThreadTaskExecutor` type L683-871 — `impl TaskExecutor for ThreadTaskExecutor` — Implementation of the dispatcher's TaskExecutor trait.
-  `execute` function L684-850 — `(&self, event: TaskReadyEvent) -> Result<ExecutionResult, DispatchError>` — to the executor based on routing rules.
-  `has_capacity` function L852-854 — `(&self) -> bool` — to the executor based on routing rules.
-  `metrics` function L856-866 — `(&self) -> ExecutorMetrics` — to the executor based on routing rules.
-  `name` function L868-870 — `(&self) -> &str` — to the executor based on routing rules.

#### crates/cloacina/src/executor/types.rs

- pub `ExecutionScope` struct L37-44 — `{ pipeline_execution_id: UniversalUuid, task_execution_id: Option<UniversalUuid>...` — Execution scope information for a context
- pub `DependencyLoader` struct L52-61 — `{ database: Database, pipeline_execution_id: UniversalUuid, dependency_tasks: Ve...` — Dependency loader for automatic context merging with lazy loading
- pub `new` function L70-81 — `( database: Database, pipeline_execution_id: UniversalUuid, dependency_tasks: Ve...` — Creates a new dependency loader instance
- pub `load_from_dependencies` function L93-130 — `( &self, key: &str, ) -> Result<Option<serde_json::Value>, ExecutorError>` — Loads a value from dependency contexts using a "latest wins" strategy
- pub `ExecutorConfig` struct L164-169 — `{ max_concurrent_tasks: usize, task_timeout: std::time::Duration }` — Configuration settings for the executor
- pub `ClaimedTask` struct L190-199 — `{ task_execution_id: UniversalUuid, pipeline_execution_id: UniversalUuid, task_n...` — Represents a task that has been claimed for execution
-  `DependencyLoader` type L63-157 — `= DependencyLoader` — and configure the behavior of the execution engine.
-  `load_dependency_context_data` function L139-156 — `( &self, task_namespace: &crate::task::TaskNamespace, ) -> Result<HashMap<String...` — Loads the context data for a specific dependency task
-  `ExecutorConfig` type L171-183 — `impl Default for ExecutorConfig` — and configure the behavior of the execution engine.
-  `default` function L177-182 — `() -> Self` — Creates a new executor configuration with default values

### crates/cloacina/src/models

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/models/context.rs

- pub `DbContext` struct L31-36 — `{ id: UniversalUuid, value: String, created_at: UniversalTimestamp, updated_at: ...` — Represents a context record (domain type).
- pub `NewDbContext` struct L40-42 — `{ value: String }` — Structure for creating new context records (domain type).
-  `tests` module L45-72 — `-` — models handle actual database interaction.
-  `test_db_context_creation` function L50-62 — `()` — models handle actual database interaction.
-  `test_new_db_context_creation` function L65-71 — `()` — models handle actual database interaction.

#### crates/cloacina/src/models/cron_execution.rs

- pub `CronExecution` struct L28-36 — `{ id: UniversalUuid, schedule_id: UniversalUuid, pipeline_execution_id: Option<U...` — Represents a cron execution audit record (domain type).
- pub `NewCronExecution` struct L40-48 — `{ id: Option<UniversalUuid>, schedule_id: UniversalUuid, pipeline_execution_id: ...` — Structure for creating new cron execution audit records (domain type).
- pub `new` function L52-62 — `(schedule_id: UniversalUuid, scheduled_time: UniversalTimestamp) -> Self` — Creates a new cron execution audit record for guaranteed execution.
- pub `with_pipeline_execution` function L65-79 — `( schedule_id: UniversalUuid, pipeline_execution_id: UniversalUuid, scheduled_ti...` — Creates a new cron execution record with pipeline execution ID.
- pub `with_claimed_at` function L82-98 — `( schedule_id: UniversalUuid, pipeline_execution_id: Option<UniversalUuid>, sche...` — Creates a new cron execution record with a specific claimed_at time.
- pub `scheduled_time` function L102-104 — `(&self) -> DateTime<Utc>` — to the pipeline executor.
- pub `claimed_at` function L106-108 — `(&self) -> DateTime<Utc>` — to the pipeline executor.
- pub `created_at` function L110-112 — `(&self) -> DateTime<Utc>` — to the pipeline executor.
- pub `updated_at` function L114-116 — `(&self) -> DateTime<Utc>` — to the pipeline executor.
- pub `execution_delay` function L118-120 — `(&self) -> chrono::Duration` — to the pipeline executor.
- pub `is_timely` function L122-125 — `(&self, tolerance: chrono::Duration) -> bool` — to the pipeline executor.
-  `NewCronExecution` type L50-99 — `= NewCronExecution` — to the pipeline executor.
-  `CronExecution` type L101-126 — `= CronExecution` — to the pipeline executor.
-  `tests` module L129-168 — `-` — to the pipeline executor.
-  `test_new_cron_execution` function L135-145 — `()` — to the pipeline executor.
-  `test_cron_execution_delays` function L148-167 — `()` — to the pipeline executor.

#### crates/cloacina/src/models/cron_schedule.rs

- pub `CronSchedule` struct L28-41 — `{ id: UniversalUuid, workflow_name: String, cron_expression: String, timezone: S...` — Represents a cron schedule record (domain type).
- pub `NewCronSchedule` struct L45-54 — `{ workflow_name: String, cron_expression: String, timezone: Option<String>, enab...` — Structure for creating new cron schedule records (domain type).
- pub `CatchupPolicy` enum L58-61 — `Skip | RunAll` — Enum representing the different catchup policies for missed executions.
- pub `ScheduleConfig` struct L90-98 — `{ name: String, cron: String, workflow: String, timezone: String, catchup_policy...` — Configuration structure for creating new cron schedules.
-  `String` type L63-70 — `= String` — These are API-level types; backend-specific models handle database storage.
-  `from` function L64-69 — `(policy: CatchupPolicy) -> Self` — These are API-level types; backend-specific models handle database storage.
-  `CatchupPolicy` type L72-80 — `= CatchupPolicy` — These are API-level types; backend-specific models handle database storage.
-  `from` function L73-79 — `(s: String) -> Self` — These are API-level types; backend-specific models handle database storage.
-  `CatchupPolicy` type L82-86 — `= CatchupPolicy` — These are API-level types; backend-specific models handle database storage.
-  `from` function L83-85 — `(s: &str) -> Self` — These are API-level types; backend-specific models handle database storage.
-  `ScheduleConfig` type L100-112 — `impl Default for ScheduleConfig` — These are API-level types; backend-specific models handle database storage.
-  `default` function L101-111 — `() -> Self` — These are API-level types; backend-specific models handle database storage.
-  `tests` module L115-150 — `-` — These are API-level types; backend-specific models handle database storage.
-  `test_cron_schedule_creation` function L120-140 — `()` — These are API-level types; backend-specific models handle database storage.
-  `test_catchup_policy_conversions` function L143-149 — `()` — These are API-level types; backend-specific models handle database storage.

#### crates/cloacina/src/models/execution_event.rs

- pub `ExecutionEvent` struct L34-51 — `{ id: UniversalUuid, pipeline_execution_id: UniversalUuid, task_execution_id: Op...` — Represents an execution event record (domain type).
- pub `NewExecutionEvent` struct L55-66 — `{ pipeline_execution_id: UniversalUuid, task_execution_id: Option<UniversalUuid>...` — Structure for creating new execution event records (domain type).
- pub `pipeline_event` function L70-83 — `( pipeline_execution_id: UniversalUuid, event_type: ExecutionEventType, event_da...` — Creates a new execution event for a pipeline-level transition.
- pub `task_event` function L86-100 — `( pipeline_execution_id: UniversalUuid, task_execution_id: UniversalUuid, event_...` — Creates a new execution event for a task-level transition.
- pub `ExecutionEventType` enum L108-146 — `TaskCreated | TaskMarkedReady | TaskClaimed | TaskStarted | TaskDeferred | TaskR...` — Enumeration of execution event types in the system.
- pub `as_str` function L150-172 — `(&self) -> &'static str` — Returns the string representation of the event type.
- pub `from_str` function L175-196 — `(s: &str) -> Option<Self>` — Parses an event type from its string representation.
- pub `is_task_event` function L199-215 — `(&self) -> bool` — Returns true if this is a task-level event.
- pub `is_pipeline_event` function L218-227 — `(&self) -> bool` — Returns true if this is a pipeline-level event.
-  `NewExecutionEvent` type L68-101 — `= NewExecutionEvent` — These are API-level types; backend-specific models handle database storage.
-  `ExecutionEventType` type L148-228 — `= ExecutionEventType` — These are API-level types; backend-specific models handle database storage.
-  `String` type L230-234 — `= String` — These are API-level types; backend-specific models handle database storage.
-  `from` function L231-233 — `(event_type: ExecutionEventType) -> Self` — These are API-level types; backend-specific models handle database storage.
-  `ExecutionEventType` type L236-240 — `= ExecutionEventType` — These are API-level types; backend-specific models handle database storage.
-  `fmt` function L237-239 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — These are API-level types; backend-specific models handle database storage.

#### crates/cloacina/src/models/key_trust_acl.rs

- pub `KeyTrustAcl` struct L31-40 — `{ id: UniversalUuid, parent_org_id: UniversalUuid, child_org_id: UniversalUuid, ...` — Domain model for a key trust ACL (Access Control List).
- pub `is_active` function L44-46 — `(&self) -> bool` — Check if this trust relationship is currently active
- pub `is_revoked` function L49-51 — `(&self) -> bool` — Check if this trust relationship has been revoked
- pub `NewKeyTrustAcl` struct L56-59 — `{ parent_org_id: UniversalUuid, child_org_id: UniversalUuid }` — Model for creating a new key trust ACL.
- pub `new` function L62-67 — `(parent_org_id: UniversalUuid, child_org_id: UniversalUuid) -> Self` — trusts packages signed by the child org's trusted keys.
-  `KeyTrustAcl` type L42-52 — `= KeyTrustAcl` — trusts packages signed by the child org's trusted keys.
-  `NewKeyTrustAcl` type L61-68 — `= NewKeyTrustAcl` — trusts packages signed by the child org's trusted keys.

#### crates/cloacina/src/models/mod.rs

- pub `context` module L72 — `-` — - Keep model definitions in sync with database schema migrations
- pub `cron_execution` module L73 — `-` — - Keep model definitions in sync with database schema migrations
- pub `cron_schedule` module L74 — `-` — - Keep model definitions in sync with database schema migrations
- pub `execution_event` module L75 — `-` — - Keep model definitions in sync with database schema migrations
- pub `pipeline_execution` module L76 — `-` — - Keep model definitions in sync with database schema migrations
- pub `recovery_event` module L77 — `-` — - Keep model definitions in sync with database schema migrations
- pub `task_execution` module L78 — `-` — - Keep model definitions in sync with database schema migrations
- pub `task_execution_metadata` module L79 — `-` — - Keep model definitions in sync with database schema migrations
- pub `task_outbox` module L80 — `-` — - Keep model definitions in sync with database schema migrations
- pub `trigger_execution` module L81 — `-` — - Keep model definitions in sync with database schema migrations
- pub `trigger_schedule` module L82 — `-` — - Keep model definitions in sync with database schema migrations
- pub `workflow_packages` module L83 — `-` — - Keep model definitions in sync with database schema migrations
- pub `workflow_registry` module L84 — `-` — - Keep model definitions in sync with database schema migrations
- pub `key_trust_acl` module L87 — `-` — - Keep model definitions in sync with database schema migrations
- pub `package_signature` module L88 — `-` — - Keep model definitions in sync with database schema migrations
- pub `signing_key` module L89 — `-` — - Keep model definitions in sync with database schema migrations
- pub `trusted_key` module L90 — `-` — - Keep model definitions in sync with database schema migrations

#### crates/cloacina/src/models/package_signature.rs

- pub `PackageSignature` struct L28-37 — `{ id: UniversalUuid, package_hash: String, key_fingerprint: String, signature: V...` — Domain model for a package signature.
- pub `NewPackageSignature` struct L41-45 — `{ package_hash: String, key_fingerprint: String, signature: Vec<u8> }` — Model for creating a new package signature.
- pub `new` function L48-54 — `(package_hash: String, key_fingerprint: String, signature: Vec<u8>) -> Self` — the SHA256 hash of the package binary.
- pub `SignatureVerification` struct L59-68 — `{ is_valid: bool, signer_fingerprint: String, signed_at: UniversalTimestamp, sig...` — Result of signature verification.
-  `NewPackageSignature` type L47-55 — `= NewPackageSignature` — the SHA256 hash of the package binary.

#### crates/cloacina/src/models/pipeline_execution.rs

- pub `PipelineExecution` struct L27-42 — `{ id: UniversalUuid, pipeline_name: String, pipeline_version: String, status: St...` — Represents a pipeline execution (domain type).
- pub `NewPipelineExecution` struct L46-51 — `{ pipeline_name: String, pipeline_version: String, status: String, context_id: O...` — Structure for creating new pipeline executions (domain type).

#### crates/cloacina/src/models/recovery_event.rs

- pub `RecoveryEvent` struct L27-36 — `{ id: UniversalUuid, pipeline_execution_id: UniversalUuid, task_execution_id: Op...` — Represents a recovery event record (domain type).
- pub `NewRecoveryEvent` struct L40-45 — `{ pipeline_execution_id: UniversalUuid, task_execution_id: Option<UniversalUuid>...` — Structure for creating new recovery event records (domain type).
- pub `RecoveryType` enum L49-54 — `TaskReset | TaskAbandoned | PipelineFailed | WorkflowUnavailable` — Enumeration of possible recovery types in the system.
- pub `as_str` function L57-64 — `(&self) -> &'static str` — These are API-level types; backend-specific models handle database storage.
-  `RecoveryType` type L56-65 — `= RecoveryType` — These are API-level types; backend-specific models handle database storage.
-  `String` type L67-71 — `= String` — These are API-level types; backend-specific models handle database storage.
-  `from` function L68-70 — `(recovery_type: RecoveryType) -> Self` — These are API-level types; backend-specific models handle database storage.

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

- pub `TaskExecution` struct L27-46 — `{ id: UniversalUuid, pipeline_execution_id: UniversalUuid, task_name: String, st...` — Represents a task execution record (domain type).
- pub `NewTaskExecution` struct L50-58 — `{ pipeline_execution_id: UniversalUuid, task_name: String, status: String, attem...` — Structure for creating new task executions (domain type).

#### crates/cloacina/src/models/task_execution_metadata.rs

- pub `TaskExecutionMetadata` struct L27-35 — `{ id: UniversalUuid, task_execution_id: UniversalUuid, pipeline_execution_id: Un...` — Represents a task execution metadata record (domain type).
- pub `NewTaskExecutionMetadata` struct L39-44 — `{ task_execution_id: UniversalUuid, pipeline_execution_id: UniversalUuid, task_n...` — Structure for creating new task execution metadata (domain type).

#### crates/cloacina/src/models/task_outbox.rs

- pub `TaskOutbox` struct L37-44 — `{ id: i64, task_execution_id: UniversalUuid, created_at: UniversalTimestamp }` — Represents a task outbox entry (domain type).
- pub `NewTaskOutbox` struct L50-53 — `{ task_execution_id: UniversalUuid }` — Structure for creating new task outbox entries (domain type).

#### crates/cloacina/src/models/trigger_execution.rs

- pub `TriggerExecution` struct L29-38 — `{ id: UniversalUuid, trigger_name: String, context_hash: String, pipeline_execut...` — Represents a trigger execution audit record (domain type).
- pub `is_in_progress` function L42-44 — `(&self) -> bool` — Returns true if this execution is currently in progress (not completed).
- pub `duration` function L47-50 — `(&self) -> Option<chrono::Duration>` — Returns the duration of this execution if completed.
- pub `started_at` function L52-54 — `(&self) -> DateTime<Utc>` — These are API-level types; backend-specific models handle database storage.
- pub `completed_at` function L56-58 — `(&self) -> Option<DateTime<Utc>>` — These are API-level types; backend-specific models handle database storage.
- pub `NewTriggerExecution` struct L63-70 — `{ id: Option<UniversalUuid>, trigger_name: String, context_hash: String, pipelin...` — Structure for creating new trigger execution audit records (domain type).
- pub `new` function L74-83 — `(trigger_name: &str, context_hash: &str) -> Self` — Creates a new trigger execution record.
- pub `with_pipeline_execution` function L86-99 — `( trigger_name: &str, context_hash: &str, pipeline_execution_id: UniversalUuid, ...` — Creates a new trigger execution record with pipeline execution ID.
- pub `with_started_at` function L102-116 — `( trigger_name: &str, context_hash: &str, pipeline_execution_id: Option<Universa...` — Creates a new trigger execution record with a specific started_at time.
-  `TriggerExecution` type L40-59 — `= TriggerExecution` — These are API-level types; backend-specific models handle database storage.
-  `NewTriggerExecution` type L72-117 — `= NewTriggerExecution` — These are API-level types; backend-specific models handle database storage.
-  `tests` module L120-174 — `-` — These are API-level types; backend-specific models handle database storage.
-  `test_new_trigger_execution` function L126-134 — `()` — These are API-level types; backend-specific models handle database storage.
-  `test_trigger_execution_in_progress` function L137-152 — `()` — These are API-level types; backend-specific models handle database storage.
-  `test_trigger_execution_completed` function L155-173 — `()` — These are API-level types; backend-specific models handle database storage.

#### crates/cloacina/src/models/trigger_schedule.rs

- pub `TriggerSchedule` struct L28-38 — `{ id: UniversalUuid, trigger_name: String, workflow_name: String, poll_interval_...` — Represents a trigger schedule record (domain type).
- pub `poll_interval` function L42-44 — `(&self) -> Duration` — Returns the poll interval as a Duration.
- pub `is_enabled` function L47-49 — `(&self) -> bool` — Returns true if the trigger is enabled.
- pub `allows_concurrent` function L52-54 — `(&self) -> bool` — Returns true if concurrent executions are allowed.
- pub `NewTriggerSchedule` struct L59-66 — `{ id: Option<UniversalUuid>, trigger_name: String, workflow_name: String, poll_i...` — Structure for creating new trigger schedule records (domain type).
- pub `new` function L70-79 — `(trigger_name: &str, workflow_name: &str, poll_interval: Duration) -> Self` — Creates a new trigger schedule.
- pub `with_allow_concurrent` function L82-85 — `(mut self, allow: bool) -> Self` — Sets whether concurrent executions are allowed.
- pub `with_enabled` function L88-91 — `(mut self, enabled: bool) -> Self` — Sets whether the trigger is enabled.
-  `TriggerSchedule` type L40-55 — `= TriggerSchedule` — These are API-level types; backend-specific models handle database storage.
-  `NewTriggerSchedule` type L68-92 — `= NewTriggerSchedule` — These are API-level types; backend-specific models handle database storage.
-  `tests` module L95-143 — `-` — These are API-level types; backend-specific models handle database storage.
-  `test_trigger_schedule_creation` function L100-119 — `()` — These are API-level types; backend-specific models handle database storage.
-  `test_new_trigger_schedule` function L122-131 — `()` — These are API-level types; backend-specific models handle database storage.
-  `test_new_trigger_schedule_builders` function L134-142 — `()` — These are API-level types; backend-specific models handle database storage.

#### crates/cloacina/src/models/trusted_key.rs

- pub `TrustedKey` struct L28-40 — `{ id: UniversalUuid, org_id: UniversalUuid, key_fingerprint: String, public_key:...` — Domain model for a trusted public key.
- pub `is_active` function L44-46 — `(&self) -> bool` — Check if this key is currently trusted (not revoked)
- pub `is_revoked` function L49-51 — `(&self) -> bool` — Check if this key has been revoked
- pub `NewTrustedKey` struct L56-61 — `{ org_id: UniversalUuid, key_fingerprint: String, public_key: Vec<u8>, key_name:...` — Model for creating a new trusted key.
- pub `new` function L64-76 — `( org_id: UniversalUuid, key_fingerprint: String, public_key: Vec<u8>, key_name:...` — derived from the organization's own signing keys.
- pub `from_signing_key` function L79-91 — `( org_id: UniversalUuid, key_fingerprint: String, public_key: Vec<u8>, key_name:...` — Create a trusted key from a signing key's public key.
-  `TrustedKey` type L42-52 — `= TrustedKey` — derived from the organization's own signing keys.
-  `NewTrustedKey` type L63-92 — `= NewTrustedKey` — derived from the organization's own signing keys.

#### crates/cloacina/src/models/workflow_packages.rs

- pub `StorageType` enum L27-32 — `Database | Filesystem` — Storage type for workflow binary data.
- pub `as_str` function L35-40 — `(&self) -> &'static str` — These are API-level types; backend-specific models handle database storage.
- pub `WorkflowPackage` struct L62-73 — `{ id: UniversalUuid, registry_id: UniversalUuid, package_name: String, version: ...` — Domain model for workflow package metadata.
- pub `NewWorkflowPackage` struct L77-85 — `{ registry_id: UniversalUuid, package_name: String, version: String, description...` — Model for creating new workflow package metadata entries (domain type).
- pub `new` function L88-106 — `( registry_id: UniversalUuid, package_name: String, version: String, description...` — These are API-level types; backend-specific models handle database storage.
-  `StorageType` type L34-41 — `= StorageType` — These are API-level types; backend-specific models handle database storage.
-  `StorageType` type L43-52 — `= StorageType` — These are API-level types; backend-specific models handle database storage.
-  `Err` type L44 — `= std::convert::Infallible` — These are API-level types; backend-specific models handle database storage.
-  `from_str` function L46-51 — `(s: &str) -> Result<Self, Self::Err>` — These are API-level types; backend-specific models handle database storage.
-  `StorageType` type L54-58 — `= StorageType` — These are API-level types; backend-specific models handle database storage.
-  `fmt` function L55-57 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — These are API-level types; backend-specific models handle database storage.
-  `NewWorkflowPackage` type L87-107 — `= NewWorkflowPackage` — These are API-level types; backend-specific models handle database storage.

#### crates/cloacina/src/models/workflow_registry.rs

- pub `WorkflowRegistryEntry` struct L27-31 — `{ id: UniversalUuid, created_at: UniversalTimestamp, data: Vec<u8> }` — Domain model for a workflow registry entry.
- pub `NewWorkflowRegistryEntry` struct L35-37 — `{ data: Vec<u8> }` — Model for creating new workflow registry entries (domain type).
- pub `new` function L40-42 — `(data: Vec<u8>) -> Self` — These are API-level types; backend-specific models handle database storage.
- pub `NewWorkflowRegistryEntryWithId` struct L47-51 — `{ id: UniversalUuid, created_at: UniversalTimestamp, data: Vec<u8> }` — Model for creating new workflow registry entries with explicit ID and timestamp.
-  `NewWorkflowRegistryEntry` type L39-43 — `= NewWorkflowRegistryEntry` — These are API-level types; backend-specific models handle database storage.

### crates/cloacina/src/packaging

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/packaging/archive.rs

- pub `create_package_archive` function L26-63 — `(compile_result: &CompileResult, output: &PathBuf) -> Result<()>` — Create a package archive from compilation results

#### crates/cloacina/src/packaging/compile.rs

- pub `compile_workflow` function L28-61 — `( project_path: PathBuf, output: PathBuf, options: CompileOptions, ) -> Result<C...` — Compile a workflow project to a dynamic library with manifest
-  `execute_cargo_build` function L63-106 — `(project_path: &PathBuf, options: &CompileOptions) -> Result<PathBuf>`
-  `find_compiled_library` function L108-154 — `( project_path: &Path, target: &Option<String>, profile: &str, ) -> Result<PathB...`
-  `copy_output_file` function L156-167 — `(source: &PathBuf, destination: &PathBuf) -> Result<()>`

#### crates/cloacina/src/packaging/debug.rs

- pub `extract_manifest_from_package` function L37-62 — `(package_path: &PathBuf) -> Result<PackageManifest>` — Extract the manifest from a package archive.
- pub `extract_library_from_package` function L65-110 — `( package_path: &PathBuf, manifest: &PackageManifest, temp_dir: &tempfile::TempD...` — Extract the dynamic library from a package archive to a temporary location.
- pub `execute_task_from_library` function L113-190 — `( library_path: &PathBuf, task_name: &str, context_json: &str, ) -> Result<Strin...` — Execute a task from a dynamic library.
- pub `resolve_task_name` function L193-220 — `(manifest: &PackageManifest, task_identifier: &str) -> Result<String>` — Resolve a task identifier (index or name) to a task name.
- pub `debug_package` function L223-276 — `( package_path: &PathBuf, task_identifier: Option<&str>, context_json: Option<&s...` — High-level debug function that handles both listing and executing tasks.
- pub `DebugResult` enum L280-283 — `TaskList | TaskExecution` — Result of a debug operation.
- pub `TaskDebugInfo` struct L287-293 — `{ index: usize, id: String, description: String, dependencies: Vec<String>, sour...` — Information about a task for debugging purposes.
-  `MANIFEST_FILENAME` variable L33 — `: &str` — for testing and development purposes.
-  `EXECUTE_TASK_SYMBOL` variable L34 — `: &str` — for testing and development purposes.
-  `RESULT_BUFFER_SIZE` variable L143 — `: usize` — for testing and development purposes.

#### crates/cloacina/src/packaging/manifest.rs

- pub `ManifestError` enum L46-93 — `NullPointer | MisalignedPointer | NullString | InvalidUtf8 | InvalidDependencies...` — Errors that can occur during manifest extraction from FFI.
- pub `generate_manifest` function L202-251 — `( cargo_toml: &CargoToml, so_path: &Path, target: &Option<String>, project_path:...` — Generate a package manifest from Cargo.toml and compiled library
-  `MAX_TASKS` variable L32 — `: usize` — Maximum number of tasks allowed in a single package.
-  `PACKAGED_WORKFLOW_REGEX` variable L36-39 — `: Lazy<Regex>` — Statically compiled regex for matching packaged_workflow attributes.
-  `safe_cstr_to_string` function L109-125 — `( ptr: *const c_char, field_name: &str, ) -> Result<String, ManifestError>` — Safely converts a C string pointer to a Rust String.
-  `safe_cstr_to_option_string` function L136-150 — `( ptr: *const c_char, field_name: &str, ) -> Result<Option<String>, ManifestErro...` — Safely converts a C string pointer to an optional Rust String.
-  `validate_ptr` function L157-169 — `( ptr: *const T, field_name: &'static str, ) -> Result<&'a T, ManifestError>` — Validates and dereferences a pointer to a type T.
-  `validate_slice` function L176-199 — `( ptr: *const T, count: usize, field_name: &'static str, ) -> Result<&'a [T], Ma...` — Validates and creates a slice from a pointer and count.
-  `PackageMetadata` struct L255-259 — `{ description: Option<String>, author: Option<String>, workflow_fingerprint: Opt...` — Package metadata extracted from the FFI
-  `extract_task_info_and_graph_from_library` function L262-431 — `( so_path: &Path, project_path: &Path, ) -> Result<( Vec<TaskInfo>, Option<crate...` — Extract task information and graph data from a compiled library using FFI metadata functions
-  `CTaskMetadata` struct L273-280 — `{ index: u32, local_id: *const std::os::raw::c_char, namespaced_id_template: *co...`
-  `CPackageTasks` struct L284-292 — `{ task_count: u32, tasks: *const CTaskMetadata, package_name: *const std::os::ra...`
-  `extract_package_names_from_source` function L434-458 — `(project_path: &Path) -> Result<Vec<String>>` — Extract package names from source files by looking for #[packaged_workflow] attributes
-  `get_current_architecture` function L460-463 — `() -> String`

#### crates/cloacina/src/packaging/manifest_v2.rs

- pub `ManifestValidationError` enum L31-52 — `MissingRuntime | UnsupportedTarget | NoTasks | DuplicateTaskId | InvalidDependen...` — Errors from manifest validation.
- pub `PackageLanguage` enum L57-60 — `Python | Rust` — Package language discriminator.
- pub `PythonRuntime` struct L64-69 — `{ requires_python: String, entry_module: String }` — Python runtime configuration.
- pub `RustRuntime` struct L73-76 — `{ library_path: String }` — Rust runtime configuration.
- pub `PackageInfoV2` struct L80-92 — `{ name: String, version: String, description: Option<String>, fingerprint: Strin...` — Package metadata.
- pub `TaskDefinitionV2` struct L96-116 — `{ id: String, function: String, dependencies: Vec<String>, description: Option<S...` — Task definition within a package.
- pub `ManifestV2` struct L122-142 — `{ format_version: String, package: PackageInfoV2, language: PackageLanguage, pyt...` — Unified package manifest (v2).
- pub `validate` function L146-210 — `(&self) -> Result<(), ManifestValidationError>` — Validate the manifest for structural correctness.
- pub `is_compatible_with_platform` function L213-215 — `(&self, platform_str: &str) -> bool` — Check if this package is compatible with a specific platform.
-  `ManifestV2` type L144-216 — `= ManifestV2` — runtime configuration applies.
-  `tests` module L219-415 — `-` — runtime configuration applies.
-  `make_python_manifest` function L222-259 — `() -> ManifestV2` — runtime configuration applies.
-  `make_rust_manifest` function L261-287 — `() -> ManifestV2` — runtime configuration applies.
-  `test_python_manifest_validates` function L290-292 — `()` — runtime configuration applies.
-  `test_rust_manifest_validates` function L295-297 — `()` — runtime configuration applies.
-  `test_missing_python_runtime` function L300-307 — `()` — runtime configuration applies.
-  `test_missing_rust_runtime` function L310-317 — `()` — runtime configuration applies.
-  `test_unsupported_target` function L320-327 — `()` — runtime configuration applies.
-  `test_no_tasks` function L330-337 — `()` — runtime configuration applies.
-  `test_duplicate_task_id` function L340-347 — `()` — runtime configuration applies.
-  `test_invalid_dependency` function L350-357 — `()` — runtime configuration applies.
-  `test_invalid_python_function_path` function L360-367 — `()` — runtime configuration applies.
-  `test_rust_function_path_no_colon_ok` function L370-373 — `()` — runtime configuration applies.
-  `test_invalid_format_version` function L376-383 — `()` — runtime configuration applies.
-  `test_serialization_roundtrip` function L386-398 — `()` — runtime configuration applies.
-  `test_platform_compatibility` function L401-406 — `()` — runtime configuration applies.
-  `test_language_serde` function L409-414 — `()` — runtime configuration applies.

#### crates/cloacina/src/packaging/mod.rs

- pub `archive` module L23 — `-` — Workflow packaging functionality for creating distributable workflow packages.
- pub `compile` module L24 — `-` — by CLI tools, tests, or other applications that need to package workflows.
- pub `debug` module L25 — `-` — by CLI tools, tests, or other applications that need to package workflows.
- pub `manifest` module L26 — `-` — by CLI tools, tests, or other applications that need to package workflows.
- pub `manifest_v2` module L27 — `-` — by CLI tools, tests, or other applications that need to package workflows.
- pub `platform` module L28 — `-` — by CLI tools, tests, or other applications that need to package workflows.
- pub `types` module L29 — `-` — by CLI tools, tests, or other applications that need to package workflows.
- pub `validation` module L30 — `-` — by CLI tools, tests, or other applications that need to package workflows.
- pub `package_workflow` function L57-72 — `( project_path: PathBuf, output_path: PathBuf, options: CompileOptions, ) -> Res...` — High-level function to package a workflow project.
-  `tests` module L33 — `-` — by CLI tools, tests, or other applications that need to package workflows.

#### crates/cloacina/src/packaging/platform.rs

- pub `SUPPORTED_TARGETS` variable L20-21 — `: &[&str]` — Supported target platforms for workflow packages.
- pub `detect_current_platform` function L24-50 — `() -> &'static str` — Detect the current platform as a target string.
-  `tests` module L53-67 — `-` — Platform detection and target validation for workflow packages.
-  `test_detect_current_platform_is_known` function L57-61 — `()` — Platform detection and target validation for workflow packages.
-  `test_supported_targets_not_empty` function L64-66 — `()` — Platform detection and target validation for workflow packages.

#### crates/cloacina/src/packaging/tests.rs

-  `tests` module L20-530 — `-` — Unit tests for packaging functionality
-  `create_test_cargo_toml` function L26-41 — `() -> types::CargoToml` — Create a minimal test Cargo.toml structure
-  `create_mock_library_file` function L44-52 — `() -> (TempDir, PathBuf)` — Create a mock compiled library file for testing
-  `create_test_project` function L55-80 — `() -> (TempDir, PathBuf)` — Create a test project structure
-  `test_generate_manifest_basic` function L83-112 — `()` — Unit tests for packaging functionality
-  `test_generate_manifest_with_target` function L115-131 — `()` — Unit tests for packaging functionality
-  `test_generate_manifest_missing_package` function L134-146 — `()` — Unit tests for packaging functionality
-  `test_extract_package_names_from_source` function L149-163 — `()` — Unit tests for packaging functionality
-  `test_extract_package_names_no_packages` function L166-191 — `()` — Unit tests for packaging functionality
-  `test_extract_package_names_missing_src` function L194-204 — `()` — Unit tests for packaging functionality
-  `test_get_current_architecture` function L207-220 — `()` — Unit tests for packaging functionality
-  `test_compile_options_builder_pattern` function L223-235 — `()` — Unit tests for packaging functionality
-  `test_package_info_creation` function L238-252 — `()` — Unit tests for packaging functionality
-  `test_library_info_creation` function L255-271 — `()` — Unit tests for packaging functionality
-  `test_task_info_creation` function L274-288 — `()` — Unit tests for packaging functionality
-  `test_package_manifest_serialization_roundtrip` function L291-349 — `()` — Unit tests for packaging functionality
-  `test_constants` function L352-372 — `()` — Unit tests for packaging functionality
-  `test_safe_cstr_to_string_null_pointer` function L377-389 — `()` — Unit tests for packaging functionality
-  `test_safe_cstr_to_string_valid` function L392-400 — `()` — Unit tests for packaging functionality
-  `test_safe_cstr_to_option_string_null_returns_none` function L403-410 — `()` — Unit tests for packaging functionality
-  `test_safe_cstr_to_option_string_valid` function L413-421 — `()` — Unit tests for packaging functionality
-  `test_validate_ptr_null_pointer` function L424-436 — `()` — Unit tests for packaging functionality
-  `test_validate_ptr_valid` function L439-446 — `()` — Unit tests for packaging functionality
-  `test_validate_slice_null_with_nonzero_count` function L449-462 — `()` — Unit tests for packaging functionality
-  `test_validate_slice_null_with_zero_count` function L465-472 — `()` — Unit tests for packaging functionality
-  `test_validate_slice_exceeds_max_tasks` function L475-490 — `()` — Unit tests for packaging functionality
-  `test_validate_slice_valid` function L493-504 — `()` — Unit tests for packaging functionality
-  `test_manifest_error_display` function L507-529 — `()` — Unit tests for packaging functionality

#### crates/cloacina/src/packaging/types.rs

- pub `CompileResult` struct L22-27 — `{ so_path: PathBuf, manifest: PackageManifest }` — Result of compiling a workflow project
- pub `CompileOptions` struct L31-40 — `{ target: Option<String>, profile: String, cargo_flags: Vec<String>, jobs: Optio...` — Options for compiling a workflow
- pub `PackageManifest` struct L55-65 — `{ package: PackageInfo, library: LibraryInfo, tasks: Vec<TaskInfo>, graph: Optio...` — Package manifest containing workflow metadata
- pub `PackageInfo` struct L69-84 — `{ name: String, version: String, description: String, author: Option<String>, wo...` — Package information from Cargo.toml
- pub `LibraryInfo` struct L88-95 — `{ filename: String, symbols: Vec<String>, architecture: String }` — Dynamic library information
- pub `TaskInfo` struct L99-110 — `{ index: u32, id: String, dependencies: Vec<String>, description: String, source...` — Task information extracted from the workflow
- pub `CargoToml` struct L114-118 — `{ package: Option<CargoPackage>, lib: Option<CargoLib>, dependencies: Option<tom...` — Parsed Cargo.toml structure
- pub `CargoPackage` struct L122-130 — `{ name: String, version: String, description: Option<String>, authors: Option<Ve...` — Package section from Cargo.toml
- pub `CargoLib` struct L134-137 — `{ crate_type: Option<Vec<String>> }` — Library section from Cargo.toml
- pub `MANIFEST_FILENAME` variable L140 — `: &str` — Constants
- pub `EXECUTE_TASK_SYMBOL` variable L141 — `: &str`
- pub `CLOACINA_VERSION` variable L142 — `: &str`
-  `CompileOptions` type L42-51 — `impl Default for CompileOptions`
-  `default` function L43-50 — `() -> Self`

#### crates/cloacina/src/packaging/validation.rs

- pub `validate_rust_crate_structure` function L25-44 — `(project_path: &PathBuf) -> Result<()>` — Validate that the project has a valid Rust crate structure
- pub `validate_cargo_toml` function L47-71 — `(project_path: &Path) -> Result<CargoToml>` — Parse and validate Cargo.toml
- pub `validate_cloacina_compatibility` function L74-92 — `(cargo_toml: &CargoToml) -> Result<()>` — Validate cloacina dependency compatibility
- pub `validate_packaged_workflow_presence` function L95-142 — `(project_path: &Path) -> Result<()>` — Check for packaged_workflow macros in the source code
- pub `validate_rust_version_compatibility` function L145-167 — `(cargo_toml: &CargoToml) -> Result<()>` — Validate Rust version compatibility

### crates/cloacina/src/python

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/python/executor.rs

- pub `PythonExecutionError` enum L28-56 — `EnvironmentSetup | TaskNotFound | TaskException | SerializationError | ImportErr...` — Errors that can occur during Python task execution.
- pub `PythonTaskResult` struct L60-65 — `{ task_id: String, output_json: String }` — Result of executing a Python task.
- pub `PythonTaskExecutor` interface L79-108 — `{ fn execute_task(), fn discover_tasks() }` — Trait for executing Python tasks from extracted packages.
-  `tests` module L111-210 — `-` — crate provides the concrete implementation.
-  `MockPythonExecutor` struct L116-118 — `{ task_ids: Vec<String> }` — A mock executor for testing without PyO3.
-  `MockPythonExecutor` type L121-150 — `impl PythonTaskExecutor for MockPythonExecutor` — crate provides the concrete implementation.
-  `execute_task` function L122-140 — `( &self, _workflow_dir: &Path, _vendor_dir: &Path, _entry_module: &str, task_id:...` — crate provides the concrete implementation.
-  `discover_tasks` function L142-149 — `( &self, _workflow_dir: &Path, _vendor_dir: &Path, _entry_module: &str, ) -> Res...` — crate provides the concrete implementation.
-  `test_mock_executor_discover` function L153-162 — `()` — crate provides the concrete implementation.
-  `test_mock_executor_execute` function L165-181 — `()` — crate provides the concrete implementation.
-  `test_mock_executor_task_not_found` function L184-197 — `()` — crate provides the concrete implementation.
-  `test_error_display` function L200-209 — `()` — crate provides the concrete implementation.

#### crates/cloacina/src/python/mod.rs

- pub `executor` module L24 — `-` — Python task execution interface for workflow packages.

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

- pub `EXECUTE_TASK_SYMBOL` variable L37 — `: &str` — Standard symbol name for task execution in cloacina packages
- pub `GET_METADATA_SYMBOL` variable L40 — `: &str` — Standard symbol name for metadata extraction
- pub `get_library_extension` function L43-51 — `() -> &'static str` — Get the platform-specific dynamic library extension
- pub `PackageMetadata` struct L77-94 — `{ package_name: String, version: String, description: Option<String>, author: Op...` — Metadata extracted from a workflow package
- pub `TaskMetadata` struct L98-111 — `{ index: u32, local_id: String, namespaced_id_template: String, dependencies: Ve...` — Individual task metadata
- pub `PackageLoader` struct L114-116 — `{ temp_dir: TempDir }` — Package loader for extracting metadata from workflow library files
- pub `new` function L120-126 — `() -> Result<Self, LoaderError>` — Create a new package loader with a temporary directory for safe operations
- pub `extract_metadata` function L177-203 — `( &self, package_data: &[u8], ) -> Result<PackageMetadata, LoaderError>` — Extract metadata from a binary package
- pub `temp_dir` function L558-560 — `(&self) -> &Path` — Get the temporary directory path for manual file operations
- pub `validate_package_symbols` function L563-606 — `( &self, package_data: &[u8], ) -> Result<Vec<String>, LoaderError>` — Validate that a package has the required symbols
-  `CTaskMetadata` struct L56-63 — `{ index: u32, local_id: *const c_char, namespaced_id_template: *const c_char, de...` — C-compatible structure for task metadata extraction via FFI
-  `CPackageTasks` struct L68-73 — `{ task_count: u32, tasks: *const CTaskMetadata, package_name: *const c_char, gra...` — C-compatible structure for package metadata extraction via FFI
-  `PackageLoader` type L118-607 — `= PackageLoader` — interface patterns.
-  `generate_graph_data_from_tasks` function L129-165 — `( &self, tasks: &[TaskMetadata], ) -> Result<serde_json::Value, LoaderError>` — Generate graph data from task dependencies
-  `is_cloacina_archive` function L206-212 — `(&self, package_data: &[u8]) -> bool` — Check if package data is a .cloacina archive
-  `extract_library_from_archive` function L224-305 — `( &self, archive_data: &[u8], ) -> Result<std::path::PathBuf, LoaderError>` — Extract the library file from a .cloacina archive.
-  `extract_metadata_from_so` function L308-357 — `( &self, library_path: &Path, ) -> Result<PackageMetadata, LoaderError>` — Extract metadata from a library file using established cloacina patterns
-  `convert_c_metadata_to_rust` function L360-471 — `( &self, c_package: &CPackageTasks, fallback_name: &str, ) -> Result<PackageMeta...` — Convert C FFI metadata structures to Rust types
-  `convert_c_task_to_rust` function L474-555 — `(&self, c_task: &CTaskMetadata) -> Result<TaskMetadata, LoaderError>` — Convert a single C task structure to Rust
-  `PackageLoader` type L609-613 — `impl Default for PackageLoader` — interface patterns.
-  `default` function L610-612 — `() -> Self` — interface patterns.
-  `tests` module L616-887 — `-` — interface patterns.
-  `create_mock_elf_data` function L620-645 — `(size: usize) -> Vec<u8>` — Helper to create a mock ELF-like binary for testing
-  `create_invalid_binary_data` function L648-650 — `() -> Vec<u8>` — Helper to create invalid binary data
-  `test_package_loader_creation` function L653-659 — `()` — interface patterns.
-  `test_package_loader_default` function L662-665 — `()` — interface patterns.
-  `test_extract_metadata_with_invalid_elf` function L668-683 — `()` — interface patterns.
-  `test_extract_metadata_with_empty_data` function L686-699 — `()` — interface patterns.
-  `test_extract_metadata_with_large_invalid_data` function L702-715 — `()` — interface patterns.
-  `test_validate_package_symbols_with_invalid_data` function L718-731 — `()` — interface patterns.
-  `test_validate_package_symbols_with_empty_data` function L734-741 — `()` — interface patterns.
-  `test_temp_dir_isolation` function L744-754 — `()` — interface patterns.
-  `test_concurrent_package_loading` function L757-785 — `()` — interface patterns.
-  `test_symbol_constants` function L788-791 — `()` — interface patterns.
-  `test_file_system_operations` function L794-808 — `()` — interface patterns.
-  `test_error_types_and_messages` function L811-831 — `()` — interface patterns.
-  `test_package_loader_memory_safety` function L834-845 — `()` — interface patterns.
-  `test_temp_directory_cleanup` function L848-862 — `()` — interface patterns.
-  `test_package_loader_sync_creation` function L865-872 — `()` — interface patterns.
-  `test_get_library_extension` function L875-886 — `()` — interface patterns.

#### crates/cloacina/src/registry/loader/python_loader.rs

- pub `ExtractedPythonPackage` struct L34-45 — `{ root_dir: PathBuf, vendor_dir: PathBuf, workflow_dir: PathBuf, entry_module: S...` — An extracted Python package ready for task execution.
- pub `PackageKind` enum L48-53 — `Python | Rust` — Result of peeking at a manifest inside an archive.
- pub `peek_manifest` function L56-92 — `(archive_data: &[u8]) -> Result<ManifestV2, LoaderError>` — Peek at the manifest inside a `.cloacina` archive without full extraction.
- pub `detect_package_kind` function L95-101 — `(archive_data: &[u8]) -> Result<PackageKind, LoaderError>` — Determine the package kind (Python or Rust) from archive data.
- pub `extract_python_package` function L108-169 — `( archive_data: &[u8], staging_dir: &Path, ) -> Result<ExtractedPythonPackage, L...` — Extract a Python workflow package from a `.cloacina` archive.
-  `tests` module L172-336 — `-` — execution via PyO3.
-  `build_test_archive` function L183-222 — `(manifest: &ManifestV2, include_workflow: bool) -> Vec<u8>` — Build a minimal Python `.cloacina` archive in memory.
-  `make_test_manifest` function L224-251 — `() -> ManifestV2` — execution via PyO3.
-  `test_peek_manifest` function L254-261 — `()` — execution via PyO3.
-  `test_detect_package_kind_python` function L264-270 — `()` — execution via PyO3.
-  `test_extract_python_package` function L273-284 — `()` — execution via PyO3.
-  `test_extract_missing_workflow_dir` function L287-294 — `()` — execution via PyO3.
-  `test_peek_manifest_missing` function L297-315 — `()` — execution via PyO3.
-  `test_wrong_language` function L318-335 — `()` — execution via PyO3.

### crates/cloacina/src/registry/loader/task_registrar

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/registry/loader/task_registrar/dynamic_task.rs

-  `DynamicLibraryTask` struct L33-42 — `{ library_data: Vec<u8>, task_name: String, package_name: String, dependencies: ...` — A task implementation that executes via dynamic library FFI calls.
-  `DynamicLibraryTask` type L44-59 — `= DynamicLibraryTask` — Dynamic library task implementation for FFI-based task execution.
-  `new` function L46-58 — `( library_data: Vec<u8>, task_name: String, package_name: String, dependencies: ...` — Create a new dynamic library task.
-  `DynamicLibraryTask` type L62-275 — `impl Task for DynamicLibraryTask` — Dynamic library task implementation for FFI-based task execution.
-  `execute` function L67-264 — `( &self, context: Context<serde_json::Value>, ) -> Result<Context<serde_json::Va...` — Execute the task using the cloacina_execute_task FFI function.
-  `id` function L267-269 — `(&self) -> &str` — Get the unique identifier for this task.
-  `dependencies` function L272-274 — `(&self) -> &[TaskNamespace]` — Get the list of task dependencies.
-  `tests` module L278-293 — `-` — Dynamic library task implementation for FFI-based task execution.
-  `test_dynamic_library_task_creation` function L282-292 — `()` — Dynamic library task implementation for FFI-based task execution.

#### crates/cloacina/src/registry/loader/task_registrar/extraction.rs

-  `TaskRegistrar` type L28-146 — `= TaskRegistrar` — Task metadata extraction from dynamic libraries.
-  `extract_task_metadata_from_library` function L34-145 — `( &self, package_data: &[u8], ) -> Result<OwnedTaskMetadataCollection, LoaderErr...` — Extract task metadata from library using get_task_metadata() FFI function.

#### crates/cloacina/src/registry/loader/task_registrar/mod.rs

- pub `TaskRegistrar` struct L49-56 — `{ temp_dir: TempDir, registered_tasks: Arc<RwLock<HashMap<String, Vec<TaskNamesp...` — Task registrar for managing dynamically loaded package tasks.
- pub `new` function L60-70 — `() -> Result<Self, LoaderError>` — Create a new task registrar with a temporary directory for operations.
- pub `register_package_tasks` function L85-183 — `( &self, package_id: &str, package_data: &[u8], _metadata: &PackageMetadata, ten...` — Register package tasks with the global task registry using new host-managed approach.
- pub `unregister_package_tasks` function L195-220 — `(&self, package_id: &str) -> Result<(), LoaderError>` — Unregister package tasks from the global registry.
- pub `get_registered_namespaces` function L223-226 — `(&self, package_id: &str) -> Vec<TaskNamespace>` — Get the list of task namespaces registered for a package.
- pub `loaded_package_count` function L229-232 — `(&self) -> usize` — Get the number of currently loaded packages.
- pub `total_registered_tasks` function L235-238 — `(&self) -> usize` — Get the total number of registered tasks across all packages.
- pub `temp_dir` function L241-243 — `(&self) -> &Path` — Get the temporary directory path for manual operations.
-  `dynamic_task` module L23 — `-` — Task registrar for integrating packaged workflow tasks with the global registry.
-  `extraction` module L24 — `-` — isolation and task lifecycle management.
-  `types` module L25 — `-` — isolation and task lifecycle management.
-  `TaskRegistrar` type L58-244 — `= TaskRegistrar` — isolation and task lifecycle management.
-  `TaskRegistrar` type L246-250 — `impl Default for TaskRegistrar` — isolation and task lifecycle management.
-  `default` function L247-249 — `() -> Self` — isolation and task lifecycle management.
-  `tests` module L253-551 — `-` — isolation and task lifecycle management.
-  `create_mock_package_metadata` function L258-286 — `(package_name: &str, task_count: usize) -> PackageMetadata` — Helper to create mock package metadata for testing
-  `create_mock_binary_data` function L289-292 — `() -> Vec<u8>` — Helper to create mock binary data (not a real .so file)
-  `test_task_registrar_creation` function L295-302 — `()` — isolation and task lifecycle management.
-  `test_task_registrar_default` function L305-309 — `()` — isolation and task lifecycle management.
-  `test_register_package_tasks_with_invalid_binary` function L312-329 — `()` — isolation and task lifecycle management.
-  `test_register_package_tasks_with_missing_symbols` function L332-352 — `()` — isolation and task lifecycle management.
-  `test_register_package_tasks_empty_metadata` function L355-366 — `()` — isolation and task lifecycle management.
-  `test_unregister_nonexistent_package` function L369-376 — `()` — isolation and task lifecycle management.
-  `test_get_registered_namespaces_empty` function L379-385 — `()` — isolation and task lifecycle management.
-  `test_registrar_metrics` function L388-404 — `()` — isolation and task lifecycle management.
-  `test_concurrent_registrar_operations` function L407-447 — `()` — isolation and task lifecycle management.
-  `test_temp_directory_isolation` function L450-458 — `()` — isolation and task lifecycle management.
-  `test_package_id_tracking` function L461-472 — `()` — isolation and task lifecycle management.
-  `test_tenant_isolation` function L475-491 — `()` — isolation and task lifecycle management.
-  `test_default_tenant` function L494-505 — `()` — isolation and task lifecycle management.
-  `test_large_package_metadata` function L508-521 — `()` — isolation and task lifecycle management.
-  `test_error_message_quality` function L524-540 — `()` — isolation and task lifecycle management.
-  `test_registrar_sync_creation` function L543-550 — `()` — isolation and task lifecycle management.

#### crates/cloacina/src/registry/loader/task_registrar/types.rs

- pub `TaskMetadata` struct L22-33 — `{ local_id: *const std::os::raw::c_char, namespaced_id_template: *const std::os:...` — C-compatible task metadata structure for FFI (from packaged_workflow macro)
- pub `TaskMetadataCollection` struct L38-47 — `{ task_count: u32, tasks: *const TaskMetadata, workflow_name: *const std::os::ra...` — C-compatible collection of task metadata for FFI (from packaged_workflow macro)
- pub `OwnedTaskMetadata` struct L54-61 — `{ local_id: String, dependencies_json: String, constructor_fn_name: String }` — Owned version of task metadata - safe to use after library is unloaded.
- pub `OwnedTaskMetadataCollection` struct L68-75 — `{ workflow_name: String, package_name: String, tasks: Vec<OwnedTaskMetadata> }` — Owned version of task metadata collection - safe to use after library is unloaded.

### crates/cloacina/src/registry/loader/validator

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/registry/loader/validator/format.rs

-  `PackageValidator` type L26-90 — `= PackageValidator` — File format validation for dynamic libraries.
-  `validate_file_format` function L28-89 — `( &self, package_path: &Path, result: &mut ValidationResult, )` — Validate file format and basic structure.

#### crates/cloacina/src/registry/loader/validator/metadata.rs

-  `PackageValidator` type L26-93 — `= PackageValidator` — Package metadata validation.
-  `validate_metadata` function L28-92 — `( &self, metadata: &PackageMetadata, result: &mut ValidationResult, )` — Validate package metadata for consistency and safety.

#### crates/cloacina/src/registry/loader/validator/mod.rs

- pub `PackageValidator` struct L43-52 — `{ temp_dir: TempDir, strict_mode: bool, max_package_size: u64, required_symbols:...` — Comprehensive package validator
- pub `new` function L56-71 — `() -> Result<Self, LoaderError>` — Create a new package validator with default settings.
- pub `strict` function L74-78 — `() -> Result<Self, LoaderError>` — Create a validator with strict validation mode enabled.
- pub `with_max_size` function L81-84 — `(mut self, max_bytes: u64) -> Self` — Set the maximum allowed package size.
- pub `with_required_symbols` function L87-96 — `(mut self, symbols: I) -> Self` — Add additional required symbols for validation.
- pub `validate_package` function L109-163 — `( &self, package_data: &[u8], metadata: Option<&PackageMetadata>, ) -> Result<Va...` — Validate a package comprehensively.
- pub `temp_dir` function L166-168 — `(&self) -> &Path` — Get the temporary directory path.
- pub `is_strict_mode` function L171-173 — `(&self) -> bool` — Check if strict mode is enabled.
- pub `max_package_size` function L176-178 — `(&self) -> u64` — Get the maximum package size limit.
-  `format` module L23 — `-` — Package validator for ensuring workflow package safety and compatibility.
-  `metadata` module L24 — `-` — metadata verification, and compatibility testing.
-  `security` module L25 — `-` — metadata verification, and compatibility testing.
-  `size` module L26 — `-` — metadata verification, and compatibility testing.
-  `symbols` module L27 — `-` — metadata verification, and compatibility testing.
-  `types` module L28 — `-` — metadata verification, and compatibility testing.
-  `PackageValidator` type L54-179 — `= PackageValidator` — metadata verification, and compatibility testing.
-  `PackageValidator` type L181-186 — `impl Default for PackageValidator` — metadata verification, and compatibility testing.
-  `default` function L182-185 — `() -> Self` — metadata verification, and compatibility testing.
-  `tests` module L189-661 — `-` — metadata verification, and compatibility testing.
-  `create_valid_elf_header` function L194-222 — `() -> Vec<u8>` — Helper to create a valid ELF header for testing
-  `create_invalid_binary` function L225-227 — `() -> Vec<u8>` — Helper to create invalid binary data
-  `create_suspicious_binary` function L230-238 — `() -> Vec<u8>` — Helper to create binary with suspicious content
-  `create_mock_metadata` function L241-269 — `(package_name: &str, task_count: usize) -> PackageMetadata` — Helper to create mock package metadata
-  `test_validator_creation` function L272-278 — `()` — metadata verification, and compatibility testing.
-  `test_validator_default` function L281-285 — `()` — metadata verification, and compatibility testing.
-  `test_strict_validator` function L288-291 — `()` — metadata verification, and compatibility testing.
-  `test_validator_with_custom_max_size` function L294-298 — `()` — metadata verification, and compatibility testing.
-  `test_validator_with_required_symbols` function L301-308 — `()` — metadata verification, and compatibility testing.
-  `test_validate_empty_package` function L311-320 — `()` — metadata verification, and compatibility testing.
-  `test_validate_oversized_package` function L323-332 — `()` — metadata verification, and compatibility testing.
-  `test_validate_invalid_elf` function L335-349 — `()` — metadata verification, and compatibility testing.
-  `test_validate_valid_elf_header` function L352-365 — `()` — metadata verification, and compatibility testing.
-  `test_validate_suspicious_content` function L368-383 — `()` — metadata verification, and compatibility testing.
-  `test_validate_with_metadata` function L386-406 — `()` — metadata verification, and compatibility testing.
-  `test_validate_metadata_with_invalid_package_name` function L409-425 — `()` — metadata verification, and compatibility testing.
-  `test_validate_metadata_with_special_characters` function L428-443 — `()` — metadata verification, and compatibility testing.
-  `test_validate_metadata_with_duplicate_task_ids` function L446-464 — `()` — metadata verification, and compatibility testing.
-  `test_validate_metadata_with_no_tasks` function L467-482 — `()` — metadata verification, and compatibility testing.
-  `test_strict_mode_validation` function L485-497 — `()` — metadata verification, and compatibility testing.
-  `test_permissive_mode_with_warnings` function L500-512 — `()` — metadata verification, and compatibility testing.
-  `test_security_assessment_levels` function L515-533 — `()` — metadata verification, and compatibility testing.
-  `test_compatibility_info` function L536-550 — `()` — metadata verification, and compatibility testing.
-  `test_concurrent_validation` function L553-580 — `()` — metadata verification, and compatibility testing.
-  `test_memory_safety_with_large_packages` function L583-598 — `()` — metadata verification, and compatibility testing.
-  `test_temp_directory_isolation` function L601-609 — `()` — metadata verification, and compatibility testing.
-  `test_validation_result_serialization` function L612-622 — `()` — metadata verification, and compatibility testing.
-  `test_error_message_quality` function L625-642 — `()` — metadata verification, and compatibility testing.
-  `test_security_level_equality` function L645-650 — `()` — metadata verification, and compatibility testing.
-  `test_validator_sync_creation` function L653-660 — `()` — metadata verification, and compatibility testing.

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

-  `RegistryReconciler` type L24-144 — `= RegistryReconciler` — Package format detection and library extraction from .cloacina archives.
-  `is_cloacina_package` function L26-32 — `(&self, package_data: &[u8]) -> bool` — Check if package data is a .cloacina archive
-  `extract_library_from_cloacina` function L35-143 — `( &self, package_data: &[u8], ) -> Result<Vec<u8>, RegistryError>` — Extract library file data from a .cloacina archive

#### crates/cloacina/src/registry/reconciler/loading.rs

-  `RegistryReconciler` type L27-469 — `= RegistryReconciler` — Package loading, unloading, and task/workflow registration.
-  `load_package` function L29-94 — `( &self, metadata: WorkflowMetadata, ) -> Result<(), RegistryError>` — Load a package into the global registries
-  `unload_package` function L97-129 — `( &self, package_id: WorkflowPackageId, ) -> Result<(), RegistryError>` — Unload a package from the global registries
-  `register_package_tasks` function L132-173 — `( &self, metadata: &WorkflowMetadata, package_data: &[u8], ) -> Result<Vec<TaskN...` — Register tasks from a package into the global task registry
-  `register_package_workflows` function L176-327 — `( &self, metadata: &WorkflowMetadata, package_data: &[u8], ) -> Result<Option<St...` — Register workflows from a package into the global workflow registry
-  `create_workflow_from_host_registry` function L330-378 — `( &self, package_name: &str, workflow_name: &str, tenant_id: &str, ) -> Result<c...` — Create a workflow using the host's global task registry (avoiding FFI isolation)
-  `create_workflow_from_host_registry_static` function L381-428 — `( package_name: &str, workflow_name: &str, tenant_id: &str, ) -> Result<crate::w...` — Static version of create_workflow_from_host_registry for use in closures
-  `unregister_package_tasks` function L431-454 — `( &self, package_id: WorkflowPackageId, task_namespaces: &[TaskNamespace], ) -> ...` — Unregister tasks from the global task registry
-  `unregister_package_workflow` function L457-468 — `( &self, workflow_name: &str, ) -> Result<(), RegistryError>` — Unregister a workflow from the global workflow registry

#### crates/cloacina/src/registry/reconciler/mod.rs

- pub `ReconcilerConfig` struct L53-68 — `{ reconcile_interval: Duration, enable_startup_reconciliation: bool, package_ope...` — Configuration for the Registry Reconciler
- pub `ReconcileResult` struct L84-99 — `{ packages_loaded: Vec<WorkflowPackageId>, packages_unloaded: Vec<WorkflowPackag...` — Result of a reconciliation operation
- pub `has_changes` function L103-105 — `(&self) -> bool` — Check if the reconciliation had any changes
- pub `has_failures` function L108-110 — `(&self) -> bool` — Check if the reconciliation had any failures
- pub `ReconcilerStatus` struct L128-134 — `{ packages_loaded: usize, package_details: Vec<PackageStatusDetail> }` — Status information about the reconciler
- pub `PackageStatusDetail` struct L138-150 — `{ package_name: String, version: String, task_count: usize, has_workflow: bool }` — Detailed status information about a loaded package
- pub `RegistryReconciler` struct L153-174 — `{ registry: Arc<dyn WorkflowRegistry>, config: ReconcilerConfig, loaded_packages...` — Registry Reconciler for synchronizing database state with in-memory registries
- pub `new` function L178-198 — `( registry: Arc<dyn WorkflowRegistry>, config: ReconcilerConfig, shutdown_rx: wa...` — Create a new Registry Reconciler
- pub `start_reconciliation_loop` function L201-274 — `(mut self) -> Result<(), RegistryError>` — Start the background reconciliation loop
- pub `reconcile` function L277-374 — `(&self) -> Result<ReconcileResult, RegistryError>` — Perform a single reconciliation operation
- pub `get_status` function L400-415 — `(&self) -> ReconcilerStatus` — Get the current reconciliation status
-  `extraction` module L34 — `-` — # Registry Reconciler
-  `loading` module L35 — `-` — - `PackageState`: Tracking loaded package state
-  `ReconcilerConfig` type L70-80 — `impl Default for ReconcilerConfig` — - `PackageState`: Tracking loaded package state
-  `default` function L71-79 — `() -> Self` — - `PackageState`: Tracking loaded package state
-  `ReconcileResult` type L101-111 — `= ReconcileResult` — - `PackageState`: Tracking loaded package state
-  `PackageState` struct L115-124 — `{ metadata: WorkflowMetadata, task_namespaces: Vec<TaskNamespace>, workflow_name...` — Tracks the state of loaded packages
-  `RegistryReconciler` type L176-416 — `= RegistryReconciler` — - `PackageState`: Tracking loaded package state
-  `shutdown_cleanup` function L377-397 — `(&self) -> Result<(), RegistryError>` — Perform cleanup operations during shutdown
-  `tests` module L419-485 — `-` — - `PackageState`: Tracking loaded package state
-  `test_reconciler_config_default` function L425-432 — `()` — - `PackageState`: Tracking loaded package state
-  `test_reconcile_result_methods` function L435-457 — `()` — - `PackageState`: Tracking loaded package state
-  `test_reconciler_status` function L460-484 — `()` — - `PackageState`: Tracking loaded package state

### crates/cloacina/src/registry/workflow_registry

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/registry/workflow_registry/database.rs

-  `store_package_metadata` function L29-56 — `( &self, registry_id: &str, package_metadata: &crate::registry::loader::package_...` — Store package metadata in the database.
-  `store_package_metadata_postgres` function L59-113 — `( &self, registry_uuid: Uuid, package_metadata: &crate::registry::loader::packag...` — Database operations for workflow registry metadata storage.
-  `store_package_metadata_sqlite` function L116-168 — `( &self, registry_uuid: Uuid, package_metadata: &crate::registry::loader::packag...` — Database operations for workflow registry metadata storage.
-  `get_package_metadata` function L171-189 — `( &self, package_name: &str, version: &str, ) -> Result< Option<( String, crate:...` — Retrieve package metadata from the database.
-  `get_package_metadata_postgres` function L192-234 — `( &self, package_name: &str, version: &str, ) -> Result< Option<( String, crate:...` — Database operations for workflow registry metadata storage.
-  `get_package_metadata_sqlite` function L237-279 — `( &self, package_name: &str, version: &str, ) -> Result< Option<( String, crate:...` — Database operations for workflow registry metadata storage.
-  `list_all_packages` function L282-288 — `(&self) -> Result<Vec<WorkflowMetadata>, RegistryError>` — List all packages in the registry.
-  `list_all_packages_postgres` function L291-331 — `(&self) -> Result<Vec<WorkflowMetadata>, RegistryError>` — Database operations for workflow registry metadata storage.
-  `list_all_packages_sqlite` function L334-374 — `(&self) -> Result<Vec<WorkflowMetadata>, RegistryError>` — Database operations for workflow registry metadata storage.
-  `delete_package_metadata` function L377-389 — `( &self, package_name: &str, version: &str, ) -> Result<(), RegistryError>` — Delete package metadata from the database.
-  `delete_package_metadata_postgres` function L392-421 — `( &self, package_name: &str, version: &str, ) -> Result<(), RegistryError>` — Database operations for workflow registry metadata storage.
-  `delete_package_metadata_sqlite` function L424-453 — `( &self, package_name: &str, version: &str, ) -> Result<(), RegistryError>` — Database operations for workflow registry metadata storage.
-  `get_package_metadata_by_id` function L456-465 — `( &self, package_id: Uuid, ) -> Result<Option<(String, WorkflowMetadata)>, Regis...` — Get package metadata by ID.
-  `get_package_metadata_by_id_postgres` function L468-519 — `( &self, package_id: Uuid, ) -> Result<Option<(String, WorkflowMetadata)>, Regis...` — Database operations for workflow registry metadata storage.
-  `get_package_metadata_by_id_sqlite` function L522-574 — `( &self, package_id: Uuid, ) -> Result<Option<(String, WorkflowMetadata)>, Regis...` — Database operations for workflow registry metadata storage.
-  `delete_package_metadata_by_id` function L577-587 — `( &self, package_id: Uuid, ) -> Result<(), RegistryError>` — Delete package metadata by ID.
-  `delete_package_metadata_by_id_postgres` function L590-613 — `( &self, package_id: Uuid, ) -> Result<(), RegistryError>` — Database operations for workflow registry metadata storage.
-  `delete_package_metadata_by_id_sqlite` function L616-640 — `( &self, package_id: Uuid, ) -> Result<(), RegistryError>` — Database operations for workflow registry metadata storage.

#### crates/cloacina/src/registry/workflow_registry/mod.rs

- pub `WorkflowRegistryImpl` struct L42-55 — `{ storage: S, database: Database, loader: PackageLoader, registrar: TaskRegistra...` — Complete implementation of the workflow registry.
- pub `new` function L69-82 — `(storage: S, database: Database) -> Result<Self, RegistryError>` — Create a new workflow registry implementation.
- pub `with_strict_validation` function L85-98 — `(storage: S, database: Database) -> Result<Self, RegistryError>` — Create a registry with strict validation enabled.
- pub `loaded_package_count` function L101-103 — `(&self) -> usize` — Get the number of currently loaded packages.
- pub `total_registered_tasks` function L106-108 — `(&self) -> usize` — Get the total number of registered tasks across all packages.
- pub `register_workflow_package` function L118-124 — `( &mut self, package_data: Vec<u8>, ) -> Result<Uuid, RegistryError>` — Register a workflow package (alias for register_workflow via the trait).
- pub `get_workflow_package_by_id` function L129-150 — `( &self, package_id: Uuid, ) -> Result<Option<(WorkflowMetadata, Vec<u8>)>, Regi...` — Get a workflow package by its UUID.
- pub `get_workflow_package_by_name` function L155-165 — `( &self, package_name: &str, version: &str, ) -> Result<Option<(WorkflowMetadata...` — Get a workflow package by name and version.
- pub `exists_by_id` function L168-170 — `(&self, package_id: Uuid) -> Result<bool, RegistryError>` — Check if a package exists by ID.
- pub `exists_by_name` function L173-182 — `( &self, package_name: &str, version: &str, ) -> Result<bool, RegistryError>` — Check if a package exists by name and version.
- pub `list_packages` function L187-189 — `(&self) -> Result<Vec<WorkflowMetadata>, RegistryError>` — List all packages in the registry.
- pub `unregister_workflow_package_by_id` function L192-216 — `( &mut self, package_id: Uuid, ) -> Result<(), RegistryError>` — Unregister a workflow package by ID.
- pub `unregister_workflow_package_by_name` function L219-235 — `( &mut self, package_name: &str, version: &str, ) -> Result<(), RegistryError>` — Unregister a workflow package by name and version.
-  `database` module L23 — `-` — Complete implementation of the workflow registry.
-  `package` module L24 — `-` — cohesive system for managing packaged workflows.
-  `register_workflow` function L240-321 — `( &mut self, package_data: Vec<u8>, ) -> Result<WorkflowPackageId, RegistryError...` — cohesive system for managing packaged workflows.
-  `get_workflow` function L323-367 — `( &self, package_name: &str, version: &str, ) -> Result<Option<LoadedWorkflow>, ...` — cohesive system for managing packaged workflows.
-  `list_workflows` function L369-371 — `(&self) -> Result<Vec<WorkflowMetadata>, RegistryError>` — cohesive system for managing packaged workflows.
-  `unregister_workflow` function L373-404 — `( &mut self, package_name: &str, version: &str, ) -> Result<(), RegistryError>` — cohesive system for managing packaged workflows.
-  `tests` module L408-431 — `-` — cohesive system for managing packaged workflows.
-  `test_registry_creation` function L413-420 — `()` — cohesive system for managing packaged workflows.
-  `test_registry_metrics` function L423-430 — `()` — cohesive system for managing packaged workflows.

#### crates/cloacina/src/registry/workflow_registry/package.rs

-  `is_cloacina_package` function L29-32 — `(data: &[u8]) -> bool` — Check if package data is a .cloacina archive (tar.gz format)
-  `extract_so_from_cloacina` function L35-76 — `( package_data: &[u8], ) -> Result<Vec<u8>, RegistryError>` — Extract .so file from .cloacina package archive

### crates/cloacina/src/runner/default_runner

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/runner/default_runner/config.rs

- pub `DefaultRunnerConfig` struct L59-87 — `{ max_concurrent_tasks: usize, scheduler_poll_interval: Duration, task_timeout: ...` — Configuration for the default runner
- pub `builder` function L91-93 — `() -> DefaultRunnerConfigBuilder` — Creates a new configuration builder with default values.
- pub `max_concurrent_tasks` function L96-98 — `(&self) -> usize` — Maximum number of concurrent task executions allowed.
- pub `scheduler_poll_interval` function L101-103 — `(&self) -> Duration` — How often the scheduler checks for ready tasks.
- pub `task_timeout` function L106-108 — `(&self) -> Duration` — Maximum time allowed for a single task to execute.
- pub `pipeline_timeout` function L111-113 — `(&self) -> Option<Duration>` — Optional maximum time for an entire pipeline execution.
- pub `db_pool_size` function L116-118 — `(&self) -> u32` — Number of database connections in the pool.
- pub `enable_recovery` function L121-123 — `(&self) -> bool` — Whether automatic recovery is enabled.
- pub `enable_cron_scheduling` function L126-128 — `(&self) -> bool` — Whether cron scheduling is enabled.
- pub `cron_poll_interval` function L131-133 — `(&self) -> Duration` — Poll interval for cron schedules.
- pub `cron_max_catchup_executions` function L136-138 — `(&self) -> usize` — Maximum catchup executions for missed cron runs.
- pub `cron_enable_recovery` function L141-143 — `(&self) -> bool` — Whether cron recovery is enabled.
- pub `cron_recovery_interval` function L146-148 — `(&self) -> Duration` — How often to check for lost cron executions.
- pub `cron_lost_threshold_minutes` function L151-153 — `(&self) -> i32` — Minutes before an execution is considered lost.
- pub `cron_max_recovery_age` function L156-158 — `(&self) -> Duration` — Maximum age of executions to recover.
- pub `cron_max_recovery_attempts` function L161-163 — `(&self) -> usize` — Maximum recovery attempts per execution.
- pub `enable_trigger_scheduling` function L166-168 — `(&self) -> bool` — Whether trigger scheduling is enabled.
- pub `trigger_base_poll_interval` function L171-173 — `(&self) -> Duration` — Base poll interval for trigger readiness checks.
- pub `trigger_poll_timeout` function L176-178 — `(&self) -> Duration` — Timeout for trigger poll operations.
- pub `enable_registry_reconciler` function L181-183 — `(&self) -> bool` — Whether the registry reconciler is enabled.
- pub `registry_reconcile_interval` function L186-188 — `(&self) -> Duration` — How often to run registry reconciliation.
- pub `registry_enable_startup_reconciliation` function L191-193 — `(&self) -> bool` — Whether startup reconciliation is enabled.
- pub `registry_storage_path` function L196-198 — `(&self) -> Option<&std::path::Path>` — Path for registry storage (filesystem backend).
- pub `registry_storage_backend` function L201-203 — `(&self) -> &str` — Registry storage backend type.
- pub `runner_id` function L206-208 — `(&self) -> Option<&str>` — Optional runner identifier for logging.
- pub `runner_name` function L211-213 — `(&self) -> Option<&str>` — Optional runner name for logging.
- pub `routing_config` function L216-218 — `(&self) -> Option<&RoutingConfig>` — Routing configuration for task dispatch.
- pub `enable_continuous_scheduling` function L221-223 — `(&self) -> bool` — Whether continuous reactive scheduling is enabled.
- pub `continuous_poll_interval` function L226-228 — `(&self) -> Duration` — Poll interval for the continuous scheduler's ledger observation.
- pub `DefaultRunnerConfigBuilder` struct L242-244 — `{ config: DefaultRunnerConfig }` — Builder for [`DefaultRunnerConfig`].
- pub `max_concurrent_tasks` function L284-287 — `(mut self, value: usize) -> Self` — Sets the maximum number of concurrent task executions.
- pub `scheduler_poll_interval` function L290-293 — `(mut self, value: Duration) -> Self` — Sets the scheduler poll interval.
- pub `task_timeout` function L296-299 — `(mut self, value: Duration) -> Self` — Sets the task timeout.
- pub `pipeline_timeout` function L302-305 — `(mut self, value: Option<Duration>) -> Self` — Sets the pipeline timeout.
- pub `db_pool_size` function L308-311 — `(mut self, value: u32) -> Self` — Sets the database pool size.
- pub `enable_recovery` function L314-317 — `(mut self, value: bool) -> Self` — Enables or disables automatic recovery.
- pub `enable_cron_scheduling` function L320-323 — `(mut self, value: bool) -> Self` — Enables or disables cron scheduling.
- pub `cron_poll_interval` function L326-329 — `(mut self, value: Duration) -> Self` — Sets the cron poll interval.
- pub `cron_max_catchup_executions` function L332-335 — `(mut self, value: usize) -> Self` — Sets the maximum catchup executions for cron.
- pub `cron_enable_recovery` function L338-341 — `(mut self, value: bool) -> Self` — Enables or disables cron recovery.
- pub `cron_recovery_interval` function L344-347 — `(mut self, value: Duration) -> Self` — Sets the cron recovery interval.
- pub `cron_lost_threshold_minutes` function L350-353 — `(mut self, value: i32) -> Self` — Sets the cron lost threshold in minutes.
- pub `cron_max_recovery_age` function L356-359 — `(mut self, value: Duration) -> Self` — Sets the maximum cron recovery age.
- pub `cron_max_recovery_attempts` function L362-365 — `(mut self, value: usize) -> Self` — Sets the maximum cron recovery attempts.
- pub `enable_trigger_scheduling` function L368-371 — `(mut self, value: bool) -> Self` — Enables or disables trigger scheduling.
- pub `trigger_base_poll_interval` function L374-377 — `(mut self, value: Duration) -> Self` — Sets the trigger base poll interval.
- pub `trigger_poll_timeout` function L380-383 — `(mut self, value: Duration) -> Self` — Sets the trigger poll timeout.
- pub `enable_registry_reconciler` function L386-389 — `(mut self, value: bool) -> Self` — Enables or disables the registry reconciler.
- pub `registry_reconcile_interval` function L392-395 — `(mut self, value: Duration) -> Self` — Sets the registry reconcile interval.
- pub `registry_enable_startup_reconciliation` function L398-401 — `(mut self, value: bool) -> Self` — Enables or disables startup reconciliation.
- pub `registry_storage_path` function L404-407 — `(mut self, value: Option<std::path::PathBuf>) -> Self` — Sets the registry storage path.
- pub `registry_storage_backend` function L410-413 — `(mut self, value: impl Into<String>) -> Self` — Sets the registry storage backend.
- pub `runner_id` function L416-419 — `(mut self, value: Option<String>) -> Self` — Sets the runner identifier.
- pub `runner_name` function L422-425 — `(mut self, value: Option<String>) -> Self` — Sets the runner name.
- pub `routing_config` function L428-431 — `(mut self, value: Option<RoutingConfig>) -> Self` — Sets the routing configuration.
- pub `enable_continuous_scheduling` function L434-437 — `(mut self, value: bool) -> Self` — Enables or disables continuous reactive scheduling.
- pub `continuous_poll_interval` function L440-443 — `(mut self, value: Duration) -> Self` — Sets the continuous scheduler poll interval.
- pub `build` function L446-448 — `(self) -> DefaultRunnerConfig` — Builds the configuration.
- pub `DefaultRunnerBuilder` struct L483-487 — `{ database_url: Option<String>, schema: Option<String>, config: DefaultRunnerCon...` — Builder for creating a DefaultRunner with PostgreSQL schema-based multi-tenancy
- pub `new` function L497-503 — `() -> Self` — Creates a new builder with default configuration
- pub `database_url` function L506-509 — `(mut self, url: &str) -> Self` — Sets the database URL
- pub `schema` function L515-518 — `(mut self, schema: &str) -> Self` — Sets the PostgreSQL schema for multi-tenant isolation
- pub `with_config` function L521-524 — `(mut self, config: DefaultRunnerConfig) -> Self` — Sets the full configuration
- pub `build` function L538-658 — `(self) -> Result<DefaultRunner, PipelineError>` — Builds the DefaultRunner
- pub `routing_config` function L676-679 — `(mut self, config: RoutingConfig) -> Self` — Sets custom routing configuration for task dispatch.
-  `DefaultRunnerConfig` type L89-229 — `= DefaultRunnerConfig` — configuring the DefaultRunner's behavior.
-  `DefaultRunnerConfigBuilder` type L246-280 — `impl Default for DefaultRunnerConfigBuilder` — configuring the DefaultRunner's behavior.
-  `default` function L247-279 — `() -> Self` — configuring the DefaultRunner's behavior.
-  `DefaultRunnerConfigBuilder` type L282-449 — `= DefaultRunnerConfigBuilder` — configuring the DefaultRunner's behavior.
-  `DefaultRunnerConfig` type L451-455 — `impl Default for DefaultRunnerConfig` — configuring the DefaultRunner's behavior.
-  `default` function L452-454 — `() -> Self` — configuring the DefaultRunner's behavior.
-  `DefaultRunnerBuilder` type L489-493 — `impl Default for DefaultRunnerBuilder` — configuring the DefaultRunner's behavior.
-  `default` function L490-492 — `() -> Self` — configuring the DefaultRunner's behavior.
-  `DefaultRunnerBuilder` type L495-680 — `= DefaultRunnerBuilder` — configuring the DefaultRunner's behavior.
-  `validate_schema_name` function L527-535 — `(schema: &str) -> Result<(), PipelineError>` — Validates the schema name contains only alphanumeric characters and underscores
-  `tests` module L683-849 — `-` — configuring the DefaultRunner's behavior.
-  `test_default_runner_config` function L687-702 — `()` — configuring the DefaultRunner's behavior.
-  `test_registry_storage_backend_configuration` function L705-728 — `()` — configuring the DefaultRunner's behavior.
-  `test_runner_identification` function L731-739 — `()` — configuring the DefaultRunner's behavior.
-  `test_registry_configuration_options` function L742-763 — `()` — configuring the DefaultRunner's behavior.
-  `test_cron_configuration` function L766-781 — `()` — configuring the DefaultRunner's behavior.
-  `test_db_pool_size_default` function L784-787 — `()` — configuring the DefaultRunner's behavior.
-  `test_config_clone` function L790-803 — `()` — configuring the DefaultRunner's behavior.
-  `test_config_debug` function L806-814 — `()` — configuring the DefaultRunner's behavior.
-  `test_builder_all_fields` function L817-848 — `()` — configuring the DefaultRunner's behavior.

#### crates/cloacina/src/runner/default_runner/cron_api.rs

- pub `register_cron_workflow` function L51-113 — `( &self, workflow_name: &str, cron_expression: &str, timezone: &str, ) -> Result...` — Register a workflow to run on a cron schedule
- pub `list_cron_schedules` function L124-143 — `( &self, enabled_only: bool, limit: i64, offset: i64, ) -> Result<Vec<crate::mod...` — List all registered cron schedules
- pub `set_cron_schedule_enabled` function L153-174 — `( &self, schedule_id: UniversalUuid, enabled: bool, ) -> Result<(), PipelineErro...` — Enable or disable a cron schedule
- pub `delete_cron_schedule` function L183-200 — `( &self, schedule_id: UniversalUuid, ) -> Result<(), PipelineError>` — Delete a cron schedule
- pub `get_cron_schedule` function L209-226 — `( &self, schedule_id: UniversalUuid, ) -> Result<crate::models::cron_schedule::C...` — Get a specific cron schedule by ID
- pub `update_cron_schedule` function L237-301 — `( &self, schedule_id: UniversalUuid, cron_expression: Option<&str>, timezone: Op...` — Update a cron schedule's expression and/or timezone
- pub `get_cron_execution_history` function L312-331 — `( &self, schedule_id: UniversalUuid, limit: i64, offset: i64, ) -> Result<Vec<cr...` — Get execution history for a cron schedule
- pub `get_cron_execution_stats` function L340-357 — `( &self, since: chrono::DateTime<chrono::Utc>, ) -> Result<crate::dal::CronExecu...` — Get cron execution statistics
- pub `get_workflow_registry` function L364-367 — `(&self) -> Option<Arc<dyn WorkflowRegistry>>` — Get access to the workflow registry (if enabled)
- pub `get_registry_reconciler_status` function L374-383 — `( &self, ) -> Option<crate::registry::ReconcilerStatus>` — Get the current status of the registry reconciler (if enabled)
- pub `is_registry_reconciler_enabled` function L386-388 — `(&self) -> bool` — Check if the registry reconciler is enabled in the configuration
-  `DefaultRunner` type L30-389 — `= DefaultRunner` — This module provides methods for managing cron-scheduled workflow executions.

#### crates/cloacina/src/runner/default_runner/mod.rs

- pub `DefaultRunner` struct L69-95 — `{ database: Database, config: DefaultRunnerConfig, scheduler: Arc<TaskScheduler>...` — Default runner that coordinates workflow scheduling and task execution
- pub `new` function L135-137 — `(database_url: &str) -> Result<Self, PipelineError>` — Creates a new default runner with default configuration
- pub `builder` function L151-153 — `() -> DefaultRunnerBuilder` — Creates a builder for configuring the executor
- pub `with_schema` function L171-177 — `(database_url: &str, schema: &str) -> Result<Self, PipelineError>` — Creates a new executor with PostgreSQL schema-based multi-tenancy
- pub `with_config` function L194-266 — `( database_url: &str, config: DefaultRunnerConfig, ) -> Result<Self, PipelineErr...` — Creates a new unified executor with custom configuration
- pub `database` function L269-271 — `(&self) -> &Database` — Returns a reference to the database.
- pub `dal` function L274-276 — `(&self) -> DAL` — Returns the DAL for database operations.
- pub `trigger_scheduler` function L281-283 — `(&self) -> Option<Arc<crate::TriggerScheduler>>` — Returns the trigger scheduler if enabled.
- pub `register_data_source` function L289-291 — `(&self, source: crate::continuous::datasource::DataSource)` — Register a data source for continuous scheduling.
- pub `register_continuous_task` function L296-304 — `( &self, registration: crate::continuous::graph::ContinuousTaskRegistration, )` — Register a continuous task declaration for graph assembly.
- pub `register_continuous_task_impl` function L309-311 — `(&self, task: Arc<dyn cloacina_workflow::Task>)` — Register a continuous task implementation.
- pub `shutdown` function L323-375 — `(&self) -> Result<(), PipelineError>` — Gracefully shuts down the executor and its background services
-  `config` module L29 — `-` — Default runner for workflow execution.
-  `cron_api` module L30 — `-` — - `DefaultRunnerBuilder`: Builder for creating runners with custom settings
-  `pipeline_executor_impl` module L31 — `-` — - `DefaultRunnerBuilder`: Builder for creating runners with custom settings
-  `pipeline_result` module L32 — `-` — - `DefaultRunnerBuilder`: Builder for creating runners with custom settings
-  `services` module L33 — `-` — - `DefaultRunnerBuilder`: Builder for creating runners with custom settings
-  `RuntimeHandles` struct L101-120 — `{ scheduler_handle: Option<tokio::task::JoinHandle<()>>, executor_handle: Option...` — Internal structure for managing runtime handles of background services
-  `DefaultRunner` type L122-376 — `= DefaultRunner` — - `DefaultRunnerBuilder`: Builder for creating runners with custom settings
-  `DefaultRunner` type L378-395 — `impl Clone for DefaultRunner` — - `DefaultRunnerBuilder`: Builder for creating runners with custom settings
-  `clone` function L379-394 — `(&self) -> Self` — - `DefaultRunnerBuilder`: Builder for creating runners with custom settings
-  `DefaultRunner` type L398-404 — `impl Drop for DefaultRunner` — - `DefaultRunnerBuilder`: Builder for creating runners with custom settings
-  `drop` function L399-403 — `(&mut self)` — - `DefaultRunnerBuilder`: Builder for creating runners with custom settings

#### crates/cloacina/src/runner/default_runner/pipeline_executor_impl.rs

-  `DefaultRunner` type L43-368 — `impl PipelineExecutor for DefaultRunner` — Implementation of PipelineExecutor trait for DefaultRunner
-  `execute` function L54-100 — `( &self, workflow_name: &str, context: Context<serde_json::Value>, ) -> Result<P...` — Executes a workflow synchronously and waits for completion
-  `execute_async` function L113-132 — `( &self, workflow_name: &str, context: Context<serde_json::Value>, ) -> Result<P...` — Executes a workflow asynchronously
-  `execute_with_callback` function L146-174 — `( &self, workflow_name: &str, context: Context<serde_json::Value>, callback: Box...` — Executes a workflow with status callbacks
-  `get_execution_status` function L183-207 — `( &self, execution_id: Uuid, ) -> Result<PipelineStatus, PipelineError>` — Gets the current status of a pipeline execution
-  `get_execution_result` function L216-221 — `( &self, execution_id: Uuid, ) -> Result<PipelineResult, PipelineError>` — Gets the complete result of a pipeline execution
-  `cancel_execution` function L230-243 — `(&self, execution_id: Uuid) -> Result<(), PipelineError>` — Cancels an in-progress pipeline execution
-  `pause_execution` function L256-291 — `( &self, execution_id: Uuid, reason: Option<&str>, ) -> Result<(), PipelineError...` — Pauses a running pipeline execution
-  `resume_execution` function L303-332 — `(&self, execution_id: Uuid) -> Result<(), PipelineError>` — Resumes a paused pipeline execution
-  `list_executions` function L340-359 — `(&self) -> Result<Vec<PipelineResult>, PipelineError>` — Lists recent pipeline executions
-  `shutdown` function L365-367 — `(&self) -> Result<(), PipelineError>` — Shuts down the executor

#### crates/cloacina/src/runner/default_runner/pipeline_result.rs

-  `DefaultRunner` type L35-177 — `= DefaultRunner` — from database records.
-  `build_pipeline_result` function L50-176 — `( &self, execution_id: Uuid, ) -> Result<PipelineResult, PipelineError>` — Builds a pipeline result from an execution ID

#### crates/cloacina/src/runner/default_runner/services.rs

-  `DefaultRunner` type L38-497 — `= DefaultRunner` — the scheduler, executor, cron scheduler, cron recovery, and registry reconciler.
-  `create_runner_span` function L40-58 — `(&self, operation: &str) -> tracing::Span` — Creates a tracing span for this runner instance with proper context
-  `start_background_services` function L70-136 — `(&self) -> Result<(), PipelineError>` — Starts the background scheduler and executor services
-  `start_cron_services` function L139-199 — `( &self, handles: &mut super::RuntimeHandles, shutdown_tx: &broadcast::Sender<()...` — Starts cron scheduler and recovery services
-  `start_cron_recovery` function L202-259 — `( &self, handles: &mut super::RuntimeHandles, shutdown_tx: &broadcast::Sender<()...` — Starts the cron recovery service
-  `start_registry_reconciler` function L262-356 — `( &self, handles: &mut super::RuntimeHandles, shutdown_tx: &broadcast::Sender<()...` — Starts the registry reconciler service
-  `start_trigger_services` function L359-413 — `( &self, handles: &mut super::RuntimeHandles, shutdown_tx: &broadcast::Sender<()...` — Starts the trigger scheduler service
-  `start_continuous_scheduler` function L416-496 — `( &self, handles: &mut super::RuntimeHandles, shutdown_tx: &broadcast::Sender<()...` — Starts the continuous reactive scheduler.

### crates/cloacina/src/runner

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/runner/mod.rs

- pub `default_runner` module L23 — `-` — Workflow runners for executing complete pipelines and workflows.

### crates/cloacina/src/security

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/security/api_keys.rs

- pub `generate_api_key` function L30-46 — `(env: &str, tenant_name: &str) -> (String, String, String)` — Generate a new API key.
- pub `extract_prefix` function L49-57 — `(full_key: &str) -> String` — Extract the prefix from a full PAK key for cache lookup.
- pub `hash_key` function L60-67 — `(key: &str) -> String` — Hash a key using argon2.
- pub `verify_key` function L70-78 — `(key: &str, hash: &str) -> bool` — Verify a key against a stored hash.
-  `tests` module L81-122 — `-` — PAK (Prefixed API Key) generation and verification.
-  `test_generate_api_key_format` function L85-90 — `()` — PAK (Prefixed API Key) generation and verification.
-  `test_verify_key_correct` function L93-96 — `()` — PAK (Prefixed API Key) generation and verification.
-  `test_verify_key_wrong` function L99-102 — `()` — PAK (Prefixed API Key) generation and verification.
-  `test_extract_prefix` function L105-108 — `()` — PAK (Prefixed API Key) generation and verification.
-  `test_extract_prefix_global` function L111-114 — `()` — PAK (Prefixed API Key) generation and verification.
-  `test_unique_keys` function L117-121 — `()` — PAK (Prefixed API Key) generation and verification.

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
-  `tests` module L258-371 — `-` — Events are logged using the `tracing` crate at appropriate levels.
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

#### crates/cloacina/src/security/db_key_manager.rs

- pub `DbKeyManager` struct L59-61 — `{ dal: DAL }` — Database-backed implementation of the [`KeyManager`] trait.
- pub `new` function L65-67 — `(dal: DAL) -> Self` — Creates a new database-backed key manager.
- pub `encode_public_key_pem` function L70-79 — `(public_key: &[u8]) -> String` — Encodes a raw Ed25519 public key to PEM format.
- pub `decode_public_key_pem` function L82-112 — `(pem_str: &str) -> Result<Vec<u8>, KeyError>` — Decodes a PEM-encoded Ed25519 public key to raw bytes.
-  `ED25519_PEM_TAG` variable L39 — `: &str` — PEM tag for Ed25519 public keys.
-  `ED25519_DER_PREFIX` variable L43-50 — `: [u8; 12]` — ASN.1 DER prefix for Ed25519 public keys (SubjectPublicKeyInfo).
-  `DbKeyManager` type L63-139 — `= DbKeyManager` — AES-256-GCM.
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
-  `tests` module L1175-1200 — `-` — AES-256-GCM.
-  `test_pem_roundtrip` function L1179-1188 — `()` — AES-256-GCM.
-  `test_invalid_pem` function L1191-1199 — `()` — AES-256-GCM.

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
-  `DbPackageSigner` type L280-510 — `impl PackageSigner for DbPackageSigner` — - [`DetachedSignature`] format for standalone signature files
-  `sign_package_with_db_key` function L281-330 — `( &self, package_path: &Path, key_id: UniversalUuid, master_key: &[u8], store_si...` — - [`DetachedSignature`] format for standalone signature files
-  `sign_package_with_raw_key` function L332-340 — `( &self, package_path: &Path, private_key: &[u8], public_key: &[u8], ) -> Result...` — - [`DetachedSignature`] format for standalone signature files
-  `sign_package_data` function L342-367 — `( &self, package_data: &[u8], private_key: &[u8], public_key: &[u8], ) -> Result...` — - [`DetachedSignature`] format for standalone signature files
-  `store_signature` function L369-404 — `( &self, signature: &PackageSignatureInfo, ) -> Result<UniversalUuid, PackageSig...` — - [`DetachedSignature`] format for standalone signature files
-  `find_signature` function L406-415 — `( &self, package_hash: &str, ) -> Result<Option<PackageSignatureInfo>, PackageSi...` — - [`DetachedSignature`] format for standalone signature files
-  `find_signatures` function L417-426 — `( &self, package_hash: &str, ) -> Result<Vec<PackageSignatureInfo>, PackageSignE...` — - [`DetachedSignature`] format for standalone signature files
-  `verify_package` function L428-466 — `( &self, package_path: &Path, org_id: UniversalUuid, ) -> Result<PackageSignatur...` — - [`DetachedSignature`] format for standalone signature files
-  `verify_package_with_detached_signature` function L468-509 — `( &self, package_path: &Path, signature: &DetachedSignature, public_key: &[u8], ...` — - [`DetachedSignature`] format for standalone signature files
-  `DbPackageSigner` type L514-590 — `= DbPackageSigner` — - [`DetachedSignature`] format for standalone signature files
-  `store_signature_postgres` function L515-536 — `( &self, new_sig: NewUnifiedPackageSignature, ) -> Result<(), PackageSignError>` — - [`DetachedSignature`] format for standalone signature files
-  `find_signature_postgres` function L538-563 — `( &self, package_hash: &str, ) -> Result<Option<PackageSignatureInfo>, PackageSi...` — - [`DetachedSignature`] format for standalone signature files
-  `find_signatures_postgres` function L565-589 — `( &self, package_hash: &str, ) -> Result<Vec<PackageSignatureInfo>, PackageSignE...` — - [`DetachedSignature`] format for standalone signature files
-  `DbPackageSigner` type L594-670 — `= DbPackageSigner` — - [`DetachedSignature`] format for standalone signature files
-  `store_signature_sqlite` function L595-616 — `( &self, new_sig: NewUnifiedPackageSignature, ) -> Result<(), PackageSignError>` — - [`DetachedSignature`] format for standalone signature files
-  `find_signature_sqlite` function L618-643 — `( &self, package_hash: &str, ) -> Result<Option<PackageSignatureInfo>, PackageSi...` — - [`DetachedSignature`] format for standalone signature files
-  `find_signatures_sqlite` function L645-669 — `( &self, package_hash: &str, ) -> Result<Vec<PackageSignatureInfo>, PackageSignE...` — - [`DetachedSignature`] format for standalone signature files
-  `tests` module L673-742 — `-` — - [`DetachedSignature`] format for standalone signature files
-  `test_sign_and_verify_with_raw_key` function L679-699 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_detached_signature_roundtrip` function L702-721 — `()` — - [`DetachedSignature`] format for standalone signature files
-  `test_detached_signature_file_io` function L724-741 — `()` — - [`DetachedSignature`] format for standalone signature files

#### crates/cloacina/src/security/verification.rs

- pub `SecurityConfig` struct L36-50 — `{ require_signatures: bool, key_encryption_key: Option<[u8; 32]> }` — Security configuration for package verification.
- pub `require_signatures` function L63-68 — `() -> Self` — Create a security config that requires signatures.
- pub `development` function L71-73 — `() -> Self` — Create a security config with no signature requirements (for development).
- pub `with_encryption_key` function L76-79 — `(mut self, key: [u8; 32]) -> Self` — Set the key encryption key for signing operations.
- pub `VerificationError` enum L86-139 — `TamperedPackage | UntrustedSigner | InvalidSignature | SignatureNotFound | Malfo...` — Errors that occur during package verification.
- pub `SignatureSource` enum L143-155 — `Database | DetachedFile | Auto` — Where to find the signature for a package.
- pub `VerificationResult` struct L165-172 — `{ package_hash: String, signer_fingerprint: String, signer_name: Option<String> ...` — Result of successful verification.
- pub `verify_package` function L193-305 — `( package_path: P, org_id: UniversalUuid, signature_source: SignatureSource, pac...` — Verify a package signature.
- pub `verify_package_offline` function L320-379 — `( package_path: P, signature_path: S, public_key: &[u8], ) -> Result<Verificatio...` — Verify a package using only a detached signature and public key (offline mode).
-  `SecurityConfig` type L52-59 — `impl Default for SecurityConfig` — - [`verify_and_load_package`] for verified package loading
-  `default` function L53-58 — `() -> Self` — - [`verify_and_load_package`] for verified package loading
-  `SecurityConfig` type L61-80 — `= SecurityConfig` — - [`verify_and_load_package`] for verified package loading
-  `SignatureSource` type L157-161 — `impl Default for SignatureSource` — - [`verify_and_load_package`] for verified package loading
-  `default` function L158-160 — `() -> Self` — - [`verify_and_load_package`] for verified package loading
-  `compute_package_hash` function L382-388 — `(data: &[u8]) -> Result<String, VerificationError>` — Compute SHA256 hash of package data.
-  `load_signature_from_db` function L391-406 — `( package_hash: &str, package_signer: &DbPackageSigner, ) -> Result<DetachedSign...` — Load signature from database.
-  `load_signature_from_file` function L409-413 — `(path: &Path) -> Result<DetachedSignature, VerificationError>` — Load signature from file.
-  `tests` module L416-479 — `-` — - [`verify_and_load_package`] for verified package loading
-  `test_security_config_default` function L423-427 — `()` — - [`verify_and_load_package`] for verified package loading
-  `test_security_config_require_signatures` function L430-433 — `()` — - [`verify_and_load_package`] for verified package loading
-  `test_security_config_with_encryption_key` function L436-440 — `()` — - [`verify_and_load_package`] for verified package loading
-  `test_verify_package_offline_with_invalid_signature` function L443-472 — `()` — - [`verify_and_load_package`] for verified package loading
-  `test_signature_source_default` function L475-478 — `()` — - [`verify_and_load_package`] for verified package loading

### crates/cloacina/src/task_scheduler

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/task_scheduler/context_manager.rs

- pub `ContextManager` struct L32-34 — `{ dal: &'a DAL }` — Context management operations for the scheduler.
- pub `new` function L38-40 — `(dal: &'a DAL) -> Self` — Creates a new ContextManager.
- pub `load_context_for_task` function L43-144 — `( &self, task_execution: &TaskExecution, ) -> Result<Context<serde_json::Value>,...` — Loads the context for a specific task based on its dependencies.
- pub `evaluate_context_condition` function L201-240 — `( context: &Context<serde_json::Value>, key: &str, operator: &ValueOperator, exp...` — Evaluates a context-based condition using the provided operator.
-  `merge_dependency_contexts` function L147-198 — `( &self, task_execution: &TaskExecution, dependencies: &[crate::task::TaskNamesp...` — Merges contexts from multiple dependencies.

#### crates/cloacina/src/task_scheduler/mod.rs

- pub `TaskScheduler` struct L185-191 — `{ dal: DAL, instance_id: Uuid, poll_interval: Duration, dispatcher: Option<Arc<d...` — The main Task Scheduler that manages workflow execution and task readiness.
- pub `new` function L221-224 — `(database: Database) -> Result<Self, ValidationError>` — Creates a new TaskScheduler instance with default configuration using global workflow registry.
- pub `with_poll_interval` function L242-250 — `( database: Database, poll_interval: Duration, ) -> Result<Self, ValidationError...` — Creates a new TaskScheduler with custom poll interval using global workflow registry.
- pub `with_dispatcher` function L276-279 — `(mut self, dispatcher: Arc<dyn Dispatcher>) -> Self` — Sets the dispatcher for push-based task execution.
- pub `dispatcher` function L282-284 — `(&self) -> Option<&Arc<dyn Dispatcher>>` — Returns a reference to the dispatcher if configured.
- pub `schedule_workflow_execution` function L329-419 — `( &self, workflow_name: &str, input_context: Context<serde_json::Value>, ) -> Re...` — Schedules a new workflow execution with the provided input context.
- pub `run_scheduling_loop` function L581-589 — `(&self) -> Result<(), ValidationError>` — Runs the main scheduling loop that continuously processes active pipeline executions.
- pub `process_active_pipelines` function L592-600 — `(&self) -> Result<(), ValidationError>` — Processes all active pipeline executions to update task readiness.
-  `context_manager` module L116 — `-` — # Task Scheduler
-  `recovery` module L117 — `-` — ```
-  `scheduler_loop` module L118 — `-` — ```
-  `state_manager` module L119 — `-` — ```
-  `trigger_rules` module L120 — `-` — ```
-  `TaskScheduler` type L193-623 — `= TaskScheduler` — ```
-  `with_poll_interval_sync` function L253-262 — `(database: Database, poll_interval: Duration) -> Self` — Creates a new TaskScheduler with custom poll interval (synchronous version).
-  `create_pipeline_postgres` function L423-480 — `( &self, pipeline_id: UniversalUuid, now: UniversalTimestamp, pipeline_name: Str...` — Creates pipeline and tasks in PostgreSQL.
-  `create_pipeline_sqlite` function L484-541 — `( &self, pipeline_id: UniversalUuid, now: UniversalTimestamp, pipeline_name: Str...` — Creates pipeline and tasks in SQLite.
-  `get_task_trigger_rules` function L603-612 — `( &self, workflow: &Workflow, task_namespace: &TaskNamespace, ) -> serde_json::V...` — Gets trigger rules for a specific task from the task implementation.
-  `get_task_configuration` function L615-622 — `( &self, _workflow: &Workflow, _task_namespace: &TaskNamespace, ) -> serde_json:...` — Gets task configuration (currently returns empty object).

#### crates/cloacina/src/task_scheduler/recovery.rs

- pub `RecoveryResult` enum L32-37 — `Recovered | Abandoned` — Result of attempting to recover a task.
- pub `RecoveryManager` struct L43-45 — `{ dal: &'a DAL }` — Recovery operations for the scheduler.
- pub `new` function L49-51 — `(dal: &'a DAL) -> Self` — Creates a new RecoveryManager.
- pub `recover_orphaned_tasks` function L63-174 — `(&self) -> Result<(), ValidationError>` — Detects and recovers tasks orphaned by system interruptions.
-  `MAX_RECOVERY_ATTEMPTS` variable L40 — `: i32` — Maximum number of recovery attempts before abandoning a task.
-  `recover_tasks_for_known_workflow` function L177-204 — `( &self, tasks: Vec<TaskExecution>, ) -> Result<usize, ValidationError>` — Recovers tasks from workflows that are still available in the registry.
-  `abandon_tasks_for_unknown_workflow` function L207-287 — `( &self, pipeline: PipelineExecution, tasks: Vec<TaskExecution>, available_workf...` — Abandons tasks from workflows that are no longer available in the registry.
-  `recover_single_task` function L290-330 — `( &self, task: TaskExecution, ) -> Result<RecoveryResult, ValidationError>` — Recovers a single orphaned task with retry limit enforcement.
-  `abandon_task_permanently` function L333-379 — `(&self, task: TaskExecution) -> Result<(), ValidationError>` — Permanently abandons a task that has exceeded recovery limits.
-  `record_recovery_event` function L382-385 — `(&self, event: NewRecoveryEvent) -> Result<(), ValidationError>` — Records a recovery event for monitoring and debugging.

#### crates/cloacina/src/task_scheduler/scheduler_loop.rs

- pub `SchedulerLoop` struct L40-46 — `{ dal: &'a DAL, instance_id: Uuid, poll_interval: Duration, dispatcher: Option<A...` — Scheduler loop operations.
- pub `new` function L50-57 — `(dal: &'a DAL, instance_id: Uuid, poll_interval: Duration) -> Self` — Creates a new SchedulerLoop.
- pub `with_dispatcher` function L60-72 — `( dal: &'a DAL, instance_id: Uuid, poll_interval: Duration, dispatcher: Option<A...` — Creates a new SchedulerLoop with an optional dispatcher.
- pub `run` function L81-96 — `(&self) -> Result<(), ValidationError>` — Runs the main scheduling loop that continuously processes active pipeline executions.
- pub `process_active_pipelines` function L99-123 — `(&self) -> Result<(), ValidationError>` — Processes all active pipeline executions to update task readiness.
-  `process_pipelines_batch` function L131-182 — `( &self, active_executions: Vec<PipelineExecution>, ) -> Result<(), ValidationEr...` — Processes multiple pipelines in batch for better performance.
-  `dispatch_ready_tasks` function L189-217 — `(&self) -> Result<(), ValidationError>` — Dispatches all Ready tasks to the executor.
-  `complete_pipeline` function L220-255 — `( &self, execution: &PipelineExecution, ) -> Result<(), ValidationError>` — Completes a pipeline by updating its final context and marking it as completed.
-  `update_pipeline_final_context` function L262-319 — `( &self, pipeline_execution_id: UniversalUuid, all_tasks: &[TaskExecution], ) ->...` — Updates the pipeline's final context when it completes.

#### crates/cloacina/src/task_scheduler/state_manager.rs

- pub `StateManager` struct L34-36 — `{ dal: &'a DAL }` — State management operations for the scheduler.
- pub `new` function L40-42 — `(dal: &'a DAL) -> Self` — Creates a new StateManager.
- pub `update_pipeline_task_readiness` function L49-82 — `( &self, pipeline_execution_id: UniversalUuid, pending_tasks: &[TaskExecution], ...` — Updates task readiness for a specific pipeline using pre-loaded tasks.
- pub `check_task_dependencies` function L87-145 — `( &self, task_execution: &TaskExecution, ) -> Result<bool, ValidationError>` — Checks if all dependencies for a task are satisfied.
- pub `evaluate_trigger_rules` function L148-242 — `( &self, task_execution: &TaskExecution, ) -> Result<bool, ValidationError>` — Evaluates trigger rules for a task based on its configuration.
-  `evaluate_condition` function L245-321 — `( &self, condition: &TriggerCondition, task_execution: &TaskExecution, ) -> Resu...` — Evaluates a specific trigger condition.

#### crates/cloacina/src/task_scheduler/trigger_rules.rs

- pub `TriggerRule` enum L86-95 — `Always | All | Any | None` — Trigger rule definitions for conditional task execution.
- pub `TriggerCondition` enum L143-156 — `TaskSuccess | TaskFailed | TaskSkipped | ContextValue` — Individual conditions that can be evaluated for trigger rules.
- pub `ValueOperator` enum L199-216 — `Equals | NotEquals | GreaterThan | LessThan | Contains | NotContains | Exists | ...` — Operators for evaluating context values in trigger conditions.

### crates/cloacina/src/trigger

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/trigger/mod.rs

- pub `registry` module L51 — `-` — # Trigger System
- pub `TriggerError` enum L65-89 — `PollError | ContextError | TriggerNotFound | Database | ConnectionPool | Workflo...` — Errors that can occur during trigger operations.
- pub `TriggerResult` enum L102-111 — `Skip | Fire` — Result of a trigger poll operation.
- pub `should_fire` function L115-117 — `(&self) -> bool` — Returns true if this result indicates the workflow should fire.
- pub `into_context` function L120-125 — `(self) -> Option<Context<serde_json::Value>>` — Extracts the context if this is a Fire result.
- pub `context_hash` function L131-144 — `(&self) -> String` — Computes a hash of the context for deduplication purposes.
- pub `TriggerConfig` struct L152-167 — `{ name: String, workflow_name: String, poll_interval: Duration, allow_concurrent...` — Configuration for a trigger.
- pub `new` function L171-179 — `(name: &str, workflow_name: &str, poll_interval: Duration) -> Self` — Creates a new trigger configuration.
- pub `with_allow_concurrent` function L182-185 — `(mut self, allow: bool) -> Self` — Sets whether concurrent executions are allowed.
- pub `with_enabled` function L188-191 — `(mut self, enabled: bool) -> Self` — Sets whether the trigger is enabled.
- pub `Trigger` interface L253-274 — `{ fn name(), fn poll_interval(), fn allow_concurrent(), fn poll() }` — Core trait for user-defined triggers.
-  `TriggerError` type L91-95 — `= TriggerError` — ```
-  `from` function L92-94 — `(err: deadpool::managed::PoolError<deadpool_diesel::Error>) -> Self` — ```
-  `TriggerResult` type L113-145 — `= TriggerResult` — ```
-  `TriggerConfig` type L169-192 — `= TriggerConfig` — ```
-  `tests` module L282-397 — `-` — ```
-  `TestTrigger` struct L286-289 — `{ name: String, should_fire: bool }` — ```
-  `TestTrigger` type L292-312 — `impl Trigger for TestTrigger` — ```
-  `name` function L293-295 — `(&self) -> &str` — ```
-  `poll_interval` function L297-299 — `(&self) -> Duration` — ```
-  `allow_concurrent` function L301-303 — `(&self) -> bool` — ```
-  `poll` function L305-311 — `(&self) -> Result<TriggerResult, TriggerError>` — ```
-  `test_trigger_result_should_fire` function L315-319 — `()` — ```
-  `test_trigger_result_into_context` function L322-329 — `()` — ```
-  `test_trigger_result_context_hash` function L332-356 — `()` — ```
-  `test_trigger_config` function L359-370 — `()` — ```
-  `test_trigger_trait` function L373-385 — `()` — ```
-  `test_trigger_fires` function L388-396 — `()` — ```

#### crates/cloacina/src/trigger/registry.rs

- pub `register_trigger_constructor` function L59-67 — `(name: impl Into<String>, constructor: F)` — Register a trigger constructor function globally.
- pub `register_trigger` function L76-79 — `(trigger: T)` — Register a trigger instance directly.
- pub `get_trigger` function L91-94 — `(name: &str) -> Option<Arc<dyn Trigger>>` — Get a trigger instance from the global registry by name.
- pub `global_trigger_registry` function L100-102 — `() -> GlobalTriggerRegistry` — Get the global trigger registry.
- pub `list_triggers` function L109-112 — `() -> Vec<String>` — Get all registered trigger names.
- pub `get_all_triggers` function L119-122 — `() -> Vec<Arc<dyn Trigger>>` — Get all registered triggers.
- pub `is_trigger_registered` function L133-136 — `(name: &str) -> bool` — Check if a trigger is registered.
- pub `clear_triggers` function L142-145 — `()` — Clear all registered triggers.
-  `TriggerConstructor` type L30 — `= Box<dyn Fn() -> Arc<dyn Trigger> + Send + Sync>` — Type alias for the trigger constructor function stored in the global registry
-  `GlobalTriggerRegistry` type L33 — `= Arc<RwLock<HashMap<String, TriggerConstructor>>>` — Type alias for the global trigger registry
-  `GLOBAL_TRIGGER_REGISTRY` variable L36-37 — `: Lazy<GlobalTriggerRegistry>` — Global registry for automatically registering triggers created with the `#[trigger]` macro
-  `tests` module L148-275 — `-` — Triggers registered here are available for use by the TriggerScheduler.
-  `TestTrigger` struct L156-158 — `{ name: String }` — Triggers registered here are available for use by the TriggerScheduler.
-  `TestTrigger` type L160-166 — `= TestTrigger` — Triggers registered here are available for use by the TriggerScheduler.
-  `new` function L161-165 — `(name: &str) -> Self` — Triggers registered here are available for use by the TriggerScheduler.
-  `TestTrigger` type L169-185 — `impl Trigger for TestTrigger` — Triggers registered here are available for use by the TriggerScheduler.
-  `name` function L170-172 — `(&self) -> &str` — Triggers registered here are available for use by the TriggerScheduler.
-  `poll_interval` function L174-176 — `(&self) -> Duration` — Triggers registered here are available for use by the TriggerScheduler.
-  `allow_concurrent` function L178-180 — `(&self) -> bool` — Triggers registered here are available for use by the TriggerScheduler.
-  `poll` function L182-184 — `(&self) -> Result<TriggerResult, TriggerError>` — Triggers registered here are available for use by the TriggerScheduler.
-  `test_register_and_get_trigger` function L192-204 — `()` — Triggers registered here are available for use by the TriggerScheduler.
-  `test_register_constructor` function L208-215 — `()` — Triggers registered here are available for use by the TriggerScheduler.
-  `test_list_triggers` function L219-231 — `()` — Triggers registered here are available for use by the TriggerScheduler.
-  `test_get_all_triggers` function L235-248 — `()` — Triggers registered here are available for use by the TriggerScheduler.
-  `test_clear_triggers` function L252-274 — `()` — Triggers registered here are available for use by the TriggerScheduler.

### crates/cloacina/src/workflow

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/src/workflow/builder.rs

- pub `WorkflowBuilder` struct L75-77 — `{ workflow: Workflow }` — Builder pattern for convenient and fluent Workflow construction.
- pub `new` function L81-85 — `(name: &str) -> Self` — Create a new workflow builder
- pub `name` function L88-90 — `(&self) -> &str` — Get the workflow name
- pub `description` function L93-96 — `(mut self, description: &str) -> Self` — Set the workflow description
- pub `tenant` function L99-102 — `(mut self, tenant: &str) -> Self` — Set the workflow tenant
- pub `tag` function L105-108 — `(mut self, key: &str, value: &str) -> Self` — Add a tag to the workflow metadata
- pub `add_task` function L111-114 — `(mut self, task: Arc<dyn Task>) -> Result<Self, WorkflowError>` — Add a task to the workflow
- pub `validate` function L117-120 — `(self) -> Result<Self, ValidationError>` — Validate the workflow structure
- pub `build` function L123-127 — `(self) -> Result<Workflow, ValidationError>` — Build the final workflow with automatic version calculation
-  `WorkflowBuilder` type L79-128 — `= WorkflowBuilder` — workflows using a chainable, fluent API.

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
-  `tests` module L1009-1377 — `-` — - `get_all_workflows`: Get all registered workflows
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

- pub `get_or_init_postgres_fixture` function L81-103 — `() -> Arc<Mutex<TestFixture>>` — Gets or initializes the PostgreSQL test fixture singleton
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
- pub `create_backend_storage` function L315-317 — `(&self) -> Box<dyn cloacina::registry::traits::RegistryStorage>` — Create storage backend matching the current database backend
- pub `create_unified_storage` function L320-322 — `(&self) -> cloacina::dal::UnifiedRegistryStorage` — Create a unified storage backend using this fixture's database
- pub `create_filesystem_storage` function L325-330 — `(&self) -> cloacina::dal::FilesystemRegistryStorage` — Create a filesystem storage backend for testing
- pub `initialize` function L333-362 — `(&mut self)` — Initialize the fixture with additional setup
- pub `reset_database` function L365-451 — `(&mut self)` — Reset the database by truncating all tables in the test schema
- pub `fixtures` module L468-534 — `-` — for integration tests.
-  `INIT` variable L41 — `: Once` — for integration tests.
-  `POSTGRES_FIXTURE` variable L43 — `: OnceCell<Arc<Mutex<TestFixture>>>` — for integration tests.
-  `SQLITE_FIXTURE` variable L45 — `: OnceCell<Arc<Mutex<TestFixture>>>` — for integration tests.
-  `DEFAULT_POSTGRES_URL` variable L49 — `: &str` — Default PostgreSQL connection URL
-  `get_test_schema` function L54-61 — `() -> String` — Get the test schema name from environment variable or generate a unique one
-  `DEFAULT_SQLITE_URL` variable L65 — `: &str` — Default SQLite connection URL (in-memory with shared cache for testing)
-  `backend_test` macro L186-206 — `-` — Macro for defining tests that run on all enabled backends.
-  `TestFixture` type L227-452 — `= TestFixture` — for integration tests.
-  `TableName` struct L383-386 — `{ tablename: String }` — for integration tests.
-  `TableName` struct L427-430 — `{ name: String }` — for integration tests.
-  `TestFixture` type L454-459 — `impl Drop for TestFixture` — for integration tests.
-  `drop` function L455-458 — `(&mut self)` — for integration tests.
-  `TableCount` struct L462-465 — `{ count: i64 }` — for integration tests.
-  `test_migration_function_postgres` function L475-502 — `()` — for integration tests.
-  `test_migration_function_sqlite` function L507-533 — `()` — for integration tests.

### crates/cloacina/tests/integration

> *Semantic summary to be generated by AI agent.*

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

#### crates/cloacina/tests/integration/logging.rs

-  `test_structured_logging` function L20-32 — `()`
-  `test_logging_with_context` function L35-50 — `()`
-  `test_span_creation` function L53-67 — `()`
-  `test_event_creation` function L70-83 — `()`

#### crates/cloacina/tests/integration/main.rs

- pub `context` module L20 — `-`
- pub `continuous` module L21 — `-`
- pub `dal` module L22 — `-`
- pub `database` module L23 — `-`
- pub `error` module L24 — `-`
- pub `executor` module L25 — `-`
- pub `logging` module L26 — `-`
- pub `models` module L27 — `-`
- pub `packaging` module L28 — `-`
- pub `packaging_inspection` module L29 — `-`
- pub `python_package` module L30 — `-`
- pub `registry_simple_functional_test` module L31 — `-`
- pub `registry_storage_tests` module L32 — `-`
- pub `registry_workflow_registry_tests` module L33 — `-`
- pub `runner_configurable_registry_tests` module L34 — `-`
- pub `scheduler` module L35 — `-`
- pub `signing` module L36 — `-`
- pub `task` module L37 — `-`
- pub `workflow` module L38 — `-`
-  `fixtures` module L41 — `-`

#### crates/cloacina/tests/integration/packaging.rs

-  `PackagingFixture` struct L32-36 — `{ temp_dir: TempDir, project_path: PathBuf, output_path: PathBuf }` — Test fixture for managing temporary projects and packages
-  `PackagingFixture` type L38-95 — `= PackagingFixture` — manifest generation, and archive creation.
-  `new` function L40-86 — `() -> Result<Self>` — Create a new packaging fixture with a test project
-  `get_project_path` function L88-90 — `(&self) -> &Path` — manifest generation, and archive creation.
-  `get_output_path` function L92-94 — `(&self) -> &Path` — manifest generation, and archive creation.
-  `test_compile_workflow_basic` function L99-142 — `()` — manifest generation, and archive creation.
-  `test_package_workflow_full_pipeline` function L146-181 — `()` — manifest generation, and archive creation.
-  `test_compile_options_default` function L184-191 — `()` — manifest generation, and archive creation.
-  `test_compile_options_custom` function L194-206 — `()` — manifest generation, and archive creation.
-  `test_packaging_with_cross_compilation` function L210-239 — `()` — manifest generation, and archive creation.
-  `test_packaging_invalid_project` function L243-254 — `()` — manifest generation, and archive creation.
-  `test_packaging_missing_cargo_toml` function L258-271 — `()` — manifest generation, and archive creation.
-  `test_packaging_with_cargo_flags` function L275-303 — `()` — manifest generation, and archive creation.
-  `test_package_manifest_serialization` function L306-343 — `()` — manifest generation, and archive creation.
-  `test_package_constants` function L346-352 — `()` — manifest generation, and archive creation.
-  `create_test_cargo_toml` function L355-370 — `() -> cloacina::packaging::types::CargoToml` — Helper function to create a minimal valid Cargo.toml for testing
-  `test_cargo_toml_parsing` function L373-387 — `()` — manifest generation, and archive creation.

#### crates/cloacina/tests/integration/packaging_inspection.rs

-  `PackageInspectionFixture` struct L32-36 — `{ temp_dir: TempDir, project_path: PathBuf, package_path: PathBuf }` — Test fixture for packaging and inspecting existing example projects
-  `PackageInspectionFixture` type L38-127 — `= PackageInspectionFixture` — and then inspecting the resulting package to verify task extraction works correctly.
-  `new` function L40-58 — `() -> Result<Self>` — Create a new fixture using an existing example project
-  `get_project_path` function L60-62 — `(&self) -> &Path` — and then inspecting the resulting package to verify task extraction works correctly.
-  `get_package_path` function L64-66 — `(&self) -> &Path` — and then inspecting the resulting package to verify task extraction works correctly.
-  `package_workflow` function L69-82 — `(&self) -> Result<()>` — Package the workflow using the cloacina library
-  `extract_manifest` function L85-104 — `(&self) -> Result<PackageManifest>` — Extract and parse the manifest from the packaged workflow
-  `verify_library_exists` function L107-126 — `(&self) -> Result<bool>` — Verify the package contains the expected library file
-  `test_package_and_inspect_workflow_complete` function L131-242 — `()` — and then inspecting the resulting package to verify task extraction works correctly.
-  `test_package_inspection_manifest_structure` function L246-284 — `()` — and then inspecting the resulting package to verify task extraction works correctly.
-  `test_package_inspection_error_handling` function L288-314 — `()` — and then inspecting the resulting package to verify task extraction works correctly.
-  `test_packaging_constants_integration` function L317-328 — `()` — and then inspecting the resulting package to verify task extraction works correctly.

#### crates/cloacina/tests/integration/python_package.rs

-  `build_archive` function L42-78 — `(manifest: &ManifestV2, workflow_files: &[(&str, &[u8])]) -> Vec<u8>` — Build a `.cloacina` archive in memory with realistic structure.
-  `data_pipeline_manifest` function L81-134 — `() -> ManifestV2` — Create a manifest matching the example data-pipeline project.
-  `data_pipeline_files` function L137-148 — `() -> Vec<(&'static str, &'static [u8])>` — Workflow source files for the data-pipeline example.
-  `peek_manifest_returns_correct_metadata` function L155-164 — `()` — round-trip: archive → peek → detect → extract → validate.
-  `detect_package_kind_identifies_python` function L167-173 — `()` — round-trip: archive → peek → detect → extract → validate.
-  `detect_package_kind_identifies_rust` function L176-191 — `()` — round-trip: archive → peek → detect → extract → validate.
-  `extract_python_package_full_roundtrip` function L194-220 — `()` — round-trip: archive → peek → detect → extract → validate.
-  `extract_rejects_rust_archive` function L223-245 — `()` — round-trip: archive → peek → detect → extract → validate.
-  `manifest_validates_task_dependency_references` function L252-261 — `()` — round-trip: archive → peek → detect → extract → validate.
-  `manifest_validates_duplicate_task_ids` function L264-273 — `()` — round-trip: archive → peek → detect → extract → validate.
-  `manifest_validates_python_function_path_format` function L276-285 — `()` — round-trip: archive → peek → detect → extract → validate.

#### crates/cloacina/tests/integration/registry_simple_functional_test.rs

-  `create_test_database` function L34-39 — `() -> Database` — Helper to create a test database using the fixture pattern
-  `create_test_storage` function L42-49 — `() -> FilesystemRegistryStorage` — Helper to create a test filesystem storage
-  `test_registry_with_simple_binary_data` function L53-75 — `()` — and demonstrates the new streamlined API.
-  `test_registry_with_real_package_if_available` function L79-140 — `()` — and demonstrates the new streamlined API.
-  `test_registry_api_simplification` function L144-175 — `()` — and demonstrates the new streamlined API.

#### crates/cloacina/tests/integration/registry_storage_tests.rs

- pub `test_store_and_retrieve_impl` function L54-67 — `(mut storage: S)` — Test store and retrieve operations
- pub `test_retrieve_nonexistent_impl` function L70-78 — `(storage: S)` — Test retrieving non-existent data
- pub `test_delete_impl` function L81-98 — `(mut storage: S)` — Test delete operations
- pub `test_invalid_uuid_impl` function L101-107 — `(mut storage: S)` — Test invalid UUID handling
- pub `test_empty_data_impl` function L110-116 — `(mut storage: S)` — Test empty data storage
- pub `test_large_data_impl` function L119-126 — `(mut storage: S)` — Test large data storage
- pub `test_uuid_format_impl` function L129-140 — `(mut storage: S)` — Test UUID format validation
- pub `test_binary_data_integrity_impl` function L143-154 — `(mut storage: S)` — Test binary data integrity
-  `create_test_workflow_data` function L35-47 — `(size: usize) -> Vec<u8>` — Helper to create test data that simulates a compiled .so file
-  `storage_tests` module L50-155 — `-` — Unified storage test implementations that work with any storage backend
-  `filesystem_tests` module L158-215 — `-` — The same test suite runs against all backends.
-  `create_filesystem_storage` function L161-166 — `() -> (FilesystemRegistryStorage, TempDir)` — The same test suite runs against all backends.
-  `test_store_and_retrieve` function L169-172 — `()` — The same test suite runs against all backends.
-  `test_retrieve_nonexistent` function L175-178 — `()` — The same test suite runs against all backends.
-  `test_delete` function L181-184 — `()` — The same test suite runs against all backends.
-  `test_invalid_uuid` function L187-190 — `()` — The same test suite runs against all backends.
-  `test_empty_data` function L193-196 — `()` — The same test suite runs against all backends.
-  `test_large_data` function L199-202 — `()` — The same test suite runs against all backends.
-  `test_uuid_format` function L205-208 — `()` — The same test suite runs against all backends.
-  `test_binary_data_integrity` function L211-214 — `()` — The same test suite runs against all backends.
-  `database_tests` module L218-284 — `-` — The same test suite runs against all backends.
-  `create_database_storage` function L222-227 — `() -> UnifiedRegistryStorage` — The same test suite runs against all backends.
-  `test_store_and_retrieve` function L231-234 — `()` — The same test suite runs against all backends.
-  `test_retrieve_nonexistent` function L238-241 — `()` — The same test suite runs against all backends.
-  `test_delete` function L245-248 — `()` — The same test suite runs against all backends.
-  `test_invalid_uuid` function L252-255 — `()` — The same test suite runs against all backends.
-  `test_empty_data` function L259-262 — `()` — The same test suite runs against all backends.
-  `test_large_data` function L266-269 — `()` — The same test suite runs against all backends.
-  `test_uuid_format` function L273-276 — `()` — The same test suite runs against all backends.
-  `test_binary_data_integrity` function L280-283 — `()` — The same test suite runs against all backends.

#### crates/cloacina/tests/integration/registry_workflow_registry_tests.rs

-  `PackageFixture` struct L38-41 — `{ temp_dir: tempfile::TempDir, package_path: std::path::PathBuf }` — Test fixture for managing package files
-  `PackageFixture` type L43-151 — `= PackageFixture` — including storage, metadata extraction, validation, and task registration.
-  `new` function L49-108 — `() -> Self` — Create a new package fixture from pre-built .so files.
-  `find_prebuilt_library` function L111-140 — `(project_path: &std::path::Path) -> Option<std::path::PathBuf>` — Find the pre-built library in the project's target directory.
-  `get_package_data` function L143-145 — `(&self) -> Vec<u8>` — Get the package data as bytes
-  `get_package_path` function L148-150 — `(&self) -> &std::path::Path` — Get the path to the package file
-  `create_mock_elf_data` function L154-179 — `() -> Vec<u8>` — Helper to create mock ELF-like binary data for testing
-  `create_test_storage` function L182-187 — `( database: cloacina::Database, ) -> impl cloacina::registry::traits::RegistrySt...` — Helper to create a test storage backend appropriate for the current database
-  `create_test_filesystem_storage` function L190-197 — `() -> FilesystemRegistryStorage` — Helper to create a test filesystem storage (for tests that specifically need filesystem)
-  `test_workflow_registry_creation` function L201-217 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_register_workflow_with_invalid_package` function L221-242 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_register_real_workflow_package` function L246-287 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_get_workflow_nonexistent` function L291-302 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_unregister_nonexistent_workflow` function L306-319 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_list_workflows_empty` function L323-335 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_workflow_registry_with_multiple_packages` function L339-370 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_concurrent_registry_operations` function L374-424 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_registry_error_handling` function L428-451 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_storage_integration` function L455-475 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_database_integration` function L479-500 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_registry_memory_safety` function L504-522 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_package_lifecycle` function L526-554 — `()` — including storage, metadata extraction, validation, and task registration.
-  `test_validation_integration` function L558-580 — `()` — including storage, metadata extraction, validation, and task registration.

#### crates/cloacina/tests/integration/runner_configurable_registry_tests.rs

- pub `test_runner_creation_impl` function L81-91 — `(runner: DefaultRunner)` — Test that a runner can be created with a specific storage backend
- pub `test_workflow_registration_impl` function L94-114 — `(runner: DefaultRunner)` — Test that workflows can be registered and listed
- pub `test_registry_configuration_impl` function L117-135 — `(runner: DefaultRunner, expected_backend: &str)` — Test that the registry configuration is applied correctly
- pub `test_runner_shutdown_impl` function L138-142 — `(runner: DefaultRunner)` — Test that the runner can be shut down cleanly
-  `create_test_package` function L34-50 — `() -> Vec<u8>` — Helper to create a minimal test package (.cloacina file)
-  `create_test_config` function L53-65 — `(storage_backend: &str, temp_dir: Option<&TempDir>) -> DefaultRunnerConfig` — Helper to create a test runner config with the specified storage backend
-  `get_database_url_for_test` function L69-74 — `() -> String` — Helper to get the appropriate database URL for testing
-  `registry_tests` module L77-143 — `-` — Unified test implementations that work with any storage backend
-  `filesystem_tests` module L146-217 — `-` — correctly in end-to-end scenarios.
-  `create_filesystem_runner` function L149-160 — `() -> (DefaultRunner, TempDir)` — correctly in end-to-end scenarios.
-  `test_filesystem_runner_creation` function L163-166 — `()` — correctly in end-to-end scenarios.
-  `test_filesystem_workflow_registration` function L169-172 — `()` — correctly in end-to-end scenarios.
-  `test_filesystem_registry_configuration` function L175-178 — `()` — correctly in end-to-end scenarios.
-  `test_filesystem_runner_shutdown` function L181-184 — `()` — correctly in end-to-end scenarios.
-  `test_filesystem_custom_path` function L187-216 — `()` — correctly in end-to-end scenarios.
-  `current_backend_tests` module L220-304 — `-` — correctly in end-to-end scenarios.
-  `create_current_backend_runner` function L223-235 — `() -> DefaultRunner` — correctly in end-to-end scenarios.
-  `get_current_backend` function L237-241 — `() -> String` — correctly in end-to-end scenarios.
-  `test_current_backend_runner_creation` function L245-248 — `()` — correctly in end-to-end scenarios.
-  `test_current_backend_workflow_registration` function L252-255 — `()` — correctly in end-to-end scenarios.
-  `test_current_backend_registry_configuration` function L259-263 — `()` — correctly in end-to-end scenarios.
-  `test_current_backend_runner_shutdown` function L267-270 — `()` — correctly in end-to-end scenarios.
-  `test_current_backend_registry_uses_same_database` function L274-303 — `()` — correctly in end-to-end scenarios.
-  `error_tests` module L307-370 — `-` — correctly in end-to-end scenarios.
-  `test_invalid_storage_backend` function L311-339 — `()` — correctly in end-to-end scenarios.
-  `test_registry_disabled` function L342-369 — `()` — correctly in end-to-end scenarios.
-  `integration_tests` module L373-451 — `-` — correctly in end-to-end scenarios.
-  `test_filesystem_and_current_backend_runners` function L378-450 — `()` — correctly in end-to-end scenarios.

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

### crates/cloacina/tests/integration/continuous

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/tests/integration/continuous/accumulator_persistence.rs

-  `get_fresh_dal` function L30-36 — `() -> cloacina::dal::DAL` — Get a DAL with fresh migrations (resets and re-initializes the fixture).
-  `test_save_and_load_accumulator_state` function L40-69 — `()` — code paths are exercised at runtime, not just compiled.
-  `test_save_upserts_on_conflict` function L73-110 — `()` — code paths are exercised at runtime, not just compiled.
-  `test_load_all_and_delete` function L114-160 — `()` — code paths are exercised at runtime, not just compiled.
-  `test_load_nonexistent_returns_none` function L164-174 — `()` — code paths are exercised at runtime, not just compiled.
-  `test_detector_state_save_and_load` function L182-216 — `()` — code paths are exercised at runtime, not just compiled.
-  `test_detector_state_upserts` function L220-260 — `()` — code paths are exercised at runtime, not just compiled.
-  `test_detector_state_load_nonexistent` function L264-290 — `()` — code paths are exercised at runtime, not just compiled.
-  `test_detector_state_load_all` function L294-334 — `()` — code paths are exercised at runtime, not just compiled.
-  `test_detector_state_save_with_null_committed_state` function L338-378 — `()` — code paths are exercised at runtime, not just compiled.
-  `test_pending_boundary_append_and_load` function L386-468 — `()` — code paths are exercised at runtime, not just compiled.
-  `test_edge_drain_cursor_lifecycle` function L472-550 — `()` — code paths are exercised at runtime, not just compiled.
-  `test_boundary_cleanup_after_all_consumers_drain` function L554-628 — `()` — code paths are exercised at runtime, not just compiled.
-  `test_init_cursor_idempotent` function L632-669 — `()` — code paths are exercised at runtime, not just compiled.
-  `test_max_id_for_source` function L673-715 — `()` — code paths are exercised at runtime, not just compiled.
-  `test_multi_source_isolation` function L719-802 — `()` — code paths are exercised at runtime, not just compiled.
-  `test_load_all_cursors` function L806-865 — `()` — code paths are exercised at runtime, not just compiled.
-  `test_list_orphaned_states` function L873-955 — `()` — code paths are exercised at runtime, not just compiled.
-  `MockConn` struct L913 — `-` — code paths are exercised at runtime, not just compiled.
-  `MockConn` type L914-927 — `impl DataConnection for MockConn` — code paths are exercised at runtime, not just compiled.
-  `connect` function L915-917 — `(&self) -> Result<Box<dyn Any>, DataConnectionError>` — code paths are exercised at runtime, not just compiled.
-  `descriptor` function L918-923 — `(&self) -> ConnectionDescriptor` — code paths are exercised at runtime, not just compiled.
-  `system_metadata` function L924-926 — `(&self) -> serde_json::Value` — code paths are exercised at runtime, not just compiled.
-  `test_prune_orphaned_states` function L959-1002 — `()` — code paths are exercised at runtime, not just compiled.
-  `test_prune_no_orphans` function L1006-1028 — `()` — code paths are exercised at runtime, not just compiled.

#### crates/cloacina/tests/integration/continuous/mod.rs

- pub `accumulator_persistence` module L22 — `-` — Integration tests for the continuous scheduling pipeline.
- pub `recovery_e2e` module L23 — `-` — detector output → accumulator → task fires → ledger records completion
- pub `runner_lifecycle` module L24 — `-` — detector output → accumulator → task fires → ledger records completion
-  `PassthroughTask` struct L43-45 — `{ id: String }` — A simple continuous task for integration tests that passes through context.
-  `PassthroughTask` type L47-51 — `= PassthroughTask` — detector output → accumulator → task fires → ledger records completion
-  `new` function L48-50 — `(id: &str) -> Self` — detector output → accumulator → task fires → ledger records completion
-  `PassthroughTask` type L54-68 — `impl Task for PassthroughTask` — detector output → accumulator → task fires → ledger records completion
-  `execute` function L55-61 — `( &self, mut context: cloacina_workflow::Context<serde_json::Value>, ) -> Result...` — detector output → accumulator → task fires → ledger records completion
-  `id` function L62-64 — `(&self) -> &str` — detector output → accumulator → task fires → ledger records completion
-  `dependencies` function L65-67 — `(&self) -> &[cloacina_workflow::TaskNamespace]` — detector output → accumulator → task fires → ledger records completion
-  `MockConn` struct L76 — `-` — detector output → accumulator → task fires → ledger records completion
-  `MockConn` type L77-90 — `impl DataConnection for MockConn` — detector output → accumulator → task fires → ledger records completion
-  `connect` function L78-80 — `(&self) -> Result<Box<dyn Any>, DataConnectionError>` — detector output → accumulator → task fires → ledger records completion
-  `descriptor` function L81-86 — `(&self) -> ConnectionDescriptor` — detector output → accumulator → task fires → ledger records completion
-  `system_metadata` function L87-89 — `(&self) -> serde_json::Value` — detector output → accumulator → task fires → ledger records completion
-  `make_source` function L92-99 — `(name: &str) -> DataSource` — detector output → accumulator → task fires → ledger records completion
-  `make_boundary` function L101-107 — `(start: i64, end: i64) -> ComputationBoundary` — detector output → accumulator → task fires → ledger records completion
-  `make_detector_completion` function L109-119 — `(task_name: &str, boundaries: Vec<ComputationBoundary>) -> LedgerEvent` — detector output → accumulator → task fires → ledger records completion
-  `test_full_reactive_loop` function L123-188 — `()` — Full reactive loop: detector emits boundaries → accumulator receives → task fires.
-  `test_multiple_detector_outputs_accumulate` function L192-239 — `()` — Multiple detector outputs accumulate before firing.
-  `test_multi_source_task` function L243-286 — `()` — Multi-source task: boundaries arrive on two sources.
-  `test_ledger_records_drains` function L290-339 — `()` — Ledger records accumulator drains.
-  `test_windowed_accumulator_waits_for_watermark` function L345-370 — `()` — WindowedAccumulator with WaitForWatermark blocks until watermark covers boundary.
-  `test_ledger_trigger_feedback_loop` function L374-409 — `()` — LedgerTrigger completes the reactive feedback loop.
-  `test_ledger_trigger_all_mode_multi_dependency` function L413-449 — `()` — LedgerTrigger All mode: waits for both upstream tasks.
-  `test_scheduler_watermark_advance_via_both` function L453-501 — `()` — Full scheduler loop with watermark advance via Both output.
-  `test_multi_cycle_reactive_loop` function L514-625 — `()` — Multi-cycle reactive loop: source → task_a → derived source → task_b.
-  `test_ledger_trigger_bridges_cycles` function L630-682 — `()` — LedgerTrigger integration: verify it correctly bridges task completion

#### crates/cloacina/tests/integration/continuous/recovery_e2e.rs

-  `MockConn` struct L46 — `-` — - Multi-consumer fan-out recovers correctly per-edge
-  `MockConn` type L47-60 — `impl DataConnection for MockConn` — - Multi-consumer fan-out recovers correctly per-edge
-  `connect` function L48-50 — `(&self) -> Result<Box<dyn Any>, DataConnectionError>` — - Multi-consumer fan-out recovers correctly per-edge
-  `descriptor` function L51-56 — `(&self) -> ConnectionDescriptor` — - Multi-consumer fan-out recovers correctly per-edge
-  `system_metadata` function L57-59 — `(&self) -> serde_json::Value` — - Multi-consumer fan-out recovers correctly per-edge
-  `make_source` function L62-69 — `(name: &str) -> DataSource` — - Multi-consumer fan-out recovers correctly per-edge
-  `make_boundary` function L71-77 — `(start: i64, end: i64) -> ComputationBoundary` — - Multi-consumer fan-out recovers correctly per-edge
-  `make_detector_completion_with_state` function L79-94 — `( task_name: &str, boundaries: Vec<ComputationBoundary>, last_known_state: serde...` — - Multi-consumer fan-out recovers correctly per-edge
-  `make_detector_completion` function L96-106 — `(task_name: &str, boundaries: Vec<ComputationBoundary>) -> LedgerEvent` — - Multi-consumer fan-out recovers correctly per-edge
-  `PassthroughTask` struct L108-110 — `{ id: String }` — - Multi-consumer fan-out recovers correctly per-edge
-  `PassthroughTask` type L112-116 — `= PassthroughTask` — - Multi-consumer fan-out recovers correctly per-edge
-  `new` function L113-115 — `(id: &str) -> Self` — - Multi-consumer fan-out recovers correctly per-edge
-  `PassthroughTask` type L119-133 — `impl Task for PassthroughTask` — - Multi-consumer fan-out recovers correctly per-edge
-  `execute` function L120-126 — `( &self, mut context: Context<serde_json::Value>, ) -> Result<Context<serde_json...` — - Multi-consumer fan-out recovers correctly per-edge
-  `id` function L127-129 — `(&self) -> &str` — - Multi-consumer fan-out recovers correctly per-edge
-  `dependencies` function L130-132 — `(&self) -> &[cloacina_workflow::TaskNamespace]` — - Multi-consumer fan-out recovers correctly per-edge
-  `get_fresh_dal` function L135-141 — `() -> cloacina::dal::DAL` — - Multi-consumer fan-out recovers correctly per-edge
-  `make_config` function L143-149 — `() -> ContinuousSchedulerConfig` — - Multi-consumer fan-out recovers correctly per-edge
-  `test_e2e_crash_recovery_single_consumer` function L159-247 — `()` — Full cycle: emit boundaries → run scheduler → task drains → persist state →
-  `test_e2e_multi_consumer_partial_drain_recovery` function L253-343 — `()` — Multi-consumer fan-out: 2 edges from same source, only one drains before crash.
-  `test_e2e_detector_state_roundtrip` function L348-388 — `()` — Verify detector state committed state is accessible via the store after restart.
-  `test_e2e_boundary_wal_restore_fires_task` function L393-454 — `()` — Verify pending boundary WAL → restore into accumulator → task fires.
-  `test_e2e_cursor_skips_already_consumed_boundaries` function L459-519 — `()` — Verify cursor-based restore only re-injects unconsumed boundaries.
-  `test_e2e_commit_gate_requires_all_consumers` function L524-601 — `()` — Verify the commit gate: detector state only commits when ALL consumers drain.

#### crates/cloacina/tests/integration/continuous/runner_lifecycle.rs

-  `test_continuous_scheduler_empty_graph_lifecycle` function L31-60 — `()` — Test: continuous scheduling enabled with empty graph starts and stops cleanly.
-  `test_continuous_scheduler_disabled_by_default` function L66-88 — `()` — Test: continuous scheduling disabled (default) starts and stops without

### crates/cloacina/tests/integration/dal

> *Semantic summary to be generated by AI agent.*

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
-  `test_concurrent_claiming_no_duplicates` function L592-724 — `()` — Test that concurrent workers don't cause duplicate claims.
-  `NUM_TASKS` variable L618 — `: usize` — Tests run on all enabled backends (SQLite, PostgreSQL) using `get_all_fixtures()`.
-  `NUM_WORKERS` variable L644 — `: usize` — Tests run on all enabled backends (SQLite, PostgreSQL) using `get_all_fixtures()`.
-  `test_event_count_and_deletion` function L732-819 — `()` — Test count_by_pipeline and delete_older_than for retention policy.
-  `test_get_recent_events` function L823-886 — `()` — Test get_recent returns events in correct order.
-  `test_manual_event_with_data` function L894-977 — `()` — Test that manually created events with event_data are correctly stored.

#### crates/cloacina/tests/integration/dal/mod.rs

- pub `context` module L17 — `-`
- pub `execution_events` module L18 — `-`
- pub `sub_status` module L19 — `-`
- pub `task_claiming` module L20 — `-`
- pub `workflow_packages` module L21 — `-`
- pub `workflow_registry` module L22 — `-`
- pub `workflow_registry_reconciler_integration` module L23 — `-`

#### crates/cloacina/tests/integration/dal/sub_status.rs

-  `test_sub_status_crud_operations` function L39-161 — `()` — Tests all sub_status operations in a single test to avoid fixture contention.

#### crates/cloacina/tests/integration/dal/task_claiming.rs

-  `test_concurrent_task_claiming_no_duplicates` function L44-199 — `()` — Test that concurrent task claiming doesn't produce duplicate claims.
-  `NUM_TASKS` variable L71 — `: usize` — Tests run on all enabled backends (SQLite, PostgreSQL) using `get_all_fixtures()`.
-  `NUM_WORKERS` variable L114 — `: usize` — Tests run on all enabled backends (SQLite, PostgreSQL) using `get_all_fixtures()`.
-  `test_claimed_tasks_marked_running` function L203-286 — `()` — Test that claimed tasks have their status properly updated to Running.
-  `test_running_tasks_not_claimable` function L290-343 — `()` — Test that already-running tasks cannot be claimed again.

#### crates/cloacina/tests/integration/dal/workflow_packages.rs

-  `test_store_and_get_package_metadata` function L23-77 — `()`
-  `test_store_duplicate_package_metadata` function L80-135 — `()`
-  `test_list_all_packages` function L138-201 — `()`
-  `test_delete_package_metadata` function L204-262 — `()`
-  `test_delete_nonexistent_package` function L265-285 — `()`
-  `test_get_nonexistent_package` function L288-306 — `()`
-  `test_store_package_with_complex_metadata` function L309-402 — `()`
-  `test_store_package_with_invalid_uuid` function L405-439 — `()`
-  `test_package_versioning` function L442-511 — `()`

#### crates/cloacina/tests/integration/dal/workflow_registry.rs

-  `MOCK_PACKAGE` variable L28 — `: OnceLock<Vec<u8>>` — Cached mock package data.
-  `get_mock_package` function L35-39 — `() -> Vec<u8>` — Get the cached mock package, creating it from pre-built .so if necessary.
-  `create_package_from_prebuilt_so` function L47-99 — `() -> Vec<u8>` — Create a package from pre-built .so file without spawning cargo.
-  `find_prebuilt_library` function L102-131 — `(project_path: &std::path::Path) -> Option<std::path::PathBuf>` — Find the pre-built library in the project's target directory.
-  `test_register_and_get_workflow_package` function L135-139 — `()`
-  `test_register_and_get_workflow_package_with_db_storage` function L141-174 — `()`
-  `test_register_and_get_workflow_package_with_fs_storage` function L176-208 — `()`
-  `test_get_workflow_package_by_name` function L212-217 — `()`
-  `test_get_workflow_package_by_name_with_db_storage` function L219-259 — `()`
-  `test_get_workflow_package_by_name_with_fs_storage` function L261-301 — `()`
-  `test_unregister_workflow_package_by_id` function L305-310 — `()`
-  `test_unregister_workflow_package_by_id_with_db_storage` function L312-350 — `()`
-  `test_unregister_workflow_package_by_id_with_fs_storage` function L352-390 — `()`
-  `test_unregister_workflow_package_by_name` function L394-399 — `()`
-  `test_unregister_workflow_package_by_name_with_db_storage` function L401-448 — `()`
-  `test_unregister_workflow_package_by_name_with_fs_storage` function L450-497 — `()`
-  `test_list_packages` function L501-506 — `()`
-  `test_list_packages_with_db_storage` function L508-548 — `()`
-  `test_list_packages_with_fs_storage` function L550-590 — `()`
-  `test_register_duplicate_package` function L594-599 — `()`
-  `test_register_duplicate_package_with_db_storage` function L601-636 — `()`
-  `test_register_duplicate_package_with_fs_storage` function L638-673 — `()`
-  `test_exists_operations` function L677-682 — `()`
-  `test_exists_operations_with_db_storage` function L684-732 — `()`
-  `test_exists_operations_with_fs_storage` function L734-782 — `()`
-  `test_get_nonexistent_package` function L786-791 — `()`
-  `test_get_nonexistent_package_with_db_storage` function L793-820 — `()`
-  `test_get_nonexistent_package_with_fs_storage` function L822-849 — `()`
-  `test_unregister_nonexistent_package` function L853-858 — `()`
-  `test_unregister_nonexistent_package_with_db_storage` function L860-891 — `()`
-  `test_unregister_nonexistent_package_with_fs_storage` function L893-924 — `()`

#### crates/cloacina/tests/integration/dal/workflow_registry_reconciler_integration.rs

-  `TEST_PACKAGE` variable L31 — `: OnceLock<Vec<u8>>` — Cached test package data.
-  `get_test_package` function L38-42 — `() -> Vec<u8>` — Get the cached test package, creating it from pre-built .so if necessary.
-  `create_package_from_prebuilt_so` function L50-103 — `() -> Vec<u8>` — Create a package from pre-built .so file without spawning cargo.
-  `find_prebuilt_library` function L106-135 — `(project_path: &std::path::Path) -> Option<std::path::PathBuf>` — Find the pre-built library in the project's target directory.
-  `test_dal_register_then_reconciler_load` function L139-229 — `()` — Integration tests for the end-to-end workflow: register package via DAL → load via reconciler
-  `test_dal_register_then_get_workflow_package_by_id_failure_case` function L233-275 — `()` — Integration tests for the end-to-end workflow: register package via DAL → load via reconciler

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
-  `test_context_merging_latest_wins` function L120-260 — `()`
-  `scope_inspector_task` function L266-276 — `(context: &mut Context<Value>) -> Result<(), TaskError>`
-  `test_execution_scope_context_setup` function L279-391 — `()`

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
-  `test_defer_until_full_pipeline` function L180-252 — `()` — Verifies that a task using `defer_until` via TaskHandle completes
-  `test_defer_until_with_downstream_dependency` function L256-339 — `()` — Verifies that a deferred task correctly chains with a downstream task.
-  `test_sub_status_transitions_during_deferral` function L344-433 — `()` — Verifies that sub_status transitions through "Deferred" while the task is

#### crates/cloacina/tests/integration/executor/mod.rs

- pub `context_merging` module L17 — `-`
- pub `defer_until` module L18 — `-`
- pub `multi_tenant` module L19 — `-`
- pub `pause_resume` module L20 — `-`
- pub `task_execution` module L21 — `-`

#### crates/cloacina/tests/integration/executor/multi_tenant.rs

-  `postgres_multi_tenant_tests` module L19-290 — `-` — Integration tests for multi-tenant functionality
-  `tenant_marker_task` function L33-37 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — Simple task that marks its tenant in the context
-  `setup_tenant_workflow` function L40-67 — `(tenant_schema: &str) -> Workflow` — Helper to create and register a workflow for a specific tenant schema
-  `test_schema_isolation` function L71-159 — `() -> Result<(), Box<dyn std::error::Error>>` — Test that schema-based multi-tenancy provides complete data isolation
-  `test_independent_execution` function L163-235 — `() -> Result<(), Box<dyn std::error::Error>>` — Test that the same workflow can execute independently in different tenants
-  `test_invalid_schema_names` function L239-260 — `()` — Test that invalid schema names are rejected
-  `test_sqlite_schema_rejection` function L264-272 — `()` — Test that schema isolation is only supported for PostgreSQL
-  `test_builder_pattern` function L276-289 — `() -> Result<(), Box<dyn std::error::Error>>` — Test builder pattern for multi-tenant setup
-  `sqlite_multi_tenant_tests` module L292-447 — `-` — Integration tests for multi-tenant functionality
-  `sqlite_tenant_task` function L305-308 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — Simple task for SQLite tests
-  `setup_sqlite_workflow` function L311-337 — `(db_name: &str) -> Workflow` — Helper to create and register a workflow for SQLite tests
-  `test_sqlite_file_isolation` function L341-424 — `() -> Result<(), Box<dyn std::error::Error>>` — Test that SQLite multi-tenancy works with separate database files
-  `test_sqlite_separate_files` function L428-446 — `() -> Result<(), Box<dyn std::error::Error>>` — Test that SQLite creates separate database files

#### crates/cloacina/tests/integration/executor/pause_resume.rs

-  `wait_for_status` function L33-55 — `( execution: &PipelineExecution, target: impl Fn(&PipelineStatus) -> bool, timeo...` — Helper to wait for a specific pipeline status without consuming the execution handle.
-  `wait_for_terminal` function L58-63 — `( execution: &PipelineExecution, timeout: Duration, ) -> Result<PipelineStatus, ...` — Wait for the pipeline to reach a terminal state (Completed, Failed, or Cancelled)
-  `WorkflowTask` struct L67-70 — `{ id: String, dependencies: Vec<TaskNamespace> }` — Integration tests for workflow pause/resume functionality.
-  `WorkflowTask` type L72-82 — `= WorkflowTask` — Integration tests for workflow pause/resume functionality.
-  `new` function L73-81 — `(id: &str, deps: Vec<&str>) -> Self` — Integration tests for workflow pause/resume functionality.
-  `WorkflowTask` type L85-100 — `impl Task for WorkflowTask` — Integration tests for workflow pause/resume functionality.
-  `execute` function L86-91 — `( &self, context: Context<serde_json::Value>, ) -> Result<Context<serde_json::Va...` — Integration tests for workflow pause/resume functionality.
-  `id` function L93-95 — `(&self) -> &str` — Integration tests for workflow pause/resume functionality.
-  `dependencies` function L97-99 — `(&self) -> &[TaskNamespace]` — Integration tests for workflow pause/resume functionality.
-  `quick_task` function L106-109 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — Integration tests for workflow pause/resume functionality.
-  `slow_first_task` function L115-120 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — Integration tests for workflow pause/resume functionality.
-  `slow_second_task` function L126-131 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — Integration tests for workflow pause/resume functionality.
-  `test_pause_running_pipeline` function L134-237 — `()` — Integration tests for workflow pause/resume functionality.
-  `test_resume_paused_pipeline` function L240-361 — `()` — Integration tests for workflow pause/resume functionality.
-  `test_pause_non_running_pipeline_fails` function L364-431 — `()` — Integration tests for workflow pause/resume functionality.
-  `test_resume_non_paused_pipeline_fails` function L434-509 — `()` — Integration tests for workflow pause/resume functionality.

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
-  `test_task_executor_basic_execution` function L119-198 — `()`
-  `test_task_executor_dependency_loading` function L201-325 — `()`
-  `test_task_executor_timeout_handling` function L328-416 — `()`
-  `unified_task_test` function L422-426 — `(context: &mut Context<Value>) -> Result<(), TaskError>`
-  `test_default_runner_execution` function L429-536 — `()`
-  `initial_context_task_test` function L542-557 — `(context: &mut Context<Value>) -> Result<(), TaskError>`
-  `test_task_executor_context_loading_no_dependencies` function L560-689 — `()`
-  `producer_context_task` function L695-710 — `(context: &mut Context<Value>) -> Result<(), TaskError>`
-  `consumer_context_task` function L716-739 — `(context: &mut Context<Value>) -> Result<(), TaskError>`
-  `test_task_executor_context_loading_with_dependencies` function L742-913 — `()`

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

-  `test_cron_evaluator_basic` function L29-41 — `()`
-  `test_cron_schedule_creation` function L45-64 — `()`
-  `test_default_runner_cron_integration` function L68-110 — `()`
-  `test_cron_scheduler_startup_shutdown` function L114-134 — `()`

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
-  `trigger_rules` module L22 — `-`

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

#### crates/cloacina/tests/integration/scheduler/trigger_rules.rs

-  `SimpleTask` struct L27-29 — `{ id: String }`
-  `SimpleTask` type L32-47 — `impl Task for SimpleTask`
-  `execute` function L33-38 — `( &self, context: Context<serde_json::Value>, ) -> Result<Context<serde_json::Va...`
-  `id` function L40-42 — `(&self) -> &str`
-  `dependencies` function L44-46 — `(&self) -> &[TaskNamespace]`
-  `test_always_trigger_rule` function L51-101 — `()`
-  `test_trigger_rule_serialization` function L105-142 — `()`
-  `test_context_value_operators` function L146-172 — `()`
-  `test_trigger_condition_types` function L176-203 — `()`
-  `test_complex_trigger_rule` function L207-233 — `()`

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
-  `test_invalid_signature_rejected` function L92-131 — `()` — Test that an invalid signature (wrong bytes) is rejected.
-  `test_wrong_hash_in_signature_rejected` function L135-163 — `()` — Test that a signature with wrong hash is rejected.
-  `test_malformed_signature_file_rejected` function L167-183 — `()` — Test that malformed signature JSON is rejected.
-  `test_missing_signature_file` function L187-196 — `()` — Test that missing signature file is handled.
-  `test_empty_package` function L200-213 — `()` — Test that empty package is handled correctly.
-  `test_revoked_key_rejected` function L220-229 — `()` — Database-based tests for revoked key rejection.
-  `sign_package_helper` function L232-258 — `( package_path: &std::path::Path, keypair: &cloacina::crypto::GeneratedKeypair, ...` — Helper function to sign a package.

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

#### crates/cloacina/tests/integration/task/continuous_macro.rs

-  `aggregate_hourly` function L26-29 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Tests for the #[continuous_task] proc macro.
-  `test_continuous_task_compiles_and_creates` function L32-35 — `()` — Tests for the #[continuous_task] proc macro.
-  `test_continuous_task_sources` function L38-40 — `()` — Tests for the #[continuous_task] proc macro.
-  `test_continuous_task_referenced_empty` function L43-45 — `()` — Tests for the #[continuous_task] proc macro.
-  `test_continuous_task_is_continuous` function L48-50 — `()` — Tests for the #[continuous_task] proc macro.
-  `test_continuous_task_has_fingerprint` function L53-56 — `()` — Tests for the #[continuous_task] proc macro.
-  `test_continuous_task_execute_runs_function` function L59-66 — `()` — Tests for the #[continuous_task] proc macro.
-  `join_metrics` function L75-78 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Tests for the #[continuous_task] proc macro.
-  `test_continuous_task_multiple_sources` function L81-83 — `()` — Tests for the #[continuous_task] proc macro.
-  `test_continuous_task_with_referenced` function L86-88 — `()` — Tests for the #[continuous_task] proc macro.
-  `test_continuous_task_with_referenced_executes` function L91-96 — `()` — Tests for the #[continuous_task] proc macro.
-  `process_boundaries` function L101-109 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Tests for the #[continuous_task] proc macro.
-  `test_continuous_task_reads_boundary_from_context` function L112-123 — `()` — Tests for the #[continuous_task] proc macro.
-  `failing_task` function L128-134 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Tests for the #[continuous_task] proc macro.
-  `test_continuous_task_failure_propagates` function L137-142 — `()` — Tests for the #[continuous_task] proc macro.

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
- pub `continuous_macro` module L18 — `-`
- pub `debug_macro` module L19 — `-`
- pub `handle_macro` module L20 — `-`
- pub `macro_test` module L21 — `-`
- pub `simple_macro` module L22 — `-`

#### crates/cloacina/tests/integration/task/simple_macro.rs

-  `test_task` function L20-24 — `( _context: &mut cloacina::Context<serde_json::Value>, ) -> Result<(), cloacina:...`
-  `test_macro_expansion` function L27-30 — `()`

### crates/cloacina/tests/integration/workflow

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina/tests/integration/workflow/basic.rs

-  `simple_task` function L20-24 — `( _context: &mut cloacina::Context<serde_json::Value>, ) -> Result<(), cloacina:...`
-  `test_simple_workflow_creation` function L27-41 — `()`

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

-  `fetch_document` function L21-24 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
-  `extract_text` function L27-30 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
-  `generate_embeddings` function L33-36 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
-  `store_embeddings` function L39-42 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
-  `test_workflow_macro_basic` function L45-122 — `()`
-  `test_workflow_macro_minimal` function L125-140 — `()`
-  `task_a` function L144-146 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
-  `task_b` function L149-151 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
-  `task_c` function L154-156 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
-  `test_workflow_execution_levels` function L159-186 — `()`
-  `test_workflow_roots_and_leaves` function L189-214 — `()`

#### crates/cloacina/tests/integration/workflow/mod.rs

- pub `basic` module L17 — `-`
- pub `callback_test` module L18 — `-`
- pub `macro_test` module L19 — `-`
- pub `subgraph` module L20 — `-`

#### crates/cloacina/tests/integration/workflow/subgraph.rs

-  `root_task_a` function L21-23 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
-  `root_task_b` function L26-28 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
-  `middle_task_c` function L31-33 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
-  `middle_task_d` function L36-38 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
-  `final_task_e` function L41-43 — `(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
-  `test_subgraph_unsupported_operation` function L46-74 — `()`
-  `test_subgraph_with_nonexistent_task` function L77-96 — `()`
-  `test_subgraph_dependency_collection` function L99-138 — `()`
-  `test_subgraph_metadata_operations` function L141-159 — `()`
-  `test_single_task_subgraph` function L162-181 — `()`
-  `test_empty_subgraph_request` function L184-208 — `()`
-  `test_subgraph_with_partial_dependencies` function L211-236 — `()`

### crates/cloacina-macros/src

> *Semantic summary to be generated by AI agent.*

#### crates/cloacina-macros/src/continuous_task.rs

- pub `ContinuousTaskAttributes` struct L34-38 — `{ id: String, sources: Vec<String>, referenced: Vec<String> }` — Attributes for the continuous_task macro.
- pub `continuous_task` function L126-140 — `(args: TokenStream, input: TokenStream) -> TokenStream` — The continuous_task proc macro implementation.
-  `ContinuousTaskAttributes` type L40-123 — `impl Parse for ContinuousTaskAttributes` — as `#[task]` — `DataSourceMap` is injected via context by the scheduler.
-  `parse` function L41-122 — `(input: ParseStream) -> SynResult<Self>` — as `#[task]` — `DataSourceMap` is injected via context by the scheduler.
-  `generate_continuous_task` function L142-254 — `(attrs: &ContinuousTaskAttributes, fn_item: &ItemFn) -> TokenStream2` — as `#[task]` — `DataSourceMap` is injected via context by the scheduler.

#### crates/cloacina-macros/src/lib.rs

- pub `task` function L65-67 — `(args: TokenStream, input: TokenStream) -> TokenStream` — ```
- pub `continuous_task` function L70-72 — `(args: TokenStream, input: TokenStream) -> TokenStream` — ```
- pub `workflow` function L75-77 — `(input: TokenStream) -> TokenStream` — ```
- pub `packaged_workflow` function L80-82 — `(args: TokenStream, input: TokenStream) -> TokenStream` — ```
-  `continuous_task` module L56 — `-` — # Cloacina Macros
-  `packaged_workflow` module L57 — `-` — ```
-  `registry` module L58 — `-` — ```
-  `tasks` module L59 — `-` — ```
-  `workflow` module L60 — `-` — ```

#### crates/cloacina-macros/src/packaged_workflow.rs

- pub `TaskMetadata` struct L34-45 — `{ local_id: *const std::os::raw::c_char, namespaced_id_template: *const std::os:...` — C-compatible task metadata structure for FFI
- pub `TaskMetadataCollection` struct L55-64 — `{ task_count: u32, tasks: *const TaskMetadata, workflow_name: *const std::os::ra...` — C-compatible collection of task metadata for FFI
- pub `PackagedWorkflowAttributes` struct L79-85 — `{ name: String, package: String, tenant: String, description: Option<String>, au...` — Attributes for the packaged_workflow macro
- pub `detect_package_cycles` function L171-203 — `( task_dependencies: &HashMap<String, Vec<String>>, ) -> Result<(), String>` — Detect circular dependencies within a package's task dependencies
- pub `calculate_levenshtein_distance` function L273-308 — `(a: &str, b: &str) -> usize`
- pub `find_similar_package_task_names` function L320-333 — `(target: &str, available: &[String]) -> Vec<String>` — Find task names similar to the given name for typo suggestions in packaged workflows
- pub `build_package_graph_data` function L347-423 — `( detected_tasks: &HashMap<String, syn::Ident>, task_dependencies: &HashMap<Stri...` — Build graph data structure for a packaged workflow
- pub `generate_packaged_workflow_impl` function L497-1173 — `( attrs: PackagedWorkflowAttributes, input: ItemMod, ) -> TokenStream2` — Generate packaged workflow implementation
- pub `packaged_workflow` function L1213-1245 — `(args: TokenStream, input: TokenStream) -> TokenStream` — The packaged_workflow macro for creating distributable workflow packages
-  `TaskMetadata` type L48 — `impl Send for TaskMetadata`
-  `TaskMetadata` type L49 — `impl Sync for TaskMetadata`
-  `TaskMetadataCollection` type L67 — `impl Send for TaskMetadataCollection`
-  `TaskMetadataCollection` type L68 — `impl Sync for TaskMetadataCollection`
-  `PackagedWorkflowAttributes` type L87-155 — `impl Parse for PackagedWorkflowAttributes`
-  `parse` function L88-154 — `(input: ParseStream) -> SynResult<Self>`
-  `dfs_package_cycle_detection` function L219-257 — `( task_id: &str, task_dependencies: &HashMap<String, Vec<String>>, visited: &mut...` — Depth-first search implementation for package-level cycle detection
-  `calculate_max_depth` function L432-441 — `(task_dependencies: &HashMap<String, Vec<String>>) -> usize` — Calculate the maximum depth in the task dependency graph
-  `calculate_task_depth` function L452-477 — `( task_id: &str, task_dependencies: &HashMap<String, Vec<String>>, visited: &mut...` — Calculate the depth of a specific task in the dependency graph

#### crates/cloacina-macros/src/registry.rs

- pub `TaskInfo` struct L41-48 — `{ id: String, dependencies: Vec<String>, file_path: String }` — Information about a registered task
- pub `CompileTimeTaskRegistry` struct L53-58 — `{ tasks: HashMap<String, TaskInfo>, dependency_graph: HashMap<String, Vec<String...` — Registry that maintains task information and dependency relationships
- pub `new` function L62-67 — `() -> Self` — Creates a new empty task registry
- pub `register_task` function L77-97 — `(&mut self, task_info: TaskInfo) -> Result<(), CompileTimeError>` — Register a task in the compile-time registry
- pub `validate_dependencies` function L108-143 — `(&self, task_id: &str) -> Result<(), CompileTimeError>` — Validate that all dependencies for a task exist in the registry
- pub `validate_single_dependency` function L154-163 — `(&self, dependency: &str) -> Result<(), CompileTimeError>` — Validate that a single dependency exists in the registry
- pub `detect_cycles` function L170-194 — `(&self) -> Result<(), CompileTimeError>` — Detect circular dependencies in the task graph using Tarjan's algorithm
- pub `get_all_task_ids` function L250-252 — `(&self) -> Vec<String>` — Get all registered task IDs
- pub `clear` function L258-261 — `(&mut self)` — Clear the registry
- pub `size` function L265-267 — `(&self) -> usize` — Get the current number of registered tasks
- pub `CompileTimeError` enum L272-300 — `DuplicateTaskId | MissingDependency | CircularDependency | TaskNotFound` — Errors that can occur during compile-time task validation
- pub `to_compile_error` function L307-371 — `(&self) -> TokenStream` — Convert the error into a compile-time error token stream
- pub `get_registry` function L377-379 — `() -> &'static Lazy<Mutex<CompileTimeTaskRegistry>>` — Get the global compile-time registry instance
-  `COMPILE_TIME_TASK_REGISTRY` variable L36-37 — `: Lazy<Mutex<CompileTimeTaskRegistry>>` — Global compile-time registry instance for task tracking
-  `CompileTimeTaskRegistry` type L60-268 — `= CompileTimeTaskRegistry` — for thread-safe access during compilation.
-  `dfs_cycle_detection` function L207-242 — `( &self, task_id: &str, visited: &mut HashMap<String, bool>, rec_stack: &mut Has...` — Depth-first search implementation for cycle detection
-  `CompileTimeError` type L302-372 — `= CompileTimeError` — for thread-safe access during compilation.
-  `find_similar_task_names` function L391-404 — `(target: &str, available: &[String]) -> Vec<String>` — Find task names similar to the given name for typo suggestions
-  `levenshtein_distance` function L417-452 — `(a: &str, b: &str) -> usize` — Calculate the Levenshtein distance between two strings

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

#### crates/cloacina-macros/src/workflow.rs

- pub `WorkflowAttributes` struct L97-104 — `{ name: String, tenant: String, package: String, description: Option<String>, au...` — Workflow macro attributes
- pub `generate_workflow_impl` function L200-486 — `(attrs: WorkflowAttributes) -> TokenStream2` — Generate Workflow with auto-versioning and compile-time validation
- pub `workflow` function L503-519 — `(input: TokenStream) -> TokenStream` — The workflow! macro for declarative workflow definition
-  `rewrite_trigger_rules_with_namespace` function L31-85 — `( tenant: &str, package: &str, workflow_name: &str, ) -> TokenStream2` — Rewrite task names in trigger rules JSON to use full namespaces
-  `WorkflowAttributes` type L106-186 — `impl Parse for WorkflowAttributes`
-  `parse` function L107-185 — `(input: ParseStream) -> SynResult<Self>`

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

- pub `assertions` module L60 — `-` — # cloacina-testing
- pub `result` module L61 — `-` — types from `cloacina` (available once CLOACI-I-0023 lands).
- pub `runner` module L62 — `-` — types from `cloacina` (available once CLOACI-I-0023 lands).
- pub `boundary` module L65 — `-` — types from `cloacina` (available once CLOACI-I-0023 lands).
- pub `mock` module L67 — `-` — types from `cloacina` (available once CLOACI-I-0023 lands).

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

- pub `ContextError` enum L37-53 — `Serialization | KeyNotFound | TypeMismatch | KeyExists` — Errors that can occur during context operations.
- pub `TaskError` enum L60-102 — `ExecutionFailed | DependencyNotSatisfied | Timeout | ContextError | ValidationFa...` — Errors that can occur during task execution.
- pub `CheckpointError` enum L118-138 — `SaveFailed | LoadFailed | Serialization | StorageError | ValidationFailed` — Errors that can occur during task checkpointing.
-  `TaskError` type L104-111 — `= TaskError` — - [`CheckpointError`]: Errors in task checkpointing
-  `from` function L105-110 — `(error: ContextError) -> Self` — - [`CheckpointError`]: Errors in task checkpointing

#### crates/cloacina-workflow/src/lib.rs

- pub `context` module L68 — `-` — # Cloacina Workflow - Minimal Types for Workflow Authoring
- pub `error` module L69 — `-` — ```
- pub `namespace` module L70 — `-` — ```
- pub `retry` module L71 — `-` — ```
- pub `task` module L72 — `-` — ```

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
- pub `RetryPolicyBuilder` struct L313-315 — `{ policy: RetryPolicy }` — Builder for creating RetryPolicy instances with a fluent API.
- pub `new` function L319-323 — `() -> Self` — Creates a new RetryPolicyBuilder with default values.
- pub `max_attempts` function L326-329 — `(mut self, max_attempts: i32) -> Self` — Sets the maximum number of retry attempts.
- pub `backoff_strategy` function L332-335 — `(mut self, strategy: BackoffStrategy) -> Self` — Sets the backoff strategy.
- pub `initial_delay` function L338-341 — `(mut self, delay: Duration) -> Self` — Sets the initial delay before the first retry.
- pub `max_delay` function L344-347 — `(mut self, delay: Duration) -> Self` — Sets the maximum delay between retries.
- pub `with_jitter` function L350-353 — `(mut self, jitter: bool) -> Self` — Enables or disables jitter.
- pub `retry_condition` function L356-359 — `(mut self, condition: RetryCondition) -> Self` — Adds a retry condition.
- pub `retry_conditions` function L362-365 — `(mut self, conditions: Vec<RetryCondition>) -> Self` — Adds multiple retry conditions.
- pub `build` function L368-370 — `(self) -> RetryPolicy` — Builds the RetryPolicy.
-  `RetryPolicy` type L134-157 — `impl Default for RetryPolicy` — ```
-  `default` function L144-156 — `() -> Self` — Creates a default retry policy with reasonable production settings.
-  `RetryPolicy` type L159-309 — `= RetryPolicy` — ```
-  `add_jitter` function L257-262 — `(&self, delay: Duration) -> Duration` — Adds random jitter to a delay to prevent thundering herd problems.
-  `is_transient_error` function L265-308 — `(&self, error: &TaskError) -> bool` — Determines if an error is transient (network, timeout, temporary failures).
-  `RetryPolicyBuilder` type L317-371 — `= RetryPolicyBuilder` — ```
-  `RetryPolicyBuilder` type L373-377 — `impl Default for RetryPolicyBuilder` — ```
-  `default` function L374-376 — `() -> Self` — ```
-  `tests` module L380-473 — `-` — ```
-  `test_default_retry_policy` function L384-394 — `()` — ```
-  `test_retry_policy_builder` function L397-412 — `()` — ```
-  `test_fixed_backoff_calculation` function L415-425 — `()` — ```
-  `test_linear_backoff_calculation` function L428-438 — `()` — ```
-  `test_exponential_backoff_calculation` function L441-455 — `()` — ```
-  `test_max_delay_capping` function L458-472 — `()` — ```

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

### crates/cloacinactl

> *Semantic summary to be generated by AI agent.*

#### crates/cloacinactl/build.rs

-  `main` function L22-46 — `()` — Build script for cloacinactl.

### crates/cloacinactl/src/auth

> *Semantic summary to be generated by AI agent.*

#### crates/cloacinactl/src/auth/cache.rs

- pub `CachedKey` struct L27-38 — `{ key_hash: String, key_id: Uuid, tenant_id: Option<Uuid>, can_read: bool, can_w...` — Cached API key with pre-loaded permissions and workflow patterns.
- pub `AuthCache` struct L54-57 — `{ inner: Arc<RwLock<HashMap<String, CacheEntry>>>, ttl: Duration }` — In-memory auth cache with configurable TTL.
- pub `new` function L60-65 — `(ttl: Duration) -> Self` — In-memory auth cache with TTL for API key lookups.
- pub `lookup` function L68-87 — `(&self, prefix: &str) -> Option<Vec<CachedKey>>` — Look up cached keys by prefix.
- pub `insert` function L90-99 — `(&self, prefix: String, keys: Vec<CachedKey>)` — Insert found keys into cache.
- pub `insert_not_found` function L102-110 — `(&self, prefix: String)` — Insert negative cache entry (prefix not found in DB).
- pub `invalidate` function L113-116 — `(&self, prefix: &str)` — Invalidate a specific prefix (e.g., after key creation or revocation).
-  `CacheEntry` enum L42-50 — `Found | NotFound` — Cache entry: either found keys or negative cache.
-  `AuthCache` type L59-117 — `= AuthCache` — In-memory auth cache with TTL for API key lookups.
-  `tests` module L120-195 — `-` — In-memory auth cache with TTL for API key lookups.
-  `make_cached_key` function L123-136 — `(name: &str) -> CachedKey` — In-memory auth cache with TTL for API key lookups.
-  `test_insert_and_lookup` function L139-149 — `()` — In-memory auth cache with TTL for API key lookups.
-  `test_ttl_expiry` function L152-165 — `()` — In-memory auth cache with TTL for API key lookups.
-  `test_negative_cache` function L168-175 — `()` — In-memory auth cache with TTL for API key lookups.
-  `test_invalidation` function L178-188 — `()` — In-memory auth cache with TTL for API key lookups.
-  `test_miss_returns_none` function L191-194 — `()` — In-memory auth cache with TTL for API key lookups.

#### crates/cloacinactl/src/auth/context.rs

- pub `AuthContext` struct L23-31 — `{ key_id: Uuid, tenant_id: Option<Uuid>, can_read: bool, can_write: bool, can_ex...` — Authenticated request context, available via axum extensions.
- pub `is_global` function L35-37 — `(&self) -> bool` — Check if this key has global (super-admin) scope.
-  `AuthContext` type L33-38 — `= AuthContext` — Authentication context injected into request extensions.

#### crates/cloacinactl/src/auth/middleware.rs

- pub `AuthState` struct L32-35 — `{ cache: AuthCache, dal: Arc<cloacina::dal::DAL> }` — Auth middleware state, shared across requests.
- pub `auth_middleware` function L39-154 — `( axum::extract::State(auth_state): axum::extract::State<AuthState>, mut request...` — Middleware function for authentication.
- pub `Permission` enum L171-176 — `Read | Write | Execute | Admin` — Permission types for route-level authorization.
- pub `require_read` function L179-181 — `(request: Request, next: Next) -> Response` — Middleware function that checks the Read permission.
- pub `require_write` function L184-186 — `(request: Request, next: Next) -> Response` — Middleware function that checks the Write permission.
- pub `require_execute` function L189-191 — `(request: Request, next: Next) -> Response` — Middleware function that checks the Execute permission.
- pub `require_admin` function L194-196 — `(request: Request, next: Next) -> Response` — Middleware function that checks the Admin permission.
-  `extract_bearer_token` function L156-163 — `(request: &Request) -> Option<String>` — Authentication and authorization middleware for axum.
-  `check_permission` function L198-226 — `(request: Request, next: Next, required: Permission) -> Response` — Authentication and authorization middleware for axum.

#### crates/cloacinactl/src/auth/mod.rs

- pub `cache` module L17 — `-`
- pub `context` module L18 — `-`
- pub `middleware` module L19 — `-`
- pub `pattern` module L20 — `-`

#### crates/cloacinactl/src/auth/pattern.rs

- pub `check_workflow_access` function L25-30 — `(patterns: &[String], workflow_name: &str) -> bool` — Check if a workflow name matches any of the allowed patterns.
-  `glob_match` function L33-65 — `(pattern: &str, text: &str) -> bool` — Simple glob matching: `*` matches any sequence of characters.
-  `tests` module L68-122 — `-` — Glob pattern matching for workflow-level ABAC.
-  `test_empty_patterns_allows_all` function L72-74 — `()` — Glob pattern matching for workflow-level ABAC.
-  `test_exact_match` function L77-81 — `()` — Glob pattern matching for workflow-level ABAC.
-  `test_glob_star` function L84-89 — `()` — Glob pattern matching for workflow-level ABAC.
-  `test_multiple_patterns` function L92-97 — `()` — Glob pattern matching for workflow-level ABAC.
-  `test_star_matches_everything` function L100-103 — `()` — Glob pattern matching for workflow-level ABAC.
-  `test_no_match` function L106-109 — `()` — Glob pattern matching for workflow-level ABAC.
-  `test_glob_match_basic` function L112-121 — `()` — Glob pattern matching for workflow-level ABAC.

### crates/cloacinactl/src/commands

> *Semantic summary to be generated by AI agent.*

#### crates/cloacinactl/src/commands/api_key.rs

- pub `create` function L26-103 — `( dal: &DAL, tenant: Option<&str>, name: Option<&str>, read: bool, write: bool, ...` — Create a new API key.
- pub `list` function L106-178 — `(dal: &DAL, tenant: Option<&str>) -> Result<()>` — List API keys.
- pub `revoke` function L181-193 — `(dal: &DAL, key_id: &str) -> Result<()>` — Revoke an API key.
- pub `create_admin` function L196-198 — `(dal: &DAL, name: &str) -> Result<()>` — Create a global super-admin key (bootstrap command).

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

#### crates/cloacinactl/src/commands/key.rs

- pub `generate` function L26-43 — `(database_url: &str, org_id: &str, name: &str) -> Result<()>` — Generate a new signing keypair.
- pub `list` function L46-79 — `(database_url: &str, org_id: &str) -> Result<()>` — List signing keys for an organization.
- pub `export` function L82-105 — `(database_url: &str, key_id: &str, format: &str) -> Result<()>` — Export a public key for distribution.
- pub `revoke` function L108-121 — `(database_url: &str, key_id: &str) -> Result<()>` — Revoke a signing key.

#### crates/cloacinactl/src/commands/key_trust.rs

- pub `add` function L26-52 — `( database_url: &str, org_id: &str, key_file: &str, name: Option<&str>, ) -> Res...` — Add a trusted public key from a PEM file.
- pub `list` function L55-93 — `(database_url: &str, org_id: &str) -> Result<()>` — List trusted public keys for an organization.
- pub `revoke` function L96-109 — `(database_url: &str, key_id: &str) -> Result<()>` — Revoke a trusted public key.

#### crates/cloacinactl/src/commands/mod.rs

- pub `api_key` module L19 — `-` — cloacinactl command implementations.
- pub `cleanup_events` module L20 — `-` — cloacinactl command implementations.
- pub `key` module L21 — `-` — cloacinactl command implementations.
- pub `key_trust` module L22 — `-` — cloacinactl command implementations.
- pub `package` module L23 — `-` — cloacinactl command implementations.
- pub `serve` module L24 — `-` — cloacinactl command implementations.
- pub `connect_db` function L34-38 — `(database_url: &str) -> Result<DAL>` — Connect to the database and return a DAL instance.
- pub `read_master_key` function L41-56 — `() -> Result<[u8; 32]>` — Read the master encryption key from CLOACINA_MASTER_KEY env var (hex-encoded).
- pub `parse_uuid` function L59-62 — `(s: &str) -> Result<cloacina::database::universal_types::UniversalUuid>` — Parse a UUID string into a UniversalUuid.
-  `CLI_POOL_SIZE` variable L31 — `: u32` — Default connection pool size for CLI operations.

#### crates/cloacinactl/src/commands/package.rs

- pub `build` function L31-79 — `(output: &str, targets: &[String], dry_run: bool, verbose: bool) -> Result<()>` — Build a .cloacina package by calling into cloaca's Python build logic via PyO3.
- pub `sign` function L82-116 — `(database_url: &str, package: &str, key_id: &str, store: bool) -> Result<()>` — Sign a package and write a detached .sig file.
- pub `verify` function L119-192 — `( database_url: Option<&str>, org_id: Option<&str>, package: &str, signature_pat...` — Verify a package signature.
- pub `inspect` function L195-207 — `(signature_path: &str) -> Result<()>` — Inspect a detached signature file.

#### crates/cloacinactl/src/commands/serve.rs

- pub `ServeMode` enum L32-41 — `All | Api | Worker | Scheduler` — Server operational mode.
- pub `ServeArgs` struct L56-72 — `{ mode: ServeMode, config: Option<String>, bind: String, port: u16 }` — Arguments for the `serve` subcommand.
- pub `app` function L92-159 — `(state: Arc<AppState>) -> Router` — Build the axum Router with application state.
- pub `run` function L218-306 — `(args: &ServeArgs) -> Result<()>` — Run the serve command.
-  `ServeMode` type L43-52 — `= ServeMode` — `cloacinactl serve` command — starts the Cloacina server.
-  `fmt` function L44-51 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — `cloacinactl serve` command — starts the Cloacina server.
-  `ApiDoc` struct L87 — `-` — `cloacinactl serve` command — starts the Cloacina server.
-  `shutdown_signal` function L162-186 — `()` — Wait for a shutdown signal (SIGTERM or Ctrl+C).
-  `build_runner_config` function L189-215 — `( config: &ServerConfig, mode: ServeMode, ) -> cloacina::runner::DefaultRunnerCo...` — Build a DefaultRunnerConfig from the ServerConfig.
-  `tests` module L309-889 — `-` — `cloacinactl serve` command — starts the Cloacina server.
-  `test_serve_health_endpoint_lifecycle` function L314-372 — `()` — `cloacinactl serve` command — starts the Cloacina server.
-  `test_health_returns_correct_mode` function L375-404 — `()` — `cloacinactl serve` command — starts the Cloacina server.
-  `test_unknown_route_returns_404` function L407-435 — `()` — `cloacinactl serve` command — starts the Cloacina server.
-  `app_with_auth_cache` function L438-458 — `(cache: crate::auth::cache::AuthCache) -> (Router, Arc<AppState>)` — Helper: create an app with auth middleware using a pre-populated cache (no DB needed).
-  `test_auth_protected_endpoint_requires_auth` function L461-492 — `()` — `cloacinactl serve` command — starts the Cloacina server.
-  `test_auth_valid_key_returns_200` function L495-553 — `()` — `cloacinactl serve` command — starts the Cloacina server.
-  `test_auth_invalid_key_returns_401` function L556-587 — `()` — `cloacinactl serve` command — starts the Cloacina server.
-  `test_api_workflows_without_runner_returns_503` function L592-645 — `()` — `cloacinactl serve` command — starts the Cloacina server.
-  `test_api_executions_without_auth_returns_401` function L648-685 — `()` — `cloacinactl serve` command — starts the Cloacina server.
-  `test_api_error_format_consistency` function L688-752 — `()` — `cloacinactl serve` command — starts the Cloacina server.
-  `test_tenant_endpoints_require_admin` function L757-823 — `()` — `cloacinactl serve` command — starts the Cloacina server.
-  `test_tenant_list_without_dal_returns_503` function L826-888 — `()` — `cloacinactl serve` command — starts the Cloacina server.

### crates/cloacinactl/src

> *Semantic summary to be generated by AI agent.*

#### crates/cloacinactl/src/config.rs

- pub `ServerConfig` struct L28-34 — `{ server: ServerSection, database: DatabaseSection, scheduler: SchedulerSection,...` — Top-level server configuration.
- pub `ServerSection` struct L50-54 — `{ bind: String, port: u16, mode: String }` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
- pub `DatabaseSection` struct L68-71 — `{ url: String, pool_size: u32 }` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
- pub `SchedulerSection` struct L84-88 — `{ poll_interval_ms: u64, enable_continuous: bool, continuous_poll_interval_ms: u...` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
- pub `WorkerSection` struct L102-105 — `{ max_concurrent_tasks: usize, task_timeout_seconds: u64 }` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
- pub `LoggingSection` struct L118-121 — `{ level: String, format: String }` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
- pub `load_config` function L133-154 — `(cli_config_path: Option<&str>, cli_args: &ServeArgs) -> Result<ServerConfig>` — Load configuration with layered precedence: defaults → TOML → env vars → CLI flags.
-  `ServerConfig` type L36-46 — `impl Default for ServerConfig` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `default` function L37-45 — `() -> Self` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `ServerSection` type L56-64 — `impl Default for ServerSection` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `default` function L57-63 — `() -> Self` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `DatabaseSection` type L73-80 — `impl Default for DatabaseSection` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `default` function L74-79 — `() -> Self` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `SchedulerSection` type L90-98 — `impl Default for SchedulerSection` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `default` function L91-97 — `() -> Self` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `WorkerSection` type L107-114 — `impl Default for WorkerSection` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `default` function L108-113 — `() -> Self` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `LoggingSection` type L123-130 — `impl Default for LoggingSection` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `default` function L124-129 — `() -> Self` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `discover_config_file` function L157-178 — `(cli_path: Option<&str>) -> Option<String>` — Discover the config file path from CLI flag, CWD, or user config dir.
-  `apply_env_overrides` function L181-214 — `(config: &mut ServerConfig)` — Apply environment variable overrides with CLOACINA_ prefix.
-  `apply_cli_overrides` function L217-221 — `(config: &mut ServerConfig, args: &ServeArgs)` — Apply CLI flag overrides (highest precedence).
-  `tests` module L224-358 — `-` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `default_args` function L228-235 — `() -> ServeArgs` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `test_default_config` function L238-247 — `()` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `test_toml_parsing` function L250-281 — `()` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `test_partial_toml_uses_defaults` function L284-294 — `()` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `test_env_var_overlay` function L297-310 — `()` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `test_cli_overrides_take_precedence` function L313-330 — `()` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `test_load_config_no_file` function L333-338 — `()` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `test_missing_config_file_is_not_error` function L341-346 — `()` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.
-  `test_explicit_missing_config_file_is_error` function L349-357 — `()` — Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.

#### crates/cloacinactl/src/main.rs

- pub `auth` module L23 — `-` — cloacinactl - Control tool for the Cloacina task orchestration engine.
- pub `config` module L25 — `-` — cloacinactl - Control tool for the Cloacina task orchestration engine.
- pub `routes` module L26 — `-` — cloacinactl - Control tool for the Cloacina task orchestration engine.
-  `commands` module L24 — `-` — cloacinactl - Control tool for the Cloacina task orchestration engine.
-  `Cli` struct L32-47 — `{ database_url: Option<String>, org_id: Option<String>, verbose: bool, command: ...` — cloacinactl - Control tool for the Cloacina task orchestration engine
-  `Commands` enum L50-77 — `Serve | Package | Key | Admin | ApiKey` — cloacinactl - Control tool for the Cloacina task orchestration engine.
-  `PackageCommands` enum L80-129 — `Build | Sign | Verify | Inspect` — cloacinactl - Control tool for the Cloacina task orchestration engine.
-  `KeyCommands` enum L132-164 — `Generate | List | Export | Revoke | Trust` — cloacinactl - Control tool for the Cloacina task orchestration engine.
-  `TrustCommands` enum L167-186 — `Add | List | Revoke` — cloacinactl - Control tool for the Cloacina task orchestration engine.
-  `AdminCommands` enum L189-207 — `CleanupEvents | ContinuousPruneState` — cloacinactl - Control tool for the Cloacina task orchestration engine.
-  `ApiKeyCommands` enum L210-261 — `Create | List | Revoke | CreateAdmin` — cloacinactl - Control tool for the Cloacina task orchestration engine.
-  `main` function L264-455 — `() -> Result<()>` — cloacinactl - Control tool for the Cloacina task orchestration engine.

### crates/cloacinactl/src/routes

> *Semantic summary to be generated by AI agent.*

#### crates/cloacinactl/src/routes/auth_test.rs

- pub `AuthTestResponse` struct L30-39 — `{ key_id: String, tenant_id: Option<String>, can_read: bool, can_write: bool, ca...` — Response from /auth-test — echoes the authenticated context.
- pub `auth_test` function L42-55 — `(Extension(auth): Extension<AuthContext>) -> impl IntoResponse` — GET /auth-test — returns the authenticated context (protected endpoint).

#### crates/cloacinactl/src/routes/error.rs

- pub `ApiErrorBody` struct L25-27 — `{ error: ApiErrorDetail }` — Consistent API error responses.
- pub `ApiErrorDetail` struct L30-33 — `{ code: String, message: String }` — Consistent API error responses.
- pub `ApiError` struct L35-39 — `{ status: StatusCode, code: String, message: String }` — Consistent API error responses.
- pub `not_found` function L42-48 — `(message: impl Into<String>) -> Self` — Consistent API error responses.
- pub `bad_request` function L49-55 — `(message: impl Into<String>) -> Self` — Consistent API error responses.
- pub `internal` function L56-62 — `(message: impl Into<String>) -> Self` — Consistent API error responses.
- pub `service_unavailable` function L63-69 — `(message: impl Into<String>) -> Self` — Consistent API error responses.
- pub `conflict` function L70-76 — `(message: impl Into<String>) -> Self` — Consistent API error responses.
- pub `timeout` function L77-83 — `(message: impl Into<String>) -> Self` — Consistent API error responses.
-  `ApiError` type L41-84 — `= ApiError` — Consistent API error responses.
-  `ApiError` type L86-96 — `impl IntoResponse for ApiError` — Consistent API error responses.
-  `into_response` function L87-95 — `(self) -> Response` — Consistent API error responses.
-  `ApiError` type L99-113 — `= ApiError` — Consistent API error responses.
-  `from` function L100-112 — `(e: cloacina::executor::pipeline_executor::PipelineError) -> Self` — Consistent API error responses.

#### crates/cloacinactl/src/routes/executions.rs

- pub `ExecutionRequest` struct L38-42 — `{ workflow_name: String, context: serde_json::Value }` — Request body for triggering an execution.
- pub `ExecutionCreatedResponse` struct L46-49 — `{ execution_id: String, status: String }` — Response for execution creation.
- pub `ExecutionStatusResponse` struct L53-61 — `{ execution_id: String, workflow_name: String, status: String, started_at: Optio...` — Response for execution status.
- pub `TaskResultResponse` struct L64-68 — `{ task_name: String, status: String, error_message: Option<String> }` — Execution management endpoints.
- pub `create_execution` function L71-97 — `( State(state): State<Arc<AppState>>, Json(body): Json<ExecutionRequest>, ) -> R...` — POST /executions — trigger a workflow execution.
- pub `list_executions` function L100-129 — `( State(state): State<Arc<AppState>>, ) -> Result<impl IntoResponse, ApiError>` — GET /executions — list recent executions.
- pub `get_execution` function L132-164 — `( State(state): State<Arc<AppState>>, Path(id): Path<String>, ) -> Result<impl I...` — GET /executions/{id} — get execution status and result.
- pub `PauseRequest` struct L168-170 — `{ reason: Option<String> }` — Request body for pause (optional reason).
- pub `ControlResponse` struct L174-177 — `{ execution_id: String, status: String }` — Simple status response for control operations.
- pub `pause_execution` function L180-200 — `( State(state): State<Arc<AppState>>, Path(id): Path<String>, body: Option<Json<...` — POST /executions/{id}/pause
- pub `resume_execution` function L203-220 — `( State(state): State<Arc<AppState>>, Path(id): Path<String>, ) -> Result<impl I...` — POST /executions/{id}/resume
- pub `cancel_execution` function L223-240 — `( State(state): State<Arc<AppState>>, Path(id): Path<String>, ) -> Result<impl I...` — DELETE /executions/{id}
-  `require_runner` function L30-34 — `(state: &AppState) -> Result<&cloacina::runner::DefaultRunner, ApiError>` — Execution management endpoints.

#### crates/cloacinactl/src/routes/health.rs

- pub `AppState` struct L29-38 — `{ startup_instant: Instant, mode: String, auth_state: Option<crate::auth::middle...` — Shared application state available to all handlers.
- pub `HealthResponse` struct L42-51 — `{ status: String, version: String, mode: String, uptime_seconds: u64 }` — Health check response body.
- pub `health` function L63-74 — `(State(state): State<Arc<AppState>>) -> impl IntoResponse` — Health check endpoint.

#### crates/cloacinactl/src/routes/mod.rs

- pub `auth_test` module L19 — `-` — HTTP route handlers for the Cloacina server.
- pub `error` module L20 — `-` — HTTP route handlers for the Cloacina server.
- pub `executions` module L21 — `-` — HTTP route handlers for the Cloacina server.
- pub `health` module L22 — `-` — HTTP route handlers for the Cloacina server.
- pub `tenants` module L23 — `-` — HTTP route handlers for the Cloacina server.
- pub `workflows` module L24 — `-` — HTTP route handlers for the Cloacina server.

#### crates/cloacinactl/src/routes/tenants.rs

- pub `CreateTenantRequest` struct L50-53 — `{ name: String, schema_name: String }` — Request body for creating a tenant.
- pub `CreateTenantResponse` struct L57-62 — `{ id: String, name: String, schema_name: String, initial_api_key: String }` — Response for tenant creation (includes the initial admin API key).
- pub `TenantResponse` struct L66-73 — `{ id: String, name: String, schema_name: String, status: String, created_at: Str...` — Response for a single tenant.
- pub `StatusResponse` struct L77-79 — `{ status: String }` — Simple status response.
- pub `CreateApiKeyRequest` struct L83-95 — `{ name: String, read: bool, write: bool, execute: bool, admin: bool, patterns: V...` — Request body for creating an API key.
- pub `CreateApiKeyResponse` struct L99-103 — `{ id: String, secret: String, prefix: String }` — Response for API key creation (secret shown once).
- pub `ApiKeyMetadataResponse` struct L107-117 — `{ id: String, name: Option<String>, key_prefix: String, can_read: bool, can_writ...` — Response for an API key (metadata only, no secret or hash).
- pub `create_tenant` function L124-189 — `( State(state): State<Arc<AppState>>, Extension(auth): Extension<AuthContext>, J...` — POST /tenants -- create a new tenant with an initial admin API key.
- pub `list_tenants` function L192-224 — `( State(state): State<Arc<AppState>>, Extension(auth): Extension<AuthContext>, )...` — GET /tenants -- list all active tenants.
- pub `get_tenant` function L227-272 — `( State(state): State<Arc<AppState>>, Extension(auth): Extension<AuthContext>, P...` — GET /tenants/{id} -- get a single tenant by ID.
- pub `deactivate_tenant` function L275-302 — `( State(state): State<Arc<AppState>>, Extension(auth): Extension<AuthContext>, P...` — DELETE /tenants/{id} -- soft-deactivate a tenant.
- pub `create_tenant_key` function L309-389 — `( State(state): State<Arc<AppState>>, Extension(auth): Extension<AuthContext>, P...` — POST /tenants/{id}/api-keys -- create a new API key for a tenant.
- pub `list_tenant_keys` function L392-431 — `( State(state): State<Arc<AppState>>, Extension(auth): Extension<AuthContext>, P...` — GET /tenants/{id}/api-keys -- list API keys for a tenant (metadata only).
- pub `revoke_tenant_key` function L434-468 — `( State(state): State<Arc<AppState>>, Extension(auth): Extension<AuthContext>, P...` — DELETE /tenants/{id}/api-keys/{key_id} -- revoke an API key.
-  `require_dal` function L36-42 — `(state: &AppState) -> Result<&Arc<DAL>, ApiError>` — Get the DAL from the AppState's auth_state.

#### crates/cloacinactl/src/routes/workflows.rs

- pub `PackageUploadResponse` struct L38-41 — `{ id: String, message: String }` — Response for package upload.
- pub `upload_package` function L44-81 — `( State(state): State<Arc<AppState>>, mut multipart: Multipart, ) -> Result<impl...` — POST /workflows/packages — upload a workflow package.
- pub `WorkflowListItem` struct L85-90 — `{ name: String, version: String, description: Option<String>, tasks: Vec<String>...` — Response for workflow list.
- pub `list_workflows` function L93-105 — `( State(state): State<Arc<AppState>>, ) -> Result<impl IntoResponse, ApiError>` — GET /workflows — list registered workflows.
-  `require_runner` function L30-34 — `(state: &AppState) -> Result<&cloacina::runner::DefaultRunner, ApiError>` — Helper to get runner or return 503.

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

### examples/features/complex-dag/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/complex-dag/src/lib.rs

-  `complex_dag_workflow` module L35-213 — `-` — - Complex branching and merging
-  `init_config` function L43-47 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `init_database` function L50-54 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `init_logging` function L57-61 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `load_schema` function L68-72 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `setup_security` function L75-79 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `configure_monitoring` function L82-88 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — - Complex branching and merging
-  `create_tables` function L95-99 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `setup_cache` function L102-106 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `load_raw_data` function L113-117 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `validate_data` function L120-124 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `clean_data` function L127-131 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `transform_customers` function L138-144 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — - Complex branching and merging
-  `transform_orders` function L147-151 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `transform_products` function L154-158 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `calculate_metrics` function L165-169 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `generate_insights` function L172-176 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `build_dashboard` function L183-187 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `generate_reports` function L190-194 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `send_notifications` function L201-205 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging
-  `cleanup_staging` function L208-212 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Complex branching and merging

### examples/features/continuous-scheduling/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/continuous-scheduling/src/main.rs

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

### examples/features/cron-scheduling/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/cron-scheduling/src/main.rs

-  `tasks` module L47 — `-` — - Recovery service for missed executions
-  `main` function L51-112 — `() -> Result<(), Box<dyn std::error::Error>>` — - Recovery service for missed executions
-  `create_data_backup_workflow` function L115-128 — `() -> Result<cloacina::Workflow, Box<dyn std::error::Error>>` — Create the data backup workflow that runs every 30 minutes
-  `create_health_check_workflow` function L131-144 — `() -> Result<cloacina::Workflow, Box<dyn std::error::Error>>` — Create the health check workflow that runs every 5 minutes
-  `create_daily_report_workflow` function L147-159 — `() -> Result<cloacina::Workflow, Box<dyn std::error::Error>>` — Create the daily report workflow that runs once per day
-  `create_cron_schedules` function L162-203 — `(runner: &DefaultRunner) -> Result<(), Box<dyn std::error::Error>>` — Create cron schedules for our workflows
-  `show_execution_stats` function L206-218 — `(runner: &DefaultRunner) -> Result<(), Box<dyn std::error::Error>>` — Display execution statistics

#### examples/features/cron-scheduling/src/tasks.rs

- pub `check_backup_prerequisites` function L38-51 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `create_backup_snapshot` function L61-76 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `verify_backup_integrity` function L86-114 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `cleanup_old_backups` function L124-135 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `check_system_resources` function L149-176 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `check_database_connectivity` function L186-210 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `check_external_services` function L220-251 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `update_health_metrics` function L261-305 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `collect_daily_metrics` function L319-338 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `generate_usage_report` function L348-380 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `send_report_notification` function L390-420 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.

### examples/features/deferred-tasks/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/deferred-tasks/src/main.rs

-  `wait_for_data` function L58-97 — `( context: &mut Context<serde_json::Value>, handle: &mut TaskHandle, ) -> Result...` — Simulates waiting for external data to become available.
-  `process_data` function L101-120 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Processes data that was fetched by the deferred task.
-  `main` function L123-161 — `() -> Result<(), Box<dyn std::error::Error>>` — ```

### examples/features/event-triggers/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/event-triggers/src/main.rs

-  `tasks` module L50 — `-` — ```
-  `triggers` module L51 — `-` — ```
-  `main` function L57-132 — `() -> Result<(), Box<dyn std::error::Error>>` — ```
-  `create_file_processing_workflow` function L135-147 — `() -> Result<cloacina::Workflow, Box<dyn std::error::Error>>` — Create the file processing workflow triggered by file watcher.
-  `create_queue_processing_workflow` function L150-162 — `() -> Result<cloacina::Workflow, Box<dyn std::error::Error>>` — Create the queue processing workflow triggered by queue depth.
-  `create_service_recovery_workflow` function L165-178 — `() -> Result<cloacina::Workflow, Box<dyn std::error::Error>>` — Create the service recovery workflow triggered by health check failures.
-  `register_triggers` function L181-196 — `()` — Register triggers in the global trigger registry.
-  `register_trigger_schedules` function L199-265 — `( runner: &DefaultRunner, ) -> Result<(), Box<dyn std::error::Error>>` — Register trigger schedules with the runner (persists configuration to DB).

#### examples/features/event-triggers/src/tasks.rs

- pub `validate_file` function L32-51 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Validates and parses an incoming file.
- pub `process_file` function L55-76 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Processes the validated file data.
- pub `archive_file` function L80-97 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Archives the processed file.
- pub `drain_queue` function L105-128 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Drains messages from the queue.
- pub `process_messages` function L132-148 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Processes the drained messages.
- pub `ack_messages` function L152-170 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Acknowledges processed messages.
- pub `diagnose_failure` function L178-202 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Diagnoses the service failure.
- pub `restart_service` function L206-223 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Attempts to restart the service.
- pub `verify_recovery` function L227-247 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Verifies service health after restart.
- pub `notify_incident` function L251-275 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Sends notification about the incident.

#### examples/features/event-triggers/src/triggers.rs

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

### examples/features/multi-tenant/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/multi-tenant/src/main.rs

-  `main` function L28-50 — `() -> Result<(), Box<dyn std::error::Error>>` — with PostgreSQL schema-based isolation.
-  `demonstrate_multi_tenant_setup` function L52-82 — `(database_url: &str) -> Result<(), PipelineError>` — with PostgreSQL schema-based isolation.
-  `demonstrate_recovery_scenarios` function L85-123 — `(database_url: &str) -> Result<(), PipelineError>` — Demonstrates recovery scenarios for multi-tenant systems

### examples/features/packaged-workflows/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/packaged-workflows/src/lib.rs

- pub `analytics_workflow` module L55-285 — `-`
- pub `extract_data` function L68-95 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
- pub `validate_data` function L107-151 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
- pub `transform_data` function L163-217 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
- pub `generate_reports` function L229-284 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>`

### examples/features/per-tenant-credentials/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/per-tenant-credentials/src/main.rs

-  `main` function L28-50 — `() -> Result<(), Box<dyn std::error::Error>>` — isolated tenant users with their own database credentials and schemas.
-  `demonstrate_admin_tenant_creation` function L52-122 — `( admin_database_url: &str, ) -> Result<(), Box<dyn std::error::Error>>` — isolated tenant users with their own database credentials and schemas.
-  `demonstrate_tenant_isolation` function L124-182 — `( admin_database_url: &str, ) -> Result<(), Box<dyn std::error::Error>>` — isolated tenant users with their own database credentials and schemas.
-  `mask_password` function L185-196 — `(connection_string: &str) -> String` — Masks passwords in connection strings for safe logging

### examples/features/python-workflow

> *Semantic summary to be generated by AI agent.*

#### examples/features/python-workflow/run_pipeline.py

- pub `check` function L34-40 — `def check(condition: bool, msg: str) -> None`

### examples/features/registry-execution/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/registry-execution/src/main.rs

-  `main` function L52-268 — `() -> Result<(), Box<dyn std::error::Error>>`
-  `build_package` function L270-290 — `() -> Result<Vec<u8>, Box<dyn std::error::Error>>`
-  `find_workspace_root` function L292-305 — `() -> Result<PathBuf, Box<dyn std::error::Error>>`

### examples/features/simple-packaged/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/simple-packaged/src/lib.rs

- pub `data_processing` module L54-147 — `-`
- pub `collect_data` function L63-78 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
- pub `process_data` function L86-109 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>`
- pub `generate_report` function L117-146 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>`
-  `tests` module L150-169 — `-`
-  `test_workflow_execution` function L154-168 — `()`

### examples/features/simple-packaged/tests

> *Semantic summary to be generated by AI agent.*

#### examples/features/simple-packaged/tests/ffi_tests.rs

-  `test_workflow_creation_directly` function L25-38 — `()` — Tests for the FFI functions generated by the packaged_workflow macro.
-  `test_get_task_metadata_integration` function L41-64 — `()` — Tests for the FFI functions generated by the packaged_workflow macro.
-  `test_metadata_functions` function L67-82 — `()` — Tests for the FFI functions generated by the packaged_workflow macro.

#### examples/features/simple-packaged/tests/host_managed_registry_tests.rs

-  `test_get_task_metadata_basic` function L27-56 — `()` — Tests for the new host-managed registry approach using the get_task_metadata() FFI function.
-  `test_get_task_metadata_task_details` function L59-126 — `()` — Tests for the new host-managed registry approach using the get_task_metadata() FFI function.
-  `test_task_metadata_memory_safety` function L129-148 — `()` — Tests for the new host-managed registry approach using the get_task_metadata() FFI function.

### examples/features/validation-failures/src

> *Semantic summary to be generated by AI agent.*

#### examples/features/validation-failures/src/circular_dependency.rs

-  `task_a` function L26-29 — `(_context: &mut Context<Value>) -> Result<(), TaskError>`
-  `task_b` function L33-36 — `(_context: &mut Context<Value>) -> Result<(), TaskError>`
-  `main` function L39-48 — `() -> Result<(), Box<dyn std::error::Error>>`

#### examples/features/validation-failures/src/duplicate_task_ids.rs

-  `task_one` function L26-29 — `(_context: &mut Context<Value>) -> Result<(), TaskError>`
-  `task_two` function L33-36 — `(_context: &mut Context<Value>) -> Result<(), TaskError>`
-  `main` function L39-48 — `() -> Result<(), Box<dyn std::error::Error>>`

#### examples/features/validation-failures/src/missing_dependency.rs

-  `valid_task` function L25-28 — `(_context: &mut Context<Value>) -> Result<(), TaskError>`
-  `invalid_task` function L32-35 — `(_context: &mut Context<Value>) -> Result<(), TaskError>`
-  `main` function L38-47 — `() -> Result<(), Box<dyn std::error::Error>>`

#### examples/features/validation-failures/src/missing_workflow_task.rs

-  `existing_task` function L25-28 — `(_context: &mut Context<Value>) -> Result<(), TaskError>`
-  `main` function L30-39 — `()`

### examples/performance/parallel/src

> *Semantic summary to be generated by AI agent.*

#### examples/performance/parallel/src/main.rs

-  `Args` struct L31-39 — `{ iterations: usize, concurrency: usize }` — Based on tutorial-03, this measures throughput of parallel 5-task fan-out/fan-in workflows.
-  `setup_data` function L46-52 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Based on tutorial-03, this measures throughput of parallel 5-task fan-out/fan-in workflows.
-  `process_batch_1` function L59-72 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Based on tutorial-03, this measures throughput of parallel 5-task fan-out/fan-in workflows.
-  `process_batch_2` function L79-92 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Based on tutorial-03, this measures throughput of parallel 5-task fan-out/fan-in workflows.
-  `process_batch_3` function L99-112 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Based on tutorial-03, this measures throughput of parallel 5-task fan-out/fan-in workflows.
-  `merge_results` function L119-142 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Based on tutorial-03, this measures throughput of parallel 5-task fan-out/fan-in workflows.
-  `main` function L145-245 — `() -> Result<(), Box<dyn std::error::Error>>` — Based on tutorial-03, this measures throughput of parallel 5-task fan-out/fan-in workflows.

### examples/performance/pipeline/src

> *Semantic summary to be generated by AI agent.*

#### examples/performance/pipeline/src/main.rs

-  `Args` struct L31-39 — `{ iterations: usize, concurrency: usize }` — Based on tutorial-02, this measures throughput of sequential 3-task pipelines.
-  `extract_numbers` function L46-50 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Based on tutorial-02, this measures throughput of sequential 3-task pipelines.
-  `transform_numbers` function L57-69 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Based on tutorial-02, this measures throughput of sequential 3-task pipelines.
-  `load_numbers` function L76-86 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Based on tutorial-02, this measures throughput of sequential 3-task pipelines.
-  `main` function L89-187 — `() -> Result<(), Box<dyn std::error::Error>>` — Based on tutorial-02, this measures throughput of sequential 3-task pipelines.

### examples/performance/simple/src

> *Semantic summary to be generated by AI agent.*

#### examples/performance/simple/src/main.rs

-  `Args` struct L31-39 — `{ iterations: usize, concurrency: usize }` — Based on tutorial-01, this measures throughput of simple single-task workflows.
-  `hello_world` function L46-50 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — Based on tutorial-01, this measures throughput of simple single-task workflows.
-  `main` function L53-146 — `() -> Result<(), Box<dyn std::error::Error>>` — Based on tutorial-01, this measures throughput of simple single-task workflows.

### examples/tutorials/01-basic-workflow/src

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/01-basic-workflow/src/main.rs

-  `hello_world` function L33-39 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — This example demonstrates the most basic usage of Cloacina with a single task.
-  `main` function L42-92 — `() -> Result<(), Box<dyn std::error::Error>>` — This example demonstrates the most basic usage of Cloacina with a single task.

### examples/tutorials/02-multi-task/src

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/02-multi-task/src/main.rs

-  `tasks` module L49 — `-` — - Different retry policies for different task types
-  `main` function L54-105 — `() -> Result<(), Box<dyn std::error::Error>>` — - Different retry policies for different task types
-  `create_etl_workflow` function L108-120 — `() -> Result<cloacina::Workflow, Box<dyn std::error::Error>>` — Create the ETL workflow

#### examples/tutorials/02-multi-task/src/tasks.rs

- pub `extract_numbers` function L36-55 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — - Load: Store the transformed numbers
- pub `transform_numbers` function L65-91 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — - Load: Store the transformed numbers
- pub `load_numbers` function L101-122 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — - Load: Store the transformed numbers

### examples/tutorials/03-dependencies/src

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/03-dependencies/src/main.rs

-  `Product` struct L57-63 — `{ id: u32, name: String, category: String, price: f64, stock: u32 }` — - **Final Convergence**: All processing completes before cleanup
-  `CategoryStats` struct L66-70 — `{ total_value: f64, total_stock: u32, product_count: u32 }` — - **Final Convergence**: All processing completes before cleanup
-  `generate_data` function L78-100 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - **Final Convergence**: All processing completes before cleanup
-  `partition_data` function L108-141 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - **Final Convergence**: All processing completes before cleanup
-  `process_partition_1` function L150-197 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - **Final Convergence**: All processing completes before cleanup
-  `process_partition_2` function L206-253 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - **Final Convergence**: All processing completes before cleanup
-  `process_partition_3` function L262-309 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - **Final Convergence**: All processing completes before cleanup
-  `combine_results` function L317-443 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - **Final Convergence**: All processing completes before cleanup
-  `generate_report` function L451-484 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - **Final Convergence**: All processing completes before cleanup
-  `send_notifications` function L492-520 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - **Final Convergence**: All processing completes before cleanup
-  `cleanup` function L528-531 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - **Final Convergence**: All processing completes before cleanup
-  `main` function L534-581 — `() -> Result<(), Box<dyn std::error::Error>>` — - **Final Convergence**: All processing completes before cleanup

### examples/tutorials/04-error-handling/src

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/04-error-handling/src/main.rs

-  `on_task_success` function L44-54 — `( task_id: &str, _context: &Context<serde_json::Value>, ) -> Result<(), Box<dyn ...` — Called when a task completes successfully.
-  `on_task_failure` function L58-72 — `( task_id: &str, error: &cloacina::cloacina_workflow::TaskError, _context: &Cont...` — Called when a task fails (after all retries are exhausted).
-  `on_data_fetch_failure` function L75-86 — `( task_id: &str, error: &cloacina::cloacina_workflow::TaskError, _context: &Cont...` — Specific callback for critical data operations
-  `fetch_data` function L98-131 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Monitoring task execution outcomes
-  `cached_data` function L139-159 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Monitoring task execution outcomes
-  `process_data` function L169-203 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Monitoring task execution outcomes
-  `high_quality_processing` function L214-243 — `( context: &mut Context<serde_json::Value>, ) -> Result<(), TaskError>` — - Monitoring task execution outcomes
-  `low_quality_processing` function L254-281 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Monitoring task execution outcomes
-  `failure_notification` function L292-306 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Monitoring task execution outcomes
-  `final_report` function L319-340 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — - Monitoring task execution outcomes
-  `main` function L343-429 — `() -> Result<(), Box<dyn std::error::Error>>` — - Monitoring task execution outcomes

### examples/tutorials/05-advanced/src

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/05-advanced/src/main.rs

-  `tasks` module L47 — `-` — - Recovery service for missed executions
-  `main` function L51-111 — `() -> Result<(), Box<dyn std::error::Error>>` — - Recovery service for missed executions
-  `create_data_backup_workflow` function L114-127 — `() -> Result<cloacina::Workflow, Box<dyn std::error::Error>>` — Create the data backup workflow that runs every 30 minutes
-  `create_health_check_workflow` function L130-143 — `() -> Result<cloacina::Workflow, Box<dyn std::error::Error>>` — Create the health check workflow that runs every 5 minutes
-  `create_daily_report_workflow` function L146-158 — `() -> Result<cloacina::Workflow, Box<dyn std::error::Error>>` — Create the daily report workflow that runs once per day
-  `create_cron_schedules` function L161-202 — `(runner: &DefaultRunner) -> Result<(), Box<dyn std::error::Error>>` — Create cron schedules for our workflows
-  `show_execution_stats` function L205-217 — `(runner: &DefaultRunner) -> Result<(), Box<dyn std::error::Error>>` — Display execution statistics

#### examples/tutorials/05-advanced/src/tasks.rs

- pub `check_backup_prerequisites` function L38-51 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `create_backup_snapshot` function L61-76 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `verify_backup_integrity` function L86-114 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `cleanup_old_backups` function L124-135 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `check_system_resources` function L149-176 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `check_database_connectivity` function L186-210 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `check_external_services` function L220-251 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `update_health_metrics` function L261-305 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `collect_daily_metrics` function L319-338 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `generate_usage_report` function L348-380 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.
- pub `send_report_notification` function L390-420 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — on a schedule, including data backup, health checks, and reporting tasks.

### examples/tutorials/06-multi-tenancy/src

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/06-multi-tenancy/src/main.rs

-  `process_customer_data` function L35-69 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — using PostgreSQL schema-based multi-tenancy and the Database Admin API.
-  `tenant_onboarding` function L75-124 — `(context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` — using PostgreSQL schema-based multi-tenancy and the Database Admin API.
-  `main` function L127-155 — `() -> Result<(), Box<dyn std::error::Error>>` — using PostgreSQL schema-based multi-tenancy and the Database Admin API.
-  `basic_multi_tenant_demo` function L157-219 — `(database_url: &str) -> Result<(), Box<dyn std::error::Error>>` — using PostgreSQL schema-based multi-tenancy and the Database Admin API.
-  `advanced_admin_demo` function L221-288 — `(admin_database_url: &str) -> Result<(), Box<dyn std::error::Error>>` — using PostgreSQL schema-based multi-tenancy and the Database Admin API.

### examples/tutorials/python

> *Semantic summary to be generated by AI agent.*

#### examples/tutorials/python/04_error_handling.py

- pub `UnreliableExternalService` class L31-56 — `{ __init__, fetch_data }` — Simulates an external service with configurable failure rates.
- pub `__init__` method L34-36 — `def __init__(self, failure_rate=0.3)`
- pub `fetch_data` method L38-56 — `def fetch_data(self, data_id)` — Fetch data with potential for failure.

#### examples/tutorials/python/05_cron_scheduling.py

- pub `get_workflow_names` function L112-116 — `def get_workflow_names()` — Get all registered workflow names.
- pub `cron_demo` function L118-169 — `def cron_demo()` — Demonstrate advanced cron scheduling patterns.
- pub `main` function L171-190 — `def main()` — Main tutorial demonstration.

#### examples/tutorials/python/06_multi_tenancy.py

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

#### examples/tutorials/python/07_event_triggers.py

- pub `on_task_success` function L23-25 — `def on_task_success(task_id, context)` — Callback called when a task completes successfully.
- pub `on_task_failure` function L28-30 — `def on_task_failure(task_id, error, context)` — Callback called when a task fails.
- pub `demo_callbacks` function L137-156 — `def demo_callbacks()` — Demonstrate task callbacks.
- pub `demo_trigger_definition` function L159-193 — `def demo_trigger_definition()` — Demonstrate trigger definition and TriggerResult usage.
- pub `demo_trigger_management` function L196-221 — `def demo_trigger_management()` — Demonstrate trigger management through Python API.
- pub `demo_concepts` function L224-256 — `def demo_concepts()` — Explain key concepts.
- pub `main` function L259-286 — `def main()` — Main tutorial demonstration.

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
