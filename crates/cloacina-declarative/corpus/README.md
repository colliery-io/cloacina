# The declarative corpus

The connector definitions that ship with cloacina, embedded in
`cloacina-declarative` and moved from weir (CLOACINA-T-0992).

- `sources/`: 36 sources in Airbyte's low-code format, imported when loaded:
  `airtable` `asana` `coingecko` `coinpaprika` `exchangerate` `frankfurter`
  `github` `gitlab` `google-analytics` `google-sheets` `hubspot` `intercom`
  `jira` `jsonplaceholder` `mediawiki` `nasa` `newsapi` `notion` `nps` `omdb`
  `openaq` `openfda` `openlibrary` `openweather` `pagerduty` `pokeapi`
  `rickandmorty` `slack` `spotify` `square` `stripe` `ticketmaster` `tmdb`
  `todoist` `xkcd` `zendesk`.
- `destinations/`: `hubspot-dest` and `salesforce-dest`, as cloacina
  destination definitions.

## Provenance

Every definition here was written **solely** from the target API's own
documentation; no third-party connector implementation was consulted. Each file
names the docs it was written from. License: Apache-2.0.

## Verification

Importing and compiling cleanly is checked on every build. Reading a live API is
not: `sources/verified.json` records the definitions that last passed a live
read, under weir, with the date. A definition not listed there has not been run
against its real API.

## Adding one

Add the file, then add it to the list in `src/corpus.rs`; a test fails when the
list and this folder disagree. Every stream must compile.
