---
title: "The Corpus"
description: "The declarative connector definitions that ship with cloacina: each one's auth, config and streams."
sidebar:
  order: 70
---

The connector definitions embedded in `cloacina-declarative` (the corpus,
CLOACINA-T-0992). A sync names one with `source_definition` or
`destination_definition`. Each was written from its API's own documentation
(Apache-2.0); see `crates/cloacina-declarative/corpus/README.md`.

Every definition imports and every stream compiles, checked on each build.
**Reading a live API is not checked**: the entries marked *live-read* last
passed a read against their real API, under weir, on 2026-08-30. The others
have not been run against their API.

## Sources

| Name | Auth (secret field) | Config | Streams | Incremental | Live-read |
|---|---|---|---|---|---|
| `airtable` | `bearer` (field api_key) | `base_id`, `table_name` | records | no |  |
| `asana` | `bearer` (field api_token) |  | tasks, projects | yes |  |
| `coingecko` | `api_key` (header x-cg-pro-api-key, field api_key) |  | coins_list, coins_markets | no |  |
| `coinpaprika` | `none` |  | coins, tickers | no | yes |
| `exchangerate` | `api_key` (header apikey, field api_key) |  | latest, historical | no |  |
| `frankfurter` | `none` |  | latest, historical, currencies, providers | yes | yes |
| `github` | `bearer` (field api_key) |  | repositories, issues | yes |  |
| `gitlab` | `bearer` (field access_token) | `base_url` | projects, issues | yes |  |
| `google-analytics` | `google_service_account` (field service_account_key) | `property_id`, `start_date` | traffic, pages, events | yes |  |
| `google-sheets` | `google_service_account` (field service_account_key) | `spreadsheet_id`, `tab` | rows | no |  |
| `hubspot` | `bearer` (field access_token) |  | contacts, companies, deals | no |  |
| `intercom` | `bearer` (field api_key) |  | contacts, conversations | no |  |
| `jira` | `bearer` (field api_token) | `domain` | issues, projects | no |  |
| `jsonplaceholder` | `none` |  | posts, comments, users | no | yes |
| `mediawiki` | `none` | `search_query`, `wiki_base_url` | search, pages | no |  |
| `nasa` | `api_key_query` (param api_key, field api_key) |  | apod, neo | yes |  |
| `newsapi` | `api_key` (header X-Api-Key, field api_key) |  | everything, top_headlines | yes |  |
| `notion` | `bearer` (field api_token) |  | databases, pages | no |  |
| `nps` | `api_key` (header X-Api-Key, field api_key) |  | parks, alerts | no |  |
| `omdb` | `api_key` (header INVALID_PLACEHOLDER_SEE_OPEN_QUESTION_1, field api_key) |  | search | no |  |
| `openaq` | `api_key` (header X-API-Key, field api_key) | `sensors_id` | locations, measurements | yes |  |
| `openfda` | `none` |  | drug_event | no |  |
| `openlibrary` | `none` |  | search | no |  |
| `openweather` | `api_key` (header X-Api-Key, field api_key) |  | weather, forecast | no |  |
| `pagerduty` | `api_key` (header Authorization, field api_key) |  | incidents, services | yes |  |
| `pokeapi` | `none` |  | pokemon, ability, type | no |  |
| `rickandmorty` | `none` |  | character, location, episode | no | yes |
| `slack` | `bearer` (field api_token) |  | conversations, users | no |  |
| `spotify` | `bearer` (field access_token) |  | new_releases, categories | no |  |
| `square` | `bearer` (field api_key) |  | payments, customers | yes |  |
| `stripe` | `bearer` (field api_key) |  | customers, charges | no |  |
| `ticketmaster` | `api_key_query` (param apikey, field api_key) |  | events, venues | no |  |
| `tmdb` | `bearer` (field access_token) |  | movie_popular, tv_popular | no |  |
| `todoist` | `bearer` (field api_key) |  | tasks, projects | no |  |
| `xkcd` | `none` |  | comics | no | yes |
| `zendesk` | `bearer` (field api_token) | `subdomain` | tickets, users | no |  |

## Destinations

| Name | Auth | Config | Objects |
|---|---|---|---|
| `hubspot-dest` | `bearer` (field token) | | contacts, upserted by `email` |
| `salesforce-dest` | `oauth2`, client credentials (client_id, client_secret) | `instance` | contacts, upserted by an external id |

