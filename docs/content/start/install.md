---
title: "Installing cloacinactl"
description: "One-line install for the cloacinactl CLI (and bundled daemon subcommand)."
weight: 15
---

# Installing `cloacinactl`

`cloacinactl` is the operator + developer CLI for Cloacina. It bundles
the daemon as a subcommand (`cloacinactl daemon`), so a single install
gives you both.

## One-liner (Linux + macOS)

```sh
curl -fsSL https://get.cloacina.dev/install.sh | bash
```

That downloads the latest release tarball for your OS + arch from
[GitHub Releases](https://github.com/colliery-io/cloacina/releases),
verifies its SHA256, and installs the binary to `~/.cloacina/bin`. If
that directory isn't on your `$PATH`, the installer prints the line to
add to your shell rc.

### Pinning a version

```sh
curl -fsSL https://get.cloacina.dev/install.sh | bash -s -- --version v0.7.0
```

Use a specific tag from the releases page. Any release that has the
binary uploaded for your OS/arch will work.

### System-wide install

```sh
curl -fsSL https://get.cloacina.dev/install.sh | bash -s -- --prefix /usr/local
```

Installs to `/usr/local/bin`. Uses `sudo` if the directory isn't writable
by the current user.

### Other knobs

| Flag | Default | Notes |
|------|---------|-------|
| `--version vX.Y.Z` | latest release | Pin to a specific tag |
| `--prefix DIR`     | `$HOME/.cloacina` | Install root; binary lands in `DIR/bin` |
| `--quiet`          | off | Suppress informational output |

Set `CLOACINA_REPO=owner/repo` to install from a fork.

## Supported platforms

Cross-compiled binaries are uploaded on every release tag for:

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`

Windows is not currently supported. If you need it, file an issue.

## Verifying the install

```sh
cloacinactl --version
cloacinactl daemon --help
```

The `--version` output should match the release tag you installed.

## Upgrading

Re-run the same one-liner. The installer is idempotent — it overwrites
the binary in place. To pin a new version, pass `--version`.

## Uninstalling

```sh
rm ~/.cloacina/bin/cloacinactl
# or for a system-wide install:
sudo rm /usr/local/bin/cloacinactl
```

Then remove the PATH-add line from your shell rc if you added one.

## Building from source

If your platform isn't covered above, install with cargo:

```sh
cargo install --git https://github.com/colliery-io/cloacina cloacinactl
```

This builds from the `main` branch tip. Pass `--tag vX.Y.Z` to pin a release.

## Python users

The Python bindings ship as a wheel — no separate installer needed:

```sh
pip install cloaca               # default (both backends)
pip install cloaca[sqlite]       # SQLite only
pip install cloaca[postgres]     # PostgreSQL only
```

See the [Python quick start]({{< ref "/python" >}}) for usage.

## Docker

The server is published as a container image on every release:

```sh
docker pull ghcr.io/colliery-io/cloacina-server:v0.7.0
```

See [Running the server image]({{< ref "/service/how-to/running-the-server-image" >}}) for the full container deploy walkthrough — environment variables, signature enforcement, log retention.

## Kubernetes (Helm)

A Helm chart with an embedded local Postgres subchart ships in-tree:

```sh
helm install cloacina-server ./charts/cloacina-server
```

See [Deploying to Kubernetes]({{< ref "/service/how-to/deploying-to-kubernetes" >}}) for production values, the embedded Postgres subchart story, and the operator-flag reference.
