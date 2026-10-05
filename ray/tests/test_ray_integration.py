import importlib.util
import os
from pathlib import Path
import socket
import unittest
from danma_ray import ClusterSpec, DanmaNode, NeuronSpec, start_local_cluster, tcp_request


def free_ports(count):
    sockets = [socket.socket() for _ in range(count)]
    try:
        for sock in sockets:
            sock.bind(("127.0.0.1", 0))
        return [sock.getsockname()[1] for sock in sockets]
    finally:
        for sock in sockets:
            sock.close()


@unittest.skipUnless(os.environ.get("DANMA_RAY_E2E") == "1" and importlib.util.find_spec("ray"),
                     "install ray, build Rust binary, set DANMA_RAY_E2E=1")
class RayE2E(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import ray
        cls.ray = ray
        cls.binary = Path(os.environ["DANMA_NODE_BIN"]).resolve(strict=True)
        ray.init(num_cpus=3, include_dashboard=False, ignore_reinit_error=False)

    @classmethod
    def tearDownClass(cls):
        cls.ray.shutdown()

    def test_three_ray_actors_one_real_rust_graph(self):
        ports = free_ports(3)
        spec = ClusterSpec((
            DanmaNode(1, ports[0], (NeuronSpec(1, ((99, 2.0),)),)),
            DanmaNode(2, ports[1], (NeuronSpec(2, ((1, 3.0),)),)),
            DanmaNode(3, ports[2], (NeuronSpec(3, ((2, 4.0),)),)),
        ))
        with start_local_cluster(spec, binary=self.binary) as cluster:
            self.assertEqual(len(cluster.actors), 3)
            client = lambda message: tcp_request(cluster.entry_port, message)
            a = client({"kind": "forward", "target": 1, "event_id": 10, "trace_id": 7,
                        "route_hops": 4, "inputs": [{"from": 99, "source_event_id": 1, "value": 1.0}],
                        "expected": [{"kind": "neuron", "neuron_id": 2, "event_id": 20}]})
            self.assertEqual(a["output"], 2.0)
            b = client({"kind": "forward", "target": 2, "event_id": 20, "trace_id": 7,
                        "route_hops": 4, "inputs": [{"from": 1, "source_event_id": 10, "value": 2.0}],
                        "expected": [{"kind": "neuron", "neuron_id": 3, "event_id": 30}]})
            self.assertEqual(b["output"], 6.0)
            c = client({"kind": "forward", "target": 3, "event_id": 30, "trace_id": 7,
                        "route_hops": 4, "inputs": [{"from": 2, "source_event_id": 20, "value": 6.0}],
                        "expected": [{"kind": "teacher"}]})
            self.assertEqual(c["output"], 24.0)
            feedback = {"kind": "backward", "target": 3, "event_id": 30,
                        "from": {"kind": "teacher"}, "gradient": 24.0,
                        "ttl_ms": 3000, "gradient_hops": 5, "route_hops": 4}
            self.assertEqual(client(feedback)["status"], "applied")
            self.assertEqual(client(feedback)["status"], "ignored_duplicate")
            for neuron_id in (1, 2, 3):
                reply = client({"kind": "inspect", "target": neuron_id, "route_hops": 4})
                self.assertEqual(reply["version"], 1)


if __name__ == "__main__":
    unittest.main()
