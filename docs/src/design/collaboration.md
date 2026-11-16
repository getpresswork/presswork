# Collaboration

This chapter tells how team members edit the same project.

## Goals

- Many persons can edit the same document at the same time. Each person sees the cursors of the others.
- Edits that you make offline merge without conflicts when the client connects again.
- A team can record named revisions, compare them, and approve them.
- The files on disk stay normal Typst files. Thus other tools can use them.

## Documents

Presswork keeps shared project content in [Loro](https://loro.dev) documents. Loro is a CRDT library. A CRDT merges concurrent edits automatically and gives the same result on all computers.

A server project has these parts:

| Part | Storage |
| --- | --- |
| The file tree: paths, file kinds, and metadata | One Loro document with a movable tree |
| Text files, for example `.typ`, `.bib`, `.yml`, `.toml`, and `.csv` files | One Loro document for each file, with a text container |
| Binary files, for example images, fonts, and PDF files | Blobs. The file tree keeps the digest of each blob. |

One document for each text file lets the server give permissions for each file. It also lets a client load only the files that it uses.

## Working copy

A local folder can link to a server project. Then the folder is a working copy:

- The `.presswork` folder in the project root keeps the Loro documents and the sync state.
- Presswork writes each text file to disk after each change. Thus other tools always see the current text.
- A file watcher finds the changes that other tools make. Presswork calculates the difference and applies it to the Loro document as a normal edit.

The `.presswork` folder contains local data. Do not add it to version control.

## Sync protocol

The client and the server send messages through one WebSocket at `/api/v1/live`. Each message is a MessagePack map with field names (D124). The `presswork-api` crate defines the messages.

1. The client sends `Hello` with the protocol version and its session token (D128). The server answers `Welcome` with the ID of the user. If the first message is not a valid `Hello` in 10 seconds, the server sends `Error` and closes the connection.
2. For each document, the client sends `Subscribe` with its version vector.
3. The server answers `Subscribed`. It contains all changes that the client does not have, and the version vector of the server. The client then sends `Updates` with the changes that the server does not have, for example the changes that it made offline.
4. After this, each side sends `Updates` for its new changes. The server sends each update to the other subscribers of the document.
5. The server sends `Ack` with the ID of an update when it has stored the update. The client keeps each change until it gets the `Ack` for it.
6. If the server cannot do a request, it sends `Error` with a code and a message. The codes are the codes of the HTTP API, and `unsupported_protocol`.

A version vector tells which changes a peer has. Thus the protocol sends each change only one time. This is also true after a long time offline.

The `presswork-sync` crate keeps the text model and the Loro document of a file together (D123). It turns each local change set into a Loro update. It turns each remote update into a change set with one edit for each changed area. Thus the cursors and the undo history of the user stay correct.

## Presence

Presence data is temporary. The server does not store it. It contains:

- The user and a color.
- The open file, the cursor, and the selection ranges.
- The current workspace, for example Write or Review.

Cursor ranges use Loro stable cursors. Thus a remote cursor stays at the correct text when other persons edit the document.

In follow mode, your view follows the view and the cursor of a different member, for example when the team examines a chapter together. The member sees who follows them.

## History slider

The history slider shows the edit history of a document. Move the slider to see the text at an earlier time, with the author of each change. You can restore the text of any point as a suggestion. The slider uses the history of the Loro document.

## Revisions

A revision is a named point in the history of a project. It records:

- The Loro version of each document.
- The digest of each binary file.
- The author, the time, a message, and the current workflow stage.

Presswork can show the differences between two revisions, file by file and page by page. An approval is always for one revision. A release export always uses a revision.

## Anchors

Tickets and comments can attach to a text range or to an area of a page.

- A text anchor uses two Loro stable cursors. The anchor moves with its text when other persons edit the document.
- If a person deletes all of the text of an anchor, the anchor becomes detached. Presswork keeps the original text and shows the ticket as detached.
- A page anchor records the revision, the page number, and a rectangle. It also records a text anchor for the source at that point, if the engine can find one.

## Offline operation

- All edits go into the local Loro documents first. The network status does not change the edit procedure.
- Tickets, tasks, and checklists also have a local copy. Changes to them go into a local queue.
- When the client connects again, it sends the queue in sequence. If the server does not accept a change, Presswork shows the change and the reason. For example, a permission can change while you are offline.

## Co-publishing

- An organization can share one publication with a partner organization on the same server.
- Each organization gives its own roles for the shared publication.
- Each organization sees only the shared publication, not the other records of the partner.

## Git

Git is not the sync method of Presswork. But many technical users use Git, so Presswork has a bridge:

- Export a revision as a Git commit to a repository.
- Import a Git repository as a new project.
- Mirror each new revision to a Git remote automatically, with a routine.
