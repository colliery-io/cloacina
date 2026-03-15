-- Accumulator state table for continuous scheduling persistence.
-- Stores per-edge consumer watermarks and drain metadata.
-- Persisted on drain, loaded on restart for watermark resume.

CREATE TABLE accumulator_state (
    edge_id VARCHAR(255) PRIMARY KEY,
    consumer_watermark TEXT CHECK (consumer_watermark IS NULL OR consumer_watermark::json IS NOT NULL),
    last_drain_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    drain_metadata TEXT DEFAULT '{}' CHECK (drain_metadata IS NULL OR drain_metadata::json IS NOT NULL),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE INDEX idx_accumulator_state_updated ON accumulator_state(updated_at DESC);
