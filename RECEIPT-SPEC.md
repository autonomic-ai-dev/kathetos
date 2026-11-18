# Diff Receipt Spec (v0.1)

The second failure mode is not bloat. It is **you becoming a passenger** — merging
a diff you cannot defend because a tool printed a verdict.

SpecBench (arXiv:2605.21384) names the mechanism: as agents produce more code
than anyone can review, "oversight collapses onto a single surface." A green
checkmark *is* that collapse. So this spec forbids the checkmark.

## Rule 0 — a receipt is not an approval

`kathetos check` never prints `OK`, `PASS`, `LGTM`, a score, or a checkmark.
A clean run prints a fact, not a verdict:

```
kathetos · rust · 12 files · 41ms
forks: 0
```

No exit code implies correctness. Exit codes report *rule violations only*
(`0` nothing fired, `1` a hard rule fired, `2` usage error). "The code is good"
is not expressible.

## Rule 1 — only forks appear in the receipt

A **fork** is a point where the diff picked one path and a materially boring path
existed. A fact without an alternative is not a fork and is dropped, however
interesting. This single filter is what keeps the receipt to three items; do not
add a "notes" or "info" section to hold what it rejects.

## Rule 2 — three items maximum, ranked

Rank by `irreversibility × blast radius`, computed from the diff only:

| Signal | Weight |
|---|---|
| New direct dependency (Cargo.toml `[dependencies]`, go.mod non-`// indirect`) | 3 |
| New runtime process / container / listener bind | 4 |
| New module boundary or new directory with ≥ 3 files | 2 |
| Error path replaced (`?` → `unwrap`/`ok`, `if err` → `_ =`) | 3 |
| Test assertion removed or skipped | 4 |

Ties break by line number, ascending, so output is stable across runs.

## Rule 3 — every item carries the four fields

Same grammar as the pre-flight fork, so one renderer serves both fixtures:

```
1. src/net.rs:42  reqwest for one GET
   native: ureq, blocking, no runtime  (+2 deps, not +18)
   6mo:    a runtime you now schedule around for one request
   ask:    will this ever need concurrent in-flight calls?
```

- `file:line` — from the diff, always. An item without a location is a hypothesis.
- `native:` — mechanism plus the observable cost delta (`+2 deps, not +18`).
- `6mo:` — what compounds. Services, credentials, compile time, failure modes.
  Never the word "complexity" alone.
- `ask:` — one question only the human can answer. Not rhetorical, not answerable
  by reading the diff.

## Rule 4 — transitive attribution is measured, not assumed

- **Rust**: parse `Cargo.lock` (v3/v4) — it carries the dependency graph, so
  `+18 transitive` and `which of them were already in the tree` are exact.
- **Go**: `go.sum` carries **no edges**. Report `+N modules in go.sum
  (attribution unavailable)` and stop. Do not guess a tree, and do not shell out
  to `go mod graph` on the hot path. Opt-in `--deep` may run it.
- Never infer a count from a heuristic. A wrong number in a receipt costs more
  trust than a missing one.

## Rule 5 — checks that hard-fail

Exactly four, all test-side, all narrowed against 50 real commits.

1. A **hermetic** `#[test]` / `func Test*` deleted with no replacement in the same
   file. If the deleted test's old body touched `os.Getenv`, `exec.Command`,
   `http.`, `net.Dial`, or its name says `Live`/`Integration`, it is a fork
   instead: removing a test that needs the outside world is maintenance.
2. `#[ignore]` or `t.Skip()` added **with no stated reason**. A bare skip hides;
   `t.Skip("set PROJECT_X_HOME to run this")` is a decision someone can review,
   and becomes a fork.
3. A test body emptied.
4. An assertion replaced by `assert!(true)`.

**Why only these four.** Every other rule in draft 1 asked a question the text of
a diff cannot answer — "is this ignored error actionable?" — and scored 0/11 on
real history: `.unwrap()` inside `#[cfg(test)]`, `_ =` on `syscall.Flock`, a
cleanup call on a path that already returned the primary error. Those judgements
live in the skill cards, where a human is in the loop. A rule earns the right to
block only when it has a data structure behind it: an assertion count, or a
lockfile graph.

**Assertion-count deltas never block.** `assert!(x)` → table-driven loop drops the
count while strengthening the test; `assert_eq!` → `assert!(a == b)` keeps it while
destroying diagnostics. A counter is trivially gamed by padding, and padded
assertions are worse code than no gate. Count drops are reported as a fork item
with the line, never as a verdict.

## Rule 6 — never silently check nothing

If the merge base cannot be resolved (shallow clone, unfetched base, empty
diff), print the reason and exit `2`. `origin/main...HEAD` on a shallow checkout
resolves to an empty range and exits green — a checker that can quietly check
nothing is worse than no checker.

```
kathetos · 0 files · base origin/main not found (shallow clone?)
hint: git fetch --deepen=50 origin main
```

## Output modes

| Mode | Use |
|---|---|
| human (default) | **≤ 15 lines total**: 2-line header, then ≤ 3 forks × 4 lines. A clean run is 2 lines. |
| `--json` | CI, agents, and the Stop hook |
| `--sarif` | GitHub code scanning → inline PR annotations. Planned, not in v0.1. |

The `--json` shape:

```json
{"v":1,"lang":"rust","files":12,"ms":41,"forks":[{"id":"dep:reqwest","file":"src/net.rs","line":42,"weight":3,"ask":"..."}],"blocked":false}
```

No field named `ok`, `pass`, `score`, or `verdict`. Keep it that way.

## The metric that decides if this works

Per fork, log whether the fix was **better or worse than the pre-fork code**. That
is the only number that distinguishes a plumb line from a lint tax, and it is
per-rule — an aggregate 8% false-positive rate can hide one rule at 40%.
