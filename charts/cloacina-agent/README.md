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
| `apiKey` *or* `apiKeySecretRef.name` | API key of role `agent` (its tenant scope decides which tenants' work the agent receives). The server refuses an agent on a key of any other role. |

## Common values

| Value | Default | Description |
|---|---|---|
| `replicaCount` | `2` | Number of agents. |
| `server.maxConcurrency` | `4` | Concurrent work packets per agent. |
| `logFormat` | `text` | Format of the agent's log lines on stdout: `text` or `json` (`--log-format`). |
| `server.capabilities` | `[]` | Capability tags advertised at registration. |
| `image.repository` / `image.tag` | ghcr image / appVersion | Agent image. |
| `automountServiceAccountToken` | `false` | Mount a service account token in the pod. The agent does not use one. |
| `server.ca.configMap` / `server.ca.secret` | `""` | A ConfigMap or a Secret (one of the two) holding a PEM CA file under `server.ca.key` (`ca.crt`). The chart mounts it and sets `CLOACINA_AGENT_CA_FILE`; the agent trusts the CA in addition to the system roots. |

See `values.yaml` for the full set (resources, security context, scheduling).

## TLS and the server's NetworkPolicy

The server listens on plain HTTP; TLS ends in the cluster, in a mesh or a
gateway in front of the server (see
[Transport Security](https://cloacina.dev/platform/how-to-guides/transport-security)).
With a gateway, set `server.url` to its `https://` address. When a private
CA signs the gateway's certificate, put the CA in a ConfigMap and name it:

```bash
kubectl create configmap cluster-ca --from-file=ca.crt=./ca.crt
helm install fleet charts/cloacina-agent \
  --set server.url=https://cloacina.internal.example.com \
  --set apiKeySecretRef.name=cloacina-agent-key \
  --set server.ca.configMap=cluster-ca
```

A CA file that cannot be read, or holds no certificate, stops the agent at
start.

The `cloacina-server` chart closes its API port to pods it does not know
(`server.networkPolicy`). Agents of this chart must be listed in the
server release's `server.networkPolicy.from`, for example by their
namespace:

```yaml
# values of the cloacina-server release
server:
  networkPolicy:
    from:
      - namespaceSelector:
          matchLabels:
            kubernetes.io/metadata.name: cloacina-agents
```

## Notes

- The image must match the **build profile** of the workflow packages it runs
  (release agents need release-built cdylibs — fidius uses bincode in release,
  JSON in debug).
- Losing an agent mid-task is safe: the server reclaims its in-flight work onto
  a surviving agent within the liveness window.
