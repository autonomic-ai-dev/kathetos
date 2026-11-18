# Kubernetes 02 · Helm for one chart

**Fires when** a Helm `Chart.yaml` is added and the chart describes fewer than
three workloads, or the same workload is templated for one environment only.

**Proposed** A chart with `values.yaml`, per-environment overrides, helpers, and
a template language between the reviewer and the manifest that actually runs.

**Native** Plain manifests in one directory, applied with `kubectl apply -f`, with
the environment difference written as a second short file. For one binary on one
host, skip the cluster entirely and use the `systemd` unit described in
[Kubernetes 01](01-kubernetes-for-one-service.md).

**6-month cost** The rendered manifest is the real artifact but nobody reviews
it; a `helm template` diff becomes the only audit trail. Every value you expose is
a public surface you must keep compatible.

**Skip when** three or more workloads share a release lifecycle, the chart is
consumed by other teams or published, or you already use Helm across services
and consistency beats the extra layer.

**Source** Helm documentation on chart structure (`Chart.yaml`, `values.yaml`);
`kubectl apply` reference.
