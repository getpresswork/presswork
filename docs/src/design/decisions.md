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

- **Status:** Accepted, 2026-11-14. Partially replaced by D157. **Origin:** Default.
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

## D116. Presswork draws its own top bar

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** On Windows and Linux, the windows of Presswork have no system title bar. Presswork draws the top bar, the minimize, maximize, and close buttons, and the resize edges. On Windows 11, the windows keep the shadow and the round corners of the system. The snap layouts of Windows 11 open with Win+Z, but not when the pointer is on the maximize button. On macOS, the traffic lights of the system stay. They float in the top bar of Presswork, because the system title bar is transparent and the content fills the full window.
- **Reason:** One bar instead of two gives more space to the document. The windows look the same on Windows and Linux. macOS users keep the controls that they know. genie uses the same design.
- **Other options:** Presswork also draws its own buttons on macOS. Only Windows gets the Presswork bar. The system title bars stay.

## D117. A top bar of 32 pixels

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The top bar is 32 pixels high. It contains the Presswork menu, the workspace tabs, the Open folder button, and the window buttons.
- **Reason:** 32 pixels is the usual height of a title bar on Windows and macOS. The traffic lights of macOS are almost at the vertical center of the bar. winit cannot move them down for a higher bar.
- **Other options:** 36, 40, or 44 pixels.

## D118. The Presswork menu

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The mark and the name at the left of the top bar open the Presswork menu. The menu shows the commands and their keys: Open folder, Save, Typst versions, Logs, and Quit. Escape or a click outside of the menu closes it.
- **Reason:** Windows and Linux have no menu bar for these commands. The menu shows the keys, thus users can learn them. genie uses the same design.
- **Other options:** The mark and the name only show the name of the application.

## D119. A small bar for the detached preview

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** The window of the detached preview has its own top bar of 32 pixels. The bar shows the title of the window and the window buttons.
- **Reason:** The two windows look the same. The preview window has no system title bar on Windows and Linux (D116).
- **Other options:** The preview window keeps the system title bar.

## D120. Ask before changes are lost

- **Status:** Accepted, 2026-11-15. **Origin:** Project owner.
- **Decision:** Before Presswork closes, it examines the open file. If the file has changes that are not saved, a dialog asks: Save, Do not save, or Cancel. Quit in the Presswork menu, Ctrl+Q, the close button, and the close command of the system (for example Alt+F4) show the dialog. If Presswork cannot save the file, it stays open and shows the problem. On macOS, Cmd+Q in the system menu ends Presswork at once in this release. winit makes this menu, and it does not let an application stop the quit.
- **Reason:** One click must not remove the work of the user.
- **Other options:** Presswork closes, and the changes are lost. Presswork saves the file without a question.

## D121. Details of the window frame

