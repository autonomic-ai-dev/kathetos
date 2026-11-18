# Docker 02 · Dockerfile for one binary

**Fires when** a `Dockerfile` (or `Dockerfile.*` / `*.Dockerfile`) is added and the
image contains one compiled program and nothing that needs a container runtime
to run: no sidecar, no second process, no per-service base image.

**Proposed** A base image, a build stage, a registry push, an image tag scheme,
and a container runtime on every host — to ship a file that could be copied.

**Native** Build the binary in CI, copy it to the host, and run it under a
`systemd` unit with `Restart=on-failure`. Rollback is a symlink swap plus
`systemctl restart`. No registry, no image layers, no base-image CVE queue.

**6-month cost** A base image you must rebuild for every OS patch, a registry
to keep alive and backed up, and a build context that quietly diverges from the
host it deploys to. The image also becomes the unit you debug, so the first
question on every incident is which layer broke.

**Skip when** the target is a container platform (Kubernetes, ECS, Cloud Run)
that consumes images, the program needs a libc or toolchain the host cannot
provide, or the build must be reproducible across machines and the image is the
only artifact that guarantees it.

**Source** `systemd.exec(5)` (`Restart=`, `EnvironmentFile=`); Dockerfile
reference, multi-stage builds (idea only, for the skip case).
