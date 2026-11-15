# Workflow

This chapter tells how to work on Presswork.

## Prepare your computer

1. Install Rust with [rustup](https://rustup.rs). The `rust-toolchain.toml` file selects the correct Rust version.
2. Install Git.
3. Clone the repository.
4. Turn on the Git hooks of the repository:

   ```sh
   git config core.hooksPath .githooks
   ```

The hooks run before each commit:

- `pre-commit` runs `cargo fmt --all --check` and `cargo clippy` with all warnings as errors.
- `commit-msg` examines the format of the commit message.

## Daily procedure

1. Write a test that fails.
2. Write the simplest code that makes the test pass.
3. Improve the code. Make sure that all tests pass.
4. Run all tests:

   ```sh
   cargo test --workspace
   ```

5. Commit the change. The hooks examine the format and the lints.
6. Push the commit.

## Commits

- Write the message on one line, in the [Conventional Commits](https://www.conventionalcommits.org) form: `type(scope): summary`.
- Use one of these types: `build`, `chore`, `ci`, `docs`, `feat`, `fix`, `perf`, `refactor`, `revert`, `style`, `test`.
- For the scope, use the crate name without `presswork-`, for example `editor` or `typst`. For documentation, use the area, for example `design`.
- Write the summary as a command, in lowercase, without a period.
- Use 72 characters or fewer for the full line.
- Make small commits. Each commit must build, and all tests must pass.

Examples:

```text
feat(text): add line index to buffer
fix(toolchain): check digest before unpack
docs(design): add collaboration model
ci: build release archives on tag push
```

The release notes come from these messages. Thus a clear summary gives clear release notes.

## Pull requests

- Keep each pull request small and about one subject.
- CI must pass on Windows, Linux, and macOS.
- If you change a design decision, update the design chapters and the [decision log](../design/decisions.md).
