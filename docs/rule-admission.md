# Rule admission

A rule may block (exit 1) only when it meets this bar. Each rule states what it
claims and what text it reads, so a reviewer can see what the claim does and does
not cover.

## Bar

Set before any measurement, so it cannot move after the numbers come in.

- **Precision:** at least 0.90 on at least 20 labeled findings, where a finding
  counts as correct only if the rule's claim holds for that finding.
- **Recall:** every seeded fault of the rule's shape is caught (see
  `.verify/functional/cli.sh` and the unit tests named `seeded_*`).
- **Status:** `provisional` until both hold. The output prints the status with
  each violation.

## Rules

| Rule | Claim | Reads | Cannot know | Blocks | Status |
|---|---|---|---|---|---|
| `#[ignore]` with no reason | A bare ignore attribute was added and no reason is documented nearby | Added attribute line, trailing comment, comment block above, file text | Whether the documented reason is good; any comment counts | yes | provisional |
| `t.Skip()` with no reason | A bare `t.Skip()` was added and no reason is documented nearby | Added line, trailing comment, comment block above, enclosing Go doc comment | Whether the documented reason is good; any comment counts; guard conditions do not count | yes | provisional |
| constant-true assertion | `assert!(true)` was added | Added line | Whether the assertion is the only one | yes | provisional |
| test body emptied | A test with a body in the base now has none after comments are stripped | Head and base file, same-named test | Whether a helper moved the body elsewhere | yes | provisional |
| test removed, no test added in this file | A test marker was removed and the file gained none | Removed and added lines | Whether the behaviour is covered elsewhere | no (fork) | fork |
| live/integration test removed | A removed test touches the outside world | Removed body keywords | Whether it was deleted because it fails | no (fork) | fork |

## Labeled findings

Labels come from the 400-commit sample run against the binary before the two
rule fixes. Each finding was read in its diff. `claim` is whether the rule's
stated claim holds for that finding.

| Repo | Commit | Rule | Location | Claim | Note |
|---|---|---|---|---|---|
| cobra | 40b5bc1437 | test removed | command_test.go:2929 | holds | `TestNoOSArgsWhenTesting` hermetic, deleted |
| logrus | 108008340c | t.Skip with no reason | logrus_test.go:42 | fails | reason passed through helper `skip(t, msg)` |
| logrus | 05046d0a35 | t.Skip with no reason | logrus_test.go:42 | fails | same helper |
| logrus | 16fee40613 | test removed | text_formatter_test.go:151 | holds | `TestEscaping_Interface` gone; coverage unknown |
| logrus | ab89975209 | test removed | text_formatter_test.go:151 | holds | same test; coverage unknown |
| ripgrep | cff2849fd4 | test removed | crates/core/flags/defs.rs:3524 | holds | `test_index*` absent from tree; intent unknown |
| ripgrep | cff2849fd4 | test removed | crates/index/src/literal.rs:869 | unknown | not inspected |
| ripgrep | be739c7910 | test body emptied | crates/index/src/literal.rs:975 | fails | new test with comment-only body, not an emptying |
| fd | 56505fceee | test removed | src/sanitize.rs:109 | holds | `strips_newline` absent by name; coverage unknown |

## Full-history labels

The checker ran on every commit of ten repositories with full history: 23,000
commits from gin, cobra, chi, fzf, logrus, ripgrep, fd, bat, axum and serde. The
blocking rules produced 11 findings in total. Each one is in
`docs/labels/full-history-2026-11-18.jsonl`, and each was read at the flagged
line in its commit.

Two labels were applied. `claim_holds` is the literal claim in the table
above. `reason_documented` records whether a comment next to the flagged line
states why the test is skipped or ignored. The rule's own purpose is that a
skip with no stated reason is the problem. A comment that states the reason
meets that purpose. This second label was defined after the data was read, so
it is applied as a stricter reading of the bar.

## Measured precision

| Rule | Findings | Claim holds | Reason documented | Strict precision (claim holds and no reason) | Against the bar |
|---|---|---|---|---|---|
| #[ignore] with no reason | 11 (5 distinct tests) | 11 | 11 | 0 / 11 | fails (n < 20, precision 0) |
| t.Skip with no reason | 1 | 1 | 1 | 0 / 1 | fails (n < 20, precision 0) |
| test body emptied | 0 | - | - | - | fails (n = 0) |
| constant-true assertion | 0 | - | - | - | fails (n = 0) |
| test removed (now a fork) | 6 | 5 | - | not a block | not a block |

The 11 `#[ignore]` findings include 6 rows that repeat the same attribute across
merge and origin commits. They come from 5 distinct tests.

Earlier 400-commit results (sample of pre-fix binary): two skip-rule findings and
one emptied-body finding were false, and both shapes were fixed afterwards.

Under the strict reading, every blocking finding in real history carries a
stated reason, so each one is a false positive by the rule's purpose. The
literal claim is true in all 12 blocking findings, which is why the first
definition alone would look clean.

## Synthetic corpus

`.verify/corpus/run.sh` runs 20 hand-written diffs. Each case's expected verdict
was written before the binary was run. The corpus measures the binary against
known answers. Its labels are not real findings, so it cannot count toward the
bar above.

| Rule | tp | fp | fn | precision | recall |
|---|---|---|---|---|---|
| t.Skip with no reason | 1 | 0 | 0 | 1.00 | 1.00 |
| #[ignore] with no reason | 1 | 0 | 0 | 1.00 | 1.00 |
| constant-true assertion | 1 | 0 | 0 | 1.00 | 1.00 |
| test body emptied | 3 | 0 | 0 | 1.00 | 1.00 |

