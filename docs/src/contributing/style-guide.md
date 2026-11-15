# Style guide

All Presswork text follows ASD-STE100 Simplified Technical English (STE). This rule applies to:

- The README and the other files in the repository root.
- This book.
- Issue templates and pull request templates.
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

### Technical verbs

| Verb | Meaning |
| --- | --- |
| approve | To accept a revision in a workflow, as a person with the necessary role. |
| assign | To give a task or an issue to a person. |
| cache | To keep a copy of data for fast access. |
| clone | To make a local copy of a Git repository. |
| commit | To record changes in Git. |
| compile | To make a document from Typst sources. |
| configure | To set the options of a program. |
| create | To make a new record, file, or project in a program. |
| delete | To remove data. You cannot get it back. |
| deploy | To install a server program on its computer. |
| download | To copy a file from a server to the local computer. |
| edit | To change the content of a file or a record. |
| export | To write a document to a file format, for example PDF. |
| implement | To write the code for a trait, an interface, or a function. |
| import | To read a file from a different format into a project. |
| install | To put a program on a computer so that it can operate. |
| merge | To make one set of changes from two sets of changes. |
| parse | To read text and find its structure. |
| publish | To make a package, an image, or a site available to other persons. |
| push | To send Git commits to a remote repository. |
| render | To make an image from a page of a document. |
| schedule | To set the time when a routine starts. |
| scroll | To move the visible part of a view. |
| sign in | To give your account data to a server to get access. |
| store | To keep data in a database or on a disk. |
| sync | To make two copies of data the same. |
| translate | To write content in a different language. |
| typeset | To put text and images on pages in a layout. |
| unpack | To get the files out of an archive. |
| upload | To copy a file from the local computer to a server. |
| validate | To examine a file for agreement with a standard. |

### Technical names

| Name | Meaning |
| --- | --- |
| anchor | The link between an issue or a comment and a text range or a page area. |
| blob | A binary file that the server keeps by its digest. |
| checklist | A list of items that persons or automatic checks must complete. |
| collection | A group of publications, for example a periodical or a book series. |
| component | A part of a publication with its own workflow stage, for example a chapter. |
| CRDT | Conflict-free replicated data type. A data structure that merges concurrent edits automatically. |
| digest | The SHA-256 hash of a file. |
| edition | One number of a periodical. |
| issue | An item in the issue list: a problem, a question, or a correction. |
| manifest | The `presswork.toml` file of a project. |
| overlay | Text from the editor that replaces a file on disk during compilation. |
| pin | A Typst version requirement in a project manifest. |
| preflight | The checks before an export. |
| publication | One work that you can publish, for example a book or an edition. |
| publishing | The work that makes publications and makes them available. |
| query | An issue type: a question to the author. |
| revision | A named point in the history of a project. |
| role | A named set of permissions. |
| routine | A sequence of steps that a schedule or an event starts. |
| stage | A state in a workflow, for example Copyedit. |
| task | Planned work for a person. |
| toolchain | One installed version of an external tool, for example Typst 0.14.2. |
| typesetting | The work that puts text and images on pages. |
| workflow | A set of stages and transitions for a publication or a component. |
| workspace | A saved pane layout for one activity, for example Write. |
