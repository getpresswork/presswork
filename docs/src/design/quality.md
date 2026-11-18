# Quality

This chapter tells how Presswork keeps its code correct and healthy, and how it gets the quality of a commercial product.

## Test-driven development

All changes are test-driven:

1. Write a test that fails.
2. Write the simplest code that makes the test pass.
3. Improve the code. Make sure that all tests pass.

A change in behavior must come with tests for that behavior.

## Test kinds

| Kind | Location | Tools |
| --- | --- | --- |
| Unit tests | Next to the code, in `#[cfg(test)]` modules | Rust test harness |
| Integration tests | The `tests` folder of each crate | Rust test harness |
| Property tests | Text model, position maps, CRDT merges | `proptest` |
| Snapshot tests | Diagnostics, API data, rendered widgets | `insta` |
| Interface tests | Application flows without a window | `iced_test` |
| Database tests | Server repositories and API handlers | PostgreSQL, with one database for each test |
| OCR tests | The OCR engines in `presswork-ocr` | OCRmyPDF with Tesseract, and ocrs with its models |
| Package checks | The installers, in the build workflow | `msiexec`, `hdiutil`, `dpkg`, `rpm`, and the AppImage runtime |
| End-to-end tests | Two clients and one server in one process | All of the above |

## OCR tests

The OCR tests run the real engines on an image of a page (D155). They run only when these variables are set. Without a variable, its tests stop at once and write a note.

| Variable | Value |
| --- | --- |
| `PRESSWORK_TEST_OCRMYPDF` | The OCRmyPDF program, for example `ocrmypdf`. `tesseract` must be on the `PATH`, with the English data. |
| `PRESSWORK_TEST_OCRS_MODELS` | A folder with the ocrs models `text-detection.onnx` and `text-recognition.onnx`. The test checks their SHA-256 digests. |

In CI, the "OCR tests" job installs the tools, downloads the models, and sets both variables.

## Package checks

The build workflow checks each installer on the runner that made it (D161). The check installs the MSI file and the DEB package, examines the installed files and the version, and removes the package again. It opens the DMG file and examines the bundle identifier, the version, the architecture, and the signature of the app. It lists the files of the RPM package and unpacks the AppImage file. The nightly builds run these checks each night.

## Interface tests

The desktop application uses the Elm architecture of iced. It has a state, messages, an `update` function, and a `view` function.

- Tests call `update` with messages and examine the new state. These tests are fast and do not open a window.
- Tests use the `iced_test` simulator to click, type, and find text in a view.
- Widget tests compare rendered images with approved snapshots.

## Deterministic tests

- Typst tests use only the fonts of `typst-assets`, not system fonts. Thus the output is the same on all platforms.
- Tests inject the clock, the random source, and the network. No test uses the real network.

## Code health

- `cargo fmt` and `cargo clippy` run before each commit, through the Git hooks, and in CI. Clippy warnings are errors.
- The Cargo workspace turns on the Clippy `pedantic` group.
- The lints of the Cargo workspace forbid `unsafe` code.
- Comments occur only where they are necessary. Names and types tell what the code does.
- Each crate and each module has one purpose. Functions stay short.
- All crates get their dependencies from the dependency table of the Cargo workspace. Thus all crates use the same versions.
- `cargo deny` examines licenses and security advisories.

## Product quality

Presswork must have the completeness and the design quality of a commercial product. These rules make this goal measurable.

### Performance limits

The reference computer has four processor cores and 16 GB of memory.

| Operation | Limit |
| --- | --- |
| Start to an open window | 1 second |
| Start to an open 300-page book | 2 seconds |
| Keystroke to the character on the screen | 16 ms |
| Edit to the new preview of a 300-page book | 150 ms |
| Search in 10,000 library pages | 100 ms |
| Memory for a 300-page book with its preview | 1 GB |

Benchmarks in CI measure these operations. If a change makes an operation slower than its limit, the benchmark job fails.

### Complete views

Each view must have:

- An empty state that tells the user what to do first.
- A load state that does not stop other actions in the window.
- An error state with an action that corrects the problem.
- An offline state.
- A state for persons without the necessary permission.
- A path for the keyboard only.
- Correct layouts in the light, dark, and high-contrast themes, with snapshot tests.
- Interface text in STE, in Fluent files.
- Undo for each change.

### Data safety

- Presswork keeps each change locally at once. A crash or a power failure does not lose an edit.
- Each file has a local history, also without a server.
- After a crash, Presswork opens the same windows, files, and cursor positions again.

### Crash reports

Presswork does not collect usage data. After a crash, it writes a crash report to its data folder. At the next start, it offers to open a GitHub issue with the report. The user examines the report before Presswork sends it.

### Design reviews

- Each new view gets a design review before its release. The review uses the design tokens and the [Interface](interface.md) chapter.
- Each release has a release checklist: snapshots, performance, accessibility, and STE text.

The `render_screens` test makes images of the main screens in the light theme and in the dark theme. Use these images for design reviews. The test does not run with the other tests. To make the images, run this command:

```sh
cargo test --package presswork render_screens -- --ignored
```

The test writes the images to `target/screens`. To use a different folder, set the `PRESSWORK_SCREENS` environment variable.
