# Toolchains

This chapter tells how Presswork installs and selects external tools, packages, and fonts.

## Tools

The toolchain manager controls these tools:

| Tool | Use | Source |
| --- | --- | --- |
| Typst | Exports with a pinned Typst version | GitHub releases of `typst/typst` |
| Pandoc | Import of Word, Markdown, and LaTeX manuscripts into Typst | GitHub releases of `jgm/pandoc` |
| veraPDF | Validation of PDF/A and PDF/UA files | veraPDF releases |
| Java runtime | The runtime for veraPDF, which is a Java program | Eclipse Temurin releases |

Each tool has a provider. A provider knows these things:

- How to list the available versions.
- Which release file is correct for the platform.
- How to get the published digest of each release file.
- How to unpack the release file, and where the executable is in it.

## Installation

The toolchain manager installs a tool version in these steps:

1. Download the release file to a temporary folder.
2. Calculate the SHA-256 digest of the file. Compare it with the published digest. If the digests are different, stop and delete the file.
3. Unpack the file into a new temporary folder. Do not unpack an entry that has an absolute path or `..` in its path.
4. Move this folder to its final location in one operation.

A lock file prevents two installations of the same version at the same time.

## Folders

Each tool version has its own folder in the data folder of Presswork:

```text
toolchains/
  typst/0.15.1/typst
  typst/0.14.2/typst
  pandoc/3.8.2/pandoc
```

The manager does not change the system `PATH`. Presswork always runs a tool through its full path.

## Version selection

A project can pin a Typst version in its manifest. A pin is a version requirement, for example `0.14` or `=0.14.2`. Presswork selects the version in this sequence:

1. If the project has no pin, or if the pin accepts the built-in version, Presswork uses the built-in engine. Refer to [Typst engine](typst-engine.md).
2. If not, Presswork uses the installed version with the highest number that agrees with the pin.
3. If no installed version agrees with the pin, the status bar shows a warning. The Toolchains panel shows the versions that you can install.

The command "Use for this project" writes an exact pin, for example `=0.14.2`. You can write a range pin, such as `0.14`, in the manifest by hand. Presswork keeps the comments and the layout of the manifest when it writes a pin.

## Digests

For Typst 0.14.0 and later, the manager compares each download with the SHA-256 digest that GitHub publishes for the file.

GitHub does not publish digests for older files. For Typst 0.11.0 to 0.13.1, Presswork includes the file `crates/presswork-toolchain/digests.txt`. The manager cannot install a release that has no digest. Thus the list shows only Typst 0.11.0 and later.

To make or extend the list of digests, do these steps:

1. Start the "Typst digests" workflow on GitHub. Its input is the list of versions.
2. The workflow downloads each release file, makes sure that its size agrees with GitHub, and calculates its digest.
3. Compare some digests with a record that is independent of the workflow, for example the history of the Typst manifest in the Scoop main bucket.
4. Copy the result of the workflow into `digests.txt`, and add a test for the digests that you compared.

## Dictionaries

The manager downloads the Hunspell dictionaries for the spell checker from the LibreOffice dictionaries repository on GitHub, at one fixed commit (D187). The file `crates/presswork-toolchain/dictionaries.txt` has the commit and one line for each dictionary: the language tag, the license, and the path, the size, and the SHA-256 digest of the `.aff` file, the `.dic` file, and the license file.

- The Toolchains panel shows the dictionaries in a third column. **Download** gets the three files of a dictionary. The manager compares each file with its digest. A file with a different digest stops the download, and Presswork keeps nothing.
- The manager puts each dictionary into the `dictionaries` folder in the local data folder of Presswork, in a folder with the language tag, for example `dictionaries/de-DE`.
- **Remove** deletes the folder of a dictionary.

To change the commit or to add a language, do these steps:

1. Select a new commit of the LibreOffice dictionaries repository.
2. Download the three files of each dictionary at that commit, and calculate their sizes and digests.
3. Read the license file of each new dictionary, and write its license into the list.
4. Make sure that each dictionary loads: set `PRESSWORK_TEST_DICTIONARIES` to a folder with the installed dictionaries, and run the ignored test `the_libreoffice_dictionaries_know_common_words` in `presswork-language`.
5. Update `dictionaries.txt`, and add the English name of each new language to the Fluent files of the desktop application.

## Network access

Presswork connects to these servers only for the actions in the table:

| Server | Actions |
| --- | --- |
| `api.github.com` and `github.com` | You open the Toolchains panel, click **Try again** in it, install a version, or download a dictionary. Presswork reads the release list from `api.github.com` and downloads the release files from `github.com`. |
| `packages.typst.org` | A document imports a `@preview` package that is not on this computer (D206). You open the Packages tab for the first time, or click **Try again** in it. You select a package that is not on this computer, and Presswork reads its README from the package archive. |

The requests contain no personal data. They identify the application as "Presswork" with its version.

## Milestone 1

In milestone 1, the toolchain manager does these things:

- It installs, lists, and removes Typst versions. Pandoc, veraPDF, and the Java runtime come later.
- It shows only stable releases. If the pin of the project agrees only with a release candidate, the panel also shows that candidate.
- It installs the versions into the `toolchains` folder in the local data folder of Presswork.
- The preview of a project uses the Typst version that its pin selects. Exports come later, and they will use the same version.

## Typst packages

- Presswork uses the standard Typst package folders. Thus the Typst command-line tool and other tools use the same packages.
- Before you go offline, the manager can download all packages of a project. It finds the imports with the Typst parser.
- A team can publish private packages to its Presswork server. The manager installs them into the local package folder under the namespace of the organization, for example `@acme/letterhead:1.0.0`.
- The package panel shows the packages in the cache. It can remove old versions.

In milestone 2, the manager downloads each `@preview` package that a document imports and that is not on this computer (D206). The Packages tab finds packages in the package index (D208).

To make sure that Presswork agrees with the real package server, run the ignored test `the_real_package_server_gives_the_index_and_the_packages` in `presswork-toolchain`. It downloads the package index and the package `@preview/example:0.1.0` into a temporary folder. It makes sure that the index has each field that Presswork reads, with the correct type. It also makes sure that the README of the package is in its archive, and that a document that imports the package compiles.

## Fonts

Presswork finds fonts in these locations:

1. The font folders of the project, for example `fonts/`.
2. The font library of the team on the server. The client keeps a local copy.
3. The system fonts.
4. The fonts that Typst supplies.

The font panel shows each font family, its styles, its glyph coverage, and its license. Presswork shows a warning if a document uses a font that is not available.

The administrators record the license of each font in a team library. Presswork shows the license before a member downloads the font.

## Manuscript import

Many authors send manuscripts as Word files. Presswork imports them with Pandoc:

1. Pandoc changes the manuscript into Typst markup.
2. A Presswork Lua filter maps the styles of the manuscript to the functions of the project template. For example, the Word style "Block Text" becomes `#quote`.
3. Presswork shows the result next to the original manuscript for examination.

## PDF validation

veraPDF validates PDF/A and PDF/UA files. It is a Java program, so the manager can also install a Java runtime for it.

Presswork reads the veraPDF report. It shows each failed rule with its page and its object.

Routines and checklists can use the validation result. Refer to [Domain model](domain-model.md).
