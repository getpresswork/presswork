# Code style

This chapter gives the rules for Rust code in Presswork.

## General rules

- Format all code with `cargo fmt`. Do not change the formatter settings without a decision in the [decision log](../design/decisions.md).
- Correct all Clippy warnings. The Cargo workspace turns on the Clippy `pedantic` group.
- Write comments only where the code cannot tell the reason. Do not repeat the code in a comment.
- Do not add license headers to source files. The `LICENSE` and `NOTICE` files apply to the full repository.
- Use clear names. A long clear name is better than a short unclear name.

## Structure

- Keep pure logic separate from input and output. Pure logic goes into the domain crates.
- Put all input and output behind traits. Tests use fakes for these traits.
- Keep modules small, with one concept in each module.
- Make items private or `pub(crate)` when possible. The `unreachable_pub` lint finds items that are too public.
- Add a crate only when its first code arrives. Split a crate when it has more than one purpose.

## Errors

- Library crates return typed errors. Use `thiserror` to make them.
- Do not use `unwrap` or `expect` outside of tests. If a value is always present, show this with a type.
- To allow a lint in one location, use `#[expect(lint, reason = "...")]`. Write the reason in the attribute, not in a comment.
- Do not use `unsafe` code. The lints of the Cargo workspace forbid it.

## Tests

- Give each test a name that tells the behavior, for example `inserts_text_at_cursor`.
- Test one behavior in each test.
- Use three parts in each test: arrange, act, assert.
- Make each test deterministic. Do not use the real clock, the real network, or system fonts.

## Dependencies

- Add each dependency to `[workspace.dependencies]` in the root `Cargo.toml`. Crates refer to it with `workspace = true`.
- Turn off default features that a crate does not use.
- Use crates that have active maintainers and permissive licenses. `cargo deny` examines the licenses in CI.
