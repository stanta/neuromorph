"""CPU tensor/autograd bridge to a distributed DANMA neuron cluster.

This package does not register a PrivateUse1 device and does not make
torch.device("danma:0") available. Remote neuron weights are owned by DANMA.
"""
from .client import DANMAClient, DANMAError, DANMATransportError
from .layer import DANMALinear

__all__ = ["DANMAClient", "DANMAError", "DANMATransportError", "DANMALinear"]
