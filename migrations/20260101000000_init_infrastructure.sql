-- Infrastructure metadata table for the platform.
--
-- V0.1 deliberately creates no trading domain tables (strategies, orders, trades,
-- positions). This table exists only so that migrations have a concrete,
-- verifiable effect and so that operators can confirm the database schema is
-- managed by this repository.

CREATE TABLE IF NOT EXISTS platform_meta (
    key         TEXT        PRIMARY KEY,
    value       TEXT        NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE platform_meta IS
    'Infrastructure metadata managed by platform migrations. Not business data.';

INSERT INTO platform_meta (key, value)
VALUES ('schema_version', 'v0.1')
ON CONFLICT (key) DO UPDATE
SET value = EXCLUDED.value,
    updated_at = now();
