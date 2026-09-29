"""Ray placement and subprocess lifecycle; neuron data stays in Rust/TCP."""
from __future__ import annotations

import json
import math
import os
from dataclasses import dataclass
from pathlib import Path
import socket
import struct
import subprocess
import tempfile
import time
from typing import Any

MAX_FRAME = 64 * 1024
MAX_NODES = 32
MAX_NEURONS = 128  # Existing danma-net v1 advert/route-table limit.


@dataclass(frozen=True)
class NeuronSpec:
    neuron_id: int
    weights: tuple[tuple[int, float], ...]


@dataclass(frozen=True)
class DanmaNode:
    node_id: int
    port: int
    neurons: tuple[NeuronSpec, ...]
    workers: int = 1
    mailbox: int = 64

    @property
    def address(self) -> str:
        return f"127.0.0.1:{self.port}"


@dataclass(frozen=True)
class ClusterSpec:
    nodes: tuple[DanmaNode, ...]

    def validate(self) -> None:
        if not 1 <= len(self.nodes) <= MAX_NODES:
            raise ValueError(f"cluster must have 1..{MAX_NODES} nodes")
        ids: set[int] = set()
        ports: set[int] = set()
        neurons: set[int] = set()
        for node in self.nodes:
            if type(node.node_id) is not int or not 1 <= node.node_id <= (1 << 64) - 1:
                raise ValueError("node ID must be nonzero u64")
            if type(node.port) is not int or not 1 <= node.port <= 65535:
                raise ValueError("port must be 1..65535")
            if node.node_id in ids or node.port in ports:
                raise ValueError("duplicate node ID or TCP port")
            if type(node.workers) is not int or not 1 <= node.workers <= 64:
                raise ValueError("workers must be 1..64")
            if type(node.mailbox) is not int or not 1 <= node.mailbox <= 4096:
                raise ValueError("mailbox must be 1..4096")
            if not node.neurons:
                raise ValueError("every node needs at least one neuron")
            ids.add(node.node_id)
            ports.add(node.port)
            for neuron in node.neurons:
                if type(neuron.neuron_id) is not int or not 1 <= neuron.neuron_id <= (1 << 64) - 1:
                    raise ValueError("neuron ID must be nonzero u64")
                if neuron.neuron_id in neurons:
                    raise ValueError("a neuron cannot have two owners")
                neurons.add(neuron.neuron_id)
                if not neuron.weights:
                    raise ValueError("the current CLI requires at least one incoming weight")
                sources: set[int] = set()
                for source, weight in neuron.weights:
                    if type(source) is not int or not 1 <= source <= (1 << 64) - 1:
                        raise ValueError("source neuron ID must be nonzero u64")
                    if source in sources:
                        raise ValueError("duplicate incoming source")
                    sources.add(source)
                    if not isinstance(weight, (float, int)) or isinstance(weight, bool) or not math.isfinite(weight):
                        raise ValueError("weights must be finite numbers")
        if len(neurons) > MAX_NEURONS:
            raise ValueError("danma-net v1 route table supports at most 128 neuron IDs")


def tcp_request(port: int, message: dict[str, Any], *, timeout: float = 2.0) -> dict[str, Any]:
    """One loopback-only DANMA v1 frame. Never auto-retry a mutating request."""
    if type(port) is not int or not 1 <= port <= 65535:
        raise ValueError("port must be 1..65535")
    if not math.isfinite(timeout) or timeout <= 0:
        raise ValueError("timeout must be finite and positive")
    data = json.dumps(message, allow_nan=False, separators=(",", ":")).encode("utf-8")
    if not 0 < len(data) <= MAX_FRAME:
        raise ValueError("DANMA message exceeds 64 KiB")
    with socket.create_connection(("127.0.0.1", port), timeout=timeout) as sock:
        sock.settimeout(timeout)
        sock.sendall(struct.pack("!I", len(data)) + data)
        header = _read_exact(sock, 4)
        length = struct.unpack("!I", header)[0]
        if not 0 < length <= MAX_FRAME:
            raise ValueError("invalid DANMA reply frame")
        result = json.loads(_read_exact(sock, length))
        if not isinstance(result, dict):
            raise ValueError("DANMA reply must be a JSON object")
        return result


def _read_exact(sock: socket.socket, n: int) -> bytes:
    out = bytearray()
    while len(out) < n:
        part = sock.recv(n - len(out))
        if not part:
            raise EOFError("DANMA connection closed before a complete frame")
        out.extend(part)
    return bytes(out)


def _command(binary: str, node: DanmaNode, nodes: tuple[DanmaNode, ...]) -> list[str]:
    cmd = [binary, "--id", str(node.node_id), "--listen", node.address,
           "--workers", str(node.workers), "--mailbox", str(node.mailbox)]
    for neuron in node.neurons:
        cmd += ["--neuron", str(neuron.neuron_id)]
        for source, weight in neuron.weights:
            cmd += ["--weight", f"{source}:{weight}"]
    for peer in nodes:
        if peer.node_id != node.node_id:
            cmd += ["--peer", f"{peer.node_id}@{peer.address}"]
    return cmd


