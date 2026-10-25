Feature: A pass is several executions, shown as one
  Each segment of a pass is an execution. The executions carry the name of
  the sync definition that fired them and the id of their pass, so a pass can
  be shown as one entry.

  Background:
    Given a sync definition "orders" that reads 3 segments from an echo source

  Scenario: A pass's executions share its id and name its sync definition
    When the sync definition "orders" runs a pass
    Then the pass ends with the source exhausted
    And the sync definition "orders" has 3 executions, each naming it
    And the passes of "orders" have "3" executions

  Scenario: Each pass has an id of its own
    Given the sync definition "orders" runs a pass
    When the sync definition "orders" runs a pass
    Then the passes of "orders" have "3, 1" executions
