## Summary

<!-- What changed? Keep this focused on one cohesive change. -->

## Why

<!-- What problem does this solve, and why is this approach appropriate? -->

## Verification

<!-- List exact commands and manual checks. Do not write "tests pass" without naming them. -->

- [ ] `cargo fmt -- --check`
- [ ] `cargo clippy --all-targets --all-features --locked -- -D warnings`
- [ ] `cargo test --all-targets --locked`
- [ ] `cargo +1.86.0 check --all-targets --locked`

## Compatibility and user impact

<!-- Describe user-visible behavior, breaking changes, migrations, storage changes, and platform-specific effects. Write "None" when not applicable. -->

## Checklist

- [ ] I added or updated tests for behavior changes, or explained why no test is needed.
- [ ] I updated documentation for user-visible changes.
- [ ] I removed secrets, environment-variable values, and private package data from this PR.
- [ ] I linked the issue this PR resolves, when one exists.