- **Status:** Accepted, 2026-11-15. **Origin:** Default.
- **Decision:** A double-click on an empty area of the top bar maximizes or restores the window. A right-click on an empty area opens the window menu of the system, on Windows and on Wayland. Each window button has a tooltip. The close button becomes red (#C42B1C) with a white symbol when the pointer is on it, as on Windows 11. When the window is maximized, the maximize button shows the restore symbol, and the resize edges are not available. The resize edges are 6 pixels wide. On Linux, a rule of 1 pixel surrounds the window when it is not maximized, because Linux desktops do not draw a shadow for a window without a system frame.
- **Reason:** These are the conventions of each system.
- **Other options:** The frame of genie, without the red close button, the restore symbol, and the window menu.

## D122. Workspace tabs in narrow windows

- **Status:** Accepted, 2026-11-15. **Origin:** Default.
- **Decision:** When the top bar does not have sufficient space for the labels of the workspace tabs, the tabs show only their icons. A tooltip shows the name of each tab. A window is 720 by 480 pixels or larger.
- **Reason:** The window buttons must always be visible. Each workspace stays one click away.
- **Other options:** Tabs that scroll. A larger minimum width, for example 1100 pixels.

## D123. Loro text documents with UTF-8 positions

- **Status:** Accepted, 2026-11-16. **Origin:** Default.
- **Decision:** The `presswork-sync` crate uses the `loro` crate, version 1. Each text file is one Loro document with one text container, `text`. Local edits go into the document with UTF-8 positions, the same unit as the text model. Remote changes come out of the document as a change set for the text model, with one edit for each changed area. Each loaded document gets a new random peer ID.
- **Reason:** D3 selected Loro. UTF-8 positions need no conversion. A change set with exact edits keeps the cursors and the undo history of the user correct.
- **Other options:** A comparison of the full text after each import, which is simple but moves the cursor of the user too much. Unicode scalar positions with a conversion.

## D124. MessagePack messages with rmp-serde

- **Status:** Accepted, 2026-11-16. **Origin:** Default.
- **Decision:** The `presswork-api` crate defines the live messages as serde types. The rmp-serde crate encodes them as MessagePack maps with field names. Loro updates and version vectors are MessagePack binary values.
- **Reason:** Field names let a newer peer add optional fields without a break. Binary values keep the Loro updates compact (D14).
- **Other options:** MessagePack arrays without field names, which are smaller but break when a field changes. CBOR.

## D125. Append-only storage of document updates

- **Status:** Accepted, 2026-11-16. **Origin:** Default.
- **Decision:** The server stores each accepted Loro update as one row of the `document_updates` table, with its author and its time. Milestone 1 does not compact the updates. For each open document, the server keeps a Loro document in memory. It loads the document from the rows when a client first subscribes.
- **Reason:** This is the simplest correct storage. Each change keeps its author for the history slider. Compaction into snapshots comes later (refer to Document storage in [Server](server.md)).
- **Other options:** One snapshot for each document, which loses the record of each change. A new snapshot after each change.

## D126. Documents of a project in milestone 1

- **Status:** Accepted, 2026-11-16. **Origin:** Default.
- **Decision:** A project has text documents. Each document has an ID and a path in the project, for example `main.typ`. A path is unique in its project. All members of the organization can make, read, and edit the documents of its projects.
- **Reason:** This is sufficient for one shared file (roadmap step 11). The file tree document of [Collaboration](collaboration.md) comes later, with the permissions for each file.
- **Other options:** The file tree document now. Only administrators make documents.

## D127. One server process for live sync in milestone 1

- **Status:** Accepted, 2026-11-16. **Origin:** Default.
- **Decision:** The server sends each update to the other subscribers through channels in its process. Milestone 1 supports one server process. PostgreSQL `LISTEN` and `NOTIFY` for many processes come later (refer to Events in [Server](server.md)).
- **Reason:** This is simple and fast for the first releases. The database stays the only durable state, so a later release can add more processes without a data migration.
- **Other options:** `LISTEN` and `NOTIFY` now.

## D128. The session token in the Hello message

- **Status:** Accepted, 2026-11-16. **Origin:** Default.
- **Decision:** The client opens the WebSocket without credentials. Its first message, `Hello`, contains the session token. The server closes the connection if the first message is not a valid `Hello` in 10 seconds.
- **Reason:** A token in a URL can go into the logs of proxies. Browsers cannot set the `Authorization` header for a WebSocket, and the web portal comes later.
- **Other options:** The `Authorization` header. A token in the query string.

## D129. tokio-tungstenite for the WebSocket client

- **Status:** Accepted, 2026-11-16. **Origin:** Default.
- **Decision:** The `presswork-client` crate uses tokio-tungstenite 0.29, the version that axum uses. TLS uses rustls with the aws-lc-rs provider and the certificate verifier of the operating system, as reqwest does in the toolchain manager.
- **Reason:** The dependency tree has one WebSocket implementation. All connections of the desktop application use the same TLS behavior.
- **Other options:** tokio-websockets. fastwebsockets.

## D130. Connect to a server from the Presswork menu

- **Status:** Accepted, 2026-11-16. **Origin:** Default.
- **Decision:** The Presswork menu has the item **Connect to a server…**. A dialog asks for the server address, the email address, and the password. After the sign-in, the dialog shows the projects of the organizations of the user, with their documents. Select a document to edit it live in the Write workspace.
- **Reason:** This is the shortest path for the thin slice (roadmap step 14). The home dashboard of milestone 2 will give the main entry point.
- **Other options:** A server panel at the side of the window. The home dashboard now.

## D131. The session token stays in memory until a credential store is selected

- **Status:** Accepted, 2026-11-16. **Origin:** Default.
- **Decision:** The desktop application keeps the session token in memory only. After a restart, the user signs in again. The token goes into the credential store of the operating system when the project owner selects a crate for the credential store.
- **Reason:** The crate for the credential store is a supply chain decision for the project owner. A token in a normal file is not safe.
- **Other options:** The `keyring` crate now. A file in the data folder.

## D132. A server document takes the place of the local text in the editor

- **Status:** Accepted, 2026-11-16. **Origin:** Default.
- **Decision:** When the user opens a server document, the editor and the preview show it instead of the local project or the scratch text. The preview compiles the document as one file. Each change set of the editor goes to the server at once. The status bar shows the path of the document and the state of the connection: connecting, live, or offline.
- **Reason:** Milestone 1 has one editor. Panes with many files, and server projects with many files, come in milestone 2.
- **Other options:** A second window for server documents.

## D133. One chat channel for each organization in milestone 1

- **Status:** Accepted, 2026-11-16. **Origin:** Default.
- **Decision:** Each organization has one chat channel. All members can read and write its messages. A message has plain text of 1 to 4000 characters, an author, and a time. The server stores the messages in PostgreSQL.
- **Reason:** This is the thin slice of roadmap step 13. Channels, direct messages, replies, and files come in milestone 3.
- **Other options:** One channel for each project. Markdown messages now.

## D134. Chat on the live WebSocket

- **Status:** Accepted, 2026-11-16. **Origin:** Default.
- **Decision:** The client sends `Join` with an organization ID on the live WebSocket. The server answers `Joined` with the last 100 messages. The client sends `Say` with the text and an ID. The server stores the message and sends `Said` with the stored message to all clients that joined, also to the author. The desktop application shares one live connection between the open document and the chat.
- **Reason:** One connection carries all live data (D14). `Said` to the author confirms the store, as `Ack` does for document updates.
- **Other options:** A second WebSocket for the chat. HTTP requests for new messages at intervals.

## D135. Tasks of the server project in the Plan workspace

- **Status:** Accepted, 2026-11-16. **Origin:** Default.
- **Decision:** When a server document is open, the Plan workspace shows the tasks of its project as a list: the title, the state, and the due date. A check box sets a task to done, or back to to do. A field at the top adds a task with a title. The list loads again each time the user selects the Plan workspace.
- **Reason:** Roadmap step 14 asks to complete tasks in the desktop application. The board, the timeline, and the calendar come in milestone 5.
- **Other options:** A Tasks panel at the bottom of the window. Live task events through the WebSocket now.

## D136. A small library in milestone 1

- **Status:** Accepted, 2026-11-16. Partially replaced by D146 and D147. **Origin:** Default.
- **Decision:** In milestone 1, the library of an organization keeps works. Each work has one edition and one PDF file. To add a work, a member types the title, the creators, the year, and the citation key. Then the member uploads the PDF file. The edition is the standard edition of the work for all purposes. Page maps, more editions, purposes, scopes, annotations, and research notes come in milestone 4.
- **Reason:** This is the thin slice of roadmap step 12. The tables follow D27 from the start. Thus milestone 4 adds editions and does not change the data.
- **Other options:** A flat list of references in milestone 1. All forms for works and editions now.

## D137. Library files in a local folder in milestone 1

- **Status:** Accepted, 2026-11-16. **Origin:** Default.
- **Decision:** The setting `data` gives a folder for the server. The server keeps the library files as blobs in its `blobs` folder, through the `object_store` crate (D23). The name of a blob is `library/<file ID>.pdf`. The database keeps the size and the SHA-256 digest of each file. The limit for one file is 64 MiB.
- **Reason:** A local folder is sufficient for small servers. S3-compatible services come with the operations work of milestone 3.
- **Other options:** S3 now. Blobs in PostgreSQL.

## D138. The text of library files from the text layer

- **Status:** Accepted, 2026-11-16. **Origin:** Default.
- **Decision:** After an upload, the server reads the text layer of each page with hayro, the PDF library that Typst uses. The server keeps the text of each page in PostgreSQL and adds it to the search index. A page without a text layer has no text until OCR is available. The OCR engine (D29) comes after the project owner approves the installation of OCRmyPDF and Tesseract.
- **Reason:** Many PDF files already have a text layer. hayro is already in the dependency tree. OCRmyPDF and Tesseract are downloads outside crates.io.
- **Other options:** OCR on all pages. A different PDF crate, for example `lopdf`.

## D139. A search index that the server can make again

- **Status:** Accepted, 2026-11-16. **Origin:** Default.
- **Decision:** The server keeps a Tantivy index (D31) of the library pages in the `index` folder of its `data` folder. Each entry has the organization, the work, the page number, and the text. If the index is empty when the server starts, the server makes the index again from PostgreSQL. A search returns the work, the page number, and the text around each match. The Tantivy build has no zstd compression and no stemmer.
- **Reason:** PostgreSQL is the only source of the data. Thus a lost index is not a loss of data. zstd adds C code to the build.
- **Other options:** The full-text search of PostgreSQL in milestone 1. An index that is the only copy of the text.

## D140. Upload in two requests

- **Status:** Accepted, 2026-11-16. Partially replaced by D143. **Origin:** Default.
- **Decision:** The client sends the data of a new work as JSON: `POST /api/v1/organizations/{id}/works`. Then it sends the PDF file as the body of a second request: `PUT /api/v1/works/{id}/file`. The second request returns when the server has read the text of all pages.
- **Reason:** The API stays JSON for data, and the server needs no multipart parser. The text is ready for search when the upload is complete.
- **Other options:** One multipart request. A background job for the text, with a state that the client gets at intervals.

## D141. Citations from the Research workspace

- **Status:** Accepted, 2026-11-16. **Origin:** Default.
- **Decision:** When a server document is open, the Research workspace shows the library of its organization: the works, an **Add PDF…** button, and a search field. Each search result has a **Cite** button. **Cite** opens the Write workspace and puts `@key[p. N]` at the cursor. N is the page in the PDF file, because page maps come in milestone 4. If the document has no `#bibliography`, Presswork adds `#bibliography("library.yml")` at the end. The desktop application makes `library.yml` in the Hayagriva format from the works of the library, and gives it to the Typst engine as a file next to the document.
- **Reason:** A citation compiles at once after one click. The format follows "Citations in Typst" in [Library](library.md).
- **Other options:** The user types the bibliography call. A `library.yml` document on the server.

## D142. A job queue in PostgreSQL

- **Status:** Accepted, 2026-11-16. **Origin:** Project owner.
- **Decision:** The server keeps background jobs in the `jobs` table. A job has a kind, a subject (the ID of an object), a state, a number of attempts, and a time from which it can run. Two worker tasks in the server process take jobs with `FOR UPDATE SKIP LOCKED`. A worker holds a job for 10 minutes. If the worker stops, a different worker takes the job after that time. A new job wakes the workers at once. The workers also look for jobs every 5 seconds.
- **Reason:** This follows the routine engine in [Server](server.md). The jobs stay in the database when the server stops. Separate worker processes can come with OCR and exports.
- **Other options:** A separate `presswork-server worker` command. Jobs in memory only.

## D143. Upload with a quick check, then a job for the text

- **Status:** Accepted, 2026-11-16. **Origin:** Project owner.
- **Decision:** `PUT /api/v1/works/{id}/file` opens the PDF file and counts its pages. If the file is not a PDF file or is encrypted, the server refuses it at once. Otherwise the server keeps the file with the state `reading`, adds a job, and answers 202 with the work. The job reads the text of the pages, keeps it, adds it to the search index, and sets the state `ready`. This replaces the part of D140 where the request waits for the text.
- **Reason:** A large file does not hold the request. A wrong file still gets a message at once.
- **Other options:** All checks in the job, so that a wrong file shows the state `failed` some seconds later.

## D144. Three attempts for a job

- **Status:** Accepted, 2026-11-16. **Origin:** Project owner.
- **Decision:** A job has 3 attempts. After a temporary error, for example a stopped database, the job runs again after 10 seconds, and then after 1 minute. A file that Presswork cannot read fails at once. A failed file has the state `failed` and a reason that the list of works shows.
- **Reason:** A short stop of the database does not leave a file without text.
- **Other options:** One attempt.

## D145. WorkChanged on the live WebSocket

- **Status:** Accepted, 2026-11-16. **Origin:** Project owner.
- **Decision:** When the state of a library file changes, the server sends `WorkChanged` with the work to all clients that joined the organization (D134). The desktop application updates its list of works at once. The protocol version becomes 2.
- **Reason:** One connection carries all live data (D14). The desktop does not have to ask again.
- **Other options:** The desktop gets the list every 3 seconds. The list changes only when the user opens the Research workspace again.

## D146. Many editions with one standard edition

- **Status:** Accepted, 2026-11-16. **Origin:** Project owner.
- **Decision:** A work can have many editions. An edition has a year, a publisher, a language, translators, and its own PDF file. One edition of each work is the standard edition of the organization. The bibliography and **Cite** use the standard edition. The first edition of a work is its standard edition until a member selects a different one. Purposes, and the scopes of collections and publications (D28), come later. This replaces the part of D136 with one edition for each work.
- **Reason:** Teams have translations and reprints of one work now. The organization is the only scope that exists in milestone 1.
- **Other options:** Standard editions with purposes and scopes now (D28). Page maps only, with one edition for each work.

## D147. Page maps from the page labels of the PDF file

- **Status:** Accepted, 2026-11-16. **Origin:** Project owner.
- **Decision:** When the server reads a file, it also reads the page labels of the PDF file, for example `xii` or `412`, and keeps them as the page map of the edition. A member can correct a range of pages, for example "PDF page 15 is printed page 1", with Arabic or Roman numbers. Search results and **Cite** use the printed page numbers. A page without a label uses its PDF page number.
- **Reason:** Many PDF files from publishers contain page labels. A correction repairs the files that do not.
- **Other options:** A page map by hand only. Page numbers from OCR of the headers and footers.

## D148. Limits for sign-in attempts

- **Status:** Accepted, 2026-11-16. **Origin:** Project owner.
- **Decision:** After 5 failed sign-ins for one email address in 15 minutes, the server refuses sign-ins for that address for 15 minutes. One client address can make 20 failed sign-ins in 15 minutes. A refused sign-in gets the status 429 and the time to wait. The server keeps the counts in memory, because milestone 1 has one server process (D127). The client address is the address of the connection. Behind a reverse proxy, the setting `trust_forwarded` makes the server use the last address in the `X-Forwarded-For` header.
- **Reason:** This stops password guessing on one account and from one address. A wrong password of the real user does not lock the account for a long time.
- **Other options:** The counts in PostgreSQL for many server processes. A delay that doubles after each failed sign-in.

## D149. Personal access tokens

- **Status:** Accepted, 2026-11-16. **Origin:** Project owner.
- **Decision:** A user can make personal access tokens for scripts and CI. A token has a name and an optional expiry date, and it has the rights of the user. The server shows the token one time and keeps only its SHA-256 hash, as for sessions. The user can list and revoke the tokens. A token starts with `pwt_`, so that secret scanners can find it. The API comes first, and a screen in the desktop application comes later.
- **Reason:** Scripts must not use a password or a session of the desktop application. A hash in the database is not useful to a thief.
- **Other options:** Tokens with scopes. No tokens in milestone 1.

## D150. Details of the sign-in limits

- **Status:** Accepted, 2026-11-17. **Origin:** Default.
- **Decision:** The sign-in limits of D148 also count the sign-ins that are in progress. All IPv6 addresses of one /64 network count as one client address. The server keeps the count of an email address as a SHA-256 hash of the email address in lowercase. Each table keeps 100,000 counts or fewer. When a table is full of current counts, a new email address or client address has no count until old counts expire. A correct sign-in removes the failed sign-ins of its email address, but not of its client address.
- **Reason:** Without the count of the sign-ins in progress, many requests at the same time can go past the limit, and each one uses much memory for Argon2. One IPv6 user frequently has a full /64 network. The hash and the maximum size keep the memory small when a client sends many long or different email addresses. An attacker who knows one password must not be able to remove the count of a client address.
- **Other options:** A lock while a sign-in for the same email address is in progress. A full table that refuses all new sign-ins, which lets an attacker stop all sign-ins.

## D151. Details of personal access tokens

- **Status:** Accepted, 2026-11-17. **Origin:** Default.
- **Decision:** Only a session can make a personal access token (D149). A request with a token can use all other operations of the user, also the list and the revocation of tokens. A new password revokes all tokens of the user, as it ends all sessions. A user can have 100 tokens or fewer. A token name has 1 to 100 characters without control characters. An expiry time must be in the future. The server records the time of the last use of a token, with a precision of one minute.
- **Reason:** A stolen token must not make new tokens that stay after the user revokes the stolen token. A password reset is the response to a stolen account, and milestone 1 does not send an email when a token is made, so the user possibly does not see a token of an attacker. The maximum count stops a script that makes tokens in a loop.
- **Other options:** A new password that keeps the tokens, so that CI continues to operate. Tokens that can make tokens. No maximum count.

## D152. OCR on the server for the pages without a text layer

- **Status:** Accepted, 2026-11-17. **Origin:** Project owner.
- **Decision:** After the job reads the text layer of a file, a second job runs OCRmyPDF with Tesseract (D29) on the pages without a text layer (`--skip-text`). The file has the state `recognizing` until OCR ends. The server keeps the copy with the new text layer next to the original file. The original file does not change. If OCR fails, or if the server has no OCR engine, the file gets the state `ready`, and the pages without a text layer have no text.
- **Reason:** Many library files are scans. A text layer from the publisher is usually better than OCR, so OCR does not replace it.
- **Other options:** OCR only when a member starts it. OCR of all pages again with `--redo-ocr`.

## D153. The OCR language from the edition

- **Status:** Accepted, 2026-11-17. **Origin:** Project owner.
- **Decision:** The server changes the language of the edition, for example `de` or `de-AT`, into the Tesseract language, for example `deu`. If the server has no Tesseract data for that language, OCR uses the languages of the setting `ocr_languages`. The default is `eng`.
- **Reason:** The language data has a large effect on the accuracy of OCR, and each edition already has its language.
- **Other options:** One server setting for all files. The language of the edition together with English.

## D154. The ocrs engine without a desktop screen in milestone 1

- **Status:** Accepted, 2026-11-17. **Origin:** Project owner.
- **Decision:** The `presswork-ocr` crate contains the ocrs engine (D29) behind the `OcrEngine` trait, in the cargo feature `ocrs`. The server does not compile it. The two model files of ocrs have fixed SHA-256 digests. Local libraries (milestone 4) and the command-line tool (milestone 2) use the engine later. Then the desktop application downloads the models from the ocrs model host at the first use, after the user approves, checks the digests, and keeps the models in its data folder, as for Typst toolchains.
- **Reason:** The engine and its tests are ready for local libraries. Milestone 1 gets no new screens before its release.
- **Other options:** OCR on the desktop before the upload. The command-line tool now. The models in the installers. A copy of the models in the releases of Presswork.

## D155. The real OCR engines in tests

- **Status:** Accepted, 2026-11-17. **Origin:** Project owner.
- **Decision:** The OCR tests use the real engines. They run only when the variables `PRESSWORK_TEST_OCRMYPDF` and `PRESSWORK_TEST_OCRS_MODELS` are set, as the database tests do (D105). The CI job "OCR tests" installs OCRmyPDF and the English Tesseract data from the Ubuntu 24.04 archive. It also downloads the ocrs models, checks their digests, and keeps them in the CI cache. On the computer of the project owner, scoop installs Tesseract and Ghostscript, and pip installs OCRmyPDF in a virtual environment.
- **Reason:** A fake engine cannot find the problems of the real tools. Without the tools, all other tests still run.
- **Other options:** The real engines only in CI. A fake engine only.

## D156. Details of OCR on the server

- **Status:** Accepted, 2026-11-17. **Origin:** Default.
- **Decision:** OCRmyPDF runs with `--output-type pdf` and `--optimize 0`, so that the copy keeps the images and the structure of the original file. Each run has a time limit of one hour. A worker extends the lease of its job (D142) while the job runs, so that a long OCR job does not start two times. Each run uses the processors of the server divided by the number of workers. A problem of the file, for example an encrypted or signed file, stops OCR at once. Other problems get three attempts (D144). After OCR fails, the file shows the problem, and its pages keep the text of the text layer. A member can correct the page map while OCR runs. `tesseract` must be on the `PATH` of the server. The setting `ocrmypdf` gives the OCRmyPDF program, and an empty value turns OCR off.
- **Reason:** A library file is a reference copy, so OCR must not change its images. A scan of many hundreds of pages can take more than the lease of ten minutes.
- **Other options:** PDF/A output. Optimized images. A time limit for each page only.

## D157. Installers from the standard tool of each platform

- **Status:** Accepted, 2026-11-17. **Origin:** Project owner.
- **Decision:** Scripts in `.github/scripts` and templates in `packaging` make the installers with the standard tool of each platform. WiX 6 makes the MSI files for x64 and Arm64. `hdiutil` makes the DMG files. `dpkg-deb` and `rpmbuild` make the DEB and RPM packages. `appimagetool` makes the AppImage files. This decision replaces the `cargo-packager` part of D25.
- **Reason:** `cargo-packager` 0.11.8 uses WiX 3.11. WiX 3.11 cannot make Arm64 MSI files, and WiX 3 got its last security correction in 2025-02. `cargo-packager` makes no RPM packages, and it downloads the AppImage tools without a check of their digests.
- **Other options:** `cargo-packager` without changes, with only a ZIP archive for Windows on Arm and no RPM packages. `cargo-packager` with pinned tools in its cache folder, and `cargo-generate-rpm` for the RPM packages.

## D158. Downloads of the installer tools in CI

- **Status:** Accepted, 2026-11-17. **Origin:** Project owner.
- **Decision:** Only CI downloads the installer tools. WiX 6.0.2 and its UI extension come from NuGet. The script compares each package with the SHA-512 digest that NuGet publishes. `appimagetool` 1.9.1 and the AppImage runtime 20251108 come from their GitHub releases. The script compares each file with the SHA-256 digest that GitHub shows for the release file. `rpmbuild` comes from the Ubuntu archive. A file with a different digest stops the build.
- **Reason:** The tools run in the release workflow, so a changed tool could change the packages. WiX 6 asks projects that earn money with WiX to pay its Open Source Maintenance Fee. WiX 7 makes this a condition of use.
- **Other options:** No AppImage files. NSIS installers in place of MSI files, with NSIS 3.10 from the Windows images of the runners.

## D159. An MSI installation for all users

- **Status:** Accepted, 2026-11-17. **Origin:** Project owner.
- **Decision:** The MSI file installs Presswork for all users of the computer, in the `Program Files` folder. The installation asks for administrator rights.
- **Reason:** This is the usual scope of MSI files. Administrators can deploy the MSI file to many computers.
- **Other options:** An installation for the current user in `AppData\Local\Programs`, without administrator rights.

## D160. The pilcrow icon

- **Status:** Accepted, 2026-11-17. **Origin:** Project owner.
- **Decision:** The application icon is a claret pilcrow on a paper tile with a claret outline. The source is `crates/presswork/icons/presswork.svg`. The pilcrow is optically centered: it is 20 units right of the geometric center and 28 units below it. The ignored test `render_icons` writes the PNG, ICO, and ICNS files from the SVG file. A test compares these files with new renders.
- **Reason:** The pilcrow is a known sign of typesetting. The colors are the accent color and the page color of the design system.
- **Other options:** A slab-serif P on a claret tile. A page with a claret drop cap and lines of text.

## D161. Details of the installers

- **Status:** Accepted, 2026-11-17. **Origin:** Default.
- **Decision:**
  - The application ID is `io.github.getpresswork.Presswork`. It is the name of the desktop file on Linux, the Wayland application ID and the X11 class of the windows, and the bundle identifier on macOS. It agrees with the data folders of the application.
  - The MSI file shows the Apache License in the WixUI_Minimal dialogs. It has a fixed upgrade code, and an installation of the same version replaces the earlier one, because nightly builds have the version of the next release. A Start menu shortcut opens Presswork.
  - The macOS app has an ad hoc signature. The DMG file has a link to the Applications folder. The macOS icon is smaller than the other icons, as the macOS icon grid tells (824 of 1024 pixels).
  - `dpkg-shlibdeps` finds the dependencies of the DEB package. The RPM package has no debug package. The AppImage file has no AppStream data.
  - The windows show the 64-pixel icon. The Windows program file has no icon resource yet.
  - The build workflow checks each package on the runner that made it. It installs and removes the MSI file and the DEB package, opens the DMG file, lists the files of the RPM package, and unpacks the AppImage file.
- **Reason:** These are the usual settings of the platforms. The checks find a broken package before a release.
- **Other options:** A Developer ID signature, which needs a certificate (D25). AppStream data, which needs a review of its text.

## D162. A container image and an archive for the server

- **Status:** Accepted, 2026-11-17. **Origin:** Project owner.
- **Decision:** The server has a container image and an archive for Linux on x86_64 and aarch64. The image uses Ubuntu 24.04 at a fixed digest, with OCRmyPDF and Tesseract from the Ubuntu archive. The archive contains the program, an example settings file, a systemd unit, and the Docker Compose file. Dependabot opens update requests for the base image and for the PostgreSQL image of the Compose file.
- **Reason:** The CI jobs that build the server and test OCR use Ubuntu 24.04 (D155). Thus the image has the same glibc and the same versions of OCRmyPDF, Tesseract, and Ghostscript as the tests.
- **Other options:** Debian 13 slim, with a newer OCRmyPDF than the version that CI tests. A distroless image without OCR.

## D163. Server images in GHCR

- **Status:** Accepted, 2026-11-17. **Origin:** Project owner.
- **Decision:** The release workflow and the nightly workflow publish the image `ghcr.io/getpresswork/presswork-server`. A release gets the tags of its version, of its minor version, and `latest`, for example `0.1.0`, `0.1`, and `latest`. A prerelease version gets only its version tag. The nightly workflow uses the tag `nightly`. Each Linux runner of the build workflow builds and checks the image of its architecture, and pushes it with the architecture in the tag. Then a job joins the two images into one image for the two platforms.
- **Reason:** GHCR is with the repository and the releases, and the workflow token can push to it. Thus no new secret is necessary. The native runners need no emulation, and the published images are the images that the checks used.
- **Other options:** GHCR with images only for releases. Docker Hub too, which needs an account and a token as a repository secret.

## D164. Only English OCR data in the server image

- **Status:** Accepted, 2026-11-17. **Origin:** Project owner.
- **Decision:** The server image contains the Tesseract data only for English. For more languages, an administrator makes an image from the Presswork image. The deployment section of the server chapter shows an example for German.
- **Reason:** This keeps the image small. A library uses only some languages.
- **Other options:** Eight common languages, which add approximately 100 MB. All languages from `tesseract-ocr-all`, which make the image much larger.

## D165. Downloads for the server image in CI

- **Status:** Accepted, 2026-11-17. **Origin:** Project owner.
- **Decision:** Only CI downloads the images and the packages for the server image. The Dockerfile gets Ubuntu 24.04 by its digest. apt gets OCRmyPDF and the English Tesseract data from the Ubuntu archive, and checks the signatures of the archive. The check of the image starts PostgreSQL 17 at the digest of the CI database tests. The computer of the project owner downloads nothing for the image.
- **Reason:** The image goes into the releases. Thus its sources must be fixed and checked, as the sources of the installer tools are (D158).
- **Other options:** The same downloads also on the computer of the project owner, to test the image with Docker Desktop.

## D166. Details of the server packages

- **Status:** Accepted, 2026-11-17. **Origin:** Default.
- **Decision:**
  - The image starts `presswork-server serve` as the user `presswork`, with the user ID and the group ID 10001. In the container, the server listens on port 8080 on all addresses, and keeps its data in the volume `/var/lib/presswork`.
  - The server stops on `SIGTERM` as it does on Ctrl+C. Without this, the server is the first process of the container and ignores `SIGTERM`, and `docker stop` kills it after 10 seconds.
  - The Compose file gets the database password from the file `.env`. It publishes the port of the server only on `127.0.0.1` of the host, for a reverse proxy. Its default image has the tag `latest`.
  - The systemd unit starts the server as the user `presswork`. The system folders are read-only for the server, and it has a private temporary folder. systemd makes the data folder.
  - The build workflow checks the archive and the image on the runner that made them. The image check starts the image with PostgreSQL through the Compose file.
  - The build workflow builds all Linux packages on Ubuntu 24.04, not on `ubuntu-latest`. A program that is built on a newer system needs a newer glibc. For example, the x86_64 server from `ubuntu-latest` needed glibc 2.43 and did not start in the image.
- **Reason:** These are the usual settings of container images and systemd services. The checks find a broken image before a release.
- **Other options:** The port of the server on all addresses of the host. A Compose file with a reverse proxy and TLS.

## D167. A process group for each OCR run

- **Status:** Accepted, 2026-11-18. **Origin:** Default.
- **Decision:** On Linux and macOS, the server starts OCRmyPDF in a new process group. At the time limit (D156), it stops all processes of the group with `SIGKILL`. A `kill` command of `sh` sends the signal, because the workspace forbids `unsafe` code. On Windows, the server stops only the OCRmyPDF process. In a terminal, Ctrl+C stops the server, but an OCR run that has started continues until it ends. A container or a systemd service stops all processes of the server.
- **Reason:** OCRmyPDF starts worker processes, and the workers start Tesseract and Ghostscript. Before this decision, only the OCRmyPDF process stopped at the time limit. The other processes continued with a hostile file and used the processors of the server.
- **Other options:** A list of the child processes from `/proc`, which is only for Linux. The `rustix` crate for the signal, which is a new direct dependency.

## D168. Dependabot updates only the digests of the server images

- **Status:** Accepted, 2026-11-18. **Origin:** Default.
- **Decision:** Dependabot opens update requests for new digests of `ubuntu:24.04` in the Dockerfile and of `postgres:17-alpine` in the Compose file. It ignores new Ubuntu releases and new major versions of PostgreSQL.
- **Reason:** The image uses the Ubuntu release of the CI jobs (D162). A new major version of PostgreSQL cannot open the data folder of the previous version. Thus a change of the Compose file to a new major version needs a migration procedure first.
- **Other options:** All version updates, with a review of each request.

## D169. typst-ide for the editor functions of the embedded engine

- **Status:** Accepted, 2026-11-18. **Origin:** Project owner.
- **Decision:** The embedded engine uses the `typst-ide` crate of the Typst project for the jumps between source and pages, completion, tooltips, and go to definition. It has the same version as the other Typst crates. Each compilation keeps a copy of the world and of the document. The editor functions use this copy. Thus they do not wait for a compilation that runs.
- **Reason:** The Typst web app and the Typst language servers use the same code. All dependencies of `typst-ide` are already in the workspace through the other Typst crates, so it adds only one crate.
- **Other options:** Code of our own for each function, on the syntax tree and the document.

## D170. A click on a web link in the preview goes to the source

- **Status:** Accepted, 2026-11-18. **Origin:** Project owner.
- **Decision:** A click on the text of a web link in the preview moves the cursor to that text, as a click on other text does. A click on a link to a label or to a position scrolls the preview to the target of the link.
- **Reason:** The preview is a proof. A click on its text always goes to the source.
- **Other options:** Open the address in the browser of the system. Show the address in a note.

## D171. Details of the jumps between source and pages

- **Status:** Accepted, 2026-11-19. **Origin:** Default.
- **Decision:**
  - The preview follows the cursor only when the cursor moves and the text does not change.
  - The mark is a ring in the accent color with a see-through fill, so that the marked characters stay readable. It shows for 1.5 seconds.
  - The preview scrolls only when the position is not in the middle two thirds of the visible part. Then the position goes to one third of the height.
  - After a jump to the source, the editor gets the keyboard focus, and the preview marks the new cursor position.
  - A click on text of another file shows a note with the name of the file, because the editor shows only the main file.
  - The jumps use the positions of the last compilation that gave pages, also when the text changed after it. The cursor goes to the nearest valid position.
- **Reason:** The preview does not move while you type, and a short mark does not hide the text. A jump that is a few characters wrong is more useful than no jump while the text has an error.
- **Other options:** Follow the cursor also while you type. A mark that stays until the next move. No jumps while the text is different from the compilation.

## D172. Details of tooltips and completion

- **Status:** Accepted, 2026-11-20. **Origin:** Default.
- **Decision:**
  - A tooltip shows after the mouse pointer stays on a glyph for 500 ms. It closes when the pointer moves, at a key, at a click, and at a turn of the mouse wheel.
  - After each typed character, the editor asks for the completions at the cursor. `typst-ide` gives completions only where they apply, for example after `#`. Ctrl+Space asks also in text, on all platforms. A list that Ctrl+Space opened stays open while you type.
  - The list shows the items that start with the typed text, without a difference between capital and small letters. It shows eight rows. A box below the list gives the description of the selected item.
  - The editor anchors the list at the start of the typed text. Thus the list does not move while you type.
  - Tooltips and completions run on a separate thread. A result that comes after the text or the cursor changed is not used.
  - They use the result of the compilation that gave the shown pages, because its document gives the labels. If there are no pages, they use the last compilation.
- **Reason:** These are the usual settings of code editors. A thread keeps typing fast, because a completion can compile the document again.
- **Other options:** A fuzzy match of the typed text. A delay of one second for tooltips. Completions only on request.

## D173. Details of go to definition

- **Status:** Accepted, 2026-11-20. **Origin:** Default.
- **Decision:**
  - F12 and Ctrl+click ask for the definition of the item. On macOS, Cmd+click replaces Ctrl+click.
  - The engine examines the character after the cursor, and then the character before it. Thus the cursor at the end of a name also finds its definition.
  - A definition in the main file selects its name. A definition in another file or in the Typst library shows a note.
  - The request runs on the separate thread of the editor functions (D172).
- **Reason:** These are the usual keys of code editors. The editor shows only the main file, so a note is the only possible answer for other files.
- **Other options:** Open the documentation of library items on the Typst website. Alt+click in place of Ctrl+click.

## D174. Labels in the completion of references

- **Status:** Accepted, 2026-11-20. **Origin:** Project owner.
- **Decision:** The engine gives all labels of the document with the kind, the text, and the page of their elements. The completion list uses them: after `@`, the description of each label gives the kind of the element, its page, and its text, for example "Figure, page 4: A cat". The page is the printed page number if the page has a numbering pattern. Otherwise, it is the position of the page in the document.
- **Reason:** A reference must go to the correct element. The kind and the page help you to select the correct label. F12 on a reference already goes to the element (D173).
- **Other options:** Also list the labeled figures, tables, and equations in the outline, below their headings. A separate list of all labels with a filter, which puts a reference at the cursor.

## D175. Details of the outline

- **Status:** Accepted, 2026-11-20. **Origin:** Default.
- **Decision:**
  - The outline panel is at the left of the editor, as in the layout of the Write workspace. It is open by default and 260 pixels wide. The **Outline** button in the status bar opens or closes it.
  - The outline shows all headings of the last compilation that gave pages. This includes the headings of other files and the headings that are not in the table of contents (`outlined: false`).
  - Each heading shows its number, its text, and its page (D174). The indent of each level is 12 pixels.
  - The heading that contains the cursor has the accent color.
  - A click on a heading moves the cursor to the start of the heading, gives the keyboard focus to the editor, and marks the heading in the preview. For a heading in another file, the preview shows the heading, and a note gives the name of the file.
  - With the managed toolchain, the outline shows a note, because it needs the document model.
- **Reason:** The outline shows the structure of the document as the reader sees it, also when code or other files make the headings. The page numbers help typesetters to find the pages.
- **Other options:** Only the headings of the table of contents. The headings of the source text of the main file, without page numbers. A panel that is closed by default.

## D176. The regex crate for find and replace

- **Status:** Accepted, 2026-11-20. **Origin:** Project owner.
- **Decision:** The search of the editor uses the `regex` crate, also for plain text: the search escapes the signs of plain text. The text model (`presswork-text`) contains the search, so that it does not need iced.
- **Reason:** Typst already uses `regex` with the same features, so the build does not change. One engine gives the same rules for case, words, and lines in all modes.
- **Other options:** A search of our own for plain text, and `regex` only for patterns. The `fancy-regex` crate, which also has look-around but can be slow.

## D177. Details of find and replace

- **Status:** Accepted, 2026-11-20. **Origin:** Default.
- **Decision:**
  - Ctrl+F opens the find bar above the editor, and Ctrl+H opens it with the replace row. On macOS, Cmd+F and Cmd+Option+F, because Cmd+H hides the application. Selected text on one line becomes the query.
  - The search ignores the case by default. **Whole word** accepts a result only if the characters before and after it are not letters, digits, or underscores. A regular expression matches `^` and `$` at each line. A pattern that matches no characters gives no results.
  - All results have the find color as background, below the selection. The selected result has the selection color. The bar shows "2 of 8", or "8 results" if the selection is not a result.
  - While you type the query, the editor selects the first result at or after the start of the selection. Enter and F3 go to the next result, and Shift+Enter and Shift+F3 to the previous result. The search goes around at the end.
  - **Replace** changes the selected result and selects the next result. If the selection is not a result, it only goes to the next result. **Replace all** is one undo step. In pattern mode, `$1` and `${name}` give the groups.
  - The search runs again after each change of the text or of the query. Escape closes the bar and focuses the editor.
- **Reason:** These are the usual keys and functions of code editors and word processors.
- **Other options:** Find in all files of the project. A search only in the selected text. A list of all results in a panel.

## D178. Details of many cursors

- **Status:** Accepted, 2026-11-20. **Origin:** Default.
- **Decision:**
  - Alt+click adds a cursor, and Alt+click on a cursor removes it. The last cursor stays. On macOS, Option+click.
  - Ctrl+Alt+Up and Ctrl+Alt+Down add a cursor above the first cursor or below the last cursor, at the same character column of the text line. On macOS, Cmd+Option+Up and Cmd+Option+Down.
  - Ctrl+D (Cmd+D on macOS) from a cursor selects the word, and then adds the next place of the same full word. From another selection, it adds the next place of the same text, also inside longer words. Ctrl+Shift+L selects all places. The search uses the case of the text, goes around at the end, and skips the places that are already selected.
  - The cursor that you added last is the primary cursor. Escape keeps only the primary cursor.
  - With many cursors, Up and Down move each cursor by one row, and each cursor keeps its own column.
  - A paste with one line for each cursor gives each cursor one line.
  - Completion, the find bar, and go to definition use the primary cursor.
- **Reason:** These are the keys of Visual Studio Code, which many writers of Typst use. Text lines and character columns do not change when the width of the editor changes.
- **Other options:** Alt+drag for a column of cursors. Shift+Alt+Up and Shift+Alt+Down on Linux, where some desktops use Ctrl+Alt+Up and Ctrl+Alt+Down. Find text without a difference between capital and small letters.
