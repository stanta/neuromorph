"""Autograd bridge for a remote DANMA layer whose weights live on CPU shards.

The output is a regular torch CPU Tensor with a custom grad_fn. This is
deliberately NOT a PrivateUse1 tensor, CUDA emulator, torch.compile operator,
or replacement for arbitrary torch.nn.Linear layers.
"""

from __future__ import annotations

import math
import secrets
from collections.abc import Sequence

import torch

from .client import DANMAClient, DANMAError, checked_id


def _event_id() -> int:
    """Random nonzero v1 wire ID. Collisions are improbable, not impossible."""
    return secrets.randbits(64) or 1


class _RemoteAffine(torch.autograd.Function):
    @staticmethod
    def forward(
        ctx: object,
        inputs: torch.Tensor,
        autograd_token: torch.Tensor,
        client: DANMAClient,
        neuron_ids: tuple[int, ...],
        input_ids: tuple[int, ...],
        feedback_ttl_ms: int,
        route_hops: int,
        training: bool,
    ) -> torch.Tensor:
        # Torch invokes Function.forward with autograd tracking disabled,
        # so the caller passes the actual training/inference decision.
        vector_input = inputs.ndim == 1
        rows: list[list[float]] = (
            [inputs.detach().tolist()] if vector_input else inputs.detach().tolist()
        )
        trace_id = _event_id()
        event_ids: list[list[int]] = []
        result_rows: list[list[float]] = []

        for row in rows:
            outputs: list[float] = []
            events: list[int] = []
            for neuron_id in neuron_ids:
                event_id = _event_id()
                value = client.forward(
                    neuron_id=neuron_id,
                    event_id=event_id,
                    trace_id=trace_id,
                    inputs=list(zip(input_ids, row)),
                    training=training,
                    route_hops=route_hops,
                )
                outputs.append(value)
                events.append(event_id)
            event_ids.append(events)
            result_rows.append(outputs)

        ctx.client = client
        ctx.neuron_ids = neuron_ids
        ctx.input_ids = input_ids
        ctx.event_ids = event_ids
        ctx.training = training
        ctx.feedback_ttl_ms = feedback_ttl_ms
        ctx.route_hops = route_hops
        ctx.vector_input = vector_input
        ctx.consumed = False

        output = torch.tensor(result_rows, dtype=inputs.dtype, device=inputs.device)
        return output[0] if vector_input else output

    @staticmethod
    def backward(ctx: object, grad_output: torch.Tensor) -> tuple:
        if ctx.consumed:
            raise DANMAError(
                "DANMA EventID already trained by this autograd graph; "
                "a second backward pass cannot update the same activation"
            )
        if not ctx.training:
            raise DANMAError(
                "DANMA inference forward has no saved activation trace; "
                "call module.train() before a differentiable forward"
            )
        expected_shape = (
            (len(ctx.neuron_ids),)
            if ctx.vector_input
            else (len(ctx.event_ids), len(ctx.neuron_ids))
        )
        if tuple(grad_output.shape) != expected_shape:
            raise DANMAError("PyTorch backward gradient shape does not match DANMA output")
        if grad_output.device.type != "cpu" or grad_output.dtype != torch.float32:
            raise DANMAError("DANMA backward only supports CPU float32 gradients")
        if not bool(torch.isfinite(grad_output).all().item()):
            raise DANMAError("DANMA backward received a non-finite gradient")

        # A partially executed remote backward cannot be rolled back on a
        # transport error. Do not attempt an automatic replay of this graph.
        ctx.consumed = True
        gradient_rows = (
            [grad_output.detach().tolist()]
            if ctx.vector_input
            else grad_output.detach().tolist()
        )
        input_gradients: list[list[float]] = []

        for events, gradients in zip(ctx.event_ids, gradient_rows):
            accumulated = [0.0] * len(ctx.input_ids)
            for neuron_id, event_id, gradient in zip(
                ctx.neuron_ids, events, gradients
            ):
                contribution = ctx.client.backward(
                    neuron_id=neuron_id,
                    event_id=event_id,
                    gradient=float(gradient),
                    input_ids=ctx.input_ids,
                    feedback_ttl_ms=ctx.feedback_ttl_ms,
                    route_hops=ctx.route_hops,
                )
                for index, value in enumerate(contribution):
                    accumulated[index] += value
            if not all(math.isfinite(value) for value in accumulated):
                raise DANMAError("accumulated DANMA input gradient is not finite")
            input_gradients.append(accumulated)

        result = torch.tensor(input_gradients, dtype=torch.float32, device="cpu")
        if ctx.vector_input:
            result = result[0]
        # Differentiable inputs: x, token. Remote neuron parameters are
        # intentionally NOT torch.nn.Parameter objects.
        return result, None, None, None, None, None, None, None


