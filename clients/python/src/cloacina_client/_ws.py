# Copyright 2025-2026 Colliery Software
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Substrate delivery WebSocket consumer (``GET /v1/ws/delivery/{recipient}``).

Protocol reference: the WebSocket Protocol page of the docs site; JSON
Schemas under ``/schemas/ws/``. Delivery is at-least-once — this stream
dedups on row id and acks each frame after yielding it.
"""

from __future__ import annotations

import asyncio
import base64
import json
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, AsyncIterator
from urllib.parse import quote

import websockets

if TYPE_CHECKING:
    from ._client import AsyncClient

#: Wire-protocol version this SDK speaks (delivery envelope).
DELIVERY_PROTOCOL_VERSION = 1


@dataclass
class DeliveryPush:
    """One decoded delivery push (already acked by the stream)."""

    id: int
    kind: str
    recipient: str
    tenant_id: str | None
    payload: bytes

    def payload_json(self) -> Any:
        return json.loads(self.payload)


class ProtocolVersionError(Exception):
    """Server closed 4426 — it does not speak our protocol version.
    Reconnecting cannot help; upgrade cloacina-client."""


def _ws_base(server: str) -> str:
    if server.startswith("https://"):
        return "wss://" + server[len("https://") :]
    if server.startswith("http://"):
        return "ws://" + server[len("http://") :]
    raise ValueError(f"server must start with http:// or https:// (got {server})")


async def subscribe_delivery(
    client: "AsyncClient",
    recipient: str,
    *,
    reconnect: bool = True,
    reconnect_initial: float = 0.1,
    reconnect_max: float = 30.0,
) -> AsyncIterator[DeliveryPush]:
    """Subscribe to the delivery stream for ``recipient``.

    Yields each push exactly once per process (dedup on row id), acking
    after yield; reconnects with exponential backoff on abnormal closure.
    """
    base = _ws_base(client.server)
    seen: set[int] = set()
    backoff = reconnect_initial

    while True:
        # Tickets are single-use — mint a fresh one per connection.
        ticket = (await client.create_ws_ticket()).ticket
        url = f"{base}/v1/ws/delivery/{quote(recipient, safe='')}?token={quote(ticket, safe='')}"

        close_code: int | None = None
        try:
            async with websockets.connect(url) as socket:
                await socket.send(
                    json.dumps(
                        {
                            "type": "hello",
                            "protocol_version": DELIVERY_PROTOCOL_VERSION,
                            "since_id": None,
                        }
                    )
                )
                async for raw in socket:
                    try:
                        frame = json.loads(raw)
                    except (TypeError, ValueError):
                        continue  # tolerate unknown frames
                    if frame.get("type") != "push":
                        continue
                    push = DeliveryPush(
                        id=int(frame["id"]),
                        kind=frame["kind"],
                        recipient=frame["recipient"],
                        tenant_id=frame.get("tenant_id"),
                        payload=base64.b64decode(frame["payload_b64"]),
                    )
                    if push.id not in seen:
                        seen.add(push.id)
                        yield push
                    # Ack after yield: a consumer crash before processing
                    # leaves the row unacked → redelivered on reconnect.
                    await socket.send(
                        json.dumps(
                            {
                                "type": "ack",
                                "protocol_version": DELIVERY_PROTOCOL_VERSION,
                                "id": push.id,
                            }
                        )
                    )
                    backoff = reconnect_initial
        except websockets.ConnectionClosed as closed:
            close_code = closed.rcvd.code if closed.rcvd else None
        except OSError as e:
            if not reconnect:
                raise ConnectionError(f"delivery WS connect failed: {e}") from e

        # 4426 = unsupported protocol_version — reconnecting cannot help.
        if close_code == 4426:
            raise ProtocolVersionError(
                f"server rejected delivery protocol v{DELIVERY_PROTOCOL_VERSION}"
            )
        if not reconnect:
            return
        await asyncio.sleep(backoff)
        backoff = min(backoff * 2, reconnect_max)


async def follow_execution_events(
    client: "AsyncClient",
    execution_id: str,
    **options: Any,
) -> AsyncIterator[Any]:
    """Stream the JSON events of one workflow execution."""
    async for push in subscribe_delivery(
        client, f"exec_events:{execution_id}", **options
    ):
        yield push.payload_json()
