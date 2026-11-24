Feature: A credential never widens the grants
  The host adds a stored secret to the requests a connector's grants allow.
  The connector never holds the secret, and a request the grants refuse does
  not leave the host.

  Background:
    Given an API that serves the records at "/items"
      | id | name  |
      | 1  | Robin |
      | 2  | Sam   |
    And a stored secret "api" with the field "token" set to "tok-SECRET-1"
    And a Postgres destination table "items"
    And the sync copies the API's records, keyed on "id"

  Scenario: The API sees the secret as a bearer header
    Given the source reads the API with the secret "api" as a bearer token
    When the sync runs a pass
    Then the pass ends with the source exhausted
    And the API saw the header "Authorization" set to "Bearer tok-SECRET-1" on every request
    And the destination table "items" holds the records
      | id | name  |
      | 1  | Robin |
      | 2  | Sam   |
    And "tok-SECRET-1" appears in no saved context and no record

  Scenario: The API sees the secret as a query parameter
    Given the source reads the API with the secret "api" as the query parameter "api_key"
    When the sync runs a pass
    Then the pass ends with the source exhausted
    And the API saw the query parameter "api_key" set to "tok-SECRET-1"
    And "tok-SECRET-1" appears in no saved context and no record

  Scenario: A request the grants refuse never reaches the API
    Given the source reads the API with the secret "api" as a bearer token
    And the source's grants allow another port only
    When the sync runs a pass
    Then the pass fails
    And the API received no request
    And "tok-SECRET-1" appears in no saved context and no record

  Scenario: A secret that is not stored fails the pass and names it
    Given the source reads the API with the secret "missing" as a bearer token
    When the sync runs a pass
    Then the pass fails, naming "missing"
    And the API received no request

  Scenario: An OAuth2 token is minted once and reused
    Given the API grants the token "tok-MINTED" at "/oauth/token"
    And a stored secret "crm" with the fields
      | client_id     | client-1      |
      | client_secret | SECRET-client |
    And the source reads the API with an OAuth2 client-credentials token from the secret "crm", granted at "/oauth/token"
    When the sync runs a pass
    And the sync runs a pass
    Then the API saw the header "Authorization" set to "Bearer tok-MINTED" on every request
    And the API granted 1 token at "/oauth/token"
    And "SECRET-client" appears in no saved context and no record