class DANMALinear(torch.nn.Module):
    """A remotely trained affine layer backed by DANMA neuron IDs.

    Weight and bias state belong exclusively to DANMA. PyTorch's backward
    sends teacher gradients and yields dLoss/dInput for composition with
    ordinary PyTorch modules; local optimizers cannot manage remote weights.

    Requires one DANMA neuron per output feature. Every neuron must have an
    incoming synapse for every input_id, configured on the Rust node. Input
    IDs must be unused by the cluster so that upstream gradients return
    as explicit no_route entries. The current Rust CLI initializes bias=0.
    """

    def __init__(
        self,
        client: DANMAClient,
        *,
        neuron_ids: Sequence[int],
        input_ids: Sequence[int],
        feedback_ttl_ms: int = 3_000,
        route_hops: int = 4,
        max_batch: int = 32,
    ) -> None:
        super().__init__()
        if not isinstance(client, DANMAClient):
            raise TypeError("client must be a DANMAClient")
        self.neuron_ids = tuple(checked_id(value, "neuron_id") for value in neuron_ids)
        self.input_ids = tuple(checked_id(value, "input_id") for value in input_ids)
        if not self.neuron_ids or not self.input_ids:
            raise ValueError("neuron_ids and input_ids must be nonempty")
        if len(set(self.neuron_ids)) != len(self.neuron_ids):
            raise ValueError("neuron_ids must be unique")
        if len(set(self.input_ids)) != len(self.input_ids):
            raise ValueError("input_ids must be unique")
        if set(self.neuron_ids) & set(self.input_ids):
            raise ValueError("input_ids and neuron_ids must be disjoint")
        if len(self.neuron_ids) > 128 or len(self.input_ids) > 128:
            raise ValueError("v1 supports at most 128 input and output features")
        if type(feedback_ttl_ms) is not int or not 1 <= feedback_ttl_ms <= 10_000:
            raise ValueError("feedback_ttl_ms must be 1..10000")
        if type(route_hops) is not int or not 1 <= route_hops <= 255:
            raise ValueError("route_hops must be 1..255")
        if type(max_batch) is not int or not 1 <= max_batch <= 32:
            raise ValueError("max_batch must be 1..32")

        self.client = client
        self.feedback_ttl_ms = feedback_ttl_ms
        self.route_hops = route_hops
        self.max_batch = max_batch

        # This nonpersistent leaf buffer is an autograd trigger even when x
        # has requires_grad=False: the actual trainable weights live remotely.
        # It is intentionally not an nn.Parameter/optimizer-owned weight.
        self.register_buffer(
            "_autograd_trigger",
            torch.zeros((), dtype=torch.float32, requires_grad=True),
            persistent=False,
        )

    def forward(self, inputs: torch.Tensor) -> torch.Tensor:
        if not isinstance(inputs, torch.Tensor):
            raise TypeError("DANMALinear requires a torch.Tensor")
        if inputs.device.type != "cpu" or self._autograd_trigger.device.type != "cpu":
            raise ValueError("DANMALinear only supports CPU tensors, not GPU tensors")
        if inputs.dtype != torch.float32:
            raise ValueError("DANMALinear requires float32 inputs")
        if inputs.layout != torch.strided:
            raise ValueError("DANMALinear requires a dense strided input")
        if inputs.ndim not in (1, 2):
            raise ValueError("DANMALinear requires rank 1 or rank 2 input")
        if inputs.shape[-1] != len(self.input_ids):
            raise ValueError("DANMALinear input feature dimension mismatch")
        if inputs.ndim == 2 and not 1 <= inputs.shape[0] <= self.max_batch:
            raise ValueError("DANMALinear batch is empty or exceeds max_batch")
        if not bool(torch.isfinite(inputs).all().item()):
            raise ValueError("DANMALinear requires finite input values")
        train_remote = bool(self.training and torch.is_grad_enabled())
        token = (
            self._autograd_trigger
            if train_remote
            else self._autograd_trigger.detach()
        )
        return _RemoteAffine.apply(
            inputs,
            token,
            self.client,
            self.neuron_ids,
            self.input_ids,
            self.feedback_ttl_ms,
            self.route_hops,
            train_remote,
        )

    def extra_repr(self) -> str:
        return (
            f"in_features={len(self.input_ids)}, out_features={len(self.neuron_ids)}, "
            f"remote=True, feedback_ttl_ms={self.feedback_ttl_ms}"
        )
