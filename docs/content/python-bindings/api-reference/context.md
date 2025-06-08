---
title: "Context"
description: "Context class for managing workflow data"
weight: 10
reviewer: "automation"
review_date: "2025-01-07"
---

# Context

The `Context` class is the primary data container that flows through your workflow, allowing tasks to share information. It provides a dictionary-like interface with additional features for workflow data management.

## Constructor

### `Context(data=None)`

Create a new Context instance.

**Parameters:**
- `data` (dict, optional): Initial data dictionary. Defaults to empty dict.

**Example:**
```python
import cloaca

# Empty context
context = cloaca.Context()

# Context with initial data
context = cloaca.Context({
    "user_id": 123,
    "email": "user@example.com"
})
```

## Core Methods

### `get(key, default=None)`

Get a value by key with optional default.

**Parameters:**
- `key` (str): The key to look up
- `default` (Any, optional): Value to return if key doesn't exist

**Returns:** The value associated with the key, or default if not found

**Example:**
```python
context = cloaca.Context({"name": "Alice", "age": 30})

name = context.get("name")           # "Alice"
city = context.get("city", "Unknown") # "Unknown" (default)
missing = context.get("missing")     # None
```

### `set(key, value)`

Set a value (insert new or update existing).

**Parameters:**
- `key` (str): The key to set
- `value` (Any): The value to store (must be JSON-serializable)

**Example:**
```python
context = cloaca.Context()

context.set("user_id", 123)
context.set("preferences", {"theme": "dark", "notifications": True})
context.set("processed_at", "2025-01-07T10:00:00Z")
```

### `insert(key, value)`

Insert a new key-value pair. Raises `ValueError` if key already exists.

**Parameters:**
- `key` (str): The key to insert
- `value` (Any): The value to store

**Raises:** `ValueError` if key already exists

**Example:**
```python
context = cloaca.Context()

context.insert("user_id", 123)        # OK
# context.insert("user_id", 456)      # ValueError: Key already exists
```

### `update(key, value)`

Update an existing key. Raises `KeyError` if key doesn't exist.

**Parameters:**
- `key` (str): The key to update
- `value` (Any): The new value

**Raises:** `KeyError` if key doesn't exist

**Example:**
```python
context = cloaca.Context({"count": 0})

context.update("count", 1)            # OK
# context.update("missing", 1)        # KeyError: Key not found
```

### `remove(key)`

Remove a key and return its value.

**Parameters:**
- `key` (str): The key to remove

**Returns:** The value that was removed, or `None` if key didn't exist

**Example:**
```python
context = cloaca.Context({"temp": "data", "keep": "this"})

removed = context.remove("temp")      # Returns "data"
missing = context.remove("missing")   # Returns None
```

## Dictionary-Style Operations

Context supports Python dictionary-style operations:

### `context[key]`

Get a value by key. Raises `KeyError` if key doesn't exist.

**Example:**
```python
context = cloaca.Context({"name": "Alice"})

name = context["name"]        # "Alice"
# missing = context["missing"] # KeyError
```

### `context[key] = value`

Set a value by key.

**Example:**
```python
context = cloaca.Context()

context["user_id"] = 123
context["data"] = {"key": "value"}
```

### `del context[key]`

Delete a key. Raises `KeyError` if key doesn't exist.

**Example:**
```python
context = cloaca.Context({"temp": "data"})

del context["temp"]           # OK
# del context["missing"]      # KeyError
```

### `key in context`

Check if a key exists.

**Example:**
```python
context = cloaca.Context({"name": "Alice"})

has_name = "name" in context      # True
has_age = "age" in context        # False
```

### `len(context)`

Get the number of key-value pairs.

**Example:**
```python
context = cloaca.Context({"a": 1, "b": 2})

count = len(context)              # 2
```

## Serialization Methods

### `to_dict()`

Convert the context to a Python dictionary.

**Returns:** dict containing all key-value pairs

**Example:**
```python
context = cloaca.Context({"name": "Alice", "age": 30})

data = context.to_dict()
# {"name": "Alice", "age": 30}
```

### `update_from_dict(data)`

Update the context from a dictionary.

**Parameters:**
- `data` (dict): Dictionary of key-value pairs to add/update

**Example:**
```python
context = cloaca.Context({"name": "Alice"})

context.update_from_dict({
    "age": 30,
    "city": "New York"
})
```

### `to_json()`

Serialize the context to a JSON string.

**Returns:** JSON string representation

**Example:**
```python
context = cloaca.Context({"name": "Alice", "age": 30})

json_str = context.to_json()
# '{"name": "Alice", "age": 30}'
```

### `Context.from_json(json_str)`

Create a Context from a JSON string.

**Parameters:**
- `json_str` (str): Valid JSON string

**Returns:** New Context instance

**Example:**
```python
json_data = '{"name": "Alice", "age": 30}'
context = cloaca.Context.from_json(json_data)

name = context.get("name")  # "Alice"
```

## Data Types

Context can store any JSON-serializable Python data:

{{< tabs "data-types" >}}
{{< tab "Basic Types" >}}
```python
context = cloaca.Context()

# Strings, numbers, booleans
context.set("name", "Alice")
context.set("age", 30)
context.set("active", True)
context.set("score", 95.5)
```
{{< /tab >}}

