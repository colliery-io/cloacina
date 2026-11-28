# CLOACINA-T-1114. Driver: `angreal test bdd standing --chapter capacity`
# (see ../README.md). It runs on the standing deployment, with the workload of
# CLOACINA-T-1107 in place.
#
# The checker judges the same promises on every pass, against the bursts of
# the standing workload (`suite/rules/capacity.py`).
#
# Team A is the team that fills up. The chapter adds agent Deployments of its
# own in team A's namespace (copies of team A's agent, with a pool and totals)
# and removes them; one scenario raises team A's agent limit by one and puts
# it back.
#
# A scenario tagged @gap does not hold today. It still runs. The second tag
# names the Kairos task that owns the gap, or is @unfiled when no task owns
# it yet.
@capacity
Feature: Work waits when a team is full, and a task lands on an agent that fits it

  A team's agents have a fixed number of slots, and an agent has a fixed
  amount of CPU and memory. Work that finds no room waits: it does not fail,
  it does not use a retry, and it runs when room comes, the most important
  first. A team that is full does not slow another team. A task that names
  an agent pool and a size runs only on an agent of that pool with room for
  it.

  Background:
    Given the standing workload is in place for each team

  Scenario: Work waits while a team is full, and runs when its held work ends
    When team A starts more holds at once than its agents can take
    Then some of team A's holds wait for a free agent
    And no task of team A fails while its agents are full
    And every hold of team A completed
    And each hold of team A ran one time

  Scenario: Waiting work runs when the team gets one more agent
    Given team A may have one agent more than it has
    When team A starts more holds at once than its agents can take
    And team A gets one more agent
    Then more holds of team A ran at once than its first agents have slots
    And every hold of team A completed
    And each hold of team A ran one time
    And team A goes back to its own number of agents

  Scenario: A full team does not slow another team's schedules
    When team A's agents are full across a fire time of the five-minute jobs
    Then the five-minute jobs of team B and team C started on time at that fire time
    And every hold of team A completed

  Scenario: A task that names a pool and a size runs only on an agent of that pool with room for it
    Given team A has an agent in the pool "sized" with 1 CPU and 1Gi of memory
    And team A has published the packages of sized tasks
    When team A starts four runs of the sized task at once
    Then each sized task ran on the agent of the pool "sized"
    And the sized agent never held more sized tasks than its memory has room for
    And every sized run of team A completed
    And team A's agent of the pool "sized" is removed

  Scenario: A task that fits no agent waits, and does not fail
    Given team A has an agent in the pool "sized" with 1 CPU and 1Gi of memory
    And team A has published the packages of sized tasks
    When team A starts a run of a task larger than any agent of its pool
    And 3 minutes pass
    Then the task waits, and has not failed
    And the task runs once an agent of the pool with room for it is added
    And team A's agent of the pool "sized" is removed

  # The server keeps the reason of a wait for capacity in a debug log line
  # only (`defer_for_capacity` in fleet_executor.rs): the task's view has none.
  @gap @unfiled
  Scenario: A task that fits no agent says why it waits
    Given team A has an agent in the pool "sized" with 1 CPU and 1Gi of memory
    And team A has published the packages of sized tasks
    When team A starts a run of a task larger than any agent of its pool
    And 1 minute passes
    Then the task's own view says why it waits, naming its pool and its request
    And the task runs once an agent of the pool with room for it is added
    And team A's agent of the pool "sized" is removed

  Scenario: Waiting work of a higher priority is dispatched first
    When team A fills its agents with holds
    And team A starts holds at priority 9, and then holds at priority 5
    Then each run got the priority it asked for
    And the waiting gauge of team A counts its waiting holds at priority 9
    And no hold at priority 9 started while a hold at priority 5 still waited
    And every hold of team A completed

  # Schedule and instance priority is the second part of CLOACINA-T-1123
  # (PR #473), which lands with the fifth integration branch.
  @gap @CLOACINA-T-1123
  Scenario: A schedule made with a priority fires its runs at that priority
    When team A makes a schedule of the hold job that fires each minute at priority 7
    Then the first run of that schedule is at priority 7
    And team A removes the schedule
