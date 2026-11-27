"""Rules of the Publish row (CLOACINA-T-1026, CLOACINA-T-1108).

The self-driving workflow of each tenant publishes new versions of two
packages, sometimes with a build error, and sometimes goes back a version. It
records each of those actions. These rules compare that record with what the
server says about the packages now: the versions list of each package, read
with the tenant's own key.

The scenarios of `features/publishing/` judge the same promises once, step by
step. These rules judge them on every pass, for every tenant.
"""

from __future__ import annotations

from .. import workload
from ..engine import broken, format_time, holds, unknown
from . import _shared

CHAPTER = "publishing"

# A step back takes effect at once; give the server a minute.
STEP_BACK_SETTLE_SECONDS = 60


def register(registry):
    _shared.need_clock(registry)
    _shared.need_driver_runs(registry)
    for kind, package in sorted(workload.TARGETS.items()):
        registry.need_api(f"versions_target_{kind}",
                          "/v1/tenants/{tenant}/workflows/" + package + "/versions")

    def settled(snapshot, tenant, action):
        """The accepted records of one action that are old enough for their
        build to be over."""
        now = _shared.clock(snapshot)
        return [record for record in _shared.driver_actions(snapshot, tenant)
                if record.get("action") == action and _shared.accepted(record)
                and now - record["at_epoch"] >= _shared.SETTLE_SECONDS]

    @registry.rule("publishing.new_version_built", "publish.own_package", CHAPTER,
                   tolerates=("postgres_away", "restart_server"),
                   about="each version a team published is built, and no publish was refused "
                         "(the restart count is judged by the feature, not here)")
    def new_version_built(snapshot):
        now = _shared.clock(snapshot)
        wrong = []
        judged = 0
        for tenant in _shared.workload_tenants(snapshot):
            for record in _shared.driver_actions(snapshot, tenant):
                status = record.get("status")
                if (record.get("action") == "publish" and isinstance(status, int)
                        and not _shared.accepted(record)
                        and now - record["at_epoch"] <= 3 * workload.SLOT_SECONDS):
                    judged += 1
                    wrong.append(f"{tenant}: the publish of {record.get('package')} "
                                 f"{record.get('version')} at {record.get('at')} answered {status}")
            for record in settled(snapshot, tenant, "publish"):
                kind = _shared.kind_of(record.get("package"))
                if kind is None:
                    continue
                judged += 1
                row = _shared.versions(snapshot, tenant, kind).get(str(record.get("version")))
                if row is None:
                    wrong.append(f"{tenant}: {record.get('package')} {record.get('version')} was "
                                 "accepted and is not in the versions list")
                elif row.get("build_status") != "success":
                    wrong.append(f"{tenant}: {record.get('package')} {record.get('version')}, "
                                 f"published at {record.get('at')}, is {row.get('build_status')}")
        if wrong:
            return broken(f"{len(wrong)} of {judged} publishes did not end as a built version", wrong)
        if not judged:
            return unknown("no publish is old enough to judge yet")
        return holds(f"{judged} published versions are built")

    @registry.rule("publishing.bad_build_keeps_running", "publish.bad_build", CHAPTER,
                   about="a version with a build error is marked failed and is not in service, "
                         "and the package still has a built version in service")
    def bad_build_keeps_running(snapshot):
        wrong = []
        judged = 0
        for tenant in _shared.workload_tenants(snapshot):
            for record in settled(snapshot, tenant, "publish_broken"):
                kind = _shared.kind_of(record.get("package"))
                if kind is None:
                    continue
                judged += 1
                rows = _shared.versions(snapshot, tenant, kind)
                row = rows.get(str(record.get("version")))
                name = f"{tenant}: {record.get('package')} {record.get('version')}"
                if row is None:
                    wrong.append(f"{name} (build error) is not in the versions list")
                    continue
                if row.get("build_status") != "failed":
                    wrong.append(f"{name} (build error) is {row.get('build_status')}, not failed")
                if row.get("active"):
                    wrong.append(f"{name} (build error) is in service")
                in_service = [r for r in rows.values() if r.get("active")]
                if not in_service:
                    wrong.append(f"{tenant}: {record.get('package')} has no version in service "
                                 "after the failed build")
                elif in_service[0].get("build_status") != "success":
                    wrong.append(f"{tenant}: {record.get('package')} has "
                                 f"{in_service[0].get('version')} in service, which is "
                                 f"{in_service[0].get('build_status')}")
        if wrong:
            return broken(f"{len(wrong)} faults after {judged} publishes with a build error", wrong)
        if not judged:
            return unknown("no publish with a build error is old enough to judge yet")
        return holds(f"{judged} publishes with a build error left a built version in service")

    @registry.rule("publishing.step_back", "publish.step_back", CHAPTER,
                   about="after a team goes back a version, that version is the one in service, "
                         "until the team publishes again")
    def step_back(snapshot):
        now = _shared.clock(snapshot)
        wrong = []
        seen = []
        for tenant in _shared.workload_tenants(snapshot):
            # The last accepted action that changes the version in service.
            changes = [record for record in _shared.driver_actions(snapshot, tenant)
                       if record.get("action") in ("publish", "step_back")
                       and _shared.accepted(record)]
            for kind in sorted(workload.TARGETS):
                mine = [r for r in changes if r.get("package") == workload.TARGETS[kind]]
                if not mine or mine[-1].get("action") != "step_back":
                    continue
                last = mine[-1]
                if now - last["at_epoch"] < STEP_BACK_SETTLE_SECONDS:
                    continue
                rows = _shared.versions(snapshot, tenant, kind)
                in_service = [r.get("version") for r in rows.values() if r.get("active")]
                if in_service == [last.get("version")]:
                    seen.append(f"{tenant} {last.get('package')} {last.get('version')}")
                else:
                    wrong.append(f"{tenant}: went back to {last.get('package')} "
                                 f"{last.get('version')} at {format_time(last['at_epoch'])}; "
                                 f"in service now: {', '.join(map(str, in_service)) or 'nothing'}")
        if wrong:
            return broken("; ".join(wrong), wrong)
        if not seen:
            return unknown("no tenant's last version change is a step back right now")
        return holds("in service after the step back: " + ", ".join(seen))
