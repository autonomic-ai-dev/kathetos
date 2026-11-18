# Rust 03 · Trait object for one impl

**Fires when** a diff introduces `dyn Trait`, `Box<dyn Trait>`, a `#[async_trait]`
block, or a factory returning `Box<dyn ...>` and exactly one type implements it.

**Proposed** A trait plus a boxed trait object plus (often) an async-trait macro,
to abstract over one implementation that has no second caller and no test double.

**Native** The concrete type, behind a plain `enum` if a second variant is
genuinely expected. Enum dispatch is monomorphic, allocation-free, `Sized`, and
gives exhaustiveness checking when the third variant appears.

**6-month cost** `dyn` erases the type at the boundary, so every caller holds a
box, every clone is an allocation, and adding a method means editing the trait
and every impl instead of one match arm.

**Skip when** the object is user-supplied at runtime (plugins, registered
handlers), crosses a crate boundary as a public extension point, or the set of
implementations must be open — that is what trait objects are for.

**Source** `rust-unofficial/patterns` (composition over inheritance, concrete
boundaries); `mre/idiomatic-rust` on monomorphisation (idea only).
