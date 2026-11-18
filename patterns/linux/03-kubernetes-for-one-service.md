# Linux 03 · Kubernetes for one deployable

**Fires when** Kubernetes manifests are added for a single image with a single
replica, or a Helm chart is introduced with fewer than three workloads.

**Proposed** A control plane, an ingress controller, a cert manager, a CNI, a
secrets story, and a YAML surface — to schedule one binary that binds one port.

**Native** The binary, a `systemd` unit with `Restart=always` and
`MemoryMax=`, `caddy` in front for TLS (automatic certificates), and
`journalctl` for logs. Zero orchestration layers, one deploy target, one rollback
(`systemctl restart` after a symlink swap).

**6-month cost** Every incident now has two candidate causes — the app and the
cluster — and the person debugging it learns YAML instead of the codebase. Node
upgrades become an application risk.

**Skip when** the service needs horizontal autoscaling on real traffic, runs on
hardware you do not own, or the org already runs a cluster and consistency beats
simplicity.

**Source** `systemd.unit(5)` (`Restart=`, `MemoryMax=`); Caddy automatic HTTPS
documentation.
