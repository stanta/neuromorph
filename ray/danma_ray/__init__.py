"""Local-only Ray control plane for the existing Rust DANMA TCP shards.

Ray manages a bounded number of *shard processes*, never individual neurons or
hot-path forward/backward messages. The current Rust v1 protocol is loopback-only.
"""
from .cluster import (
    ClusterSpec,
    DanmaCluster,
    DanmaNode,
    NeuronSpec,
    start_local_cluster,
    tcp_request,
)

__all__ = [
    "ClusterSpec", "DanmaCluster", "DanmaNode", "NeuronSpec",
    "start_local_cluster", "tcp_request",
]
