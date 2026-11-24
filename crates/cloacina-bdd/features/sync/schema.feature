Feature: A stream's schema is captured and evolves
  Each write captures the shape of the records it sees. A new field joins
  the schema. A field whose values no longer fit its type marks the schema
  broken, and those records go to the dead-letters; nothing pauses. Accepting
  the new shape lets the next write capture it.

  Background:
    Given a Postgres source table "people" with the records
      | id | age |
      | 1  | 30  |
      | 2  | 41  |
    And an echo destination
    And the sync copies "people" by the cursor "id", keyed on "id"
    And the sync runs a pass

  Scenario: The first write captures the records' shape
    Then the stream's schema has the field "age" as Integer
    And the stream's schema is not broken

  Scenario: A new field joins the schema
    When the source table "people" gains the text column "email"
    And the source table "people" gains the records
      | id | age | email         |
      | 3  | 52  | sam@example.org |
    And the sync runs a pass
    Then the stream's schema has the field "email" as Str
    And the stream's schema has the field "age" as Integer
    And the stream's schema is not broken

  Scenario: A field whose values no longer fit marks the schema broken
    When the column "age" of the source table "people" becomes text
    And the source table "people" gains the records
      | id | age     |
      | 3  | unknown |
    And the sync runs a pass
    Then the pass ends with the source exhausted
    And the stream's schema is broken, naming "age"
    And the stream's schema has the field "age" as Integer
    And the dead-letters hold 1 record

  Scenario: Accepting the new shape lets the next write capture it
    When the column "age" of the source table "people" becomes text
    And the source table "people" gains the records
      | id | age     |
      | 3  | unknown |
    And the sync runs a pass
    And the stream's new shape is accepted
    And the source table "people" gains the records
      | id | age  |
      | 4  | many |
    And the sync runs a pass
    Then the stream's schema has the field "age" as Str
    And the stream's schema is not broken
    And the dead-letters hold 1 record
