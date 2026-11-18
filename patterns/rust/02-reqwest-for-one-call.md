# Rust 02 · reqwest for one call

**Fires when** `reqwest` is added and the diff has a single request site with no
streaming, no connection reuse across calls, and no proxy/TLS-special config.

**Proposed** `reqwest` + `hyper` + `hyper-util` + `tower` + `rustls` + a runtime.
Roughly +18 transitive crates and a multi-second cold build for one GET.

**Native** `ureq`: blocking, ~4 deps, TLS included, `ureq::get(url).call()`. Or
`std::net` directly if the endpoint is a known host over plaintext HTTP.

**6-month cost** The TLS and HTTP stack is now your supply chain: every rustls or
hyper advisory is a release, and the async runtime it pulled in constrains how
the rest of the binary is written.

**Skip when** the call site multiplies (paginated sync, concurrent fetches),
needs HTTP/2 multiplexing, or needs proxy and connection-pool tuning — then
`reqwest` is the boring option.

**Source** `blessed.rs` HTTP client comparison; crate graph measured from
`Cargo.lock`.
