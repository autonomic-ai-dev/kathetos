# Go 04 · Unbounded goroutines

**Fires when** a `go func()` or `go method()` appears inside a loop, or per
request, with no semaphore, worker pool, `errgroup.SetLimit`, or buffered
channel bounding it.

**Proposed** One goroutine per item. It reads as concurrency; it is actually an
unbounded fan-out with no backpressure. The failure arrives as memory growth and
a scheduler that spends its time on runnable queues.

**Native** `errgroup.Group` with `g.SetLimit(n)` for the common case, or a
buffered channel of capacity `n` as a token bucket. The limit should be a named
constant derived from what is downstream (connection pool size, CPU count).

**6-month cost** The limit that was never chosen gets discovered during an
incident, at which point the correct value is guesswork and the queue is already
somebody else's timeout budget.

**Skip when** the loop is bounded by a compile-time constant small enough to
reason about, and the work is genuinely independent (fan-in over ≤ 8 items).

**Source** `uber-go/guide` concurrency section; `golang.org/x/sync/errgroup`
documentation.
