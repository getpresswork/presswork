# Decisions

This chapter records the main design decisions. Each decision has a number, a status, an origin, a reason, and the other options.

- The origin "project owner" means that the project owner made the decision.
- The origin "default" means that the design team selected the option. The project owner can review it.
- The origin "default, reviewed by the project owner" means that the project owner examined a default decision and kept it.

Do not change what an accepted decision says. You can correct its language. To change a decision, add a new decision that replaces it. Then set the status of the old decision to "replaced by D*n*".

## D1. Embedded Typst engine and managed toolchains

- **Status:** Accepted, 2026-11-14. Partially replaced by D100. **Origin:** Project owner.
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
- **Decision:** The first release builds make archives. `cargo-packager` makes the installers before release 0.1.0. The first releases have no digital signatures.
- **Reason:** The pipeline operates from the first commit. Installers must have icons, and digital signatures must have certificates.
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

- **Status:** Accepted, 2026-11-15. Partially replaced by D81. **Origin:** Project owner.
- **Decision:** Every second Monday, CI bumps the version from the commit types and makes a release, if there are `feat`, `fix`, or `perf` commits. A nightly prerelease contains the newest packages.
- **Reason:** Versions change routinely, but not with each commit.
- **Other options:** Release pull requests with release-plz. A monthly train. Manual releases at milestones.

## D38. An updater with digital signatures

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The desktop application checks GitHub for releases at start, checks an Ed25519 digital signature, and installs the update at the next start. Users can turn it off.
- **Reason:** Users get corrections quickly. The digital signature protects them from changed packages.
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

## D52. Tools for typesetters

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Add version migration, the typesetting QA report, the compile profiler, the tools for packages and templates, the baseline grid check, the soft proof, hyphenation exceptions, and the copyfitting assistant.
- **Reason:** Typesetters must find layout problems and move projects to new Typst versions safely.
- **Other options:** Diagnostics from Typst only.

## D53. Excellent logs

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Structured logs in all crates, a log panel, build reports for each compilation and export, a support bundle, and OpenTelemetry export on the server.
- **Reason:** Persons must find the cause of a problem quickly, and repeat a build exactly.
- **Other options:** Log files only.

## D54. Design tools that write Typst

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Add the style panel, the table, math, and chart builders, and brand kits. Each tool writes normal Typst code that stays visible.
- **Reason:** Designers who do not write code can make page designs. The project stays a normal Typst project.
- **Other options:** Author mode, a rich-text view that hides the markup. The project owner did not select it, so Typst stays visible to all users.

## D55. Lift from PDF

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner, from an idea of the project owner.
- **Decision:** Rebuild charts as data, tables, and equations from PDF pages as Typst code. Charts in images use AI assistance.
- **Reason:** Publications often reuse charts and tables from other documents. A rebuilt chart is sharp and editable, and it uses the house style.
- **Other options:** Images of the original only.

## D56. Structure and reuse

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Add the structure board, shared content blocks, variants, and data merge.
- **Reason:** Teams reuse pages and text in many publications, and make several outputs from one source.
- **Other options:** Copies by hand.

## D57. More editorial checks

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Add proof collation, the accessibility audit, controlled language checks, and find and replace in the organization.
- **Reason:** These checks make the proof rounds faster and the publications accessible.
- **Other options:** A round trip through Word files with tracked changes. The project owner did not select it.

## D58. Contacts, contracts, and print plans

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Add the contacts directory, contracts with deliverables, and print plans with cost estimates.
- **Reason:** Deliverables and print costs control the schedule and the budget of a publication.
- **Other options:** Royalty statements. The project owner did not select them.

## D59. Sales and promotion outputs out of scope

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Presswork does not make catalogs, sell sheets, websites, newsletters, or audiobooks.
- **Reason:** Other tools do these jobs. Presswork stays focused on the publication.
- **Other options:** Sales and promotion outputs from the metadata.

## D60. Teamwork functions

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Add the email bridge, watermarked proofs, follow mode, and co-publishing.
- **Reason:** Authors use email, unpublished books must stay protected, and some books have two publishers.
- **Other options:** None of these functions.

