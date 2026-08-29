# Copyright 2026 Cloacina Contributors
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.

"""Multi-replica leadership e2e for the fleet control plane (CLOACI-T-0818).

Proves the design recorded in ADR CLOACI-A-0008 against a REAL 2-replica k3s
deployment: an *in-process advisory-lock leader* gates ONLY the fleet control
loop (autoscale + reconcile), while the API and the per-task scheduler scale
freely across every replica. The T-0815 1-replica soak could not test this —
its `advisory_holder` was always null. At replicaCount=2 the advisory lock is
the real test.

Reuses the T-0815/T-0816 k3s platform helpers verbatim
(`.angreal/test/e2e/k8s_fleet.py`): the same `docker-compose.k8s.yaml` (k3s +
registry), the same image build/push, the same chart RBAC. The ONLY deltas are
`replicaCount=2` and a distinct compose project + port (18096) so this lane
cannot clash with the e2e (18092) / soak (18094) platforms.

Assertions (priority order):
  1. BOTH replicas Ready — 2/2 server pods Running + readyReplicas==2, `/ready`
     healthy through the Service.
  2. SINGLE LEADER — the fleet advisory lock (key 8110127) is held by AT MOST
     one Postgres connection at any instant. Sampled at high frequency against
     `pg_locks`; the holder's `client_addr` is mapped to the owning server pod.
     (Leadership is per-tick: the loop takes `pg_try_advisory_lock` at the start
     of each control tick and releases it at the end — see
     crates/cloacina-server/src/autoscaler/leader.rs — so the validated
     invariant is "never two simultaneous holders", and the leader may legitimately
     differ tick-to-tick.)
  3. SINGLE-WRITER PROVISIONING — create a tenant, set its limit, provision N
     agents via REST; the (leader-only) reconcile actuates the tenant Deployment
     to EXACTLY N (not 2N) despite two server replicas. Deprovision → scales down.
  4. DISJOINT CLAIMING — drive workflow executions and assert each task runs
     exactly once across both replicas' schedulers. The AUTHORITATIVE proof of
     this property lives in the DAL integration test
     `dal::task_claiming::test_concurrent_task_claiming_no_duplicates`
     (crates/cloacina/tests/integration/dal/task_claiming.rs), which runs N
     concurrent claimers on separate pooled connections against the same ready
     outbox rows and asserts disjoint + complete claiming with zero slack — run
     it via `angreal test integration --backend postgres -- task_claiming`. This
     full-stack e2e path is BEST-EFFORT / opt-in (`--claiming`): a helm-only
     cloacina-server deploy ships NO compiler, so source `.cloacina` packages
     never build → cannot execute. See the BLOCKED note + the per-claim SQL in
     `_assert_disjoint_claiming`.
  5. FAILOVER — delete the lock-holding replica; assert the surviving replica
     acquires the lock (control plane keeps working: provisioning still scales
     correctly) and the killed replica reschedules + rejoins as a follower
     (2/2 Ready again, lock holder count stays <=1).

Run with:  angreal test e2e k8s-leader               # build images + full run
           angreal test e2e k8s-leader --skip-build  # reuse retained k8s-soak images
           angreal test e2e k8s-leader --no-cleanup   # keep the cluster
           angreal test e2e k8s-leader --claiming     # also attempt assertion 4
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.request
import uuid
from pathlib import Path

import angreal  # type: ignore

from .._utils import print_section_header, print_final_success

# Reuse the k3s platform bring-up + image/kubectl helpers verbatim — this lane
# MUST exercise the SAME real-RBAC chart path as the e2e/soak (don't duplicate
# brittle logic). Only the helm values (replicaCount=2) + identity differ.
from .k8s_fleet import (
    CHART_DIR,
    COMPOSE_FILE,
    REGISTRY_HOST,
    REGISTRY_K8S,
    _check_tool,
    _compose,
    _image_exists,
    _kube_env,
    _kubectl,
    _kubectl_json,
    _prepare_images,
    _run,
    _server_values,
    _wait_http,
    bring_up_cluster,
    start_port_forward,
)

test = angreal.command_group(
    name="test", about="Cloacina test suites (unit, integration, e2e, soak)"
)
e2e = angreal.command_group(name="e2e", about="end-to-end tests against a live server")

# --- this lane's identity (distinct from e2e/soak so platforms never clash) ---
RELEASE = "fleet-leader"
NS = "cloacina-leader-e2e"
TENANT = "acme"
TENANT_NS = f"cloacina-tenant-{TENANT}"
BOOTSTRAP_KEY = "k8s-leader-e2e-bootstrap-key"
BOOTSTRAP_SECRET = "cloacina-bootstrap"
FWD_PORT = 18096
REPLICAS = 2

# Advisory-lock key the fleet control loop leader-elects on
# (crates/cloacina-server/src/autoscaler/leader.rs::FLEET_CONTROL_LOCK_KEY).
FLEET_LOCK_KEY = 8110127


def reactor_lock_key(tenant, name):
    """Python port of `cloacina::computation_graph::reactor_lock_key`.

    Must agree bit-for-bit with the Rust implementation — a divergent key here
    would make the harness watch a lock nobody holds, and (per the
    advisory-lock trap documented on `advisory_lock_parts`) a lock query that
    matches nothing PASSES a "single holder" assertion. The self-check below
    pins this port to the exact literals the Rust stability test pins, so the
    two implementations cannot drift apart silently.
    """
    fnv_offset = 0xCBF2_9CE4_8422_2325
    fnv_prime = 0x0000_0100_0000_01B3
    mask = 0xFFFF_FFFF_FFFF_FFFF

    buf = bytearray(b"cloacina.reactor.ownership.v1\0")
    if tenant is not None:
        t = tenant.encode()
        buf.append(1)
        buf += len(t).to_bytes(8, "big")
        buf += t
    else:
        buf.append(0)
    n = name.encode()
    buf += len(n).to_bytes(8, "big")
    buf += n

    h = fnv_offset
    for b in buf:
        h ^= b
        h = (h * fnv_prime) & mask
    h |= 1 << 63
    # Reinterpret the u64 as i64 (the sign bit is forced, so always negative).
    return h - (1 << 64)


# Cross-language pin: these literals are asserted by the Rust test
# `reactor_lock_key::tests::keys_are_stable_across_processes`. If either side
# changes its encoding, this import fails loudly instead of the harness
# silently watching the wrong lock.
assert reactor_lock_key("public", "orders_reactor") == -798_654_939_832_276_275, \
    "python reactor_lock_key diverged from the Rust implementation (tenant case)"
assert reactor_lock_key(None, "orders_reactor") == -6_219_432_407_812_253_675, \
    "python reactor_lock_key diverged from the Rust implementation (untenanted case)"


def lock_query(key):
    """psql for 'who currently holds advisory lock `key`', as (client_addr, pid).

    Parameterized by key because CLOACI-T-0851 adds per-reactor ownership locks
    alongside the fleet control lock, and those assertions want this lane's
    existing machinery — `_wait_lock_holder`'s "never two simultaneous holders"
    invariant and `_sample_lock`'s high-frequency sampling — rather than a
    second, subtly-different copy of it.

    Matches on the FULL key (classid + objid + objsubid), not `objid` alone —
    see `advisory_lock_parts` for why matching a partial key is a correctness
    bug in a test rather than a rounding error.
    """
    classid, objid, objsubid = advisory_lock_parts(key)
    return (
        "SELECT a.client_addr, a.pid FROM pg_locks l "
        "JOIN pg_stat_activity a ON l.pid=a.pid "
        f"WHERE l.locktype='advisory' AND l.classid={classid} AND l.objid={objid} "
        f"AND l.objsubid={objsubid} AND l.granted;"
    )


def advisory_lock_parts(key):
    """Split a 64-bit advisory key the way `pg_locks` reports it.

    Postgres does not store a bigint advisory key in one column. For the
    single-argument `pg_advisory_lock(bigint)` form it reports
    `classid` = high 32 bits, `objid` = low 32 bits, `objsubid` = 1. (The
    two-int4 form uses objsubid = 2, which we do not use.)

    Matching on `objid` alone is wrong in BOTH directions and both failures are
    quiet:

      * Under-match — a full i64 compared against the 32-bit `objid` column
        matches nothing. A lock query that matches nothing PASSES a "never two
        simultaneous holders" assertion, so the test goes green while proving
        the absence of the thing it was meant to observe.
      * Over-match — two distinct keys sharing their low 32 bits look like the
        same lock. Reactor keys have the sign bit forced, so every one of them
        shares its high bits with the others; the low word is all that
        distinguishes them, and a partial match would happily conflate a
        reactor lock with the fleet lock.

    Returned as unsigned, which is how `pg_locks` exposes them.
    """
    return ((key >> 32) & 0xFFFF_FFFF, key & 0xFFFF_FFFF, 1)


# The exact psql query used for assertion 2 (single leader). Reported verbatim.
LOCK_QUERY = lock_query(FLEET_LOCK_KEY)

SERVER_SELECTOR = "app.kubernetes.io/name=cloacina-server"


# ---------------------------------------------------------------------------
# REST helper (urllib, this lane's bootstrap key)
# ---------------------------------------------------------------------------

def _api(method, path, body=None, expect=(200, 201), base=None):
    url = f"{base}{path}"
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Authorization", f"Bearer {BOOTSTRAP_KEY}")
    if data is not None:
        req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            raw = resp.read().decode()
            code = resp.status
    except urllib.error.HTTPError as exc:
        raw = exc.read().decode()
        code = exc.code
    if expect is not None and code not in expect:
        raise AssertionError(f"{method} {path} -> {code} (expected {expect}); body: {raw[:400]}")
    try:
        return code, json.loads(raw) if raw else None
    except json.JSONDecodeError:
        return code, raw


RX_NAME = "packaged_market_maker_reactor"
RX_EXAMPLE = "examples/features/computation-graphs/packaged-graph"
RX_PKG_DIR = "packaged-graph-example-0.1.0"  # must match package.toml name-version
RX_ACCUMULATOR = "orderbook"


def _pack_rx_fixture(tmpdir):
    """Pack the CG example into a source archive the IN-CLUSTER compiler can build.

    The pre-packed `examples/fixtures/dist/*.cloacina` archives are HOST-flavored:
    `pack-demo-fixtures.sh` rewrites `__WORKSPACE__` to the packing machine's
    checkout path, so their Cargo.tomls carry absolute host paths (e.g.
    `/Users/<user>/...`) that resolve only for a compiler running ON that host —
    the demos compose stack. Inside the cluster they fail `cargo fetch` with
    "No such file or directory". This bit as a 13ms build failure that took a
    live cluster and the `build_error` column to diagnose.

    The features example instead ships crates.io VERSION deps (the form real
    users ship), which the in-cluster compiler resolves against its baked
    `/workspace` source via CLOACINA_COMPILER_DEV_WORKSPACE — see
    `_deploy_compiler`. So: pack the example verbatim, no rewriting at all.
    """
    import tarfile
    src = Path(angreal.get_root()).parent / RX_EXAMPLE
    out = Path(tmpdir) / "packaged-graph.cloacina"
    with tarfile.open(out, "w:bz2") as tf:
        for rel in ("package.toml", "Cargo.toml", "build.rs", "src/lib.rs"):
            tf.add(src / rel, arcname=f"{RX_PKG_DIR}/{rel}")
    return out


def _curl_pod_exec(kubeconfig, args):
    """Run curl INSIDE the cluster (one-shot probe pod).

    Needed because the owner addresses published under reactorAffinity are
    headless-service DNS names — resolvable only by cluster DNS. Following a
    redirect from the host through the port-forward would fail on DNS, and
    that failure would be indistinguishable from the redirect being wrong.
    """
    r = _run(["kubectl", "run", "curl-probe-851", "--rm", "-i", "--restart=Never",
              "--image=curlimages/curl:8.7.1", "-n", NS, "--command", "--",
              "curl", "-s", *args],
             env=_kube_env(kubeconfig), check=False, capture=True)
    return (r.stdout or "").strip()


def _assert_reactor_ownership(kubeconfig, target, base, results, tag, skip_build):
    """CLOACI-T-0851 / A-0012: single reactor owner, published address, hot-path
    edge routing, and failover with address republication. Best-effort like
    assertion 4 — each unavailable precondition BLOCKS with a precise reason."""
    key = "6. reactor ownership + edge routing"
    compiler_ref = _prepare_compiler_image(tag, skip_build)
    if compiler_ref is None:
        results[key] = f"BLOCKED: no compiler image present; {CLAIMING_BLOCKED_REASON}"
        print("  BLOCKED [6]: no compiler image to build the CG package")
        return
    _deploy_compiler(kubeconfig, compiler_ref)  # kubectl apply — idempotent
    import tempfile
    tmpdir = tempfile.mkdtemp(prefix="rx-fixture-")
    try:
        fixture_path = _pack_rx_fixture(tmpdir)
    except Exception as exc:
        results[key] = f"BLOCKED: packing {RX_EXAMPLE} failed: {exc}"
        return
    try:
        code, body = _upload_package(base, str(fixture_path))
        print(f"  upload {RX_EXAMPLE} (fresh pack) -> {code}")
    except Exception as exc:
        # 409 = already uploaded by a previous best-effort assertion: fine.
        if "409" not in str(exc):
            results[key] = f"BLOCKED: package upload failed: {exc}"
            return
    # 900s, matching the demos harness bound: a COLD in-cluster build of a CG
    # package takes ~5-10 min (the demos print exactly that warning). The
    # original 360s bound expired mid-build and reported "never built" — the
    # same words a real failure produces, which is how a too-short timeout
    # masquerades as a compiler bug. On timeout we now also report the LAST
    # OBSERVED status so "still building" and "failed" are distinguishable.
    print("  waiting for build_status=success (bounded 15m; cold builds ~5-10m)...")
    deadline = time.time() + 900
    built = False
    last_status = "unknown"
    while time.time() < deadline:
        _, wf = _api("GET", f"/v1/tenants/{TENANT}/workflows", expect=None, base=base)
        items = wf.get("items", []) if isinstance(wf, dict) else []
        statuses = [w.get("build_status") for w in items]
        if statuses:
            last_status = ",".join(str(s) for s in statuses)
        if any(s == "success" for s in statuses):
            built = True
            break
        if any(s == "failed" for s in statuses):
            break
        time.sleep(10)
    if not built:
        results[key] = (f"BLOCKED: CG package build_status={last_status} "
                        f"after wait; {CLAIMING_BLOCKED_REASON}")
        print(f"  BLOCKED [6]: build not successful (last status: {last_status})")
        return

    # 6a: exactly one replica claims the reactor. Ownership locks are held
    # CONTINUOUSLY (unlike the per-tick fleet lock), so the holder can be read
    # directly — but the server-side catcher still bounds the wait for the
    # reconciler to load the package and claim.
    rx_key = reactor_lock_key(TENANT, RX_NAME)
    print(f"  waiting for the reactor lock (key={rx_key}) to be claimed...")
    holder = _catch_lock_holder(kubeconfig, target, key=rx_key, timeout_s=120)
    if holder is None:
        results[key] = "BLOCKED: reactor lock never claimed (package loaded but no reactor?)"
        print("  BLOCKED [6]: no reactor-lock holder observed within 120s")
        return
    owner_pod, owner_addr_ip, _pid = holder
    holders = _lock_holders(kubeconfig, target, query=lock_query(rx_key))
    assert len(holders) <= 1, f"TWO simultaneous reactor owners: {holders}"
    print(f"  6a OK: single reactor owner {owner_pod}")

    # 6b: the owner PUBLISHED a routable address naming itself. Rows live in
    # the public schema (the server's admin DAL); tenant is a column.
    published = _psql(kubeconfig, target,
                      f"SELECT address FROM reactor_owner_addresses "
                      f"WHERE reactor_name='{RX_NAME}' AND tenant_id='{TENANT}';")
    assert published, "owner claimed the lock but published no address row"
    assert owner_pod in published, (
        f"published address {published!r} does not name the lock holder {owner_pod}")
    print(f"  6b OK: owner address published: {published}")

    # 6c: THE HOT-PATH ASSERTION (the point of A-0012 Amendment 3). Inject via
    # the non-owner: expect a 307 whose Location is the owner's address; follow
    # it in-cluster; then assert the outbox saw ZERO reactor_event rows —
    # proving the redirect, not the durable fallback, carried the event.
    pods = _server_pods_running(kubeconfig)
    non_owner = next((p for p in pods if p != owner_pod), None)
    assert non_owner, f"no second replica found among {pods}"
    non_owner_ip = {v: k for k, v in _server_pod_ips(kubeconfig).items()}.get(non_owner)
    assert non_owner_ip, f"no IP for non-owner pod {non_owner}"
    inject_path = f"/v1/health/accumulators/{RX_ACCUMULATOR}/inject"
    payload = '{"event": {"price": 101.5, "qty": 3}}'
    out = _curl_pod_exec(kubeconfig, [
        "-o", "/dev/null", "-w", "%{http_code} %{redirect_url}",
        "-X", "POST", "-H", f"Authorization: Bearer {BOOTSTRAP_KEY}",
        "-H", "Content-Type: application/json", "-d", payload,
        f"http://{non_owner_ip}:8080{inject_path}"])
    print(f"  non-owner inject -> {out}")
    assert out.startswith("307"), f"expected 307 from the non-owner, got: {out}"
    redirect_url = out.split(" ", 1)[1].strip()
    assert owner_pod in redirect_url, (
        f"redirect {redirect_url!r} does not point at the owner {owner_pod}")
    followed = _curl_pod_exec(kubeconfig, [
        "-o", "/dev/null", "-w", "%{http_code}",
        "-X", "POST", "-H", f"Authorization: Bearer {BOOTSTRAP_KEY}",
        "-H", "Content-Type: application/json", "-d", payload,
        redirect_url])
    assert followed.strip() == "200", f"owner did not accept the redirected inject: {followed}"
    outbox = _psql(kubeconfig, target,
                   "SELECT count(*) FROM delivery_outbox WHERE kind='reactor_event';")
    assert outbox.strip() == "0", (
        f"steady-state inject wrote {outbox} outbox rows — the hot path has "
        f"silently regressed to the durable fallback")
    print("  6c OK: 307 to the owner, owner accepted, ZERO outbox rows (hot path held)")

    # 6d: failover — kill the owner; the survivor must claim AND republish.
    print(f"  killing reactor owner {owner_pod}...")
    _kubectl(["delete", "pod", owner_pod, "-n", NS, "--wait=false"], kubeconfig)
    survivor = _catch_lock_holder(kubeconfig, target, key=rx_key,
                                  exclude_pod=owner_pod, timeout_s=90)
    assert survivor is not None, (
        "no surviving replica claimed the reactor within 90s of the owner dying")
    new_pod, _, _ = survivor
    # Republication is async wrt the claim; poll briefly.
    new_published = ""
    for _ in range(30):
        new_published = _psql(kubeconfig, target,
                              f"SELECT address FROM reactor_owner_addresses "
                              f"WHERE reactor_name='{RX_NAME}' AND tenant_id='{TENANT}';")
        if new_pod in new_published:
            break
        time.sleep(2)
    assert new_pod in new_published, (
        f"survivor {new_pod} claimed the lock but the address row still reads "
        f"{new_published!r} — takeover did not republish")
    print(f"  6d OK: survivor {new_pod} claimed and republished ({new_published})")

    results[key] = "PASS"
    print("  PASS [6]: single owner, published address, hot-path redirect, failover republish")


# ---------------------------------------------------------------------------
# helm deploy at replicaCount=2 (real chart RBAC, fleet.actuator=kubernetes)
# ---------------------------------------------------------------------------

def _leader_values(tag, agent_ref, *, interval_s=1):
    """Reuse the shared server values, then force replicaCount=2 + a fast control
    tick so the advisory lock is observable and failover is quick."""
    base = _server_values(tag, agent_ref, BOOTSTRAP_SECRET)
    # Override the autoscale interval (shared default is 5s) for a faster tick.
    base = base.replace(
        '    - {name: CLOACINA_AUTOSCALE_INTERVAL_S, value: "5"}\n',
        f'    - {{name: CLOACINA_AUTOSCALE_INTERVAL_S, value: "{interval_s}"}}\n',
    )
    # CLOACI-T-0851: per-pod DNS identity + advertised address, so reactor
    # ownership publishes routable owner addresses (assertion 6).
    return f"replicaCount: {REPLICAS}\nreactorAffinity:\n  enabled: true\n" + base


def _helm_deploy(kubeconfig, hostdir, tag, agent_ref):
    _kubectl(["create", "namespace", NS], kubeconfig)
    _kubectl(["create", "secret", "generic", BOOTSTRAP_SECRET, "-n", NS,
              f"--from-literal=bootstrap-key={BOOTSTRAP_KEY}"], kubeconfig)
    values = hostdir / "values.yaml"
    values.write_text(_leader_values(tag, agent_ref))
    try:
        _run(["helm", "install", RELEASE, str(CHART_DIR), "-n", NS,
              "-f", str(values), "--wait", "--timeout=8m"], env=_kube_env(kubeconfig))
    except subprocess.CalledProcessError:
        _dump_diag(kubeconfig, "helm install failed")
        raise
    _kubectl(["rollout", "status", f"deploy/{RELEASE}-cloacina-server", "-n", NS,
              "--timeout=5m"], kubeconfig)


def _dump_diag(kubeconfig, label):
    print(f"\n===== DIAGNOSTICS: {label} =====", flush=True)
    _kubectl(["get", "pods", "-A", "-o", "wide"], kubeconfig, check=False)
    print("----- server logs (tail 200) -----", flush=True)
    print(_server_logs_leader(kubeconfig, tail=200), flush=True)
    print(f"===== END DIAGNOSTICS: {label} =====\n", flush=True)


def _server_logs_leader(kubeconfig, tail=300):
    # --all-containers + the leader release/ns (the shared _server_logs targets
    # the e2e release/ns). --prefix tags each line with its pod so we can see
    # leader vs follower behaviour across both replicas.
    proc = _run(["kubectl", "logs", f"deploy/{RELEASE}-cloacina-server", "-n", NS,
                 f"--tail={tail}", "--prefix=true", "--all-containers=true"],
                env=_kube_env(kubeconfig), check=False, capture=True)
    return (proc.stdout or "") + (proc.stderr or "")


# ---------------------------------------------------------------------------
# postgres / advisory-lock probes
# ---------------------------------------------------------------------------

def _postgres_target(kubeconfig):
    """Return a `kubectl exec` target for the chart's bundled postgres.

    Prefers the conventional `deploy/<release>-postgresql`; falls back to the
    first pod matching the subchart label if the name differs."""
    probe = _run(["kubectl", "get", f"deploy/{RELEASE}-postgresql", "-n", NS],
                 env=_kube_env(kubeconfig), check=False, capture=True)
    if probe.returncode == 0:
        return f"deploy/{RELEASE}-postgresql"
    pod = _run(["kubectl", "get", "pods", "-n", NS, "-l",
                "app.kubernetes.io/name=postgresql",
                "-o", "jsonpath={.items[0].metadata.name}"],
               env=_kube_env(kubeconfig), check=False, capture=True).stdout.strip()
    if not pod:
        raise AssertionError("could not locate the chart's postgresql pod "
                             "(tried deploy/<release>-postgresql + label "
                             "app.kubernetes.io/name=postgresql)")
    return pod


def _psql(kubeconfig, target, sql, capture=True):
    r = _run(["kubectl", "exec", target, "-n", NS, "--",
              "env", "PGPASSWORD=cloacina", "psql", "-U", "cloacina", "-d", "cloacina",
              "-tAc", sql], env=_kube_env(kubeconfig), check=False, capture=capture)
    return (r.stdout or "").strip()


def _lock_holders(kubeconfig, target, query=None):
    """Return list of (client_addr, pid) currently granted an advisory lock.

    Defaults to the fleet control lock. Pass `query=lock_query(k)` to assert on
    a different key — e.g. a per-reactor ownership lock (CLOACI-T-0851).
    """
    out = _psql(kubeconfig, target, query or LOCK_QUERY)
    rows = []
    for line in out.splitlines():
        line = line.strip()
        if not line:
            continue
        parts = line.split("|")
        rows.append((parts[0], parts[1] if len(parts) > 1 else "?"))
    return rows


def _server_pod_ips(kubeconfig):
    """Map server pod IP -> pod name (for client_addr -> pod resolution)."""
    data = _kubectl_json(["get", "pods", "-n", NS, "-l", SERVER_SELECTOR], kubeconfig)
    ip_to_pod = {}
    if data:
        for p in data.get("items", []):
            ip = p.get("status", {}).get("podIP")
            name = p.get("metadata", {}).get("name")
            if ip:
                ip_to_pod[ip] = name
    return ip_to_pod


def _server_pods_running(kubeconfig):
    data = _kubectl_json(["get", "pods", "-n", NS, "-l", SERVER_SELECTOR], kubeconfig)
    running = []
    if data:
        for p in data.get("items", []):
            phase = p.get("status", {}).get("phase")
            ready = all(c.get("ready") for c in p.get("status", {}).get("containerStatuses", []) or [])
            if phase == "Running" and ready:
                running.append(p["metadata"]["name"])
    return running


# ---------------------------------------------------------------------------
# assertion 2 helper: sample the lock, with churn to extend lock-hold windows
# ---------------------------------------------------------------------------

def _churn(base, stop_evt, n):
    """Flip the tenant's desired_count to force the (leader) reconcile to actuate
    each tick — this lengthens each lock-hold window, raising sampling catch
    probability. Best-effort; ignores transient errors."""
    high = True
    while not stop_evt.is_set():
        try:
            target = n if high else max(0, n - 1)
            _, fleet = _api("GET", f"/v1/tenants/{TENANT}/fleet", expect=None, base=base)
            desired = fleet.get("desired_count") if isinstance(fleet, dict) else None
            if desired is not None:
                if desired < target:
                    _api("POST", f"/v1/tenants/{TENANT}/fleet/provision", expect=None, base=base)
                elif desired > target:
                    _api("POST", f"/v1/tenants/{TENANT}/fleet/deprovision", expect=None, base=base)
            high = not high
        except Exception:
            pass
        stop_evt.wait(1.0)


def _sample_lock(kubeconfig, target, window_s, base=None, churn_n=0):
    """High-frequency sample of the advisory-lock holders over `window_s`.

    Returns (max_simultaneous, observed_holders) where observed_holders is a dict
    pod_name -> sample_count for every distinct holder seen."""
    ip_to_pod = _server_pod_ips(kubeconfig)
    max_simul = 0
    observed = {}
    catches = 0
    stop_evt = threading.Event()
    churn_t = None
    if base is not None and churn_n > 0:
        churn_t = threading.Thread(target=_churn, args=(base, stop_evt, churn_n), daemon=True)
        churn_t.start()
    deadline = time.time() + window_s
    try:
        while time.time() < deadline:
            holders = _lock_holders(kubeconfig, target)
            max_simul = max(max_simul, len(holders))
            if holders:
                catches += 1
                for addr, _pid in holders:
                    pod = ip_to_pod.get(addr, addr)
                    observed[pod] = observed.get(pod, 0) + 1
            time.sleep(0.1)
    finally:
        stop_evt.set()
        if churn_t is not None:
            churn_t.join(timeout=3)
    return max_simul, observed, catches


def _catch_lock_holder_sql(key, *, exclude_addr=None, poll_ms=10, timeout_s=20):
    """SQL that waits INSIDE Postgres for a holder of advisory lock `key`.

    Why this exists: the fleet lock is taken and released within a single
    control-loop tick, so it is held for a brief instant. Polling it from
    Python cost TWO subprocess spawns per sample (`kubectl get pods` +
    `kubectl exec … psql`), hundreds of milliseconds each, so the sample rate
    was latency-bound and the observed hit rate was ~6 catches per window.
    Assertion 5 has to pin the holder at one instant before killing it, and it
    lost that race often enough to block the lane — a CORE assertion failing
    the run for reasons unrelated to anything under test.

    Moving the loop server-side turns one exec into thousands of samples at
    `poll_ms`, returning the instant a holder appears. Same observation, same
    `pg_locks` predicate, ~1000x the sampling density.

    `exclude_addr` skips a specific `client_addr` — used post-kill to wait for a
    holder that is NOT the replica we just deleted, which is otherwise the same
    race in the other direction.

    NOTE: still matches the FULL key (classid + objid + objsubid). See
    `advisory_lock_parts` for why a partial match is a silent failure.
    """
    classid, objid, objsubid = advisory_lock_parts(key)
    iterations = max(1, int((timeout_s * 1000) // poll_ms))
    exclude = ""
    if exclude_addr:
        # client_addr is inet; compare as text so a NULL addr cannot swallow it.
        exclude = f" AND a.client_addr::text <> '{exclude_addr}'"
    return (
        "CREATE TEMP TABLE IF NOT EXISTS _holder_catch(addr text, pid int); "
        "DELETE FROM _holder_catch; "
        "DO $do$ DECLARE v_addr text; v_pid int; i int := 0; BEGIN LOOP "
        # host(): client_addr is `inet`, and casting it to text can carry the
        # netmask ("10.42.0.3/32"), which does NOT match the bare IPs in
        # _server_pod_ips. The lookup then misses and the caller's fallback
        # hands the ADDRESS back as if it were a pod name — observed for real:
        # `kubectl delete pod 10.42.0.3/32`. host() yields the bare address.
        #
        # coalesce: client_addr is NULL for a local (unix-socket) connection, and
        # NULL || '|' || pid is NULL — the holder row would exist but render as
        # nothing, so the catcher would silently report "no holder found".
        "SELECT coalesce(host(a.client_addr), 'local'), a.pid INTO v_addr, v_pid "
        "FROM pg_locks l JOIN pg_stat_activity a ON l.pid = a.pid "
        f"WHERE l.locktype='advisory' AND l.classid={classid} AND l.objid={objid} "
        f"AND l.objsubid={objsubid} AND l.granted{exclude} LIMIT 1; "
        "IF v_pid IS NOT NULL THEN INSERT INTO _holder_catch VALUES (v_addr, v_pid); RETURN; END IF; "
        f"i := i + 1; EXIT WHEN i >= {iterations}; PERFORM pg_sleep({poll_ms / 1000.0}); "
        "END LOOP; END $do$; "
        "SELECT addr || '|' || pid FROM _holder_catch LIMIT 1;"
    )


def _catch_lock_holder(kubeconfig, target, *, key=FLEET_LOCK_KEY, exclude_pod=None,
                       timeout_s=20):
    """Wait (server-side) for a lock holder and resolve it to a pod.

    Returns (pod, addr, pid) or None if none appeared within `timeout_s`.
    `exclude_pod` resolves to that pod's IP and excludes it in SQL.
    """
    ip_to_pod = _server_pod_ips(kubeconfig)
    exclude_addr = None
    if exclude_pod is not None:
        for ip, pod in ip_to_pod.items():
            if pod == exclude_pod:
                exclude_addr = ip
                break
    sql = _catch_lock_holder_sql(key, exclude_addr=exclude_addr, timeout_s=timeout_s)
    out = _psql(kubeconfig, target, sql).strip()
    # psql echoes CREATE TABLE / DELETE / DO before the final SELECT; the holder
    # row is the only line containing our '|' separator.
    row = next((l for l in out.splitlines() if "|" in l), None)
    if not row:
        return None
    addr, _, pid = row.strip().partition("|")
    # Re-resolve: the pod set may have changed while we waited.
    ip_to_pod = _server_pod_ips(kubeconfig)
    pod = ip_to_pod.get(addr)
    if pod is None:
        # Do NOT fall back to returning the address as the pod name. The caller
        # feeds this straight to `kubectl delete pod`, and an unresolved address
        # then becomes a delete against a nonexistent pod that crashes the lane
        # with a confusing CalledProcessError. Observed for real when the address
        # carried a /32 suffix. Report it as "no usable holder" instead, which
        # blocks the assertion honestly and prints what could not be resolved.
        print(f"  WARN: lock holder addr={addr} pid={pid} did not resolve to a "
              f"server pod (known: {sorted(ip_to_pod)}); treating as no holder")
        return None
    return (pod, addr, pid)


def _wait_lock_holder(kubeconfig, target, want_pod=None, not_pod=None, timeout_s=40):
    """Poll until a single lock holder is observed that matches the predicate.

    `want_pod`: holder must equal this pod. `not_pod`: holder must NOT equal this
    pod. Returns (pod, addr, pid) of the first matching catch, or None on timeout.
    Also asserts we never see >1 simultaneous holder while polling."""
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        ip_to_pod = _server_pod_ips(kubeconfig)
        holders = _lock_holders(kubeconfig, target)
        if len(holders) > 1:
            raise AssertionError(f"TWO simultaneous fleet-lock holders observed: {holders}")
        if len(holders) == 1:
            addr, pid = holders[0]
            pod = ip_to_pod.get(addr, addr)
            if want_pod is not None and pod != want_pod:
                pass
            elif not_pod is not None and pod == not_pod:
                pass
            else:
                return pod, addr, pid
        time.sleep(0.1)
    return None


# ---------------------------------------------------------------------------
# assertion 4 (best-effort / opt-in): in-cluster compiler + real executions
# ---------------------------------------------------------------------------

CLAIMING_BLOCKED_REASON = (
    "not validated end-to-end HERE: a helm-only cloacina-server deploy ships NO compiler "
    "(charts/cloacina-server has no compiler template), so uploaded source .cloacina "
    "packages stay build_status='pending' forever and never execute. This is a "
    "harness limitation, NOT a coverage gap: the disjoint-claiming property is proven "
    "deterministically by the DAL integration test "
    "dal::task_claiming::test_concurrent_task_claiming_no_duplicates "
    "(crates/cloacina/tests/integration/dal/task_claiming.rs) — N concurrent claimers on "
    "separate pooled connections, asserting each ready task is claimed by exactly one "
    "(disjoint) and all are claimed (complete), zero slack. Run it with "
    "`angreal test integration --backend postgres -- task_claiming`. The property is "
    "enforced in cloacina-core by the task_outbox claim 'DELETE ... FOR UPDATE SKIP "
    "LOCKED' + the claimed_by CAS "
    "(crates/cloacina/src/dal/unified/task_execution/claiming.rs) and BOTH replicas "
    "run the per-tenant scheduler unconditionally (services.rs: 'Always: per-runner "
    "task scheduler'; lib.rs:689 global runner per replica) — it is NOT leader-gated. "
    "To ALSO validate behaviourally full-stack: deploy a matching-ABI compiler Deployment "
    "against the same Postgres, upload a package whose path-deps resolve in-container, await "
    "build_status='success', drive M executions, then assert no (workflow_execution_id, "
    "task_name) has >1 task_executions row. Pass --claiming to attempt it."
)


def _prepare_compiler_image(tag, skip_build):
    """Tag+push a locally-present demo compiler image to the registry.

    Returns the in-cluster ref, or None if no compiler image is available (we do
    NOT trigger the ~2GB Dockerfile.compiler build implicitly)."""
    compiler_host = f"{REGISTRY_HOST}/cloacina-compiler:{tag}"
    if _image_exists(compiler_host):
        _run(["docker", "push", compiler_host])
        return f"{REGISTRY_K8S}/cloacina-compiler:{tag}"
    for src in ("cloacina-demo-fleet-compiler:latest", "docker-compiler:latest"):
        if _image_exists(src):
            _run(["docker", "tag", src, compiler_host])
            _run(["docker", "push", compiler_host])
            return f"{REGISTRY_K8S}/cloacina-compiler:{tag}"
    return None


def _deploy_compiler(kubeconfig, compiler_ref):
    db_url = f"postgres://cloacina:cloacina@{RELEASE}-postgresql:5432/cloacina"
    manifest = {
        "apiVersion": "apps/v1", "kind": "Deployment",
        "metadata": {"name": "cloacina-compiler", "namespace": NS},
        "spec": {
            "replicas": 1,
            "selector": {"matchLabels": {"app": "cloacina-compiler"}},
            "template": {
                "metadata": {"labels": {"app": "cloacina-compiler"}},
                "spec": {"containers": [{
                    "name": "compiler",
                    "image": compiler_ref,
                    "imagePullPolicy": "IfNotPresent",
                    "args": ["--bind", "0.0.0.0:9000", "--database-url", db_url,
                             "--poll-interval-ms", "1000",
                             "--cargo-target-dir", "/workspace/target",
                             "--cargo-flags-replace=build", "--cargo-flags-replace=--lib"],
                    "env": [{"name": "CARGO_PROFILE_DEV_DEBUG", "value": "0"},
                            # CLOACI-T-0779: compilers are TENANT-SCOPED by
                            # design — one per tenant, like the agent fleet.
                            # Unscoped, this compiler polls only the `public`
                            # schema while the lane uploads to TENANT, so every
                            # build sits `pending` forever and the compiler
                            # logs nothing. That silent mismatch blocked
                            # assertions 4 AND 6 across four runs before being
                            # diagnosed from the live DB, so: never omit this.
                            {"name": "CLOACINA_TENANT_SCHEMA", "value": TENANT},
                            # CLOACI-T-0887 dev escape hatch: fixtures ship
                            # crates.io version deps (the form users ship);
                            # resolve them against the workspace source baked
                            # into the compiler image at /workspace.
                            {"name": "CLOACINA_COMPILER_DEV_WORKSPACE",
                             "value": "/workspace"}],
                }]},
            },
        },
    }
    proc = subprocess.run(["kubectl", "apply", "-n", NS, "-f", "-"],
                          input=json.dumps(manifest), env=_kube_env(kubeconfig),
                          text=True, capture_output=True)
    print(proc.stdout + proc.stderr)
    if proc.returncode != 0:
        raise AssertionError("failed to apply compiler Deployment")


def _upload_package(base, fixture_path):
    boundary = "----CloacinaLeaderE2E"
    body = (f"--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; "
            f"filename=\"package.cloacina\"\r\nContent-Type: application/octet-stream\r\n\r\n").encode()
    body += Path(fixture_path).read_bytes()
    body += f"\r\n--{boundary}--\r\n".encode()
    req = urllib.request.Request(f"{base}/v1/tenants/{TENANT}/workflows", data=body, method="POST")
    req.add_header("Authorization", f"Bearer {BOOTSTRAP_KEY}")
    req.add_header("Content-Type", f"multipart/form-data; boundary={boundary}")
    with urllib.request.urlopen(req, timeout=30) as resp:
        return resp.status, json.loads(resp.read())


def _task_exec_schemas(kubeconfig, target):
    out = _psql(kubeconfig, target,
                "SELECT table_schema FROM information_schema.tables "
                "WHERE table_name='task_executions';")
    return [s.strip() for s in out.splitlines() if s.strip()]


def _assert_disjoint_claiming(kubeconfig, target, base, results, tag, skip_build,
                              workflow="data_processing",
                              fixture="examples/fixtures/dist/simple-packaged.cloacina",
                              m_execs=8):
    """Best-effort behavioural proof of disjoint claiming. Reports BLOCKED with a
    precise reason if the compiler path is unavailable / does not converge."""
    key = "4. disjoint claiming (scheduler scales)"
    compiler_ref = _prepare_compiler_image(tag, skip_build)
    if compiler_ref is None:
        results[key] = f"BLOCKED: no compiler image present; {CLAIMING_BLOCKED_REASON}"
        print("  BLOCKED [4]: no compiler image present to drive executions")
        return
    print(f"  deploying in-cluster compiler {compiler_ref} (best-effort)")
    _deploy_compiler(kubeconfig, compiler_ref)
    fixture_path = Path(angreal.get_root()).parent / fixture
    if not fixture_path.exists():
        results[key] = f"BLOCKED: fixture {fixture} missing"
        return
    try:
        code, body = _upload_package(base, str(fixture_path))
        print(f"  upload {fixture} -> {code} ({body})")
    except Exception as exc:
        results[key] = f"BLOCKED: package upload failed: {exc}"
        return
    # Await build_status=success (bounded). With cold ABI/path constraints this
    # frequently never converges in a helm-only deploy — hence best-effort.
    print("  waiting for build_status=success (bounded 6m)...")
    deadline = time.time() + 360
    built = False
    while time.time() < deadline:
        _, wf = _api("GET", f"/v1/tenants/{TENANT}/workflows", expect=None, base=base)
        items = wf.get("items", []) if isinstance(wf, dict) else []
        if any(w.get("build_status") == "success" for w in items):
            built = True
            break
        time.sleep(10)
    if not built:
        results[key] = (f"BLOCKED: package never reached build_status=success within 6m; "
                        f"{CLAIMING_BLOCKED_REASON}")
        print("  BLOCKED [4]: package never built (no/incompatible compiler)")
        return
    # Drive M executions; both replicas' schedulers race for the ready tasks.
    print(f"  driving {m_execs} executions of '{workflow}'")
    for i in range(m_execs):
        _api("POST", f"/v1/tenants/{TENANT}/workflows/{workflow}/execute",
             {"context": {"i": i}}, expect=(200, 201, 202), base=base)
    time.sleep(30)  # let them complete
    # Duplicate-dispatch check: any (workflow_execution_id, task_name) with >1 row.
    dup_found = None
    for schema in _task_exec_schemas(kubeconfig, target):
        dup = _psql(kubeconfig, target,
                    f"SELECT count(*) FROM (SELECT workflow_execution_id, task_name "
                    f"FROM {schema}.task_executions GROUP BY 1,2 HAVING count(*)>1) d;")
        total = _psql(kubeconfig, target, f"SELECT count(*) FROM {schema}.task_executions;")
        print(f"  schema {schema}: task_executions={total} duplicate_groups={dup}")
        if dup and dup.isdigit() and int(dup) > 0:
            dup_found = f"{schema}:{dup} duplicate (workflow_execution_id,task_name) groups"
    if dup_found:
        results[key] = f"FAIL: double-dispatch detected — {dup_found}"
        print(f"  FAIL [4]: {dup_found}")
    else:
        results[key] = "PASS"
        print("  PASS [4]: each task ran exactly once across both replicas' schedulers")


# ---------------------------------------------------------------------------
# the command
# ---------------------------------------------------------------------------

@test()
@e2e()
@angreal.command(
    name="k8s-leader",
    about="2-replica leadership e2e: single fleet leader + scaling API/scheduler (CLOACI-T-0818)",
    when_to_use=[
        "validating ADR CLOACI-A-0008's in-process advisory-lock leader at replicaCount=2",
        "proving single-writer fleet provisioning + leader failover on a real cluster",
    ],
    when_not_to_use=["unit testing", "running without docker/kubectl/helm",
                     "the 1-replica fleet-actuator correctness check (use `e2e k8s-fleet`)"],
)
@angreal.argument(name="no_cleanup", long="no-cleanup", takes_value=False, is_flag=True,
                  help="Leave the k3s cluster + helm release up for inspection")
@angreal.argument(name="skip_build", long="skip-build", takes_value=False, is_flag=True,
                  help="Reuse already-built server/agent images (no rebuild)")
@angreal.argument(name="claiming", long="claiming", takes_value=False, is_flag=True,
                  help="Also attempt assertion 4 (disjoint claiming) via an in-cluster compiler")
@angreal.argument(name="agents", long="agents", default_value="3",
                  help="Number of agents to provision for the single-writer test (default 3)")
@angreal.argument(name="tag", long="tag", default_value="k8s-soak",
                  help="Image tag in the local registry (default k8s-soak — the retained images)")
def k8s_leader(no_cleanup=False, skip_build=False, claiming=False, agents="3", tag="k8s-soak"):
    _check_tool("docker", "install Docker Desktop or colima")
    _check_tool("kubectl", "sudo port install kubectl")
    _check_tool("helm", "sudo port install kubernetes-helm")

    n_agents = int(agents)
    project = f"cloacina-k8s-leader-{uuid.uuid4().hex[:8]}"
    hostdir = Path(tempfile.mkdtemp(prefix="cloacina-k8s-leader-"))
    kubeconfig = hostdir / "kubeconfig.host.yaml"

    print_section_header(f"cloacina multi-replica leadership e2e (project: {project})")
    print(f"host kubeconfig dir: {hostdir}")
    print(f"replicaCount={REPLICAS}  port-forward={FWD_PORT}  fleet-lock-key={FLEET_LOCK_KEY}")

    results = {}
    fwd = None
    compose_env = os.environ.copy()
    compose_env["CLOACINA_K8S_E2E_HOSTDIR"] = str(hostdir)

    try:
        print("\n--- 1. bring up k3s + registry (docker compose) ---\n")
        bring_up_cluster(project, hostdir, compose_env, kubeconfig)

        print("\n--- 2. build/push images ---\n")
        server_ref, agent_ref = _prepare_images(tag, skip_build)

        print(f"\n--- 3. helm install cloacina-server (replicaCount={REPLICAS}, "
              f"fleet.actuator=kubernetes) ---\n")
        _helm_deploy(kubeconfig, hostdir, tag, agent_ref)

        print("\n--- 4. port-forward the server Service ---\n")
        fwd, base = start_port_forward(kubeconfig, release=RELEASE, ns=NS, port=FWD_PORT)
        if not _wait_http(f"{base}/ready", timeout_s=90, proc=fwd):
            print(_server_logs_leader(kubeconfig))
            raise AssertionError("server /ready never became healthy via the Service port-forward")

        postgres = _postgres_target(kubeconfig)
        print(f"  postgres exec target: {postgres}")

        # ===== ASSERTION 1: both replicas Ready ==============================
        print("\n--- ASSERTION 1: both replicas Ready ---\n")
        deploy = _kubectl_json(["get", "deploy", f"{RELEASE}-cloacina-server", "-n", NS], kubeconfig)
        ready = deploy.get("status", {}).get("readyReplicas") if deploy else None
        running = _server_pods_running(kubeconfig)
        if ready == REPLICAS and len(running) == REPLICAS:
            results["1. both replicas Ready (2/2) + /ready healthy"] = "PASS"
            print(f"  PASS [1]: {ready}/{REPLICAS} readyReplicas, {len(running)} pods Running "
                  f"({running}), /ready healthy through the Service")
        else:
            _dump_diag(kubeconfig, "replicas not both ready")
            raise AssertionError(f"expected {REPLICAS} ready/running server pods; "
                                 f"readyReplicas={ready} running={running}")

        # ===== ASSERTION 3a: provision N (single-writer) =====================
        # Provision BEFORE the lock sampling so the reconcile loop has real work
        # (which lengthens lock-hold windows for assertion 2).
        print("\n--- ASSERTION 3a: create tenant + provision N agents (REST) ---\n")
        code, _ = _api("POST", "/v1/tenants", {"name": TENANT}, expect=(200, 201, 409), base=base)
        print(f"  create tenant '{TENANT}' -> {code}")
        _api("POST", f"/v1/tenants/{TENANT}/limits", {"max_agents": n_agents + 5}, base=base)
        for i in range(n_agents):
            _api("POST", f"/v1/tenants/{TENANT}/fleet/provision", expect=(200, 201), base=base)
        _, fleet = _api("GET", f"/v1/tenants/{TENANT}/fleet", base=base)
        if fleet.get("desired_count") != n_agents:
            raise AssertionError(f"desired_count={fleet.get('desired_count')} != {n_agents}")
        # Reconcile (leader-only) must drive the tenant Deployment to EXACTLY N.
        deadline = time.time() + 120
        replicas = None
        deploys = None
        while time.time() < deadline:
            d = _kubectl_json(["get", "deploy", "cloacina-agent", "-n", TENANT_NS], kubeconfig)
            replicas = d.get("spec", {}).get("replicas") if d else None
            deploys = _kubectl_json(["get", "deploy", "-n", TENANT_NS], kubeconfig)
            if replicas == n_agents:
                break
            time.sleep(3)
        n_deploys = len(deploys.get("items", [])) if deploys else 0
        if replicas == n_agents and n_deploys == 1:
            print(f"  provision N: desired_count={n_agents}, agent Deployment replicas={replicas}, "
                  f"deployments-in-ns={n_deploys} (exactly N, not {REPLICAS}xN — single writer)")
        else:
            _dump_diag(kubeconfig, "single-writer provision mismatch")
            raise AssertionError(f"single-writer provision: replicas={replicas} (want {n_agents}), "
                                 f"deployments={n_deploys} (want 1)")

        # ===== ASSERTION 2: single leader (advisory lock) ====================
        print("\n--- ASSERTION 2: single fleet-lock holder (sampled) ---\n")
        print(f"  psql lock query: {LOCK_QUERY}")
        max_simul, observed, catches = _sample_lock(
            kubeconfig, postgres, window_s=60, base=base, churn_n=n_agents)
        print(f"  samples with a holder: {catches}; max simultaneous holders: {max_simul}; "
              f"holders observed (pod -> samples): {observed}")
        if max_simul > 1:
            raise AssertionError(f"TWO replicas held the fleet lock simultaneously ({observed}) — "
                                 f"single-writer leadership VIOLATED")
        if catches == 0:
            results["2. single fleet-lock holder"] = (
                "BLOCKED: never caught the lock held (tick window too small to sample); "
                "max simultaneous holders stayed 0")
            print("  BLOCKED [2]: could not catch the transient per-tick lock during sampling")
        else:
            leader_pods = sorted(observed.keys())
            results["2. single fleet-lock holder"] = "PASS"
            print(f"  PASS [2]: fleet lock held by at most ONE connection at a time over {catches} "
                  f"catches; holder pod(s) seen: {leader_pods} "
                  f"(per-tick election: may differ tick-to-tick, never simultaneous)")

        # ===== ASSERTION 3b: deprovision scales down =========================
        print("\n--- ASSERTION 3b: deprovision -> scale down ---\n")
        # churn during sampling may have moved desired_count; drive it explicitly to 0.
        for _ in range(n_agents + 2):
            _, fl = _api("GET", f"/v1/tenants/{TENANT}/fleet", expect=None, base=base)
            if (fl.get("desired_count") or 0) <= 0:
                break
            _api("POST", f"/v1/tenants/{TENANT}/fleet/deprovision", expect=None, base=base)
        deadline = time.time() + 90
        replicas = None
        while time.time() < deadline:
            d = _kubectl_json(["get", "deploy", "cloacina-agent", "-n", TENANT_NS], kubeconfig)
            replicas = d.get("spec", {}).get("replicas") if d else None
            if replicas == 0:
                break
            time.sleep(3)
        _, fleet = _api("GET", f"/v1/tenants/{TENANT}/fleet", base=base)
        if (fleet.get("desired_count") or 0) == 0 and replicas == 0:
            results["3. single-writer provisioning (N, then scale down)"] = "PASS"
            print("  PASS [3]: provisioned to exactly N then deprovisioned to 0 — single writer")
        else:
            results["3. single-writer provisioning (N, then scale down)"] = (
                f"BLOCKED: desired_count={fleet.get('desired_count')} replicas={replicas}")
            print(f"  BLOCKED [3]: desired_count={fleet.get('desired_count')} replicas={replicas}")

        # ===== ASSERTION 4: disjoint claiming (best-effort) ==================
        print("\n--- ASSERTION 4: disjoint claiming ---\n")
        if claiming:
            _assert_disjoint_claiming(kubeconfig, postgres, base, results, tag, skip_build)
        else:
            results["4. disjoint claiming (scheduler scales)"] = (
                f"BLOCKED: {CLAIMING_BLOCKED_REASON}")
            print("  BLOCKED [4]: not attempted (--claiming off); see report for the architectural reason")

        # ===== ASSERTION 5: failover =========================================
        print("\n--- ASSERTION 5: leader failover ---\n")
        # Re-provision so the reconcile loop has work (and so we can prove
        # provisioning still works AFTER the failover).
        _api("POST", f"/v1/tenants/{TENANT}/limits", {"max_agents": n_agents + 5}, base=base)
        for _ in range(2):
            _api("POST", f"/v1/tenants/{TENANT}/fleet/provision", expect=None, base=base)
        # Identify the current lock holder (the leader for the tick we catch).
        # Server-side wait: the lock lives for an instant per tick, so polling
        # from here lost the race often enough to block this assertion. See
        # _catch_lock_holder_sql.
        holder = _catch_lock_holder(kubeconfig, postgres, timeout_s=40)
        if holder is None:
            results["5. leader failover"] = (
                "BLOCKED: could not catch a lock holder to target for the kill")
            print("  BLOCKED [5]: never caught the lock holder pre-kill")
        else:
            old_pod, old_addr, old_pid = holder
            print(f"  current lock holder: pod={old_pod} addr={old_addr} pid={old_pid} — deleting it")
            _kubectl(["delete", "pod", old_pod, "-n", NS, "--wait=false"], kubeconfig)
            # The surviving replica must acquire the lock (different pod than killed).
            # Same instant-lifetime race as the pre-kill catch, but this one
            # HARD-FAILS the lane rather than blocking, so it needs the
            # server-side wait at least as much.
            survivor = _catch_lock_holder(
                kubeconfig, postgres, exclude_pod=old_pod, timeout_s=60)
            if survivor is None:
                _dump_diag(kubeconfig, "no survivor acquired the lock")
                raise AssertionError("after killing the leader, NO surviving replica acquired the "
                                     "fleet lock within 60s — failover FAILED")
            new_pod, new_addr, new_pid = survivor
            print(f"  failover: lock re-acquired by pod={new_pod} addr={new_addr} pid={new_pid} "
                  f"(was pod={old_pod} pid={old_pid})")
            # `kubectl port-forward svc/...` pins to ONE pod; if it was pinned to
            # the killed leader it is now dead, so the next API call would hit a
            # closed connection (RemoteDisconnected). Re-establish the forward
            # against the Service (the survivor is still a Ready endpoint) before
            # the post-failover provisioning check.
            if fwd is not None and fwd.poll() is None:
                fwd.terminate()
                try:
                    fwd.wait(timeout=5)
                except Exception:
                    fwd.kill()
            fwd, base = start_port_forward(kubeconfig, release=RELEASE, ns=NS, port=FWD_PORT)
            if not _wait_http(f"{base}/ready", timeout_s=60, proc=fwd):
                raise AssertionError("server /ready not healthy via the refreshed port-forward "
                                     "after the leader kill")
            print("  port-forward re-established after the kill; verifying provisioning still works")
            # Provisioning still works under the new leader.
            _, fbefore = _api("GET", f"/v1/tenants/{TENANT}/fleet", base=base)
            d_before = (fbefore.get("desired_count") or 0)
            _api("POST", f"/v1/tenants/{TENANT}/fleet/provision", expect=(200, 201), base=base)
            want = d_before + 1
            deadline = time.time() + 90
            post_replicas = None
            while time.time() < deadline:
                d = _kubectl_json(["get", "deploy", "cloacina-agent", "-n", TENANT_NS], kubeconfig)
                post_replicas = d.get("spec", {}).get("replicas") if d else None
                if post_replicas == want:
                    break
                time.sleep(3)
            # Killed replica reschedules + rejoins as a follower (2/2 Ready again).
            _kubectl(["rollout", "status", f"deploy/{RELEASE}-cloacina-server", "-n", NS,
                      "--timeout=3m"], kubeconfig, check=False)
            rejoined = len(_server_pods_running(kubeconfig))
            post_holders = _lock_holders(kubeconfig, postgres)
            if post_replicas == want and rejoined == REPLICAS and len(post_holders) <= 1:
                results["5. leader failover"] = "PASS"
                print(f"  PASS [5]: survivor {new_pod} leads; provision scaled to {post_replicas}; "
                      f"killed replica rescheduled → {rejoined}/{REPLICAS} Ready; "
                      f"lock holders still <=1 ({len(post_holders)})")
            else:
                results["5. leader failover"] = (
                    f"BLOCKED: post-failover provision replicas={post_replicas} (want {want}), "
                    f"rejoined={rejoined}/{REPLICAS}, lock_holders={len(post_holders)}")
                print(f"  BLOCKED [5]: provision={post_replicas} rejoined={rejoined} "
                      f"holders={len(post_holders)}")

        # ===== ASSERTION 6: reactor ownership + edge routing (CLOACI-T-0851) =
        print("\n--- ASSERTION 6: reactor ownership + edge routing ---\n")
        try:
            _assert_reactor_ownership(kubeconfig, postgres, base, results, tag, skip_build)
        except AssertionError as exc:
            results["6. reactor ownership + edge routing"] = f"FAIL: {exc}"
            print(f"  FAIL [6]: {exc}")

        # --- summary ---------------------------------------------------------
        print("\n" + "=" * 70)
        print("ASSERTION RESULTS")
        print("=" * 70)
        for label, status in results.items():
            print(f"  [{status.split(':')[0]:7}] {label}")
        print("=" * 70)
        blocked = [k for k, v in results.items() if v.startswith("BLOCKED")]
        failed = [k for k, v in results.items() if v.startswith("FAIL")]
        if not blocked and not failed:
            print_final_success("cloacina multi-replica leadership e2e — ALL assertions green")
        else:
            print(f"\n{len(results) - len(blocked) - len(failed)}/{len(results)} green; "
                  f"blocked: {[b.split('.')[0] for b in blocked]}; failed: {[f.split('.')[0] for f in failed]}")

        # Core leadership assertions (1, 2, 3, 5) must be green for a pass. 4 is
        # best-effort (BLOCKED is tolerated; an actual FAIL is not).
        core = ["1. both replicas Ready (2/2) + /ready healthy",
                "2. single fleet-lock holder",
                "3. single-writer provisioning (N, then scale down)",
                "5. leader failover"]
        bad_core = [c for c in core if results.get(c, "").startswith(("BLOCKED", "FAIL")) or c not in results]
        if bad_core or failed:
            print(f"\nFAILED core assertions: {bad_core or failed}")
            sys.exit(1)

    finally:
        if fwd is not None and fwd.poll() is None:
            fwd.terminate()
        if no_cleanup:
            print(f"\n--no-cleanup: cluster left up.\n"
                  f"  KUBECONFIG={kubeconfig}\n"
                  f"  kubectl --kubeconfig {kubeconfig} get pods -A -o wide\n"
                  f"  teardown: docker compose -f {COMPOSE_FILE} -p {project} down -v")
        else:
            print(f"\n--- cleanup: docker compose -p {project} down -v ---\n")
            _compose(["down", "-v"], project, env=compose_env)
            shutil.rmtree(hostdir, ignore_errors=True)
