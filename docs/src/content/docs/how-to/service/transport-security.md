---
title: "Transport Security"
description: "What is encrypted in transit inside a Kubernetes install, what a service mesh or a gateway must do, the chart's network policies, and the agent's --ca-file"
sidebar:
  order: 59
---

`cloacina-server` listens on plain HTTP only. It has no TLS listener and
takes no certificate. TLS between the clients, the agents and the server
ends in the cluster: in a service mesh with mTLS, or in a gateway or an
ingress in front of the server (CLOACINA-T-1069). An install with neither
sends API keys and task data in clear text inside the cluster.

## What is encrypted in transit

| Connection | Protocol | Encrypted by |
|---|---|---|
| Browser, `cloacinactl`, SDKs → server, from outside the cluster | HTTP, WebSocket | the ingress or gateway (`ingress.tls` of the chart), from the client to the ingress |
| Agent → server: register, heartbeat, results, package downloads, the delivery WebSocket | HTTP, WebSocket | a mesh, or a gateway the agents reach by `https://` and `wss://` |
| Server → server: a reactor inject forwarded to the replica that owns the reactor (`reactorAffinity.enabled`) | HTTP to the pod IP | a mesh only: the call goes to a pod IP, not through a gateway |
| Server, compiler → Postgres | Postgres | what the database URL sets; the chart's bundled Postgres is not set up for TLS |
| Server → identity provider, Kubernetes API | HTTPS | the server's TLS client |
| Server → alert webhook | HTTP or HTTPS, as its URL says | the server's TLS client, for an `https://` URL |

Every authenticated request carries a bearer key: agent keys, minted login
keys and the bootstrap key all cross these connections.

## With a service mesh

The mesh must:

- give mTLS to the server pods, the agent pods in the tenant namespaces
  (`cloacina-tenant-<tenant>`) and the compiler pods;
- cover traffic sent to a pod IP between server pods, for the reactor
  forwarding above;
- keep the API port 8080 the port the agents call: the agent NetworkPolicy
  allows egress to the server pods on that port only.

The server makes the tenant namespaces and the agent pods itself. For a
mesh that injects by namespace label, label each tenant namespace; the
server's apply keeps labels it did not write. For a mesh that injects by pod
annotation or label, set them on the agent pool (`pod_annotations`,
`pod_labels` of the pool settings).

## With a gateway

The agents reach the server through the gateway: set
`fleet.agentServerUrl` to its `https://` address, for example
`https://cloacina.internal.example.com`. The agent speaks `https://` and
`wss://` with that URL. The gateway must:

- forward HTTP and WebSocket upgrades to the server Service on port 8080;
- be listed in `server.networkPolicy.from`, or the server's NetworkPolicy
  refuses it.

A gateway does not cover the server-to-server forwarding. With
`reactorAffinity.enabled` and more than one replica, that call stays plain
HTTP between the server pods.

### A private CA: `--ca-file`

The agent trusts the system roots of its image. For a gateway whose
certificate a private CA signs, give the agent the CA:

- `--ca-file <path>`, or `CLOACINA_AGENT_CA_FILE`: a PEM file of one or more
  CA certificates. The agent's HTTP client and its WebSocket connector trust
  them in addition to the system roots.
- A file that cannot be read, is not PEM, or holds no certificate stops the
  agent at start, with the path in the error.
- The `cloacina-agent` chart mounts the file from a ConfigMap or a Secret:
  `server.ca.configMap` or `server.ca.secret`, with the key in
  `server.ca.key` (`ca.crt`).

## The chart's network policies

The `cloacina-server` chart closes its ports by default. The policies need a
CNI that enforces NetworkPolicy; on a CNI that does not, they change
nothing. Traffic from a pod's own node is allowed whatever the policy, so
the kubelet probes pass, and `kubectl port-forward` is not affected.

| Port | Open to by default | Value to open it | Value to turn it off |
|---|---|---|---|
| Server API, 8080 | the server pods; with `fleet.actuator=kubernetes`, the pods labelled `cloacina.managed=true` in namespaces labelled `cloacina.managed=true` (the agents) | `server.networkPolicy.from` | `server.networkPolicy.enabled=false` |
| Server metrics, 9090 | nobody | `metrics.networkPolicy.from` | `metrics.networkPolicy.enabled=false` |
| Chart compiler, 9000 (`/health`, `/v1/status`) | nobody | `compiler.networkPolicy.from` | `server.networkPolicy.enabled=false` |
| Bundled Postgres, 5432 | the server pods and the chart's compiler; with `fleet.actuator=kubernetes` and `fleet.compiler.enabled`, the tenant compilers | `postgresql.networkPolicy.from` | `postgresql.networkPolicy.enabled=false` |

Each `from` is a list of NetworkPolicy peers. What goes in
`server.networkPolicy.from`:

- the ingress controller or gateway: the UI and `cloacinactl` from outside
  the cluster come through it;
- agents you run yourself (`fleet.externalAgents=true`, the
  `cloacina-agent` chart);
- tools in the cluster that call the API.

```yaml
server:
  networkPolicy:
    from:
      - namespaceSelector:
          matchLabels:
            kubernetes.io/metadata.name: ingress-nginx
      - namespaceSelector:
          matchLabels:
            kubernetes.io/metadata.name: cloacina-agents
```

The policies cover ingress. The egress of the server pods (the database,
the identity provider, webhooks, the Kubernetes API) is not limited by the
chart. The agent and tenant compiler pods have egress policies of their
own, written by the server: see
[Deploying to Kubernetes](../deploying-to-kubernetes/#hardened-agent-pods--networkpolicy).

### Bundled Postgres password

With `postgresql.auth.password` empty (the default) the chart generates a
32-character password at install and keeps it in the Secret
`<release>-postgresql`, key `POSTGRES_PASSWORD`. Each `helm upgrade` reads
it back from that Secret. The server and the compiler take it from the
Secret; it is not in their manifests. A password you set is used as it is;
it goes into a URL, so use letters and digits. Postgres reads the password
at its first start only: to change it later, `ALTER ROLE` in the database
and set the same value in `postgresql.auth.password`.

`helm template` and `helm install --dry-run` cannot read the Secret of a
release, so they show a new value.

## The agent NetworkPolicy and the server's start

With `fleet.actuator=kubernetes` and `fleet.networkPolicy.enabled` (the
default), the server writes a NetworkPolicy in each tenant namespace. It
checks at start that it can, and does not start when it cannot:

- `CLOACINA_SERVER_NAMESPACE` and `CLOACINA_SERVER_POD_SELECTOR` must be
  set. The chart sets them.
- Its ServiceAccount must be allowed `create`, `patch` and `delete` on
  `networkpolicies` (`networking.k8s.io`) in every namespace. The chart's
  ClusterRole grants them. The server asks with a
  `SelfSubjectAccessReview`, which every authenticated identity may create
  under the Kubernetes default role `system:basic-user`.

The log line names the missing variables or verbs. To run without the
agent policy, set `fleet.networkPolicy.enabled=false`.

## Upgrading

See the upgrade note in the
[cloacina-server chart README](https://github.com/colliery-io/cloacina/tree/main/charts/cloacina-server#upgrade-note-the-api-port-and-bundled-postgres-are-closed).
In short, before the upgrade:

- list the ingress controller or gateway, agents you run yourself and
  in-cluster tools in `server.networkPolicy.from`;
- list other clients of bundled Postgres in `postgresql.networkPolicy.from`;
- or set `server.networkPolicy.enabled=false` and
  `postgresql.networkPolicy.enabled=false` to keep the ports open.
