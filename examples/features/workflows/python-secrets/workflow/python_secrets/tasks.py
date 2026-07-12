"""Workflow secrets in Python.

`@cloaca.workflow_secrets("name", ...)` declares the secrets a workflow
requires (the Python parity of Rust's `#[workflow(secrets(...))]`). A run binds
each declared name to a concrete tenant secret with a `{"$secret": "..."}`
reference; the plaintext is resolved at execution through `context.secret(...)`
and is never written into the durable context.

    resolve_token -> send_notification
"""
from __future__ import annotations

import cloaca


@cloaca.workflow_secrets("api_token")
@cloaca.workflow_params(channel=(str, "#ops"))
@cloaca.task(id="resolve_token", dependencies=[])
def resolve_token(context):
    """what: Resolve the bound secret and prove it WITHOUT leaking it.

    why: Only non-sensitive derived facts (a boolean + the token length) go back
    into the durable context; the value stays out of history entirely.
    """
    token = context.secret_field("api_token", "token")
    context.set("token_resolved", True)
    context.set("token_len", len(token))
    return context


@cloaca.task(id="send_notification", dependencies=["resolve_token"])
def send_notification(context):
    """what: 'Send' the notification to the configured channel."""
    if not context.get("token_resolved"):
        raise ValueError("token was not resolved")
    channel = context.get("channel") or "#ops"
    context.set("notification", {"channel": channel, "sent": True})
    return context
