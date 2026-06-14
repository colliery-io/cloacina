# @cloacina/ui

The Cloacina web UI — a tenant-scoped control plane (CLOACI-I-0117). A React + Vite + TypeScript SPA that consumes [`@cloacina/client`](../clients/typescript) as its only data layer.

> **Status:** app skeleton (T-0651). Feature views land in follow-on tasks (T-0652–T-0662).

## Stack

- **React 18 + Vite + TypeScript**
- **Mantine** (accessible component library)
- **TanStack Query** over `@cloacina/client`
- **React Router**
- Auth: API key + tenant in `sessionStorage`; OIDC "Login with…" is a later task (T-0662, gated on the server auth initiative I-0118)

## Develop

The UI links the TypeScript SDK locally (`file:../clients/typescript`). **Build the SDK first**, then install the UI:

```bash
# 1. build the SDK (once, or after SDK changes)
cd ../clients/typescript && npm install && npm run build

# 2. install + run the UI
cd ../../ui && npm install
npm run dev          # http://localhost:5173
```

On the connect screen, point it at a running `cloacina-server` (e.g. `http://localhost:8080`) with a tenant API key. The server must have CORS enabled for the UI origin:

```bash
cloacina-server … --cors-allowed-origins http://localhost:5173
```

## Scripts

| script | what |
|---|---|
| `npm run dev` | Vite dev server |
| `npm run build` | typecheck + production build |
| `npm run typecheck` | types only |
| `npm run lint` | ESLint |
| `npm run test` | Vitest (component/hook tests) |

## Layout

```
src/
  main.tsx            providers (Mantine, Query, Auth, Router)
  App.tsx             route map (IA)
  config.ts           runtime server-URL config (T-0659)
  auth/AuthContext    connection state, CloacinaClient, sessionStorage, gate
  api/
    errors.ts         CloacinaApiError → typed UI kind (REQ-007)
    queryClient.ts    TanStack Query client + retry policy
    hooks.ts          query-key factory + hook convention the feature tasks follow
  components/
    Shell             authenticated shell (nav + connection + disconnect)
    RequireAuth       route guard
    states/States     Loading / Empty / ErrorState primitives (NFR-001)
  routes/
    Connect           manual API-key gate
    Overview          landing (real rollup in T-0655)
    Placeholder       stubs for not-yet-built views
```
