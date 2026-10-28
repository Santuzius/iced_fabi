# Testing

- `cargo test` runs the crate's unit tests (`src/**/tests`, one directory per browser/cabinet topic) and the public API's doctests.
- The `cloud_browse` example carries its own `#[cfg(test)]` tests for the fake-cloud helpers: `cargo test --example cloud_browse` runs them; plain `cargo test` only compiles examples, it does not run their tests. Keep the two commands together as "the tests" of a change.
- Rendered behaviour neither command can reach (tool bar layout, the mode switch, dialogs) is verified with the `agent-screenshot` click recipe in `AGENTS.md`.
