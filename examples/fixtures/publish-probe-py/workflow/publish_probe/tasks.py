"""A probe job for the publishing behaviour spec (CLOACINA-T-1026).

Team A publishes this job. Its task tries to reach what belongs to another
team and writes down what it could reach. The spec reads the report and
decides what is a rough spot.

The report holds names, counts and status codes only. It never holds a secret
value, a key, a file's content or the value of an environment variable.

What to try comes from the execution context:

- `other_tenant`: the other team's tenant.
- `other_digest`: the content digest of a package of the other team.
- `other_secret`: the name of a secret of the other team.
- `own_digest`, `own_secret`: the same for this team, as controls. A control
  that fails says the attempt itself is broken, not that the wall holds.
- `database_host`, `database_port`: where the orchestrator's database listens.
- `metrics_url`: the orchestrator's metrics page.
- `other_namespace`: the Kubernetes namespace of the other team's agents.
- `internet_host`, `internet_port`: a public address, to see if the task can
  reach the internet.
- `needles`: strings that name the other team's things, to look for in the
  environment and on disk.

The module also notes where it was imported, because a package's code runs at
import time in every process that loads it, not only in the agent.
"""
from __future__ import annotations

import json
import os
import socket
import ssl
import sys
import urllib.error
import urllib.request

import cloaca

REPORT_KEY = "publish_probe_report"
IMPORT_MARKER = "PUBLISH_PROBE_IMPORT"
REPORT_MARKER = "PUBLISH_PROBE_REPORT"

# Names that give their holder the orchestrator's own reach.
ORCHESTRATOR_CREDENTIALS = (
    "DATABASE_URL",
    "CLOACINA_BOOTSTRAP_KEY",
    "CLOACINA_SECRET_KEK",
    "PGPASSWORD",
)
SERVICE_ACCOUNT_TOKEN = "/var/run/secrets/kubernetes.io/serviceaccount/token"
KUBERNETES_API = "https://kubernetes.default.svc"
MAX_FILES = 3000
MAX_CONTENT_BYTES = 262144


def _process_name():
    try:
        with open("/proc/self/cmdline", "rb") as handle:
            first = handle.read().split(b"\0")[0].decode("utf-8", "replace")
        return os.path.basename(first)
    except Exception:  # noqa: BLE001
        return os.path.basename(sys.argv[0]) if sys.argv and sys.argv[0] else "unknown"


def _where(moment):
    """Which process runs this code, and which credentials it can see."""
    token_readable = False
    try:
        with open(SERVICE_ACCOUNT_TOKEN, "rb") as handle:
            token_readable = bool(handle.read(1))
    except Exception:  # noqa: BLE001
        token_readable = False
    return {
        "moment": moment,
        "process": _process_name(),
        "hostname": socket.gethostname(),
        "orchestrator_credentials_in_environment": [
            name for name in ORCHESTRATOR_CREDENTIALS if os.environ.get(name)
        ],
        "kubernetes_token_readable": token_readable,
    }


def _emit(marker, payload):
    """One marked line on stdout and stderr, so the pod's log carries it."""
    line = f"{marker} {json.dumps(payload, sort_keys=True)}"
    for stream in (sys.stdout, sys.stderr):
        try:
            print(line, file=stream, flush=True)
        except Exception:  # noqa: BLE001
            pass


try:
    IMPORT_FACTS = _where("import")
    _emit(IMPORT_MARKER, IMPORT_FACTS)
except Exception as exc:  # noqa: BLE001 - the probe must never break the load
    IMPORT_FACTS = {"moment": "import", "error": exc.__class__.__name__}


def _http_status(server, key, path):
    """GET `path` with the key of this process. Status and size, no body."""
    request = urllib.request.Request(f"{server.rstrip('/')}{path}")
    request.add_header("Authorization", f"Bearer {key}")
    try:
        with urllib.request.urlopen(request, timeout=5) as response:
            return {"path": path, "status": response.status, "bytes": len(response.read())}
    except urllib.error.HTTPError as exc:
        return {"path": path, "status": exc.code, "bytes": 0}
    except Exception as exc:  # noqa: BLE001
        return {"path": path, "status": None, "error": exc.__class__.__name__}


