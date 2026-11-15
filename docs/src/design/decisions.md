# Decisions

This chapter records the main design decisions. Each decision has a number, a status, an origin, a reason, and the other options.

- The origin "project owner" means that the project owner made the decision.
- The origin "default" means that the design team selected the option. The project owner can review it.

Do not change the text of an accepted decision. To change a decision, add a new decision that replaces it. Then set the status of the old decision to "replaced by D*n*".

## D1. Embedded Typst engine and managed toolchains

- **Status:** Accepted, 2026-11-14. **Origin:** Project owner.
- **Decision:** Link the Typst crates for the preview and the IDE functions. Install official Typst CLI versions for the exports of projects that pin a different version.
- **Reason:** Only the embedded engine gives access to the document model. Only the pinned CLI gives the exact output of a different version.
- **Other options:** One worker process for each Typst version, which has a high build cost. The embedded engine only, which has no pins. The CLI only, which gives a weak preview.

## D2. Local-first desktop application

- **Status:** Accepted, 2026-11-14. **Origin:** Project owner.
- **Decision:** The desktop application does all typesetting work without a server. A server connection adds the team functions. Offline edits sync when the client connects again.
- **Reason:** Persons who work alone can use Presswork without a server. Teams can work during network problems.
- **Other options:** A server for all projects. Separate local and server modes without offline sync.

## D3. Live editing with Loro and revisions on the server

- **Status:** Accepted, 2026-11-14. **Origin:** Project owner.
- **Decision:** Use the Loro CRDT for live editing, with remote cursors and presence. The server records named revisions for review and approval. A bridge exports revisions to Git.
- **Reason:** Publishing teams frequently edit together at the same time. Formal review and approval must use fixed revisions.
- **Other options:** Asynchronous work with Git only. A lock on each file, as in classic publishing systems.

## D4. PostgreSQL for the server

- **Status:** Accepted, 2026-11-14. **Origin:** Project owner.
- **Decision:** The server keeps all records in PostgreSQL.
- **Reason:** PostgreSQL is good for many concurrent users. It has full-text search, JSONB, and `LISTEN` and `NOTIFY` for live events.
- **Other options:** SQLite, which has limits for concurrent writes. Both databases behind an abstraction, which makes twice the number of tests.

## D5. Four publication kinds on one model

- **Status:** Accepted, 2026-11-14. **Origin:** Project owner.
- **Decision:** Supply templates for books, periodicals, technical documentation, and academic work. All kinds use the same records: collections, publications, and components.
- **Reason:** One generic model is simpler to keep correct than four special models.
- **Other options:** Start with one kind only.

## D6. Configurable workflows with templates

- **Status:** Accepted, 2026-11-14. **Origin:** Project owner.
- **Decision:** Administrators define stages, transitions, guards, and actions. Presswork supplies templates.
- **Reason:** Each publishing team has its own procedure.
- **Other options:** One fixed workflow. Workflows as scripts, which are difficult for persons who do not write code.

## D7. Routines with schedules and events

- **Status:** Accepted, 2026-11-14. **Origin:** Project owner.
- **Decision:** A routine is a sequence of steps. A schedule or an event starts it.
- **Reason:** Periodicals have work that occurs again at intervals. All kinds can use automatic checks and exports.
- **Other options:** Schedules only. Events only. Manual procedures only.

## D8. Role workspaces on one pane system

- **Status:** Accepted, 2026-11-14. **Origin:** Project owner.
- **Decision:** One dockable pane system with preset workspaces: Write, Typeset, Review, Plan, and Publish.
- **Reason:** Authors get a calm interface. Typesetters get all tools. All roles use the same application.
- **Other options:** A dense IDE layout for all users. A minimal interface with overlays.

## D9. One-line Conventional Commits

- **Status:** Accepted, 2026-11-14. **Origin:** Project owner.
- **Decision:** Write each commit message on one line, in the form `type(scope): summary`. The `commit-msg` hook examines the format.
- **Reason:** The history stays clear, and git-cliff makes the release notes from it.
- **Other options:** A scope prefix only. A type prefix only.

## D10. Repository in the getpresswork organization

- **Status:** Accepted, 2026-11-14. **Origin:** Project owner.
- **Decision:** The repository is `getpresswork/presswork` on GitHub.
- **Reason:** A different organization has the GitHub name `presswork`.
- **Other options:** A personal account. A different organization name.

## D11. Toolchain scope

- **Status:** Accepted, 2026-11-14. **Origin:** Project owner.
- **Decision:** The toolchain manager controls Typst versions, Typst packages, fonts, Pandoc, veraPDF, and a Java runtime for veraPDF.
- **Reason:** These tools do all steps from the manuscript to the validated PDF.
- **Other options:** Typst versions only.

## D12. A thin slice of all parts in milestone 1

- **Status:** Accepted, 2026-11-14. **Origin:** Project owner.
- **Decision:** Milestone 1 makes a small version of each part: editor, preview, toolchains, server, sync, and tasks.
- **Reason:** An early end-to-end version shows design problems between the parts.
- **Other options:** The local workbench first. The server and the production functions first.

