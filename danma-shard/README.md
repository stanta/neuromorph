# DANMA multi-neuron CPU shard

The danma-shard crate separates logical neuron addresses from OS processes and
threads. One shard owns multiple danma-core Neuron values. A **fixed number of
CPU workers** each owns a partition of the neuron map, and each has a bounded
mailbox. A neuron never creates an OS thread, Tokio task, socket or simulated
CUDA stream.

## Contract

- Stable routing from NeuronId to worker index inside one owning process
  (prototype: NeuronId modulo worker count).
- Each neuron has one mutator. Forward, backward, inspect and trace requests
  for the same neuron execute in the same FIFO worker mailbox.
- The Tokio I/O runtime only queues operations and awaits typed responses.
  Heavy neuron calculations take place on dedicated CPU threads.
- Mailboxes are bounded; requests rejected when full return ShardError::Busy.
  This is overload signalling, not evidence that an already accepted request
  was cancelled.
- Invalid topology (empty shard, duplicate neuron IDs), worker counts and
  mailbox sizes are rejected before starting work.
- The existing danma-core EventID ledger, branch deduplication, weight
  snapshots, feedback TTL and version checks remain owned by each neuron.
- The shard API can return per-neuron state and active activation traces to
  the routed network observer.

## Run

    cargo test --locked -p danma-shard --all-targets

The tests exercise multiple neurons on fewer CPU workers and local backward
propagation across worker mailboxes.

## Current limitations

This is a reference correctness-oriented shard, not a measured billion-neuron
engine. Neurons still contain BTreeMap weights; there is no CSR/SoA storage,
vectorized kernel, dynamic resharding, checkpointing, durable inbox/outbox or
persistent replay ledger. Fixed worker ownership and queue capacity are chosen
at startup. An async caller may stop waiting after its command was accepted,
while the worker can still finish the side effect. A durable command ID and
journal are needed for crash-safe end-to-end training effects.
