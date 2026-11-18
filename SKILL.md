---
name: kathetos
description: Fires only at architecture decision points — adding a direct dependency that duplicates a stdlib/OS capability, introducing a new runtime process or port, adding a cache/queue/transport abstraction, or re-implementing something the OS already owns. Emits a max 6-line "architectural fork" (proposed vs boring native option vs 6-month risk) and waits for a one-word pick. Silent on routine edits, refactors, renames, and test-only changes.
license: Apache-2.0
---

# Kathetos — the plumb line

A plumb line does not build the wall. It hangs still and tells you whether the
wall is already crooked. This skill does the same thing at one moment: the moment
before the agent commits to an architecture.

## The one law

**Never slow the agent down.** You may emit at most one fork per task. You never
ask a question the developer cannot answer with one word. If the developer does
not answer, you proceed with the boring option and say so in one line. Silence is
consent for the boring option, never for the proposed one.

## When to fire (all four are diff-shaped signals)

Fire only when the work in progress does one of these:

1. **Adds a direct dependency that duplicates a stdlib or OS capability.**
   `tokio` for a CLI, `zap` for logging where `log/slog` exists, a router for
   twelve routes, `chrono` for one timestamp.
2. **Adds a runtime process, port, container, or daemon** the program did not
   have before. Docker service, Redis, a queue, a sidecar, a listener.
3. **Adds a cache, queue, transport, or persistence abstraction** in front of
   something that already worked in-process or in a single file.
4. **Re-implements something the OS already owns.** Scheduler → cron/systemd
   timer. Queue → SQLite or Postgres. Host isolation → loopback addresses.

## Never fire (this list is the product)

- Adding *any* dependency. Only the ones that shadow a native capability.
- More than 3 new files. File count is not an architecture decision.
- Routine edits, renames, formatting, test-only changes, bug fixes.
- A second time in the same task. One fork, then build at full speed.
- When the "boring" option cannot actually work. A card that does not apply is
  noise; a wrong card is worse than silence.
- When the developer already picked a direction this session.

Say nothing otherwise. No acknowledgement, no "looks good", no summary.

## The format (6 lines, hard cap)

```
kathetos · fork 1 of 1
you:    redis for the job queue
native: sqlite, one table, BEGIN IMMEDIATE + RETURNING
6mo:    1 service, 1 auth path, 1 backup story, 1 on-call surface
ask:    fanout across hosts, or serial work on one box?
```

- Line 1: header with the fork index. Never more than `1 of 1` unless a second
  decision genuinely appears mid-task (max `2 of 2`).
- Line 2: what the developer's plan actually is, in their words, ≤ 12 words.
- Line 3: the boring native option. Name the mechanism, not the vibe. If you
  cannot name the mechanism, you do not have a card — stay silent.
- Line 4: the 6-month cost of the proposed option. Services, credentials,
  failure modes, or compile time. Not "complexity".
- Line 5: one question that decides it. Answerable in one word.

No prose before or after. No markdown fences. No bullet lists.

The five fields are the whole fork. A sixth line is allowed for one `source:`
citation and nothing else — it exists so you never pad the argument to fill space.

## Answering

| Pick | What you do |
|---|---|
| `native` / `boring` / `stdlib` | Build the native option. Do not editorialise. |
| `proposed` / `keep` / `as-is` | Build the proposal. Do not re-warn. Never repeat a fork. |
| `why` | Quote the card's mechanism line only, then wait again. |
| anything else, or silence | Build the native option and write one line: `went native: <mechanism>`. |

`why` is the only case where you spend a second turn. Every other path costs
zero.

## Cards

Read only the card that matches. Do not load the directory.

**Go** — `patterns/go/`
| Card | Fires on |
|---|---|
| 01 · HTTP client without timeout | `http.Client{}` with no `Timeout` |
| 02 · Framework for twelve routes | gin/echo/chi added, ≤ 15 handlers in diff |
| 03 · zap/logrus where `log/slog` exists | logger dep added on Go ≥ 1.21 |
| 04 · Unbounded goroutines | `go ` in a loop, no limit/semaphore |
| 05 · Error masking | `_ = fn()`, empty `if err != nil {}` |

**Rust** — `patterns/rust/`
| Card | Fires on |
|---|---|
| 01 · tokio for a CLI | `tokio` added, no server/listener in diff |
| 02 · reqwest for one call | `reqwest` added, one call site |
| 03 · Trait object for one impl | `dyn Trait`/`Box<dyn>` with a single implementor |
| 04 · anyhow in a library | `anyhow` added under a `lib.rs`/published crate |
| 05 · Derive-macro crates for hand-writable impls | `derive_more`/`strum`/`thiserror` for ≤ 2 impls |

**Docker** — `patterns/docker/`
| Card | Fires on |
|---|---|
| 01 · Compose for local services | a compose file added (`docker-compose.yml`, `compose.yaml`) |
| 02 · Dockerfile for one binary | a `Dockerfile` added |
| 03 · Container per "host" | new container just for address/port isolation |

**Kubernetes** — `patterns/kubernetes/`
| Card | Fires on |
|---|---|
| 01 · Kubernetes for one service | a manifest added with `kind: Deployment`/`StatefulSet`/`DaemonSet` |
| 02 · Helm for one chart | a `Chart.yaml` added |

**Linux / bare metal** — `patterns/linux/`
| Card | Fires on |
|---|---|
| 01 · Redis as a queue | `redis` client added for jobs |
| 02 · In-app ticker | `time.Ticker`/`tokio::interval`/`setInterval` for housekeeping |

The Docker and Kubernetes rows fire from the checker (`rules::infra`), and only
for files the diff adds.

At merge time, the same grammar renders as a receipt — see `RECEIPT-SPEC.md`.
One format, two moments: before the code, and before the merge.
