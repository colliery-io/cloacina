-- Revert to using uuid_generate_v4() (requires uuid-ossp extension)
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

ALTER TABLE contexts
    ALTER COLUMN id SET DEFAULT uuid_generate_v4();
