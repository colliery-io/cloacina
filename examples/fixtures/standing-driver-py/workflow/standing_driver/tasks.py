"""The self-driving workflow of the behaviour suite (CLOACINA-T-1107).

It runs every five minutes in each tenant of the standing workload, and acts
on the server it runs on, the way a team's own automation does:

- it publishes new versions of two packages of its tenant, a Python one and a
  Rust one. Their source is in this file;
- once an hour one of those publishes has a build error;
- once an hour it goes back one version;
- it starts other workflows, several at once, to fill its tenant's agents;
- every time, it starts one run by hand through the API, and tries its key on
  another tenant.

It writes what it did into its context, under `standing_driver_actions`: a
list of small records with names, versions and status codes. It never writes a
key or a secret value. The suite's checker reads that list and judges the
promises against it. This workflow judges nothing.

Inputs, bound by the workflow instance that schedules it:

- `tenant`: the tenant it runs in.
- `offset`: a shift in its hourly cycle, so tenants do not act together.
- `other_tenant`: another tenant, to try its key on.
- the secret `standing_driver_key`, field `key`: a write key of its tenant.

The cycle and the names below are also in the suite's description of the
workload (`.angreal/test/bdd/suite/workload.py`); a unit test compares them.
"""
from __future__ import annotations

import io
import json
import os
import socket
import tarfile
import time
import urllib.error
import urllib.request
import uuid
from datetime import datetime, timezone

import cloaca

# Also for a Python older than 3.11, which has no `datetime.UTC`.
UTC = timezone.utc  # noqa: UP017

WORKFLOW = "standing_driver"
SECRET = "standing_driver_key"
ACTIONS_KEY = "standing_driver_actions"
PLAN_KEY = "standing_driver_plan"

SLOT_SECONDS = 300
CYCLE_SLOTS = 12
SLOT_ACTIONS = {
    0: ("publish", "py"),
    3: ("publish_broken", "rust"),
    4: ("start_holds", ""),
    6: ("publish", "py"),
    7: ("step_back", "py"),
    9: ("publish", "rust"),
}

TARGET_PY = "standing-target-py"
TARGET_PY_WORKFLOW = "standing_target_py"
TARGET_RUST = "standing-target-rust"
TARGET_RUST_WORKFLOW = "standing_target_rust"
TARGETS = {"py": TARGET_PY, "rust": TARGET_RUST}
TARGET_WORKFLOWS = {"py": TARGET_PY_WORKFLOW, "rust": TARGET_RUST_WORKFLOW}

HOLD_WORKFLOW = "standing_hold"
HOLD_SECONDS = 120
HOLD_BURST = 12

# The name the broken Rust version calls and does not define: rustc names it.
BROKEN_MARK = "standing_target_total_that_does_not_exist"

# --- the source of the two target packages ----------------------------------

PY_PACKAGE_TOML = """\
[package]
name = "standing-target-py"
version = "@VERSION@"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
workflow_name = "standing_target_py"
language = "python"
description = "standing workload: a package the self-driving workflow publishes"
author = "behaviour-suite"
entry_module = "standing_target_py.tasks"
"""

PY_TASKS = '''\
"""A package the self-driving workflow of the behaviour suite publishes
(CLOACINA-T-1107). Its task writes the version of the code that ran it."""
from __future__ import annotations

import cloaca

PACKAGE_VERSION = "@VERSION@"


@cloaca.task(dependencies=[])
def report_version(context):
    context.set("standing_target_version", PACKAGE_VERSION)
    return context
'''

RUST_PACKAGE_TOML = """\
[package]
name = "standing-target-rust"
version = "@VERSION@"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
workflow_name = "standing_target_rust"
language = "rust"
description = "standing workload: a package the self-driving workflow publishes"
author = "behaviour-suite"
"""

RUST_CARGO_TOML = """\
[workspace]

[package]
name = "standing-target-rust"
version = "@VERSION@"
edition = "2024"
rust-version = "1.99"

[package.metadata.cargo-machete]
ignored = ["cloacina-macros"]

[lib]
crate-type = ["cdylib", "rlib"]

[features]
default = ["packaged"]
packaged = []

[dependencies]
cloacina-macros = "0.12"
cloacina-workflow = { version = "0.12", features = ["packaged"] }
cloacina-workflow-plugin = "0.12"
serde_json = "1.0"

[build-dependencies]
cloacina-build = "0.12"
"""

