# Go 05 · Error masking

**Fires when** a diff adds `_ = fn()` where `fn` returns an `error`, an empty
`if err != nil { }`, or `return nil` inside an error branch.

**Proposed** A discarded error. The compiler is satisfied, the linter needs
`-blank` to notice, and the failure is now invisible until a user reports a
silently missing write.

**Native** `return fmt.Errorf("flush counter: %w", err)` at the boundary, or
`if err != nil { return err }` when there is nothing to add. If the error truly
does not matter, say so once, with a comment that names why.

**6-month cost** One masked error turns a clear stack trace into a "sometimes it
doesn't save" bug that costs a day to bisect, and the pattern spreads because it
compiles.

**Skip when** the call is `Close()` on a resource whose data was already flushed
successfully, or a best-effort cleanup where the primary error is already being
returned — then `_ =` is honest and a comment is enough.

**Source** `uber-go/guide` error section; `errcheck -blank`; the error-handling
mistakes in `100-go-mistakes` (idea only).