## D61. Server operations

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Add the admin console, backups and restore, and enterprise deployment.
- **Reason:** A self-hosted server must be safe and easy to operate.
- **Other options:** Importers from other tools. The project owner did not select them.

## D62. Desktop experience

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Add the home dashboard, the history slider, operating system integration, and the Typst scratchpad.
- **Reason:** A commercial product starts in a useful place and fits into the operating system.
- **Other options:** A start screen only.

## D63. Editor additions

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Add Vim mode, and full support for right-to-left and CJK text.
- **Reason:** Technical users want Vim keys. Translators and international publishers use all scripts.
- **Other options:** Emacs mode, and support for external editors. The project owner did not select them.

## D64. Research additions

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Add the similarity check, JATS export and ORCID identifiers, the citation style editor, and semantic search.
- **Reason:** Academic publishers use these functions, and all authors must prevent accidental plagiarism.
- **Other options:** None of these functions.

## D65. Phones and calendars

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Members can use the web portal on phones, with web push notifications. The calendar syncs in two directions with CalDAV, Google Calendar, and Microsoft 365.
- **Reason:** Approvals and messages must not wait for a desktop.
- **Other options:** Native mobile applications. The project owner did not select them.

## D66. Account and data controls

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Add sessions and devices, retention rules, audit log export, and IP allow lists.
- **Reason:** Organizations must control who signs in, from where, and how long data stays.
- **Other options:** None of these controls.

## D67. AI as tools and an assistant that proposes

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Presswork has inline AI tools and an assistant panel. The assistant reads the project and uses Presswork commands as tools. Each change from AI is a suggestion or a proposal that a person approves.
- **Reason:** AI is useful, but persons stay responsible for the text and the schedule.
- **Other options:** Inline tools only. An autonomous agent. Minimal AI.

## D68. AI providers and the AI gateway

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Support the Anthropic API and local model servers. The server keeps the provider keys in an AI gateway that applies the policy, the quotas, the audit log, and the removal of personal data. Solo users and local models can connect directly from the desktop.
- **Reason:** Organizations control the keys and the data. Persons without a server can still use AI.
- **Other options:** Cloud platforms and other hosted services. Keys on each desktop. A gateway for all requests.

## D69. MCP servers

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Presswork has a desktop MCP server and a team MCP endpoint. Changes from MCP clients become suggestions and proposals.
- **Reason:** Persons can use their own AI clients with Presswork projects.
- **Other options:** An assistant that uses other MCP servers. The project owner did not select it.

## D70. AI functions

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The AI functions are in [AI assistance](ai.md): editorial, Typst, library, translation, and management functions.
- **Reason:** Each function gives its result to a person through an object that Presswork already has.
- **Other options:** Developmental feedback and summaries of library chapters. The project owner did not select them.

## D71. AI consent and controls

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** An AI policy, a preview of each request, and the AI audit log. Confirmation only for sensitive data. Quotas, cost reports, removal of personal data, and provenance labels.
- **Reason:** Persons see what goes to a provider, without a dialog for each request.
- **Other options:** A confirmation for each request. A policy without a preview.

## D72. One default AI model

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** All AI functions use Claude Opus 5.5. An administrator can select a different model.
- **Reason:** One strong model gives consistent results and one cache for each book.
- **Other options:** A model for each function. Automatic selection.

## D73. AI for large jobs

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Use Message Batches for overnight jobs, whole-book context with prompt caching, and the inference region where the provider supports it.
- **Reason:** Book-length jobs cost less in batches, and repeated requests on a book cost less with a prompt cache.
- **Other options:** A fallback to a local model. The project owner did not select it.

## D74. A local embedding model

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Semantic search uses a local embedding model on the desktop and on the server.
- **Reason:** No text leaves the organization for search.
- **Other options:** An embedding service of a provider.

## D75. Direct calls to the Messages API