RUST_BUILD_RS = """\
fn main() {
    cloacina_build::configure();
}
"""

RUST_LIB_RS = """\
// A package the self-driving workflow of the behaviour suite publishes
// (CLOACINA-T-1107). Its task writes the version of the code that ran it.

use cloacina_macros::{task, workflow};
use cloacina_workflow::{Context, TaskError};

cloacina_workflow_plugin::package!();

#[workflow(
    name = "standing_target_rust",
    description = "standing workload: a package the self-driving workflow publishes",
    author = "behaviour-suite"
)]
pub mod standing_target_rust {
    use super::*;

    #[task(retry_attempts = 0)]
    pub async fn report_version(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
@BODY@
        context.insert("standing_target_version", serde_json::json!("@VERSION@"))?;
        Ok(())
    }
}
"""

RUST_GOOD_BODY = "        // Nothing more: this version builds."
RUST_BROKEN_BODY = (
    "        // Deliberate: this function is not defined anywhere.\n"
    "        let _total = standing_target_total_that_does_not_exist();"
)


def target_files(kind, version, broken=False):
    """The files of one version of a target package: path -> text."""
    if kind == "py":
        return {
            "package.toml": PY_PACKAGE_TOML.replace("@VERSION@", version),
            "workflow/standing_target_py/__init__.py": "",
            "workflow/standing_target_py/tasks.py": PY_TASKS.replace("@VERSION@", version),
        }
    if kind == "rust":
        body = RUST_BROKEN_BODY if broken else RUST_GOOD_BODY
        return {
            "package.toml": RUST_PACKAGE_TOML.replace("@VERSION@", version),
            "Cargo.toml": RUST_CARGO_TOML.replace("@VERSION@", version),
            "build.rs": RUST_BUILD_RS,
            "src/lib.rs": RUST_LIB_RS.replace("@BODY@", body).replace("@VERSION@", version),
        }
    raise ValueError(f"unknown target kind: {kind}")


def build_archive(name, version, files):
    """A package archive as `cloacinactl package pack` makes it: a bzip2 tar
    of the package's files under `<name>-<version>/`, with no entry for a
    folder."""
    prefix = f"{name}-{version}"
    buffer = io.BytesIO()
    now = int(time.time())
    with tarfile.open(fileobj=buffer, mode="w:bz2") as tar:
        for rel in sorted(files):
            data = files[rel].encode("utf-8")
            info = tarfile.TarInfo(f"{prefix}/{rel}")
            info.size = len(data)
            info.mode = 0o644
            info.mtime = now
            tar.addfile(info, io.BytesIO(data))
    return buffer.getvalue()


# --- the plan ----------------------------------------------------------------

def _epoch(text):
    """An RFC 3339 time as seconds since the epoch, or None."""
    if not text or not isinstance(text, str):
        return None
    try:
        # Before Python 3.11, `fromisoformat` does not read a trailing Z.
        return datetime.fromisoformat(text.replace("Z", "+00:00")).timestamp()  # noqa: FURB162
    except ValueError:
        return None


