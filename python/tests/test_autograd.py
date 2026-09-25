"""Real three-process integration for the PyTorch ↔ DANMA autograd bridge.

Each test uses a fresh localhost cluster to avoid mutable neuron weights
leaking between cases. No fake tensor kernels or simulated DANMA responses.
"""

from __future__ import annotations

import os
import socket
import subprocess
import time
import unittest
from pathlib import Path

import torch
from torch.nn import functional as F

from danma_torch import DANMAClient, DANMAError, DANMALinear


ROOT = Path(__file__).resolve().parents[2]
NODE_BINARY = Path(os.environ.get("DANMA_NODE_BIN", ROOT / "target/debug/danma-node"))


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


class Cluster:
    """Three physical nodes with independent, fixed synapse states."""

    def __init__(self) -> None:
        ports: list[int] = []
        while len(ports) < 3:
            candidate = free_port()
            if candidate not in ports:
                ports.append(candidate)
        self.ports = ports
        self.processes: list[subprocess.Popen[bytes]] = []
        layouts = (
            (11, ("901:2", "902:3")),
            (21, ("901:-1", "902:4")),
            (31, ("901:0.5", "902:-2")),
        )
        for index, (neuron, weights) in enumerate(layouts):
            argv = [
                str(NODE_BINARY),
                "--id", str(index + 1),
                "--listen", f"127.0.0.1:{ports[index]}",
                "--neuron", str(neuron),
            ]
            for weight in weights:
                argv.extend(("--weight", weight))
            for peer_index, port in enumerate(ports):
                if peer_index != index:
                    argv.extend(("--peer", f"{peer_index + 1}@127.0.0.1:{port}"))
            self.processes.append(
                subprocess.Popen(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            )
        self.client = DANMAClient("127.0.0.1", ports[0])
        until = time.monotonic() + 10.0
        while time.monotonic() < until:
            if any(p.poll() is not None for p in self.processes):
                raise RuntimeError("DANMA node exited before route convergence")
            try:
                routes = self.client.routes()
                if all(routes.get(str(neuron)) == owner for neuron, owner in (
                    (11, 1), (21, 2), (31, 3)
                )):
                    return
            except (OSError, DANMAError):
                pass
            time.sleep(0.05)
        self.stop()
        raise TimeoutError("three-node DANMA cluster did not converge")

    def stop(self) -> None:
        for proc in self.processes:
            if proc.poll() is None:
                proc.terminate()
        for proc in self.processes:
            try:
                proc.wait(timeout=2)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=2)


class PyTorchIntegrationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.cluster = Cluster()
        self.client = self.cluster.client
        self.model = DANMALinear(
            self.client,
            neuron_ids=(11, 21, 31),
            input_ids=(901, 902),
            feedback_ttl_ms=3_000,
        )

    def tearDown(self) -> None:
        self.cluster.stop()

    def inspect(self, neuron: int) -> dict:
        return self.client.inspect(neuron)

    def test_batch_forward_and_input_gradient_match_torch_reference(self) -> None:
        inputs = torch.tensor([[1.0, 2.0], [-2.0, 1.0]], dtype=torch.float32, requires_grad=True)
        expected_inputs = inputs.detach().clone().requires_grad_(True)
        weights = torch.tensor([[2.0, 3.0], [-1.0, 4.0], [0.5, -2.0]])
        expected = F.linear(expected_inputs, weights)
        expected.sum().backward()

        actual = self.model(inputs)
        self.assertEqual(tuple(actual.shape), (2, 3))
        torch.testing.assert_close(actual, expected.detach(), atol=1e-6, rtol=0)
        actual.sum().backward()
        torch.testing.assert_close(inputs.grad, expected_inputs.grad, atol=1e-6, rtol=0)
        for neuron in (11, 21, 31):
            self.assertEqual(self.inspect(neuron)["version"], 2)

    def test_remote_neurons_train_when_input_does_not_require_grad(self) -> None:
        inputs = torch.tensor([1.0, 2.0])
        result = self.model(inputs)
        self.assertTrue(result.requires_grad, "remote SGD must run for ordinary training inputs")
        result.sum().backward()
        self.assertEqual(self.inspect(11)["version"], 1)
        self.assertEqual(self.inspect(21)["version"], 1)
        self.assertEqual(self.inspect(31)["version"], 1)
        self.assertEqual(float(self.inspect(11)["weights"]["901"]), 1.9)
        self.assertIsNone(inputs.grad)

    def test_gradients_propagate_through_an_ordinary_torch_linear_layer(self) -> None:
        inputs = torch.tensor([1.0, 2.0], requires_grad=True)
        head = torch.nn.Linear(3, 1, bias=False)
        with torch.no_grad():
            head.weight.copy_(torch.tensor([[2.0, -1.0, 0.5]]))

        activations = self.model(inputs)
        head(activations).sum().backward()
        torch.testing.assert_close(inputs.grad, torch.tensor([5.25, 1.0]), atol=1e-6, rtol=0)
        torch.testing.assert_close(
            head.weight.grad, torch.tensor([[8.0, 7.0, -3.5]]), atol=1e-6, rtol=0
        )
        for neuron in (11, 21, 31):
            self.assertEqual(self.inspect(neuron)["version"], 1)

    def test_nine_samples_fit_current_remote_staleness_limit(self) -> None:
        model = DANMALinear(
            self.client,
            neuron_ids=(11, 21, 31),
            input_ids=(901, 902),
            max_batch=9,
        )
        inputs = torch.ones((9, 2), dtype=torch.float32, requires_grad=True)
        output = model(inputs)
        output.sum().backward()
        torch.testing.assert_close(
            inputs.grad,
            torch.tensor([[1.5, 5.0]]).expand(9, 2),
            atol=1e-6,
            rtol=0,
        )
        for neuron in (11, 21, 31):
            self.assertEqual(self.inspect(neuron)["version"], 9)

    def test_non_finite_gradient_is_rejected_without_remote_side_effects(self) -> None:
        inputs = torch.tensor([1.0, 2.0], requires_grad=True)
        output = self.model(inputs)
        with self.assertRaisesRegex(DANMAError, "non-finite"):
            output.backward(torch.tensor([1.0, float("nan"), 1.0]))
        for neuron in (11, 21, 31):
            self.assertEqual(self.inspect(neuron)["version"], 0)

    def test_eval_no_grad_does_not_create_training_events_or_change_weights(self) -> None:
        self.model.eval()
        with torch.no_grad():
            result = self.model(torch.tensor([1.0, 2.0]))
        self.assertFalse(result.requires_grad)
        torch.testing.assert_close(result, torch.tensor([8.0, 7.0, -3.5]))
        for neuron in (11, 21, 31):
            self.assertEqual(self.inspect(neuron)["version"], 0)

    def test_a_saved_autograd_graph_cannot_train_the_same_event_twice(self) -> None:
        inputs = torch.tensor([1.0, 2.0], requires_grad=True)
        result = self.model(inputs)
        result.sum().backward(retain_graph=True)
        with self.assertRaisesRegex(DANMAError, "already"):
            result.sum().backward(retain_graph=True)
        for neuron in (11, 21, 31):
            self.assertEqual(self.inspect(neuron)["version"], 1)

    def test_unknown_remote_neuron_is_not_silently_evaluated_on_cpu(self) -> None:
        invalid_model = DANMALinear(self.client, neuron_ids=(999,), input_ids=(901, 902))
        with self.assertRaisesRegex(DANMAError, "route_unknown"):
            invalid_model(torch.tensor([1.0, 2.0]))
        self.assertEqual(self.inspect(11)["version"], 0)


class InputContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.client = DANMAClient("127.0.0.1", 9)
        self.model = DANMALinear(self.client, neuron_ids=(11,), input_ids=(901, 902))

    def test_rejects_wrong_dtype_shape_and_non_finite_input_before_network(self) -> None:
        with self.assertRaisesRegex((ValueError, TypeError), "float32"):
            self.model(torch.ones((2,), dtype=torch.float64))
        with self.assertRaisesRegex((ValueError, TypeError), "rank"):
            self.model(torch.ones((2, 2, 2), dtype=torch.float32))
        with self.assertRaisesRegex((ValueError, TypeError), "finite"):
            self.model(torch.tensor([1.0, float("nan")]))
        with self.assertRaisesRegex((ValueError, TypeError), "feature"):
            self.model(torch.ones((3,), dtype=torch.float32))

    def test_rejects_reused_and_overlapping_ids(self) -> None:
        with self.assertRaisesRegex(ValueError, "unique"):
            DANMALinear(self.client, neuron_ids=(11, 11), input_ids=(901, 902))
        with self.assertRaisesRegex(ValueError, "unique"):
            DANMALinear(self.client, neuron_ids=(11,), input_ids=(901, 901))
        with self.assertRaisesRegex(ValueError, "disjoint"):
            DANMALinear(self.client, neuron_ids=(11,), input_ids=(11, 902))

    def test_rejects_batch_above_current_remote_staleness_budget(self) -> None:
        # Current danma-node accepts at most 8 intervening parameter versions
        # for a stored forward trace. A single batch must not partially train
        # 9 samples then fail after earlier samples have committed.
        with self.assertRaisesRegex(ValueError, "staleness"):
            DANMALinear(
                self.client,
                neuron_ids=(11,),
                input_ids=(901, 902),
                max_batch=10,
            )

    def test_batch_limit_is_checked_before_network(self) -> None:
        model = DANMALinear(
            self.client,
            neuron_ids=(11,),
            input_ids=(901, 902),
            max_batch=2,
        )
        with self.assertRaisesRegex(ValueError, "batch"):
            model(torch.ones((3, 2), dtype=torch.float32))


if __name__ == "__main__":
    unittest.main()
