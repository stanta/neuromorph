# DANMA + Ray: CPU shard control plane (local v1)

This package adds Ray **process placement and lifecycle management** to the existing
Rust DANMA runtime. Ray launches **one actor per multi-neuron shard**, not one
actor per neuron, and is not in the signal/gradient hot path.

## Existing history preserved

- danma-core: local single-writer neuron, activation EventID, exact branch
  dedup, TTL and local backward.
- danma-shard: bounded CPU worker threads, neuron ownership and mailboxes.
- danma-net: three-process localhost TCP/Gossip v1, direct addressed data
  plane and routed inspection.
- PyTorch autograd bridge: already developed separately on the
  feat/danma-pytorch-autograd-adapter branch. This Ray package does not
  duplicate or replace it. A true PrivateUse1 DANMA device is not implemented.

## Critical scope and safety constraints

The Rust TCP v1 listener **only accepts loopback peers** and uses a static
bootstrap allowlist, which is not cryptographic authentication. All Ray
shard actors must therefore be pinned to **one physical host** and the
driver must run on that same host; do not use remote Ray Client to call
start_local_cluster. This can use Ray scheduling but is **not** a secure
three-host DANMA deployment. Do not expose TCP, Ray Dashboard, Job API,
or Ray Client endpoints to an untrusted network.

Ray actor restart and actor task retries are both disabled: there is no
crash-safe durable DANMA state, dedup journal or transactional outbox yet.
A crash of a Ray worker can leave a child Rust node running; inspect and
stop orphan processes/ports before attempting manual recovery. Ray cannot
make a committed neuron update rollback on a timeout.

## Install and run

From the repository root, on a trusted local CPU host:

~~~bash
cargo build --locked -p danma-net --bin danma-node
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -e ./ray
PYTHONPATH=ray python -m unittest discover -s ray/tests -v
~~~

For the Ray/Rust end-to-end test on the same host:

~~~bash
PYTHONPATH=ray DANMA_NODE_BIN=target/debug/danma-node DANMA_RAY_E2E=1 \
  python -m unittest discover -s ray/tests -v
~~~

The standalone unit tests do not require a running Ray cluster. The E2E
test starts Ray locally, places three actors on the driver's Ray node,
launches three actual Rust TCP processes, verifies gossip convergence,
then trains the three-neuron graph and checks backward deduplication.

## Python API

~~~python
from danma_ray import ClusterSpec, DanmaNode, NeuronSpec, start_local_cluster
import ray

ray.init(num_cpus=3, include_dashboard=False)
spec = ClusterSpec((
    DanmaNode(1, 9101, (NeuronSpec(1, ((99, 2.0),)),)),
    DanmaNode(2, 9102, (NeuronSpec(2, ((1, 3.0),)),)),
    DanmaNode(3, 9103, (NeuronSpec(3, ((2, 4.0),)),)),
))
try:
    with start_local_cluster(spec, binary="target/debug/danma-node") as cluster:
        # Inference/training use DANMA's existing direct localhost TCP
        # endpoint (or the existing DANMAClient from the PyTorch branch).
        from danma_ray import tcp_request
        print(tcp_request(cluster.entry_port, {"kind": "routes"}))
finally:
    ray.shutdown()
~~~

With the PyTorch adapter branch checked out or merged, construct its
DANMAClient("127.0.0.1", cluster.entry_port) and DANMALinear normally.
Ray manages the node processes; DANMAClient still talks directly to
danma-net rather than invoking a Ray Actor for each activation.

## Next milestone: real multi-host Ray

Introduce an authenticated cross-host transport with TLS and stable
node/shard identities, versioned owner epochs and fencing, explicit
backpressure and bounded batches. Add atomic checkpoint + dedup journal
and outbox before configuring actor restarts. Only then allow actors
to spread across Ray hosts and benchmark Ray-actor batching versus direct
Rust transport. Do not weaken the v1 loopback check as a shortcut.
