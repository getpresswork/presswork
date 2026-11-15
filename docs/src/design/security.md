# Security

This chapter tells how Presswork protects documents, accounts, and computers.

## Typst sandbox

- A Typst document can read only the files in its project root and the installed packages.
- A Typst document cannot use the network. Only the toolchain manager downloads packages, from Typst Universe or from the team server.

## Server

- The server keeps password hashes with Argon2id, and token hashes with SHA-256.
- The server limits how many times a user can try to sign in during a period.
- Each request handler checks the permission before it does the operation. The default is no access.
- Put the server behind a reverse proxy that uses TLS. Do not let clients connect to the server with plain HTTP.
- The audit log records all changes.

## Downloads

- The toolchain manager downloads only through HTTPS.
- The toolchain manager compares the SHA-256 digest of each file with the published digest before it unpacks the file.
- The toolchain manager does not unpack paths outside of the target folder.

## Credentials on the desktop

The desktop application keeps tokens in the credential store of the operating system:

- Windows Credential Manager on Windows.
- Keychain on macOS.
- Secret Service on Linux.

## Supply chain

- In CI, `cargo deny` examines the licenses, the security advisories, and the sources of all dependencies.
- A vulnerability in any dependency stops the build. An unmaintained crate stops the build only if a Presswork crate uses it directly. For the other unmaintained crates, the upstream projects must replace them.
- CI workflows pin each third-party action to a commit hash.
- Dependabot opens update requests each week.

## Vulnerability reports

Refer to the `SECURITY.md` file in the repository root.
