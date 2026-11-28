# CLOACINA-T-1112. Driver: `angreal test bdd standing --chapter failure`
# (see ../README.md). It runs on the standing deployment, beside its workload.
#
# Each scenario asks the disturbance schedule (CLOACINA-T-1109) for one
# disturbance now, through the same runner, and reads its window from the
# disturbance record. The checker's rules of this chapter
# (`.angreal/test/bdd/suite/rules/failure.py`) judge every disturbance of the
# schedule the same way, all the time.
#
# A scenario tagged @gap does not hold today. It still runs. The second tag
# names the Kairos task that owns the gap, or is @unfiled when no task owns
# it yet.
@failure
Feature: Work survives when a part of the orchestrator goes away

  An agent dies, a server replica restarts, Postgres goes away for minutes.
  The work that was running finishes, and nothing is lost or doubled beyond
  the stated rule.

  The stated rule (docs: explanation/service/execution-agent-fleet.md, the
  sections on a replica that stops, recovery and stopping an agent; and
  explanation/engine/cron-scheduling.md, "Missed execution handling"):

  - A task on an agent that dies runs again from its beginning on a live
    agent: at least once. An agent that is stopped the Kubernetes way lets
    its task finish within its drain time, or hands it back to run again on
    another agent.
  - A task that runs on an agent when a server replica stops finishes once:
    another replica takes over the wait. No task is sent again.
  - A fire time missed while the scheduler cannot reach Postgres follows the
    schedule's catch-up policy: "skip" gives one run for the fire times it
    missed, "run_all" gives one run for each, oldest first. No fire time has
    two runs.
  - The docs name no rule for a task that runs while Postgres is away. This
    chapter holds it to the rule of a server restart: it ends, once.

  Background:
    Given the standing workload runs for team A, team B and team C
    And the disturbance schedule acts on the suite's own cluster only

  Scenario: An agent dies while it runs a task, and the task finishes on another agent
    Given team A has a held job of five minutes running on one of its agents
    When that agent's pod is killed with no time to stop
    Then the disturbance is on record with the pod and its window
    And the held job's run ends Completed within the window
    And the held job's task started again on another agent

  Scenario: An agent is stopped the way Kubernetes stops a pod, and its task still finishes
    Given team B has a held job of five minutes running on one of its agents
    When that agent's pod is stopped the way Kubernetes stops a pod
    Then the disturbance is on record with the pod and its window
    And the held job's run ends Completed within the window

  Scenario: Every server replica restarts while a task runs, and the task finishes once
    Given team C has a held job of five minutes running on one of its agents
    When each server replica in turn is stopped with no time to stop
    Then each restart is on record with the number of server replicas
    And the orchestrator is ready again
    And the held job's run ends Completed within the window
    And the held job's task started once

  Scenario: Postgres goes away for minutes, and each fire time follows its catch-up policy
    Given team A's extract job has a schedule that runs every missed fire time, one at a time
    When Postgres goes away for eleven minutes and comes back
    Then the disturbance is on record with its window
    And the orchestrator is ready again
    And each fire time of the window has one run of team A's in-order extract schedule
    And each five-minute schedule of team A that skips missed fire times has one run for the window
    And no fire time of the window has two runs
    And every run of team A that was active when Postgres went away has ended
