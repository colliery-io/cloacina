# cloacina-agent Helm chart

Deploys a fleet of [execution agents](https://cloacina.dev/platform/explanation/execution-agent-fleet/)
(`cloacina-agent`) — DB-less workers that register with a `cloacina-server`,
fetch compiled workflow cdylibs, execute tasks, and report results.

Agents hold **no** database connection and expose **no** HTTP surface, so they
can run outside the database trust zone. Whether work reaches the fleet is set
on the *server* via its default executor (`[server].default_executor = "fleet"`
in config.toml, or `CLOACINA_DEFAULT_EXECUTOR=fleet`); this chart just runs the
workers.

## Quick start

```bash
helm install fleet charts/cloacina-agent \
  --set server.url=http://my-release-cloacina-server:8080 \
  --set apiKeySecretRef.name=cloacina-agent-key   # a Secret you manage (key: api-key)
```

For dev you can inline the key instead (renders a Secret):

```bash
helm install fleet charts/cloacina-agent \
  --set server.url=http://my-release-cloacina-server:8080 \
  --set apiKey=sk-dev-key
```

## Required values

| Value | Description |
|---|---|
| `server.url` | Base URL of the cloacina-server to register with. |
| `apiKey` *or* `apiKeySecretRef.name` | API key (its tenant scope decides which tenants' work the agent receives). |

## Common values

| Value | Default | Description |
|---|---|---|
| `replicaCount` | `2` | Number of agents. |
| `server.maxConcurrency` | `4` | Concurrent work packets per agent. |
| `server.capabilities` | `[]` | Capability tags advertised at registration. |
| `image.repository` / `image.tag` | ghcr image / appVersion | Agent image. |

See `values.yaml` for the full set (resources, security context, scheduling).

## Notes

- The image must match the **build profile** of the workflow packages it runs
  (release agents need release-built cdylibs — fidius uses bincode in release,
  JSON in debug).
- Losing an agent mid-task is safe: the server reclaims its in-flight work onto
  a surviving agent within the liveness window.
