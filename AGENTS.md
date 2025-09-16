# Rust/gamacros

The application code is divided into modules located in the `./crates` folder:

- Crate names are prefixed with `gamacros-`. For example, the `workspace` folder's crate is named `gamacros-workspace`. The only exception is the daemon whose crate is called `gamacrosd`.
- When using format! and you can inline variables into {}, always do that.

Run `just fmt` (in the project directory) automatically after making Rust code changes; do not ask for approval to run it. Before finalizing a change to `gamacros`, run `just fix` to fix any linter issues in the code. Prefer scoping with `-p` to avoid slow workspace‑wide Clippy builds; only run `just fix` without `-p` if you changed shared crates. Additionally, run the tests:
1. Run the test for the specific project that was changed. For example, if changes were made in `crates/gamacros-gamepad`, run `just test -p gamacros-gamepad`.
2. Once those pass, if any changes were made in common, core, or protocol, run the complete test suite with `just test --all-features`.
Don't ask the user before running `just fix` to finalize. `just fmt` does not require approval. project-specific or individual tests can be run without asking the user, but do ask the user before running the complete test suite.

To build the project, run `just build`.
