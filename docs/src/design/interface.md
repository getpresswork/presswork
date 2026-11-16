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

## Top bar

Each window has a top bar of 32 pixels (D117). On Windows and Linux, Presswork draws the top bar and the window buttons, and the window has no system title bar (D116). On macOS, the traffic lights of the system are at the left of the top bar.

The top bar of the project window shows these items, from left to right:

- The Presswork mark and name. Click them to open the Presswork menu.
- The workspace tabs. If the window is too narrow for the labels, the tabs show only their icons (D122).
- The **Open folder** button. In a narrow window, it shows only its icon.
- On Windows and Linux, the minimize, maximize, and close buttons.

To move the window, drag an empty area of the top bar. To maximize or restore the window, double-click an empty area. On Windows and on Wayland, right-click an empty area to open the window menu of the system. To change the size of the window, drag an edge or a corner. A window is 720 by 480 pixels or larger.

The window of the detached preview has its own top bar. It shows the title of the window and the window buttons (D119).

### Presswork menu

| Item | Keys | Result |
| --- | --- | --- |
| Open folder… | Ctrl+O | Opens a project folder. |
| Save | Ctrl+S | Saves the main file. This item is not available when no project is open. |
| Connect to a server… | | Opens the server dialog. Refer to [Server documents](#server-documents). |
| Typst versions | | Opens or closes the Toolchains panel. |
| Logs | | Opens or closes the log panel. |
| Quit | Ctrl+Q | Closes Presswork. |

On macOS, the keys use Cmd. To close the menu, push Escape or click outside of the menu.

### Changes that are not saved

If the main file has changes that are not saved, a dialog opens before Presswork closes (D120). Quit, Ctrl+Q, the close button of the project window, and the close command of the system (for example Alt+F4) open the dialog. The close button of the detached preview only attaches the preview to the project window again.

- **Save** saves the file and closes Presswork. If Presswork cannot save the file, it stays open and shows the problem.
- **Do not save** closes Presswork. The changes are lost.
- **Cancel** closes the dialog. Presswork stays open.

Escape gives the same result as **Cancel**, and Enter gives the same result as **Save**. On macOS, Cmd+Q in the system menu closes Presswork at once in this release.

## Commands

All actions are commands. A command has an identifier, a title, an optional shortcut, and a condition that tells when it is available.

- The command palette finds and runs all commands. Open it with Ctrl+K, or with Cmd+K on macOS.
- Menus, toolbars, and shortcuts run the same commands.
- You can change all shortcuts.

Thus each action has one origin, and you can use all functions with the keyboard.

## Visual design

The interface has a quiet editorial style. Warm paper colors, ink colors, and thin rules make it look like a good book page. Pages and text get the most space. Tools stay small and quiet until you use them.

- The `presswork-ui` crate contains the design tokens: colors, type sizes, spacing, radii, and shadows. Widgets use only tokens, not literal values.
- Presswork has a light theme, a dark theme, and a high-contrast theme. By default, it follows the theme of the operating system.
- Presswork includes its interface fonts and its icons. Thus the interface looks the same on all platforms.

### Design tokens

| Token | Values |
| --- | --- |
| Interface font | Inter: regular, medium, and semibold |
| Font for code and data | JetBrains Mono NL: regular, bold, and italic. These faces have no ligatures. |
| Icons | Lucide, as an icon font |
| Accent color | Oxblood: #8C2F39 in the light theme, #E07A84 in the dark theme |
| Type sizes | 11, 12, 13, 15, 18, and 24 pixels. Body text is 13 pixels. |
| Spacing | 2, 4, 8, 12, 16, 24, and 32 pixels |
| Radii | 4, 6, and 10 pixels |
| Icon sizes | 12, 14, and 28 pixels |
| Bars | Top bar: 32 pixels. Status bar: 28 pixels. Window buttons: 46 pixels wide. Resize edges: 6 pixels. |

### Color roles

| Role | Use |
| --- | --- |
| Page | The background of documents and of the main area |
| Chrome | The background of bars and panels |
| Raised | The background of fields, menus, and dialogs |
| Rule | Lines of 1 pixel between areas |
| Ink | Body text |
| Muted ink | Labels, times, and secondary text |
| Faint ink | Disabled items only |
| Accent | The selected item, the focus, and the primary action |
| Danger, warning, success, and info | Status only |
| Close | The close button of the window when the pointer is on it, with a white symbol. It is #C42B1C in all themes, as on Windows 11. |

### Rules

- Use the accent color only for the selected item, the focus, and the primary action. Do not use it as decoration.
- Use the status colors only for status.
- Use one rule of 1 pixel between two areas.
- Show times, areas, file names, and data in JetBrains Mono.
- Each workspace and each command has a Lucide icon.

## Status bar

The status bar shows the workflow stage, the open tasks and queries, the diagnostics, the sync status, the team members in the document, and the Typst version.

In milestone 1, the status bar shows these items:

- The project name and the open file, or "No project open". For a server document, the path of the document and the state of the connection.
- "Not saved" if the open file has changes that are not saved. Click it to save the file.
- The duration of the last compilation and the position of the cursor, in the Write workspace.
- The Typst version that the project uses. If no installed version agrees with the pin of the project, the item shows a warning. Click it to open the Toolchains panel.
- The number of log errors and warnings, and the Logs button.

The Toolchains panel and the log panel open at the bottom of the window. Only one of them is open at a time.

## Projects

A project is a folder with a `presswork.toml` manifest. The manifest has a `[project]` table:

| Key | Value | Default |
| --- | --- | --- |
| `name` | The name of the project. | No default. The key is necessary. |
| `main` | The path of the main Typst file, relative to the folder. | `main.typ` |
| `typst` | The Typst version that the project needs, for example `0.15`. | No pin |

Presswork ignores the keys that it does not know. Thus an older Presswork release can open a project from a newer release.

- To open a project, click **Open folder** in the top bar, select **Open folder…** in the Presswork menu, or push Ctrl+O. Then select the project folder.
- The Write workspace shows the main file in the editor and its pages in the preview.
- To save the main file, push Ctrl+S.
- If the folder is not a project, a notice tells the problem.

## Server documents

In milestone 1, you can edit one text document of a server project with other persons at the same time (D130, D132).

1. In the Presswork menu, select **Connect to a server…**.
2. Type the server address, your email address, and your password. Then select **Sign in**.
3. The dialog shows the projects of your organizations and their documents. Select a document. If a project has no documents, select **Make main.typ**.

The document opens in the Write workspace instead of the local project. The preview compiles it as one file. Each change goes to the server at once, and the changes of the other persons appear in the editor. Your cursor stays at its place in the text.

The status bar shows the path of the document and the state of the connection:

| State | Meaning |
| --- | --- |
| Connecting | Presswork connects to the server and gets the document. |
| Live | Your changes go to the server, and the changes of the other persons come from the server. |
| Offline | The connection stopped. Presswork tries again every 3 seconds. Your changes stay in the document. They go to the server when the connection is live again. |

Presswork keeps the session in memory only (D131). After a restart, sign in again.

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
- All themes have a contrast ratio of 4.5:1 or more for text. Tests measure each text color on each surface. Disabled text is the only exception.
- The interface scale can change from 75% to 200%.
- iced does not have screen reader support at this time. Presswork will add it when iced gets AccessKit integration.

## Platform conventions

- On macOS, shortcuts use Cmd. On Windows and Linux, shortcuts use Ctrl.
- In milestone 1, the Presswork menu is in the top bar on all platforms. A later release adds the global menu bar of macOS.
- Presswork uses the native file dialogs of each platform.
