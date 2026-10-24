# cloacina-declarative

Declarative connectors for cloacina's Sync. A declarative connector is a YAML
connector definition run on the shared REST runtime, the `rest` and
`rest-dest` guests.

- `definition`: the model, and the compiler that turns a definition, a stream
  and a user's config into the runtime's config, the mapping, the typed schema
  and the credential spec.
- `importer`: Airbyte low-code sources to connector definitions.
- `corpus`: the definitions that ship with cloacina, embedded.

The crate depends on nothing of the engine, so a CLI can import and check
definitions without a WASM runtime.