- **Status:** Accepted, 2026-11-15. **Origin:** Default.
- **Decision:** Presswork calls the Anthropic Messages API over HTTPS. It uses structured outputs, citations, prompt caching, Message Batches, streaming, and the server-side refusal fallback.
- **Reason:** There is no official Rust SDK. These API functions give valid results, page citations, and lower costs.
- **Other options:** A community Rust crate. A layer that is compatible with many providers.

## D76. Signature and digital signature

- **Status:** Accepted, 2026-11-15. **Origin:** Default.
- **Decision:** "Signature" means a group of pages that a printer folds and binds. "Digital signature" means cryptographic data.
- **Reason:** Print plans and imposition use signatures. In STE, each word has only one meaning.
- **Other options:** "Section" for the printed group.

## D77. A quiet editorial style

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The interface uses warm paper colors, ink colors, thin rules, and much space. One accent color marks the selected item, the focus, and the primary action.
- **Reason:** This style looks like a good book page. It keeps the attention on the document.
- **Other options:** A dense style for developer tools, with cool grays and a bright accent. A soft modern style, with rounded cards, soft shadows, and pastel colors.

## D78. Inter and JetBrains Mono

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The interface text uses Inter in three weights: regular, medium, and semibold. Code, times, and data use JetBrains Mono. Presswork includes the font files.
- **Reason:** Both fonts are easy to read at small sizes. Both have the SIL Open Font License. Included fonts make the interface the same on all platforms.
- **Other options:** IBM Plex Sans and IBM Plex Mono. Source Sans 3 and Source Code Pro.

## D79. Oxblood as the accent color

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The accent color is oxblood: #8C2F39 in the light theme and #E07A84 in the dark theme.
- **Reason:** Oxblood is a red from the tradition of printing. Its contrast is 4.5:1 or more on all surfaces of the two themes.
- **Other options:** Deep indigo, which was the recommended option. Ink blue. Forest green.

## D80. Lucide icons

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Presswork uses the Lucide icons as an icon font.
- **Reason:** Lucide has approximately 1,500 icons with one line style. It has the ISC license.
- **Other options:** Phosphor, with six weights. Tabler, with more than 5,000 icons.

## D81. Milestones set the minor version before 1.0

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Before 1.0, each release of the release train bumps only the patch version. When a milestone is complete, a maintainer starts the release train with the next minor version. After 1.0, the commit types set the version change. This decision replaces the version rule of D37 before 1.0.
- **Reason:** The version tells which milestone a release belongs to, as the roadmap shows. The release train continues every two weeks.
- **Other options:** A minor version for each release with a `feat` commit, and no version numbers in the roadmap.

## D82. ropey for the text buffer

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The `Buffer` of `presswork-text` keeps the text in a ropey 1.6 rope. Other crates do not use ropey directly.
- **Reason:** ropey is stable and fast, and the Helix editor uses it. It finds line breaks with the same rules as Typst, and it converts to UTF-16 units.
- **Other options:** crop 0.4, which finds only LF and CRLF line breaks. ropey 2.0, which is a beta release. An own rope, which needs much more work.

## D83. Change sets with runs over the full text

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** A change set is a sequence of runs that covers the full text. Each run keeps text or replaces text. Change sets can compose, invert, and map through other change sets, as in CodeMirror 6.
- **Reason:** Collaborative undo, remote cursors, and diagnostics must map through the changes of other persons.
- **Other options:** A sorted list of edits.

## D84. Linear undo with time groups

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Undo and redo use one line of groups. An edit joins the last group if it is less than 500 ms after the last edit and touches the text of the last edit. Remote changes rebase the groups. Thus undo changes only your own edits.
- **Reason:** Collaborative editors such as Google Docs operate in this way. It is easy to understand.
- **Other options:** An undo tree, as in Vim and Helix.

## D85. An editor that wraps long lines

- **Status:** Accepted, 2026-11-15. **Origin:** Default, reviewed by the project owner.
- **Decision:** The editor wraps long lines at the width of the view. It shapes each line with cosmic-text through iced, and it keeps the shaped lines in a cache.
- **Reason:** Typst source contains much prose. Writers do not add line breaks in paragraphs.
- **Other options:** No wrap, with a horizontal scroll bar. A fixed wrap column.

