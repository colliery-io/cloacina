/*
 *  Copyright 2025-2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

/**
 * Typed wrapper for the substrate delivery WebSocket
 * (`GET /v1/ws/delivery/{recipient}`) and the execution-events stream
 * built on it. Protocol reference: the WebSocket Protocol page of the
 * docs site; JSON Schemas under `/schemas/ws/`.
 *
 * Uses the platform `WebSocket` — native in browsers and in Node >= 21.
 * On Node 20, pass a `WebSocket` implementation (e.g. the `ws` package)
 * via {@link DeliverySubscribeOptions.webSocket}.
 */

import type { CloacinaClient } from "./client.js";

/** Wire-protocol version this SDK speaks (delivery envelope). */
export const DELIVERY_PROTOCOL_VERSION = 1;

export interface DeliveryWelcome {
  type: "welcome";
  protocol_version: number;
  max_known_id: number;
}

export interface DeliveryPush {
  type: "push";
  protocol_version: number;
  id: number;
  kind: string;
  recipient: string;
  tenant_id: string | null;
  payload_b64: string;
}

export type DeliveryServerMessage = DeliveryWelcome | DeliveryPush;

/** Minimal structural WebSocket type so `ws` instances are accepted. */
export interface WebSocketLike {
  send(data: string): void;
  close(code?: number, reason?: string): void;
  addEventListener(type: string, listener: (event: never) => void): void;
}

export type WebSocketConstructor = new (url: string) => WebSocketLike;

export interface DeliverySubscribeOptions {
  /** Stop the stream. */
  signal?: AbortSignal;
  /**
   * WebSocket constructor override. Defaults to `globalThis.WebSocket`
   * (browsers, Node >= 21). Required on Node 20 (pass `ws`).
   */
  webSocket?: WebSocketConstructor;
  /** Reconnect on abnormal closure (default true). */
  reconnect?: boolean;
  /** Initial reconnect backoff in ms (default 100, doubles to max). */
  reconnectInitialMs?: number;
  /** Max reconnect backoff in ms (default 30_000). */
  reconnectMaxMs?: number;
  /**
   * Max frames buffered ahead of the consumer before the connection is
   * closed and re-established once drained (default 1024). Delivery is
   * at-least-once server-side, so dropping the socket never loses rows —
   * unacked rows are re-pushed on reconnect.
   */
  highWaterMark?: number;
}

function decodeBase64(b64: string): Uint8Array {
  if (typeof atob === "function") {
    const bin = atob(b64);
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) {
      out[i] = bin.charCodeAt(i);
    }
    return out;
  }
  // Node without atob (very old) — Buffer fallback.
  return Uint8Array.from(Buffer.from(b64, "base64"));
}

/** Decode a push frame's payload bytes. */
export function decodePushPayload(push: DeliveryPush): Uint8Array {
  return decodeBase64(push.payload_b64);
}

/** Decode a push frame's payload as JSON (execution events are JSON). */
export function decodePushJson(push: DeliveryPush): unknown {
  return JSON.parse(new TextDecoder().decode(decodePushPayload(push)));
}

interface QueueWaiter {
  resolve: (value: IteratorResult<DeliveryPush>) => void;
  reject: (err: unknown) => void;
}

/**
 * Subscribe to the substrate delivery stream for a recipient.
 *
 * Yields every `push` frame exactly once per row id (per-process dedup),
 * acking each frame after it is yielded. Reconnects with exponential
 * backoff on abnormal closure; the server re-pushes unacked rows on
 * reconnect (at-least-once), and the dedup set suppresses replays.
 */
