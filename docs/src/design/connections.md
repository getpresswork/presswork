# Connections

This chapter tells how the parts of Presswork connect. All parts use the same links, the same events, and the same search. Thus a change in one part shows in all related parts.

## Links

Each object has a stable link. Objects are, for example, publications, chapters, files, revisions, tasks, tickets, works, editions, annotations, research notes, assets, persons, channels, and chat messages.

You can mention an object in each text field: in comments, in tickets, in tasks, in research notes, and in chat messages.

| Type | Objects | Examples |
| --- | --- | --- |
| `@` | Persons and teams | `@ana`, `@copyedit-desk` |
| `#` | Tasks and tickets | `#118`, `#204` |
| `[[` | All other objects | `[[Chapter 3]]`, `[[War and Peace]]`, `[[Revision 12]]` |

Tasks and tickets use one number sequence in each organization. Thus `#118` always identifies one object.

When you type the character, a list of objects opens. Type more letters to make the list shorter.

Typst files stay normal Typst files. In Typst files, these objects connect to the text:

- A citation, for example `@war-and-peace[p. 412]`, links to the cited edition in the library.
- A Typst comment can mention a task or a ticket, for example `// #118`. The editor shows the mention as a link.
- An anchor connects a ticket or a comment to a text range. Refer to [Collaboration](collaboration.md).

## Backlinks

Each object shows the objects that link to it. For example, an edition shows the chapters that cite it, the tickets about it, and the chat messages that mention it.

## Related panel

The Related panel shows the objects that relate to the current selection:

| Selection | Related objects |
| --- | --- |
| Text in the editor | The tickets and comments on the text, the task for the chapter, the cited editions, the history of the text |
| A task | Its chapter, its checklist, its time entries, its dependencies, its comments |
| A work or an edition | The chapters that cite it, the quotations from it, its annotations, its clearances |
| An asset | The pages that use it, its clearance, its renditions |
| A person | Their tasks, their open tickets, their time entries of this week |

## Hover cards

Put the mouse pointer on a link to see a hover card. The card shows the main data of the object and its main actions. For example, a task card shows the status, the assignee, and the due date. It also has buttons to complete the task and to open it.

## Events

Each change makes an event. Refer to [Server](server.md). The events go to these functions:

- The activity list of each object.
- The inbox of each person.
- The chat, when a person shares an event in a channel.
- The routines that start on an event.
- The reports.

## Search

One search finds all objects: the text in projects, the pages in the library, tasks, tickets, research notes, chat messages, and assets. Refer to [Library](library.md) for the search engine.

## Drag and drop

| Drag | Drop on | Result |
| --- | --- | --- |
| A highlight in a library file | The editor | The quotation, with its citation and page |
| A chapter | A person | A new task for that person on that chapter |
| A task | A day in the calendar | A new due date |
| An asset | The editor | An `#image` call, with the caption and the credit |
| A ticket | A revision | A link from the ticket to that revision |

## Status in context

Each view shows the state of the related objects at the location of your cursor or selection:

- The outline shows the workflow stage of each chapter.
- The editor gutter shows tickets, suggestions, spelling problems, and the results of the quote checker.
- The preview shows comments and preflight problems on the page.
- The status bar shows the open tasks and tickets of the current chapter.

## Rules for new functions

Each new function must:

1. Give a stable link to each of its objects.
2. Make an event for each change.
3. Add its objects to the search index.
4. Show its objects in the Related panel and in hover cards.
5. Add its actions to the command palette.
