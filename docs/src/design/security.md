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

## Two-factor sign-in

- A user can add a second factor: TOTP codes from an authenticator application, or security keys and passkeys through WebAuthn.
- The user gets recovery codes when they add the second factor.
- An administrator can make a second factor necessary for all members of an organization.

## Encryption at rest

- The server encrypts the blobs, the document updates, and the chat messages before it stores them.
- The operator controls the keys. The server reads the keys from a file or from a key management service.

## Encrypted projects

A project can be end-to-end encrypted. Then only its members can read its content. The server keeps only encrypted data.

- Each encrypted project has a project key. Presswork encrypts the Loro updates, the files, the chat messages, and the names in the project with XChaCha20-Poly1305.
- Each device of a member has a device key pair (X25519). Presswork encrypts the project key for each device of each member.
- To add a member or a device, the desktop of a current member encrypts the project key for the new device.
- If a member leaves, Presswork makes a new project key. New content uses the new key.
- The server sees only metadata: the members, the sizes, and the times of changes.

In an encrypted project, the server cannot do these things:

- Compile, render, or export documents.
- Run OCR or search.
- Run routines.
- Show the project in the web portal.
- Put content into summary emails.

The desktop does these jobs. A member can select one desktop to run the routines of the project when it is online.

Before release 1.0, an external security audit examines the encryption design and its code.

## Downloads

- The toolchain manager downloads only through HTTPS.
- The toolchain manager compares the SHA-256 digest of each file with the published digest before it unpacks the file.
- The toolchain manager does not unpack paths outside of the target folder.

## Updates

- The release workflow signs the checksum file of each release with an Ed25519 key. The private key is a secret of the CI system.
- The updater of the desktop application contains the public key. It checks the signature and the checksum before it installs an update.
- The update check sends no data except the request for the release list.

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
