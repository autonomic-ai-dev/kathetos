# Feature: rule admission

## Sub-features

- **blocking-syntax-rules**: a bare `#[ignore]`, a bare `t.Skip()`, `assert!(true)`, and an emptied test body block the diff (exit 1). Each is reported as `provisional`.
- **removal-is-fork**: a removed test with no test added in the same file is a fork, never a block.
- **reasoned-skips-are-forks**: an ignore or skip with a stated reason is a fork.
- **stub-is-not-emptied**: a new empty test is not a block, because nothing was emptied.
- **basis-in-output**: every violation carries its basis in the human and JSON output.

## How to get to it (user POV)

- CLI: `kathetos check --diff <range>` on a git repository. Exit 0 when nothing blocks, 1 when a violation fires.
- CLI: `kathetos check --diff <range> --json` prints the same findings as JSON, with `"basis"` on each violation.

## Verification Suites (Targeted Test Commands)

Unit: `cargo test`. Functional: `sh .verify/functional/cli.sh target/release/kathetos <out>`. Allowlisted: `.verify/run.sh`.

1. **bare-skip**
   - Request: `check --diff HEAD~1...HEAD` on a Go test that replaces a body with `t.Skip()`.
   - Why: a bare skip is the blocking skip shape.
   - Expected: exit 1, `violation: t.Skip with no reason` with `[provisional]`.
   - Impact: none; read-only check.

2. **removed-test**
   - Request: `check --diff HEAD~1...HEAD` on a Rust file that drops one of two tests.
   - Why: removal is a question for a person, so it must not block.
   - Expected: exit 0, no violation, fork `test removed, no test added in this file`.
   - Impact: none.

3. **ignore-with-reason**
   - Request: `check --diff HEAD~1...HEAD` on a Rust test that gains `#[ignore = "needs docker"]`.
   - Why: a reasoned ignore is a decision, so it is a fork.
   - Expected: exit 0, fork `test ignored (reason given)`, no violation.
   - Impact: none.

4. **bare-ignore-on-existing-test**
   - Request: `check --diff HEAD~1...HEAD` on a Rust test that gains a bare `#[ignore]` while its `#[test]` line is untouched.
   - Why: the block must not depend on the test marker being in the diff.
   - Expected: exit 1, `violation: #[ignore] with no reason`.
   - Impact: none.

5. **emptied-body**
   - Request: `check --diff HEAD~1...HEAD` on a Go test whose body is deleted.
   - Why: a test that had a body and now has none is emptied.
   - Expected: exit 1, `violation: test body emptied`.
   - Impact: none.

6. **stub-test**
   - Request: `check --diff HEAD~1...HEAD` on a Go file that adds an empty test.
   - Why: a new empty test was never emptied, so it must not block.
   - Expected: exit 0, no violation.
   - Impact: none.

7. **clean-change**
   - Request: `check --diff HEAD~1...HEAD` on a non-test source change.
   - Why: unrelated edits must stay silent.
   - Expected: exit 0, no violation.
   - Impact: none.

8. **json-basis**
   - Request: `check --diff HEAD~1...HEAD --json` on the bare-skip change.
   - Why: JSON consumers need the basis per violation.
   - Expected: exit 1, JSON contains `"basis":"provisional"`.
   - Impact: none.

9. **empty-range**
   - Request: `check --diff HEAD...HEAD` in a repository with a change in its history.
   - Why: an empty diff must not pass silently, per RECEIPT-SPEC.md Rule 6.
   - Expected: exit 2, `empty diff, nothing to check`.
   - Impact: none.

10. **unknown-revision**
   - Request: `check --diff nope...HEAD`.
   - Why: an unknown ref must be named, and must not get the shallow-clone hint.
   - Expected: exit 2, `unknown revision `nope``, and no `deepen` hint.
   - Impact: none.

## Gotchas

- Every blocking rule is `provisional` until it meets the bar in `docs/rule-admission.md`. The status is printed with each violation, so a reader can see it.
- Rust tests defined inline in a source file that the diff does not otherwise mark as test code are not loaded for body comparison. Only test files (`_test.go`, `_test.rs`, `tests/`) or diffs that touch a test marker are compared.
- Hurl is not used. The product is a CLI with no HTTP surface, so the functional phase runs the binary against scratch git repositories.
