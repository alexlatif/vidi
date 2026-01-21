# Execution Rules

## Source of Truth
- Standard verification: `just verify` (always runs)
- Issue-specific verification: defined in the plan's "Issue-Specific Verification" section

## Implementation Requirements
1. FIRST: Create/update the test or example that proves the issue is resolved
2. THEN: Implement the actual fix/feature
3. FINALLY: Run both `just verify` AND the issue-specific verification

## PR Updates (not new PRs)
- Check if branch `agent/issue-<N>` exists
- If yes: checkout, pull, add commits, force push if needed
- If no: create the branch
- Check if PR exists for this branch
- If yes: update it (push triggers update)
- If no: create new PR

## Definition of Done
- `just verify` passes
- Issue-specific verification passes (test/example/command from plan)
- PR description includes verification output
