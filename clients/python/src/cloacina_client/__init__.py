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

"""Python SDK for cloacina-server (CLOACI-I-0113 / T-0647).

This is the *service client* — it talks to a running cloacina-server over
HTTP/WebSocket. The embedded workflow runtime is the separate ``cloaca``
package; the two are deliberately distinct.

Quickstart::

    from cloacina_client import Client

    client = Client("http://localhost:8080", api_key="clk_...", tenant="public")
    accepted = client.execute_workflow("my_workflow", {"input": 42})

    import asyncio
    from cloacina_client import AsyncClient

    async def follow():
        aclient = AsyncClient("http://localhost:8080", api_key="clk_...")
        async for event in aclient.follow_execution_events(accepted.execution_id):
            print(event)

    asyncio.run(follow())
"""

from ._client import AsyncClient, Client, CloacinaApiError
from ._generated import models
from ._ws import DELIVERY_PROTOCOL_VERSION, DeliveryPush

__all__ = [
    "AsyncClient",
    "Client",
    "CloacinaApiError",
    "DeliveryPush",
    "DELIVERY_PROTOCOL_VERSION",
    "models",
]

__version__ = "0.7.0"
