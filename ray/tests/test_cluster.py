import math
import unittest
from danma_ray.cluster import ClusterSpec, DanmaNode, NeuronSpec, _command, tcp_request


def three_nodes(ports=(9101, 9102, 9103)):
    return ClusterSpec((
        DanmaNode(1, ports[0], (NeuronSpec(1, ((99, 2.0),)),)),
        DanmaNode(2, ports[1], (NeuronSpec(2, ((1, 3.0),)),)),
        DanmaNode(3, ports[2], (NeuronSpec(3, ((2, 4.0),)),)),
    ))


class SpecTests(unittest.TestCase):
    def test_valid_and_command(self):
        spec = three_nodes()
        spec.validate()
        command = _command("/bin/danma-node", spec.nodes[0], spec.nodes)
        self.assertIn("2@127.0.0.1:9102", command)
        self.assertIn("99:2.0", command)
        self.assertEqual(command[:3], ["/bin/danma-node", "--id", "1"])

    def test_duplicate_owner(self):
        nodes = list(three_nodes().nodes)
        nodes[1] = DanmaNode(2, 9102, (NeuronSpec(1, ((99, 1.0),)),))
        with self.assertRaisesRegex(ValueError, "two owners"):
            ClusterSpec(tuple(nodes)).validate()

    def test_invalid_weights_and_ports(self):
        with self.assertRaises(ValueError):
            ClusterSpec((DanmaNode(1, 9101, (NeuronSpec(1, ((99, math.inf),)),)),)).validate()
        with self.assertRaises(ValueError):
            ClusterSpec((DanmaNode(1, 0, (NeuronSpec(1, ((99, 1.0),)),)),)).validate()

    def test_no_data_plane_ray_import(self):
        with self.assertRaises(ValueError):
            tcp_request(0, {"kind": "routes"})
        with self.assertRaises(ValueError):
            tcp_request(9101, {"kind": "forward", "value": float("nan")})


if __name__ == "__main__":
    unittest.main()
