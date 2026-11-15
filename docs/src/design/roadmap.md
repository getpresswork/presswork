# Roadmap

This chapter gives the milestones. The release train makes a release every two weeks. Thus each milestone has many releases. Refer to [Delivery](delivery.md).

## Milestone 1: thin slice (0.1)

Milestone 1 makes a small version of each part, from end to end. The goal is to find design problems between the parts early.

1. Cargo workspace, CI, release builds, the release train, and this book.
2. Desktop application: a main window with the workspace bar, and the design system with tokens, themes, fonts, and icons.
3. Logs: structured logs, log files, and a first log panel.
4. Text model: buffer, positions, edits, selections, and history.
5. Editor widget: one cursor, text input, deletion, cursor movement, selection, scroll, and Typst syntax colors.
6. Typst engine: a world with overlays, compilation, diagnostics, and page images.
7. Preview widget: a vertical list of pages, with zoom, also in a detached window.
8. Project model: open a local folder with a `presswork.toml` manifest.
9. Toolchain manager: list, install, and select Typst versions.
10. Server: accounts, sign-in, organizations, projects, and tasks.
11. Sync: live edits of one text file by two persons, through Loro.
12. Library: upload a PDF file, run OCR on the server, find its text, and cite it in Typst.
13. Chat: one channel with live messages.
14. Desktop application: sign in, open a server project, edit with another person, complete tasks, and chat.
15. Installers for all platforms.

## Milestone 2: editor and preview (0.2)

- Editor: many cursors, find and replace, completion, tooltips, go to definition, and folding.
- Editor: Vim mode, and right-to-left and CJK text.
- Preview: spreads, guides, jumps between source and pages, and thumbnails.
- The style panel, the table builder, and the math builder.
- The typesetting QA report and the baseline grid check.
- Spelling and house style.
- Suggestion mode.
- Export profiles, native preflight, and build reports.
- The home dashboard, the Typst scratchpad, and operating system integration.
- The command-line tool.
- The updater, with digital signatures.

## Milestone 3: collaboration (0.3)

- Presence with remote cursors, and follow mode.
- Revisions, the history slider, text compare, and page compare.
- Tickets and comments with text anchors and page anchors.
- Links, backlinks, hover cards, and the Related panel.
- Chat with channels, direct messages, replies, and files.
- The inbox, notifications, summary emails, and web push notifications.
- The email bridge.
- The offline queue and conflict messages.
- Two-factor sign-in, sessions and devices, and encryption at rest.
- The web portal for queries and proofs, also on phones, with watermarked proofs.
- Proof collation.
- The admin console, backups and restore, and the support bundle.
- The Git bridge.

## Milestone 4: library and research (0.4)

- Works, editions, files, and page maps.
- Standard editions with purposes and scopes.
- OCR on the server with Tesseract, and on the desktop with ocrs.
- Shared annotations and research notes.
- The quote checker, the similarity check, and the concordance.
- The citation style editor.
- Lift from vector PDF files: charts, tables, and equations.
- Import from Zotero, BibTeX, BibLaTeX, and Hayagriva, and lookups by ISBN and DOI.

## Milestone 5: production management (0.5)

- Roles, permissions, teams, and co-publishing.
- Configurable workflows and the workflow board.
- Checklists with manual and automatic items.
- Templates for books, periodicals, technical documentation, and academic publications.
- List, board, timeline, calendar, and table views, with two-way calendar sync.
- Dependencies, milestones, and schedule templates.
- Progress and time.
- Contacts, contracts, and deliverables.
- Routines with schedules and event triggers.
- Reports, with CSV export and Typst PDF export.

## Milestone 6: production and output (0.6)

- veraPDF validation and the accessibility audit.
- EPUB export with EPUBCheck.
- Covers, spine widths, and barcodes.
- The ISBN pool, ONIX export, Crossref deposits, JATS export, and ORCID identifiers.
- Print production: ICC profiles, the soft proof, marks, imposition, and PDF/X.
- Print plans, cost estimates, and the copyfitting assistant.
- Hyphenation exceptions.
- The asset library and clearances.
- Budgets and costs.
- Brand kits, the template gallery, the chart builder, and the tools for packages and templates.
- Version migration and the compile profiler.

## Milestone 7: language, reuse, and intake (0.7)

- The termbase.
- The translation workspace, translation memory, and XLIFF.
- Controlled language checks, and find and replace in the organization.
- Shared content blocks, variants, the structure board, and data merge.
- Back-of-book index tools.
- The read-aloud function.
- Calls for papers, submissions, and peer review.

## Milestone 8: AI assistance (0.8)

- The AI gateway, the AI policy, the AI audit log, quotas, and provenance labels.
- Inline AI tools and the assistant.
- The editorial, Typst, library, translation, and management AI functions.
- Lift from images with AI.
- Semantic search with a local embedding model.
- The desktop MCP server and the team MCP endpoint.
- Overnight batch jobs and whole-book context.

## Milestone 9: platform and operations (0.9)

- Plugins with WebAssembly.
- Encrypted projects.
- Webhooks.
- Team packages and team font libraries.
- Enterprise deployment, retention rules, audit log export, and IP allow lists.
- OpenTelemetry export.

## Release 1.0

- Screen reader support and translations of the interface.
- Digital signatures for the installers.
- Flatpak, Homebrew, and winget packages.
- Sign-in through OpenID Connect.
- An external security audit.
- Guides for users and administrators.