class _Process:
    """One Ray actor owns one Rust subprocess. No automatic restart or replay."""

    def __init__(self, binary: str, node: DanmaNode, nodes: tuple[DanmaNode, ...], timeout_s: float):
        self.node = node
        self._log = tempfile.TemporaryFile(mode="w+b")
        self._child: subprocess.Popen[bytes] | None = None
        try:
            self._child = subprocess.Popen(
                _command(binary, node, nodes),
                stdin=subprocess.DEVNULL, stdout=self._log, stderr=subprocess.STDOUT,
                close_fds=True,
            )
            until = time.monotonic() + timeout_s
            while time.monotonic() < until:
                if self._child.poll() is not None:
                    raise RuntimeError(f"DANMA node {node.node_id} exited: {self.logs()}")
                try:
                    if tcp_request(node.port, {"kind": "routes"}, timeout=0.25).get("kind") == "routes_result":
                        return
                except (OSError, EOFError, ValueError):
                    pass
                time.sleep(0.05)
            raise TimeoutError(f"DANMA node {node.node_id} did not become ready: {self.logs()}")
        except BaseException:
            self.stop()
            raise

    def logs(self) -> str:
        self._log.flush()
        self._log.seek(0, os.SEEK_END)
        size = self._log.tell()
        self._log.seek(max(0, size - 4096))
        return self._log.read().decode("utf-8", "replace")

    def health(self) -> dict[str, Any]:
        if self._child is None or self._child.poll() is not None:
            raise RuntimeError(f"DANMA node {self.node.node_id} is not running")
        routes = tcp_request(self.node.port, {"kind": "routes"})
        if routes.get("kind") != "routes_result":
            raise RuntimeError(f"node {self.node.node_id} returned invalid health response")
        return {"node_id": self.node.node_id, "port": self.node.port, "routes": routes["routes"]}

    def stop(self) -> None:
        if self._child is not None:
            if self._child.poll() is None:
                self._child.terminate()
                try:
                    self._child.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    self._child.kill()
                    self._child.wait(timeout=2)
            self._child = None
        if not self._log.closed:
            self._log.close()


class ShardActor:
    """Ray worker-side supervisor; DANMA events never transit this actor."""

    def __init__(self, binary: str, node: DanmaNode, nodes: tuple[DanmaNode, ...], timeout_s: float):
        self._process = _Process(binary, node, nodes, timeout_s)

    def health(self) -> dict[str, Any]:
        return self._process.health()

    def stop(self) -> None:
        self._process.stop()


class DanmaCluster:
    def __init__(self, ray: Any, actors: list[Any], spec: ClusterSpec):
        self._ray = ray
        self.actors = actors
        self.spec = spec
        self._closed = False

    @property
    def entry_port(self) -> int:
        return self.spec.nodes[0].port

    def close(self) -> None:
        if self._closed:
            return
        self._closed = True
        for actor in self.actors:
            try:
                self._ray.get(actor.stop.remote(), timeout=5)
            except Exception:
                # An unresponsive/killed Ray actor can leave an orphan Rust process.
                # No automatic unsafe restart is attempted.
                pass
            try:
                self._ray.kill(actor, no_restart=True)
            except Exception:
                pass

    def __enter__(self) -> DanmaCluster:
        return self

    def __exit__(self, *_: object) -> None:
        self.close()


def start_local_cluster(spec: ClusterSpec, *, binary: str | Path,
                        timeout_s: float = 12.0, gossip_timeout_s: float = 12.0) -> DanmaCluster:
    """Launch all shards on the driver's Ray node (required by TCP v1 loopback).

    Driver must run on that Ray host, not via remote Ray Client. Ray schedules
    shard processes; direct TCP/Gossip remains DANMA's data/control protocol.
    """
    spec.validate()
    if not math.isfinite(timeout_s) or timeout_s <= 0 or not math.isfinite(gossip_timeout_s) or gossip_timeout_s <= 0:
        raise ValueError("timeouts must be finite and positive")
    binary_path = Path(binary).resolve(strict=True)
    if not binary_path.is_file() or not os.access(binary_path, os.X_OK):
        raise ValueError("DANMA executable must be a local executable file")
    try:
        import ray
        from ray.util.scheduling_strategies import NodeAffinitySchedulingStrategy
    except ImportError as exc:
        raise RuntimeError("install Ray before starting the DANMA Ray cluster") from exc
    if not ray.is_initialized():
        raise RuntimeError("call ray.init() on the same host as DANMA before starting")
    local_node_id = ray.get_runtime_context().get_node_id()
    if not any(n.get("NodeID") == local_node_id and n.get("Alive") for n in ray.nodes()):
        raise RuntimeError("Ray driver has no live local node; remote Ray Client is unsupported")
    actor_type = ray.remote(max_restarts=0, max_task_retries=0)(ShardActor)
    actors: list[Any] = []
    cluster = DanmaCluster(ray, actors, spec)
    try:
        for node in spec.nodes:
            actor = actor_type.options(
                num_cpus=node.workers,
                scheduling_strategy=NodeAffinitySchedulingStrategy(node_id=local_node_id, soft=False),
            ).remote(str(binary_path), node, spec.nodes, timeout_s)
            actors.append(actor)
        # Readiness is a control-plane call; no per-event Ray RPC.
        ray.get([actor.health.remote() for actor in actors], timeout=timeout_s + 8)
        expected = {str(neuron.neuron_id): node.node_id for node in spec.nodes for neuron in node.neurons}
        until = time.monotonic() + gossip_timeout_s
        while time.monotonic() < until:
            try:
                if all(tcp_request(node.port, {"kind": "routes"}) == {"kind": "routes_result", "routes": expected}
                       for node in spec.nodes):
                    return cluster
            except (OSError, EOFError, ValueError):
                pass
            time.sleep(0.05)
        raise TimeoutError("DANMA gossip routes did not converge")
    except BaseException:
        cluster.close()
        raise
