# danma-torch — PyTorch autograd bridge (CPU MVP)

This is the first usable PyTorch integration with the existing distributed
DANMA CPU runtime. It connects ordinary CPU float32 torch Tensors to remote,
addressable DANMA neurons through their existing TCP v1 protocol. It does
**not** register a PyTorch PrivateUse1 backend: torch.device("danma:0"),
tensor.to("danma"), torch.compile, CUDA/PTX, arbitrary torch operators,
torch.vmap and higher-order gradients are **not** implemented.

## Install and verify

Build the actual Rust TCP node:

    cargo build --locked -p danma-net --bin danma-node

Install CPU-only PyTorch and the editable Python package (or use an existing
compatible PyTorch CPU environment):

    python -m pip install --index-url https://download.pytorch.org/whl/cpu "torch==2.8.0"
    python -m pip install -e ./python --no-deps

Run the full end-to-end test against three *separate* real Rust processes:

    PYTHONPATH=python DANMA_NODE_BIN=target/debug/danma-node \
      python -m unittest discover -s python/tests -v

## Start a simple three-node layer

The first node owns neuron 11 with host inputs 901 and 902:

    cargo run -p danma-net --bin danma-node -- \
      --id 1 --listen 127.0.0.1:9101 \
      --neuron 11 --weight 901:2 --weight 902:3 \
      --peer 2@127.0.0.1:9102 --peer 3@127.0.0.1:9103

The second node owns neuron 21:

    cargo run -p danma-net --bin danma-node -- \
      --id 2 --listen 127.0.0.1:9102 \
      --neuron 21 --weight 901:-1 --weight 902:4 \
      --peer 1@127.0.0.1:9101 --peer 3@127.0.0.1:9103

The third node owns neuron 31:

    cargo run -p danma-net --bin danma-node -- \
      --id 3 --listen 127.0.0.1:9103 \
      --neuron 31 --weight 901:0.5 --weight 902:-2 \
      --peer 1@127.0.0.1:9101 --peer 2@127.0.0.1:9102

After gossip converges, train from ordinary PyTorch:

    import torch
    from danma_torch import DANMAClient, DANMALinear

    client = DANMAClient("127.0.0.1", 9101)
    layer = DANMALinear(
        client, neuron_ids=(11, 21, 31), input_ids=(901, 902)
    )
    x = torch.tensor([1.0, 2.0], dtype=torch.float32)
    y = layer(x)              # CPU torch.Tensor; remote forward
    loss = y.square().sum()
    loss.backward()           # remote DANMA updates + local autograd
    print(client.inspect(11)["version"])   # 1
    layer.eval()
    with torch.no_grad():
        print(layer(x))       # inference without remote training context

You can compose DANMALinear with ordinary torch.nn.Linear: autograd forwards
dLoss/dInput through the DANMA layer, and the regular PyTorch module receives
its ordinary Parameter gradients.

## Training contract

* A neuron is an output feature; all its input synapses must already exist in
  danma-node. The current CLI initializes each neuron's bias to zero and uses
  linear activation and local SGD with learning rate 0.1.
* input_ids are reserved, **unowned** host input identities. They must never
  be advertised as actual neurons in the current cluster. The backward
  response returns each unrouteable input gradient with reason "no_route".
* A forward activation gets a fresh random u64 EventID and one training
  expectation per remote neuron (teacher feedback). The backward step passes
  the PyTorch gradient to each output neuron. Each neuron applies one local
  update; the adapter gathers and sums its upstream contributions into
  dLoss/dInput.
* A nonpersistent autograd-trigger buffer lets remote training occur even
  when x.requires_grad=False. The remote weights are not nn.Parameters:
  layer.parameters() has no trainable synapse weights. Do not expect
  torch.optim.Optimizer.step(), state_dict() or model.to("danma") to update,
  serialize or move remote weights.
* Each differentiable forward supports one backward. Running backward twice
  on the same retained graph is rejected. Failed/expired gradients raise
  DANMAError; an earlier neuron may already have committed its local update.
* Only CPU float32 dense inputs of shape [in_features] or
  [batch, in_features] are supported; the default batch limit is deliberately
  small because the current Rust neuron accepts at most eight intervening
  weight versions before treating an activation as stale.
* Use model.train() and normal grad mode to retain remote forward traces.
  Use model.eval() together with torch.no_grad() for inference. Calling
  backward on inference output is unsupported.

## Limitations before PrivateUse1

This is a CPU Tensor with a custom Python autograd.Function, **not** a true
native DANMA storage/allocator or PyTorch device. The adapter currently sends
one synchronous TCP request per output activation and per output gradient,
so its throughput is limited by request latency and JSON framing. A future
shard/tensor kernel and batch protocol are prerequisites for meaningful CPU
performance comparisons with torch.nn.Linear. PyTorch gradcheck, vmap,
double-backward and torch.compile are unsupported because local remote SGD
is stateful and does not implement a pure functional gradient rule.

The wire uses u64 EventIDs and a trusted loopback-only network without
authenticated teacher messages, persistent deduplication, transactional
outbox/checkpoint, cross-host clock control or fault recovery. A timeout
does not roll back an already applied remote update. Do not deploy v1 over
untrusted networks or treat its state_dict as a checkpoint of the remote model.
