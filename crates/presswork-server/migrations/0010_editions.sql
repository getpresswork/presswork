ALTER TABLE editions
    ADD COLUMN publisher TEXT NOT NULL DEFAULT '' CHECK (length(publisher) <= 200),
    ADD COLUMN language TEXT NOT NULL DEFAULT '' CHECK (length(language) <= 35),
    ADD COLUMN translators TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN standard BOOLEAN NOT NULL DEFAULT false;

UPDATE editions SET standard = true;

CREATE UNIQUE INDEX editions_standard ON editions (work_id) WHERE standard;
