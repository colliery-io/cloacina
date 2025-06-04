# Python Bindings Implementation Checklist

## Adding ANY New Symbol to Python (Classes, Functions, Constants, Enums, etc.)

When adding ANY new symbol (class, function, constant, enum, etc.) to the Rust backend, you MUST update ALL of these locations or the symbol won't be properly exposed to Python users:

**TL;DR: 4 places to update for EVERY new symbol: Rust → Python wrapper template → Dispatcher → Tests**

### 1. Rust Implementation 
**File**: `./cloaca-backend/src/lib.rs` (relative to project root)

```rust
// Examples of different symbol types:

// Class
#[pyclass]
pub struct YourClass { /* fields */ }

// Function  
#[pyfunction]
fn your_function() -> String { "result".to_string() }

// Enum
#[pyenum]
enum YourEnum { Variant1, Variant2 }

// Add to BOTH backend modules (postgres and sqlite) - around lines 60 & 76
#[pymodule]
#[cfg(feature = "postgres")]
fn cloaca_postgres(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<YourClass>()?;        // <- ADD THIS for classes
    m.add_function(wrap_pyfunction!(your_function, m)?)?;  // <- ADD THIS for functions  
    m.add("YOUR_CONSTANT", 42)?;       // <- ADD THIS for constants
    // ... rest
}

#[pymodule] 
#[cfg(feature = "sqlite")]
fn cloaca_sqlite(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<YourClass>()?;        // <- ADD THIS for classes
    m.add_function(wrap_pyfunction!(your_function, m)?)?;  // <- ADD THIS for functions
    m.add("YOUR_CONSTANT", 42)?;       // <- ADD THIS for constants  
    // ... rest
}
```

### 2. Python Backend Wrapper Template
**File**: `./cloaca-backend/python/cloaca_{{backend}}/__init__.py` (relative to project root)

```python
# Import from the extension module built by maturin - around line 6
from .cloaca_{{backend}} import (
    hello_world, 
    get_backend, 
    YourClass,      # <- ADD classes
    your_function,  # <- ADD functions  
    YOUR_CONSTANT,  # <- ADD constants
    __backend__
)

# Around line 10-15
__all__ = [
    "hello_world",
    "get_backend", 
    "YourClass",      # <- ADD classes
    "your_function",  # <- ADD functions
    "YOUR_CONSTANT",  # <- ADD constants
    "__backend__",
]
```

### 3. Dispatcher Package  
**File**: `./cloaca/src/cloaca/__init__.py` (relative to project root)

```python
# Around line 86-92 - Also expose commonly used symbols directly
if hasattr(_backend_module, "hello_world"):
    hello_world = _backend_module.hello_world
if hasattr(_backend_module, "get_backend"):
    get_backend = _backend_module.get_backend
if hasattr(_backend_module, "YourClass"):      # <- ADD classes
    YourClass = _backend_module.YourClass
if hasattr(_backend_module, "your_function"):  # <- ADD functions
    your_function = _backend_module.your_function  
if hasattr(_backend_module, "YOUR_CONSTANT"):  # <- ADD constants
    YOUR_CONSTANT = _backend_module.YOUR_CONSTANT
```

### 4. Add Tests 
**File**: `./python-tests/test_basic.py` (relative to project root)

```python
# Add to TestBackendFunctionality class around line 120+
def test_your_symbol_basic(self):
    """Test basic functionality of your new symbol."""
    import cloaca
    
    # Test class
    obj = cloaca.YourClass()
    assert obj is not None
    
    # Test function
    result = cloaca.your_function()
    assert result == "result"
    
    # Test constant
    assert cloaca.YOUR_CONSTANT == 42
```

## Critical Template Configuration

### Cargo.toml Template
**File**: `./.angreal/templates/backend_cargo.toml.j2` (relative to project root)
```toml
# Around line 8-10
[lib]
name = "cloaca_{{backend}}"  # <- MUST match pyproject.toml module-name
crate-type = ["cdylib"]
```

### PyProject.toml Template
**File**: `./.angreal/templates/backend_pyproject.toml.j2` (relative to project root)
```toml  
# Around line 38-42
[tool.maturin]
features = ["{{backend}}"]
module-name = "cloaca_{{backend}}"  # <- MUST match Cargo.toml lib name
python-source = "python"
```

## Build Script Configuration

### Wheel Location 
**File**: `./.angreal/task_cloaca.py` (relative to project root)
```python
# Around line 159-161 in _build_and_install_cloaca_backend()
wheel_pattern = f"cloaca_{backend_name}-*.whl"
wheel_dir = backend_dir / "target" / "wheels"  # <- NOT project_root / "target" / "wheels"
```

## Common Failure Modes

1. **Class compiles but not available in Python**:
   - Check backend wrapper template imports the class
   - Check dispatcher re-exports the class

2. **Import errors during build**:
   - Check Cargo.toml `lib.name` matches pyproject.toml `module-name`
   - Both should be `cloaca_{{backend}}`

3. **Wheel not found during tests**:
   - Check build script looks in `backend_dir / "target" / "wheels"`
   - NOT `project_root / "target" / "wheels"`

4. **Functions work but classes don't**:
   - Always a Python import layer issue, never a Rust compilation issue
   - Check steps 2 and 3 above

## Testing the Pattern

Always test through the full angreal pipeline:
```bash
angreal cloaca test --backend sqlite -k "your_class"
```

This ensures:
- Template generation works
- Rust compilation works  
- Python wrapper imports work
- Dispatcher re-exports work
- End-to-end functionality works

## Why This is So Complex

The Python bindings use a **dispatcher pattern**:
1. Templates generate backend-specific packages (`cloaca_sqlite`, `cloaca_postgres`)
2. Each package has Rust extension + Python wrapper 
3. Main `cloaca` package imports from whichever backend is installed
4. User imports from `cloaca` and gets the right backend automatically

This means every class needs to be:
1. Exported from Rust ✅
2. Imported by Python wrapper ✅  
3. Re-exported by dispatcher ✅
4. Tested end-to-end ✅

**Missing ANY step = class won't work for users.**