# Security

This chapter tells how Presswork protects documents, accounts, and computers.

## Typst sandbox

- A Typst document can read only the files in its project root and the installed packages.
- A Typst document cannot use the network. Only the toolchain manager downloads packages, from Typst Universe or from the team server.

## Server

- The server keeps password hashes with Argon2id, and token hashes with SHA-256.
- The server limits the failed sign-ins for each email address and for each client address (D148). Refer to [Server](server.md).
- Each request handler checks the permission before it does the operation. The default is no access.
- Put the server behind a reverse proxy that uses TLS. Do not let clients connect to the server with plain HTTP.
- OCR runs OCRmyPDF, Tesseract, and Ghostscript on the files that members upload (D152). These programs are not in Rust, and a hostile PDF file can attack them. Run the server in a container, or with a user that can write only to the `data` folder. The container image and the systemd unit of the server archive do this (D166). Each OCR run has a time limit of one hour.
- The audit log records all changes.

## Two-factor sign-in

- A user can add a second factor: TOTP codes from an authenticator application, or security keys and passkeys through WebAuthn.
- The user gets recovery codes when they add the second factor.
- An administrator can make a second factor necessary for all members of an organization.

## Sessions and devices

- Each user sees their sessions and their devices, with the time and the place of the last use.
- A user can sign out any session or device.
- When a user removes a device, Presswork also removes the device key from all encrypted projects. Refer to the encrypted projects below.
- An organization can limit sign-in to given IP address ranges. Refer to [Operations](operations.md).

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
- The two model files of the ocrs engine have fixed SHA-256 digests in the `presswork-ocr` crate. The engine does not load a model file with a different digest (D154).
- The release workflow downloads WiX and the AppImage tools at fixed versions. `package-tools.sh` compares each file with a fixed digest, and a different digest stops the build (D158).
- The server image uses Ubuntu 24.04 at a fixed digest. OCRmyPDF and Tesseract come from the Ubuntu archive, and apt checks the signatures of the archive. The Compose file uses PostgreSQL 17 at a fixed digest. Dependabot opens update requests for new digests of the two images (D165 and D168).

## Updates

- The release workflow signs the checksum file of each release with an Ed25519 key. The private key is a secret of the CI system.
- The updater of the desktop application contains the public key. It checks the digital signature and the checksum before it installs an update.
- The update check sends no data except the request for the release list.

## Credentials on the desktop

The desktop application keeps tokens in the credential store of the operating system:

- Windows Credential Manager on Windows.
- Keychain on macOS.
- Secret Service on Linux.

## Supply chain

- In CI, `cargo deny` examines the licenses, the security advisories, and the sources of all dependencies. It examines the dependency graph of the six release targets, because only these dependencies go into a release.
- A vulnerability in any dependency stops the build. An unmaintained crate stops the build only if a Presswork crate uses it directly. For the other unmaintained crates, the upstream projects must replace them.
- An exception for a vulnerability is permitted only if no fixed release exists. Each exception in `deny.toml` tells the cause and the condition that removes it.
- CI workflows pin each third-party action to a commit hash.
- Dependabot opens update requests each week.

### Current exceptions

| Advisory | Cause | Risk | Removal |
| --- | --- | --- | --- |
| RUSTSEC-2026-0194 and RUSTSEC-2026-0195 | Typst 0.15.1 uses `quick-xml` 0.38 through `hayagriva` and `citationberg`. These crates have no fixed release. | A hostile CSL style file can make a compilation slow or use much memory. On the desktop, only the document with the file is affected. | Remove the exceptions when Typst uses `quick-xml` 0.41 or later. |

The project owner reviewed these exceptions on 2026-11-15. The other option was a fork of `quick-xml` 0.38 with the fix.

## Vulnerability reports

Refer to the `SECURITY.md` file in the repository root.
