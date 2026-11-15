# Roadmap

This chapter gives the milestones. The release train makes a release every two weeks. Thus each milestone has many releases. Refer to [Delivery](delivery.md).

## Milestone 1: thin slice (0.1)

Milestone 1 makes a small version of each part, from end to end. The goal is to find design problems between the parts early.

1. Cargo workspace, CI, release builds, the release train, and this book.
2. Desktop application: a main window with the workspace bar.
3. Text model: buffer, positions, edits, selections, and history.
4. Editor widget: one cursor, text input, deletion, cursor movement, selection, scroll, and Typst syntax colors.
5. Typst engine: a world with overlays, compilation, diagnostics, and page images.
6. Preview widget: a vertical list of pages, with zoom, also in a detached window.
7. Project model: open a local folder with a `presswork.toml` manifest.
8. Toolchain manager: list, install, and select Typst versions.
9. Server: accounts, sign-in, organizations, projects, and tasks.
10. Sync: live edits of one text file by two persons, through Loro.
11. Library: upload a PDF file, run OCR on the server, find its text, and cite it in Typst.
12. Chat: one channel with live messages.
13. Desktop application: sign in, open a server project, edit with another person, complete tasks, and chat.
14. Installers for all platforms.

## Milestone 2: editor and preview (0.2)

- Editor: many cursors, find and replace, completion, tooltips, go to definition, and folding.
- Preview: spreads, guides, jumps between source and pages, and thumbnails.
- Spelling and house style.
- Suggestion mode.
- Export profiles and native preflight.
- The command-line tool.
- The updater, with digital signatures.

## Milestone 3: collaboration (0.3)

- Presence with remote cursors.
- Revisions, with text compare and page compare.
- Tickets and comments with text anchors and page anchors.
- Links, backlinks, hover cards, and the Related panel.
- Chat with channels, direct messages, replies, and files.
- The inbox, notifications, and summary emails.
- The offline queue and conflict messages.
- Two-factor sign-in and encryption at rest.
- The web portal for queries and proofs.
- The Git bridge.

## Milestone 4: library and research (0.4)

- Works, editions, files, and page maps.
- Standard editions with purposes and scopes.
- OCR on the server with Tesseract, and on the desktop with ocrs.
- Shared annotations and research notes.
- The quote checker and the concordance.
- Import from Zotero, BibTeX, BibLaTeX, and Hayagriva, and lookups by ISBN and DOI.

## Milestone 5: production management (0.5)

- Roles, permissions, and teams.
- Configurable workflows and the workflow board.
- Checklists with manual and automatic items.
- Templates for books, periodicals, technical documentation, and academic publications.
- List, board, timeline, calendar, and table views.
- Dependencies, milestones, and schedule templates.
- Progress and time.
- Routines with schedules and event triggers.
- Reports, with CSV export and Typst PDF export.

## Milestone 6: production and output (0.6)

- veraPDF validation.
- EPUB export with EPUBCheck.
- Covers, spine widths, and barcodes.
- The ISBN pool, ONIX export, and Crossref deposits.
- Print production: ICC profiles, marks, imposition, and PDF/X.
- The asset library and clearances.
- Budgets and costs.

## Milestone 7: language and intake (0.7)

- The termbase.
- The translation workspace, translation memory, and XLIFF.
- Back-of-book index tools.
- The read-aloud function.
- Calls for papers, submissions, and peer review.

## Milestone 8: platform (0.8)

- Plugins with WebAssembly.
- Opt-in AI assistance.
- Encrypted projects.
- Webhooks.
- Team packages and team font libraries.

## Release 1.0

- Screen reader support and translations of the interface.
- Signed packages.
- Flatpak, Homebrew, and winget packages.
- Sign-in through OpenID Connect.
- An external security audit.
- Guides for users and administrators.
