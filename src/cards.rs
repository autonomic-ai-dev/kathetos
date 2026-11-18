//! The catalog, compiled in. A fork is only emitted when a card exists — the
//! checker refuses to editorialise about dependencies it has no mechanism for.

pub struct Card {
    pub native: &'static str,
    pub cost: &'static str,
    pub ask: &'static str,
}

pub fn card(dep: &str) -> Option<Card> {
    let c = match dep {
        "tokio" => Card {
            native: "std::thread + std::net, no runtime",
            cost: "a scheduler you now schedule around; Send bounds spread through internal types",
            ask: "does this binary serve concurrent sockets?",
        },
        "reqwest" => Card {
            native: "ureq, blocking, ~4 crates",
            cost: "TLS + HTTP stack becomes your supply chain; runtime constrains the rest of the binary",
            ask: "will this ever need concurrent in-flight calls?",
        },
        "axum" | "actix-web" | "warp" | "rocket" => Card {
            native: "hyper directly, or std::net for a single endpoint",
            cost: "framework types leak into every handler; swapping it is a rewrite",
            ask: "how many routes does the diff actually add?",
        },
        "tonic" | "prost" => Card {
            native: "serde_json over std::net, or a length-prefixed local socket",
            cost: "protoc in the build, a codegen step, and a wire format you must version",
            ask: "is this RPC crossing a process boundary or a module boundary?",
        },
        "chrono" => Card {
            native: "std::time + a 12-line RFC3339 formatter",
            cost: "a full calendar/timezone engine for formatting one timestamp",
            ask: "does the diff do arithmetic across timezones?",
        },
        "anyhow" => Card {
            native: "thiserror enum at the crate boundary; anyhow only in main and tests",
            cost: "public errors become un-matchable; recovery turns into string sniffing",
            ask: "does an external caller need to match this error?",
        },
        "rayon" => Card {
            native: "std::thread::scope over chunks",
            cost: "a work-stealing pool that competes with any other runtime you add",
            ask: "is the work CPU-bound or IO-bound?",
        },
        "opendal" => Card {
            native: "the provider CLI as a subprocess (aws s3 cp, rclone)",
            cost: "an S3/QUIC/i18n layer in the build for one upload path",
            ask: "which provider does the diff actually talk to?",
        },
        "keyring" => Card {
            native: "OS keychain CLI: security find-generic-password / secret-tool",
            cost: "a cross-platform secret store you ship, debug and support on three OSes",
            ask: "does this need to work on more than one OS in v1?",
        },
        "age" => Card {
            native: "the age binary as a subprocess, or ssh-key encryption",
            cost: "crypto primitives compiled into your binary become your CVE surface",
            ask: "is the key material ever handled in-process?",
        },
        "zstd" | "flate2" | "brotli" => Card {
            native: "the zstd/gzip CLI as a subprocess",
            cost: "a C toolchain in the build, and a compression level you must tune forever",
            ask: "is decompression on the hot path?",
        },
        "tar" => Card {
            native: "the tar binary as a subprocess",
            cost: "hand-rolled archive semantics for a file that ships once",
            ask: "is this archive read by anything but your own unpacker?",
        },
        "redis" | "redis-rs" => Card {
            native: "Postgres SKIP LOCKED, or SQLite with BEGIN IMMEDIATE",
            cost: "a second source of truth with no shared transaction; one more backup to restore-test",
            ask: "do jobs fan out across hosts, or serialize on one?",
        },
        "gin" | "gin-gonic" | "echo" | "chi" | "gorilla/mux" => Card {
            native: "http.ServeMux with method+wildcard patterns (Go 1.22+)",
            cost: "the framework context type becomes the application interface",
            ask: "how many handlers does the diff add?",
        },
        "zap" | "logrus" | "zerolog" => Card {
            native: "log/slog from the standard library",
            cost: "two logging idioms once a dependency brings its own; JSON shape becomes a contract",
            ask: "is slog allocation measured as the bottleneck?",
        },
        "sqlx" | "diesel" | "gorm" | "ent" => Card {
            native: "database/sql + hand-written SQL",
            cost: "a query language, a migration story and a compile step between you and the database",
            ask: "does the diff need compile-time query checking?",
        },
        "kube" | "k8s-openapi" => Card {
            native: "kubectl as a subprocess, or systemd on the target host",
            cost: "the whole API surface pinned to a cluster version you must track",
            ask: "does this run inside the cluster or manage it?",
        },
        "bollard" | "docker-api" => Card {
            native: "the docker CLI, or containerd's socket directly",
            cost: "a Docker API version handshake on every host you support",
            ask: "is this managing containers or just running one?",
        },
        _ => return None,
    };
    Some(c)
}

/// Infrastructure files added to a diff. These are not dependencies, so they
/// have their own catalogue, keyed by what the file declares.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Infra {
    Compose,
    Dockerfile,
    KubeWorkload,
    Helm,
}

pub fn infra(kind: Infra) -> Card {
    match kind {
        Infra::Compose => Card {
            native: "one systemd --user unit per process, Restart=on-failure, EnvironmentFile",
            cost: "the Compose file becomes a second description of the system that production never runs",
            ask: "does every service in this file run only on the developer's own machine?",
        },
        Infra::Dockerfile => Card {
            native: "ship the compiled binary under a systemd unit, built in CI",
            cost: "a base image to rebuild for every OS patch, a registry to keep alive, a build context that drifts",
            ask: "does the target consume images, or one binary on a host you own?",
        },
        Infra::KubeWorkload => Card {
            native: "a systemd unit with Restart=always and MemoryMax=, caddy in front for TLS",
            cost: "a control plane, a YAML surface and a node-upgrade cadence for one binary",
            ask: "does this need horizontal scaling on real traffic, or a cluster you already run?",
        },
        Infra::Helm => Card {
            native: "plain manifests applied with kubectl apply, or the systemd unit",
            cost: "a template language between the reviewer and the manifest that actually runs",
            ask: "does the chart hold three or more workloads that share a release?",
        },
    }
}
