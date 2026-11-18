# Go 01 · HTTP client without a timeout

**Fires when** a diff constructs `&http.Client{}` or uses `http.DefaultClient` and
no `Timeout` is set in the same hunk.

**Proposed** A zero-value `http.Client` has **no timeout** — not a short default,
none. A wedged upstream holds the goroutine until the kernel gives up, which for
a half-open connection is measured in hours.

**Native** `&http.Client{Timeout: 10 * time.Second}`, and the caller attaches its
own deadline with `req.WithContext(ctx)` so the timeout is a ceiling, not the
policy.

**6-month cost** One hung upstream drains a worker pool; the fix becomes a
timeout audit across every call site, most of them written by someone else.

**Skip when** the client is deliberately long-lived for streaming bodies (SSE,
chunked downloads) — then the deadline belongs on the context, and a client
`Timeout` would truncate legitimate streams.

**Source** `uber-go/guide` (timeouts); the HTTP-client mistake in the
`100-go-mistakes` index (idea only, text not reproduced).
