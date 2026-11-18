# Linux 02 · Redis as a queue

**Fires when** a Redis client is added and the diff uses `LPUSH`/`BRPOP`,
`XADD`/`XREADGROUP`, or a Bull/Celery-style worker for jobs that live in one
database the app already writes to.

**Proposed** A second stateful system with its own persistence story, its own
auth, its own memory limit, and no transactional relationship to the data the
job is about. Enqueue succeeds, the write it describes rolls back.

**Native** **Postgres:** `SELECT ... FOR UPDATE SKIP LOCKED LIMIT 1` inside a
transaction — the queue *is* the table, so enqueue and state change commit
together. **SQLite:** one table, `BEGIN IMMEDIATE`, `UPDATE ... WHERE id = (SELECT
id ... LIMIT 1) RETURNING *`, `PRAGMA busy_timeout`. Single writer, no broker.

**6-month cost** Two sources of truth with no shared transaction, so every
"job ran but the row is missing" bug is a reconciliation script, and the queue
becomes the second backup you forget to restore-test.

**Skip when** jobs must fan out across hosts at a rate one database cannot absorb
(roughly >1k writes/s sustained), or you need cross-language consumers, delayed
schedules, and retries with backoff on day one.

**Source** PostgreSQL `SELECT` docs (`SKIP LOCKED`); SQLite `RETURNING` (3.35+)
and `PRAGMA busy_timeout`.