## D13. A custom editor on a pure text model

- **Status:** Accepted, 2026-11-14. **Origin:** Default.
- **Decision:** Write an editor widget on a pure text model with change sets and position mapping.
- **Reason:** The iced `text_editor` widget cannot show remote cursors or diagnostics, and it cannot map positions through remote edits.
- **Other options:** Extend the iced `text_editor` widget.

## D14. HTTP with JSON, and a WebSocket with MessagePack

- **Status:** Accepted, 2026-11-14. **Origin:** Default.
- **Decision:** Use a JSON HTTP API for records. Use one WebSocket with MessagePack messages for sync, presence, and events. Publish an OpenAPI description.
- **Reason:** JSON is easy for other programs. MessagePack can contain the binary Loro updates without an extra encoding.
- **Other options:** gRPC, which is difficult for browsers. GraphQL, which adds complexity.

## D15. mdBook for the documentation site

- **Status:** Accepted, 2026-11-14. **Origin:** Default.
- **Decision:** Write the documentation as an mdBook in `docs`. Publish it to GitHub Pages.
- **Reason:** mdBook is the standard tool for Rust projects. It has search, and it operates without other runtimes.
- **Other options:** Zola. Typst HTML export, which is experimental.

## D16. A pinned Rust toolchain and the 2024 edition

- **Status:** Accepted, 2026-11-14. **Origin:** Default.
- **Decision:** `rust-toolchain.toml` pins Rust 1.100.0. All crates use the 2024 edition.
- **Reason:** All contributors and CI get the same compiler and the same lints.
- **Other options:** The current stable version without a pin.

## D17. Git hooks in the repository

- **Status:** Accepted, 2026-11-14. **Origin:** Default.
- **Decision:** The `.githooks` folder contains a `pre-commit` hook for `cargo fmt` and `cargo clippy`, and a `commit-msg` hook for the message format.
- **Reason:** Each commit gets the checks before it leaves the computer. CI does the same checks again.
- **Other options:** CI checks only. An external hook manager.

## D18. Fluent for all interface text

- **Status:** Accepted, 2026-11-14. **Origin:** Default.
- **Decision:** Keep all interface text in Fluent files from the first window.
- **Reason:** It is difficult to move text out of the code at a later time.
- **Other options:** Text in the code until a translation starts.

## D19. STE for all text

- **Status:** Accepted, 2026-11-14. **Origin:** Project owner, with a default extension.
- **Decision:** Write all documentation in ASD-STE100. Also use STE for interface text, templates, and commit messages.
- **Reason:** Clear text helps readers and translators. Interface text and commit messages are also documentation.
- **Other options:** STE for documentation only.

## D20. No license headers in source files

- **Status:** Accepted, 2026-11-14. **Origin:** Default.
- **Decision:** Source files have no license headers. `LICENSE` and `NOTICE` apply to the full repository.
- **Reason:** The project uses very few comments. The Apache License 2.0 does not make headers necessary.
- **Other options:** An SPDX header in each file.

## D21. Strict lints

- **Status:** Accepted, 2026-11-14. **Origin:** Default.
- **Decision:** Turn on the Clippy `pedantic` group. Make all warnings errors. Forbid `unsafe` code. Do not allow `unwrap` and `expect` outside of tests.
- **Reason:** Code health is a main goal of the project.
- **Other options:** The default Clippy lints.

## D22. SQL queries that tests check

- **Status:** Accepted, 2026-11-14. **Origin:** Default.
- **Decision:** Use `sqlx` queries that the database checks at run time. Each query has database tests.
- **Reason:** A build of the workspace operates without a database or a query cache.
- **Other options:** The `sqlx` compile-time macros with an offline query cache.

## D23. One blob store interface

- **Status:** Accepted, 2026-11-14. **Origin:** Default.
- **Decision:** Keep blobs through the `object_store` crate. It can use a local folder or an S3-compatible service.
- **Reason:** Small servers use a local folder. Large servers use object storage. The code stays the same.
- **Other options:** A local folder only.

## D24. No telemetry

- **Status:** Accepted, 2026-11-14. **Origin:** Default.
- **Decision:** Presswork does not collect usage data.
- **Reason:** Users can be sure that Presswork does not send their data to other persons.
- **Other options:** Telemetry that users must turn on.

## D25. Archives first, then installers

- **Status:** Accepted, 2026-11-14. **Origin:** Default.
- **Decision:** The first release builds make archives. `cargo-packager` makes the installers before release 0.1.0. The first releases are not signed.
- **Reason:** The pipeline operates from the first commit. Installers must have icons, and signatures must have certificates.
- **Other options:** Installers from the first commit.

## D26. Edition and issue

- **Status:** Accepted, 2026-11-14. **Origin:** Default.
- **Decision:** The word "edition" means one number of a periodical. The word "issue" always means an item in the issue list.
- **Reason:** Periodicals use "issue" for a number. One word must have one meaning.
- **Other options:** Use "issue" for both meanings.
