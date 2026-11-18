# Rust 04 · anyhow in a library

**Fires when** `anyhow` is added under a crate that exposes a `pub` API, `lib.rs`,
or is published — as opposed to a binary's `main.rs`.

**Proposed** `anyhow::Result<T>` in public signatures. Callers cannot match on
the error, so the only recovery available is `to_string()` and a string comparison
that breaks silently when a message is reworded.

**Native** A concrete `thiserror` enum at the crate boundary, or a hand-written
`impl std::error::Error` for the three real failure modes. `anyhow` stays in the
binary and in tests, where the caller is a human.

**6-month cost** The public error type becomes un-matchable, so recovery logic
migrates into string sniffing; adding a variant is now a semver event if you use
`#[non_exhaustive]`, or a semver break if you do not.

**Skip when** the crate is an application crate that merely lives in `src/lib.rs`
with no external consumers, or the function is `pub` only for integration tests —
then `anyhow` is the correct, boring choice.

**Source** `rust-unofficial/patterns` error-handling chapter; clippy's
`result_large_err` and the `thiserror`/`anyhow` split in `blessed.rs`.
