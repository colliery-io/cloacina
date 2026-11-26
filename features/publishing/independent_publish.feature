# CLOACINA-T-1026. Driver: `angreal test bdd publish` (see README.md here).
#
# A scenario tagged @gap does not hold today. It still runs. The second tag
# names the Kairos task that owns the gap, or is @unfiled when no task owns it
# yet. When such a scenario passes, the run says so loudly: remove the tags
# then.
@publishing
Feature: A team publishes its own jobs to a shared orchestrator

  Teams author, compile and link their jobs to a central orchestrator on
  their own, safely, and the orchestrator runs them. The publish is "compile
  and push, however it happens": a CI job, a build, or a person at a shell.
  No team puts its job code in a central repository of jobs.

  Background:
    Given a shared orchestrator runs in Kubernetes with a compiler and agents
    And team A and team B each have their own tenant, publish key, secret and agent

  Scenario: A team builds and validates a package from its own repository with no server
    Given team A keeps its report job and its orders job in its own repository
    When team A builds, validates and packs both jobs with no server to talk to
    Then each job becomes one package file that is ready to publish

  # Observed on k3s, 2026-11-26: the package compiles and loads, but its
  # schedule does not start. The server makes a tenant's runner on the first
  # execute, trigger fire or instance call for that tenant, not on a publish.
  @gap @unfiled
  Scenario: A team publishes with its own key and the schedule starts with no restart
    Given team A has packed version 0.1.0 of its report job
    When team A publishes the report job with its own key
    Then the orchestrator compiles the report job
    And the report job runs on its schedule
    And the orchestrator was not restarted

  Scenario: A publish with a build error leaves the running version in service
    Given version 0.1.0 of team A's report job is in service
    When team A publishes version 0.2.0 of the report job, which does not compile
    Then team A sees that the build failed, with the compiler's error
    And team A's list of versions of the report job shows 0.1.0 in service
    And the report job still runs on its schedule

  Scenario: A new version arrives while an execution runs
    Given version 0.1.0 of team A's orders job is in service
    And an execution of the orders job is running
    When team A publishes version 0.2.0 of the orders job
    And version 0.2.0 of the orders job comes into service
    Then the running execution finishes
    And every task of the running execution ran on version 0.1.0
    And a new execution of the orders job runs on version 0.2.0

  Scenario: A team goes back to an earlier version with one command
    Given team A has published versions 0.1.0 and 0.2.0 of its orders job
    And a new execution of the orders job runs on version 0.2.0
    When team A goes back to version 0.1.0 of the orders job with one command
    Then team A's list of versions of the orders job shows 0.1.0 in service
    And a new execution of the orders job runs on version 0.1.0

  Scenario: A key of one team does nothing in another team's tenant
    Given team B has published its ledger job
    When Robin of team A uses team A's key against team B's tenant
    Then Robin cannot publish a package to team B
    And Robin cannot read team B's packages, executions or secrets
    And Robin cannot run team B's ledger job
    And the same calls succeed with team B's key

  @gap @CLOACINA-T-1025
  Scenario: A package not signed by a key the team trusts is refused
    Given signing is turned on for the orchestrator
    When team A publishes a package with no signature
    Then the publish is refused
    When team A registers Robin's signing key as a key it trusts
    And Robin signs a package and publishes it to team A
    Then the publish is accepted
    When Sam, whose key team A does not trust, signs a package and publishes it to team A
    Then the publish is refused

  # Observed on k3s, 2026-11-26: every step holds but the last. The server
  # process imports a team's Python package, with the database URL, the
  # bootstrap key and the secret KEK in its environment.
  @gap @CLOACINA-T-1022
  Scenario: A task of one team cannot read another team's packages or secrets
    Given team B has published its ledger job
    And team B keeps a secret in its tenant
    When team A publishes a job whose task tries to read what belongs to team B
    And that job runs
    Then the task could not fetch team B's package
    And the task could not read team B's secret
    And the task could not use its key in team B's tenant
    And the task found nothing of team B in its environment or on its disk
    And the task could not reach the orchestrator's database
    And team A's code never ran where the orchestrator keeps its own credentials
