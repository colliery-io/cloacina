-- Accumulator state table for continuous scheduling persistence.
CREATE TABLE accumulator_state (
    edge_id VARCHAR(255) PRIMARY KEY NOT NULL,
    consumer_watermark TEXT,
    last_drain_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    drain_metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
