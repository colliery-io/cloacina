-- Computation graph resilience tables (I-0081 / T-0407)
-- Stores checkpoint, boundary, reactor, and state accumulator data
-- for restart recovery across all computation graph components.

-- Accumulator checkpoint state (poll counters, custom state, etc.)
CREATE TABLE accumulator_checkpoints (
    id BLOB PRIMARY KEY NOT NULL,
    graph_name TEXT NOT NULL,
    accumulator_name TEXT NOT NULL,
    checkpoint_data BLOB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_acc_checkpoints_graph_acc
    ON accumulator_checkpoints (graph_name, accumulator_name);

-- Last-emitted boundary per accumulator (for reactor self-seeding on restart)
CREATE TABLE accumulator_boundaries (
    id BLOB PRIMARY KEY NOT NULL,
    graph_name TEXT NOT NULL,
    accumulator_name TEXT NOT NULL,
    boundary_data BLOB NOT NULL,
    sequence_number INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_acc_boundaries_graph_acc
    ON accumulator_boundaries (graph_name, accumulator_name);

-- Reactor input cache and dirty flags snapshot
CREATE TABLE reactor_state (
    id BLOB PRIMARY KEY NOT NULL,
    graph_name TEXT NOT NULL,
    cache_data BLOB NOT NULL,
    dirty_flags BLOB NOT NULL,
    sequential_queue BLOB,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_reactor_state_graph
    ON reactor_state (graph_name);

-- State accumulator VecDeque buffer persistence
CREATE TABLE state_accumulator_buffers (
    id BLOB PRIMARY KEY NOT NULL,
    graph_name TEXT NOT NULL,
    accumulator_name TEXT NOT NULL,
    buffer_data BLOB NOT NULL,
    capacity INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_state_acc_buffers_graph_acc
    ON state_accumulator_buffers (graph_name, accumulator_name);
