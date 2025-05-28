CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE contexts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    value VARCHAR NOT NULL CHECK (value::json IS NOT NULL),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX contexts_created_at_idx ON contexts(created_at DESC);
