# Planning Rules

## When to Plan
- On issue opened/edited: produce initial plan
- On any comment (except bot::run/bot::merge): refine the plan based on feedback
- After bot::run completes: if user comments, return to planning to discuss changes

## Plan Structure (MANDATORY)

### Objective
One paragraph: what we're solving and why it matters.

### Context
- Current behavior (what exists today)
- Relevant files and line numbers
- Dependencies and constraints

### Theory
- Design approach and rationale
- Invariants that must hold
- Tradeoffs considered

### Issue-Specific Verification (CRITICAL)
This is NOT just "run tests". Define EXACTLY how we prove this issue is resolved:

```
Verification for Issue #N:
1. [ ] Test: <specific test name/description that validates the fix>
2. [ ] Example: <example that demonstrates the new behavior>
3. [ ] Command: <exact command to run and expected output>
```

Examples:
- "Test: `cargo test test_parse_empty_input` passes (currently fails)"
- "Example: `cargo run --example cli_demo -- --new-flag` outputs 'flag enabled'"
- "Command: `just bin myapp --version` shows '0.2.0' (was '0.1.0')"

### Risks
Top 3 risks or unknowns.

### Questions
Blockers only. Tag @author. If any exist, end with: **BLOCKED: awaiting answers**

### Plan
Numbered high-level steps.

### Todos
Checkbox list, each = one commit:
- [ ] Add failing test for the expected behavior
- [ ] Implement the fix
- [ ] Update example/docs if needed
- [ ] Verify all checks pass
