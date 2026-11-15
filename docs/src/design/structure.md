# Structure and reuse

This chapter tells how Presswork helps teams to structure publications and to reuse content.

## Structure board

- The structure board shows the parts, the chapters, and the sections of a publication as cards.
- Drag a card to move a chapter. Presswork changes the `#include` lines of the Typst source.
- Each card shows the workflow stage, the word count, the assignee, and the open tickets of its chapter.

## Shared content blocks

- A content block is a piece of Typst content that many publications use, for example a copyright page, an author biography, a disclaimer, or a series page.
- The content blocks of a team are on the server. A publication uses a block through a link.
- Presswork copies the block into the project as a normal file. Thus the project compiles without Presswork.
- When a block changes, Presswork shows the change in each publication that uses the block. A person accepts the update for each publication.
- A publication after its release keeps the version of the block that it used.

## Variants

- A variant is one output of a publication, for example a print variant and a screen variant, a student variant and a teacher variant, or a regional variant.
- Each variant has a name and a set of values for `sys.inputs`.
- Conditional text shows only in some variants. The Presswork helper package supplies the Typst function for conditional text.
- The variants panel shows which text is in which variant. The preview can show each variant.
- Each export profile selects one variant.

## Data merge

- A data file supplies data to a document, for example a catalog of products, a directory, a price list, or an annual report.
- Typst reads CSV, JSON, and YAML files with its `csv`, `json`, and `yaml` functions.
- Presswork shows the data in the table view, checks it against a schema, and compiles again when the data changes.
- A routine can download a data file from an address on a schedule, for example the CSV export of a spreadsheet.
