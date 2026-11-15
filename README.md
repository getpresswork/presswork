# Presswork

Presswork is a typesetting and publication workbench for [Typst](https://typst.app). It has a desktop application for Windows, Linux, and macOS, and a server for publishing teams.

> **Status:** Presswork is in early development. It is not ready for production work. The functions below are the plan for the first releases.

## Functions

- **Typst workbench.** Edit Typst sources with a live preview, diagnostics, completion, and jumps between the source and the pages.
- **Toolchains.** Install Typst versions and pin one version for each project. Import manuscripts with Pandoc. Validate PDF/A and PDF/UA files with veraPDF.
- **Live collaboration.** Edit the same document with your team at the same time. Work offline and sync later.
- **Production management.** Use issues, tasks, roles, checklists, workflows, routines, and reports for books, periodicals, technical documentation, and academic work.
- **Workspaces.** Each role gets a clear layout: Write, Typeset, Review, Plan, and Publish.
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
