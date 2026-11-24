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

In milestone 2, suggestion mode does these things (D198, D199, D200):

- To start or stop suggestion mode, click **Suggest** in the status bar or push Ctrl+Shift+E. On macOS, push Shift+Cmd+E. While suggestion mode is on, the button stays pressed. Suggestion mode stays on when you open a different project or a document from the server.
- Inserted text has a green background and a green line below it. Deleted text stays in the editor with a red background and a red line through it.
- If you type over a selection, the selection becomes a deletion, and the new text becomes an insertion after it. If you delete inserted text, the text goes away immediately, also when a different person suggested it.
- Each suggestion records its author and its time. When you are signed in to a server, the author is the name of your account. Without a sign-in, the author is the user name of the computer. Put the mouse pointer on a suggestion to see its author and its time.
- To accept or reject a suggestion, push Ctrl+. in it. A deletion and an insertion side by side from the same author are one suggestion. If the document has more suggestions, the list also shows "Accept all suggestions" and "Reject all suggestions". To accept an insertion keeps the text. To accept a deletion removes the text. Undo brings back the suggestion.
- The file on disk contains the text without the open suggestions: the deleted text is in the file, and the inserted text is not. Thus the export profiles and `presswork-cli` use the text without the suggestions.
- Presswork keeps the open suggestions of a file in the folder `.presswork/suggestions` of the project, for example in `.presswork/suggestions/main.typ.json`. When you open the project again, the suggestions come back. If the file changed outside Presswork, the suggestions do not agree with the file. Then Presswork moves them to a file that ends with `.json.old` and tells you in the notice bar.
- The preview shows the text as it is after you accept all suggestions. Errors, problem marks, and clicks on the pages go to the correct place in the editor.
- In a document from the server, the suggestions are marks in the Loro document. All persons who have the document open see a suggestion immediately, and each of them can accept or reject it. Presswork 0.1.0 does not know these marks: it shows the inserted text and the deleted text as text without suggestions.
- Undo and redo change only the suggestions that their step changed. Thus a suggestion that a different person made after your step stays.

## Spelling

- Presswork checks the spelling of the text parts of Typst files. It does not check code, math, or raw text.
- The spell checker uses Hunspell dictionaries through the `spellbook` crate. The toolchain manager downloads the dictionaries.
- The team has shared dictionaries for names and special words. Each person can also have a personal dictionary.
- The language of each part of the text comes from the Typst `lang` setting.

In milestone 2, the spell checker does these things (D186, D187, D188):

- You download the dictionaries in the Toolchains panel. There are 14 dictionaries: English (United States and United Kingdom), German (Germany, Austria, and Switzerland), French, Spanish, Italian, Dutch, Portuguese (Brazil and Portugal), Polish, Russian, and Danish. Refer to [Toolchains](toolchains.md).
- The checker examines the text parts of all Typst files that the document shows. It finds the language of each text part in the layout: the `lang` and `region` that Typst used for that text. A language without a region uses the first dictionary of that language in the list, for example `en` uses English (United States).
- It does not check words with digits, words in capital letters only, code, math, raw text, and text that the document does not show.
- A word with an apostrophe, for example "can't", is one word.
- The editor shows each misspelled word in the main file with a dotted line in the error color. The check runs on a separate thread after each compilation.
- A tooltip on a misspelled word tells the problem. Ctrl+. on the word shows the suggestions of the dictionary (D189).
- The file `presswork-words.txt` in the project folder is the word list of the team. It has one word on each line. Lines that start with `#` are comments. A word in small letters also permits the same word with capitals, for example `typst` permits "Typst". The editor reads the list again before each check.
- To add a misspelled word to the word list, push Ctrl+. on the word and select "Add “word” to the word list" (D190). The editor adds the word on a new line at the end of the file. If there is no file, the editor makes it. The editor removes the dotted lines of the word immediately.
- Each user also has a personal dictionary: the file `personal-words.txt` in the data folder of Presswork (D197). It has the same format as the word list. Its words are correct in all projects and in documents from the server. To add a word, push Ctrl+. on the word and select "Add “word” to the personal dictionary". The editor reads the file again before each check.
- `presswork-cli check` lists the misspelled words, and they make the check fail. It uses the word list of the project, but not the personal dictionary, thus all computers give the same result. If the document has text in a language without a dictionary, the command names the language one time. Without any dictionary, the command does not check the spelling.

## House style

A house style is a set of rules. Examples:

- Spelling variants: "organize", not "organise".
- Punctuation: an en dash for number ranges, and typographic quotation marks.
- Numbers: words for numbers below a limit, for example "ten", not "10".
- Terms: the preferred terms from the termbase.

Each rule has a pattern, a message, and an optional correction. Presswork shows each problem in the editor. A person can apply the correction or keep the text.

In milestone 2, the house style does these things (D189):

- The file `presswork-style.toml` in the project folder has the rules. Each `[[rule]]` table is one rule:

  ```toml
  [[rule]]
  text = "organise"
  replacement = "organize"

  [[rule]]
  text = "in order to"
  replacement = "to"
  message = "Use the short form."

  [[rule]]
  pattern = '(\d+)-(\d+)'
  replacement = '$1–$2'
  message = "Use an en dash between two numbers."
  ```

- A rule has `text` or `pattern`. `text` finds whole words. A text in small letters also finds the words with capitals, and the replacement keeps the capital of the first letter. `pattern` is a regular expression in the syntax of the Rust `regex` crate. In the replacement, `$1` gives the first group of the pattern.
- `replacement` and `message` are optional. Without a message, the tooltip tells that the house style does not permit the text.
- The rules examine the same text parts as the spell checker. They do not examine code, math, raw text, or text that the document does not show.
- The editor shows each problem with a dotted line in the warning color. The tooltip gives the message. Ctrl+. shows the replacement, and Enter puts it into the text.
- The editor reads the file again before each check. If the file is not valid, the notice bar tells the problem. Until the file is correct, the editor uses no rules.
- `presswork-cli check` lists the problems, and they make the check fail. A file that is not valid is an error.
- Rules from the termbase, rules for each language, and house style corrections as suggestions come later.

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