def _plain_status(url, token=None):
    """GET a URL with no cloacina key (and a bearer token when given). Status
    and size only. Certificates are not checked: this asks "can I reach it"."""
    request = urllib.request.Request(url)
    if token:
        request.add_header("Authorization", f"Bearer {token}")
    try:
        unverified = ssl._create_unverified_context()
        with urllib.request.urlopen(request, timeout=5, context=unverified) as response:
            return {"url": url, "status": response.status, "bytes": len(response.read())}
    except urllib.error.HTTPError as exc:
        return {"url": url, "status": exc.code, "bytes": 0}
    except Exception as exc:  # noqa: BLE001
        return {"url": url, "status": None, "error": exc.__class__.__name__}


def _kubernetes(own_namespace_hint, other_namespace):
    """What the pod's service account can do in the Kubernetes API. Status
    codes only; the token itself is never reported."""
    token = None
    try:
        with open(SERVICE_ACCOUNT_TOKEN, "r", encoding="utf-8") as handle:
            token = handle.read().strip()
    except Exception:  # noqa: BLE001
        token = None
    own_namespace = own_namespace_hint
    try:
        with open(SERVICE_ACCOUNT_TOKEN.replace("token", "namespace"), "r", encoding="utf-8") as handle:
            own_namespace = handle.read().strip()
    except Exception:  # noqa: BLE001
        pass
    out = {
        "token_readable": bool(token),
        "own_namespace": own_namespace,
        "version_no_token": _plain_status(f"{KUBERNETES_API}/version"),
    }
    if token:
        out["list_namespaces"] = _plain_status(f"{KUBERNETES_API}/api/v1/namespaces", token)
        if own_namespace:
            out["own_namespace_secrets"] = _plain_status(
                f"{KUBERNETES_API}/api/v1/namespaces/{own_namespace}/secrets", token)
        if other_namespace:
            out["other_namespace_secrets"] = _plain_status(
                f"{KUBERNETES_API}/api/v1/namespaces/{other_namespace}/secrets", token)
            out["other_namespace_pods"] = _plain_status(
                f"{KUBERNETES_API}/api/v1/namespaces/{other_namespace}/pods", token)
    return out


def _filesystem():
    """What this process can see and write on its disk. Names and counts only."""
    out = {"uid": os.getuid(), "cwd": os.getcwd()}
    writable = []
    for path in ("/", "/tmp", "/home/cloacina", "/usr/local/bin", "/etc",
                 os.environ.get("CLOACINA_AGENT_CACHE_DIR", "")):
        if path and os.access(path, os.W_OK):
            writable.append(path)
    out["writable"] = writable
    cache = os.environ.get("CLOACINA_AGENT_CACHE_DIR", "")
    try:
        out["agent_cache_entries"] = len(os.listdir(cache)) if cache else None
    except Exception as exc:  # noqa: BLE001
        out["agent_cache_entries"] = exc.__class__.__name__
    readable = []
    for path in ("/proc/1/environ", "/proc/self/environ", SERVICE_ACCOUNT_TOKEN,
                 "/etc/shadow", "/var/run/docker.sock"):
        try:
            with open(path, "rb") as handle:
                handle.read(1)
            readable.append(path)
        except Exception:  # noqa: BLE001
            pass
    out["readable"] = readable
    return out


def _read_secret(context, name):
    """Ask the runtime for a secret by name. Field names only, never a value."""
    if not name:
        return {"name": name, "outcome": "not tried"}
    try:
        fields = context.secret(name)
        return {
            "name": name,
            "outcome": "read",
            "fields": sorted(str(k) for k in dict(fields)),
        }
    except Exception as exc:  # noqa: BLE001
        return {"name": name, "outcome": exc.__class__.__name__}


def _tcp_reach(host, port):
    """Can this process open a TCP connection to host:port? No bytes sent."""
    if not host or not port:
        return {"host": host, "port": port, "outcome": "not tried"}
    try:
        with socket.create_connection((host, int(port)), timeout=5):
            return {"host": host, "port": int(port), "outcome": "connected"}
    except Exception as exc:  # noqa: BLE001
        return {"host": host, "port": port, "outcome": exc.__class__.__name__}


