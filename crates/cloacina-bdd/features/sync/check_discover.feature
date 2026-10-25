Feature: Check and discover run where connectors run
  Before a sync is saved, its sides can be checked, and its source's streams
  discovered. Both run the connector where Sync's tasks run, with the stored
  secret, never in the server.

  Background:
    Given an API that serves the records at "/items" to the bearer token "tok-RIGHT"
      | id | name  |
      | 1  | Robin |

  Scenario: A check with the right secret passes
    Given a stored secret "api" with the field "token" set to "tok-RIGHT"
    When the API source is checked with the secret "api" as a bearer token
    Then the check passes

  Scenario: A check with a wrong secret fails, saying the API refused it
    Given a stored secret "api" with the field "token" set to "tok-WRONG"
    When the API source is checked with the secret "api" as a bearer token
    Then the check fails, saying "401"

  Scenario: A check with a secret that is not stored cannot run, and names it
    When the API source is checked with the secret "missing" as a bearer token
    Then the probe cannot run, naming "missing"

  Scenario: Discover lists a definition's streams with their fields
    When the source definition "github" is discovered
    Then the stream "repositories" is offered incrementally by "updated_at", with the field "id" as an integer

  Scenario: Discover lists a Postgres source's tables
    Given a Postgres source table "orders" with the records
      | id | name  |
      | 1  | Robin |
    When the Postgres source is discovered
    Then the source table "orders" is offered as a stream

  Scenario: A compiled connector describes its config
    When the Postgres connector is asked to describe itself
    Then its config has the field "table", and "password" is secret
