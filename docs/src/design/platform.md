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

In milestone 2, the program has the `build` and `check` commands (D183):

- The last argument is the project folder. The default is the current folder.
- `--profile NAME` selects an export profile. To select more profiles, give the option again. Without the option, the command uses all profiles of the manifest. If the manifest has no profiles, the command uses one PDF profile, as the Publish workspace does.
- `build` compiles the project and writes the files and the build report of each profile. It writes the path of each new file to the standard output, one path on each line.
- Each command runs preflight for each selected profile. A problem of the document shows one time. A problem of a profile shows with the name of the profile (D184).
- `check` also lists the problems of the typesetting QA report, for example widows and loose lines. These problems do not change the exit code (D185).
- `check` also checks the spelling with the downloaded dictionaries. Misspelled words make the check fail. `--dictionaries FOLDER` gives a different folder with dictionaries, for example a cache in a CI system. The default is the folder of the desktop application (D188).
- `check` also checks the house style rules in `presswork-style.toml`. House style problems make the check fail. A house style file that is not valid is an error (D189).
- `check` compiles the project and runs preflight. It writes no files. The quote checker comes later.
- Errors, warnings, and preflight problems go to the standard error stream, one on each line, for example `chapters/one.typ:12:5: error: unknown variable: x`. Lines and columns start at 1. Preflight problems are warnings.
- The exit code is 0 for success and 1 for a failure. `check` fails if the document has errors, preflight problems, misspelled words, or house style problems. `build` fails if the document has errors or if a profile cannot write its files. Preflight problems do not stop `build`.
- If the Typst pin of the project does not accept the built-in Typst version, the command stops.
- `presswork-cli --help` shows all options.

The release packages do not contain the command-line tool yet. To install it from the source code, run `cargo install --path crates/presswork-cli` in the repository.

## API

The server API is public. An OpenAPI description documents it. Refer to [Server](server.md). Personal access tokens give scripts access to the API.

## Plugins

Plugins add functions to Presswork. A plugin is a WebAssembly module that uses the WebAssembly component model. It runs in a wasmtime sandbox.

- A plugin can add checks, importers, exporters, routine steps, and panels.
- A plugin declares the permissions that it must have, for example "read project files". The user approves these permissions during the installation.
- A plugin cannot use the network or the file system without a permission.
- A plugin panel uses a declarative description of its widgets. Presswork draws the widgets with its own design system. Thus plugins look like the rest of Presswork.
- Plugins run on the desktop. The server runs only the plugins that an administrator installs.

## AI assistance and MCP

AI assistance is off by default. Refer to [AI assistance](ai.md) for the AI functions, the AI gateway, the providers, and the MCP servers.
