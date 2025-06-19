---
title: "FFI System"
description: "C FFI interface for dynamic plugin loading"
weight: 21
reviewer: "dstorey"
review_date: "2025-01-17"
---

This article provides a comprehensive technical overview of Cloacina's FFI (Foreign Function Interface) system, which enables dynamic loading and execution of packaged workflows. The FFI system creates a standardized C-compatible interface that allows Cloacina to load and execute workflow packages at runtime.

## Overview

The FFI system serves as the bridge between dynamically loaded workflow packages and the Cloacina runtime. It provides:

- **C-compatible interface** for cross-platform dynamic loading
- **Standardized symbol exports** for metadata extraction and task execution
- **Safe memory management** with static data structures
- **JSON-based data exchange** for context and results
- **Simple error handling** with integer return codes

## Required FFI Symbols

Every packaged workflow must export exactly two C-compatible symbols:

### 1. `cloacina_execute_task`

**Purpose**: Execute a specific task with JSON context data

**C Signature**:
```c
extern "C" int cloacina_execute_task(
    const char* task_name,      // Task name as UTF-8 bytes
    uint32_t task_name_len,     // Length of task name
    const char* context_json,   // Input context as JSON bytes
    uint32_t context_len,       // Length of context JSON
    uint8_t* result_buffer,     // Buffer for result JSON
    uint32_t result_capacity,   // Size of result buffer
    uint32_t* result_len        // Actual length of result written
);
```

**Return Values**:
- `0`: Success
- `-1`: General error (task failed, invalid input, etc.)
- `-2`: Critical system error (JSON serialization failure)

### 2. `cloacina_get_task_metadata`

**Purpose**: Extract package and task metadata

**C Signature**:
```c
extern "C" const cloacina_ctl_package_tasks* cloacina_get_task_metadata();
```

**Returns**: Pointer to static metadata structure

## C Data Structures

The FFI interface uses `#[repr(C)]` structures with defined memory layout for C compatibility:

### Task Metadata Structure

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct cloacina_ctl_task_metadata {
    pub index: u32,                           // Task index in package
    pub local_id: *const c_char,             // Task ID (null-terminated)
    pub namespaced_id_template: *const c_char, // Template for namespaced IDs
    pub dependencies_json: *const c_char,     // Dependencies as JSON array
    pub description: *const c_char,           // Task description
    pub source_location: *const c_char,       // Source file location
}
```

### Package Metadata Structure

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct cloacina_ctl_package_tasks {
    pub task_count: u32,                     // Number of tasks in package
    pub tasks: *const cloacina_ctl_task_metadata, // Array of task metadata
    pub package_name: *const c_char,         // Package name (null-terminated)
    pub graph_data_json: *const c_char,      // Workflow graph as JSON
}
```

{{< hint type=warning title="Memory Safety" >}}
All string pointers in FFI structures point to static data within the library. The caller must not free these pointers, and they remain valid for the lifetime of the loaded library.
{{< /hint >}}

## Generated FFI Implementation

The `#[packaged_workflow]` macro automatically generates the complete FFI implementation. Here's what it actually generates based on the implementation in `cloacina-macros/src/packaged_workflow.rs`:

### Static Data Generation

For a workflow like:

```rust
#[packaged_workflow(
    name = "data_processing",
    package = "example",
    description = "Example data processing workflow"
)]
pub mod data_processing {
    #[task(id = "collect_data", dependencies = [])]
    pub async fn collect_data(context: &mut Context<Value>) -> Result<(), TaskError> {
        // Task implementation
    }

    #[task(id = "process_data", dependencies = ["collect_data"])]
    pub async fn process_data(context: &mut Context<Value>) -> Result<(), TaskError> {
        // Task implementation
    }
}
```

The macro generates:

```rust
/// Static array of task metadata
static TASK_METADATA_ARRAY: [cloacina_ctl_task_metadata; 2] = [
    cloacina_ctl_task_metadata {
        index: 0,
        local_id: "collect_data\0".as_ptr() as *const c_char,
        namespaced_id_template: "{}::{}::data_processing::collect_data\0".as_ptr() as *const c_char,
        dependencies_json: "[]\0".as_ptr() as *const c_char,
        description: "Collect input data\0".as_ptr() as *const c_char,
        source_location: "src/lib.rs:45:1\0".as_ptr() as *const c_char,
    },
    cloacina_ctl_task_metadata {
        index: 1,
        local_id: "process_data\0".as_ptr() as *const c_char,
        namespaced_id_template: "{}::{}::data_processing::process_data\0".as_ptr() as *const c_char,
        dependencies_json: "[\"collect_data\"]\0".as_ptr() as *const c_char,
        description: "Process collected data\0".as_ptr() as *const c_char,
        source_location: "src/lib.rs:67:1\0".as_ptr() as *const c_char,
    }
];

/// Static package metadata
static PACKAGE_TASKS_METADATA: cloacina_ctl_package_tasks = cloacina_ctl_package_tasks {
    task_count: 2,
    tasks: TASK_METADATA_ARRAY.as_ptr(),
    package_name: "example\0".as_ptr() as *const c_char,
    graph_data_json: "{\"tasks\":{...}}\0".as_ptr() as *const c_char,
};
```

