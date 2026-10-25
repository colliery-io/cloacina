Feature: A sync definition can be changed and keeps its cursor
  Changing a sync definition replaces its params. Its committed cursor and
  its dead-letters are kept under the sync's name, so the next pass carries
  on from where the last one stopped, and the name cannot change.

  Background:
    Given a sync definition "orders" that reads 3 segments from an echo source
    And the sync definition "orders" runs a pass

  Scenario: A changed sync carries on from its cursor
    When the sync definition "orders" is changed to read 5 segments
    Then the change is accepted
    When the sync definition "orders" runs a pass
    Then the passes of "orders" have "3, 2" executions
    And the sync "orders" has read up to segment 5

  Scenario: A sync cannot be renamed
    When the sync definition "orders" is changed to the sync name "orders-2"
    Then the change is refused, saying "cannot be renamed"
    And the sync "orders" has read up to segment 3
