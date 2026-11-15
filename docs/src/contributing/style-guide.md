# Style guide

All Presswork text follows ASD-STE100 Simplified Technical English (STE). This rule applies to:

- The README and the other files in the repository root.
- This book.
- GitHub issue templates and pull request templates.
- Interface text: labels, messages, and tooltips.
- Commit messages.

The official specification is free from the [ASD-STE100 website](https://www.asd-ste100.org). The specification is the authority. This page gives a summary of the rules and the project dictionary.

## Words

- Use only approved words, with their approved meanings.
- Use the technical names and technical verbs of the project dictionary below.
- Use one word for one meaning. Use the same word for the same thing each time.
- Do not use contractions. Write "do not", not "don't".

These replacements occur frequently:

| Do not use | Use |
| --- | --- |
| utilize | use |
| begin, commence | start |
| ensure | make sure |
| obtain | get |
| provide | give, supply |
| perform | do |
| display | show |
| assist | help |
| modify | change |
| require | must, necessary |
| prior to | before |
| in order to | to |
| via | through, with |
| e.g. | for example |

## Sentences

- Write procedural sentences with 20 words or fewer.
- Write descriptive sentences with 25 words or fewer.
- Write one instruction in each sentence.
- Use the active voice.
- Use only the simple tenses: present, past, and future.
- Do not use the -ing form of a verb. This rule also applies to headings. Write "Test strategy", not "Testing strategy".
- Use articles ("the", "a") where possible.
- Do not make noun clusters of more than three words.

## Paragraphs and procedures

- Write about one topic in each paragraph. Start with the topic sentence.
- Write six sentences or fewer in each paragraph.
- Write procedures as numbered steps. Write each step as a command.
- Put a condition before its instruction: "If the build fails, examine the log."
- Write a warning or a caution as a command, then give the reason.

## Interface text

- Write labels and buttons as commands: "Export PDF", "Install version".
- Write messages that tell what occurred and what to do: "The download of Typst 0.14.2 failed. Examine the network connection, then try again."
- Do not use humor or idioms. Translators must be able to translate each message.

## Commit messages

Write the summary of a commit as a command, with approved words: `fix(toolchain): check digest before unpack`. Refer to [Workflow](workflow.md).

## Project dictionary

STE lets each project add technical names and technical verbs. These are the Presswork words. Add a word here before you use it in the documentation.

### Words with one meaning

Some words have a different meaning in daily language. In Presswork text, use them only with these meanings:

| Word | Presswork meaning | Do not use it for |
| --- | --- | --- |
| issue | One number of a periodical | An item in the ticket list. Use "ticket". |
| ticket | An item in the ticket list | |
| edition | A published form of a work in the library | A number of a periodical. Use "issue". For the Rust language, write "Rust edition". |
| query | A ticket type: a question to the author | Database statements. Write "SQL statement". |
| work | A text as an abstract creation, in the library | Labor or activity. Use "task", "job", or a verb. |
| workspace | A pane layout in the desktop application | The Cargo workspace. Write "Cargo workspace". |
| digest | The SHA-256 hash of a file | An email with many events. Use "summary email". |
| thread | A thread of a program | Replies in the chat. Use "replies" or "comments". |
| component | A part of a publication | WebAssembly modules. Use "WebAssembly module". |
| source | The original text from which a result comes: Typst source files, or the source text of a translation | Items in the library. Use "work" or "edition". |
| index | A back-of-book index | The search index. Write "search index". |
| permission | The right to do an operation in Presswork | The right to reuse a text or an image. Use "clearance". |
| signature | A group of pages that a printer folds and binds together | Cryptographic data. Write "digital signature". |

### Technical verbs

| Verb | Meaning |
| --- | --- |
| approve | To accept a revision in a workflow, as a person with the necessary role. |
| assign | To give a task or a ticket to a person. |
| bump | To increase the version number of a release. |
| cache | To keep a copy of data for fast access. |
| clone | To make a local copy of a Git repository. |
| collate | To put the corrections of many persons into one list. |
| commit | To record changes in Git. |
| compile | To make a document from Typst sources. |
| configure | To set the options of a program. |
| create | To make a new record, file, or project in a program. |
| decrypt | To make encrypted data readable with a key. |
| delete | To remove data. You cannot get it back. |
| deploy | To install a server program on its computer. |
| detach | To move a pane into its own window. |
| download | To copy a file from a server to the local computer. |
| edit | To change the content of a file or a record. |
| encrypt | To make data unreadable for persons without the key. |
| export | To write a document to a file format, for example PDF. |
| highlight | To mark a part of the text on a page of a library file. |
| implement | To write the code for a trait, an interface, or a function. |
| import | To read a file from a different format into a project. |
| install | To put a program on a computer so that it can operate. |
| lift | To rebuild content from a PDF page as Typst code. |
| mention | To refer to a person or an object in a text with `@`, `#`, or `[[`. |
| merge | To make one set of changes from two sets of changes. |
| parse | To read text and find its structure. |
| publish | To make a package, an image, a release, or a site available to other persons. |
| push | To send Git commits to a remote repository. |
| render | To make an image from a page of a document. |
| schedule | To set the time when a routine or a task starts. |
| scroll | To move the visible part of a view. |
| sign | To add a digital signature to a file. |
| sign in | To give your account data to a server to get access. |
| sign off | To record that a manual checklist item is complete. |
| store | To keep data in a database or on a disk. |
| sync | To make two copies of data the same. |
| track | To record the progress, the time, or the costs of a job. |
| translate | To write content in a different language. |
| typeset | To put text and images on pages in a layout. |
| unpack | To get the files out of an archive. |
| upload | To copy a file from the local computer to a server. |
| validate | To examine a file for agreement with a standard. |

### Technical names

| Name | Meaning |
| --- | --- |
| AI gateway | The part of the Presswork server that sends AI requests, with the policy, the quotas, and the audit log. |
| anchor | The link between a ticket or a comment and a text range or a page area. |
| annotation | A highlight or a note on a page of a library file. |
| assistant | The AI panel that answers questions and proposes changes through Presswork commands. |
| asset | An image or a graphic in the asset library, with its credits and its rights data. |
| baseline grid | The grid of lines on which the body text of a page design sits. |
| binding | The method that holds the pages of a printed book together, for example a perfect binding. |
| blob | A binary file that the server keeps by its digest. |
| brand kit | A team package with the fonts, colors, page designs, and series designs of an organization. |
| budget | The planned costs of a publication. |
| build report | The record of one compilation or export: versions, inputs, fonts, packages, warnings, times, and digests. |
| channel | A named conversation in the team chat, for example `#production`. |
| checklist | A list of items that persons or automatic checks must complete. |
| clearance | A record of the agreement of a rights holder to the use of a text or an image. |
| collection | A group of publications, for example a periodical or a book series. |
| component | A part of a publication with its own workflow stage, for example a chapter or an article. |
| concordance | A map between the page numbers of two editions of a work. |
| contact | A person or an organization outside of the team, for example an author or a printer. |
| content block | A piece of Typst content that many publications use, for example a copyright page. |
| contract | A record of an agreement with a contact, with its deliverables and fees. |
| copyfitting | The adjustment of a text and its layout to a target page count. |
| cost item | One planned or actual cost in a budget. |
| CRDT | Conflict-free replicated data type. A data structure that merges concurrent edits automatically. |
| data merge | The use of a data file to make the content of a document, for example a directory. |
| deliverable | A result that a contract makes necessary, for example a manuscript. It becomes a task or a milestone. |
| dependency | A relation between two tasks: one task cannot start or finish before the other task. |
| device key | The key pair of one computer of a user, for encrypted projects. |
| digest | The SHA-256 hash of a file. |
| digital signature | Cryptographic data that shows who made a file, and that the file did not change. |
| direct message | A chat conversation between two or more persons, outside of channels. |
| edition | One published form of a work, for example an original, a translation, or a reprint. It has its own page numbers. |
| embedding model | A model that changes text into vectors for semantic search. |
| folding | An editor function that hides parts of the text and shows them again. |
| follow mode | A mode in which your view follows the view of a different member. |
| heading | The title of a part, a chapter, or a section in a document. |
| history slider | A control that shows the text of a document at an earlier time. |
| home dashboard | The start page with your tasks, your inbox, deadlines, and recent projects. |
| inbox | The list of mentions, assignments, replies, and approvals for one person. |
| issue | One number of a periodical, for example Issue 42. |
| leading | The vertical distance between the baselines of two lines of text. |
| manifest | The `presswork.toml` file of a project. |
| MCP | Model Context Protocol. A protocol that lets AI clients use the tools of other programs. |
| milestone | A date in a schedule that marks the end of a phase, for example "Proofs to author". |
| nightly build | A prerelease build that CI makes each day when `main` changes. |
| OCR | Optical character recognition. It makes a text layer from the images of scanned pages. |
| overlay | Text from the editor that replaces a file on disk during compilation. |
| page map | The relation between the pages of a file and the printed page numbers of its edition. |
| pin | A Typst version requirement in a project manifest. |
| portal | The web site of a Presswork server for persons who do not use the desktop application. |
| preflight | The checks before an export. |
| print plan | The division of the page count into signatures, with the cost estimates. |
| prompt caching | An API function that keeps the start of a request, so that later requests with the same start cost less. |
| proof mark | A standard mark that a proofreader puts on a proof page to show a correction. |
| proof round | One set of proof pages for a revision, for example the first proofs. |
| provenance label | A label on an AI suggestion that shows the provider and the model. |
| project key | The key that encrypts the content of an encrypted project. |
| publication | A book, an issue of a periodical, a manual, or a paper that a team makes and publishes. |
| publishing | The production and the distribution of publications. |
| purpose | The use of a standard edition, for example "English quotations". |
| query | A ticket type: a question to the author. |
| reading order | The sequence in which a screen reader reads the parts of a page. |
| release train | The scheduled release that CI makes every two weeks. |
| rendition | A different form of an asset, for example a thumbnail or a CMYK version. |
| research note | A text with links to works, editions, chapters, and tasks. |
| revision | A named point in the history of a project. |
| rights holder | A person or an organization that owns the rights to a text or an image. |
| role | A named set of permissions. |
| routine | A sequence of steps that a schedule or an event starts. |
| running head | The line of text at the top of each page, for example the title of the chapter. |
| retention rule | A rule that deletes a type of data after a period. |
| scratchpad | A Typst document outside of projects, with an instant preview. |
| segment | A sentence or a short part of a text in the translation workspace. |
| signature | A group of pages that a printer folds and binds together, for example 16 pages. |
| soft proof | A simulation of the printed colors on the screen. |
| spelling | The correct letters of the words in a language. The spell checker examines it. |
| stage | A state in a workflow, for example Copyedit. |
| standard edition | An edition that a team selects for a work and a purpose. |
| streaming | An API function that sends the result in parts while the model writes it. |
| structure board | A view that shows the parts and chapters of a publication as cards. |
| submission | A manuscript that a person sends through a call for papers. |
| suggestion | A proposed insertion or deletion in suggestion mode. The author accepts or rejects it. |
| summary email | One email that contains the events of a day or a week. |
| support bundle | A collection of logs, versions, and settings, without personal data, for a problem report. |
| task | An assignment for a person, with dates and a status. |
| termbase | The approved and forbidden terms of a team, with definitions and translations. |
| text layer | The text of a PDF page that tools can find and copy. |
| ticket | An item in the ticket list: a query, a correction, a problem, or a different type that the team adds. |
| timesheet | The time entries of one person for a period. |
| toolchain | One installed version of an external tool, for example Typst 0.14.2. |
| tracking | The space that is added between all letters of a text. |
| translation memory | A store of source segments and their approved translations. |
| typesetting | The layout of text and images on pages. |
| variant | One output of a publication from the same source, for example a print variant. |
| watermark | Text on each page of a proof that identifies the person who got the proof. |
| work | A text as an abstract creation, for example *War and Peace*. A work has one or more editions. |
| workflow | A set of stages and transitions for a publication or a component. |
| workspace | A saved pane layout for one activity, for example Write. |
