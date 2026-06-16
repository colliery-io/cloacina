/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 */

// Live boundary-data producer for the demo computation graphs (CLOACI-I-0124 /
// WS-11). Without a data source the demo's CGs sit idle ("socket only", never
// firing). This pushes a steady stream of market-data events so the reactors
// fire and the UI shows live throughput / executions:
//
//   - socket accumulators `orderbook` + `pricing` (feed market_pipeline +
//     market_maker) — pushed over the server's WebSocket producer endpoint
//     `/v1/ws/accumulator/{name}` (authenticated with a short-lived ws-ticket).
//   - Kafka stream accumulator `kafka_alpha` (feeds demo_kafka_graph) — produced
//     to the `demo.kafka.stream` topic, only when a Kafka broker is configured
//     (the docker-compose demo). Skipped on the host `ui up` stack (no broker).
//
// Stack-agnostic: it only needs a server URL + key; Kafka auto-skips when
// `HARNESS_KAFKA_BROKER` is unset or unreachable.

import WebSocket from "ws";

const log = (...a) => console.log("[produce]", ...a);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// --- Boundary-event generators (match the fixtures' boundary structs) --------
// market_pipeline / market_maker: OrderBook{best_bid,best_ask}, Pricing{mid_price}
function orderbookEvent(t) {
  const mid = 100 + Math.sin(t / 5) * 2;
  const spread = 0.08 + (t % 9) * 0.02; // varies so routing flips Trade/NoAction
  return {
    best_bid: Number((mid - spread / 2).toFixed(4)),
    best_ask: Number((mid + spread / 2).toFixed(4)),
  };
}
function pricingEvent(t) {
  return { mid_price: Number((100 + Math.cos(t / 6) * 2).toFixed(4)) };
}
// demo_kafka_graph: EventData{value}
function kafkaEvent(t) {
  return { value: Number((50 + Math.sin(t / 4) * 10).toFixed(4)) };
}

// Generators per known socket accumulator; unknown names get a generic {value}.
const GENERATORS = { orderbook: orderbookEvent, pricing: pricingEvent };

/** Socket accumulators to feed over WS, from `cfg.wsAccumulators` (a comma list;
 *  default orderbook,pricing). Empty list → Kafka-only (e.g. the compose demo,
 *  which has no socket accumulators — avoids 403 reconnect spam). */
function socketFeeds(cfg) {
  return (cfg.wsAccumulators ?? [])
    .map((s) => s.trim())
    .filter(Boolean)
    .map((accumulator) => ({
      accumulator,
      gen: GENERATORS[accumulator] ?? ((t) => ({ value: Number((t % 100).toFixed(4)) })),
    }));
}

async function fetchTicket(serverUrl, apiKey) {
  const res = await fetch(`${serverUrl}/v1/auth/ws-ticket`, {
    method: "POST",
    headers: { Authorization: `Bearer ${apiKey}` },
  });
  if (!res.ok) throw new Error(`ws-ticket request failed: ${res.status}`);
  const body = await res.json();
  if (!body.ticket) throw new Error("ws-ticket response had no ticket");
  return body.ticket;
}

function wsUrl(serverUrl, accumulator, ticket) {
  const base = serverUrl.replace(/^http/, "ws").replace(/\/+$/, "");
  return `${base}/v1/ws/accumulator/${encodeURIComponent(accumulator)}?token=${encodeURIComponent(ticket)}`;
}

/** One resilient WS feed: connects, pushes `gen(seq)` every interval, and
 *  reconnects (with a fresh ticket) on close/error. Runs until `state.stop`. */
async function runSocketFeed({ accumulator, gen }, cfg, state) {
  let seq = 0;
  while (!state.stop) {
    let ws;
    try {
      const ticket = await fetchTicket(cfg.serverUrl, cfg.apiKey);
      ws = new WebSocket(wsUrl(cfg.serverUrl, accumulator, ticket));
      await new Promise((resolve, reject) => {
        ws.once("open", resolve);
        ws.once("error", reject);
      });
      log(`ws ${accumulator}: connected`);
      while (!state.stop && ws.readyState === WebSocket.OPEN) {
        // The accumulator endpoint accepts BINARY frames only (it forwards the
        // bytes straight to the accumulator's deserializer) — send a Buffer so
        // `ws` frames it as binary, not text.
        ws.send(Buffer.from(JSON.stringify(gen(seq))), { binary: true });
        seq += 1;
        await sleep(cfg.intervalMs);
      }
    } catch (err) {
      log(`ws ${accumulator}: ${err instanceof Error ? err.message : err} — reconnecting in 3s`);
      await sleep(3000);
    } finally {
      try {
        ws?.close();
      } catch {
        /* ignore */
      }
    }
  }
}

/** Kafka feed for `kafka_alpha` — only runs when a broker is configured. */
async function runKafkaFeed(cfg, state) {
  if (!cfg.kafkaBroker) {
    log("kafka: no HARNESS_KAFKA_BROKER set — skipping kafka_alpha feed (host stack)");
    return;
  }
  let Kafka;
  try {
    ({ Kafka } = await import("kafkajs"));
  } catch {
    log("kafka: kafkajs not installed — skipping");
    return;
  }
  const kafka = new Kafka({ clientId: "cloacina-demo-producer", brokers: [cfg.kafkaBroker] });
  const admin = kafka.admin();
  const producer = kafka.producer();
  try {
    await admin.connect();
    await admin.createTopics({ topics: [{ topic: cfg.kafkaTopic, numPartitions: 1 }] }).catch(() => {});
    await admin.disconnect();
    await producer.connect();
    log(`kafka: producing to '${cfg.kafkaTopic}' at ${cfg.kafkaBroker}`);
  } catch (err) {
    log(`kafka: connect failed (${err instanceof Error ? err.message : err}) — skipping`);
    return;
  }
  let seq = 0;
  while (!state.stop) {
    try {
      await producer.send({
        topic: cfg.kafkaTopic,
        messages: [{ value: JSON.stringify(kafkaEvent(seq)) }],
      });
      seq += 1;
    } catch (err) {
      log(`kafka: send failed — ${err instanceof Error ? err.message : err}`);
    }
    await sleep(cfg.intervalMs);
  }
  await producer.disconnect().catch(() => {});
}

/**
 * Run all data feeds concurrently until SIGINT/SIGTERM. Returns when stopped.
 */
export async function produce(cfg) {
  const state = { stop: false };
  const onStop = () => {
    state.stop = true;
  };
  process.on("SIGINT", onStop);
  process.on("SIGTERM", onStop);

  const feeds = socketFeeds(cfg);
  log(
    `feeding [${feeds.map((f) => f.accumulator).join(", ") || "none"}] over WS every ${cfg.intervalMs}ms` +
      (cfg.kafkaBroker ? ` + kafka '${cfg.kafkaTopic}'` : " (no kafka)"),
  );

  await Promise.all([
    ...feeds.map((f) => runSocketFeed(f, cfg, state)),
    runKafkaFeed(cfg, state),
  ]);
  log("producer stopped");
}
