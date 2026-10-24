Feature: A sync pass
  A pass reads the source segment by segment until it ends. Each segment is
  written to the destination, and the cursor is committed after the write.

  Background:
    Given a Postgres source table "orders" with the records
      | id | name  |
      | 1  | Robin |
      | 2  | Sam   |
      | 3  | Jamie |
    And a Postgres destination table "orders_copy"
    And the sync copies "orders" by the cursor "id", keyed on "id"

  Scenario: The records arrive and the cursor follows them
    When the sync runs a pass
    Then the pass ends with the source exhausted
    And the destination table "orders_copy" holds the records
      | id | name  |
      | 1  | Robin |
      | 2  | Sam   |
      | 3  | Jamie |
    And the committed cursor is "3"

  Scenario: The next pass starts from the cursor
    Given the sync runs a pass
    When the source table "orders" gains the records
      | id | name  |
      | 4  | Robin |
    And the sync runs a pass
    Then the destination table "orders_copy" holds the records
      | id | name  |
      | 1  | Robin |
      | 2  | Sam   |
      | 3  | Jamie |
      | 4  | Robin |
    And the committed cursor is "4"
