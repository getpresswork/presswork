# Communication

This chapter tells how team members and outside contributors communicate in Presswork.

## Chat

- **Channels** are named conversations for a team, a publication, or a subject. A channel is open to the organization or private.
- **Direct messages** are conversations between two or more persons.
- **Replies** keep a discussion about one message together, in a separate list.
- **Comments** are on each object, for example a task, a ticket, a chapter, an edition, or a revision.
- Messages use Markdown. They can contain links to objects, with hover cards. Refer to [Connections](connections.md).
- Messages can contain files. The server keeps them as blobs.
- The chat and the inbox can open in their own window.
- In an encrypted project, the channels of the project are encrypted. Refer to [Security](security.md).

### Chat in milestone 1

Milestone 1 has one channel for each organization (D133). All members of the organization can read and write it. A message has plain text of 1 to 4000 characters. The server stores the messages in the `chat_messages` table, with the author and the time.

The chat uses the live WebSocket (D134):

1. The client sends `Join` with the ID of the organization. The server answers `Joined` with the last 100 messages, oldest first.
2. The client sends `Say` with the text. The server stores the message, and then sends `Said` with the stored message to each client that joined, also to the author.
3. If a client is not a member of the organization, the server answers `Error` with the code `not_found`. If a client sends `Say` before `Join`, or the text is empty or too long, the code is `invalid_request`.

A message can arrive in `Joined` and in `Said`. The client keeps each message one time, by its ID.

In the desktop application, the **Chat** button in the status bar opens the chat of the organization of the open server document. The chat panel opens at the bottom of the window. Only one of the log panel, the Toolchains panel, and the chat panel is open at a time.

## Inbox and notifications

- The inbox of each person contains mentions, assignments, replies, approvals, and the events of watched objects.
- Each person selects which events make a notification: in the application, through the operating system, or by email.
- A person can get a summary email each day or each week, in place of single emails.

## Email bridge

- Each publication can have an email address on the server.
- Presswork can send queries to an author by email. The reply of the author becomes a comment on the query.
- Files that persons send to the address of a publication become uploads, or tickets with attachments.
- The email bridge accepts mail only from known contacts. Other mail goes to a review list.

## Webhooks

An administrator can send events to other systems through webhooks, for example to Matrix, Slack, or Discord. Each webhook has a filter for the event types.

## Web portal

The web portal is a web site on the Presswork server. It is for persons who do not use the desktop application, for example authors, outside reviewers, and freelancers.

- An invitation link gives a person access to some objects only, for a limited time.
- In the portal, a person can answer queries, examine proofs, add comments to pages, approve a revision, and upload files.
- The server renders the proof pages. Thus the browser does not run Typst.
- The portal uses HTML that the server renders, with small scripts. It follows WCAG 2.2 at level AA.
- The portal is not available for encrypted projects.
- Members can also use the portal on a phone for the inbox, approvals, tasks, and chat. Typesetting stays on the desktop.
- Web push notifications go to phones and browsers without an app store.

## Watermarked proofs

- Each proof PDF that a person downloads or gets has a watermark with their name and the date.
- A share link for proofs has an expiry date. You can close a link at any time.
- The audit log records each download.

## Submissions

A journal or an anthology can open a call for papers:

1. The team publishes a submission form on the portal.
2. A person sends a manuscript through the form.
3. Presswork makes a submission record with the files and the metadata.
4. An editor assigns reviewers. The review is single-blind or double-blind.
5. The editor records the decision: accept, revise, or reject.
6. For an accepted submission, Presswork makes a component in a publication and starts its workflow.
