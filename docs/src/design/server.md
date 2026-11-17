# Server

This chapter tells how the Presswork server operates.

## Parts

The server is one Rust program, `presswork-server`. It uses:

- axum and Tokio for HTTP and WebSocket.
- PostgreSQL for all records.
- A blob store for binary files. The blob store is a local folder or an S3-compatible service.

You can deploy the server as a container image, or as one executable with systemd.

## API

- The HTTP API uses JSON. All paths start with `/api/v1`.
- The live API uses one WebSocket at `/api/v1/live`. The client and the server use it for document sync, presence, and events.
- The `presswork-api` crate contains all request, response, and message types. The client and the server use the same types.
- The server publishes an OpenAPI description at `/api/v1/openapi.json`. Other programs can use the API with it.

## Accounts and sign-in

- A user signs in with an email address and a password. The server keeps an Argon2id hash of the password.
- After sign-in, the server gives an access token. The desktop application keeps the token in the credential store of the operating system.
- Users can make personal access tokens for scripts and CI.
- A later release adds sign-in through OpenID Connect providers.
- The server keeps only the hashes of tokens.

### Accounts in milestone 1

- Each account comes from an invite link (D102). A command on the server makes an organization and the invite link of its first administrator. After that, administrators make invite links through the API or with a command.
- An invite link is for one email address and one role: `administrator` or `member`. It is valid for 7 days and for one use. If the email address has an account, the person must give the password of that account.
- A password must have 12 characters or more (D113). A reset link is valid for 24 hours and for one use. A new password ends all sessions of the user and revokes all personal access tokens of the user (D151).
- A session lasts 30 days after its last use (D104).
- Each token has a prefix that tells its type: `pws_` for a session, `pwt_` for a personal access token, `pwi_` for an invite, and `pwr_` for a password reset. Thus secret scanners can find tokens in text.
- The server does not send email in milestone 1 (D103). The administrator sends each link.

These commands make links. They print the link and write nothing else to the standard output:

| Command | Result |
| --- | --- |
| `presswork-server create-organization --name "Field Press" --administrator ann@example.com` | A new organization, and an invite link for its first administrator. |
| `presswork-server invite --organization <ID> --email bob@example.com --role member` | An invite link. The default role is `member`. |
| `presswork-server reset-password --email ann@example.com` | A password reset link. |

The API has these operations for accounts:

| Operation | Use | Authentication |
| --- | --- | --- |
| `POST /api/v1/sessions` | Sign in, and get a session for this device. | None |
| `GET /api/v1/sessions` | List the sessions of the user. | Session or access token |
| `DELETE /api/v1/sessions/{id}` | End a session. | Session or access token |
| `POST /api/v1/access-tokens` | Make a personal access token. | Session |
| `GET /api/v1/access-tokens` | List the personal access tokens of the user, without the tokens. | Session or access token |
| `DELETE /api/v1/access-tokens/{id}` | Revoke a personal access token. | Session or access token |
| `POST /api/v1/invites/{token}/accept` | Accept an invite, and get a session. | None |
| `POST /api/v1/password-resets/{token}` | Set a new password. | None |
| `POST /api/v1/organizations/{id}/invites` | Make an invite link. | Session of an administrator |

A request gives its session token in the `Authorization: Bearer` header. An error answer is JSON with a `code` and a `message`. The codes are `unauthenticated`, `wrong_credentials`, `too_many_attempts`, `invalid_request`, `conflict`, `forbidden`, `not_found`, `link_not_valid`, and `internal`. The desktop application shows a translated text for each code.

The server limits the sign-in attempts (D148, D150):

- After 5 failed sign-ins for one email address in 15 minutes, the server refuses sign-ins for that email address for 15 minutes, also with the correct password. A correct sign-in before the limit removes the count of the failed sign-ins.
- After 20 failed sign-ins from one client address in 15 minutes, the server refuses sign-ins from that address for 15 minutes. All IPv6 addresses of one /64 network count as one address.
- A sign-in that is in progress also counts. Thus many sign-ins at the same time cannot go past the limit.
- A refused sign-in gets the status 429, the code `too_many_attempts`, and the `Retry-After` header with the number of seconds to wait.
- The server keeps the counts in memory. A restart removes them. The server keeps the count of an email address as a SHA-256 hash, and it keeps 100,000 counts or fewer of each type.
- The client address is the address of the connection. Behind a reverse proxy, set `trust_forwarded`. Then the server uses the last address in the `X-Forwarded-For` header. Set it only if the reverse proxy adds this header to each request, because a client can send a false header.

A personal access token is for scripts and CI (D149, D151):

- A token has a name with 1 to 100 characters and an optional expiry time in the future. It has the rights of its user in all operations and on the live WebSocket.
- The server shows the token only in the answer of `POST /api/v1/access-tokens`. It keeps only the SHA-256 hash of the token. The list shows the name, the time of creation, the expiry time, and the time of the last use.
- Only a session can make a token. Thus a stolen token cannot make more tokens.
- A user can have 100 tokens or fewer.

