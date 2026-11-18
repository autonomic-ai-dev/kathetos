# Kathetos

**κάθετος** — the plumb line. Hangs still, tells you the wall is crooked.

An anti-bloat and principal-context layer for coding agents. Runs at full agent
speed: no LLM calls, no second turn, no test re-runs.

Two failure modes, two moments, one format:

| | Moment | Artifact |
|---|---|---|
| Agent inflates architecture | before codegen | `SKILL.md` + 15 cards → a 6-line fork |
| You merge what you can't defend | before merge | `RECEIPT-SPEC.md` → a 3-item receipt |

Both render the same four lines: **decision · boring native option · 6-month
cost · one question only you can answer.** One renderer, two call sites.

## Why it is diff-only

`kathetos check` reads `git diff base...HEAD` and nothing else. On a fresh repo it
sees everything from commit #1. On a 100k-LOC repo it sees only what you are
touching, so day-1 CI is green and legacy debt is nobody's problem. No baseline
file, no ratchet, no whole-repo scan.

## Install

```sh
cargo install --path .          # binary: kathetos
./target/release/kathetos check --diff origin/main...HEAD
```

Zero dependencies, 379 KB, no runtime. Medians from 50 real commits across two
repos: **39 ms median, 77 ms p95, 88 ms max**.

```
kathetos · rust · 15 files · 48ms · deps +7 -> +75 pkg (220 in tree) · tests 0
forks: 3
1. agent-brain/Cargo.toml:17  7 direct deps for one change
   native: ship the part of the feature that needs none of them
   6mo:    +75 packages, 220 already in the tree
   ask:    which of these does the diff call directly?
2. agent-brain/Cargo.toml:17  age added (12 refs)
   native: the age binary as a subprocess, or ssh-key encryption
   6mo:    +51 packages
   ask:    is the key material ever handled in-process?
3. agent-brain/Cargo.toml:23  keyring added (1 ref)
   native: OS keychain CLI: security find-generic-password / secret-tool
   6mo:    +1 package
   ask:    does this need to work on more than one OS in v1?
```

Exit `0` nothing fired, `1` a hard rule fired, `2` the base revision could not be
resolved. `--json` for CI and agents. See `docs/teardown-2026-11-18.md` for the
measurements, including the eleven rules that were wrong.

## Install the skill

Copy `SKILL.md` and `patterns/` into your agent's skills directory. For Claude
Code:

```sh
git clone https://github.com/autonomic-ai-dev/kathetos
mkdir -p .claude/skills/kathetos
cp -r kathetos/SKILL.md kathetos/patterns .claude/skills/kathetos/
```

The content is plain markdown; Codex and OpenCode read the same file.

## The cards

Fifteen decision points, each with a `Skip when` line — because the fastest way
to make a tool useless is to fire on the cases where the boring option is wrong.

**Go** — HTTP timeout · framework for twelve routes · logger where `log/slog`
exists · unbounded goroutines · error masking

**Rust** — tokio for a CLI · reqwest for one call · trait object for one impl ·
anyhow in a library · derive-macro crates for hand-writable impls

**Linux** — Compose for local services · Redis as a queue · Kubernetes for one
deployable · in-app ticker · container per "host"

Error masking and `.unwrap()` are cards, not checks. A diff can tell you an error
was discarded; only a person can tell you whether it mattered.

## What this deliberately is not

No LLM review. No PR comment bot. No dashboards. No token compression. No score.
No green checkmark — a receipt that says "0 forks" is a fact about the diff, not
a judgement about the code.

## Research

- GitClear, *The Maintainability Gap* (June 2026): +81% block duplication, −70%
  refactoring, −74% legacy maintenance across 623M changes.
- *AI-Generated Smells* (arXiv:2605.02741): Reasoning-Complexity Trade-off;
  Volume-Quality Inverse Law.
- SpecBench (arXiv:2605.21384): oversight collapses onto the test suite.
- RHB (arXiv:2605.02964): exploit rates 0–13.9%; environmental hardening cuts
  exploits 5.7pp (87.7% relative) **without degrading task success**.
- *Dependency Gaps* (arXiv:2512.22387): 31.7% of generated projects fail to
  execute from declared dependencies alone.

## License

Apache-2.0. All fifteen cards are written clean-room from the underlying idea;
no text is copied from any source, including CC BY-NC-ND sources such as
`teivah/100-go-mistakes` (ideas cited, text never reproduced).
