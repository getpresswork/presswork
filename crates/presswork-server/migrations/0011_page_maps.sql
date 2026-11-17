CREATE TABLE page_ranges (
    file_id UUID NOT NULL REFERENCES library_files (id) ON DELETE CASCADE,
    source TEXT NOT NULL CHECK (source IN ('pdf', 'member')),
    first_page INTEGER NOT NULL CHECK (first_page >= 1),
    style TEXT NOT NULL CHECK (
        style IN ('arabic', 'upper_roman', 'lower_roman', 'upper_letters', 'lower_letters', 'none')
    ),
    prefix TEXT NOT NULL DEFAULT '' CHECK (length(prefix) <= 20),
    start INTEGER NOT NULL DEFAULT 1 CHECK (start >= 1),
    PRIMARY KEY (file_id, source, first_page)
);

ALTER TABLE library_pages ADD COLUMN label TEXT NOT NULL DEFAULT '';

UPDATE library_pages SET label = number::TEXT;