{{< tab "Collections" >}}
```python
context = cloaca.Context()

# Lists and dictionaries
context.set("tags", ["user", "premium", "active"])
context.set("preferences", {
    "theme": "dark",
    "notifications": True,
    "language": "en"
})

# Nested structures
context.set("user_profile", {
    "personal": {"name": "Alice", "age": 30},
    "settings": {"theme": "dark"},
    "activity": [
        {"action": "login", "timestamp": "2025-01-07T10:00:00Z"},
        {"action": "view_page", "timestamp": "2025-01-07T10:01:00Z"}
    ]
})
```
{{< /tab >}}

{{< tab "Common Patterns" >}}
```python
# Timestamps (store as ISO strings)
context.set("created_at", "2025-01-07T10:00:00Z")
context.set("processed_at", datetime.now().isoformat())

# Identifiers
context.set("user_id", 123)
context.set("session_id", "sess_abc123")
context.set("request_id", str(uuid.uuid4()))

# Status tracking
context.set("status", "processing")
context.set("progress", 0.75)
context.set("errors", [])

# Counts and metrics
context.set("items_processed", 100)
context.set("total_items", 150)
context.set("error_count", 2)
```
{{< /tab >}}
{{< /tabs >}}

## Usage Patterns

### Safe Access Pattern

```python
@cloaca.task(id="safe_task")
def safe_task(context):
    # Safe access with defaults
    user_id = context.get("user_id", 0)
    preferences = context.get("preferences", {})

    # Validate required data
    if not user_id:
        raise ValueError("user_id is required")

    # Process data
    context.set("processed", True)
    return context
```

### Data Transformation Pattern

```python
@cloaca.task(id="transform_data")
def transform_data(context):
    # Get input data
    raw_data = context.get("raw_data", [])

    # Transform
    processed_data = []
    for item in raw_data:
        processed_item = {
            "id": item["id"],
            "value": item["value"] * 2,
            "processed_at": datetime.now().isoformat()
        }
        processed_data.append(processed_item)

    # Store results
    context.set("processed_data", processed_data)
    context.set("transformation_complete", True)

    return context
```

### Accumulator Pattern

```python
@cloaca.task(id="accumulate_results")
def accumulate_results(context):
    # Get existing results
    all_results = context.get("all_results", [])

    # Add new results
    new_results = context.get("batch_results", [])
    all_results.extend(new_results)

    # Update context
    context.set("all_results", all_results)
    context.set("total_count", len(all_results))

    return context
```

## Thread Safety

- **Read operations** (`get`, `to_dict`, `in`, `len`): Thread-safe
- **Write operations** (`set`, `insert`, `update`, `remove`): Use locks for concurrent access
- **Dictionary operations**: Follow same thread safety rules

**Example with threading:**
```python
import threading

context = cloaca.Context()
lock = threading.Lock()

def safe_update(key, value):
    with lock:
        context.set(key, value)

# Use safe_update for concurrent writes
```

## Performance Considerations

- **Serialization**: Context data is serialized for database persistence
- **Memory usage**: Large objects should be avoided; consider external storage
- **JSON compatibility**: All data must be JSON-serializable
- **Deep copying**: Context operations may create deep copies for safety

## Best Practices

{{< tabs "best-practices" >}}
{{< tab "Naming" >}}
```python
# Good: descriptive, consistent naming
context.set("user_profile", user_data)
context.set("processing_start_time", start_time)
context.set("validation_errors", errors)

# Avoid: unclear or inconsistent names
context.set("data", user_data)        # Too generic
context.set("start", start_time)      # Unclear
context.set("errs", errors)           # Abbreviated
```
{{< /tab >}}

{{< tab "Structure" >}}
```python
# Good: organized structure
context.set("user", {
    "id": 123,
    "name": "Alice",
    "email": "alice@example.com"
})
context.set("processing", {
    "status": "active",
    "progress": 0.5,
    "started_at": start_time
})

# Avoid: flat structure with prefixes
context.set("user_id", 123)
context.set("user_name", "Alice")
context.set("user_email", "alice@example.com")
context.set("processing_status", "active")
```
{{< /tab >}}

{{< tab "Error Handling" >}}
```python
@cloaca.task(id="robust_task")
def robust_task(context):
    try:
        # Validate required data
        required_fields = ["user_id", "operation_type"]
        for field in required_fields:
            if field not in context:
                raise ValueError(f"Missing required field: {field}")

        # Process data
        result = process_operation(context)
        context.set("result", result)
        context.set("success", True)

    except Exception as e:
        # Store error information
        context.set("success", False)
        context.set("error_message", str(e))
        context.set("error_type", type(e).__name__)
        raise  # Re-raise to mark task as failed

    return context
```
{{< /tab >}}
{{< /tabs >}}

## Related Classes

- **[DefaultRunner](/python-bindings/api-reference/runner/)** - Executes workflows with Context
- **[WorkflowBuilder](/python-bindings/api-reference/workflow-builder/)** - Builds workflows that use Context
- **[Task Decorator](/python-bindings/api-reference/task/)** - Defines tasks that receive Context
