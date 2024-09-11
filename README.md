# Distributed Neuromorphic System (WebSocket-based)

This project implements a distributed neural network where "neurons" act as WebSocket servers and "dendrites" act as WebSocket clients. Neurons process incoming data, compute activations, and handle backpropagation using `tokio`, `tungstenite`, and `async` Rust features.

## Features

- Asynchronous WebSocket servers (neurons) and clients (dendrites).
- Dynamic IPv6 address assignment.
- Real-time backpropagation and neuron replacement.
- Mutex locks for thread safety.

## Functionality Overview
- Neuron: Acts as a WebSocket server, listens for incoming signals from dendrites, computes forward propagation using sigmoid activation, and sends feedback.
- Dendrite: Connects to neuron servers and sends signals. Monitors output and participates in backpropagation.
- Error Calculation & Replacement: Replaces the worst-performing dendrites periodically, simulating network pruning.

## Requirements

- Rust (latest version).

## How to Run
- Clone the repository:
git clone [https://github.com/your-username/repo-name.git](https://github.com/stanta/neuromorph.git)
- cargo build 
- cargo run 

