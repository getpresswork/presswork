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

- To do an API operation, a user must have its permission, for example `issue.create` or `document.write`.
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
