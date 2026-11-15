# Overview

This chapter gives the purpose, the users, and the principles of Presswork.

## Purpose

Publishing teams use many tools. Authors write in word processors. Typesetters use layout programs. Managers use task trackers and spreadsheets. Presswork puts all of this work into one workbench.

Presswork uses [Typst](https://typst.app) for all typesetting. Typst is a markup-based typesetting system. It compiles fast and gives high-quality pages.

## Users

These persons use Presswork:

| Role | Work |
| --- | --- |
| Author | Writes and corrects the content. |
| Editor | Examines the structure and the content. Sends queries to the author. |
| Copyeditor | Corrects the language and the style. |
| Proofreader | Examines the typeset pages and records corrections. |
| Typesetter | Makes the page design and the styles in Typst. |
| Production manager | Makes the schedule, assigns tasks, and approves the output. |
| Reviewer | Examines the content and gives approval or comments. |
| Translator | Translates the content into other languages. |
| Administrator | Controls the server, the organizations, and the roles. |

These roles are examples. Each organization can change them.

## Publication kinds

Presswork has templates for four kinds of publication. All kinds use the same data model.

| Kind | Examples | Structure |
| --- | --- | --- |
| Book | Novel, nonfiction book, textbook | Parts and chapters |
| Periodical | Magazine, newsletter, journal | Editions and articles |
| Technical documentation | Manual, specification, guide | Versions, variants, and sections |
| Academic | Paper, thesis, journal article | Sections, citations, and review rounds |

## Principles

These principles control all design decisions.

1. **Typst first.** Presswork gives full access to Typst. It does not put a different language in front of Typst.
2. **Local first.** The desktop application does all typesetting work without a server. The server adds team functions.
3. **No lock-in.** A Presswork project is a normal Typst project. You can compile it with the Typst command-line tool.
4. **One workbench.** Content, typesetting, review, and production management are in one application.
5. **Simple by default.** Each role sees the tools for its work first. All other tools stay available.
6. **Reproducible output.** A project can pin its Typst version. Thus the same sources always give the same output.
7. **All platforms.** Windows, Linux, and macOS get the same functions and the same quality.
8. **Open.** Presswork is open source under the Apache License 2.0. Anyone can operate a Presswork server.

## Non-goals

Presswork does not try to do these things:

- Replace Typst markup with a visual page layout program.
- Be a general code editor for all programming languages.
- Collect usage data. Presswork has no telemetry.
- Print, sell, or distribute publications. Other services do this work.

## Terms

The [project dictionary](../contributing/style-guide.md#project-dictionary) gives the meaning of each Presswork term, for example publication, component, revision, and routine.
