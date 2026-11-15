# Quality

This chapter tells how Presswork keeps its code correct and healthy.

## Test-driven development

All work is test-driven:

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
| End-to-end tests | Two clients and one server in one process | All of the above |

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
- The workspace turns on the Clippy `pedantic` group.
- The workspace lints forbid `unsafe` code.
- Comments occur only where they are necessary. Names and types tell what the code does.
- Each crate and each module has one purpose. Functions stay short.
- All crates get their dependencies from the workspace dependency table. Thus all crates use the same versions.
- `cargo deny` examines licenses and security advisories.
