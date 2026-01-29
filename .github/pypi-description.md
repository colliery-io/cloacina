# Cloaca - Python bindings for Cloacina

**Cloaca** provides Python bindings for [Cloacina](https://github.com/colliery-io/cloacina), a high-performance workflow orchestration engine written in Rust.

## Quick Start

Install with your preferred backend:

```bash
# For PostgreSQL
pip install cloaca[postgres]

# For SQLite
pip install cloaca[sqlite]
```

## Simple Example

```python
from cloaca import task, workflow, Context

@task(id="hello_task")
def hello_world(context: Context) -> Context:
    context.set("message", "Hello from Cloaca!")
    return context

# Create and run workflow
with DefaultRunner("sqlite:///workflow.db") as runner:
    result = runner.execute("my_workflow", Context({"input": "data"}))
    print(result.context.get("message"))
```

## Features

- **High Performance**: Rust-powered execution engine with Python-friendly APIs
- **Multiple Backends**: Support for both PostgreSQL and SQLite
- **Async Support**: Native async/await patterns throughout
- **Type Safety**: Full type hints for excellent IDE support
- **Cron Scheduling**: Built-in support for scheduled workflows
- **Error Recovery**: Comprehensive retry policies and error handling
- **Multi-tenant**: Isolated execution contexts for different tenants

## Architecture

Cloaca uses a dispatcher pattern with separate backend packages:

- `cloaca` - Pure Python dispatcher that loads the appropriate backend
- `cloaca-postgres` - PostgreSQL backend (Rust extension via PyO3)
- `cloaca-sqlite` - SQLite backend (Rust extension via PyO3)

This design ensures you only install the dependencies you need while providing a unified API.

## Documentation

- [Full Documentation](https://cloacina.dev)
- [API Reference](https://docs.rs/cloacina)
- [Examples](https://github.com/colliery-io/cloacina/tree/main/examples)
- [Tutorial](https://cloacina.dev/tutorials)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE) or http://opensource.org/licenses/MIT)

at your option.
