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

- It wraps long lines at the width of the view. Typst source contains much prose, and writers do not add line breaks in paragraphs.
- It shapes each line with cosmic-text and keeps the result in a cache. Thus an edit shapes only the changed lines. It draws only the visible lines.
- It shows line numbers, the current line, selections, and a blinking cursor. Later, it will also show remote cursors and diagnostic underlines.
- It changes keyboard, mouse, and input method events into editor commands.
- It gets syntax colors from a highlighter. The Typst highlighter uses `typst-syntax`, and it parses again only the changed part of the text. Other highlighters can come later, for example for TOML or BibTeX.

Each editor command is a pure function from the editor state to a transaction. Thus tests can run all commands without a window.

Until the project model is ready, the Write workspace shows a scratch document. Presswork does not save it.

### Keys

| Key | Command |
| --- | --- |
| Arrow keys | Move the cursor. With Shift, extend the selection. |
| Ctrl+Left and Ctrl+Right | Move to the previous or the next word. |
| Home | Move to the first text of the line. Push Home again to move to the start of the line. |
| End | Move to the end of the line. |
| Ctrl+Home and Ctrl+End | Move to the start or the end of the document. |
| Page Up and Page Down | Move by one page. |
| Backspace and Delete | Delete one character. With Ctrl, delete one word. |
| Enter | Start a new line with the same indentation. |
| Tab | Insert two spaces. |
| Ctrl+A, Ctrl+C, Ctrl+X, and Ctrl+V | Select all, copy, cut, and paste. |
| Ctrl+Z, and Ctrl+Y or Ctrl+Shift+Z | Undo and redo. |

On macOS, Cmd replaces Ctrl in shortcuts, and Alt replaces Ctrl for word movement and word deletion. Cmd+Left and Cmd+Right move to the start or the end of the line.

Click to move the cursor. Double-click to select a word, and triple-click to select a line. Drag to select text. Shift+click extends the selection.

### Syntax colors

Each syntax color is a design token. Tests make sure that each color has a contrast ratio of 4.5:1 or more on the page, on the current line, and on the selection.

| Role | Typst syntax |
| --- | --- |
| Keyword | Keywords, for example `let`, `set`, and `show`, and list markers |
| Function | Function names |
| String | Strings and raw text |
| Number | Numbers and math delimiters |
| Operator | Operators and punctuation |
| Reference | Labels, references, links, escapes, and interpolated values |
| Comment | Comments, in italic |
| Error | Syntax errors |

Strong text, headings, and terms are bold. Emphasis is italic.

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

In milestone 1, the preview does these things:

- The Write workspace shows the editor on the left and the preview on the right.
- After each change to the text, the engine compiles the text again. The status bar shows the duration of the last compilation.
- By default, the pages fill the width of the preview. The zoom buttons change the zoom from 25% to 400%. If the pages are wider than the preview, you can scroll to the left and to the right.
- If the text has errors, the preview continues to show the last pages without errors. A bar above the pages shows the first error and its line.

## Source and pages

- If you click a point in the preview, the engine finds the source location of that point. The editor opens the file and moves the cursor there.
- If the cursor moves, the preview scrolls to the cursor position in the pages and marks it for a short time.
