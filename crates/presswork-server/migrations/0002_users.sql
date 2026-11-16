CREATE TABLE users (
    id UUID PRIMARY KEY,
    email TEXT NOT NULL CHECK (length(email) BETWEEN 3 AND 320),
    name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 200),
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX users_email_unique ON users (lower(email));
