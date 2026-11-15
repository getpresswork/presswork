# Delivery

This chapter tells how a commit becomes a release.

## Platforms

| Program | Platform | Architectures | Packages |
| --- | --- | --- | --- |
| Desktop | Windows 10 and 11 | x86_64, aarch64 | MSI installer, ZIP archive |
| Desktop | macOS 13 or later | aarch64, x86_64 | DMG with an app bundle, archive |
| Desktop | Linux | x86_64, aarch64 | AppImage, DEB, RPM, archive |
| Server | Linux | x86_64, aarch64 | Container image, archive |

The first releases contain only the archives. The installers and the container image come before release 0.1.0.

## Continuous integration

Each push to `main` and each pull request runs these jobs:

| Job | Platforms | Command |
| --- | --- | --- |
| Format | Linux | `cargo fmt --all --check` |
| Lint and test | Windows, Linux, macOS | `cargo clippy` with all warnings as errors, then `cargo test --workspace` |
| Dependencies | Linux | `cargo deny check` |
| Documentation | Linux | `mdbook build docs` |

## Releases

1. A maintainer sets the new version in the root `Cargo.toml`.
2. The maintainer pushes a tag with the version, for example `v0.1.0`.
3. The release workflow builds the packages for all platforms.
4. git-cliff writes the release notes from the commit messages since the last tag.
5. The workflow makes a GitHub release with the packages, a SHA-256 checksum file, and the notes.

A maintainer can also start the release workflow by hand. Then it builds all packages, but it does not make a release.

## Versions

Presswork uses Semantic Versioning. All crates have the same version. Before version 1.0, a minor version can contain changes that are not compatible.

## Code signing

The first releases are not signed. When the project has certificates, the release workflow will sign the Windows installer. It will also sign and notarize the macOS app.

## Documentation site

After each push to `main`, the Pages workflow builds this book with mdBook. Then it publishes the book to GitHub Pages.