A request with a personal access token uses the same `Authorization: Bearer` header as a session. A sign-in with a password always makes a session.

### Projects and tasks in milestone 1

A project belongs to one organization. A task belongs to one project. A task has a title, a Markdown description, a state (`to_do`, `in_progress`, or `done`), one assignee, and a due date (D106). The due date is a calendar date, for example `2026-11-23` (D115). The assignee must be a member of the organization.

The `presswork-domain` crate contains the permission check (D114). Administrators can do all operations. Members can see all projects and tasks, make and change tasks, and delete the tasks that they made. If a user is not a member of the organization, the server answers 404, so that it does not tell which items exist.

| Operation | Use | Permission |
| --- | --- | --- |
| `GET /api/v1/organizations` | List the organizations of the user, with the role in each. | Session |
| `GET /api/v1/organizations/{id}/projects` | List the projects. | Member |
| `POST /api/v1/organizations/{id}/projects` | Make a project. | Administrator |
| `PATCH /api/v1/projects/{id}` | Rename a project. | Administrator |
| `GET /api/v1/projects/{id}/tasks` | List the tasks of a project. | Member |
| `POST /api/v1/projects/{id}/tasks` | Make a task. | Member |
| `PATCH /api/v1/tasks/{id}` | Change a task. Absent fields stay the same. `null` removes the assignee or the due date. | Member |
| `DELETE /api/v1/tasks/{id}` | Delete a task. | Administrator, or the member who made the task |

### Documents and live sync in milestone 1

A project has text documents (D126). Each document has a path in the project, for example `main.typ`. A path is unique in its project. It has 1 to 500 characters and uses `/` between its parts. It cannot start with `/`. It cannot contain an empty part, `.`, `..`, a control character, or the characters `\ : * ? " < > |`. Thus each path is also a valid file path on Windows, Linux, and macOS.

| Operation | Use | Permission |
| --- | --- | --- |
| `GET /api/v1/projects/{id}/documents` | List the documents of a project, in the sequence of their paths. | Member |
| `POST /api/v1/projects/{id}/documents` | Make an empty document. | Member |
| `GET /api/v1/live` | Open the WebSocket for live sync. Refer to [Collaboration](collaboration.md). | Session, in the `Hello` message |

- The server stores each accepted Loro update as one row of the `document_updates` table, with its author and its time (D125).
- For each document that has subscribers, the server keeps a Loro document in memory. When the last subscriber leaves, the server removes the document from memory.
- If the server cannot store an update, it ends all subscriptions of the document and loads the document from the database again. The clients subscribe again, and the version vectors bring back each change that the server did not store.
- A WebSocket message can have 16 MiB or less.
- In milestone 1, live sync operates in one server process (D127).

### Library in milestone 1

The library of an organization keeps works (D136). A work has a citation key, a title, a list of creators, and editions. A citation key is unique in its organization. It has 1 to 64 lowercase letters, digits, and single hyphens, and starts with a letter, for example `war-and-peace`. The `presswork-library` crate contains this rule, so that the desktop application can propose a valid key.

An edition has a year, a publisher, a language, translators, and one PDF file (D146). One edition of each work is its standard edition. A new work gets its first edition, which is the standard edition.

| Operation | Use | Permission |
| --- | --- | --- |
| `GET /api/v1/organizations/{id}/works` | List the works with their editions, in the sequence of their titles. | Member |
| `POST /api/v1/organizations/{id}/works` | Add a work and its first edition, without a file. | Member |
| `POST /api/v1/works/{id}/editions` | Add an edition to a work. | Member |
| `POST /api/v1/editions/{id}/standard` | Make an edition the standard edition of its work. | Member |
| `PUT /api/v1/editions/{id}/file` | Upload the PDF file of an edition, 64 MiB or less (D140, D143). A new file takes the place of the old file. The answer is 202 with the file in the state `reading`. | Member |
| `PUT /api/v1/editions/{id}/page-map` | Replace the corrections of the page map of an edition (D147). The edition must have a file in the state `ready`. | Member |
| `GET /api/v1/organizations/{id}/library/search?q=…` | Find the pages that have all the words, in all editions. Each result has the printed page number and tells if its edition is the standard edition. | Member |

- The upload request opens the PDF file and counts its pages (D143). It refuses a file that is not a PDF file or that is encrypted.
- A job then reads the text layer of each page with the `presswork-ocr` crate (D138) and sets the state `ready`. If the job cannot read the file, the state is `failed`, with a reason. Refer to "Jobs in milestone 1".
- The job also reads the page labels of the PDF file and keeps them in the `page_ranges` table. The corrections of the members are in the same table. The `presswork-library` crate calculates the printed page number of each page from the two lists, and the server keeps it with the text of the page.
- A page map has 100 corrections or fewer. A correction has a PDF page of the file, a style, a start number from 1 to 100,000, and a prefix of 20 characters or fewer, without control characters. A new file of the edition has no corrections.
- When the state of a file changes, the server sends `WorkChanged` on the live WebSocket to each client that joined the organization (D145).
- The server keeps each file in the `blobs` folder of its `data` folder (D137), and the text of each page in the `library_pages` table.
- The `presswork-search` crate keeps a Tantivy index of the pages in the `index` folder (D139). If the index is empty when the server starts, the server makes it again from the database.
- A search gives 20 pages or fewer, with the text around the words.