### FFI Function Generation

#### Metadata Function

```rust
#[no_mangle]
pub extern "C" fn cloacina_get_task_metadata() -> *const cloacina_ctl_package_tasks {
    &PACKAGE_TASKS_METADATA
}
```

#### Task Execution Function

```rust
#[no_mangle]
pub extern "C" fn cloacina_execute_task(
    task_name: *const c_char,
    task_name_len: u32,
    context_json: *const c_char,
    context_len: u32,
    result_buffer: *mut u8,
    result_capacity: u32,
    result_len: *mut u32,
) -> i32 {
    // 1. Convert raw pointers to safe Rust types
    let task_name_bytes = unsafe {
        std::slice::from_raw_parts(task_name as *const u8, task_name_len as usize)
    };

    let task_name_str = match std::str::from_utf8(task_name_bytes) {
        Ok(s) => s,
        Err(_) => {
            return write_error_result("Invalid UTF-8 in task name", result_buffer, result_capacity, result_len);
        }
    };

    // 2. Parse context from JSON
    let context_bytes = unsafe {
        std::slice::from_raw_parts(context_json as *const u8, context_len as usize)
    };

    let context_str = match std::str::from_utf8(context_bytes) {
        Ok(s) => s,
        Err(_) => {
            return write_error_result("Invalid UTF-8 in context", result_buffer, result_capacity, result_len);
        }
    };

    let mut context = match cloacina::Context::from_json(context_str.to_string()) {
        Ok(ctx) => ctx,
        Err(e) => {
            return write_error_result(&format!("Failed to create context from JSON: {}", e), result_buffer, result_capacity, result_len);
        }
    };

    // 3. Execute task in async runtime
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            return write_error_result(&format!("Failed to create async runtime: {}", e), result_buffer, result_capacity, result_len);
        }
    };

    let task_result = runtime.block_on(async {
        match task_name_str {
            "collect_data" => data_processing::collect_data(&mut context).await,
            "process_data" => data_processing::process_data(&mut context).await,
            _ => Err(format!("Unknown task: {}", task_name_str))
        }
    });

    // 4. Handle result and write to buffer
    match task_result {
        Ok(()) => {
            match context.to_json() {
                Ok(context_json) => {
                    match serde_json::from_str::<serde_json::Value>(&context_json) {
                        Ok(context_value) => write_success_result(&context_value, result_buffer, result_capacity, result_len),
                        Err(e) => write_error_result(&format!("Failed to parse context JSON: {}", e), result_buffer, result_capacity, result_len)
                    }
                }
                Err(e) => write_error_result(&format!("Failed to serialize context: {}", e), result_buffer, result_capacity, result_len)
            }
        }
        Err(e) => {
            write_error_result(&format!("Task '{}' failed: {}", task_name_str, e), result_buffer, result_capacity, result_len)
        }
    }
}
```

## Buffer Management and Error Handling

### Actual Buffer Management Implementation

The FFI system uses a simple buffer approach. From the actual implementation:

```rust
fn write_success_result(result: &serde_json::Value, buffer: *mut u8, capacity: u32, result_len: *mut u32) -> i32 {
    let json_str = match serde_json::to_string(result) {
        Ok(s) => s,
        Err(_) => return -1,  // JSON serialization failed
    };

    let bytes = json_str.as_bytes();
    let len = bytes.len().min(capacity as usize);  // Truncate if too large

    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, len);
        *result_len = len as u32;
    }

    0 // Success
}

fn write_error_result(error: &str, buffer: *mut u8, capacity: u32, result_len: *mut u32) -> i32 {
    let error_json = serde_json::json!({
        "error": error,
        "status": "error"
    });

    let json_str = match serde_json::to_string(&error_json) {
        Ok(s) => s,
        Err(_) => return -2,  // Critical: can't even serialize error
    };

    let bytes = json_str.as_bytes();
    let len = bytes.len().min(capacity as usize);  // Truncate if too large

    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, len);
        *result_len = len as u32;
    }

    -1 // Error
}
```

### Client-Side Buffer Usage

From `cloacina-ctl/src/library/execution.rs`, the client uses a fixed 4KB buffer:

```rust
const RESULT_BUFFER_SIZE: usize = 4096;  // 4KB buffer for result
let mut result_buffer = vec![0u8; RESULT_BUFFER_SIZE];
let mut result_len: u32 = 0;

let return_code = unsafe {
    execute_task(
        task_name_bytes.as_ptr(),
        task_name_bytes.len() as u32,
        context_bytes.as_ptr(),
        context_bytes.len() as u32,
        result_buffer.as_mut_ptr(),
        result_buffer.len() as u32,
        &mut result_len,
    )
};

// Handle result
if return_code == 0 {
    // Success
    if result_len > 0 && result_len <= result_buffer.len() as u32 {
        let result_json = String::from_utf8_lossy(&result_buffer[..result_len as usize]);
        println!("Task execution successful!");
        println!("Result: {}", result_json);
    }
} else {
    // Error - result_buffer contains error JSON
    let error_msg = if result_len > 0 && result_len <= result_buffer.len() as u32 {
        String::from_utf8_lossy(&result_buffer[..result_len as usize]).to_string()
    } else {
        format!("Unknown error (code: {})", return_code)
    };

    bail!("Task execution failed: {}", error_msg);
}
```