## D86. Syntax colors from typst-syntax

- **Status:** Accepted, 2026-11-15. **Origin:** Default, reviewed by the project owner.
- **Decision:** The editor gets its syntax colors from the highlighter of `typst-syntax`, and it parses incrementally. The colors have eight roles, and each role is a design token with a contrast test.
- **Reason:** The Typst compiler uses the same parser. Thus the colors always agree with the compiler.
- **Other options:** A TextMate grammar. Tree-sitter.

## D87. Bold and italic faces of JetBrains Mono

- **Status:** Accepted, 2026-11-15. **Origin:** Default, reviewed by the project owner.
- **Decision:** Presswork includes the bold and the italic faces of JetBrains Mono. The editor shows strong text and headings in bold, and emphasis and comments in italic.
- **Reason:** Writers see the style of the markup while they write.
- **Other options:** Synthetic bold and italic. Colors only.

## D88. Editor keys that follow the platform

- **Status:** Accepted, 2026-11-15. **Origin:** Default, reviewed by the project owner.
- **Decision:** Shortcuts use Ctrl on Windows and Linux, and Cmd on macOS. Word movement uses Ctrl on Windows and Linux, and Alt on macOS. Tab inserts two spaces.
- **Reason:** Users expect the keys of their platform. Typst code usually has an indentation of two spaces.
- **Other options:** The same keys on all platforms. Tab characters.

## D89. Optimized dependencies in development builds

- **Status:** Accepted, 2026-11-15. **Origin:** Default, reviewed by the project owner.
- **Decision:** Development builds compile all dependencies with optimization level 2. Presswork crates stay at level 0.
- **Reason:** Text shaping and Typst are very slow without optimization. With this setting, the tests of the desktop application take 1.5 seconds, not 21 seconds, and the debug application responds quickly.
- **Other options:** Level 1, which compiles faster but runs slower. No optimization.

## D90. A preview beside the editor in the Write workspace

- **Status:** Accepted, 2026-11-15. **Origin:** Default, reviewed by the project owner.
- **Decision:** The Write workspace shows the editor and the preview side by side, with the same width. The engine compiles after each change to the text, on its own thread, with one compilation at a time.
- **Reason:** Writers see the effect of each change at once. One compilation at a time keeps the computer responsive.
- **Other options:** A preview that shows only after a pause in typing. A preview in a separate window.

## D91. A small manifest that later releases can extend

- **Status:** Accepted, 2026-11-15. **Origin:** Default, reviewed by the project owner.
- **Decision:** The `presswork.toml` manifest has a `[project]` table with `name`, `main`, and `typst`. Only `name` is necessary. Presswork ignores the keys that it does not know.
- **Reason:** A small manifest is easy to write by hand. Older releases can open projects of newer releases.
- **Other options:** A strict manifest that refuses unknown keys. A manifest with all future tables now.

## D92. A native dialog to open a project folder

- **Status:** Accepted, 2026-11-15. **Origin:** Default, reviewed by the project owner.
- **Decision:** Presswork opens project folders with the native folder dialog of each platform, through `rfd`. On Linux, it uses the XDG desktop portal.
- **Reason:** Users know the dialog of their platform. The portal needs no GTK libraries.
- **Other options:** A Presswork dialog. The GTK dialog on Linux.

## D93. JetBrains Mono without ligatures

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Presswork uses the NL faces of JetBrains Mono. These faces have no ligatures. Thus the editor shows each character as you type it.
- **Reason:** In Typst, character sequences such as `==`, `---`, and `->` have a meaning. With ligatures, the editor showed `==` as one long sign, and the level of a heading was not clear. The NL faces are also smaller.
- **Other options:** JetBrains Mono with ligatures. A setting that turns the ligatures on and off.

## D94. reqwest for HTTP in the desktop application

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The desktop application uses reqwest 0.13 for HTTP, with its default TLS: rustls and the certificate store of the operating system. iced runs its tasks on tokio.
- **Reason:** The toolchain manager and the server client use the same HTTP client. The certificate store of the operating system also contains the certificates of company proxies.
- **Other options:** ureq 3, a small blocking client on a worker thread.

