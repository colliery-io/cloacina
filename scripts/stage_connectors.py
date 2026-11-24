#!/usr/bin/env python3
"""Build cloacina's first-party Sync connectors and stage them as provider
packages (COLLIERY-I-0635 decision 14 / CLOACINA-T-1009).

    python3 scripts/stage_connectors.py OUT_DIR [--only postgres,rest]

Each connector in `connectors/` is built to a wasm32-wasip2 component and
written to OUT_DIR/<package>/ with the `package.toml` and `provider.json` the
provider loader reads. Point a server or agent at OUT_DIR with
`CLOACINA_PROVIDER_PATH`. The images run this in a build stage; locally,
`angreal connectors stage` does.

The config schema in `provider.json` is a placeholder: the catalog and the UI
read a compiled connector's real schema from its `spec()`, run where
connectors run.
"""

import argparse
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CONNECTORS = ROOT / "connectors"

# Each connector's members: (member name, role, sync modes).
MEMBERS = {
    "postgres": [
        ("source", "Source", ["FullRefresh", "Incremental", "Cdc"]),
        ("destination", "Destination", ["FullRefresh", "Incremental", "Cdc"]),
    ],
    "mssql": [("source", "Source", ["FullRefresh", "Incremental"])],
    "s3": [("source", "Source", ["FullRefresh", "Incremental"])],
    "snowflake": [
        ("source", "Source", ["FullRefresh", "Incremental"]),
        ("destination", "Destination", ["FullRefresh", "Incremental"]),
    ],
    "rest": [("source", "Source", ["FullRefresh", "Incremental"])],
    "rest-dest": [("destination", "Destination", ["FullRefresh", "Incremental"])],
}


def package_field(cargo_toml: Path, field: str) -> str:
    """A `[package]` field of a Cargo.toml."""
    in_package = False
    for line in cargo_toml.read_text().splitlines():
        s = line.strip()
        if s.startswith("["):
            in_package = s == "[package]"
            continue
        m = re.match(rf'{field}\s*=\s*"([^"]+)"', s)
        if in_package and m:
            return m.group(1)
    raise SystemExit(f"{cargo_toml}: no [package] {field}")


def stage(name: str, out: Path) -> str:
    crate = CONNECTORS / name
    package = package_field(crate / "Cargo.toml", "name")
    version = package_field(crate / "Cargo.toml", "version")
    print(f"building {package} {version}", flush=True)
    subprocess.run(
        ["cargo", "build", "--lib", "--release", "--locked", "--target", "wasm32-wasip2"],
        cwd=crate,
        check=True,
    )
    component = package.replace("-", "_") + ".wasm"
    built = crate / "target" / "wasm32-wasip2" / "release" / component
    dest = out / package
    if dest.exists():
        shutil.rmtree(dest)
    dest.mkdir(parents=True)
    shutil.copyfile(built, dest / component)
    (dest / "package.toml").write_text(
        f'[package]\nname = "{package}"\nversion = "{version}"\n'
        'interface = "connector"\ninterface_version = 1\nruntime = "wasm"\n\n'
        '[metadata]\ncategory = "constructor"\n\n'
        f'[wasm]\ncomponent = "{component}"\n'
    )
    manifest = {
        "name": package,
        "version": version,
        "component": component,
        "runtime": "wasm",
        "constructors": [
            {
                "name": member,
                "version": version,
                "primitive_kind": member,
                "interface": "connector",
                "interface_version": 1,
                "params": [],
                "config_fields": [{"name": "json", "ty": "String"}],
                "dependencies": [],
                "connector": {
                    "roles": [role],
                    "supported_sync_modes": modes,
                    "config_schema": '{"type":"object"}',
                },
            }
            for member, role, modes in MEMBERS[name]
        ],
    }
    (dest / "provider.json").write_text(json.dumps(manifest, indent=2) + "\n")
    return package


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("out", type=Path, help="the provider directory to stage into")
    parser.add_argument("--only", help="a comma-separated subset, e.g. postgres,rest")
    args = parser.parse_args()
    names = list(MEMBERS)
    if args.only:
        wanted = [n.strip() for n in args.only.split(",") if n.strip()]
        unknown = [n for n in wanted if n not in MEMBERS]
        if unknown:
            raise SystemExit(f"unknown connectors: {', '.join(unknown)}")
        names = wanted
    args.out.mkdir(parents=True, exist_ok=True)
    staged = [stage(n, args.out) for n in names]
    print(f"staged {len(staged)} connectors in {args.out}: {', '.join(staged)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
