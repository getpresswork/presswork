CREATE TABLE projects (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 200),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX projects_organization ON projects (organization_id);

CREATE TABLE tasks (
    id UUID PRIMARY KEY,
    project_id UUID NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    title TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 300),
    description TEXT NOT NULL DEFAULT '' CHECK (length(description) <= 100000),
    state TEXT NOT NULL CHECK (state IN ('to_do', 'in_progress', 'done')),
    assignee_id UUID REFERENCES users (id) ON DELETE SET NULL,
    due_date DATE,
    created_by UUID REFERENCES users (id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX tasks_project ON tasks (project_id);
CREATE INDEX tasks_assignee ON tasks (assignee_id);
