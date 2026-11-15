# Operations

This chapter tells how administrators operate a Presswork server.

## Admin console

The admin console is a part of the web portal for administrators. It shows:

- Users, organizations, teams, and roles.
- The storage use and the quota of each organization.
- The audit log, with filters.
- The job queues: OCR, exports, routines, AI batch jobs, and summary emails.
- The health of the server: the database, the blob store, the search index, and the workers.
- The AI policy and the AI cost reports.

## Backups and restore

- The server makes scheduled backups of the database and the blobs.
- Point-in-time recovery uses the write-ahead log of PostgreSQL.
- A restore test runs on a schedule. It restores the last backup into a temporary database and checks it.
- The backups of encrypted projects stay encrypted.

## Enterprise deployment

- The desktop application uses the proxy settings and the certificate authorities of the operating system.
- Administrators can set managed settings for many desktops, for example the server address, the update check, and the AI policy. Windows uses Group Policy, macOS uses configuration profiles, and Linux uses a system settings file.
- The installers support silent installation.

## Data retention

Each organization can set retention rules for chat messages, logs, old revisions, and deleted objects. A retention rule keeps the data for a period, then deletes it.

## Audit log export

The server can send the audit log to security systems as syslog or as JSON lines.

## IP allow lists

An organization can let its members sign in only from given IP address ranges.

## Logs

Refer to [Logs and diagnostics](logs.md).
