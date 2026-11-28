# CLOACINA-T-1107. Driver: `angreal test bdd standing` (see ../README.md).
#
# This feature brings the standing workload of the behaviour suite up and
# says that it runs. It does not judge the product's promises: the checker
# does that, all the time, against the work this feature starts.
#
# A scenario tagged @gap does not hold today. It still runs. The second tag
# names the Kairos task that owns the gap, or is @unfiled when no task owns
# it yet.
@standing
Feature: A standing workload of several teams runs without end

  Several teams share one orchestrator. Each has its own tenant and its own
  agents, and runs real work on it without end: data jobs on five-minute and
  hourly schedules, and a workflow that drives the orchestrator itself, by
  publishing new versions of other packages, starting other workflows and
  starting runs by hand.

  Background:
    Given a shared orchestrator runs in Kubernetes with a compiler for each team
    And team A, team B and team C each have their own tenant, keys and agents

  Scenario: Each team publishes the packages of the workload with its own key
    When each team publishes the packages of the standing workload
    Then every package of every team is built and in service
    And the Rust package of each team was built in that team's tenant

  Scenario: Each team's schedules are in place
    Given the packages of the standing workload are in service for each team
    And each team keeps a write key of its own as a secret for its self-driving workflow
    When each team sets up the schedules of the standing workload
    Then each team's list of schedules holds every schedule of the workload
    And each of those schedules is switched on

  Scenario: The scheduled jobs run for each team
    Given the schedules of the standing workload are in place for each team
    When one period of the five-minute jobs has passed
    Then each five-minute job of each team has a finished run
    And the data jobs wrote back the interval they were given

  Scenario: The self-driving workflow acts on the orchestrator it runs on
    Given the schedules of the standing workload are in place for each team
    When the self-driving workflow of each team has finished a run
    Then each team's self-driving workflow published its two target packages
    And each team's self-driving workflow started a run by hand
    And each team's self-driving workflow got nothing from another team's tenant

  Scenario: The checker has what it needs to judge the workload
    Given the schedules of the standing workload are in place for each team
    When the driver writes the settings of the checker
    Then the settings hold the server, the database and a key for each team
    And only the owner of the settings file can read it
