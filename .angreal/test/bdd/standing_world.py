"""The world of the standing workload (CLOACINA-T-1107).

It is the world of the publishing chapter (`world.PublishWorld`) with the
workload's three tenants, and the few things only the standing workload
does: more than one agent for each tenant, the secret of the self-driving
workflow, the schedules, and the settings file of the checker.

Every method here can be called again on a deployment that is already up:
it makes what is missing and leaves the rest. The workload stands for days;
the driver comes back to it.

Nothing in this file runs against a cluster until a method is called.
"""

from __future__ import annotations

import json
import os
import time
from pathlib import Path

from .suite import workload
from .world import PublishWorld

TERMINAL = ("Completed", "Failed", "Cancelled")


class StandingWorld(PublishWorld):
    def __init__(self, *, settings_path, database_port, **kwargs):
        super().__init__(**kwargs)
        self.tenants = workload.TENANTS
        self.settings_path = Path(settings_path)
        self.database_port = database_port
        # Read keys for the checker, one for each tenant.
        self.checker_keys = {}
        # True once the packages, the secret and the schedules are in place.
        self.workload_ready = False

    # -- tenants and agents --------------------------------------------------

    def provision_workload_tenants(self, compiler_ref, agent_ref=None):
        """The tenants, their keys and compilers (as the publishing chapter
        makes them), then the workload's number of agents for each tenant and
        a read key for the checker."""
        self.provision_tenants(compiler_ref, agent_ref=agent_ref)
        for tenant in self.tenants:
            self.api("POST", f"/v1/tenants/{tenant}/limits", key=self.bootstrap_key,
                     body={"max_agents": workload.MAX_AGENTS_PER_TENANT}, expect=(200, 201))
            self.scale_agents(tenant, workload.AGENTS_PER_TENANT, applied_by_driver=agent_ref is not None)
            _, created = self.api("POST", f"/v1/tenants/{tenant}/keys", key=self.bootstrap_key,
                                  body={"name": f"{tenant}-checker", "role": "read"},
                                  expect=(200, 201))
            self.checker_keys[tenant] = created["key"]
        self.wait_for_agent_count(workload.AGENTS_PER_TENANT)

    def scale_agents(self, tenant, count, *, applied_by_driver):
        """Bring a tenant to `count` agents."""
        if applied_by_driver:
            # The driver applied the agent Deployment itself: scale it.
            self.kubectl("scale", "deploy/cloacina-agent", "-n", self.tenant_namespace(tenant),
                         f"--replicas={count}")
            return
        # The server's actuator owns the agents: one more for each call.
        for _ in range(count + 1):
            _, fleet = self.api("GET", f"/v1/tenants/{tenant}/fleet", key=self.bootstrap_key,
                                expect=(200,))
            if int((fleet or {}).get("desired_count") or 0) >= count:
                return
            self.api("POST", f"/v1/tenants/{tenant}/fleet/provision", key=self.bootstrap_key,
                     expect=(200, 201))
        raise AssertionError(f"{tenant} did not reach a desired count of {count} agents")

    def agent_counts(self):
        """tenant -> registered agents, from the roster."""
        _, roster = self.api("GET", "/v1/agents", key=self.bootstrap_key, expect=(200,))
        items = roster.get("items", []) if isinstance(roster, dict) else []
        return {tenant: sum(1 for agent in items if agent.get("tenant_id") == tenant)
                for tenant in self.tenants}

    def wait_for_agent_count(self, count, timeout_s=300):
        deadline = time.time() + timeout_s
        counts = {}
        while time.time() < deadline:
            counts = self.agent_counts()
            if all(have >= count for have in counts.values()):
                return counts
            time.sleep(5)
        raise AssertionError(f"not every tenant has {count} registered agents: {counts}")

    # -- the workload ----------------------------------------------------------

    def publish_workload(self, tenant):
        """Publish each package of the workload to a tenant with the tenant's
        own write key, unless the tenant already has a good build of it."""
        rows = {}
        for package in workload.PACKAGES:
            rows[package.name] = self.ensure_version(
                tenant, package.name, package.fixture, "0.1.0", key=self.keys[tenant])
        return rows

    def ensure_driver_secret(self, tenant):
        """A write key of the tenant, kept as the tenant secret that its
        self-driving workflow reads. A new key each call: the old secret
        value is replaced, never read back."""
        _, created = self.api("POST", f"/v1/tenants/{tenant}/keys", key=self.bootstrap_key,
                              body={"name": f"{tenant}-self-driving", "role": "write"},
                              expect=(200, 201))
        fields = {"key": created["key"]}
        code, body = self.api("POST", f"/v1/tenants/{tenant}/secrets", key=self.bootstrap_key,
                              body={"name": workload.DRIVER_SECRET, "fields": fields}, expect=None)
        if code not in (200, 201):
            code, body = self.api("PUT", f"/v1/tenants/{tenant}/secrets/{workload.DRIVER_SECRET}",
                                  key=self.bootstrap_key, body={"fields": fields}, expect=None)
        if code not in (200, 201, 204):
            raise AssertionError(f"storing the self-driving secret of {tenant} -> {code}: "
                                 f"{str(body)[:200]}")

    def instance_params(self, tenant, schedule):
        if not schedule.bind_driver_secret:
            return {}
        others = [name for name in self.tenants if name != tenant]
        position = self.tenants.index(tenant)
        return {
            "tenant": tenant,
            "offset": workload.TENANT_OFFSETS[tenant],
            "other_tenant": self.tenants[(position + 1) % len(self.tenants)] if others else "",
            # A reference, not a value: the instance names the secret.
            workload.DRIVER_SECRET: {"$secret": workload.DRIVER_SECRET},
        }

    def setup_schedules(self, tenant):
        """Create the workflow instances of the workload that the tenant does
        not have yet. The other schedules come with their packages."""
        made = []
        for schedule in workload.SCHEDULES:
            if schedule.kind != "instance":
                continue
            path = f"/v1/tenants/{tenant}/workflows/{schedule.workflow}/instances"
            code, _ = self.api("GET", f"{path}/{schedule.instance}", key=self.keys[tenant], expect=None)
            if code == 200:
                continue
            body = {"instance_name": schedule.instance, "cron": schedule.cron,
                    "params": self.instance_params(tenant, schedule)}
            if schedule.catchup:
                body["catchup_policy"] = schedule.catchup
            if schedule.max_active_runs:
                body["max_active_runs"] = schedule.max_active_runs
            code, answer = self.api("POST", path, key=self.keys[tenant], body=body, expect=None)
            if code not in (200, 201, 409):
                raise AssertionError(f"creating the instance {schedule.instance} of "
                                     f"{schedule.workflow} in {tenant} -> {code}: {str(answer)[:300]}")
            made.append(f"{schedule.workflow}/{schedule.instance}")
        return made

    def ensure_workload(self):
        """Packages, secret and schedules for every tenant, once."""
        if self.workload_ready:
            return
        for tenant in self.tenants:
            self.publish_workload(tenant)
            self.ensure_driver_secret(tenant)
            self.setup_schedules(tenant)
        self.workload_ready = True

    # -- reading ---------------------------------------------------------------

    def cron_schedules(self, tenant):
        """workflow -> the cron schedules the server lists for it."""
        code, body = self.api("GET", f"/v1/tenants/{tenant}/triggers", key=self.keys[tenant],
                              expect=None)
        listed = {}
        if code == 200 and isinstance(body, dict):
            for item in body.get("items", []):
                if item.get("schedule_type") == "cron":
                    listed.setdefault(item.get("workflow_name"), []).append(item)
        return listed

    def runs(self, tenant, workflow, limit=50):
        """The newest runs of a workflow of a tenant."""
        code, body = self.api("GET", f"/v1/tenants/{tenant}/executions?workflow={workflow}&limit={limit}",
                              key=self.keys[tenant], expect=None)
        if code == 200 and isinstance(body, dict):
            return body.get("items", [])
        return []

    def driver_actions(self, tenant, runs=5):
        """What the self-driving workflow of a tenant recorded in its newest
        finished runs (read from the run's context in postgres)."""
        actions = []
        done = [run for run in self.runs(tenant, workload.DRIVER_WORKFLOW) if run.get("status") == "Completed"]
        for run in done[:runs]:
            context = self.execution_context(tenant, run["id"])
            actions.extend(context.get(workload.DRIVER_ACTIONS_KEY) or [])
        return actions

    # -- the checker's settings -------------------------------------------------

    @staticmethod
    def _write_owner_only(path, value):
        path = Path(path)
        path.parent.mkdir(parents=True, exist_ok=True)
        handle = os.open(str(path), os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
        with os.fdopen(handle, "w", encoding="utf-8") as out:
            json.dump(value, out, indent=1)
        os.chmod(str(path), 0o600)

    def write_checker_settings(self):
        """Write what the checker needs: the server URL, the database URL and
        a read key for each tenant. Owner-only: the file holds keys.

        The database credentials are the throwaway ones of the chart's test
        values, on a port that is forwarded to localhost."""
        settings = {
            "server": self.base_url,
            "database_url": f"postgres://cloacina:cloacina@127.0.0.1:{self.database_port}/cloacina",
            "keys": dict(self.checker_keys),
        }
        self._write_owner_only(self.settings_path, settings)
        return settings

    def write_disturb_settings(self, path):
        """Write what the disturbance schedule needs (CLOACINA-T-1109): the
        server URL, a write key for each tenant (to start held runs) and a
        read key (to read the roster). Owner-only: the file holds keys."""
        settings = {
            "server": self.base_url,
            "keys": dict(self.keys),
            "read_keys": dict(self.checker_keys),
        }
        self._write_owner_only(path, settings)
        return settings
