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


def collect(registry, api, database, keys, disturbances_path=None, now=None):
    """Take one snapshot: every source the registered rules need, for each
    tenant that has a key."""
    snapshot = {
        "taken_at": time.time() if now is None else now,
        "tenants": {},
        "admin": {"db": {}},
        "disturbances": read_disturbances(disturbances_path),
    }
    for tenant in sorted(keys):
        entry = {"api": {}, "db": {}}
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
