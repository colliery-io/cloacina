"""Demo Python constructor workflow (CLOACI-T-0831).

The Python twin of demo-constructor-rust: the first DAG node is
cloacina-provider-fs's `read_file` member, resolved from the provider BUNDLED
by the compiler (declared in this package's `[metadata.providers]`) and
executed in a WASM sandbox that can reach ONLY the granted `/etc`
(default-closed otherwise). It reads `/etc/hostname` — a REGULAR file docker
bind-mounts into every container (NOT `/etc/os-release`, a symlink the sandbox
correctly refuses to follow out of the grant) — and the downstream Python task
summarizes what came through the sandbox.

    py_reader (wasm constructor) ─▶ py_summarize (python task)
"""
from __future__ import annotations

import cloaca

cloaca.constructor(
    id="py_reader",
    from_="cloacina-provider-fs@0.1.0",
    constructor="read_file",
    config={"path": "/etc/hostname"},
    grants={"fs": ["ro:/etc"]},
)


@cloaca.task(dependencies=["py_reader"])
def py_summarize(context):
    contents = context.get("contents") or ""
    context.set("py_sandbox_read_bytes", len(contents))
    context.set("py_sandbox_read_hostname", contents.strip())
    return context
