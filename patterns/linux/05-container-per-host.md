# Linux 05 · Container per "host"

**Fires when** a container is added not to package a service but to give it a
distinct address, hostname, or network namespace — a container used as a "host".

**Proposed** An image, a bridge network, a DNS entry, and a port publish, to
obtain what is otherwise a routing table entry. Debugging now crosses a namespace
boundary: `localhost` means the container, not the machine.

**Native** Loopback sharding: `127.0.0.1`, `127.0.0.2`, `127.0.0.3` are all the
local machine on Linux. Bind each service to its own address and port, use
`/etc/hosts` for names, and `ss -ltnp` shows the whole topology in one command.

**6-month cost** The address becomes an image: rebuilds to change a port, an
image registry in the critical path of a config edit, and logs spread across two
`docker logs` invocations.

**Skip when** the isolation is a real security boundary (untrusted code, a
different libc or kernel version, a service that must not see the host
filesystem), which is what namespaces are actually for.

**Source** `ip-address(8)` (127.0.0.0/8 scope host); `ss(8)`.
