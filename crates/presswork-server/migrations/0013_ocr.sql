ALTER TABLE library_files DROP CONSTRAINT library_files_state_check;

ALTER TABLE library_files
    ADD CONSTRAINT library_files_state_check
    CHECK (state IN ('reading', 'recognizing', 'ready', 'failed'));
