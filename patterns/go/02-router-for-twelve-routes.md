# Go 02 · Framework for twelve routes

**Fires when** `gin-gonic/gin`, `labstack/echo`, `go-chi/chi`, or `gorilla/mux` is
added to `go.mod` and the diff contains fifteen handlers or fewer.

**Proposed** A routing framework with its own context type, middleware signature,
binding tags, and validation DSL. Every handler now receives a `*gin.Context`
instead of `http.ResponseWriter` and `*http.Request`.

**Native** `http.ServeMux` with method-and-wildcard patterns (Go 1.22+):
`mux.HandleFunc("GET /users/{id}", h)`, `r.PathValue("id")`. Middleware is a
twenty-line `func(http.Handler) http.Handler` chain.

**6-month cost** Framework types leak into every handler, so the framework becomes
the interface of the application; upgrading it is now a refactor, and dropping it
is a rewrite.

**Skip when** the team already runs the framework elsewhere and consistency is
worth more than the dependency, or the diff needs real per-route middleware,
grouping, and binding on more than ~50 routes.

**Source** `go-critic` and `uber-go/guide` on interface surface; Go 1.22 release
notes for `ServeMux` patterns.
