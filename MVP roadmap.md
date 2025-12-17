# MVP roadmap

## 1. CUDA Integration Plan
Phase 1: Infrastructure & Setup
 Add Dependency: Update root Cargo.toml to include neuromorph-driver in the [dependencies] section.
Definition of Done: cargo build passes and neuromorph-driver is accessible in src/main.rs.
 Initialize Driver: Add neuromorph_driver::init() call at the start of main() in src/main.rs.
Definition of Done: Application starts successfully without panicking on initialization.
Phase 2: Memory Management Integration
 Refactor Neuron Struct: Modify Neuron struct to hold DeviceMemory handles for weights and bias.
Definition of Done: Neuron struct compiles with neuromorph_driver::DeviceMemory fields replacing or augmenting Vec<i32>.
 Implement Data Transfer: Create helper methods in Neuron to copy weights/bias to device upon creation (new).
Definition of Done: Unit test confirms data is correctly copied to device memory (verified by copying back and comparing).
Phase 3: Kernel Execution Integration
 Create Dummy Kernel: Create a dummy byte array representing a compiled kernel to be loaded by Kernel::from_bytes.
Definition of Done: Kernel::from_bytes returns Ok with the dummy data.
 Implement Forward Pass on Device: Rewrite Neuron::forward to:
Allocate device memory for input.
Copy input data from host to device.
Launch the kernel using Kernel::launch.
Copy the result back to host.
Definition of Done: forward method compiles and runs using neuromorph-driver APIs without errors.
Phase 4: Integration & Verification
 Update WebSocket Handler: Ensure handle_client calls the new CUDA-backed forward method.
Definition of Done: WebSocket server responds to input (e.g., "1,2,3") with a result calculated via the driver.
 Integration Test: Create a test in tests/ that starts the server, connects via WebSocket, sends data, and asserts a valid response is received.
Definition of Done: cargo test passes the new integration test.

## 2. NN frameworks integration 
Phase 1: Computational Engine (Data Plane)
Goal: Enable the simulator to actually execute mathematical operations, moving beyond "mock" execution.

1.1 Define Tensor Metadata Structure

Task: Modify neuromorphMalloc in simulator.rs to allocate a structured Tensor object (containing shape, strides, data type, and data pointer) instead of just raw bytes.
Definition of Done:
neuromorphMalloc allocates a structure that tracks dimensions (e.g., [3, 3]) and type (e.g., f32).
neuromorphMemcpy validates that the source/destination sizes match the allocated tensor's capacity.
Unit tests confirm metadata is preserved across allocations.
1.2 Implement Kernel Registry System

Task: Replace the opaque Vec<u8> kernel blob with a registry of built-in Rust functions. Create an enum or ID system to dispatch kernels by name/ID.
Definition of Done:
A KernelRegistry struct exists in simulator.rs.
neuromorphLaunchKernel accepts a Kernel ID, looks up the corresponding Rust function, and executes it.
Unknown Kernel IDs return a specific error code.
1.3 Implement Dense Matrix Multiplication (GEMM) Kernel

Task: Implement a Rust function that performs C = alpha _ (A @ B) + beta _ C and register it in the kernel registry.
Definition of Done:
A unit test allocates three tensors (A, B, C) on the "device".
neuromorphLaunchKernel is called with the GEMM kernel ID.
The result in C matches the expected matrix multiplication result (verified against a CPU reference).
1.4 Implement Element-wise Operations (Add, Mul, ReLU)

Task: Implement kernels for basic arithmetic and activation functions.
Definition of Done:
Kernels for Add, Mul, and ReLU are registered.
Unit tests verify correctness for each operation on random input data.
Phase 2: Training Primitives (Autograd Support)
Goal: Enable the calculation of gradients required for backpropagation.

2.1 Implement Backward Pass for GEMM (MatMulGradient)

Task: Implement the gradient computation for Matrix Multiplication: given dC (gradient of output), compute dA = dC @ B^T and dB = A^T @ dC.
Definition of Done:
A MatMulBackward kernel is registered.
Gradient checks (finite difference comparison) confirm the kernel computes correct gradients for inputs A and B.
2.2 Implement Backward Pass for Element-wise Ops

Task: Implement derivative kernels for ReLU (step function), Add (pass-through), and Mul.
Definition of Done:
Backward kernels are registered for all Phase 1 operators.
Unit tests verify that gradients propagate correctly through a chain of operations (e.g., ReLU(Add(A, B))).
2.3 Implement SGD Optimizer Kernel

Task: Implement a kernel that updates weights in-place: param -= learning_rate \* grad.
Definition of Done:
An SGD kernel is registered.
A test case shows a tensor's values changing in the direction of the negative gradient after execution.
2.4 Implement Random Number Generation (RNG) Kernel

Task: Implement a kernel to fill a tensor with random values (uniform/normal distribution) for weight initialization.
Definition of Done:
An RNG kernel is registered.
Calling it multiple times produces different sequences (unless seeded deterministically).
Statistical tests (mean/variance) confirm the distribution matches the request.
Phase 3: Python & PyTorch Integration
Goal: Connect the Rust backend to the Python ecosystem.

3.1 Create neuromorph-python PyO3 Bindings

Task: Create a new crate that exposes neuromorph-driver functionality to Python using PyO3.
Definition of Done:
pip install . works.
Python script can import neuromorph, allocate memory, and launch a dummy kernel.
3.2 Implement PyTorch PrivateUse1 Dispatch Key

Task: Create a C++ extension (using torch::extension) that registers neuromorph as a device using the PrivateUse1 key.
Definition of Done:
torch.tensor([1, 2], device='privateuse1') (or mapped name) does not crash.
The tensor is reported as being on the custom device.
3.3 Register Allocator with PyTorch (c10::Allocator)

Task: Map neuromorphMalloc/neuromorphFree to PyTorch's allocator interface.
Definition of Done:
PyTorch can allocate and free memory on the neuromorph device without memory leaks.
torch.cuda.memory_allocated() (or equivalent for custom device) reports correct usage.
3.4 Register Core Operators (aten::mm, aten::add)

Task: Register the Rust kernels from Phase 1 to PyTorch's dispatcher for specific ATen operators.
Definition of Done:
c = torch.mm(a, b) runs on the simulator when a and b are on the neuromorph device.
Standard PyTorch training loop (Forward -> Loss -> Backward -> Step) runs without error on a simple MLP.
