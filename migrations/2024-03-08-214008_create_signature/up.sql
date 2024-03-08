-- Your SQL goes here

CREATE TABLE signatures (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    first_name VARCHAR NOT NULL,
    last_name VARCHAR NOT NULL,
    org VARCHAR,
    email VARCHAR NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    verified BOOLEAN NOT NULL DEFAULT false,
    verified_at TIMESTAMP
);
