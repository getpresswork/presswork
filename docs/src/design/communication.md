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

## Inbox and notifications

- The inbox of each person contains mentions, assignments, replies, approvals, and the events of watched objects.
- Each person selects which events make a notification: in the application, through the operating system, or by email.
- A person can get a summary email each day or each week, in place of single emails.

## Webhooks

An administrator can send events to other systems through webhooks, for example to Matrix, Slack, or Discord. Each webhook has a filter for the event types.

## Web portal

The web portal is a web site on the Presswork server. It is for persons who do not use the desktop application, for example authors, outside reviewers, and freelancers.

- An invitation link gives a person access to some objects only, for a limited time.
- In the portal, a person can answer queries, examine proofs, add comments to pages, approve a revision, and upload files.
- The server renders the proof pages. Thus the browser does not run Typst.
- The portal uses HTML that the server renders, with small scripts. It follows WCAG 2.2 at level AA.
- The portal is not available for encrypted projects.

## Submissions

A journal or an anthology can open a call for papers:

1. The team publishes a submission form on the portal.
2. A person sends a manuscript through the form.
3. Presswork makes a submission record with the files and the metadata.
4. An editor assigns reviewers. The review is single-blind or double-blind.
5. The editor records the decision: accept, revise, or reject.
6. For an accepted submission, Presswork makes a component in a publication and starts its workflow.