{{< hint type=note title="Buffer Truncation" >}}
If the result JSON is larger than the buffer capacity, it will be silently truncated. The current implementation does not handle buffer overflow gracefully - results are simply cut off at the buffer boundary.
{{< /hint >}}

## Symbol Discovery and Dynamic Loading

### Symbol Discovery Process

The package loader attempts symbol discovery with fallback:

```rust
// From cloacina/src/registry/loader/package_loader.rs
let get_metadata = unsafe {
    match lib.get::<unsafe extern "C" fn() -> *const CPackageTasks>(
        "cloacina_get_task_metadata".as_bytes(),
    ) {
        Ok(func) => func,
        Err(_) => {
            // Try package-specific function name as fallback
            let func_name = format!("cloacina_get_task_metadata_{}\0", package_name);
            lib.get::<unsafe extern "C" fn() -> *const CPackageTasks>(func_name.as_bytes())
                .map_err(|e| LoaderError::SymbolNotFound {
                    symbol: "cloacina_get_task_metadata".to_string(),
                    error: e.to_string(),
                })?
        }
    }
};
```

### Metadata Extraction

```rust
// Call the metadata function
let c_package_tasks = unsafe { get_metadata() };
if c_package_tasks.is_null() {
    return Err(LoaderError::MetadataExtraction {
        reason: "Metadata function returned null pointer".to_string(),
    });
}

// Convert C structures to Rust structures
let package_tasks = unsafe { &*c_package_tasks };
```

## Memory Management and Safety

### Static Data Approach

The FFI system relies entirely on static data:

- **String literals**: All strings are stored as static `&'static str` with null terminators
- **Metadata arrays**: Task metadata is stored in static arrays
- **Pointer safety**: All pointers reference static data, eliminating use-after-free risks
- **Thread safety**: Static data is inherently thread-safe for read access

### Memory Layout Guarantees

```rust
// Safety: These pointers point to static string literals which are safe to share
unsafe impl Sync for cloacina_ctl_task_metadata {}

// Safety: These pointers point to static data which is safe to share
unsafe impl Sync for cloacina_ctl_package_tasks {}
```

### Cross-Platform Compatibility

The FFI system handles platform differences through:

- **`#[no_mangle]`**: Prevents Rust name mangling
- **`extern "C"`**: Uses C calling convention across platforms
- **`#[repr(C)]`**: Ensures consistent memory layout
- **Standard types**: Uses `std::os::raw::c_char` for portability

## Empty Package Handling

For packages with no tasks, the macro generates a special implementation:

```rust
pub extern "C" fn cloacina_execute_task(...) -> i32 {
    let error_json = serde_json::json!({
        "error": "No tasks defined in this package",
        "status": "error"
    });

    let json_str = match serde_json::to_string(&error_json) {
        Ok(s) => s,
        Err(_) => return -2,  // JSON serialization failed
    };

    // Write error to buffer and return -1
    let bytes = json_str.as_bytes();
    let len = bytes.len().min(result_capacity as usize);

    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), result_buffer, len);
        *result_len = len as u32;
    }

    -1
}
```

## Limitations and Considerations

### Current Limitations

1. **Fixed Buffer Size**: Client uses 4KB buffer with no dynamic resizing
2. **Silent Truncation**: Large results are truncated without error indication
3. **No Buffer Overflow Detection**: No mechanism to detect when results are too large
4. **Single Threaded Execution**: Each task execution creates its own tokio runtime

### Performance Considerations

- **Static Metadata Access**: Zero-cost metadata access through static arrays
- **Runtime Creation Overhead**: New tokio runtime created for each task execution
- **JSON Serialization**: Context serialized/deserialized for each task call
- **Memory Copying**: Results copied through buffer interface

## Best Practices

### For FFI Implementation

1. **Use `#[repr(C)]` and `#[no_mangle]`** for all exported structures and functions
2. **Include null terminators** for all C strings in static data
3. **Handle UTF-8 validation** explicitly at FFI boundary
4. **Use static data** for all exported metadata and strings

### For Buffer Management

1. **Check result_len** against buffer capacity after FFI calls
2. **Handle truncation gracefully** in client code
3. **Consider larger buffers** for complex workflows with large context data
4. **Validate JSON completeness** after reading from buffer

### For Error Handling

1. **Always check return codes** - only 0 indicates success
2. **Parse error JSON** from result buffer for detailed error information
3. **Handle both general (-1) and critical (-2) error cases**
4. **Provide meaningful error context** in client applications

## Related Resources

- [Tutorial: Creating Your First Packaged Workflow]({{< ref "/tutorials/07-packaged-workflows/" >}})
- [Explanation: Package Format]({{< ref "/explanation/package-format/" >}})
- [Explanation: Packaged Workflow Architecture]({{< ref "/explanation/packaged-workflow-architecture/" >}})