export async function* subscribeDelivery(
  client: CloacinaClient,
  recipient: string,
  options: DeliverySubscribeOptions = {},
): AsyncGenerator<DeliveryPush, void, void> {
  const WS: WebSocketConstructor =
    options.webSocket ??
    (globalThis as { WebSocket?: WebSocketConstructor }).WebSocket ??
    (() => {
      throw new Error(
        "no WebSocket implementation: pass options.webSocket (e.g. `ws`) on Node 20",
      );
    })();

  const reconnect = options.reconnect ?? true;
  const initialBackoff = options.reconnectInitialMs ?? 100;
  const maxBackoff = options.reconnectMaxMs ?? 30_000;
  const highWaterMark = options.highWaterMark ?? 1024;

  const wsBase = client.baseUrl.replace(/^http/, "ws");
  const seen = new Set<number>();

  let backoff = initialBackoff;
  let stopped = false;
  options.signal?.addEventListener("abort", () => {
    stopped = true;
  });

  while (!stopped) {
    // A ticket is single-use — mint a fresh one per connection attempt.
    const { ticket } = await client.createWsTicket();
    const url = `${wsBase}/v1/ws/delivery/${encodeURIComponent(recipient)}?token=${encodeURIComponent(ticket)}`;

    const queue: DeliveryPush[] = [];
    let waiter: QueueWaiter | null = null;
    let closed = false;
    let closeCode: number | undefined;
    let socketError: unknown = null;

    const socket = new WS(url);
    const abort = () => socket.close(1000, "client abort");
    options.signal?.addEventListener("abort", abort, { once: true } as never);

    socket.addEventListener("open", (() => {
      backoff = initialBackoff;
      socket.send(
        JSON.stringify({
          type: "hello",
          protocol_version: DELIVERY_PROTOCOL_VERSION,
          since_id: null,
        }),
      );
    }) as never);

    socket.addEventListener("message", ((event: { data: unknown }) => {
      let frame: DeliveryServerMessage;
      try {
        frame = JSON.parse(String(event.data)) as DeliveryServerMessage;
      } catch {
        return; // tolerate unknown frames
      }
      if (frame.type !== "push") {
        return; // welcome is informational
      }
      if (queue.length >= highWaterMark) {
        // Backpressure: shed the connection; unacked rows redeliver later.
        socket.close(1000, "client backpressure");
        return;
      }
      queue.push(frame);
      if (waiter !== null && queue.length > 0) {
        const w = waiter;
        waiter = null;
        w.resolve({ value: queue.shift() as DeliveryPush, done: false });
      }
    }) as never);

    socket.addEventListener("close", ((event: { code?: number }) => {
      closed = true;
      closeCode = event?.code;
      if (waiter !== null) {
        const w = waiter;
        waiter = null;
        w.resolve({ value: undefined, done: true });
      }
    }) as never);

    socket.addEventListener("error", ((event: unknown) => {
      socketError = event;
    }) as never);

    const nextFrame = (): Promise<IteratorResult<DeliveryPush>> => {
      if (queue.length > 0) {
        return Promise.resolve({
          value: queue.shift() as DeliveryPush,
          done: false,
        });
      }
      if (closed) {
        return Promise.resolve({ value: undefined, done: true });
      }
      return new Promise((resolve, reject) => {
        waiter = { resolve, reject };
      });
    };

    try {
      for (;;) {
        const result = await nextFrame();
        if (result.done) {
          break;
        }
        const push = result.value;
        if (!seen.has(push.id)) {
          seen.add(push.id);
          yield push;
        }
        // Ack after yield so a consumer crash before processing leaves the
        // row unacked → redelivered on the next connection.
        if (!closed) {
          socket.send(
            JSON.stringify({
              type: "ack",
              protocol_version: DELIVERY_PROTOCOL_VERSION,
              id: push.id,
            }),
          );
        }
      }
    } finally {
      if (!closed) {
        socket.close(1000, "consumer done");
      }
    }

    if (stopped || !reconnect) {
      return;
    }
    // 4426 = unsupported protocol_version: reconnecting cannot help.
    if (closeCode === 4426) {
      throw new Error(
        "server rejected delivery protocol_version " +
          `${DELIVERY_PROTOCOL_VERSION} (close 4426) — upgrade @cloacina/client`,
      );
    }
    if (socketError !== null && backoff >= maxBackoff) {
      throw new Error(`delivery WS failed repeatedly: ${String(socketError)}`);
    }
    await new Promise((r) => setTimeout(r, backoff));
    backoff = Math.min(backoff * 2, maxBackoff);
  }
}

/**
 * Stream the JSON events of one workflow execution
 * (recipient convention `exec_events:<execution_id>` — the same stream
 * `cloacinactl execution follow` renders).
 */
export async function* followExecutionEvents(
  client: CloacinaClient,
  executionId: string,
  options: DeliverySubscribeOptions = {},
): AsyncGenerator<unknown, void, void> {
  for await (const push of subscribeDelivery(
    client,
    `exec_events:${executionId}`,
    options,
  )) {
    yield decodePushJson(push);
  }
}
