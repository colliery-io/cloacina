Feature: A sync resumes after a crash
  A crash can fall after the destination took a segment and before the cursor
  commits. The segment is then written again, the destination keeps one copy
  of each record, and the cursor moves once.

  Background:
    Given a Postgres source table "orders" with the records
      | id | name  |
      | 1  | Robin |
      | 2  | Sam   |
      | 3  | Jamie |
    And a Postgres destination table "orders_copy"
    And the sync copies "orders" by the cursor "id", keyed on "id"

  Scenario: A write task that dies before the commit is retried
    Given the write task dies once, after the destination has the segment and before the cursor commits
    When the sync runs a pass
    Then the pass ends with the source exhausted
    And the segment's write ran twice
    And the destination table "orders_copy" holds the records
      | id | name  |
      | 1  | Robin |
      | 2  | Sam   |
      | 3  | Jamie |
    And the committed cursor is "3"

  Scenario: A new runner finishes the segment of a runner that died mid-write
    When the runner dies while it writes the segment, before the cursor commits
    Then no cursor is committed
    When a new runner runs a pass
    Then the pass ends with the source exhausted
    And the destination table "orders_copy" holds the records
      | id | name  |
      | 1  | Robin |
      | 2  | Sam   |
      | 3  | Jamie |
    And the committed cursor is "3"
