# CLOACINA-T-1115. Driver: `angreal test bdd standing --chapter operations`
# (see ../README.md). It runs on the standing deployment, with the workload of
# CLOACINA-T-1107 in place.
#
# The checker judges the same promises on every pass
# (`suite/rules/operations.py`). The deployment's alert webhook points at the
# suite's alert receiver (`suite/alerts.py`), which runs in the deployment's
# namespace; the checker and these steps read it through a port-forward.
#
# Team A keeps one run that waits for ever (scenario 6): the retention
# scenario looks for it from the third day, when the deployment is older than
# its retention period of 2 days.
#
# A scenario tagged @gap does not hold today. It still runs. The second tag
# names the Kairos task that owns the gap, or is @unfiled when no task owns
# it yet.
@operations
Feature: The people who run the orchestrator see what it does

  An operator learns of a run that failed for good from an alert, and of
  nothing that recovered by itself. Who did what is on record, and a team
  sees only its own record. The backlog and the lag are on the metrics, and
  they go back down when the work is done. Old history goes when retention
  is set, and work that is still active stays. The server's log holds no
  task output and no secret.

  Background:
    Given the standing workload is in place for each team

  Scenario: A run that fails for good raises one alert, with its tenant, its workflow and its run
    Given the alert receiver of the deployment is up
    And team A has published the operations packages
    When team A starts a run of the failing job that fails for good
    Then the run ends Failed
    And one alert for that run arrives within two minutes
    And the alert names team A, the failing job and the run

  Scenario: A run that fails once and then passes on a retry raises no alert
    Given the alert receiver of the deployment is up
    And team A has published the operations packages
    When team A starts a run of the failing job that fails once and then passes
    Then the run ends Completed after a failed attempt
    And the failed attempt's error is on record with the run
    And no alert for that run arrives within two minutes

  Scenario: Each publish, version change and run by hand of the self-driving workflow is on record
    When team A's admin reads team A's audit record
    Then each publish, version change and run by hand in the last five runs of team A's self-driving workflow has a row
    And each of those rows names the key of the self-driving workflow and team A

  Scenario: A team reads only its own audit record
    When team A's admin reads team A's audit record
    Then every row of the answer is a row of team A
    And team A's admin key cannot read team B's audit record

  Scenario: The backlog and the lag of a full team are visible, and go back after
    When team A starts more holds at once than its agents can take
    And team A makes a schedule of the hold job each minute, with one active run at most
    Then team A's waiting gauge rises above 0
    And team A's schedule lag gauge rises above 30 seconds
    And team A removes the schedule
    And team A's waiting gauge and schedule lag gauge go back to 0 once the holds end

  Scenario: Team A keeps a run that waits for ever
    Given team A has published the operations packages
    When team A has a run of the job of the pool no agent serves
    Then that run waits, and is active

  Scenario: Old history is removed when retention is set, and active work stays
    Given the server runs with a retention of 2 days
    And team A has run for longer than the retention period
    Then no finished run of any team is older than the retention period
    And team A's run that waits is still there, older than the retention period

  Scenario: The server log holds no task output and no secret while the workload runs
    Given team A has published the operations packages
    When team A's self-driving workflow gets a new key in its secret
    And the self-driving workflow of team A has run with it
    And team A starts a run of the failing job that fails for good
    Then the run ends Failed
    And the server log since the scenario began holds no line a task printed
    And the server log holds no value of team A's secret

  # The output of each attempt is CLOACINA-T-1091 (PR #474), which lands with
  # the fifth integration branch.
  @gap @CLOACINA-T-1091
  Scenario: An operator reads the output of a failed attempt with its run
    Given team A has published the operations packages
    When team A starts a run of the failing job that fails for good
    Then the run ends Failed
    And the output of its failed attempt holds the line the task printed
