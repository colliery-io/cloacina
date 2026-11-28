# CLOACINA-T-1110. Driver: `angreal test bdd standing --chapter run_once`.
#
# The Run once row of the behaviour suite. Each scenario shows one promise
# once, on the standing deployment. The checker's rules of the chapter
# (`.angreal/test/bdd/suite/rules/run_once.py`) judge the same promises on
# every pass, against the standing workload.
#
# "Ran once" is counted from the task itself: each task body prints one mark
# when it starts, and the count of marks is compared with the attempts the
# server recorded. A count from the server's own rows cannot show a start the
# server did not record.
#
# A scenario tagged @gap does not hold today. It still runs. The second tag
# names the Kairos task that owns the gap. When such a scenario passes, the
# run says so loudly: remove the tags then. A scenario that fails on a run
# stays here, tagged with the task that owns its gap.
@run_once
Feature: A task runs one time

  Several teams share one server, which runs two replicas, and each team has
  its own agents. A team's task runs one time: when it runs longer than the
  server's push and heartbeat periods, when it has to wait for a free agent,
  and when its result reaches a replica of the server that did not send it
  out.

  Background:
    Given a shared orchestrator runs in Kubernetes with two server replicas
    And team A has the packages of the standing workload in service

  Scenario: A task that runs longer than the push and heartbeat periods runs one time
    When team A starts the long job by hand, holding for six minutes
    Then the run of the long job completes
    And the server recorded one attempt of the long task
    And the body of the long task started one time
    And the long task held for more than five minutes

  Scenario: A task with no free agent waits, and then runs one time
    Given every agent slot of team A is taken by a held task
    When team A starts two more held tasks
    Then those two tasks wait for a free agent
    And each of those two tasks completes once an agent is free
    And the body of each of those two tasks started one time
    And neither of those two tasks used up an attempt while it waited

  Scenario: A task whose result reaches the other server replica runs one time
    When team A runs short held tasks until a result reaches the replica that did not dispatch its task
    Then a server replica stored that task's result for the replica that dispatched it
    And that task completed
    And the body of that task started one time
    And no stored result of that task is left untaken
