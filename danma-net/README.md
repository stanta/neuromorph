# DANMA TCP node — distributed CPU MVP slice

This is a **working three-process localhost prototype**, not a public P2P
network or a production device driver. It depends on the standalone
[danma-core](../danma-core/README.md) single-writer neuron state machine.
CUDA and GPU support are out of scope.

## Architecture

- Each OS process owns a **multi-neuron CPU shard**, served by one TCP
  listener and a fixed, bounded number of CPU workers. NeuronId selects a
  local worker mailbox; a neuron has no individual process, thread or socket.
  Different neurons on the same process propagate feedback locally.
- A small, static allowlist supplies the identities and addresses of bootstrap
  peers. Round-robin gossip (one peer per 100 ms) exchanges bounded neuron
  owner/epoch advertisements. Forward activations and backward feedback are
  sent by direct TCP to the discovered owner, rather than flooded by gossip.
- The protocol frames each JSON request with a 4-byte, big-endian frame length
  and caps the payload at 64 KiB. There is one request and one response per
  connection. The listener limits concurrent connections to 32; input fan-in,
  feedback fan-out and route table size are bounded separately.
- The wire protocol v1 uses u64 EventIDs; danma-core retains u128 identifiers.
  Migration to an unambiguous globally unique wire ID is required before
  large-scale dynamic network deployment.
- An activation specifies expected downstream contributions before dispatch.
  A neuron aggregates distinct contributions and applies one weight update;
  repeated delivery of an already-seen contribution does not update twice.
  Relative feedback TTL decreases across relays; gradient hops are separate
  from routing hops. Routes are neither globally consistent nor transactional.

## Run a local three-node cluster

From the repo root, in three terminals:

    cargo run -p danma-net --bin danma-node -- \
      --id 1 --listen 127.0.0.1:9101 --neuron 1 --weight 99:2 \
      --peer 2@127.0.0.1:9102 --peer 3@127.0.0.1:9103

    cargo run -p danma-net --bin danma-node -- \
      --id 2 --listen 127.0.0.1:9102 --neuron 2 --weight 1:3 \
      --peer 1@127.0.0.1:9101 --peer 3@127.0.0.1:9103

    cargo run -p danma-net --bin danma-node -- \
      --id 3 --listen 127.0.0.1:9103 --neuron 3 --weight 2:4 \
      --peer 1@127.0.0.1:9101 --peer 2@127.0.0.1:9102

The --neuron/--weight pair can be repeated for multiple neurons within one
process. For example, Node 1 can also host neuron 4 with a local input from
neuron 1:

    cargo run -p danma-net --bin danma-node -- \
      --id 1 --listen 127.0.0.1:9101 --workers 2 --mailbox 16 \
      --neuron 1 --weight 99:2 --neuron 4 --weight 1:1 \
      --peer 2@127.0.0.1:9102 --peer 3@127.0.0.1:9103

To inspect gossip convergence from another shell:

    python3 - <<'PY'
    import json, socket, struct
    request = json.dumps({"kind": "routes"}).encode()
    with socket.create_connection(("127.0.0.1", 9101), timeout=2) as s:
        s.sendall(struct.pack("!I", len(request)) + request)
        size = struct.unpack("!I", s.recv(4))[0]
        data = bytearray()
        while len(data) < size:
            part = s.recv(size - len(data))
            if not part:
                raise EOFError("incomplete DANMA frame")
            data.extend(part)
        print(json.loads(data))
    PY

A routed inspect request returns a neuron's current version, bias and
weights. During an active training activation, a routed trace request such as
{"kind":"trace","target":2,"event_id":50,"route_hops":4} returns the
TraceID, output, weight version, expiry and number of received/expected
feedback contributions. Completed traces are removed from active memory;
there is no durable per-activation history yet.

The integration tests launch three **separate child processes**, including
a six-neuron layout with two neurons and two CPU workers per process. They
exercise A→B→C forward, C→B→A backward, local and remote gradient hops,
retry/dedup, TTL expiry, invalid contributor, gossip convergence, remote
traces and malformed frames:

    cargo test --locked -p danma-core -p danma-shard -p danma-net --all-targets

## Known gaps and non-goals

**Security:** the entire v1 network is restricted to loopback addresses.
A static peer allowlist is not authentication: the incoming claimed node ID
is not cryptographically bound to the TCP source. No TLS, signed gossip,
Sybil resistance, or authorization of teachers is implemented. Do not expose
this prototype to untrusted networks or accept arbitrary user traffic.

**Delivery and crash recovery:** outbound feedback is sent only after the local
weight update. If the process crashes or the network fails in between, there
is no durable outbox; a timeout means the result may be unknown. The bounded
in-memory activation and dedup ledger does not survive restart. The response
returns unrouted/uncertain upstream gradients rather than silently dropping
them. A production version needs atomic journaling, retry/replay, epochs,
dedup persistence and versioned owner fencing.

**Topology:** gossip operates over a known three-peer mesh; there is no DHT,
dynamic admission, signed advertisement, liveness suspicion or route lease
expiry. Routing state is bounded but not guaranteed to be fresh. All nodes
must agree on their bootstrap peer identities and network addresses.

**Time and training:** each host currently derives the local core deadline
from wall-clock Unix milliseconds, while the transport carries a relative
remaining TTL. For independent remote hosts, clock-skew handling and a
monotonic per-node activation clock are required. Asynchronous training
convergence has not been established.

**Framework integration:** no PyTorch PrivateUse1 or TensorFlow PluggableDevice
is registered. A tensor/shard adapter and numeric correctness benchmarks
against existing framework operators are the next milestones.