def slot_of(epoch_seconds, offset=0):
    return (int(epoch_seconds) // SLOT_SECONDS + int(offset)) % CYCLE_SLOTS


def plan_actions(slot):
    """What one run does, in order, for its slot of the cycle."""
    actions = []
    if slot in SLOT_ACTIONS:
        action, kind = SLOT_ACTIONS[slot]
        actions.append({"action": action, "kind": kind})
    actions.append({"action": "run_by_hand", "kind": "py"})
    actions.append({"action": "other_tenant", "kind": ""})
    return actions


def next_version(rows):
    """The version after the newest one in a versions list: 0.1.N."""
    newest = -1
    for row in rows or []:
        parts = str(row.get("version", "")).split(".")
        if len(parts) == 3 and parts[0] == "0" and parts[1] == "1" and parts[2].isdigit():
            newest = max(newest, int(parts[2]))
    return f"0.1.{newest + 1}"


def _version_key(version):
    return tuple(int(part) if part.isdigit() else 0 for part in str(version).split("."))


def version_to_go_back_to(rows):
    """The newest good build that is older than the version in service."""
    rows = rows or []
    active = next((row for row in rows if row.get("active")), None)
    if active is None:
        return None
    older = [row for row in rows
             if row.get("build_status") == "success"
             and _version_key(row.get("version")) < _version_key(active.get("version"))]
    if not older:
        return None
    return max(older, key=lambda row: _version_key(row.get("version"))).get("version")


# --- the server ---------------------------------------------------------------

def _now():
    return datetime.now(UTC).strftime("%Y-%m-%dT%H:%M:%SZ")


def _call(server, key, method, path, body=None, raw=None, content_type=None, timeout=30):
    """One API call. Returns (status, parsed body or None). Never raises."""
    data = raw
    request = urllib.request.Request(f"{server.rstrip('/')}{path}", method=method)
    request.add_header("Authorization", f"Bearer {key}")
    if body is not None:
        data = json.dumps(body).encode("utf-8")
        request.add_header("Content-Type", "application/json")
    elif content_type:
        request.add_header("Content-Type", content_type)
    try:
        with urllib.request.urlopen(request, data=data, timeout=timeout) as response:
            text, status = response.read().decode("utf-8", "replace"), response.status
    except urllib.error.HTTPError as exc:
        text, status = exc.read().decode("utf-8", "replace"), exc.code
    except Exception:  # noqa: BLE001 - a call that fails is a record, not a crash
        return None, None
    try:
        return status, (json.loads(text) if text else None)
    except ValueError:
        return status, None


def _upload(server, key, tenant, filename, archive):
    boundary = "standing" + uuid.uuid4().hex
    head = (
        f"--{boundary}\r\n"
        f'Content-Disposition: form-data; name="file"; filename="{filename}"\r\n'
        "Content-Type: application/octet-stream\r\n\r\n"
    ).encode()
    tail = f"\r\n--{boundary}--\r\n".encode()
    status, _ = _call(server, key, "POST", f"/v1/tenants/{tenant}/workflows",
                      raw=head + archive + tail,
                      content_type=f"multipart/form-data; boundary={boundary}", timeout=60)
    return status


def _versions(server, key, tenant, package):
    status, body = _call(server, key, "GET", f"/v1/tenants/{tenant}/workflows/{package}/versions")
    if status == 200 and isinstance(body, dict):
        return body.get("items", [])
    return []


# --- the actions --------------------------------------------------------------

def _publish(server, key, tenant, kind, broken=False):
    package = TARGETS[kind]
    version = next_version(_versions(server, key, tenant, package))
    archive = build_archive(package, version, target_files(kind, version, broken=broken))
    status = _upload(server, key, tenant, f"{package}-{version}.cloacina", archive)
    return {"action": "publish_broken" if broken else "publish", "package": package,
            "version": version, "status": status, "bytes": len(archive)}


def _step_back(server, key, tenant, kind):
    package = TARGETS[kind]
    version = version_to_go_back_to(_versions(server, key, tenant, package))
    if version is None:
        return {"action": "step_back", "package": package, "version": None, "status": None,
                "skipped": "no older good build"}
    status, _ = _call(server, key, "POST",
                      f"/v1/tenants/{tenant}/workflows/{package}/versions/{version}/activate", body={})
    return {"action": "step_back", "package": package, "version": version, "status": status}


def _run_by_hand(server, key, tenant, workflow, context):
    status, body = _call(server, key, "POST", f"/v1/tenants/{tenant}/workflows/{workflow}/execute",
                         body={"context": context})
    execution = body.get("execution_id") if isinstance(body, dict) else None
    return {"action": "run_by_hand", "workflow": workflow, "status": status, "execution_id": execution}


def _start_holds(server, key, tenant):
    statuses = []
    for _ in range(HOLD_BURST):
        # The tag names the run in the marks its task prints (CLOACINA-T-1110).
        context = {"hold_seconds": HOLD_SECONDS, "run_once_tag": f"hold-{uuid.uuid4()}"}
        status, _body = _call(server, key, "POST",
                              f"/v1/tenants/{tenant}/workflows/{HOLD_WORKFLOW}/execute",
                              body={"context": context})
        statuses.append(status)
    return {"action": "start_holds", "workflow": HOLD_WORKFLOW, "count": HOLD_BURST,
            "statuses": statuses}


def _other_tenant(context, server, key, other_tenant):
    """Try this tenant's key, and this task's reach, on another tenant.
    Status codes and outcome names only."""
    record = {"action": "other_tenant", "other_tenant": other_tenant}
    if not other_tenant:
        record["skipped"] = "no other tenant bound"
        return record
    record["packages_status"], _ = _call(server, key, "GET", f"/v1/tenants/{other_tenant}/workflows")
    record["executions_status"], _ = _call(server, key, "GET", f"/v1/tenants/{other_tenant}/executions")
    record["secrets_status"], _ = _call(server, key, "GET", f"/v1/tenants/{other_tenant}/secrets")
    try:
        # The other tenant's secret has the same name as this tenant's own, so
        # ask for a name that only the other tenant could give a meaning to.
        context.secret(f"{other_tenant}_only")
        record["secret_by_name"] = "read"
    except Exception as exc:  # noqa: BLE001
        record["secret_by_name"] = exc.__class__.__name__
    return record


def _mark(context, task):
    """One marked line for each start of a task body."""
    line = "STANDING_MARK " + json.dumps({
        "workflow": WORKFLOW, "task": task, "scheduled_time": context.get("scheduled_time", None),
        "host": socket.gethostname(), "pid": os.getpid(), "at": time.time()}, sort_keys=True)
    print(line, flush=True)


# --- the tasks ----------------------------------------------------------------

@cloaca.workflow_params(
    tenant=(str, ""),
    offset=(int, 0),
    other_tenant=(str, ""),
)
@cloaca.task(dependencies=[], retry_attempts=0)
def plan(context):
    _mark(context, "plan")
    fired = _epoch(context.get("scheduled_time", None)) or time.time()
    slot = slot_of(fired, context.get("offset", 0) or 0)
    context.set(PLAN_KEY, {"slot": slot, "actions": plan_actions(slot)})
    return context


@cloaca.workflow_secrets("standing_driver_key")
@cloaca.task(dependencies=["plan"], retry_attempts=0, timeout_seconds=240)
def act(context):
    _mark(context, "act")
    tenant = context.get("tenant", "") or ""
    server = os.environ.get("CLOACINA_SERVER", "")
    records = []

    def note(record):
        record["at"] = _now()
        records.append(record)

    try:
        key = context.secret(SECRET)["key"]
    except Exception as exc:  # noqa: BLE001
        note({"action": "read_key", "error": exc.__class__.__name__})
        context.set(ACTIONS_KEY, records)
        return context
    if not tenant or not server:
        note({"action": "configure", "error": "no tenant or no server"})
        context.set(ACTIONS_KEY, records)
        return context

    # The first run of a tenant publishes both targets, whatever the slot.
    for kind in ("py", "rust"):
        if not _versions(server, key, tenant, TARGETS[kind]):
            note(_publish(server, key, tenant, kind))

    plan_ = context.get(PLAN_KEY, None) or {"actions": plan_actions(-1)}
    for step in plan_.get("actions", []):
        action, kind = step.get("action"), step.get("kind")
        try:
            if action == "publish":
                note(_publish(server, key, tenant, kind))
            elif action == "publish_broken":
                note(_publish(server, key, tenant, kind, broken=True))
            elif action == "step_back":
                note(_step_back(server, key, tenant, kind))
            elif action == "start_holds":
                note(_start_holds(server, key, tenant))
            elif action == "run_by_hand":
                note(_run_by_hand(server, key, tenant, TARGET_WORKFLOWS[kind], {}))
            elif action == "other_tenant":
                note(_other_tenant(context, server, key, context.get("other_tenant", "") or ""))
        except Exception as exc:  # noqa: BLE001 - one action that fails does not stop the rest
            note({"action": action, "error": exc.__class__.__name__})

    context.set(ACTIONS_KEY, records)
    return context
