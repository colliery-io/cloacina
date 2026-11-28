"""Where the checker's snapshots come from (CLOACINA-T-1108).

The checker reads two things of the system under test: its HTTP API, with one
key for each tenant, and its database. A snapshot is a plain dictionary, so it
can be written to a file and judged again later.

The database is read with `psql`, the client the driver already uses inside
the cluster. The connection details go to the child process in its
environment, never on its command line.

A source that fails is in the snapshot as `{"error": ...}`. The pass goes on,
and the rules that need it answer "cannot judge".
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import time
import urllib.error
import urllib.parse
import urllib.request

from . import workload
from .engine import parse_time

SCHEMA_NAME = re.compile(r"^[a-z][a-z0-9_]*$")
PSQL_NAMES = ("psql", "psql17", "psql16", "psql15")


class Api:
    def __init__(self, server, timeout_s=15):
        self.server = server.rstrip("/")
        self.timeout_s = timeout_s

    def get(self, path, key):
        """One GET with a key. Returns a snapshot entry."""
        request = urllib.request.Request(f"{self.server}{path}")
        request.add_header("Authorization", f"Bearer {key}")
        try:
            with urllib.request.urlopen(request, timeout=self.timeout_s) as response:
                text, status = response.read().decode("utf-8", "replace"), response.status
        except urllib.error.HTTPError as exc:
            text, status = exc.read().decode("utf-8", "replace"), exc.code
        except Exception as exc:
            return {"error": f"{exc.__class__.__name__}: {exc}"}
        try:
            body = json.loads(text) if text else None
        except ValueError:
            body = text[:2000]
        return {"status": status, "body": body}


class Psql:
    """Queries through the `psql` program."""

    def __init__(self, database_url, binary=None, timeout_s=30):
        parsed = urllib.parse.urlparse(database_url)
        if parsed.scheme not in ("postgres", "postgresql") or not parsed.hostname:
            raise ValueError("the database URL must be postgres://user:password@host:port/database")
        self.binary = binary or find_psql()
        self.timeout_s = timeout_s
        self.env = dict(os.environ)
        self.env.update({
            "PGHOST": parsed.hostname,
            "PGPORT": str(parsed.port or 5432),
            "PGUSER": urllib.parse.unquote(parsed.username or ""),
            "PGPASSWORD": urllib.parse.unquote(parsed.password or ""),
            "PGDATABASE": (parsed.path or "/").lstrip("/") or "postgres",
            "PGCONNECT_TIMEOUT": "5",
            "PGAPPNAME": "cloacina-behaviour-suite-checker",
        })

    def query(self, sql):
        """Run one SELECT. Returns a snapshot entry with the rows as a list
        of dictionaries."""
        if not self.binary:
            return {"error": "no psql program found; give its path with --psql"}
        wrapped = f"SELECT coalesce(json_agg(q), '[]'::json) FROM ({sql.strip().rstrip(';')}) q"
        try:
            proc = subprocess.run(
                [self.binary, "-X", "-q", "-t", "-A", "-v", "ON_ERROR_STOP=1", "-c", wrapped],
                env=self.env, capture_output=True, text=True, timeout=self.timeout_s, check=False)
        except Exception as exc:
            return {"error": f"{exc.__class__.__name__}: {exc}"}
        if proc.returncode != 0:
            return {"error": (proc.stderr or proc.stdout).strip()[:500] or f"psql exited {proc.returncode}"}
        try:
            return {"rows": json.loads(proc.stdout.strip() or "[]")}
        except ValueError:
            return {"error": "psql did not answer with JSON"}


class Logs:
    """Lines of pod output, read with `kubectl logs` from outside the cluster
    (CLOACINA-T-1110).

    A pod that a disturbance deletes takes its output with it. So each source
    keeps the lines it has seen in a file of its own under `store`, for
    `keep_s` seconds: a line read once stays in later snapshots after its pod
    is gone. The first read of a source asks for `keep_s` of output; later
    reads ask for the time since the last one, with a margin.
    """

    def __init__(self, kubeconfig, namespace, store, kubectl="kubectl", keep_s=3 * 3600,
                 timeout_s=60):
        self.env = dict(os.environ)
        if kubeconfig:
            self.env["KUBECONFIG"] = str(kubeconfig)
        self.namespace = namespace
        self.store = store
        self.kubectl = kubectl
        self.keep_s = keep_s
        self.timeout_s = timeout_s

    def _run(self, args):
        try:
            proc = subprocess.run([self.kubectl, *args], env=self.env, capture_output=True,
                                  text=True, timeout=self.timeout_s, check=False)
        except Exception as exc:
            return None, f"{exc.__class__.__name__}: {exc}"
        if proc.returncode != 0:
            return None, (proc.stderr or proc.stdout).strip()[:500] or f"kubectl exited {proc.returncode}"
        return proc.stdout, None

    def where(self, place, tenant):
        """(namespace, label selector) of a place."""
        if place == "agents":
            return workload.agent_namespace(tenant), workload.AGENT_SELECTOR
        return self.namespace, workload.SERVER_SELECTOR

    def _path(self, place, tenant, name):
        return os.path.join(self.store, f"{place}-{tenant or 'deployment'}-{name}.json")

    def _load(self, path):
        try:
            with open(path, "r", encoding="utf-8") as handle:
                kept = json.load(handle)
        except (OSError, ValueError):
            return {"read_at": None, "rows": []}
        return kept if isinstance(kept, dict) else {"read_at": None, "rows": []}

    def read(self, place, tenant, name, marker, now=None):
        """One snapshot entry: the kept lines that hold `marker`, and the
        ready pods of the place."""
        now = time.time() if now is None else now
        namespace, selector = self.where(place, tenant)
        path = self._path(place, tenant, name)
        kept = self._load(path)
        last = kept.get("read_at")
        since = self.keep_s if not last else int(min(self.keep_s, max(300, now - last + 120)))
        listed, error = self._run(["get", "pods", "-n", namespace, "-l", selector, "-o",
                                   "jsonpath={range .items[*]}{.metadata.name} "
                                   "{.status.containerStatuses[0].ready}{\"\\n\"}{end}"])
        if error:
            return {"error": f"kubectl get pods in {namespace}: {error}"}
        pods = sorted(line.split()[0] for line in listed.splitlines()
                      if len(line.split()) == 2 and line.split()[1] == "true")
        text, error = self._run(["logs", "-n", namespace, "-l", selector, "--all-containers=true",
                                 "--prefix", "--timestamps", "--tail=-1", f"--since={since}s",
                                 "--max-log-requests=20"])
        if error:
            return {"error": f"kubectl logs in {namespace}: {error}"}
        rows = {(row.get("pod"), row.get("at"), row.get("text")): row
                for row in kept.get("rows") or [] if isinstance(row, dict)}
        for row in parse_log_lines(text, marker):
            rows.setdefault((row["pod"], row["at"], row["text"]), row)
        fresh = sorted((row for row in rows.values()
                        if (parse_time(row.get("at")) or now) >= now - self.keep_s),
                       key=lambda row: (str(row.get("at")), str(row.get("pod"))))
        os.makedirs(self.store, exist_ok=True)
        temporary = path + ".tmp"
        with open(temporary, "w", encoding="utf-8") as handle:
            json.dump({"read_at": now, "rows": fresh}, handle)
        os.replace(temporary, path)
        return {"rows": fresh, "pods": pods}


LOG_PREFIX = re.compile(r"^\[pod/([^/\]]+)/[^\]]*\]\s+")


def parse_log_lines(text, marker):
    """The lines of `kubectl logs --prefix --timestamps` output that hold
    `marker`: `{"pod", "at", "text"}`, `text` the line with no prefix and no
    time stamp."""
    found = []
    for line in (text or "").splitlines():
        if marker not in line:
            continue
        pod = ""
        match = LOG_PREFIX.match(line)
        if match:
            pod = match.group(1)
            line = line[match.end():]
        stamp, _, rest = line.partition(" ")
        if parse_time(stamp) is None:
            stamp, rest = None, line
        found.append({"pod": pod, "at": stamp, "text": rest.strip()})
    return found


def find_psql():
    for name in PSQL_NAMES:
        found = shutil.which(name)
        if found:
            return found
    return None


def read_disturbances(path):
    """The lines of the disturbance record as dictionaries. No file is no
    disturbance."""
    if not path or not os.path.exists(path):
        return []
    found = []
    with open(path, "r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            try:
                value = json.loads(line)
            except ValueError:
                continue
            if isinstance(value, dict):
                found.append(value)
    return found


def _log_entry(logs, place, tenant, name, marker):
    if logs is None:
        return {"error": "no kubeconfig was given; the pods' output is not read"}
    if tenant is not None and not SCHEMA_NAME.match(tenant):
        return {"error": f"{tenant!r} is not a tenant name"}
    return logs.read(place, tenant, name, marker)


def fetch(url, timeout_s=15, as_json=True):
    """One GET with no key, for the channels of the harness (CLOACINA-T-1115):
    the alert receiver (JSON) and the server's metrics listener (text).
    Returns a snapshot entry: `{"body": ...}`, `{"text": ...}` or
    `{"error": ...}`."""
    try:
        with urllib.request.urlopen(url, timeout=timeout_s) as response:
            text = response.read().decode("utf-8", "replace")
    except Exception as exc:
        return {"error": f"{exc.__class__.__name__}: {exc}"}
    if not as_json:
        return {"text": text}
    try:
        return {"body": json.loads(text)}
    except ValueError:
        return {"error": "the answer is not JSON"}


def collect(registry, api, database, keys, disturbances_path=None, now=None, logs=None,
            alerts_url=None, metrics_url=None):
    """Take one snapshot: every source the registered rules need, for each
    tenant that has a key. `logs` is a `Logs` reader, or None. With
    `alerts_url` and `metrics_url`, also the alerts of the last three hours
    and the server's metrics."""
    snapshot = {
        "taken_at": time.time() if now is None else now,
        "tenants": {},
        "admin": {"db": {}, "logs": {}},
        "disturbances": read_disturbances(disturbances_path),
    }
    for name, (place, marker) in sorted(registry.log_needs.items()):
        if place == "server":
            snapshot["admin"]["logs"][name] = _log_entry(logs, place, None, name, marker)
    if alerts_url:
        since = int(float(snapshot["taken_at"])) - 3 * 3600
        snapshot["alerts"] = fetch(f"{alerts_url}?since={since}")
    if metrics_url:
        snapshot["metrics"] = fetch(metrics_url, as_json=False)
    for tenant in sorted(keys):
        entry = {"api": {}, "db": {}, "logs": {}}
        for name, (place, marker) in sorted(registry.log_needs.items()):
            if place == "agents":
                entry["logs"][name] = _log_entry(logs, place, tenant, name, marker)
        for name, path in sorted(registry.api_needs.items()):
            entry["api"][name] = api.get(path.replace("{tenant}", tenant), keys[tenant])
        for name, sql in sorted(registry.db_needs.items()):
            if not SCHEMA_NAME.match(tenant):
                entry["db"][name] = {"error": f"{tenant!r} is not a schema name"}
            elif database is None:
                entry["db"][name] = {"error": "no database URL was given"}
            else:
                entry["db"][name] = database.query(sql.replace("{schema}", tenant))
        snapshot["tenants"][tenant] = entry
    for name, sql in sorted(registry.admin_db_needs.items()):
        if database is None:
            snapshot["admin"]["db"][name] = {"error": "no database URL was given"}
        else:
            snapshot["admin"]["db"][name] = database.query(sql)
    return snapshot


def write_snapshot(folder, snapshot):
    """Keep a snapshot as a file, named by its time. Returns the path."""
    os.makedirs(folder, exist_ok=True)
    stamp = time.strftime("%Y%m%dT%H%M%SZ", time.gmtime(float(snapshot["taken_at"])))
    path = os.path.join(folder, f"snapshot-{stamp}.json")
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(snapshot, handle, indent=1, sort_keys=True)
    return path


def read_snapshot(path):
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)
