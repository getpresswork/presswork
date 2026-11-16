CREATE TABLE chat_messages (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    author_id UUID REFERENCES users (id) ON DELETE SET NULL,
    text TEXT NOT NULL CHECK (length(text) BETWEEN 1 AND 4000),
    sent_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX chat_messages_organization ON chat_messages (organization_id, sent_at);
