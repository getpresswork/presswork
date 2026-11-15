# Editorial tools

This chapter tells how Presswork helps authors, editors, copyeditors, proofreaders, indexers, and translators.

## Suggestion mode

In suggestion mode, an edit does not change the text directly. It becomes a suggestion:

- An insertion shows as marked text.
- A deletion shows with a line through the text.
- Each suggestion records the person and the time.

The author accepts or rejects each suggestion, or all suggestions in a range. Suggestions are marks in the Loro document, so they also operate during live edits.

The file on disk contains the text without the open suggestions. The preview can show the text with or without the suggestions.

Other functions also make suggestions: the concordance, the house style corrections, and AI assistance. Thus a person always approves a change that a program proposes.

## Spelling

- Presswork checks the spelling of the text parts of Typst files. It does not check code, math, or raw text.
- The spell checker uses Hunspell dictionaries through the `spellbook` crate. The toolchain manager downloads the dictionaries.
- The team has shared dictionaries for names and special words. Each person can also have a personal dictionary.
- The language of each part of the text comes from the Typst `lang` setting.

## House style

A house style is a set of rules. Examples:

- Spelling variants: "organize", not "organise".
- Punctuation: an en dash for number ranges, and typographic quotation marks.
- Numbers: words for numbers below a limit, for example "ten", not "10".
- Terms: the preferred terms from the termbase.

Each rule has a pattern, a message, and an optional correction. Presswork shows each problem in the editor. A person can apply the correction or keep the text.

## Termbase

The termbase keeps the terms of the team:

- The preferred term and its forbidden variants.
- A definition and an example.
- Translations into other languages.

The house style rules and the translation workspace use the termbase. Presswork can import and export the termbase in the TBX format.

## Back-of-book index

Typst has no built-in back-of-book index. Presswork supplies a Typst package for indexes and publishes it on Typst Universe. Thus a project with an index compiles without Presswork.

- You mark index terms in the text with a command or a shortcut.
- The index panel shows all terms as a tree, with main entries, subentries, and cross-references.
- You can merge, rename, and move terms in the tree. Presswork then changes the marks in the text.

## Translation workspace

The translation workspace shows the source text and the target text side by side:

- Presswork divides the text into segments, for example sentences. It keeps the Typst markup out of the segments.
- The translation memory proposes earlier translations of the same or similar segments.
- The termbase shows the approved translations of the terms in each segment.
- Each segment has a status: new, draft, translated, or approved.
- Presswork can export and import XLIFF files for other translation tools.

## Proof collation

- A proof round is one set of proof pages for a revision, for example the first proofs.
- Proofreaders mark the pages with standard proof marks, in the desktop application or in the web portal.
- Presswork collates the corrections of all proofreaders into one list for the proof round. It merges corrections at the same location, and it shows the conflicts between them.
- Each correction links to its page area and to its source text. The typesetter applies it as a suggestion, or rejects it with a reason.
- The next proof round shows which corrections are complete.

## Accessibility audit

The accessibility audit examines the compiled document and the exports for these items:

- Alternative text for each image.
- A correct sequence of heading levels.
- The reading order.
- Header rows in tables.
- The language of each part of the text.
- The color contrast of text in figures.

The results show in the preflight panel. A checklist item can use them. The audit applies to PDF/UA files and to EPUB files.

## Controlled language checks

- An organization can turn on controlled language rules, for example the rules of ASD-STE100 for text.
- The rules examine the sentence length, the -ing forms, the passive voice, the noun clusters, and the lists of approved and forbidden words.
- The organization loads its own dictionary. Presswork does not include the ASD-STE100 dictionary, because ASD owns its copyright.
- Problems show in the editor, in the same way as spelling problems. AI assistance can propose a rewrite.

## Find and replace in the organization

- Find a word or a pattern in all publications of the organization.
- Replace it in all locations. Each replacement is a suggestion in its publication.
- Presswork does not change publications after their release.

## Read aloud

The read-aloud function reads the text aloud. It marks each word while it speaks the word. This helps proofreaders find repeated words and words that are not there. Presswork uses the speech functions of the operating system.