## D95. Pure Rust crates to unpack toolchains

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The toolchain manager unpacks `.zip` files with zip, and `.tar.xz` files with tar and lzma-rust2.
- **Reason:** lzma-rust2 is pure Rust, and it has active maintenance. The zip crate also uses it.
- **Other options:** liblzma, which binds the C library of xz-utils.

## D96. A reviewed list of digests for older Typst releases

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** For Typst 0.14.0 and later, the toolchain manager compares each download with the SHA-256 digest that GitHub publishes. For Typst 0.11.0 to 0.13.1, Presswork includes a list of digests. A CI job calculates the list one time, and a maintainer examines it before the commit.
- **Reason:** GitHub publishes digests only for the files of Typst 0.14.0 and later. Projects that pin an older version must also get a digest check.
- **Other options:** Only Typst 0.14.0 and later. An installation without a digest after a warning.

## D97. A Toolchains panel that opens from the status bar

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The status bar shows the Typst version of the open project. A click opens the Toolchains panel. The panel shows the installed and the available versions and the progress of an installation. It has commands to install, remove, and use a version.
- **Reason:** The version is always visible, and the panel is one click away.
- **Other options:** A section on a settings page. A modal dialog.

## D98. A pin in the manifest selects the Typst version

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The command "Use for this project" writes the pin into `presswork.toml`. toml_edit keeps the comments and the layout of the file. If more than one installed version agrees with the pin, Presswork uses the highest.
- **Reason:** All members of a team then use the same Typst version.
- **Other options:** A default version in the user settings. A pin and a default version in the user settings.

## D99. Only stable Typst releases in the list

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The list of available versions shows only stable releases. If the pin of the project agrees only with a release candidate, the panel also shows that candidate.
- **Reason:** Release candidates are for tests. Most users need only stable releases.
- **Other options:** A switch that shows the release candidates. All releases, with a label.

## D100. One Typst version for the preview and the exports

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The preview and the exports of a project always use the same Typst version. If the pin of the project does not accept the built-in version, the pinned Typst program makes the preview pages and the exports. The editor functions that need the document model, such as completion, tooltips, and jumps from the pages to the source, are off for such a project, and a note tells why. If no installed version agrees with the pin, Presswork shows no preview and asks you to install a version. This decision replaces the part of D1 about the preview.
- **Reason:** Writers must proof the same output that goes to print.
- **Other options:** Only the built-in version, with a request to update old pins. A worker process for each Typst version, which D1 rejected because of its build cost.

## D101. Many organizations on one server

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** One server can hold many organizations. Each record belongs to an organization, and a user can be a member of more than one organization.
- **Reason:** One server can serve a publisher and its partners, and a hosting service can serve many publishers. A later change from one organization to many would need a large migration.
- **Other options:** One organization for each server.

## D102. Accounts through invite links

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** A command on the server makes the first administrator. After that, an administrator makes an invite link for each person. A link is valid for 7 days and for one use. The server has no open sign-up.
- **Reason:** Publishers control who can see their unpublished texts.
- **Other options:** Open sign-up. Invite links and a setting for open sign-up.

## D103. No email in milestone 1

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** In milestone 1, the server sends no email. Administrators send invite links themselves and reset forgotten passwords with a server command. Email starts with the email bridge in milestone 3.
- **Reason:** Email needs mail settings and delivery checks. Milestone 1 can operate without it.
- **Other options:** Invite and password-reset emails through SMTP from milestone 1.

## D104. Device sessions that last 30 days after their last use

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** A sign-in makes a session for one device, with a random token. The server keeps only the hash of the token. Each use of the session extends it to 30 days after that use. The list of sessions shows each device, and a user can end each session.
- **Reason:** The model is simple, and it agrees with the sessions and devices of D66.
- **Other options:** Access tokens that last 15 minutes, with refresh tokens that rotate.

