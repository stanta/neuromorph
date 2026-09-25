# DANMA CPU runtime (first vertical slice)

This is the CPU-first DANMA engine, intentionally independent of the CUDA-style
simulator already present in this repository. The Rust crate has no external
runtime dependencies. It is not a GPU emulator.

## Implemented

- One neuron is an addressable value owned by a single runtime actor, **not** an
  OS thread, WebSocket server or CUDA context.
- Each forward call computes a real weighted sum with Linear or ReLU activation.
  Training activations retain the exact input values and forward-time weights
  under their unique local EventID; inference retains no full trace.
- Each activation declares its expected downstream feedback branches. Feedback
  is identified by the downstream neuron and its own activation EventID.
  Repeated delivery of a branch contributes **once**; distinct branches
  accumulate, and the neuron applies **one** local update when all expected
  branches have arrived.
- A feedback packet has both a time deadline and a hop budget. Expired packets
  cannot learn. Activation traces and deduplication tombstones have bounded
  retention and share a maximum number of live event slots.
- Gradient propagation uses the weights snapshotted during the forward pass.
  A configurable weight-version staleness bound rejects old training contexts.
- The trace accessor exposes the activation's output, parameter version,
  expiry and feedback progress.

The core deliberately accepts caller-supplied time. The current prototype
assumes comparable millisecond clocks across nodes; a production transport
must translate wire TTL to a local monotonic deadline and account for clock
skew. A finite hop budget is independent of the time TTL.

## Run

From the repository root:

    cargo test -p danma-core --all-targets

The in-process three-neuron integration test illustrates propagation through
three independently owned neuron states. It does **not** open TCP connections
or establish a gossip overlay.

## Explicitly not implemented yet

1. Shared, bounded CPU-worker pool and CSR/SoA synapse storage for large shards.
2. Real cross-process transport, signed membership, gossip discovery, route
   caching, batch delivery, backpressure and node failure/recovery.
3. Durable atomic weight-update + dedup journaling and checkpoint/replay.
   The current once-per-EventID effect is in-memory, for one owning actor.
4. PyTorch PrivateUse1 and TensorFlow PluggableDevice adapters; this crate is
   the backend that those adapters will eventually call through a shared ABI.
5. Dynamic synapse replacement, recurrent-cycle semantics, arbitrary tensor
   kernels, convergence benchmarks and billion-neuron scaling.

## Invariants to preserve in the next slice

- The owning shard alone mutates a neuron's weights and local event ledger.
- Different downstream branches of one activation are valid independent
  contributions. A redelivery of the same downstream activation is not.
- Never train from an expired trace; never train a finalized event again while
  its replay window remains open.
- Authenticate feedback against the graph's expected edge/child activation
  before processing it on a distributed node.
- Gossip distributes membership/routing metadata; addressed data-plane
  delivery must have explicit retry/ack and bounded queues.
- The mathematical choice to aggregate gradients before one weight update is
  deliberate. Opportunistic incremental updates would be a different
  training algorithm and require separate tests.
