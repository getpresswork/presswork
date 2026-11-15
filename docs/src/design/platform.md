# Platform

This chapter tells how scripts, other programs, plugins, and AI services connect to Presswork.

## Command-line tool

The `presswork-cli` program uses the same engine as the desktop application. It operates without a window. Examples:

| Command | Result |
| --- | --- |
| `presswork-cli build --profile print` | Compiles the project and exports the print profile. |
| `presswork-cli check` | Runs preflight, the quote checker, the spell checker, and the house style rules. |
| `presswork-cli ocr scan.pdf` | Adds a text layer to a scanned PDF file. |
| `presswork-cli toolchain install typst 0.14` | Installs a Typst version. |

CI systems can use the command-line tool to check and build publications.

## API

The server API is public. An OpenAPI description documents it. Refer to [Server](server.md). Personal access tokens give scripts access to the API.

## Plugins

Plugins add functions to Presswork. A plugin is a WebAssembly module that uses the WebAssembly component model. It runs in a wasmtime sandbox.

- A plugin can add checks, importers, exporters, routine steps, and panels.
- A plugin declares the permissions that it must have, for example "read project files". The user approves these permissions during the installation.
- A plugin cannot use the network or the file system without a permission.
- A plugin panel uses a declarative description of its widgets. Presswork draws the widgets with its own design system. Thus plugins look like the rest of Presswork.
- Plugins run on the desktop. The server runs only the plugins that an administrator installs.

## AI assistance

AI assistance is off by default. An administrator must turn it on for the organization.

- The AI functions are: copyedit suggestions, alternative text for images, index terms, translation drafts, and summaries.
- An AI result is always a suggestion. A person accepts or rejects each result. Copyedit results use suggestion mode.
- The administrator selects the provider: the Anthropic API, or a local model server with an OpenAI-compatible API.
- Before Presswork sends content, it shows the content and the provider. The person must confirm.
- In encrypted projects, AI assistance is off. A person can approve one request at a time.
- If the provider has a setting that stops the use of content to train models, Presswork turns on this setting.
