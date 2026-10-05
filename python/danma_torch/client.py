"""Bounded, synchronous Python client for the existing localhost DANMA TCP v1.

One length-prefixed JSON request and one response per TCP connection.
A network timeout after sending a stateful command has an unknown outcome;
never transparently retry forward/backward with a new EventID.
"""

from __future__ import annotations

import ipaddress
import json
import math
import socket
import struct
from collections.abc import Sequence
from typing import Any

MAX_FRAME_BYTES = 64 * 1024
MAX_U64 = (1 << 64) - 1


class DANMAError(RuntimeError):
    """Protocol, validation, training or routed-computation failure."""


class DANMATransportError(DANMAError):
    """The remote outcome may be unknown after a socket/timeout failure."""


def checked_id(value: int, name: str) -> int:
    if type(value) is not int or not 1 <= value <= MAX_U64:
        raise ValueError(f"{name} must be a nonzero unsigned 64-bit integer")
    return value


def _read_exact(stream: socket.socket, length: int) -> bytes:
    output = bytearray()
    while len(output) < length:
        chunk = stream.recv(length - len(output))
        if not chunk:
            raise ConnectionError("DANMA TCP peer closed the frame")
        output.extend(chunk)
    return bytes(output)


class DANMAClient:
    """Communicate with one known entry node; that node routes by NeuronId.

    v1 is loopback-only. An allowlisted gossip peer name is NOT authentication.
    Do not expose the development node to an untrusted remote network.
    """

    def __init__(
        self,
        host: str,
        port: int,
        *,
        timeout_seconds: float = 4.0,
    ) -> None:
        try:
            loopback = host == "localhost" or ipaddress.ip_address(host).is_loopback
        except ValueError:
            loopback = False
        if not loopback:
            raise ValueError("DANMA TCP v1 only supports loopback hosts")
        if type(port) is not int or not 1 <= port <= 65535:
            raise ValueError("port must be 1..65535")
        if not isinstance(timeout_seconds, (int, float)) or (
            not math.isfinite(timeout_seconds) or timeout_seconds <= 0
        ):
            raise ValueError("timeout_seconds must be finite and positive")
        self.host = host
        self.port = port
        self.timeout_seconds = float(timeout_seconds)

    def request(self, message: dict[str, Any]) -> dict[str, Any]:
        try:
            encoded = json.dumps(
                message, allow_nan=False, separators=(",", ":")
            ).encode("utf-8")
        except (TypeError, ValueError) as exc:
            raise DANMAError(f"cannot encode DANMA request: {exc}") from exc
        if not 0 < len(encoded) <= MAX_FRAME_BYTES:
            raise DANMAError("DANMA request exceeds 64 KiB protocol frame limit")
        try:
            with socket.create_connection(
                (self.host, self.port), timeout=self.timeout_seconds
            ) as stream:
                stream.settimeout(self.timeout_seconds)
                stream.sendall(struct.pack(">I", len(encoded)) + encoded)
                frame_length = struct.unpack(">I", _read_exact(stream, 4))[0]
                if not 0 < frame_length <= MAX_FRAME_BYTES:
                    raise DANMAError("invalid or oversized DANMA response frame")
                payload = _read_exact(stream, frame_length)
        except (OSError, ConnectionError) as exc:
            raise DANMATransportError(
                f"DANMA transport failure; remote effect may have committed: {exc}"
            ) from exc
        try:
            reply = json.loads(payload)
        except (UnicodeError, ValueError) as exc:
            raise DANMAError(f"invalid DANMA JSON response: {exc}") from exc
        if not isinstance(reply, dict):
            raise DANMAError("DANMA response must be a JSON object")
        if reply.get("kind") == "error":
            raise DANMAError(f"DANMA node rejected request: {reply.get('code', 'unknown')}")
        return reply

    def routes(self) -> dict[str, int]:
        reply = self.request({"kind": "routes"})
        if reply.get("kind") != "routes_result" or not isinstance(
            reply.get("routes"), dict
        ):
            raise DANMAError("unexpected DANMA routes response")
        return reply["routes"]

    def inspect(self, neuron_id: int, *, route_hops: int = 4) -> dict[str, Any]:
        checked_id(neuron_id, "neuron_id")
        reply = self.request(
            {"kind": "inspect", "target": neuron_id, "route_hops": route_hops}
        )
        if reply.get("kind") != "inspect_result" or reply.get("target") != neuron_id:
            raise DANMAError("unexpected DANMA inspect response")
        return reply

    def forward(
        self,
        *,
        neuron_id: int,
        event_id: int,
        trace_id: int,
        inputs: Sequence[tuple[int, float]],
        training: bool,
        route_hops: int,
    ) -> float:
        reply = self.request(
            {
                "kind": "forward",
                "target": checked_id(neuron_id, "neuron_id"),
                "event_id": checked_id(event_id, "event_id"),
                "trace_id": checked_id(trace_id, "trace_id"),
                "route_hops": route_hops,
                "inputs": [
                    {
                        "from": checked_id(source, "input_id"),
                        "source_event_id": event_id,
                        "value": float(value),
                    }
                    for source, value in inputs
                ],
                "expected": [{"kind": "teacher"}] if training else [],
            }
        )
        if reply.get("kind") != "forward_result":
            raise DANMAError(f"unexpected DANMA forward response: {reply}")
        value = reply.get("output")
        if type(value) not in (int, float) or not math.isfinite(value):
            raise DANMAError("DANMA neuron returned a non-finite output")
        return float(value)

    def backward(
        self,
        *,
        neuron_id: int,
        event_id: int,
        gradient: float,
        input_ids: Sequence[int],
        feedback_ttl_ms: int,
        route_hops: int,
    ) -> list[float]:
        if not math.isfinite(gradient):
            raise DANMAError("non-finite PyTorch backward gradient")
        reply = self.request(
            {
                "kind": "backward",
                "target": checked_id(neuron_id, "neuron_id"),
                "event_id": checked_id(event_id, "event_id"),
                "from": {"kind": "teacher"},
                "gradient": float(gradient),
                "ttl_ms": feedback_ttl_ms,
                "gradient_hops": 2,
                "route_hops": route_hops,
            }
        )
        if reply.get("kind") != "backward_result" or reply.get("status") != "applied":
            raise DANMAError(
                f"DANMA backward not applied for EventID {event_id}: {reply}; "
                "earlier remote updates may already be committed"
            )
        entries = reply.get("unrouted")
        if not isinstance(entries, list) or len(entries) != len(input_ids):
            raise DANMAError(
                f"DANMA backward missing host-input gradients for EventID {event_id}; "
                "the remote weight update may already be committed"
            )
        expected_ids = set(input_ids)
        found: dict[int, float] = {}
        for item in entries:
            if not isinstance(item, dict):
                raise DANMAError("malformed DANMA host-input gradient")
            target = item.get("target")
            value = item.get("gradient")
            if (
                type(target) is not int
                or target not in expected_ids
                or target in found
                or item.get("reason") != "no_route"
                or item.get("event_id") != str(event_id)
                or type(value) not in (int, float)
                or not math.isfinite(value)
            ):
                raise DANMAError(
                    f"unexpected host-input gradient for EventID {event_id}: {item}"
                )
            found[target] = float(value)
        return [found[source] for source in input_ids]
