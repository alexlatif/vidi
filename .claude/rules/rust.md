# Rust Standards

## Formatting
- `cargo fmt --all` must pass
- No manual formatting overrides

## Linting
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- No `#[allow(clippy::...)]` without justification

## Testing
- New behavior requires a test that:
  1. Fails before the fix
  2. Passes after the fix
- Test names should describe the behavior, not the implementation

## Examples
- Prefer examples over doc comments for complex features
- Examples must compile and run without errors
- Use `//! Example description` at top of example files

## Dependencies
- Prefer existing crates
- New deps require justification in PR description
