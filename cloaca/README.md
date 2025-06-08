# Cloaca

Python bindings for Cloacina - a robust workflow orchestration engine.

This is the dispatcher package that automatically selects and loads the appropriate backend (PostgreSQL or SQLite) based on availability.

## Installation

```bash
# For PostgreSQL backend
pip install cloaca[postgres]

# For SQLite backend
pip install cloaca[sqlite]
```

## Usage

```python
import cloaca

# The backend is automatically detected
print(f"Using backend: {cloaca.get_backend()}")
print(cloaca.hello_world())
```
