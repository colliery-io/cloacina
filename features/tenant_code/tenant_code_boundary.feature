# CLOACINA-T-1079. Driver: `angreal test bdd standing --chapter tenant_code`.
#
# One scenario for each claim of the ADR CLOACINA-A-0019 ("Claims a test must
# show"), in the order of the ADR, each named after its claim. They show in
# the report under "Tenants", next to the chapter of CLOACINA-T-1113.
#
# A scenario tagged @gap does not hold today. It still runs. The second tag
# names the Kairos task that owns the gap. When such a scenario passes, the
# run says so loudly: remove the tags then.
@tenant_code
Feature: Tenant code runs only on the tenant's agents and in the tenant's compiler

  Several teams share one server. The server process holds every team's
  data and keys, so no team's code runs in it: not a library's load-time
  code, not a Python import, not a task, a trigger, a graph or a native
  provider. A team's code runs on that team's agents, and its build runs in
  that team's compiler, with a credential that opens that team's build
  tables only.

  Background:
    Given a shared orchestrator runs in Kubernetes with tenant compilers and agents
    And team D has its own tenant, keys, compiler and agent

  Scenario: A Rust package's load-time code never runs in the server process
    Given team D has the Rust mark package in service
    When team D runs the Rust mark workflow
    Then the task of the run left its mark in a process on an agent of team D
    And the package's load-time code ran in that same process
    And no log line of the server carries the mark of the Rust package
    And no log line of the compiler's own process carries the mark of the Rust package
    And the server process has no library of a tenant open

  Scenario: A Python package's import never runs in the server process
    Given team D has the Python mark package in service
    When team D runs the Python mark workflow
    Then the task of the run left its mark in a process on an agent of team D
    And the package's import ran in that same process
    And no log line of the server carries the mark of the Python package
    And no log line of the compiler's own process carries the mark of the Python package

  Scenario: A default chart install runs no packaged work in the server process
    When the chart is rendered with its default values
    Then the render is refused until the install says where the agents come from
    When the chart is rendered with the Kubernetes actuator and nothing else
    Then the server it renders does not run with single trust
    And the server of this deployment does not run with single trust
    Given team D has the Rust mark package and the Python mark package in service
    When team D runs both mark workflows
    Then every task of the mark packages ran on an agent

  Scenario: A graph firing with no agent fails with a reason and does not run in the server
    Given team D has the graph mark package in service
    And a firing of team D's reactor ran on an agent of team D
    When team D has no live agent
    And team D fires its reactor
    Then the firing failed with a reason that names the missing agent
    And no log line of the server carries the mark of the graph

  Scenario: A poll trigger's code runs on the tenant's agent, not in the server
    Given team D has the Rust mark package in service
    When the poll trigger of the Rust mark package has fired a run
    Then the poll trigger's code ran in a process on an agent of team D
    And no log line of the server carries the poll trigger's mark

  Scenario: Native provider and Python accumulator code do not run in the server, and a WASM provider runs under its grants
    When team D publishes a package whose stream source is a native provider
    Then the build of that package fails with a reason that names its native provider
    When team D publishes a Python package with a polling accumulator
    Then the build of that package fails with a reason that names its polling accumulator
    And no log line of the server carries the mark of the polling accumulator
    When team D publishes a package whose task uses a WASM provider
    Then that package is built and in service
    And a run of it reads the file its grant allows
    When team D publishes a version whose WASM provider reads outside its grant
    Then a run of that version fails, and nothing outside the grant was read

  Scenario: A package of public is treated as a package of a named tenant
    Given the tenant public has a key, a compiler and an agent
    And public has the Rust mark package in service
    When public runs the Rust mark workflow
    Then the task of the run left its mark in a process on an agent of public
    And the package's load-time code ran in that same process
    And no log line of the server carries the mark of the Rust package
    And no log line of the compiler's own process carries the mark of the Rust package
    And public's compiler connects to Postgres with the role public$build

  Scenario: A tenant's compiler holds a credential that opens only that tenant's build tables
    Given team D's compiler has built a package
    Then team D's compiler connects to Postgres with the role team_d$build
    And that role can read team D's build tables
    And Postgres refuses that role a read of another tenant's tables
    And Postgres refuses that role a read of the server's own tables in public and cloacina_global
    And no build role has a right outside its own tenant's build tables

  # The real path, a new server image over packages that an older compiler
  # built, needs two versions of the images: it is the Upgrade chapter
  # (CLOACINA-T-1046). This scenario makes the same state on one image.
  Scenario: A package built before the change runs after an upgrade with no publish
    Given team D has the Rust mark package in service
    When the stored description of the Rust mark package is removed, as for a package built before the change
    Then team D's compiler describes the package again with no publish
    And a new run of the Rust mark workflow completes on an agent of team D

  Scenario: A query from a tenant's pool on another tenant's schema is refused by Postgres
    Given team D has the Rust mark package in service
    When team D runs the Rust mark workflow
    Then the server's connections for team D use team D's server role
    And Postgres refuses team D's server role a read of another tenant's tables
    And team D's server role can read team D's own tables
