# Rust 05 · Derive-macro crates for hand-writable impls

**Fires when** `derive_more`, `strum`, `educe`, or `smart-default` is added for
two impls or fewer, and no existing derive is being extended.

**Proposed** A proc-macro crate plus its `syn`/`quote`/`proc-macro2` build
graph, typically 4-6 crates and a measurable compile-time step, to generate
`Display`, `From`, or `Default` for two types.

**Native** Write the impls. `impl fmt::Display for Kind` is nine lines and reads
better in review than a derive attribute whose expansion nobody opens. `From` is
three lines per variant.

**6-month cost** The derive becomes the only documented behaviour: a new team
member must expand the macro to know what `Display` prints, and the generated
format becomes a user-visible contract nobody intended.

**Skip when** the crate is used for dozens of types (serialisation-shaped
boilerplate), or for a reverse-engineering boundary where hand-written impls are
the actual bug source — `strum` over 20 enums is a good trade.

**Source** `cargo tree`/`-Z timings` for the proc-macro build cost;
`rust-unofficial/patterns` on explicit over implicit.
