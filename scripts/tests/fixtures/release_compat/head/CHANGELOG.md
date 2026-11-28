# Changelog

## [Unreleased]

### Changed (breaking)

- Things can no longer be deleted (`DELETE /v1/things/{id}`).
- A new thing must name its colour (`NewThing.colour`).
- Agents speak protocol 2 (`AGENT_PROTOCOL_VERSION`).

### Upgrade notes

- Migration `002` adds the state of a thing.

## [0.13.0] - 2026-12-01

### Changed

- The fixture release.
