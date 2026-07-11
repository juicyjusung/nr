# Contributing to nr

Thanks for helping improve `nr`. This guide describes the development workflow and the checks a pull request must pass.

## Before you start

- Search [open issues](https://github.com/juicyjusung/nr/issues) before starting overlapping work.
- For a substantial change, open a feature request first so its behavior and compatibility can be agreed on.
- Report security vulnerabilities privately as described in [SECURITY.md](SECURITY.md), not in a public issue.

You need Git, Rust 1.86 or newer, and at least one supported package manager (npm, pnpm, Yarn, or Bun) for end-to-end testing.

## Set up the project

1. Fork and clone the repository.

   ```bash
   git clone https://github.com/YOUR_USERNAME/nr.git
   cd nr
   ```

2. Build and test the project.

   ```bash
   cargo build --locked
   cargo test --all-targets --locked
   ```

3. Run `nr` from a fixture project.

   ```bash
   cd examples/demo-project
   ../../target/debug/nr
   ```

## Architecture

`nr` keeps state management, business logic, persistence, and rendering separate:

```text
src/
├── main.rs    CLI startup, TUI lifecycle, persistence, and script execution
├── app.rs     Application state machine and input handling
├── fuzzy.rs   Fuzzy-matching adapter
├── sort.rs    Favorites, frecency, and fuzzy-result ordering
├── core/      Project discovery, package data, environment files, and execution
├── store/     JSON-backed user preferences and execution history
└── ui/        Stateless Ratatui rendering functions
```

Keep these boundaries intact unless a change has a clear reason to move them:

- `App` owns mutable UI state; input handling returns an `Action` for lifecycle work.
- `core` contains stateless business logic and must not depend on the TUI.
- `ui` functions render data passed to them and do not retain application state.
- Filtered lists store indices into source collections instead of cloned entries.
- Persistence is scoped by project ID under the user's platform config directory.
- Use `anyhow::Result` at application boundaries and typed errors where callers need to distinguish failures.
- Do not use `unwrap()` on production paths; propagate errors or handle them explicitly.

## Make a change

Create a focused branch from `main` and keep each pull request to one cohesive change.

```bash
git checkout main
git pull --ff-only
git checkout -b fix/short-description
```

Add a regression or characterization test before changing behavior that users may rely on. Tests normally live in a `#[cfg(test)] mod tests` next to the implementation. Cover failure paths and boundary cases as well as the successful path.

For TUI or runner changes, also exercise the relevant flow manually with the package managers and operating systems affected by the change. Avoid putting secrets or real credentials in test fixtures, logs, screenshots, or issues.

## Run the checks

Run the same checks enforced by CI before opening a pull request:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --locked
cargo +1.86.0 check --all-targets --locked
```

Install the MSRV toolchain with `rustup toolchain install 1.86.0` if needed. Run `cargo build --release --locked` when changing packaging, dependencies, release settings, or code that may materially affect binary size.

Use [Conventional Commits](https://www.conventionalcommits.org/) for commit messages and pull request titles, for example:

```text
feat(ui): add keyboard shortcut help
fix(sort): preserve fuzzy relevance
docs(readme): clarify installation
```

## Open a pull request

In the pull request:

- Explain the problem and why the chosen change solves it.
- Describe user-visible or compatibility effects, including any migration needs.
- List the exact automated and manual verification performed.
- Update documentation when commands, behavior, storage, or support change.
- Link the issue the pull request resolves, when one exists.

CI runs formatting, Clippy, MSRV, and tests on Linux, macOS, and Windows. Address failures rather than suppressing a lint or weakening a test.

## Get support

Use the repository's [issue forms](https://github.com/juicyjusung/nr/issues/new/choose) for reproducible bugs and feature requests. Search existing issues first and include your `nr` version, operating system, package-manager version, and relevant project or workspace layout.

If you are unsure whether unexpected behavior is a bug, file a bug report with the information you have; maintainers can reclassify it. Do not include access tokens, environment-variable values, private package contents, or other sensitive data.

## License

By contributing to `nr`, you agree that your contributions will be licensed under the [MIT License](LICENSE).
