-- Add migration script here
CREATE TABLE IF NOT EXISTS users (
    id VARCHAR(26) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id)  
);

CREATE TABLE IF NOT EXISTS refresh_tokens (
    jti VARCHAR(26) NOT NULL,
    exp TIMESTAMPTZ NOT NULL,
    iat TIMESTAMPTZ NOT NULL,
    owner_id VARCHAR(26) NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS UQ_refresh_tokens_owner ON refresh_tokens (owner_id);