## D105. Database tests on PostgreSQL in Docker

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The database tests use a PostgreSQL server that a URL in the environment names. In CI, a PostgreSQL service on Linux supplies it. On a developer computer, PostgreSQL in Docker supplies it. Without the URL, the database tests stop at once and write a note.
- **Reason:** The tests use the real database, without a download in the build.
- **Other options:** Embedded PostgreSQL binaries on all platforms. Database tests only in CI.

## D106. A small task model in milestone 1

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** In milestone 1, a task has a title, a Markdown description, a state (to do, in progress, or done), one assignee, a due date, and a project.
- **Reason:** This is sufficient to complete tasks in the desktop application. Workflows and checklists come in milestone 5.
- **Other options:** The small model with checklists. Tasks with the workflow stages of D6.

## D107. Server commands for administration in milestone 1

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** In milestone 1, administrators use subcommands of `presswork-server` to make the first administrator and organizations, make invite links, and reset passwords. The admin console of D61 comes later.
- **Reason:** Commands are small and easy to test. Milestone 1 needs no admin pages.
- **Other options:** The first pages of the admin console, as server-rendered HTML.

## D108. utoipa for the OpenAPI description

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The server makes its OpenAPI description from the code with utoipa. A test makes sure that the description is valid.
- **Reason:** The description always agrees with the code. utoipa has the most users of the crates for this.
- **Other options:** A file that a person writes, with a test that compares it with the routes. aide, which uses schemars.

## D109. TLS to PostgreSQL with the certificates of the operating system

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The server connects to PostgreSQL through sqlx with rustls, the ring crypto library, and the certificate store of the operating system.
- **Reason:** Managed databases often use a certificate authority that the administrator installs on the system. The desktop application also uses the certificate store of the operating system (D94).
- **Other options:** rustls with aws-lc-rs and the Mozilla root list, which needs the CDLA-Permissive-2.0 license. No TLS in milestone 1.

## D110. clap for the server commands

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** `presswork-server` reads its subcommands and options with clap 4 and its derive macros.
- **Reason:** clap makes the help text and the error messages from the code. It is the standard parser for Rust programs.
- **Other options:** lexopt, a very small parser. A parser that we write ourselves.

## D111. UUID version 7 for record IDs

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Each record has a UUID version 7 as its ID. Presswork makes the IDs with the uuid crate, not the database. PostgreSQL keeps them in `UUID` columns.
- **Reason:** IDs of version 7 sort by time, so indexes stay compact. Offline clients can make IDs for new records (D2).
- **Other options:** Random UUIDs of version 4 from PostgreSQL. Numbers from the database.

## D112. jiff for all times

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The workspace uses jiff 1.0 for all times. The server keeps times in PostgreSQL through jiff-sqlx.
- **Reason:** One time library in the desktop application, the logs, and the server.
- **Other options:** The time crate for the server. The chrono crate for the server.

## D113. Argon2id with the parameters that OWASP recommends

- **Status:** Accepted, 2026-11-15. **Origin:** Default.
- **Decision:** The server hashes passwords with the `argon2` crate of the RustCrypto project. It uses Argon2id with 19 MiB of memory, 2 passes, and 1 lane. A password must have 12 characters or more. Server commands never receive a password. They make one-time links, and each person sets the password through the API.
- **Reason:** These are the parameters that OWASP recommends for Argon2id. Passwords do not go into the history of a shell.
- **Other options:** bcrypt. Commands that read a password from the terminal.

## D114. Permissions of the two roles in milestone 1

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** In milestone 1, an organization has two roles. Administrators can do all operations. Members can see all projects and tasks of the organization. They can make, edit, and assign tasks, and change the state of tasks. A member can delete only the tasks that the member made. Only administrators make and rename projects and make invite links.
- **Reason:** Each member can do the daily work. Administrators control the structure of the organization.
- **Other options:** All members make projects. All members delete all tasks. Only administrators delete tasks.

## D115. Due dates are calendar dates

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The due date of a task is a calendar date, for example 2026-11-23. It has no time and no time zone.
- **Reason:** A due date means the same day for each person. Most task tools operate in this way.
- **Other options:** A date and a time, shown in the time zone of each person.
