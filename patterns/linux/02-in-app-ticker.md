# Linux 04 · In-app ticker for housekeeping

**Fires when** a diff adds `time.Ticker`, `tokio::time::interval`,
`setInterval`, or a `for { sleep; work }` loop for periodic housekeeping inside
the application process.

**Proposed** A scheduler inside the process. It only runs while the process is
healthy and only once per instance — so three replicas run the job three times,
and a crash loop silently means the job never runs. Missed runs are lost.

**Native** A `systemd` timer: `OnCalendar=hourly`, `Persistent=true` (catches up
after downtime), `RandomizedDelaySec=60` (de-synchronises replicas),
`Type=oneshot` with the work as a subcommand of the same binary.

**6-month cost** Duplicate execution becomes a data bug that appears only under
scale, and "did the cleanup run?" has no answer outside the app's own logs.

**Skip when** the work is genuinely per-process and in-memory (cache eviction,
connection pool health, metrics flush), or the process is the only writer and
runs exactly once by construction.

**Source** `systemd.timer(5)`, `systemd.time(7)` `OnCalendar` syntax.
