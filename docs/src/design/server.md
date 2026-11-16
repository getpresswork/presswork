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
