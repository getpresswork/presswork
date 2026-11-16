CREATE TABLE works (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    key TEXT NOT NULL CHECK (length(key) BETWEEN 1 AND 64),
    title TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 500),
    creators TEXT[] NOT NULL DEFAULT '{}',
    created_by UUID REFERENCES users (id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, key)
);

CREATE TABLE editions (
    id UUID PRIMARY KEY,
    work_id UUID NOT NULL REFERENCES works (id) ON DELETE CASCADE,
    year INTEGER CHECK (year BETWEEN -9999 AND 9999),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX editions_work ON editions (work_id);

CREATE TABLE library_files (
    id UUID PRIMARY KEY,
    edition_id UUID NOT NULL UNIQUE REFERENCES editions (id) ON DELETE CASCADE,
    size BIGINT NOT NULL CHECK (size > 0),
    sha256 BYTEA NOT NULL,
    pages INTEGER NOT NULL CHECK (pages >= 0),
    pages_with_text INTEGER NOT NULL CHECK (pages_with_text BETWEEN 0 AND pages),
    created_by UUID REFERENCES users (id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE library_pages (
    file_id UUID NOT NULL REFERENCES library_files (id) ON DELETE CASCADE,
    number INTEGER NOT NULL CHECK (number >= 1),
    text TEXT NOT NULL,
    PRIMARY KEY (file_id, number)
);
