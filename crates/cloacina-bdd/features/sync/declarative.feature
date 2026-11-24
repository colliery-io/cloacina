Feature: A declarative sync
  A sync side can name a connector definition instead of a provider: one of
  the corpus, or one from the server's definitions directory. The definition
  supplies the REST runtime's config, its grants and its auth; the sync gives
  the user config and the stored secret.

  Scenario: A corpus source syncs to Postgres with a stored secret, then reads only what is new
    Given the GitHub API serves these repositories to the bearer token "ghp-SECRET", each with every field the definition declares
      | id | updated_at           |
      | 1  | 2026-01-01T00:00:00Z |
      | 2  | 2026-01-02T00:00:00Z |
      | 3  | 2026-01-03T00:00:00Z |
    And a stored secret "github-token" with the field "api_key" set to "ghp-SECRET"
    And the source is the connector definition "github", at the API's address, with the secret "github-token"
    And a Postgres destination table "repos"
    And the sync reads the stream "repositories" by the cursor "updated_at", keyed on "id"
    When the sync is created
    Then the sync is accepted
    When the sync runs a pass
    Then the pass ends with the source exhausted
    And the destination table "repos" holds the records
      | id |
      | 1  |
      | 2  |
      | 3  |
    And the committed cursor is the time "2026-01-03T00:00:00Z"
    When the GitHub API also serves these repositories to the bearer token "ghp-SECRET"
      | id | updated_at           |
      | 4  | 2026-01-04T00:00:00Z |
      | 5  | 2026-01-05T00:00:00Z |
    And the sync runs a pass
    Then the GitHub API was last asked for repositories since "2026-01-03T00:00:00Z"
    And the destination table "repos" holds the records
      | id |
      | 1  |
      | 2  |
      | 3  |
      | 4  |
      | 5  |
    And the API saw the header "Authorization" set to "Bearer ghp-SECRET" on every request
    And "ghp-SECRET" appears in no saved context and no record

  Scenario: A definition that does not exist is refused when the sync is created
    Given the source is the connector definition "githubb", at the API's address, with the secret "github-token"
    And a Postgres destination table "repos"
    And the sync reads the stream "repositories" by the cursor "updated_at", keyed on "id"
    When the sync is created
    Then the sync is refused, naming "githubb"

  Scenario: A stream the definition does not have is refused, naming the streams it has
    Given the source is the connector definition "github", at the API's address, with the secret "github-token"
    And a Postgres destination table "repos"
    And the sync reads the stream "repos" by the cursor "updated_at", keyed on "id"
    When the sync is created
    Then the sync is refused, naming "repositories"

  Scenario: A definition from the definitions directory syncs like one of the corpus
    Given an API that serves the records at "/notes"
      | id | body  |
      | 1  | hello |
      | 2  | world |
    And a stored secret "notes-key" with the field "api_token" set to "nk-SECRET"
    And the source is the connector definition "notes", at the API's address, with the secret "notes-key"
    And a Postgres destination table "notes"
    And the sync reads the whole stream "notes", keyed on "id"
    When the sync is created
    Then the sync is accepted
    When the sync runs a pass
    Then the pass ends with the source exhausted
    And the destination table "notes" holds the records
      | id | body  |
      | 1  | hello |
      | 2  | world |
    And the API saw the header "Authorization" set to "Bearer nk-SECRET" on every request
