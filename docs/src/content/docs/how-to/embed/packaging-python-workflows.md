---
title: "Packaging Python Workflows"
description: "How to package Python workflows as .cloacina archives for deployment to the daemon or server"
sidebar:
  order: 40
---

This guide explains how to turn a Python workflow into a `.cloacina` package that
can be deployed to the Cloacina daemon or server without shipping your source
environment.

> **One canonical format.** A `.cloacina` package is described by a top-level
> **`package.toml`** and a Python module tree under a **`workflow/`** directory.
> This is the only format the server/daemon accept. (Older docs that used
> `[tool.cloaca]` in `pyproject.toml` + a hand-written `manifest.json` are
> obsolete. A `pyproject.toml` in the package now has one job: it declares the
> package's dependencies, see Step 3.)

## Prerequisites

- A working Python workflow (see [Python tutorials](../../../tutorials/embed/))
- `cloaca` installed (`pip install cloaca`)
- Understanding of the daemon or server deployment model

## Project Layout

The package root contains a `package.toml` and a `workflow/` directory holding
your module tree:

```
data-pipeline/
├── package.toml             # package + metadata (REQUIRED, at the root)
└── workflow/                # REQUIRED directory — your modules live here
    └── data_pipeline/       # your entry module package
        ├── __init__.py
        └── tasks.py         # @cloaca.task decorators here
```

Key requirements:
- The module tree **must** live under `workflow/`. A top-level module (e.g.
  `data-pipeline/data_pipeline/`) fails to load with
  `Missing workflow source directory`.
- `entry_module` in `package.toml` is the dotted path **relative to `workflow/`**
  (here, `data_pipeline.tasks`).
- Tasks/triggers must register at **import time** (module level) with **bare
  `@cloaca.task` / `@cloaca.trigger` decorators** — do **not** wrap them in a
  `WorkflowBuilder` context. The loader establishes the workflow context (from
  `workflow_name`) before importing your module; a `WorkflowBuilder` here would
  shadow it and the package would load with zero tasks. (`WorkflowBuilder` is only
  for the in-process test path in Step 4.)

## Step 1: Write `package.toml`

```toml
[package]
name = "data-pipeline"
version = "1.0.0"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
language = "python"
workflow_name = "data_pipeline"
entry_module = "data_pipeline.tasks"
description = "ETL pipeline for analytics"
author = "you@example.com"
requires_python = ">=3.10"
```

The `[metadata]` table is a **closed schema** — unknown keys are rejected at
upload. The accepted keys are:

| Field | Required | Description |
|-------|----------|-------------|
| `language` | yes | `"python"` (or `"rust"`). |
| `workflow_name` | for workflows | The `WorkflowBuilder(name=...)` value. |
| `graph_name` | for computation graphs | The graph name (instead of `workflow_name`). |
| `entry_module` | yes | Dotted module path **relative to `workflow/`** that the loader imports. |
| `description` | no | Human-readable description. |
| `author` | no | Author/owner. |
| `requires_python` | no | A version specifier, e.g. `">=3.11"`. Checked against the Python that runs the package: at `package validate`, at upload and at load. |
| `vendored` | written by `package build` | The `[metadata.vendored]` table: the Python version and the platforms of `vendor/`. See Step 3. Do not write it by hand. |
| `reaction_mode` / `input_strategy` | computation graphs | `when_any`/`when_all`, `latest`/`sequential`. |
| `accumulators` | computation graphs | `[[metadata.accumulators]]` entries (stream/kafka sources). |

> **Do not** add a `package_type` key or `[[metadata.triggers]]` table — both are
> rejected by the parser. Triggers are declared in code via `@cloaca.trigger`,
> not in the manifest.

## Step 2: Write Your Workflow

In your entry module (`workflow/data_pipeline/tasks.py`), declare tasks with
**bare `@cloaca.task` decorators at module level** — no `WorkflowBuilder`:

```python
import cloaca

@cloaca.task(dependencies=[])
def extract(context):
    # cloaca.var() reads from CLOACINA_VAR_ env vars at runtime
    # See "External Configuration" section below
    source = cloaca.var("DATA_SOURCE")
    context.set("raw_data", fetch_from(source))  # Replace with your data function
    return context

@cloaca.task(dependencies=["extract"])
def transform(context):
    raw = context.get("raw_data")
    context.set("clean_data", clean(raw))  # Replace with your transform logic
    return context

@cloaca.task(dependencies=["transform"])
def load(context):
    dest = cloaca.var("WAREHOUSE_URL")
    write_to(dest, context.get("clean_data"))  # Replace with your load logic
    return context
```

The tasks are grouped into a workflow by the `workflow_name` you set in
`package.toml` — the loader establishes that workflow context before importing
`entry_module`, and your bare decorators register into it. Make
`workflow/data_pipeline/__init__.py` import the entry module (so importing the
package registers the tasks), or point `entry_module` directly at the file that
defines them (`data_pipeline.tasks`, as above).

:::caution[Bare decorators — not WorkflowBuilder]
In a **packaged** workflow, do **not** wrap tasks in
`with cloaca.WorkflowBuilder(...)`. That context manager is for running a
workflow in-process (it pushes its own workflow context); inside a package it
shadows the loader's context, so the loader finds no tasks under your
`workflow_name` and rejects the package with *"Empty package: registered no
tasks"*. Use bare `@cloaca.task` decorators and let `workflow_name` in
`package.toml` name the workflow.

All decorators **must** run at import time (module level) — if registration is
gated behind `if __name__ == "__main__"`, the loader won't find the tasks.
:::

## Step 3: Declare and Lock the Dependencies

A package runs with its own third-party libraries. The agent that runs it
installs nothing: it puts a folder of the package's dependencies on `sys.path`.
There are two ways to get that folder:

- **The compiler vendors (the default).** You publish the source, a
  `pyproject.toml` and a lock file. The compiler installs the wheels that the
  lock file names, for each platform it is set up for, and the server gives
  each agent the set of its platform.
- **You vendor (`--vendor`).** `cloacinactl package build --vendor` fills a
  `vendor/` folder on your machine and the package ships it. The compiler
  downloads nothing for such a package.

### Declare the dependencies

Add a `pyproject.toml` at the package root and list the dependencies in
`[project].dependencies`:

```toml
[project]
name = "data-pipeline"
version = "1.0.0"
dependencies = [
    "requests>=2.31",
    "pyarrow>=18",
]
```

This list is the source of truth. A package with no `pyproject.toml`, or with no
`[project].dependencies`, has nothing to lock, and the command says so.

### Build: write the lock file

```bash
cloacinactl package build .
```

The command needs [`uv`](https://docs.astral.sh/uv/) on `PATH` (or its path in
`CLOACINA_UV`). It resolves the dependencies to pinned versions with hashes and
writes them to `requirements.lock` in the package root
(`uv pip compile --universal --generate-hashes`). It does not fill `vendor/`.

```
data-pipeline/
├── package.toml
├── pyproject.toml           # the dependencies
├── requirements.lock        # the pinned versions and hashes
└── workflow/
    └── data_pipeline/
        └── tasks.py
```

`cloacinactl package publish` runs `package build` first, so a publish writes
the lock file. Commit `pyproject.toml` and `requirements.lock`.

### What the compiler does with the lock file

When the package is published, the compiler builds it. For a Python package
with a `requirements.lock` and no `vendor/` folder it:

1. checks the lock file: it may hold pinned versions (`name==version`) with
   `--hash` lines, environment markers and comments, and nothing else. An
   option line (`-r`, `--index-url`, `--find-links`), a URL or a path is
   refused with its line number. Each dependency of `pyproject.toml` must be
   pinned in it;
2. installs the wheels with `uv`, one time for each platform of its settings
   (by default `aarch64-linux` and `x86_64-linux`, for Python 3.11), with
   `--only-binary :all:`, `--no-deps` and `--require-hashes`: it installs what
   the lock names, and refuses a wheel whose hash the lock does not list;
3. stores one set of dependencies per platform next to the package.

The package you uploaded is not changed. A server or an agent that loads the
package gets the set of its own platform and unpacks it into
`vendor/<platform>/`.

The build fails, and `cloacinactl package inspect` shows the error, when:

| Case | The error names |
|---|---|
| `pyproject.toml` declares dependencies and the package has no `requirements.lock` and no `vendor/` | the package and the dependencies |
| The lock file holds an option, a URL, a path or a pin with no hash | the line |
| The lock file does not pin a dependency of `pyproject.toml` | the dependency |
| A dependency has no wheel for a platform (the compiler builds no source distribution) | the dependency, the platform and the Python version |
| A wheel is not in the compiler's wheel source (an offline compiler with a wheel folder or a cache) | the dependency and the source |
| A wheel's hash is not in the lock file | the dependency |
| `requires_python` excludes the compiler's Python version | both versions |
| The compiler image has no `uv` | `uv` |
| The set of one platform is over the unpack limits (see Size) | the size and the limit |

`cloacinactl package validate` and `package pack` check the first three before
the upload.

Where the wheels come from (a package index, or wheels in the compiler image)
and which platforms and Python version the compiler vendors for are settings of
the compiler: see [Running the Compiler](../../service/running-the-compiler/#python-packages-vendoring-dependencies).

### Vendor on your machine: `--vendor`

```bash
cloacinactl package build . --vendor
```

With `--vendor` the command also:

1. installs the wheels of each target platform into `vendor/<platform>/`
   (`uv pip install --target`, with `--only-binary :all:` and
   `--require-hashes`);
2. records the Python version and the platforms in `[metadata.vendored]` of
   `package.toml`.

```
data-pipeline/
├── package.toml             # now with [metadata.vendored]
├── pyproject.toml
├── requirements.lock
├── workflow/
│   └── data_pipeline/
│       └── tasks.py
└── vendor/
    ├── aarch64-linux/       # wheels for 64-bit ARM Linux agents
    │   ├── requests/
    │   └── pyarrow/
    └── x86_64-linux/        # wheels for 64-bit x86 Linux agents
        ├── requests/
        └── pyarrow/
```

```toml
# Written by `cloacinactl package build`: what is in vendor/. Do not edit.
[metadata.vendored]
python_version = "3.11"
platforms = ["aarch64-linux", "x86_64-linux"]
native = true
lock_file = "requirements.lock"
```

A package that ships `vendor/` is used as it is: the compiler runs no `uv` for
it and stores no set. Use `--vendor` when:

- the package goes to a **daemon**: a daemon loads archives from a folder and
  has no compiler in front of it;
- the compiler cannot reach the wheels (no index, and they are not in its
  image);
- a dependency is not a pinned release of an index (a URL, a local wheel).

`vendor/` can stay out of version control: the build writes it again.

A later build without `--vendor` keeps the `vendor/` folder of an earlier
`--vendor` build while `requirements.lock` is the same. When the lock file
changes, the build stops: run it with `--vendor` again, or remove `vendor/` and
the `[metadata.vendored]` table to let the compiler vendor.

### Flags

| Flag | Default | Meaning |
|------|---------|---------|
| `--vendor` | off | Also fill `vendor/<platform>/` and record it. |
| `--python-version <MAJOR.MINOR>` | `3.11` | The Python the dependencies are locked for, and with `--vendor` vendored for. |
| `--arch <x86_64\|aarch64\|both>` | `both` | With `--vendor`: the CPU architecture of the Linux agents. |
| `--platform <ARCH-OS>` | — | With `--vendor`: a platform to vendor for, repeatable: `aarch64-linux`, `x86_64-linux`, `aarch64-macos`, `x86_64-macos`, `x86_64-windows`. It replaces the platforms of `--arch`. |
| `--locked` | off | Use the `requirements.lock` in the package as it is. Without it the build resolves the versions again. |
| `--no-lock` | off | Read no `pyproject.toml`, write no lock file and do not touch `vendor/`. |

The default target of `--vendor` is the official agent image: Python 3.11 on
Linux, for both CPU architectures the image is built for. To run a package with
native dependencies on a daemon on a Mac as well as on Linux agents, name each
platform:

```bash
cloacinactl package build . --vendor \
  --platform aarch64-linux --platform x86_64-linux --platform aarch64-macos
```

A platform name is `<arch>-<os>`, the same name an agent reports for itself.

### What the build refuses

- **`uv` is not installed.** The error says how to install it.
- **A `requires_python` that excludes the target Python.**
- **Without `--vendor`: a lock file the compiler would refuse** (see the table
  above). A dependency that cannot be a pin with a hash ships with `--vendor`.
- **With `--vendor`: a dependency with no wheel for the target.** A package
  holds built wheels only; the build compiles no source distribution. The error
  names the dependency, the platform and the Python version. Pin a version that
  has a wheel, or remove the dependency.
- **With `--vendor`: a `vendor/` folder with files the command did not write.**
  The command writes only `vendor/<platform>/` folders. Move hand-copied files
  out and declare them in `pyproject.toml`, or build without `--vendor` to keep
  `vendor/` as it is.

### How a package is matched to an agent

- **Python version.** `requires_python` must allow the Python that runs the
  package, and vendored dependencies must be for the same `major.minor`.
  `cloacinactl package validate` and `package pack` check it (against
  `--python-version`, else the recorded version, else 3.11). The server checks
  it at upload against its own Python and refuses the upload. The loader checks
  it against the interpreter of the process that loads the package.
- **Platform, for a package that ships `vendor/`.** The loader uses
  `vendor/<its own platform>/`. A package whose dependencies are all pure
  Python (`native = false`) loads on any platform, from one of its folders. A
  package with compiled dependencies and no folder for the loading platform is
  refused, with an error that names the platforms it has and the one it needs.
- **Platform, for a package the compiler vendored.** The package loads on the
  platforms the compiler vendored for, and on no other: a server or an agent
  gets the set of its own platform only. The load on another platform fails
  with an error that names the platform. The compiler also checks the Python
  version: `requires_python` must allow the version it vendors for, and the
  loader refuses a set vendored for another `major.minor` than its interpreter.

### Two packages, one library, different versions

This is about packages that share one process: the tasks an agent runs inline
(`inline=True`), and a server or daemon that runs tasks itself. A task that an
agent runs in a worker process has an interpreter of its own, with its one
package, so two such packages can vendor different versions of a library on
one agent.

One process has one Python interpreter. A module is imported one time,
and every package in the process gets that copy. So when a package loads, each
top-level module of its vendor folder is compared with the packages loaded
before it. If another package vendors the same module at a different version,
the load of the new package fails:

```
package `reports` (tenant `acme`) vendors the module `yaml` at version 6.0.2
(PyYAML 6.0.2), and package `billing` (tenant `acme`), loaded before it in this
process, vendors `yaml` at version 5.4.1 (PyYAML 5.4.1), which is already
imported. One Python interpreter holds one version of a module, so package
`reports` (tenant `acme`) is not loaded. Vendor the same version in both
packages, or run them on different agents.
```

The package loaded first keeps running. Two packages that vendor the same
version share one copy. To run both versions, give the packages to different
agents.

A process keeps the modules it has imported, and the vendor folders on its
`sys.path`, until it stops. To move two packages to a new version of a shared
library, publish both with the new version, then restart the processes that
load them (the agents, and the server when it runs tasks itself).

The comparison reads the versions from the `.dist-info` folders that the build
writes. It does not cover a namespace package (a top-level folder shared by
several distributions, such as `google`), a `vendor/` folder with no
`.dist-info` folders, a shared library that two different modules each bundle,
or a new version of the same package that changes a vendored version.

### Size

The server accepts a package up to its upload limit: 100 MiB by default, set
with `--max-package-bytes` or `CLOACINA_MAX_PACKAGE_BYTES` on `cloacina-server`.
An upload over the limit is refused with status 413 and an error that names the
size of the upload and the limit. With `--vendor`, each platform adds its own
copy of the dependencies to the archive; `--arch` vendors for one architecture.
A package the compiler vendors uploads as source and a lock file, and is small.

An archive is also limited in what it unpacks to: 500 MiB, 10,000 files and
folders, and 10 times its own size. The first limit is a setting:
`--max-unpacked-bytes` or `CLOACINA_MAX_UNPACKED_BYTES`, in bytes. The other
two are fixed. `package validate` and `package pack` refuse a package source
over the first two, and the server refuses an archive over any of them.

Every process that unpacks the archive reads the setting, so a package over
500 MiB needs the same value in each of them:

| Process | How to set it |
|---|---|
| `cloacina-server` | `--max-unpacked-bytes` or `CLOACINA_MAX_UNPACKED_BYTES` (with `--max-package-bytes` when the archive is over 100 MiB) |
| `cloacina-compiler` | `--max-unpacked-bytes` or `CLOACINA_MAX_UNPACKED_BYTES` |
| `cloacina-agent` | `--max-unpacked-bytes` or `CLOACINA_MAX_UNPACKED_BYTES`. An agent that the server's fleet actuator starts gets the server's value. |
| `cloacinactl package validate`, `pack`, `publish` | `--max-unpacked-bytes` or `CLOACINA_MAX_UNPACKED_BYTES` |
| `cloacinactl daemon`, `cloacinactl server start`, an embedded engine | `CLOACINA_MAX_UNPACKED_BYTES` |

For example, a package with pyarrow, duckdb and pyiceberg vendored with
`--vendor` for both architectures is 549 MiB unpacked in a 157 MiB archive: it
needs `CLOACINA_MAX_UNPACKED_BYTES=629145600` (600 MiB) everywhere and
`CLOACINA_MAX_PACKAGE_BYTES=209715200` (200 MiB) on the server.

When the compiler vendors, the limits apply to the set of each platform on its
own: the same three limits, with the same setting. The set of one architecture
for that stack is 286 MiB unpacked in an 81 MiB archive, under the defaults.
The compiler refuses a set over a limit, and the build error gives the size.

### A `vendor/` folder filled by hand

A package with a `vendor/` folder and no `[metadata.vendored]` table still
loads: the loader puts `vendor/` itself on `sys.path`, and the compiler leaves
the package as it is. A build without `--vendor` does not touch the folder;
`--no-lock` also writes no lock file. The loader refuses the package when it
holds a wheel or a compiled file built for another platform.

:::caution[Stdlib Shadowing]
The loader **rejects** any package that shadows Python standard library modules.
You cannot vendor modules named `os`, `sys`, `json`, `pathlib`, `subprocess`, or
other stdlib names. This is a security measure to prevent code injection.

Blocked modules include: `os`, `sys`, `subprocess`, `shutil`, `socket`, `http`,
`urllib`, `ctypes`, `importlib`, `pathlib`, `io`, `json`, `pickle`, `marshal`,
`code`, `codeop`, `compile`, `compileall`, `builtins`, `signal`,
`multiprocessing`, `threading`, `tempfile`, `glob`, `fnmatch`.
:::

## Step 4: Test Before Packaging

To run a bare-decorator module in-process, supply the workflow context the
packaged loader would normally provide by wrapping the **import** in a
`WorkflowBuilder` with the same name as your `workflow_name`:

```python
import cloaca

# WorkflowBuilder here stands in for the loader's context (in-process only —
# it is NOT part of the packaged module).
with cloaca.WorkflowBuilder("data_pipeline"):
    import data_pipeline.tasks   # bare @cloaca.task decorators register here

runner = cloaca.DefaultRunner(":memory:")
try:
    result = runner.execute("data_pipeline", cloaca.Context())
    assert result.status == "completed"
finally:
    runner.shutdown()
```

## Step 5: Build the Package

Optionally check the layout first — `cloacinactl package validate` runs the same
schema + `workflow/`/`entry_module` checks as the server, against a source
directory or a packed archive, without uploading:

```bash
cloacinactl package validate .
```

Then pack. Run `cloacinactl package build .` first when the package has
dependencies (Step 3): `pack` refuses dependencies that have neither a lock
file nor a `vendor/` folder. `cloacinactl package pack` reads `package.toml`, runs the
same validation, and emits the `.cloacina` archive:

```bash
cloacinactl package pack . --out data-pipeline-1.0.0.cloacina
```

A `.cloacina` package is a bzip2-compressed tar archive of `package.toml` + the
`workflow/` tree (+ `pyproject.toml`, `requirements.lock` and `vendor/` if
present), under a single `<name>-<version>/` top-level directory:

```
data-pipeline-1.0.0.cloacina
└── data-pipeline-1.0.0/
    ├── package.toml
    ├── pyproject.toml          # if present
    ├── requirements.lock       # if present
    ├── workflow/
    │   └── data_pipeline/
    │       └── tasks.py
    └── vendor/                 # if present
        ├── aarch64-linux/
        └── x86_64-linux/
```

:::note[Packing fails fast on a bad layout]
If the module tree isn't under `workflow/`, or `entry_module` doesn't resolve to
a module there, `pack` errors immediately — you don't have to wait for the server
to reject the upload. The same parse rejects `package_type` and
`[[metadata.triggers]]`.
:::

<details>
<summary>Building the archive by hand (no <code>cloacinactl</code>)</summary>

The archive is a plain bzip2 tar, so you can build it with standard tools:

```bash
name=data-pipeline
version=1.0.0
prefix="$name-$version"

stage="$(mktemp -d)/$prefix"
mkdir -p "$stage"
cp package.toml "$stage/"
cp -R workflow "$stage/"
[ -d vendor ] && cp -R vendor "$stage/"

tar -cjf "$name-$version.cloacina" -C "$(dirname "$stage")" "$prefix"
```

</details>

## Step 6: Deploy

### To the Daemon

Copy the `.cloacina` file into one of the daemon's watched directories:

```bash
cp data-pipeline-1.0.0.cloacina ~/.cloacina/packages/
```

The daemon's reconciler detects it, extracts the archive, imports your
`entry_module`, registers the tasks, and registers the workflow.

A daemon has no compiler in front of it. A package with dependencies for a
daemon ships them: build it with `--vendor` and the daemon's platform (for a
Mac with Apple silicon, `--platform aarch64-macos`).

### To the Server

Upload via the HTTP API (or `cloacinactl package upload`):

```bash
curl -X POST \
  -H "Authorization: Bearer $API_KEY" \
  -F "file=@data-pipeline-1.0.0.cloacina" \
  https://cloacina.example.com/v1/tenants/my_tenant/workflows
```

## External Configuration with var/var_or

Use `cloaca.var()` and `cloaca.var_or()` to read configuration at runtime instead
of hardcoding values:

```python
# Resolved from CLOACINA_VAR_DATA_SOURCE environment variable
source = cloaca.var("DATA_SOURCE")

# With a default fallback
timeout = cloaca.var_or("FETCH_TIMEOUT", "30")
```

Set the variables on the host where the daemon or server runs:

```bash
export CLOACINA_VAR_DATA_SOURCE=postgres://analytics:pass@host/warehouse
export CLOACINA_VAR_FETCH_TIMEOUT=60
```

See [Variable Registry](../variable-registry/) for details.

## Troubleshooting

### "Missing workflow source directory"

Your module tree must live under a `workflow/` directory at the package root. A
top-level module (`<pkg>/<module>/`) is rejected — move it to
`<pkg>/workflow/<module>/`.

### "unknown field `package_type`" (or another `[metadata]` key)

`[metadata]` is a closed schema. Remove `package_type`, `[[metadata.triggers]]`,
or any other key not in the table in Step 1.

### "entry_module not found"

`entry_module` is a dotted path **relative to `workflow/`**. Verify the directory
matches (e.g. `workflow/data_pipeline/tasks.py` for
`entry_module = "data_pipeline.tasks"`) and that `__init__.py` files exist.

### Import times out after 60 seconds

The loader enforces a 60-second import timeout. Move expensive module-level
initialization into task functions; the package is rejected with
`"Python workflow import timed out after 60s"` otherwise.

### "requires Python …, and this host runs Python …"

`requires_python` in `package.toml` excludes the Python of the process that
loads the package. Change `requires_python`, or run the package on agents with a
Python it allows.

### "has dependencies vendored for Python …"

The dependencies in `vendor/` were built for another Python `major.minor` than
the one that loads the package. Build again with
`cloacinactl package build --python-version <MAJOR.MINOR>`.

### "pins dependencies in requirements.lock and has no vendored dependencies for this platform"

The package was published with a lock file and no `vendor/` folder, and the
process that loads it has no set for its platform. Either the compiler does not
vendor for this platform (add it to the compiler's `--python-platform` and
publish the package again), or nothing vendored at all (a daemon, or a
filesystem registry): build the package with `--vendor`.

### "declares dependencies in pyproject.toml … and has no requirements.lock"

Run `cloacinactl package build .` to write the lock file, then publish again.

### "has native dependencies vendored for …, and none for this platform"

The package has compiled dependencies and no `vendor/<platform>/` folder for
the platform that loads it. Build again and name every platform it runs on
(`--arch both`, or `--platform` for each).

### "vendors the module … at version …, and package …"

Two packages in one process vendor different versions of one library. See
"Two packages, one library, different versions" in Step 3.

### "package too large"

The archive is larger than the server's upload limit. See "Size" in Step 3.

### "stdlib shadowing detected"

Remove or rename any vendored module that conflicts with the Python standard
library (see the blocked list above).

### "no tasks registered" (or "Empty package: registered no tasks")

Tasks must register at **import time** with **bare `@cloaca.task` decorators at
module level** — the loader builds the workflow context from `workflow_name`, so
you do *not* wrap them in a `WorkflowBuilder` (wrapping shadows the loader's
context and the package loads with zero tasks). Tasks defined inside a function or
behind an `if __name__ == "__main__"` guard also won't be discovered on import.
The `WorkflowBuilder` in Step 4 is strictly for running the module in-process; it
is not part of the package.

## See Also

- [Creating Your First Package](../../service/creating-your-first-package/) — scaffold this layout with `cloacinactl package new`
- [Package Format](../../../explanation/engine/package-format/) — `.cloacina` archive structure
- [Running the Daemon](../running-the-daemon/) — deploying to the local scheduler
- [Deploying the API Server](../../service/deploying-the-api-server/) — deploying to the HTTP server
