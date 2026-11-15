# Decisions

This chapter records the main design decisions. Each decision has a number, a status, an origin, a reason, and the other options.

- The origin "project owner" means that the project owner made the decision.
- The origin "default" means that the design team selected the option. The project owner can review it.

Do not change what an accepted decision says. You can correct its language. To change a decision, add a new decision that replaces it. Then set the status of the old decision to "replaced by D*n*".

## D1. Embedded Typst engine and managed toolchains

- **Status:** Accepted, 2026-11-14. **Origin:** Project owner.
- **Decision:** Link the Typst crates for the preview and the IDE functions. Install official Typst CLI versions for the exports of projects that pin a different version.
- **Reason:** Only the embedded engine gives access to the document model. Only the pinned CLI gives the exact output of a different version.
- **Other options:** One worker process for each Typst version, which has a high build cost. The embedded engine only, which has no pins. The CLI only, which gives a weak preview.

## D2. Local-first desktop application

- **Status:** Accepted, 2026-11-14. **Origin:** Project owner.
- **Decision:** The desktop application does all typesetting without a server. A server connection adds the team functions. Offline edits sync when the client connects again.
- **Reason:** A person without a team can use Presswork without a server. Teams can continue to edit during network problems.
- **Other options:** A server for all projects. Separate local and server modes without offline sync.

## D3. Live edits with Loro and revisions on the server

- **Status:** Accepted, 2026-11-14. **Origin:** Project owner.
- **Decision:** Use the Loro CRDT for live edits, with remote cursors and presence. The server records named revisions for review and approval. A bridge exports revisions to Git.
- **Reason:** Publishing teams frequently edit together at the same time. Formal review and approval must use fixed revisions.
- **Other options:** Asynchronous changes with Git only. A lock on each file, as in classic publishing systems.

## D4. PostgreSQL for the server

- **Status:** Accepted, 2026-11-14. **Origin:** Project owner.
- **Decision:** The server keeps all records in PostgreSQL.
- **Reason:** PostgreSQL is good for many concurrent users. It has full-text search, JSONB, and `LISTEN` and `NOTIFY` for live events.
- **Other options:** SQLite, which has limits for concurrent writes. Both databases behind an abstraction, which makes twice the number of tests.

## D5. Four publication kinds on one model

- **Status:** Accepted, 2026-11-14. **Origin:** Project owner.
- **Decision:** Supply templates for books, periodicals, technical documentation, and academic publications. All kinds use the same records: collections, publications, and components.
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
- **Reason:** Periodicals have tasks that occur again at intervals. All kinds can use automatic checks and exports.
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
- **Decision:** Write an editor widget on a pure text model with change sets that map positions.
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

## D16. A pinned Rust toolchain and the Rust 2024 edition

- **Status:** Accepted, 2026-11-14. **Origin:** Default.
- **Decision:** `rust-toolchain.toml` pins Rust 1.100.0. All crates use the Rust 2024 edition.
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

## D22. SQL statements that tests check

- **Status:** Accepted, 2026-11-14. **Origin:** Default.
- **Decision:** Use `sqlx` statements that the database checks at run time. Each statement has database tests.
- **Reason:** A build of the Cargo workspace operates without a database or a cache of SQL metadata.
- **Other options:** The `sqlx` compile-time macros with an offline cache of SQL metadata.

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

- **Status:** Replaced by D48. **Origin:** Default.
- **Decision:** The word "edition" means one number of a periodical. The word "issue" always means an item in the issue list.
- **Reason:** Periodicals use "issue" for a number. One word must have one meaning.
- **Other options:** Use "issue" for both meanings.

## D27. A library of works, editions, and files

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The library keeps works, editions, and files, in a simple form of the FRBR model. Citations point to editions.
- **Reason:** One work has many editions and translations, each with its own page numbers. A team must select among them.
- **Other options:** Flat references, as in Zotero. Zotero as the store.

## D28. Standard editions with purposes and scopes

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** For each work, a team sets standard editions, each with a purpose. The scope is the organization, a collection, or a publication. A narrow scope replaces a wide scope. Citations of other editions get a warning.
- **Reason:** All members must quote and cite the same editions.
- **Other options:** Standards for each publication only. Recommendations without checks.

## D29. OCR with Tesseract on the server and ocrs on the desktop

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The server runs OCRmyPDF with Tesseract. The desktop runs ocrs, a pure Rust engine. One `OcrEngine` trait covers both.
- **Reason:** The server gets the best accuracy and more than 100 languages. The desktop gets offline OCR without external tools.
- **Other options:** Tesseract on all computers. ocrs only. OCR only on the server.

## D30. Research functions

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Add the quote checker, the concordance, shared annotations, and research notes.
- **Reason:** These functions connect the library to the text of the publications.
- **Other options:** A library without connections to the text.

## D31. Tantivy for full-text search

- **Status:** Accepted, 2026-11-15. **Origin:** Default.
- **Decision:** Use Tantivy for full-text search on the desktop and on the server.
- **Reason:** One engine gives the same results everywhere. It operates offline and inside encrypted projects on the desktop.
- **Other options:** PostgreSQL search on the server and SQLite search on the desktop. Meilisearch as a separate service.

## D32. Five task views

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Tasks and tickets have list, board, timeline, calendar, and table views. The calendar has an ICS feed.
- **Reason:** Each role plans in a different way. All views show the same records.
- **Other options:** Fewer views.

## D33. Progress, time, clearances, and budgets

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Track progress from the real sources, time on tasks, clearances for texts and images, and budgets with costs.
- **Reason:** Production managers must see the state, the costs, and the legal risks of each publication.
- **Other options:** Progress only.

