Feature: The cursor never passes an unwritten record
  A sync commits its cursor in the same step that marks the write done. A
  segment the destination did not take leaves the cursor where it was, so the
  next pass reads it again.

  Background:
    Given a Postgres source table "orders" with the records
      | id | name  |
      | 1  | Robin |
      | 2  | Sam   |
      | 3  | Jamie |
    And a Postgres destination table "orders_copy"
    And the sync copies "orders" by the cursor "id", keyed on "id"

  Scenario: A destination that refuses the first segment commits no cursor
    Given the destination's database refuses connections
    When the sync runs a pass
    Then the pass fails
    And no cursor is committed

  Scenario: A destination that refuses a later segment keeps the cursor where it was
    Given the sync runs a pass
    And the source table "orders" gains the records
      | id | name  |
      | 4  | Robin |
    And the destination's database refuses connections
    When the sync runs a pass
    Then the pass fails
    And the committed cursor is "3"
    And the destination table "orders_copy" holds the records
      | id | name  |
      | 1  | Robin |
      | 2  | Sam   |
      | 3  | Jamie |

  Scenario: The records the destination refused arrive when it is back
    Given the sync runs a pass
    And the source table "orders" gains the records
      | id | name  |
      | 4  | Robin |
    And the destination's database refuses connections
    And the sync runs a pass
    When the destination's database is back
    And the sync runs a pass
    Then the pass ends with the source exhausted
    And the destination table "orders_copy" holds the records
      | id | name  |
      | 1  | Robin |
      | 2  | Sam   |
      | 3  | Jamie |
      | 4  | Robin |
    And the committed cursor is "4"