Forks: 10 of 10 expected forks found. Clean cases with a fork: 0 of 10. The
dependency forks now fire without a `Cargo.lock`, with the cost marked
`transitive cost unmeasured`. The corpus runs as a phase of `.verify/run.sh`.

## Wide candidate sweep

Eighteen repositories were checked: the ten with full history, and eight more
with history since 2024-06-01 (syncthing, hugo, frp, ollama, tauri, uv, rustdesk,
codex). The candidate commits were every commit whose diff adds `#[ignore`,
`.Skip(` or `assert!(true)`, plus every commit that touches a test file or adds a
test marker, for the emptied-body rule. That is about 11,000 commits. The sweep
produced 62 blocking findings after deduplication. Labels are in
`docs/labels/wide-candidates-2026-11-18.jsonl`, one row per finding. The earlier
file `full-history-2026-11-18.jsonl` is a subset of the same revisions.

| Rule | Findings | Claim holds (literal) | Undocumented (strict correct) | Strict precision | Against the bar |
|---|---|---|---|---|---|
| #[ignore] with no reason | 38 | 38 | 23 | 0.61 | fails (precision < 0.90) |
| t.Skip with no reason | 23 | 23 | 19 | 0.83 | fails (precision < 0.90) |
| test body emptied | 1 | 1 | 1 | 1.00 | fails (n = 1) |
| constant-true assertion | 0 | - | - | - | fails (n = 0) |

"Reason documented" means one of: a comment within four lines above the flagged
line, a trailing comment on it, the enclosing function's doc comment, or a
reason argument on the skip. A guard condition such as
`if !dartsass.Supports() { t.Skip() }` does not count. The condition names the
cause to a reader, but nothing states a reason. This is the definition that was
applied. If guard conditions did count, the `t.Skip()` strict precision would
fall to 0.00, because all 19 undocumented findings are guard-only or fuzz-input
filters.

Circularity: if the `#[ignore]` and `t.Skip()` rules are changed to honour
documented reasons, the findings that remain are undocumented by construction.
Their strict precision would then be 1.00 by definition. A bar that the code
change can satisfy by construction is not a measurement. Admission needs labels
that were produced independently of the rule change.

## Comment-aware rule (2026-11-19)

The `#[ignore]` and `t.Skip()` rules now read the file's text. A bare skip is a
fork, not a violation, when a reason is documented nearby: a trailing comment on
the line, a comment block directly above it (attribute lines are skipped, and a
commented-out attribute does not count), or, for Go skips, the enclosing
function's doc comment. A guard condition does not count.

Re-running the 62 labelled commits with the new binary:

| Rule | Blocking before | Blocking after | Documented, now a fork | Undocumented, now passes |
|---|---|---|---|---|
| #[ignore] with no reason | 38 | 19 | 15 | 4 |
| t.Skip with no reason | 23 | 10 | 13 | 9 |
| test body emptied | 1 | 1 | - | - |

Every documented finding now produces a fork. The 13 undocumented findings that
now pass do so because the rule counts any comment as a reason: doc comments on
a test for `#[ignore]`, and comments above the enclosing function for `t.Skip()`.
The labels were stricter and required a comment that states why. The code cannot
judge whether a comment states a reason, so this is a known gap.

Measured against those labels, every block that remains is labelled undocumented
(19 of 19 for `#[ignore]`, 10 of 10 for `t.Skip()`). That figure is circular.
The labels were used to design the change, so they cannot admit a rule. Admission
needs labels produced independently. Even on these labels, `#[ignore]` (19) falls
short of n ≥ 20, and `t.Skip()` (10) does not reach it.

## Status

No rule meets the bar. The wide sweep produced 62 blocking findings. Strict
precision is 0.61 for `#[ignore]` (n = 38), 0.83 for `t.Skip()` (n = 23), and
n = 1 for the emptied-body rule. The constant-true assertion rule has no
findings in real history. The three blocking rules remain `provisional`: they
still exit 1, and the output says so. Under the bar they should not block.
Demoting them to forks, or changing them to honour documented reasons, is a
product decision and is left for the owner.

Reaching n ≥ 20 per rule is now met for the `#[ignore]` and `t.Skip()` rules.
Their precision is not. The emptied-body and constant-true rules still need
repositories that exercise them.

## Re-measured after the reason fix

The 0.61 above was measured before two binary changes: a reason now has to state
a cause (a comment that only describes the test no longer counts), and repeat
findings collapse per file rather than across the repo. Replaying the binary on
the 62 labeled findings:

- Agreement with the strict label (`reason_documented`): 56 of 62, up from 43.
- Blocks the binary emits on labeled rows: 37. All 37 are strictly unreasoned,
  so precision on this set is 1.00 (n = 37).
- Unreasoned rows the binary does not block: 6, all Hugo skips whose enclosing
  doc says `Issue NNNN`. The labels disagree on bare issue references, so those
  are forked on purpose: a blocking rule errs toward fork when the reason is
  ambiguous.
- The cue list was tuned on these same 62 findings. The 1.00 is in-sample, not a
  held-out result.
- Sweep of the last 300 first-parent commits in each of the eight repositories
  (2,397 commits run): 8 blocks, all eight already among the labeled findings. No
  new blocking case appeared.

Status is unchanged: `provisional`. A held-out set of labeled findings, labeled
blind to the cue list, is needed before the bar can be called met.
