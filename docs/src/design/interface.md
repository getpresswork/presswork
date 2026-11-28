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
| New project… | | Makes a project in a folder that you select, and opens it (D205). |
| Open folder… | Ctrl+O | Opens a project folder. |
| Save | Ctrl+S | Saves the file in the editor. This item is not available when no project is open. |
| Connect to a server… | | Opens the server dialog. Refer to [Server documents](#server-documents). |
| Typst versions | | Opens or closes the Toolchains panel. |
| Typst documentation | F1 | Shows the Typst documentation next to the editor (D208). |
| Typst packages | | Shows the packages of Typst Universe next to the editor (D208). |
| Personal dictionary | | Opens your personal dictionary in the text editor of the system (D212). |
| Logs | | Opens or closes the log panel. |
| Quit | Ctrl+Q | Closes Presswork. |

On macOS, the keys use Cmd. To close the menu, push Escape or click outside of the menu.

### Changes that are not saved

If a file of the project has changes that are not saved, a dialog opens before Presswork closes (D120). The dialog names all files with changes, and **Save** saves all of them (D204). Quit, Ctrl+Q, the close button of the project window, and the close command of the system (for example Alt+F4) open the dialog. The close button of the detached preview only attaches the preview to the project window again.

- **Save** saves the file and closes Presswork. If Presswork cannot save the file, it stays open and shows the problem.
- **Do not save** closes Presswork. The changes are lost.
- **Cancel** closes the dialog. Presswork stays open.

Escape gives the same result as **Cancel**, and Enter gives the same result as **Save**. On macOS, Cmd+Q in the system menu closes Presswork at once in this release.

The same dialog opens before Presswork opens a different project, a new project, or a server document (D205). **Save** and **Do not save** then open it, and **Cancel** keeps the open project.

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

In milestone 2, the status bar of the Write workspace also shows the **Outline** button. It opens or closes the outline panel at the left of the editor. Refer to [Editor and preview](editor.md#outline). With a project, the **Files** button opens or closes the files panel above the outline. Refer to [Editor and preview](editor.md#files).

The **Suggest** button starts or stops suggestion mode. While suggestion mode is on, the button stays pressed. Refer to [Editorial tools](editorial.md#suggestion-mode).

The Toolchains panel, the log panel, and the chat panel open at the bottom of the window. Only one of them is open at a time.

## Projects

A project is a folder with a `presswork.toml` manifest. The manifest has a `[project]` table:

| Key | Value | Default |
| --- | --- | --- |
| `name` | The name of the project. | No default. The key is necessary. |
| `main` | The path of the main Typst file, relative to the folder. | `main.typ` |
| `typst` | The Typst version that the project needs, for example `0.15`. | No pin |

Presswork ignores the keys that it does not know. Thus an older Presswork release can open a project from a newer release.

- To open a project, click **Open folder** in the top bar, select **Open folder…** in the Presswork menu, or push Ctrl+O. Then select the project folder.
- To make a project, select **New project…** in the Presswork menu. Then select or make a folder. Presswork writes `presswork.toml` and `main.typ` if the folder does not have them, and opens the project (D205).
- The Write workspace shows the main file in the editor and its pages in the preview. To edit another file, click it in the files panel. The files panel also makes, renames, and deletes files. Refer to [Editor and preview](editor.md#files).
- To save the file in the editor, push Ctrl+S.
- If the folder is not a project, a notice tells the problem.

The manifest can also have `[[export]]` tables. Each table is an export profile:

| Key | Value | Default |
| --- | --- | --- |
| `name` | The name of the profile, for example `print`. | No default. The key is necessary. |
| `format` | `pdf`, `png`, or `svg`. | `pdf` |
| `standards` | The PDF standards of the file, for example `["a-2b", "ua-1"]`. | No standards |
| `output` | The path of the file, relative to the project folder. For PNG and SVG, `{p}` is the page number. `{0p}` is the page number with zeros at the start, so that all names have the same length. | The name of the main file next to the main file, with the extension of the format. For PNG and SVG, `-{p}` comes before the extension. |
| `ppi` | The resolution of PNG files. | `144` |
| `image-ppi` | The lowest resolution of images that preflight accepts. | No check |
| `page-size` | The trim size of each page, as width x height with a unit: `mm`, `cm`, `in`, or `pt`. For example `"148mm x 210mm"` or `"6in x 9in"`. | No check |
| `colors` | The colors that preflight accepts: `cmyk` for CMYK, gray, and spot colors, or `gray` for gray colors only. | No check |

The names of the standards are `1.4`, `1.5`, `1.6`, `1.7`, `2.0`, `a-1b`, `a-1a`, `a-2b`, `a-2u`, `a-2a`, `a-3b`, `a-3u`, `a-3a`, `a-4`, `a-4f`, `a-4e`, and `ua-1`.

```toml
[project]
name = "Field Notes"

[[export]]
name = "print"
standards = ["a-2b"]
image-ppi = 300
page-size = "148mm x 210mm"
colors = "cmyk"
output = "out/field-notes-print.pdf"

[[export]]
name = "pages"
format = "png"
output = "out/pages/page-{0p}.png"
```

The manifest can also have a `[typeset]` table for the typesetting QA report (D194):

| Key | Value | Default |
| --- | --- | --- |
| `baseline-grid` | The distance between two lines of the baseline grid, as a number with a unit: `mm`, `cm`, `in`, or `pt`. For example `"13pt"`. The value must be larger than 0. | No grid and no grid check |
| `baseline-start` | The distance of the first grid line from the top of the page. | The baseline of the first line of body text in the document |
| `widows` | `true` finds widows, `false` does not. | `true` |
| `orphans` | `true` finds orphans, `false` does not. | `true` |
| `stacked-hyphens` | The largest number of hyphenated lines in sequence that the report accepts. | `3` |
| `short-last-line` | The smallest number of characters, without spaces, that the last line of a paragraph must have. `0` accepts all last lines. | `5` |
| `loose-spacing` | The widest word space that the report accepts, in percent of the normal width of the space. The value must be 100 or more. | `150` |

```toml
[typeset]
baseline-grid = "13pt"
baseline-start = "30mm"
widows = false
stacked-hyphens = 2
loose-spacing = 180
```

### Export

The Publish workspace shows the export profiles of the project. If the manifest has no `[[export]]` tables, the workspace shows one PDF profile.

1. Select the **Publish** workspace.
2. Click **Export** next to the profile.
3. Read the result below the profile. It gives the files, the build report, and the preflight problems.
4. To see the source of a problem, click the problem. The Write workspace opens and selects the source in the editor.

The export uses the text in the editor, also the changes that you did not save. Presswork writes export files only in the project folder. In milestone 2, only the built-in Typst engine can export (D182).

Each export also writes a build report, `<profile name>.report.json`, in the folder of the export files. Refer to [Logs and diagnostics](logs.md) (D184).

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

When a server document is open, the **Chat** button in the status bar opens the chat of its organization. Refer to [Communication](communication.md).

If you open a local project, it takes the place of the server document. The connection and the chat stay open.

### Tasks

When a server document is open, the Plan workspace shows the tasks of its project (D135). Each line shows the title and the due date of a task. A task that is in progress also shows **In progress**.

- To set a task to done, click its check box. To set it back to to do, click the check box again. The change goes to the server at once. If the server refuses the change, a message tells the problem and the list loads again.
- To add a task, type its title in the field at the top. Then select **Add** or push Enter.
- The list loads again each time that you select the Plan workspace.

### Library

When a server document is open, the Research workspace shows the library of its organization (D141). Refer to [Library](library.md). Each work shows its title, its creators, and its citation key. Below the work, each edition shows its year, its publisher, its language, its translators, and the number of pages with text. The standard edition shows "Standard edition".

- To add a work, select **Add PDF…** and select a PDF file. Presswork proposes a title from the file name and a citation key from the title. Type the data of the edition, change the values if necessary, then select **Add**. Presswork uploads the file. The limit for one file is 64 MiB.
- To add an edition, for example a translation, select **Add edition…** next to the work and select its PDF file.
- To change the standard edition, select **Make standard** next to an edition (D146).
- After the upload, the edition shows "The server reads the text of the pages." When the server is ready, the list changes at once and shows the number of pages with text. If the server cannot read the file, the edition shows the reason.
- To correct the printed page numbers of an edition, select **Page numbers…** next to the edition (D147). Each correction shows its first PDF page and its first printed page, for example "PDF page 15 is printed page 1". To add a correction, type the PDF page and the printed page, select the style (1, 2, 3 or i, ii, iii or I, II, III), then select **Add correction**. Select **Save** to send the corrections to the server.
- To find a passage, type words in the search field. Then select **Find** or push Enter. The results show each page that has all the words, with its printed page number and the text around the words.
- To cite a page, select **Cite** next to a result. Presswork opens the Write workspace and puts `@key[p. N]` at the cursor, with the printed page number. If the document has no `#bibliography`, Presswork adds `#bibliography("library.yml")` at the end. A result from an edition that is not the standard edition shows "Not the standard edition" and has no **Cite**.
- Presswork gives the Typst engine the file `library.yml` with the standard editions of the works. Thus the citations compile in the preview.

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

In milestone 2, the home dashboard has these parts (D214):

- **Home** is the first tab of the workspace bar. Presswork starts with it.
- **Start** has the buttons **Open folder…**, **New project…**, and **Connect to a server…**. They do the same as the items of the Presswork menu.
- **Recent projects** shows the last 10 local projects that you opened, the newest first, with the name and the folder. A click opens the project and asks about the changes that are not saved, as **Open folder…** does. If the folder is not there, the row tells it, and a click gives a note. The **X** button removes a project from the list. The folder stays on the disk.
- Presswork keeps the list in the file `recent-projects.json` in the local data folder, because the paths belong to the computer.

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
