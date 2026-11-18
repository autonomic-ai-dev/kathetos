# Rust 01 · tokio for a CLI

**Fires when** `tokio` enters `[dependencies]` and the diff contains no listener,
no `serve`, and no concurrent in-flight work — a parse-and-print binary.

**Proposed** `#[tokio::main]` plus a runtime: ~18 transitive crates, a scheduler
thread pool sized to the CPU, and an `async fn main` that exists to satisfy one
`.await` on a library call.

**Native** Plain `fn main`, `std::thread::scope` for the two things that should
overlap, `std::net::TcpStream` for the one connection, and a blocking client
(`ureq`) if HTTP is needed. Threads are fine into the hundreds.

**6-month cost** The runtime boundary becomes the binary's architecture: every
future addition is expressed as a task, `Send` bounds propagate through internal
types, and the build gains ~18 crates that must track each other.

**Skip when** the diff contains a server, more than ~32 concurrent sockets, or a
library that is already async-only — at that point the runtime is the cheap
option, not the expensive one.

**Source** `blessed.rs` on runtime selection; `rust-unofficial/patterns` on
keeping boundaries concrete (idea only, MPL-2.0 text not reproduced).
