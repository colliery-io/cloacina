# Cloacina Python Bindings

Python bindings for the Cloacina workflow orchestration framework, built with PyO3.

## Implementation Status

### ✅ Phase 1 Complete (Foundation)

**Core Components:**
- **PythonTask**: Rust Task implementation that wraps Python callables
- **Task Registration**: Global registry with `@task` decorator support
- **PyWorkflow**: Thin wrapper around Rust Workflow that auto-includes registered tasks
- **PyUnifiedExecutor**: Wrapper around Rust UnifiedExecutor with async methods
- **Context Conversion**: Bidirectional conversion between Python objects and Rust Context
- **Error Handling**: Proper error conversion between Rust and Python
- **Package Structure**: Complete Python package with proper `__init__.py`

**Target API Implemented:**
```python
from cloacina import task, Workflow, UnifiedExecutor

@task(id="extract_data", dependencies=[])
def extract_data(context):
    context["raw_data"] = {"users": [1, 2, 3]}
    return context

@task(id="transform_data", dependencies=["extract_data"])
def transform_data(context):
    raw = context.get("raw_data", {})
    context["transformed_data"] = {"processed": raw}
    return context

# Create workflow with all registered tasks
workflow = Workflow("my_pipeline")

# Execute (placeholder implementation)
executor = UnifiedExecutor()
await executor.initialize()
await executor.execute(workflow)
await executor.shutdown()
```

### ⏳ Phase 2 & 3 (Next Steps)

**Async Integration:**
- Full async Python function execution in Rust tasks
- Proper integration with Rust async runtime
- Context passing and dependency resolution

**Execution Engine:**
- Complete pipeline execution implementation
- Integration with UnifiedExecutor's actual API
- Task state management and monitoring

**Advanced Features:**
- Error handling and retry policies
- Trigger rules and conditional execution
- Checkpointing and recovery

**Distribution:**
- Maturin build system configuration
- PyPI package distribution
- CI/CD integration

## Architecture

The implementation follows the principle of **thin wrappers** around existing Rust code:

1. **PythonTask** implements the existing `Task` trait, bridging Python callables to Rust
2. **Context conversion utilities** handle data serialization between Python and Rust
3. **Workflow and Executor wrappers** expose existing Rust orchestration logic
4. **Registration system** manages Python tasks in a global registry

This approach leverages all existing battle-tested Rust code rather than reimplementing orchestration logic.

## Key Design Decisions

- **Simplicity**: Minimal wrapper approach rather than complex reimplementation
- **Reuse**: Leverage existing Rust Task trait, Workflow, and UnifiedExecutor
- **Registration**: Global task registry for automatic workflow inclusion
- **Async-Ready**: Foundation in place for full async integration in Phase 3

## File Structure

```
cloacina-python/
├── Cargo.toml              # Rust crate configuration
├── pyproject.toml          # Python package configuration
├── src/
│   ├── lib.rs              # PyO3 module exports
│   ├── task.rs             # PythonTask and registration
│   ├── workflow.rs         # PyWorkflow wrapper
│   ├── executor.rs         # PyUnifiedExecutor wrapper
│   ├── context.rs          # Context conversion utilities
│   └── error.rs            # Error type conversions
└── python/cloacina/
    └── __init__.py         # Python package exports
```

## Building

```bash
cargo check --package cloacina-python  # Verify Rust compilation
```

For full build and distribution:
```bash
pip install maturin
maturin develop  # Development build
maturin build    # Production build
```

The foundation is now complete for full Python integration with Cloacina!
