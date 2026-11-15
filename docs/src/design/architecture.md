# Architecture

This chapter shows the parts of Presswork and how they connect.

## System context

```text
+-----------------------+   HTTP and WebSocket   +----------------------+
| Presswork desktop     | <--------------------> | Presswork server     |
| Windows, Linux, macOS |                        | Linux or container   |
+-----------+-----------+                        +-----+----------+-----+
            |                                          |          |
            v                                          v          v
  Local project folders                          PostgreSQL   Blob storage
  Typst toolchains
  Typst package cache

External services:
  Typst Universe ........ Typst packages
  GitHub releases ....... Typst and Pandoc toolchains
  veraPDF releases ...... PDF validation
  Eclipse Temurin ....... Java runtime for veraPDF
```

The desktop application and the server are two programs. Both use the same crates for the Typst engine, the domain model, the sync protocol, and the API types.

## Layers

The code has four layers. A layer can use the layers below it. A layer cannot use the layers above it.

| Layer | Contents | Rules |
| --- | --- | --- |
| Applications | Desktop application, server | Connect the parts. Contain no business rules. |
| Adapters | File system, network, database, Typst engine, widgets | Do the input and output. Implement the ports of the services. |
| Services | Toolchain manager, sync engine, workflow engine, routine engine | Use ports (traits) for all input and output. |
| Domain | Text model, project model, domain model, API types | Pure code. No input and output. No async code. |

This structure gives fast tests. Most tests use the domain and service layers with fake adapters.

## Crates

All crates are in the `crates` folder. Each crate has one purpose. This table shows the planned crates. A crate comes into the repository with its first code.

| Crate | Layer | Purpose |
| --- | --- | --- |
| `presswork` | Application | The desktop application, with iced. |
| `presswork-server` | Application | The collaboration server, with axum and PostgreSQL. |
| `presswork-ui` | Adapter | The design system: theme, design tokens, and shared widgets. |
| `presswork-editor` | Adapter | The text editor widget. |
| `presswork-preview` | Adapter | The page preview widget. |
| `presswork-typst` | Adapter | The embedded Typst engine: compilation, page images, export, and IDE functions. |
| `presswork-client` | Adapter | The HTTP and WebSocket client for the server API. |
| `presswork-toolchain` | Service | Installs and selects Typst, Pandoc, veraPDF, and Java versions. Controls packages and fonts. |
| `presswork-sync` | Service | Connects the text model to Loro documents and sends updates. |
| `presswork-text` | Domain | Text buffer, positions, edits, selections, and history. |
| `presswork-project` | Domain | The project manifest and the project file tree. |
| `presswork-domain` | Domain | Organizations, roles, publications, tickets, tasks, checklists, workflows, routines, and reports. |
| `presswork-api` | Domain | The request, response, and event types of the server API. |

## Data flow

### From a keystroke to the preview

1. The editor widget changes the text buffer with a transaction.
2. The application sends the edits to the Typst engine.
3. The engine updates its overlay for the file and starts a compilation on its worker thread.
4. The engine renders the changed pages to images.
5. The preview shows the new images. The diagnostics panel shows the new errors and warnings.

If you make more edits during a compilation, the engine does one more compilation after the current one.

### From a keystroke to the team

1. The sync engine applies the same transaction to the Loro document of the file.
2. The client sends the Loro update to the server through the WebSocket.
3. The server makes sure that the user has write permission. Then it stores the update.
4. The server sends the update to all other clients that have the document open.
5. Each client applies the update to its Loro document and to its text buffer.

## Threads and tasks

- The iced runtime runs the `update` and `view` functions on the main thread. These functions must return quickly.
- Long operations run in iced tasks on the Tokio runtime.
- The Typst engine compiles on one dedicated thread, because the Typst compiler is synchronous.
- The engine renders pages on a thread pool.
- The server runs Typst compilations in separate worker processes. Thus a slow document cannot stop the server.

## Errors

- Library crates make their error types with `thiserror`.
- The applications show errors to the user in STE. Each message tells what occurred and what to do.
- Code outside of tests does not use `unwrap` or `expect`. Clippy finds these calls.

## Logs

- All crates write logs with `tracing`.
- The desktop application writes its logs to a file in its data folder.
- The server writes structured logs to the standard output.

## Interface text

- All interface text is in Fluent files (`.ftl`). The code contains no interface text.
- English is the source language. Translators add other languages.
- Interface text follows STE.

## Settings and data

Presswork uses the standard folders of each platform for settings, data, and cache:

- On Windows, the folders are in `%APPDATA%` and `%LOCALAPPDATA%`.
- On Linux, the folders follow the XDG Base Directory specification.
- On macOS, the folders are in `~/Library`.
