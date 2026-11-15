# Roadmap

This chapter gives the milestones. Each milestone ends with a release.

## Milestone 1: thin slice (0.1)

Milestone 1 makes a small version of each part, from end to end. The goal is to find design problems early.

1. Workspace, CI, release builds, and this book.
2. Desktop application: a main window with the workspace bar.
3. Text model: buffer, positions, edits, selections, and history.
4. Editor widget: one cursor, text input, deletion, cursor movement, selection, scroll, and Typst syntax colors.
5. Typst engine: a world with overlays, compilation, diagnostics, and page rendering.
6. Preview widget: a vertical list of pages, with zoom.
7. Project model: open a local folder with a `presswork.toml` manifest.
8. Toolchain manager: list, install, and select Typst versions.
9. Server: accounts, sign-in, organizations, projects, and tasks.
10. Sync: live editing of one text file by two persons, through Loro.
11. Desktop application: sign in, open a server project, edit with another person, and see and complete tasks.
12. Release 0.1.0 with installers for all platforms.

## Milestone 2: workbench depth (0.2)

- Editor: many cursors, find and replace, completion, tooltips, go to definition, and folding.
- Preview: spreads, guides, jumps between source and pages, and thumbnails.
- Toolchains: Pandoc import, the package cache, and the font panel.
- Export profiles and native preflight.

## Milestone 3: collaboration depth (0.3)

- Presence with remote cursors.
- Revisions, with text compare and page compare.
- Issues and comments with text anchors and page anchors.
- The offline queue and conflict messages.
- The Git bridge.

## Milestone 4: production management (0.4)

- Roles, permissions, and teams.
- Configurable workflows and the workflow board.
- Checklists with manual and automatic items.
- Templates for books, periodicals, technical documentation, and academic work.

## Milestone 5: automation and reports (0.5)

- Routines with schedules and event triggers.
- Reports in the Plan workspace, with CSV export and Typst PDF export.
- veraPDF validation in routines and checklists.
- Team packages and team font libraries.

## Milestone 6: release quality (1.0)

- Accessibility and translations.
- Signed packages.
- Flatpak, Homebrew, and winget packages.
- Sign-in through OpenID Connect.
- Guides for users and administrators.
