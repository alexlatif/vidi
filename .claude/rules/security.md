# Security Rules

## Secrets
- Never read or print: .env, credentials, API keys, private keys
- If referencing secret-handling code, never output values

## Dangerous Operations
- No `rm -rf` without explicit approval
- No network calls in automated runs
- No SSH or remote execution

## Code Review Flags
- Any new `unsafe` blocks
- Any new network/filesystem operations
- Any new dependencies with native code
