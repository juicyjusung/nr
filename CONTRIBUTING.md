# Contributing to nr

Thank you for your interest in contributing to `nr`! This document provides guidelines and instructions for contributing.

## Getting Started

### Prerequisites

- Rust 1.85 or later
- Git
- A package manager (npm, yarn, pnpm, or bun) for testing

### Development Setup

1. Fork and clone the repository:
   ```bash
   git clone https://github.com/YOUR_USERNAME/nr.git
   cd nr
   ```

2. Build the project:
   ```bash
   cargo build
   ```

3. Run tests:
   ```bash
   cargo test
   ```

4. Run the binary:
   ```bash
   cargo run
   ```

## Development Workflow

### Code Style

We follow standard Rust conventions:

- **Format**: Run `cargo fmt` before committing
- **Lint**: Ensure `cargo clippy` passes with no warnings
- **Tests**: All tests must pass via `cargo test`

```bash
# Check everything before committing
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

### Project Structure

```
src/
├── main.rs           # CLI entry point, lifecycle management
├── app.rs            # Main application state machine
├── fuzzy.rs          # Fuzzy search implementation
├── sort.rs           # Script sorting with frecency algorithm
├── core/             # Business logic (stateless)
│   ├── package_manager.rs
│   ├── project_root.rs
│   ├── scripts.rs
│   ├── workspaces.rs
│   ├── runner.rs
│   └── package_json.rs
├── store/            # Persistence layer
│   ├── favorites.rs
│   ├── recents.rs
│   ├── project_id.rs
│   └── config_path.rs
└── ui/               # Pure rendering functions
    ├── script_list.rs
    ├── package_list.rs
    ├── search_input.rs
    ├── status_bar.rs
    ├── tabs.rs
    └── header_bar.rs
```

### Architecture Principles

- **Pure UI functions**: All `ui/` modules should be stateless render functions
- **Stateless core**: `core/` modules should be pure functions with no shared state
- **Index-based filtering**: Use `Vec<usize>` indices to avoid cloning data
- **Error handling**: Use `anyhow::Result` for application-level errors, `thiserror` for domain errors
- **No unwrap()**: Use `?` operator or explicit error handling in production code

## Writing Tests

### Test Guidelines

1. **Location**: Tests live in `#[cfg(test)] mod tests` within the same file as the code
2. **Coverage**: Add tests for new features and bug fixes
3. **Edge cases**: Test empty inputs, invalid data, and boundary conditions

### Example Test

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_name() {
        // Arrange
        let input = "test";
        
        // Act
        let result = my_function(input);
        
        // Assert
        assert_eq!(result, expected);
    }
}
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for a specific module
cargo test sort

# Run tests with output
cargo test -- --nocapture
```

## Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/) for clear commit history:

```
<type>(<scope>): <subject>

[optional body]

[optional footer]
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, no logic change)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks, dependency updates

### Examples

```bash
feat(ui): add keyboard shortcuts help dialog
fix(sort): correct frecency calculation for old entries
docs(readme): update installation instructions
test(fuzzy): add test for case-insensitive matching
```

## Pull Request Process

### Before Submitting

1. **Create a branch** from `main`:
   ```bash
   git checkout -b feat/my-feature
   ```

2. **Make your changes** following the code style guidelines

3. **Run the pre-submission checklist**:
   ```bash
   cargo fmt
   cargo clippy -- -D warnings
   cargo test
   cargo build --release
   ```

4. **Write clear commit messages** using Conventional Commits

5. **Push your branch**:
   ```bash
   git push origin feat/my-feature
   ```

### PR Guidelines

- **Title**: Use Conventional Commits format (e.g., `feat: add script history`)
- **Description**: Clearly explain:
  - What changes were made
  - Why these changes are needed
  - Any breaking changes or migration notes
- **Tests**: Include tests for new features
- **Documentation**: Update README.md or other docs if needed
- **One feature per PR**: Keep PRs focused and reviewable

### PR Checklist

- [ ] Code follows project style guidelines (`cargo fmt`, `cargo clippy`)
- [ ] All tests pass (`cargo test`)
- [ ] New tests added for new features
- [ ] Documentation updated if needed
- [ ] Commit messages follow Conventional Commits
- [ ] No breaking changes (or clearly documented)
- [ ] Binary size checked (should stay ~1 MB)

## Reporting Issues

### Bug Reports

When reporting bugs, please include:

1. **Description**: Clear description of the issue
2. **Reproduction steps**: Step-by-step instructions to reproduce
3. **Expected behavior**: What you expected to happen
4. **Actual behavior**: What actually happened
5. **Environment**:
   - OS and version (e.g., macOS 14.2, Ubuntu 22.04)
   - `nr` version (`nr --version`)
   - Package manager (npm, yarn, pnpm, bun) and version
6. **Logs**: Any error messages or debug output

### Feature Requests

For feature requests, please describe:

1. **Problem**: What problem does this solve?
2. **Proposed solution**: How should it work?
3. **Alternatives**: Other solutions you've considered
4. **Context**: Why is this important to you?

## Development Tips

### Testing with Example Project

```bash
# Build and run in example project
cargo build --release
cd examples/demo-project
../../target/release/nr
```

### Debugging TUI Issues

1. Use `eprintln!()` for debug output (goes to stderr, not captured by TUI)
2. Test panic recovery manually (trigger a panic to see if terminal restores)
3. Test on different terminals (iTerm2, Terminal.app, Windows Terminal, etc.)

### Performance Testing

For large projects with many scripts:

```bash
# Generate large package.json
node -e "
const scripts = {};
for (let i = 0; i < 500; i++) {
  scripts[\`script-\${i}\`] = 'echo test';
}
console.log(JSON.stringify({ scripts }, null, 2));
" > package.json

# Test nr performance
time cargo run --release
```

## Code Review Process

1. **Automatic checks**: CI runs tests, linting, and formatting checks
2. **Maintainer review**: A maintainer will review your code
3. **Feedback**: Address any requested changes
4. **Merge**: Once approved, maintainer will merge your PR

## Questions?

- **Discussions**: Use [GitHub Discussions](https://github.com/juicyjusung/nr/discussions) for general questions
- **Issues**: Use [GitHub Issues](https://github.com/juicyjusung/nr/issues) for bugs and feature requests
- **Chat**: (Add Discord/Slack link if available)

## License

By contributing to `nr`, you agree that your contributions will be licensed under the [MIT License](LICENSE).

---

Thank you for contributing! 🎉
