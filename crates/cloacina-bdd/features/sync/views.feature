Feature: What a sync shows
  A sync's view names the sync, not a workflow: its passes with their
  segments, the records it set aside, and its stream's cursor and schema.

  Scenario: A sync's passes are listed newest first, each with its segments
    Given a sync definition "orders" that reads 3 segments from an echo source
    And the sync definition "orders" runs a pass
    And the sync definition "orders" runs a pass
    Then the syncs listed are "orders"
    And the passes of "orders" are listed newest first with "1, 3" segments
    And the newest pass of "orders" ended on segment 3
    And the stream of "orders" shows the cursor at segment 3, with the field "id" as Integer

  Scenario: A sync's dead-letters are listed with their reasons
    Given a sync definition "people" whose echo source sends these records, with the field "id" as an integer
      | id |
      | 1  |
      | x  |
    And the sync definition "people" runs a pass
    Then the dead-letters of "people" list 1 record, for its field "id"

  Scenario: A run by hand shows as a pass of its sync
    Given a sync definition "orders" that reads 2 segments from an echo source
    When the sync "orders" is run by hand
    Then the passes of "orders" are listed newest first with "2" segments
    And the newest pass of "orders" ended on segment 2

  Scenario: A sync that does not exist is named
    Then reading the sync "nope" says there is no such sync
