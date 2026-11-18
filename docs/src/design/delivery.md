# Delivery

This chapter tells how a commit becomes a release.

## Platforms

| Program | Platform | Architectures | Packages |
| --- | --- | --- | --- |
| Desktop | Windows 10 and 11 | x86_64, aarch64 | MSI installer, ZIP archive |
| Desktop | macOS 13 or later | aarch64, x86_64 | DMG with an app bundle, archive |
| Desktop | Linux | x86_64, aarch64 | AppImage, DEB, RPM, archive |
| Server | Linux | x86_64, aarch64 | Container image, archive |

The Linux packages come from Ubuntu 24.04 runners. Thus they need glibc 2.39 or later.

Each release contains the archives and the installers of the desktop application, and the archives of the server. The release workflow also publishes the container image of the server. Refer to [Deployment](server.md#deployment).

## Installers

The build workflow makes the installers with the standard tool of each platform (D157):

| Package | Tool | Contents |
| --- | --- | --- |
| MSI | WiX 6 | `presswork.exe`, `LICENSE.txt`, and `NOTICE.txt` in `Program Files\Presswork`, and a Start menu shortcut. The installation is for all users (D159). |
| DMG | `hdiutil` | `Presswork.app`, and a link to the Applications folder. |
| DEB and RPM | `dpkg-deb` and `rpmbuild` | `/usr/bin/presswork`, the desktop file, the icons in the `hicolor` theme, and the license files. |
| AppImage | `appimagetool` | The files of the DEB package in one file, which runs without an installation. |

The templates are in the `packaging` folder. The scripts are in `.github/scripts`:

- `package-tools.sh` downloads WiX or the AppImage tools and checks their digests (D158).
- `package-linux.sh`, `package-macos.sh`, and `package-windows.ps1` make the packages in the `dist` folder.
- `check-linux-packages.sh`, `check-macos-package.sh`, and `check-windows-package.ps1` install and remove the packages, and examine their files.

A maintainer can start the build workflow by hand. It then makes all packages without a release.

The application icon comes from `crates/presswork/icons/presswork.svg` (D160). After a change to the SVG file, write the PNG, ICO, and ICNS files again with the ignored test `render_icons`:

```sh
cargo test --workspace --lib -- --ignored render_icons
```

## Server packages

On the two Linux runners, the build workflow also builds the server (D162 and D163):

- `package-server.sh` makes the server archive. `check-server-archive.sh` examines its files and the version, and examines the systemd unit with `systemd-analyze verify`.
- On the x86_64 runner, `check-server-service.sh` installs the server archive with systemd, as the [server chapter](server.md#server-archive) tells. It uses the PostgreSQL of the runner. It examines the ready check, the user of the service, the data folder, the `create-organization` command, OCR, and the stop.
- `server-image.sh` builds the container image from `packaging/server/Dockerfile`. `check-server-image.sh` starts the image and PostgreSQL with the Compose file. It examines the version, the user, the OCR tools, the ready check, the `create-organization` command, and the stop with `SIGTERM`.
- For OCR, `check-server-library.sh` accepts the invite, uploads a scanned page, and finds its text with the library search. Ghostscript makes the scanned page from `crates/presswork-ocr/fixtures/typst.pdf`.
- In the release workflow and the nightly workflow, `push-server-image.sh` pushes the checked image of each architecture to GHCR. Then `publish-server-image.sh` joins the two images under the tags of the release. `server-image-tags.sh` gives these tags.

## Continuous integration

Each push to `main` and each pull request runs these jobs:

| Job | Platforms | Command |
| --- | --- | --- |
| Format | Linux | `cargo fmt --all --check` |
| Lint and test | Windows, Linux, macOS | `cargo clippy` with all warnings as errors, then `cargo test --workspace` |
| Database tests | Linux | `cargo test --package presswork-server` with PostgreSQL |
| OCR tests | Linux | `cargo test --package presswork-ocr` with OCRmyPDF and the ocrs models |
| Dependencies | Linux | `cargo deny check` |
| Scripts | Linux | The tests of the scripts in `.github/scripts` |
| Documentation | Linux | `mdbook build docs` |

## Release train

The release train makes a release every two weeks. A commit does not bump the version.

1. On the Monday of each even week, the release train workflow starts.
2. It looks for `feat`, `fix`, and `perf` commits since the last release. If there are none, it stops.
3. The workflow calculates the next version. Refer to [Versions](#versions).
4. The workflow sets the version in the root `Cargo.toml`, updates `Cargo.lock`, and writes `CHANGELOG.md`.
5. It commits these files to `main` with the message `chore(release): <version>`. Then it makes a tag through the GitHub API, for example `v0.2.0`.
6. It starts the release workflow for the tag.
7. The release workflow builds the packages for all platforms. It makes a GitHub release with the packages, a SHA-256 checksum file, and the release notes.

A maintainer can start the release train by hand, for example for an urgent correction. A dry run calculates the next version but changes nothing. When a milestone is complete, a maintainer starts the release train by hand with the next minor version, for example 0.2.0 after milestone 1.

## Nightly builds

Each night, the nightly workflow looks for new commits on `main`. If there are new commits, it builds the packages for all platforms. It puts them into the `nightly` prerelease on GitHub, in place of the previous nightly packages. It also publishes the server image with the tag `nightly`. Testers use the nightly builds. The updater does not install them unless the user selects the nightly builds.

## Versions

Presswork uses Semantic Versioning. All crates have the same version.

Before 1.0, the minor version shows the milestone that is in progress. Refer to the [Roadmap](roadmap.md).

- Each release of the release train bumps only the patch version, for example from 0.1.3 to 0.1.4.
- When a milestone is complete, a maintainer releases the next minor version by hand.

After 1.0, git-cliff calculates the version from the commit types:

| Commits since the last release | Version change |
| --- | --- |
| A change that is not compatible | Major |
| `feat` | Minor |
| `fix` or `perf` | Patch |

The `.github/scripts/next-version.sh` script applies these rules. CI runs its tests.

## Updates

The desktop application has an updater that checks digital signatures:

1. At start, the updater asks GitHub for the release list. It sends no other data.
2. If a newer release exists, the application shows the release notes.
3. The updater downloads the package for the platform. It checks the Ed25519 digital signature and the SHA-256 checksum.
4. The update installs when the application starts the next time.

Users can turn off the update check. Installations from package managers do not use the updater. Refer to [Security](security.md) for the key of the digital signatures.

## Digital signatures of packages

The first releases have no digital signatures. When the project has certificates, the release workflow will sign the Windows installer. It will also sign and notarize the macOS app.

Until then, Windows SmartScreen shows a warning before the installation. The macOS app has only an ad hoc signature, so macOS does not open it until the user approves it in System Settings, in Privacy & Security.

## Documentation site

After each push to `main`, the Pages workflow builds this book with mdBook. Then it publishes the book to GitHub Pages.
