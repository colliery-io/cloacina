Feature: Dead-letters never stop a run
  A record that does not meet the stream's schema is set aside as a
  dead-letter, with the reason. The rest of its segment arrives, and the
  cursor moves on.

  Scenario: A record that breaks the schema is set aside and the rest arrive
    Given a Postgres source table "people" with the records
      | id | age     |
      | 1  | 30      |
      | 2  | unknown |
      | 3  | 41      |
    And a Postgres destination table "people_copy"
    And the sync copies "people" by the cursor "id", keyed on "id"
    And the stream's schema is
      | field | type    | nullable |
      | id    | integer | no       |
      | age   | integer | yes      |
    When the sync runs a pass
    Then the pass ends with the source exhausted
    And the destination table "people_copy" holds the records
      | id | age |
      | 1  | 30  |
      | 3  | 41  |
    And the dead-letters hold 1 record
    And a dead-letter holds the record with id 2, for its field "age"
    And the committed cursor is "3"

  Scenario: A segment where every record breaks the schema still commits its cursor
    Given a Postgres source table "people" with the records
      | id | age     |
      | 1  | unknown |
      | 2  | none    |
    And a Postgres destination table "people_copy"
    And the sync copies "people" by the cursor "id", keyed on "id"
    And the stream's schema is
      | field | type    | nullable |
      | id    | integer | no       |
      | age   | integer | yes      |
    When the sync runs a pass
    Then the pass ends with the source exhausted
    And the destination table "people_copy" holds no records
    And the dead-letters hold 2 records
    And the committed cursor is "2"
