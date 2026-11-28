# CLOACINA-T-1111. Driver: `angreal test bdd standing --chapter schedules`.
#
# The Schedules row of the behaviour suite. Each scenario shows one promise
# once, on the standing deployment. The checker's rules of the chapter
# (`.angreal/test/bdd/suite/rules/schedules.py`) judge the same promises on
# every pass, for every schedule of the standing workload.
#
# The fire times a scenario expects come from the schedule's cron
# expression, computed by the driver (`suite/cron.py`), not from the
# server. "On time" is a start within 60 seconds of the fire time: the limit
# of the checker's rule `schedules.on_time`. The first full run measures the
# real lateness, and the limit is set from it in both places.
#
# Scenarios 5 and 6 restart both server replicas. They write each restart to
# the disturbance record, so the checker does not count it as a break.
#
# A scenario tagged @gap does not hold today. It still runs. The second tag
# names the Kairos task that owns the gap. When such a scenario passes, the
# run says so loudly: remove the tags then. A scenario that fails on a run
# stays here, tagged with the task that owns its gap.
@schedules
Feature: A schedule fires on time, a run knows its interval, and a backlog drains in order

  Several teams share one server. Each team's schedules fire at the times
  their cron expressions name, in their own time zone, while other work is
  slow, and when the server restarts. Each run knows the interval it covers.
  A schedule that runs every fire time it missed runs them oldest first, no
  more at once than its limit.

  Background:
    Given a shared orchestrator runs in Kubernetes with the standing workload of each team

  Scenario: A schedule fires on time while other work is slow
    Given every agent slot of team C is held for six minutes
    When the next fire time of the five-minute jobs has passed
    Then each five-minute schedule of team C started its run for that fire time within 60 seconds
    And team C's held tasks were still running at that fire time

  Scenario: A run knows its interval
    When the next fire time of the five-minute jobs has passed
    Then each data job of each team completed a run for that fire time
    And each of those runs wrote the interval from the fire time before it to its own

  Scenario: A backlog drains in order at its limit
    Given team B has an every-minute schedule of the extract job that runs every missed fire time, one run at a time
    When every agent slot of team B is held for four minutes
    And the held tasks of team B have ended
    And the schedule has run every fire time that is due
    Then the schedule had a backlog: runs that started more than a minute after their fire time
    And its runs started in the order of their fire times
    And no two of its runs were active at once
    And every fire time since the schedule was made has exactly one run
    And team B can remove that schedule

  Scenario: A schedule with a time zone fires at the local time it names
    When team B makes an hourly schedule of the extract job for a minute of the hour in Asia/Kathmandu, a few minutes from now
    Then the server's next fire time of that schedule is that minute
    And the schedule fires one run for that minute
    And that run covers the hour before that minute
    And team B can remove that schedule

  Scenario: A fire that is due while the server restarts is not lost
    Given the next fire time of the five-minute jobs is a minute away
    When both server replicas are restarted together just before that fire time
    And the server replicas are serving again
    Then no server replica was serving at that fire time
    And each five-minute schedule of team A has one run for that fire time

  Scenario: Two server replicas that start together register each schedule once
    Given the server runs two ready replicas
    When both server replicas are restarted together
    And the server replicas are serving again
    Then each schedule of the workload is registered once in each team
    And at the next fire time each five-minute schedule of each team has one run