def _scan_for_needles(needles):
    """Count where the other team's names show up in this process's view.

    Environment variable names and values, and file paths under the writable
    roots. Reports counts and the first few paths, never the content.
    """
    needles = [n for n in (needles or []) if n]
    env_hits = []
    for key, value in os.environ.items():
        for needle in needles:
            if needle in key or needle in value:
                env_hits.append(key)
                break
    path_hits = []
    scanned = 0
    for root in ("/home/cloacina", "/tmp", os.getcwd()):
        for base, _dirs, files in os.walk(root):
            for name in files:
                scanned += 1
                if scanned > MAX_FILES:
                    break
                full = os.path.join(base, name)
                if any(needle in full for needle in needles):
                    path_hits.append(full)
            if scanned > MAX_FILES:
                break
    return {
        "needles": needles,
        "environment_name_or_value_hits": sorted(set(env_hits)),
        "path_hits": path_hits[:10],
        "files_scanned": scanned,
    }


@cloaca.workflow_secrets("report_key")
@cloaca.task(dependencies=[])
def probe(context):
    server = os.environ.get("CLOACINA_SERVER", "")
    key = os.environ.get("CLOACINA_API_KEY", "")
    other_tenant = context.get("other_tenant", "") or ""
    own_tenant = context.get("own_tenant", "") or ""
    own_digest = context.get("own_digest", "") or ""
    other_digest = context.get("other_digest", "") or ""

    report = {
        "import": IMPORT_FACTS,
        "run": _where("run"),
        "fetch_own_artifact": _http_status(server, key, f"/v1/agent/artifact/{own_digest}")
        if own_digest
        else {"outcome": "not tried"},
        "fetch_other_artifact": _http_status(server, key, f"/v1/agent/artifact/{other_digest}")
        if other_digest
        else {"outcome": "not tried"},
        "fetch_other_source": _http_status(server, key, f"/v1/agent/source/{other_digest}")
        if other_digest
        else {"outcome": "not tried"},
        "list_other_workflows": _http_status(
            server, key, f"/v1/tenants/{other_tenant}/workflows"
        )
        if other_tenant
        else {"outcome": "not tried"},
        "list_other_executions": _http_status(
            server, key, f"/v1/tenants/{other_tenant}/executions"
        )
        if other_tenant
        else {"outcome": "not tried"},
        "read_own_secret": _read_secret(context, context.get("own_secret", "")),
        "read_other_secret": _read_secret(context, context.get("other_secret", "")),
        "list_other_secrets": _http_status(
            server, key, f"/v1/tenants/{other_tenant}/secrets"
        )
        if other_tenant
        else {"outcome": "not tried"},
        "reach_database": _tcp_reach(
            context.get("database_host", ""), context.get("database_port", "")
        ),
        "scan": _scan_for_needles(context.get("needles", [])),
        # What else the task's own agent key opens. Status and size only.
        "agent_key_present": bool(key),
        "fetch_other_providers": _http_status(server, key, f"/v1/agent/providers/{other_digest}")
        if other_digest
        else {"outcome": "not tried"},
        "list_own_secrets": _http_status(server, key, f"/v1/tenants/{own_tenant}/secrets")
        if own_tenant
        else {"outcome": "not tried"},
        "list_own_keys": _http_status(server, key, f"/v1/tenants/{own_tenant}/keys")
        if own_tenant
        else {"outcome": "not tried"},
        "list_own_workflows": _http_status(server, key, f"/v1/tenants/{own_tenant}/workflows")
        if own_tenant
        else {"outcome": "not tried"},
        "list_all_tenants": _http_status(server, key, "/v1/tenants"),
        "list_all_agents": _http_status(server, key, "/v1/agents"),
        "compiler_status": _http_status(server, key, "/v1/compiler/status"),
        # Names only, never a value.
        "environment_names": sorted(os.environ.keys()),
        "filesystem": _filesystem(),
        "network": {
            "server_metrics": _plain_status(context.get("metrics_url", "") or f"{server.rstrip('/')}/metrics"),
            "server_health": _plain_status(f"{server.rstrip('/')}/health"),
            "kubernetes": _kubernetes("", context.get("other_namespace", "") or ""),
            "internet": _tcp_reach(context.get("internet_host", ""), context.get("internet_port", "")),
        },
    }

    _emit(REPORT_MARKER, report)
    context.set(REPORT_KEY, report)
    return context
