# AI assistance

This chapter tells how Presswork uses AI models. AI assistance is off by default. An administrator must turn it on for the organization.

## Principles

- AI proposes, and persons decide. Each AI result goes into a Presswork object that a person examines: a suggestion, a comment, a ticket, a draft record, or a draft data table.
- AI never changes a document directly. Each AI suggestion shows the provider and the model.
- AI uses the same links, search, and permissions as a person. Refer to [Connections](connections.md).
- AI acts only through Presswork commands. Thus an AI action gets the same checks as the same action by a person.
- AI requests go only to the provider that the administrator selects. Presswork does not collect usage data.

## Inline tools and the assistant

| Part | Description |
| --- | --- |
| Inline tools | Actions on a selection, for example "Copyedit with house style" or "Write alternative text". The result shows next to the selection. |
| Assistant | A panel for questions and requests about the project. It reads the project through tools, and it proposes changes through Presswork commands. |

The assistant can do these things:

- Read files, the outline, diagnostics, tickets, tasks, schedules, and the library, within the permissions of the person.
- Compile the project and examine page images.
- Propose changes. Each change becomes a suggestion, a draft ticket, or a draft task. The person accepts or rejects each one.

The assistant cannot apply a change, run a routine, or send a message without the approval of the person.

## Functions

| Area | Function | Result |
| --- | --- | --- |
| Editorial | Copyedit with the house style, the termbase, and the standard editions | Suggestions |
| Editorial | Rewrite the sentences that the controlled language checks flag | Suggestions |
| Editorial | Check claims and quotations against the library | Fact-check tickets with the passages and pages |
| Typst | Explain diagnostics, correct errors, and write show rules, functions, and templates | Suggestions in the code |
| Typst | Correct widows, orphans, and loose lines from the typesetting QA report with small text edits | Suggestions |
| Typst | Make a template from a sample page in a PDF file or an image | A draft template project |
| Typst | Write alternative text and captions | Suggestions |
| Typst | Propose index terms | Suggestions |
| Library | Answer questions with quoted passages and printed page numbers | Answers with links to the pages |
| Library | Propose edition records from scanned title pages | Draft edition records |
| Library | Read the data of a chart that is only an image | A draft data table, refer to [Typesetting tools](typesetting-tools.md) |
| Library | Find passages by meaning | Search results |
| Translation | Draft translations of segments, with the termbase and the translation memory | Draft segments |
| Management | Summarize the changes since your last visit | A summary with links |
| Management | Propose the type, the priority, the assignee, and duplicates for new tickets | Proposals on the ticket |
| Management | Explain the risks to a milestone and propose schedule changes | A proposal in the timeline |
| Management | Draft weekly status reports | A draft report |

## Providers

| Provider | Use |
| --- | --- |
| Anthropic API | Claude models. The default model for all functions is Claude Opus 5.5. An administrator can select a different Claude model. |
| Local models | A local model server with an OpenAI-compatible API, for example Ollama, llama.cpp, or vLLM. The content stays on the computers of the organization. |

Presswork calls the Anthropic Messages API directly over HTTPS, because there is no official Rust SDK. The adapter uses these API functions:

- Structured outputs, so that each result is a valid list of suggestions.
- Citations on PDF documents, so that each answer from the library has page locations.
- Prompt caching, so that repeated requests on the same book cost less.
- Message Batches, for overnight jobs at a lower price.
- Streaming, so that results show while the model writes them.
- The refusal stop reason. If the model refuses a request, Presswork shows the reason to the person. The adapter turns on the server-side refusal fallback of the API, which can answer with a different Claude model. Presswork never sends the request to a different provider.

## AI gateway

The Presswork server has an AI gateway. Desktops send AI requests through it.

- The gateway keeps the provider keys. Desktops never get the keys.
- The gateway applies the AI policy of the organization.
- The gateway removes personal data from requests, for example the email addresses and telephone numbers of contacts.
- The gateway applies the quotas and records each request in the AI audit log.
- The gateway sends whole-book jobs through Message Batches. The results arrive as suggestions.

A person without a server, or with a local model, can connect the desktop directly to a provider. Then the desktop applies the same policy, removal of personal data, and audit log locally.

## Policy and consent

An administrator sets the AI policy of the organization:

- Which functions are on.
- Which provider can get the content of which collections, publications, and parts of the library.
- Which data is sensitive. A request with sensitive data needs a confirmation each time.
- The inference region, where the provider supports it.

Before each request, the AI panel shows the content that Presswork sends and the provider that gets it. The person can stop the request. The AI audit log records each request.

If the provider has a setting that stops the use of content to train models, Presswork turns on this setting.

AI assistance is off in encrypted projects. A member can approve one request at a time.

## Controls

- **AI audit log.** It records the person, the function, the provider and the model, the objects, the tokens, and the cost of each request.
- **Quotas and cost reports.** Each organization and each person can have a monthly limit. The cost report shows the use by function and by person.
- **Removal of personal data.** The gateway removes personal data before a request leaves the server.
- **Provenance labels.** Each AI suggestion shows the provider and the model. Accepted text keeps an AI mark in its history. Thus a team can tell readers and partners where AI helped.

## MCP

Presswork has two MCP servers (Model Context Protocol). Other AI clients, for example Claude Desktop or Claude Code, can use them.

| Server | Location | Access |
| --- | --- | --- |
| Desktop MCP server | On the computer of the user, for the open projects | Only local clients that the user approves |
| Team MCP endpoint | On the Presswork server | OAuth sign-in, with the permissions of the person |

Both servers give tools to read projects, the library, tickets, and tasks. Changes from MCP clients become suggestions and proposals, the same as changes from the assistant.

## Large documents

- A whole book can go into one request, up to the context limit of the model. Presswork caches the book text, so that later requests on the same book cost less.
- Overnight jobs run as routine steps through Message Batches, for example a copyedit of a full book or alternative text for all images.

## Semantic search

Semantic search finds library passages by meaning. A local embedding model makes the search vectors on the desktop and on the server. Thus no text leaves the organization for this function. Semantic search is available only when AI assistance is on.

## Tests and quality

- Unit tests use recorded responses. No test calls a real provider.
- Each function has an evaluation set that measures the quality of its results. The team runs it before a change to a prompt or to a model.
- Prompts are files in the repository, with versions.