## Permissions

- To do an API operation, a user must have its permission, for example `ticket.create` or `document.write`.
- A role is a named set of permissions. Each organization can make its own roles from templates.
- A role assignment gives a role to a user or to a team, in a scope. The scope is the organization, a collection, a publication, or a component.
- The `presswork-domain` crate contains the permission check. It is a pure function, so tests can examine all rules.
- Each request handler checks the permission before it does the operation. The default is no access.

## Events

- Each change to a record makes a domain event, for example `TaskAssigned` or `StageChanged`.
- The server writes the event in the same database transaction as the change. This is an outbox table.
- PostgreSQL `LISTEN` and `NOTIFY` tell all server processes about new events.
- The server sends events to the clients that subscribe to them.
- The routine engine reads events to start triggered routines.
- The audit log keeps all events.

## Routine engine

- A scheduler starts routines at their scheduled times. A cron expression gives each schedule.
- An event trigger starts a routine when an event of the correct type occurs.
- The engine puts each routine run into a job queue in PostgreSQL. Workers take jobs with `FOR UPDATE SKIP LOCKED`.
- Each step of a routine is idempotent. If a worker stops during a run, a different worker can do the run again without problems.
- Compile and export steps run in separate worker processes, with limits for time and memory.

### Jobs in milestone 1

Milestone 1 has the job queue without routines (D142 to D144). The only kind of job is `read_text`, which reads the pages of a library file.

- The `jobs` table keeps each job with its kind, its subject (the ID of an object), its state (`queued`, `running`, `done`, or `failed`), its attempts, and the time from which it can run.
- The server adds a job in the same transaction as the record that needs it.
- Two worker tasks in the server process take jobs with `FOR UPDATE SKIP LOCKED`. A worker holds a job for 10 minutes. If the server stops during a job, a worker takes the job again after that time.
- A new job wakes the workers at once. The workers also look for jobs every 5 seconds.
- A job has 3 attempts. After a temporary error, it runs again after 10 seconds, and then after 1 minute. A file that Presswork cannot read fails at once.
- Each job is idempotent. A job for an object that does not exist any more ends without an error.

## Document storage

- The server keeps the Loro updates of each document in PostgreSQL.
- At intervals, the server compacts the updates of a document into a snapshot. This keeps loads fast.
- Revisions refer to Loro versions. Thus compaction keeps all versions that revisions use.

## Deployment

- The release workflow publishes a container image for each release.
- The repository contains a Docker Compose file with the server and PostgreSQL.
- The server reads its settings from a TOML file. An environment variable can replace each setting.
- The server has health endpoints for load balancers and container platforms.

## Settings

The environment variable `PRESSWORK_CONFIG` gives the path of the settings file. Without the variable, the server uses the default values and the environment variables.

| Setting | Environment variable | Default | Use |
| --- | --- | --- | --- |
| `listen` | `PRESSWORK_LISTEN` | `127.0.0.1:8080` | The address and the port of the API. |
| `database_url` | `PRESSWORK_DATABASE_URL` | None. The setting is necessary. | The PostgreSQL connection URL. |
| `data` | `PRESSWORK_DATA` | `data` in the working folder | The folder for the library files (`blobs`) and the search index (`index`). |
| `trust_forwarded` | `PRESSWORK_TRUST_FORWARDED` | `false` | If `true`, the client address for the sign-in limits is the last address in the `X-Forwarded-For` header. Use `true` only behind a reverse proxy that adds this header. |

The server refuses a settings file with an unknown key. Thus a key with a spelling error gives an error and is not ignored.

At the start, the server connects to PostgreSQL and applies the migrations that the database does not have. The server writes its logs as JSON lines to the standard output. The `RUST_LOG` variable sets the log level, for example `RUST_LOG=debug`.

## Health checks

| Path | Answer |
| --- | --- |
| `GET /api/v1/health/live` | 200 when the server process runs. |
| `GET /api/v1/health/ready` | 200 when the server can use its database. 503 when it cannot. |

Both answers contain the status and the version of the server.

## Database tests

The database tests use the PostgreSQL server that the `PRESSWORK_TEST_DATABASE_URL` variable gives. Each test makes its own database and applies the migrations. Without the variable, the database tests stop at once and write a note. Refer to D105 in [Decisions](decisions.md).

To run the database tests on your computer, start PostgreSQL in Docker:

```sh
docker run --name presswork-postgres -e POSTGRES_PASSWORD=presswork -p 5432:5432 -d postgres:17-alpine
```

Then set `PRESSWORK_TEST_DATABASE_URL=postgres://postgres:presswork@localhost:5432/postgres` and run `cargo test --package presswork-server`.

In CI, the "Database tests" job does the same with a PostgreSQL service. The job pins the image by its digest. Update the digest by hand when PostgreSQL publishes a new image.
