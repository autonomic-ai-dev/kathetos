# Linux 01 · Compose for local services

**Fires when** a `docker-compose.yml` is added and every service it defines runs on
the developer's own machine — no multi-arch, no remote daemon, no CI matrix.

**Proposed** A Compose file, an image build, a volume mapping, and a port map per
service, so that the local topology is maintained separately from the production
topology in a different language with different defaults.

**Native** One `systemd --user` unit per process, `Restart=on-failure`,
`EnvironmentFile` for config, and `journalctl -u name -f` for logs. Need two
things on port 80? Bind `127.0.0.2:80` and `127.0.0.3:80` — the whole `127.0.0.0/8`
block is loopback on Linux.

**6-month cost** The Compose file becomes the only accurate description of the
system, so production drift is invisible; every new service adds a volume, a
network, and a healthcheck that only exists locally.

**Skip when** the team spans machines and needs an identical environment, the
service genuinely needs cgroup limits or a non-host kernel, or the diff targets a
CI runner where the container is the reproducibility unit.

**Source** `systemd.exec(5)`, `systemd.unit(5)`; loopback routing in
`ip-address(8)`.
