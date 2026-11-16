# Library

This chapter tells how the team library keeps texts, and how the library connects to the text of the publications.

## Purpose

The library keeps the texts that a team reads, quotes, and cites. All members use the same files and the same editions. Thus all quotations and page numbers agree.

## Library in milestone 1

Milestone 1 has a small library of the organization (D136 to D141):

- Each work has one edition and one PDF file. The edition is the standard edition for all purposes.
- To add a work, type the title, the creators, the year, and the citation key. Then upload the PDF file.
- The server reads the text layer of each page. A page without a text layer has no text until OCR is available.
- The search finds the pages of all works of the organization.
- **Cite** puts `@key[p. N]` in the document. N is the page in the PDF file.
- The desktop application makes the bibliography file `library.yml` from the works.

## Records

The library follows the FRBR model of libraries, in a simpler form.

| Record | Description |
| --- | --- |
| Work | A text as an abstract creation, for example *War and Peace*. It has the creators, the original language, and the original date. |
| Edition | One published form of a work: an original, a translation, or a reprint. It has the publisher, the year, the translators, the identifiers, and its own page numbers. |
| File | A PDF or EPUB file of an edition. The server keeps it as a blob. |
| Page map | The relation between the pages of a file and the printed page numbers of its edition. |
| Standard edition | An edition that the team selects for a work and a purpose. |
| Annotation | A highlight or a note on a page of a file. |
| Research note | A text with links to works, editions, pages, chapters, and tasks. |

## Standard editions

For each work, the team can set one or more standard editions. Each standard edition has a purpose, for example "English quotations" or "Original reference".

A standard edition has a scope. A narrow scope replaces a wide scope:

1. The organization.
2. A collection.
3. A publication.

If a document cites an edition that is not a standard edition in its scope, Presswork shows a warning. A checklist can make this check necessary before a workflow stage.

## Citations in Typst

- Each work has a citation key, for example `war-and-peace`.
- Each purpose has a key suffix, for example `original`. One purpose of each publication is the default purpose. It has no suffix.
- Authors cite the work, not the edition: `@war-and-peace[p. 412]` or `@war-and-peace:original[p. 388]`.
- Presswork writes a bibliography file in the Hayagriva format into each project. The file maps each key to the standard edition for the scope of the publication.
- If the team changes a standard edition, Presswork writes the bibliography file again. The concordance then proposes new page numbers for the citations.

The bibliography file is a normal file in the project. Thus the project compiles with the Typst command-line tool.

## OCR

Scanned PDF files have no text layer. OCR makes the text layer.

| Location | Engine | Use |
| --- | --- | --- |
| Server | OCRmyPDF with Tesseract | All server libraries. More than 100 languages. The server runs OCR as a background job. |
| Desktop | ocrs | Local libraries and encrypted projects. Languages with the Latin script. |

Both engines are behind the `OcrEngine` trait. The result of OCR has these parts:

- A copy of the PDF file with a text layer. The original file does not change.
- The text of each page, with the position of each word.
- A proposed page map, from the page numbers that OCR finds in the headers and footers.

A person can correct the page map. Presswork shows the page map next to the page images.

## Search

Presswork uses Tantivy for full-text search, on the desktop and on the server. Tantivy is a search library in Rust.

- The search index contains each page of each library file, with its work, its edition, and its printed page number.
- The same engine also finds the text in projects, tasks, tickets, research notes, chat messages, and assets. Refer to [Connections](connections.md).
- On the desktop, search operates offline.
- For encrypted projects, only the desktop has a search index. Refer to [Security](security.md).

## Annotations

- A person can highlight text on a page and add a note to the highlight.
- An annotation is private or shared with the team.
- If you drag a highlight into the editor, Presswork inserts the quotation and its citation, with the printed page number.

## Quote checker

The quote checker examines each quotation in a document:

1. It finds the quotations: `#quote` elements and quoted text with a citation.
2. It finds the cited page in the text of the standard edition.
3. It compares the quotation with the text of that page. Small differences in spaces, hyphens, and line breaks do not count.
4. It shows the result in the editor gutter: correct, different words, different page, or not found.

A checklist item "All quotations agree with the standard edition" can use this result automatically.

## Concordance

A concordance maps the page numbers of one edition to the page numbers of a different edition of the same work.

- Presswork makes a concordance automatically for two editions with the same text, for example two printings of one translation. It finds the same paragraphs in the OCR text of the two editions.
- For editions with different texts, for example two translations, a person makes the concordance. Presswork shows the two editions side by side for this job.

When the team changes a standard edition, Presswork uses the concordance to propose a new page number for each citation. The proposals are suggestions. A person accepts or rejects each one. Refer to [Editorial tools](editorial.md).

## Similarity check

The similarity check finds text in a document that agrees with the text of a library edition, but has no quotation marks or citation. It shows each match with the edition and the page. This helps authors to prevent accidental plagiarism.

## Citation styles

- Each publication has a citation style in the CSL format. Typst uses CSL styles.
- The citation style editor shows a sample of each citation form.
- You can select a style, or change a copy of a style for the house.

## Semantic search

Semantic search finds passages by meaning. It is available when AI assistance is on. Refer to [AI assistance](ai.md).

## Research notes

- A research note is a text in Markdown with links to other objects.
- A research note shows its backlinks.
- A research note is private or shared with the team.
- You can drag a research note into a chapter. Presswork inserts its text as Typst markup.

## Import

- Zotero exports: CSL JSON, BibTeX, and BibLaTeX files, with their attached files.
- Hayagriva YAML files.
- Lookup by ISBN through Open Library, and by DOI through Crossref. A lookup sends the identifier to the external service. Thus Presswork asks for approval before the first lookup.

## Access and rights

- Many library files have copyright. The server shows them only to members with the `library.read` permission.
- An administrator can make a part of the library read-only. Then members can read the pages in Presswork, but they cannot download the files.
- Presswork does not share library files outside of the organization.
- Clearances record the right to quote a text in a publication. Refer to [Production](production.md).
