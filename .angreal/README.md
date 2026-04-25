# Angreal task harness

All project automation — build, test, lint, demos, services, CI — lives under
this directory and is exposed through the `angreal` CLI. Run `angreal tree`
for the full command surface.

## Layout

```
.angreal/
├── task_*.py          # root registrars; angreal auto-loads these by regex
├── test/              # `angreal test …`     — unit, integration, macros,
│   ├── e2e/                                    auth, all, coverage,
│   └── soak/                                   metrics-format, + nested
│                                               `e2e {cli,compiler,ws}` and
│                                               `soak {daemon,server}`
├── lint/              # `angreal lint …`     — fmt, clippy,
│                                               credential-logging, all
├── ci/                # `angreal ci …`       — fast, full
├── demos/
│   ├── tutorials/
│   │   ├── rust.py                          # `angreal demos tutorials rust NN`
│   │   └── python.py                        # `angreal demos tutorials python NN`
│   └── features/                            # `angreal demos features <name>`
├── performance.py     # `angreal performance …`
├── task_check.py      # `angreal check …`     — static cargo checks
├── task_services.py   # `angreal services …`  — up/down/reset/clean/purge
├── task_docs.py       # `angreal docs …`
├── task_project.py    # single registrar; imports every subpackage
├── utils.py           # shared helpers (docker, example runners)
└── docker-compose.yaml
```

**Loader convention:** angreal auto-discovers files matching `task_*.py` at
the root of `.angreal/`. Files inside subpackages (like `.angreal/test/unit.py`)
are loaded via normal Python imports from `task_project.py`, so subpackage
files do *not* need the `task_` prefix. Prefer subpackages/namespaces for
grouping related commands; use root `task_*.py` only as a thin registrar.

## Flag conventions

Consistency makes the harness easier to remember.

| Pattern            | Meaning                                                |
| ------------------ | ------------------------------------------------------ |
| `--skip-<thing>`   | Opt-out of an otherwise-default step                   |
| `--no-<thing>`     | Disable a feature (e.g. `--no-warnings`)               |
| `--<thing>`        | Opt-in flag (e.g. `--check`, `--html`)                 |
| `-v` / `--verbose` | Increase verbosity (reserved — use angreal's own `-v`) |
| `<FILTER>` (pos.)  | Optional substring filter for tests where it applies   |
| `--backend`        | `postgres` or `sqlite` on integration/tutorial tasks   |

Short flags (`-i`, `-c`) should be reserved for tasks where they are
genuinely common, and always come with a matching long form.

## Risk levels

Tasks that destroy state (remove docker volumes, wipe `target/`, scrub
artifacts) carry `tool=angreal.ToolDescription(..., risk_level="destructive")`
so AI agents and automation can gate on it.

Currently marked destructive:

- `services purge`   — wipes docker volumes, all `target/` dirs, Python venvs
- `services clean`   — wipes docker volumes + `target/` dirs
- `services reset`   — restart; with `--clean`, removes volumes
- `demos tutorials python <NN>` — with `--backend postgres`, removes volumes
  from the shared compose stack during cleanup

Test tasks that bring up docker for the duration of a run and tear it down
on exit are not marked destructive — that is their documented lifecycle.

## Adding a command

1. Put it in the right subpackage (create one if the group doesn't exist).
2. Register it under its command group(s):

   ```python
   import angreal

   test = angreal.command_group(name="test", about="...")
   e2e  = angreal.command_group(name="e2e",  about="...")

   @test()
   @e2e()
   @angreal.command(name="new-thing", about="...", when_to_use=[...], when_not_to_use=[...])
   def new_thing():
       ...
   ```

   Decorator nearest `def` = innermost group. Stacked groups produce
   `angreal test e2e new-thing`.
3. If the task destroys state, pass
   `tool=angreal.ToolDescription("...", risk_level="destructive")`.
4. Verify with `angreal tree` and `angreal <your> --help`.
