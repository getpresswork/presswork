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

## Release train

The release train makes a release every two weeks. A commit does not bump the version.

1. On the Monday of each even week, the release train workflow starts.
2. It looks for `feat`, `fix`, and `perf` commits since the last release. If there are none, it stops.
3. git-cliff calculates the next version from the commit types.
4. The workflow sets the version in the root `Cargo.toml`, updates `Cargo.lock`, and writes `CHANGELOG.md`.
5. It commits these files to `main` with the message `chore(release): <version>`. Then it makes a tag through the GitHub API, for example `v0.2.0`.
6. It starts the release workflow for the tag.
7. The release workflow builds the packages for all platforms. It makes a GitHub release with the packages, a SHA-256 checksum file, and the release notes.

A maintainer can start the release train by hand, for example for an urgent correction. A dry run calculates the next version but changes nothing.

## Nightly builds

Each night, the nightly workflow looks for new commits on `main`. If there are new commits, it builds the packages for all platforms. It puts them into the `nightly` prerelease on GitHub, in place of the previous nightly packages. Testers use the nightly builds. The updater does not install them unless the user selects the nightly builds.

## Versions

Presswork uses Semantic Versioning. All crates have the same version.

| Commits since the last release | Version change before 1.0 | Version change after 1.0 |
| --- | --- | --- |
| A change that is not compatible | Minor | Major |
| `feat` | Minor | Minor |
| `fix` or `perf` | Patch | Patch |

## Updates

The desktop application has a signed updater:

1. At start, the updater asks GitHub for the release list. It sends no other data.
2. If a newer release exists, the application shows the release notes.
3. The updater downloads the package for the platform. It checks the Ed25519 signature and the SHA-256 checksum.
4. The update installs when the application starts the next time.

Users can turn off the update check. Installations from package managers do not use the updater. Refer to [Security](security.md) for the key of the signatures.

## Signatures

The first releases are not signed. When the project has certificates, the release workflow will sign the Windows installer. It will also sign and notarize the macOS app.

## Documentation site

After each push to `main`, the Pages workflow builds this book with mdBook. Then it publishes the book to GitHub Pages.
