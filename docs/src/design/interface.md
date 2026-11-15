# Interface

This chapter tells how the desktop interface operates.

## Workspaces

The interface has one pane system. A workspace is a saved layout of panes for one activity.

| Workspace | Users | Main panes |
| --- | --- | --- |
| Plan | Production managers | Tasks, timeline, calendar, workflow board, budget, reports |
| Research | Authors, editors, and researchers | Library, file reader with annotations, research notes, quote checker |
| Write | Authors and editors | Outline, editor, preview |
| Translate | Translators | Source text, target text, translation memory, termbase |
| Review | Editors, proofreaders, and reviewers | Preview, comments, tickets, suggestions, revision compare |
| Typeset | Typesetters | Files, editor, preview with guides, styles, fonts, assets, preflight |
| Publish | Production managers and typesetters | Export profiles, preflight, validation, covers, metadata, releases |

```text
+ Plan | Research | Write | Review | Typeset | Publish ----- Ctrl+K +
| Outline  | chapter-03.typ         | Preview                       |
|  Part I  | = The Long Night       |  +-------------------------+  |
|   Ch. 1  |                        |  |         Page 41         |  |
|   Ch. 2  | It was late when the   |  |                         |  |
| > Ch. 3  | press stopped.         |  +-------------------------+  |
|----------+------------------------+-------------------------------|
| Stage: Copyedit | 3 tasks | 2 queries | 0 errors | Typst 0.15     |
+-------------------------------------------------------------------+
```

The workspace bar at the top shows the workspaces in the sequence of a publication, from the plan to the release. It shows only the workspaces of the roles of the user. Select a workspace to change the layout. Each user can change a workspace and save it. Each role has a default workspace.

## Panes

- Each pane has tabs.
- You can split, move, and close panes with the mouse or with commands.
- Each panel can open in each pane. The panels are: project files, outline, editor, preview, diagnostics, build log, tickets, tasks, timeline, calendar, table, checklists, workflow, revisions, suggestions, comments, chat, inbox, related, library, annotations, research notes, termbase, translation memory, back-of-book index, assets, clearances, budget, time, contacts, contracts, structure board, variants, styles, QA report, proof corrections, history, logs, assistant, fonts, packages, toolchains, preflight, covers, metadata, reports, and team.

## Windows

- Each project opens in its own window. A window shows one project.
- You can detach each pane into its own window, for example the preview on a second screen. You can attach the pane to the project window again.
- Presswork remembers the windows of each project: the position, the size, the screen, and the layout.
- All windows run in one process. They share the Typst engine, the caches, and the server connection.
- Commands apply to the window that has the focus.
- The chat and the inbox can open in their own window, outside of a project.

## Commands

All actions are commands. A command has an identifier, a title, an optional shortcut, and a condition that tells when it is available.

- The command palette finds and runs all commands. Open it with Ctrl+K, or with Cmd+K on macOS.
- Menus, toolbars, and shortcuts run the same commands.
- You can change all shortcuts.

Thus each action has one origin, and you can use all functions with the keyboard.

## Visual design

- The `presswork-ui` crate contains the design tokens: colors, type sizes, spacing, radii, and shadows. Widgets use only tokens, not literal values.
- Presswork has a light theme, a dark theme, and a high-contrast theme. By default, it follows the theme of the operating system.
- The interface uses a calm style that puts the document first. Pages and text get the most space. Tools stay small until you use them.
- Presswork has its own interface fonts. Thus the interface looks the same on all platforms.

## Status bar

The status bar shows the workflow stage, the open tasks and queries, the diagnostics, the sync status, the team members in the document, and the Typst version.

## Home dashboard

The home dashboard is the start page. It shows:

- Your tasks and your inbox.
- The deadlines of the next days.
- Your recent projects.
- The activity of your teams.

From the home dashboard, you can also:

- Open a local folder.
- Make a new project from a template or a brand kit: book, periodical, documentation, or paper.
- Connect to a server and open a server project.
- Open the Typst scratchpad. Refer to [Typesetting tools](typesetting-tools.md).

## Operating system integration

- Presswork registers itself for `.typ` files and for `presswork.toml` files.
- The file managers show thumbnails of Typst documents, and macOS Quick Look shows a preview.
- The jump lists of Windows and the dock menu of macOS show recent projects.

## Accessibility

- You can use all functions with the keyboard.
- All themes have a contrast ratio of 4.5:1 or more for text.
- The interface scale can change from 75% to 200%.
- iced does not have screen reader support at this time. Presswork will add it when iced gets AccessKit integration.

## Platform conventions

- On macOS, shortcuts use Cmd, and the menu is in the global menu bar. On Windows and Linux, shortcuts use Ctrl, and the menu is in the window.
- Presswork uses the native file dialogs of each platform.
