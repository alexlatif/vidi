# Verification Report Schema

## Report Structure

### Standard Verification
```
Command: just verify
Result: PASS/FAIL
Duration: Xs
```

### Issue-Specific Verification
For each item in the plan's "Issue-Specific Verification":
```
[x] Test: test_parse_empty_input - PASS
[x] Example: cli_demo --new-flag - PASS (output: "flag enabled")
[ ] Command: just bin myapp --version - FAIL (got "0.1.0", expected "0.2.0")
```

### Summary
- What changed (2-3 sentences)
- What was verified
- Any caveats or follow-ups

### Merge Recommendation
```
VERIFIER: PASS
```
or
```
VERIFIER: FAIL
Reason: <specific failure>
Next: <what needs to happen>
```
