# Go 03 · zap/logrus where `log/slog` exists

**Fires when** `go.uber.org/zap`, `sirupsen/logrus`, or `rs/zerolog` is added to
`go.mod` on a module declaring Go 1.21 or later.

**Proposed** A structured logger with its own field constructors, its own level
type, and its own handler interface. Every call site is written against that
API and cannot be replaced without touching all of them.

**Native** `log/slog` from the standard library: `slog.Info("req", "path", p)`,
`slog.NewJSONHandler(os.Stdout, nil)`, `slog.With` for context. Swappable via
`slog.SetDefault`, zero dependencies.

**6-month cost** Two logging idioms in one codebase once a dependency pulls its
own logger; the vendor API becomes load-bearing and the JSON shape becomes an
accidental contract.

**Skip when** the hot path is measured and `slog` allocation is the bottleneck
(it is real: zap is ~3-5× faster), or the logger must ship to an existing
vendor pipeline that `slog` handlers cannot satisfy.

**Source** Go 1.21 `log/slog` release notes; `uber-go/guide` logging section.
