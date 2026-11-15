# Editor and preview

This chapter tells how the text editor and the page preview operate.

## Why Presswork has its own editor

The iced `text_editor` widget is good for simple forms. Presswork must have more functions:

- Remote cursors and selections of other team members.
- Edits from the network that change the text near the cursor.
- Underlines for diagnostics.
- Completion lists and tooltips.
- Many cursors, folding, and find and replace.

Thus Presswork has its own editor. It has two parts: a pure text model and an iced widget.

## Text model

The `presswork-text` crate contains the text model. It does not use iced or Typst.

| Type | Purpose |
| --- | --- |
| `Buffer` | The text, in a ropey rope. Edits and line access stay fast in large files. |
| `Position` | A byte offset. It converts to and from lines and columns in UTF-8 bytes, characters, and UTF-16 units. |
| `Edit` | The replacement of one range with new text. |
| `ChangeSet` | Runs that keep or replace text, over the full text. Change sets can compose, invert, and map through other change sets. A change set can also map a position. |
| `Selection` | One or more ranges, with one primary range. |
| `Transaction` | A change set and a new selection. All changes go through transactions. |
| `History` | Undo and redo of your own edits. An edit joins the last group if it is less than 500 ms after the last edit and touches its text. |
| `Document` | A buffer, a selection, and a history. It applies transactions, undo, redo, and remote changes. |

The model maps positions through each change set. Thus cursors, remote cursors, diagnostics, and anchors stay at the correct text after an edit.

The buffer finds line breaks with the same rules as Typst: LF, CR, CRLF, VT, FF, NEL, LS, and PS.

Remote changes rebase the history. Thus undo changes only your own edits, also after other persons changed the text.

## Editor widget

The `presswork-editor` crate contains the iced widget. The widget does these things:

- It shapes and draws only the visible lines.
- It draws selections, cursors, remote cursors, and diagnostic underlines.
- It changes keyboard, mouse, and input method events into editor commands.
- It gets syntax colors from a highlighter. The Typst highlighter uses `typst-syntax`. Other highlighters can come later, for example for TOML or BibTeX.

Each editor command is a pure function from the editor state to a transaction. Thus tests can run all commands without a window.

## Scripts and input

- The editor supports bidirectional text, for example Arabic or Hebrew mixed with Latin text. The cursor moves in the visual sequence of the line.
- The editor supports the input methods for Chinese, Japanese, and Korean, with the composition text in the line.
- Line breaks, word selection, and cursor movement follow the Unicode rules for each script.

## Vim mode

An optional Vim mode gives the Vim keys and modes for movement and edits. The Vim commands are editor commands. Thus they also make transactions, and undo operates in the same way.

## Preview widget

The `presswork-preview` crate contains the page preview. The preview does these things:

- It shows pages in a vertical list, or as spreads of facing pages for books.
- It shows only the visible pages, and it keeps a cache of rendered images.
- It can show guides: margins, bleed, trim box, and baseline grid.
- In the Review workspace, it lets reviewers attach comments to an area of a page.

## Source and pages

- If you click a point in the preview, the engine finds the source location of that point. The editor opens the file and moves the cursor there.
- If the cursor moves, the preview scrolls to the cursor position in the pages and marks it for a short time.