## D34. Dependencies, milestones, and schedule templates

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Tasks have dependencies. Schedules have milestones and come from templates. Presswork proposes a new schedule when a task is late.
- **Reason:** Publication schedules depend on fixed dates, for example the print date.
- **Other options:** Due dates only. Full resource plans with the capacity of each person.

## D35. Full team chat

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Presswork has channels, direct messages, replies, and comments on each object, with an inbox and summary emails.
- **Reason:** The team discussion stays next to the tasks, tickets, and chapters that it is about.
- **Other options:** Comments on objects only. An external chat only.

## D36. Project windows with detachable panes

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Each project opens in its own window. Each pane can move into its own window. One process runs all windows.
- **Reason:** Typesetters use a second screen for the preview. One project in each window keeps the context clear.
- **Other options:** Free windows with panes from many projects. One window with project tabs.

## D37. A release train every two weeks, and nightly builds

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Every second Monday, CI bumps the version from the commit types and makes a release, if there are `feat`, `fix`, or `perf` commits. A nightly prerelease contains the newest packages.
- **Reason:** Versions change routinely, but not with each commit.
- **Other options:** Release pull requests with release-plz. A monthly train. Manual releases at milestones.

## D38. A signed updater

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The desktop application checks GitHub for releases at start, checks an Ed25519 signature, and installs the update at the next start. Users can turn it off.
- **Reason:** Users get corrections quickly. The signature protects them from changed packages.
- **Other options:** A message only. No update check.

## D39. Two-factor sign-in and encryption

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Add two-factor sign-in, encryption at rest, and end-to-end encrypted projects before 1.0. Sign-in through OpenID Connect stays at 1.0.
- **Reason:** Publishers keep unpublished texts and sensitive data on the server.
- **Other options:** Sign-in through OpenID Connect earlier.

## D40. Editorial tools

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Add suggestion mode, spelling and house style, the termbase, and back-of-book index tools.
- **Reason:** Copyeditors and indexers must edit the same documents as the authors.
- **Other options:** External tools for these jobs.

## D41. Production functions

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Add EPUB export, covers, identifiers and metadata, and print production.
- **Reason:** A publication is not complete without these outputs.
- **Other options:** PDF output only.

## D42. Outside contributors and translation

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Add the web portal, the translation workspace, calls for papers, and the read-aloud function.
- **Reason:** Authors, reviewers, and translators take part without the desktop application, or with special tools.
- **Other options:** Desktop users only.

## D43. Platform functions

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Add the command-line tool, the asset library, WebAssembly plugins, and opt-in AI assistance.
- **Reason:** Teams automate their jobs and add their own functions. AI assistance stays off until an administrator turns it on.
- **Other options:** No extension points.

## D44. Connections between all parts

- **Status:** Accepted, 2026-11-15. **Origin:** Default, from a requirement of the project owner.
- **Decision:** All objects have stable links, mentions, backlinks, and hover cards. All changes make events. One search finds all objects. Each new function must follow the rules in [Connections](connections.md).
- **Reason:** The project owner asked that the parts of Presswork operate as one system.
- **Other options:** Separate modules with a small number of fixed links.

## D45. Seven workspaces in the sequence of a publication

- **Status:** Accepted, 2026-11-15. **Origin:** Default.
- **Decision:** The workspaces are Plan, Research, Write, Translate, Review, Typeset, and Publish, in this sequence. Each user sees only the workspaces of their roles.
- **Reason:** The library and the translation workspace must have their own layouts. The sequence follows the life of a publication.
- **Other options:** Five workspaces, with the library and translation as panels only.

## D46. Product quality limits

- **Status:** Accepted, 2026-11-15. **Origin:** Default, from a requirement of the project owner.
- **Decision:** Presswork has performance limits, rules for complete views, rules for data safety, local crash reports, and design reviews. Refer to [Quality](quality.md).
- **Reason:** The project owner asked for the quality of a commercial product. Measurable limits make this goal clear.
- **Other options:** Quality goals without numbers.

## D47. One crate for each domain area

- **Status:** Accepted, 2026-11-15. **Origin:** Default.
- **Decision:** Replace the single domain crate with one crate for each area and a shared kernel. Areas refer to each other only through links.
- **Reason:** The scope is now large. Small areas with one way to connect keep the code healthy.
- **Other options:** One large domain crate.

## D48. Words with one meaning

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** "Issue" means one number of a periodical. "Ticket" means an item in the ticket list. "Edition" means a published form of a work in the library. The style guide lists all words with one meaning.
- **Reason:** Periodical teams use "issue" for their numbers. The library must use "edition" for its records. In STE, each word has only one meaning.
- **Other options:** "Number" for periodicals. "Query" for tracker items.

## D49. Markdown for short texts

- **Status:** Accepted, 2026-11-15. **Origin:** Default.
- **Decision:** Research notes, comments, tickets, tasks, and chat messages use Markdown. Publications use Typst.
- **Reason:** Markdown is easy for short texts, and iced can show it. Typst stays the language of publications.
- **Other options:** Typst markup for all texts.

## D50. A web portal with server-rendered HTML

- **Status:** Accepted, 2026-11-15. **Origin:** Default.
- **Decision:** The server renders the HTML of the web portal, with small scripts. The server renders the proof pages as images.
- **Reason:** Plain HTML loads quickly, and screen readers can read it well. The browser does not run Typst.
- **Other options:** The iced application compiled to WebAssembly. A separate web framework.

## D51. Presswork helper packages on Typst Universe

- **Status:** Accepted, 2026-11-15. **Origin:** Default.
- **Decision:** Presswork publishes its Typst helper packages, for example the index package, on Typst Universe.
- **Reason:** Projects that use these packages still compile with the Typst command-line tool.
- **Other options:** Packages inside Presswork only.
