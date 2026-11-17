CREATE TABLE jobs (
    id UUID PRIMARY KEY,
    kind TEXT NOT NULL,
    subject UUID NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('queued', 'running', 'done', 'failed')),
    attempts INTEGER NOT NULL DEFAULT 0,
    run_after TIMESTAMPTZ NOT NULL DEFAULT now(),
    locked_until TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX jobs_waiting ON jobs (run_after) WHERE state IN ('queued', 'running');

ALTER TABLE library_files
    ADD COLUMN state TEXT NOT NULL DEFAULT 'ready' CHECK (state IN ('reading', 'ready', 'failed')),
    ADD COLUMN problem TEXT;
