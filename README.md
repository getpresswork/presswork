# Presswork

Presswork is a typesetting and publication workbench for [Typst](https://typst.app). It has a desktop application for Windows, Linux, and macOS, and a server for publishing teams.

> **Status:** Presswork is in early development. It is not ready for production use. The functions below are the plan for the first releases.

## Functions

- **Typst workbench.** Edit Typst sources with a live preview, diagnostics, completion, and jumps between the source and the pages. Detach the preview to a second screen.
- **Toolchains.** Install Typst versions and pin one version for each project. Import manuscripts with Pandoc. Validate PDF/A and PDF/UA files with veraPDF.
- **Team library.** Keep works, editions, and files with OCR. Set the standard editions for quotations. Check each quotation against its edition.
- **Live collaboration.** Edit the same document with your team at the same time, with suggestion mode. Edit offline and sync later.
- **Production management.** Use tickets, tasks, schedules, roles, checklists, workflows, routines, budgets, and reports for books, periodicals, technical documentation, and academic publications.
- **Editorial tools.** Use spelling and house style checks, a termbase, back-of-book index tools, and a translation workspace.
- **Production.** Make PDF, EPUB, covers, ONIX metadata, and print-ready files. Control assets and clearances.
- **Typesetting tools.** Use the style panel, table and math builders, a QA report for bad breaks, soft proofs, and copyfitting. Lift charts and tables from PDF files into Typst.
- **Team chat.** Use channels, direct messages, and comments on each object, with links between all parts.
- **AI assistance.** Turn on AI tools and an assistant that only proposes changes, through Claude or a local model, with a policy, an audit log, and quotas. AI assistance is off by default.
- **Logs.** Find problems quickly with structured logs, build reports, and support bundles.
- **Workspaces.** Each role gets a clear layout: Plan, Research, Write, Translate, Review, Typeset, and Publish.
- **No lock-in.** A Presswork project is a normal Typst project.

## Documentation

Read the documentation at <https://getpresswork.github.io/presswork/>. It contains the design and the contribution guide.

## Build from source

1. Install Rust with [rustup](https://rustup.rs).
2. Clone the repository.
3. Run the desktop application:

   ```sh
   cargo run --package presswork
   ```

## Contribute

Read [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Presswork is available under the [Apache License 2.0](LICENSE).
