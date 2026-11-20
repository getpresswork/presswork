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
| Ctrl+Space | Show the completions at the cursor. |
| F12 | Go to the definition of the item at the cursor. |
| Ctrl+F | Open the find bar. |
| Ctrl+H | Open the find bar with the replace row. |
| F3 and Shift+F3 | Go to the next or the previous result of the find bar. |
| Ctrl+D | Select the word at the cursor. Then add the next place of the same text. |
| Ctrl+Shift+L | Select all places of the selected text. |
| Ctrl+Alt+Up and Ctrl+Alt+Down | Add a cursor on the line above or below. |
| Escape | With many cursors, keep only the primary cursor. |
| Ctrl+Shift+[ and Ctrl+Shift+] | Fold or unfold the part of the text at the cursor. |

On macOS, Cmd replaces Ctrl in shortcuts, and Alt replaces Ctrl for word movement and word deletion. Cmd+Left and Cmd+Right move to the start or the end of the line. Ctrl+Space stays Ctrl+Space, because Cmd+Space opens Spotlight. Cmd+Option+F opens the replace row, because Cmd+H hides the application. Cmd+Option+Up and Cmd+Option+Down add cursors. Cmd+Option+[ and Cmd+Option+] fold and unfold.

Click to move the cursor. Double-click to select a word, and triple-click to select a line. Drag to select text. Shift+click extends the selection. Ctrl+click a name to go to its definition. On macOS, use Cmd+click. Alt+click adds a cursor. On macOS, use Option+click.

### Tooltips

If the mouse pointer stays on the text for half a second, the editor shows a tooltip for the item below the pointer. The tooltip comes from the embedded engine (D169). For example, it gives a short description of a function, or a length in other units. Code in a tooltip has the monospace font. The tooltip goes above the line, or below the line if there is no space above. If you move the pointer, push a key, or click, the tooltip closes.

### Completion

While you type Typst code, the editor shows completions, for example after `#`. To see the completions at the cursor, also in text, push Ctrl+Space.

- The list shows the completions that start with the typed text, without a difference between capital and small letters. It shows eight items at a time.
- Up and Down select an item. Enter or Tab inserts the selected item. A click on an item inserts it. Escape closes the list.
- If a completion has placeholders, the editor selects the first placeholder. If the placeholder is empty, the cursor goes to its position. For example, `text` becomes `text()` with the cursor between the parentheses.
- A box below the list gives the description of the selected item.
- For a label after `@`, the description gives the kind of the element, its page, and its text, for example "Figure, page 4: A cat" (D174).
- If you move the cursor, the list closes.

Tooltips and completions use the text in the editor, also if it changed after the last compilation (D172).

### Go to definition

Push F12 or Ctrl+click a name to go to the definition of the item (D173).

- If the definition is in the main file, the editor selects the name in the definition. For a reference, the editor selects the element with the label.
- If the definition is in another file, a note gives the name of the file, because the editor shows only the main file.
- If the item is part of the Typst library, for example `text`, a note tells this.

### Find and replace

Push Ctrl+F to find text in the document (D176, D177). The find bar opens above the editor. If you selected text on one line, the bar finds that text.

- Each result has a background color. The bar shows the number of results, for example "2 of 8". While you type, the editor selects the first result after the start of the selection.
- Enter or F3 goes to the next result. Shift+Enter or Shift+F3 goes to the previous result. After the last result, the search starts again at the top.
- **Match case** finds only text with the same capital and small letters. **Whole word** finds only full words. **Regular expression** finds a pattern, for example `\d+` for numbers. The signs `^` and `$` match at the start and at the end of each line.
- To replace text, push Ctrl+H or click the arrow at the left of the bar. **Replace** changes the selected result and selects the next result. **Replace all** changes all results in one step, and one undo restores them. With a regular expression, `$1` in the replacement gives the first group of the result.
- The results follow all changes of the text, also the changes of other persons.
- Escape closes the bar and gives the keyboard focus back to the editor.

### Many cursors

You can edit the text at many places at the same time (D178).

- Alt+click a place to add a cursor. Alt+click a cursor to remove it. Some Linux desktops use Alt+click to move windows. Then use the keys.
- Ctrl+Alt+Up adds a cursor above the first cursor, and Ctrl+Alt+Down adds a cursor below the last cursor, at the same column.
- Ctrl+D selects the word at the cursor. Push Ctrl+D again to add the next place of the same word. If you selected the text in a different way, Ctrl+D also finds the text in longer words. Ctrl+Shift+L selects all places at one time. These keys find only text with the same capital and small letters.
- Typing, deletion, new lines, and movement apply at each cursor. A new line keeps the indent of its own line.
- If you paste as many lines as there are cursors, each cursor gets one line. Otherwise, each cursor gets all of the text.
- Escape keeps only the primary cursor. The primary cursor is the cursor that you added last. Undo restores all the cursors.
- A click without Alt, the find bar, completion, and go to definition use the primary cursor and go back to one cursor.

### Folding

You can fold parts of the text to hide them (D179).

- A heading folds its text until the next heading of the same level or a higher level. A block in brackets, a raw block, an equation, and a comment fold the lines between their first line and their last line.
- Move the mouse pointer over the line numbers to see an arrow next to each part that can fold. Click the arrow to fold the part. Push Ctrl+Shift+[ to fold the part at the cursor.
- A folded line shows an arrow that points to the right and a box with three dots after its text. To unfold the part, click the arrow or the box, or push Ctrl+Shift+] on the line.
- The arrow keys go over folded text. If the cursor goes into folded text in a different way, for example from the find bar or from a click in the preview, the text unfolds.
- Folds move with the changes of the text. Presswork does not keep the folds after you close the document.

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
- A button in the toolbar of the preview opens the preview in its own window, for example on a second screen. The editor then uses the full width of the main window. To show the preview beside the editor again, close the preview window or click its button. If you close the main window, Presswork closes.

## Source and pages

The embedded engine connects the points on the pages to the source, and the source positions to the pages (D169).

- Click a point on a page. If the point is on text of the main file, the editor moves the cursor to that character and gets the keyboard focus. A click on the text of a web link also goes to the text (D170). A click on a link to a label or to a position scrolls the preview to the target of the link.
- If the text comes from another file, a note gives the name of the file, because the editor shows only the main file.
- If you move the cursor with the keys or the mouse, the preview marks the cursor position in the pages for 1.5 seconds. If the position is not in the visible part of the preview, the preview scrolls until the position is at one third of its height. While you type, the preview does not move (D171).
- The positions come from the last compilation that gave pages. If you changed the text after that compilation, a position can be a few characters away from the correct position.
- With the managed toolchain, the jumps are not available. A click on the pages shows a note that tells why.

## Outline

The outline panel shows the headings of the document at the left of the editor (D175).

- Each heading shows its number, its text, and its page. The indent shows the level of the heading.
- The page is the printed page number. If the page has no numbering, it is the position of the page in the document.
- The heading that contains the cursor has the accent color.
- Click a heading to move the cursor to the start of the heading. The editor gets the keyboard focus, and the preview shows and marks the heading.
- If the heading is in another file, the preview shows the heading, and a note gives the name of the file.
- The **Outline** button in the status bar opens or closes the panel. In a narrow window, close the panel to give more space to the editor.
- The outline comes from the last compilation that gave pages. With the managed toolchain, the outline is not available, and a note tells why.
