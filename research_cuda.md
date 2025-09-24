Перейти к содержимому
Журнал чата

Вы сказали:
Develop  cuda-style drivers to access Neuromorph from standard ML libraries (Pytorch, TensorFlow, etc)
https://github.com/stanta/neuromorph
ChatGPT сказал:
To help you best, could you clarify what you're aiming for with the CUDA-style drivers for Neuromorph?

Are you trying to allow Neuromorph to act as a backend accelerator (like a GPU) for PyTorch or TensorFlow models?

Do you need a full runtime integration, or would a wrapper library to convert models or operations suffice?

Are there specific operations or model types (e.g., CNNs, RNNs, SNNs) you want to support with this integration?

What level of performance or feature compatibility are you targeting?

This will help in framing an implementation plan or surfacing relevant resources.

Вы сказали:
1. Yes
2. Full runtime as possible
3. Make as more as possible compatible with CUDA
4. feature compatibility first

Please use Rust
ChatGPT сказал:
Thanks for the details. I’ll begin researching how to develop CUDA-style drivers in Rust that allow Neuromorph to serve as an accelerator backend for PyTorch and TensorFlow, with maximum feature compatibility and a runtime integration. I’ll focus on matching CUDA functionality closely while using the Neuromorph architecture and API.

I’ll get back to you with a plan, architecture overview, and potential libraries and resources you can use.

Research completed in 6m· 22 источников· 148 поисков
CUDA-Style Driver Integration for Neuromorph Using Rust

Plan for a CUDA-Style Neuromorph Driver in Rust
Overview of CUDA Driver Architecture and Neuromorph Mapping
CUDA’s software stack is organized into layered components that manage GPU execution. At the lowest level, a CUDA driver interfaces with the GPU hardware, coordinating tasks like kernel execution scheduling, memory transfers, and synchronization between CPU and GPU
docs.vultr.com
docs.vultr.com
. Above the driver sits the CUDA runtime API, which provides higher-level, user-friendly functions for device management, memory allocation, and launching kernels, abstracting some of the low-level details
docs.vultr.com
. In addition, the CUDA toolkit includes compiler tools (like NVCC) to compile GPU code and a suite of optimized libraries (cuBLAS, cuDNN, etc.) that implement common computations on the GPU
docs.vultr.com
. In summary, key concepts in the CUDA driver model include:
Device Contexts: an execution context on a specific GPU device, analogous to a process handle, which holds GPU memory allocations, kernel modules, and streams.
Streams and Events: streams are sequences of commands (kernel launches, memcopies) that execute in order on a device, enabling concurrency by running multiple streams in parallel
leimao.github.io
. Events mark synchronization points.
Memory Management: allocation and deallocation of GPU memory, data transfer between host (CPU) and device (GPU) memory, often with support for page-pinned (“pinned”) memory for faster transfers.
Kernel Launch: the invocation of GPU functions (kernels) with a specified grid (thread blocks) and block dimensions. The driver loads compiled kernel modules or JIT-compiles PTX (GPU assembly) into binary code and dispatches the kernel to the device.
Mapping to Neuromorph: Neuromorph (a neuromorphic accelerator) will require analogous components in its driver, even though its architecture is fundamentally different from a GPU. A neuromorphic device typically consists of a network of spiking neuron cores or crossbar arrays rather than SIMD multiprocessors, and it operates on an event-driven model (spike events propagating between neurons) rather than executing warps of threads. Despite this difference, we aim to mimic the semantics of CUDA’s API as much as possible to integrate with existing ML frameworks:
A Neuromorph Context can represent a handle to the neuromorphic processor or simulator instance. Like a CUDA context, it would manage resources (e.g. allocated synapse memory, neuron state) and would be created per device (if multiple Neuromorph devices) or per application. This is similar to how a CUDA context encapsulates a GPU’s state for a process
forums.developer.nvidia.com
.
Streams in a GPU correspond to independent command queues. In a neuromorphic setting, true concurrent kernel execution may not be applicable (the device might run a continuous simulation rather than discrete kernel calls), but we can still expose a stream abstraction. For example, one could use streams to separate different execution flows (perhaps different neural network simulations) or to overlap host-device communication with neuron simulation steps. If Neuromorph supports time-sliced execution or multiple networks concurrently, streams could map to those concurrent pipelines. At minimum, we can define a default stream to enqueue operations (like loading data or triggering a simulation step).
Memory Management for Neuromorph must handle the device’s memory (which might represent synaptic weights, neuron parameters, etc.). The driver should provide functions analogous to cuMemAlloc/cudaMalloc (e.g., neuromorphMalloc) to allocate memory on the accelerator’s memory or in the simulator’s data structures. Transfers (cudaMemcpy) would correspond to loading data (e.g. weight matrices or input stimuli) from host memory into Neuromorph device memory. If Neuromorph memory is unified with host memory (e.g., if it’s a simulator using host RAM), these calls could be mostly bookkeeping. If it’s a PCIe device, the driver might perform actual DMA transfers. Synchronization of memory (like CUDA’s pinned host memory for direct access) should be considered for performance.
Kernel Execution Equivalents: In CUDA, a kernel is arbitrary code running on the GPU. Neuromorphic hardware doesn’t execute arbitrary code; instead, it runs a neural network simulation or inference continuously or for a number of time steps. To mimic “kernel launches,” we could define a concept of a neuromorphic kernel as a pre-configured neural network task. For example, the driver might have an API to load a neural network model (spiking neural network or even a converted standard DNN) onto the device. Launching a “kernel” could mean triggering the execution of the network (e.g., run the simulation for N cycles or process a batch of inputs). This call would be analogous to cudaLaunchKernel but would map to “start the neuromorphic computation for the given input.” The grid and block dimensions in CUDA have no direct meaning for a neuromorphic processor; instead, the parallelism is inherent in the network of neurons. We might repurpose these parameters or ignore them. For instance, one might treat each neuron core or block of neurons as akin to a CUDA thread block, but it may be more sensible to hide this from the user and provide a simpler launch API (since the user typically won’t manually configure neuron parallelism the way they configure thread grids). Instead, the driver might automatically utilize all neuromorphic cores for the given network.
Feature parity considerations: To be as transparent as possible to existing ML libraries, the Neuromorph driver should handle constructs like streams, events, contexts, memory copy, and kernel launch in ways that don’t break the expected semantics. For example, if PyTorch expects that operations on different devices or streams can run concurrently, the Neuromorph driver could internally spawn threads or utilize asynchronous simulation to emulate concurrency. Similarly, if PyTorch calls for a synchronization (e.g., cudaStreamSynchronize or barrier at end of an operation), the Neuromorph driver must flush the pending neuromorphic computations and ensure results are ready. The goal is that, to the framework, using device="neuromorph:0" would feel much like using CUDA:0, even if under the hood the execution model is different. However, not everything maps cleanly. Neuromorphic hardware typically operates in real-time or event-driven fashion, and it might not support precise analogues of every GPU operation. For instance, random number generation (cuRAND) or BLAS libraries would need neuromorphic-specific implementations (if at all possible). Some GPU features like concurrent kernel execution, atomic operations on global memory, or very large memory buffers might be limited on Neuromorph. These limitations need to be identified and documented. In the design, we should allow the higher-level software to query capabilities (perhaps via a properties API similar to cudaGetDeviceProperties). In cases where a direct mapping is impossible, the driver might emulate the behavior on the host or fall back to CPU execution for that operation (with appropriate warning). The emphasis is on feature compatibility – maximizing what can be done on Neuromorph while presenting a CUDA-like interface.
Rust-Based Driver Stack Architecture
Designing the driver stack in Rust brings benefits of memory safety and modern abstractions, which is especially valuable given the complexity of accelerator drivers. Below we outline a Rust-centric architecture for the Neuromorph driver, including how it will interface with other languages (FFI), manage resources, and execute computations. We propose separating the stack into layered components, each potentially implemented as a Rust crate for clarity and modularity:
Layers and FFI Boundaries
Low-Level Hardware Interface (HAL layer): If the Neuromorph accelerator has a low-level interface (e.g., registers, PCIe memory map, sysfs or ioctl interface for a kernel driver), this layer interacts with it. In Rust, this might be a neuromorph-sys crate exposing unsafe FFI bindings to C functions or direct OS calls. For example, if a kernel module exposes a /dev/neuromorphX device file, the Rust driver can use the libc crate or nix crate to perform open/mmap/ioctl system calls to communicate with the device. If Neuromorph’s hardware API is provided as a C library (say the vendor provides a C SDK), we would generate Rust bindings to that with bindgen or write them manually. This layer is kept minimal and unsafe, focusing on translating Rust calls to the actual hardware commands.
Core Driver API (Safe Rust): On top of the HAL, we build a safe Rust API that mirrors the CUDA driver/runtime functionality. This could be our main neuromorph crate. It will provide Rust structs and impl methods for concepts like Device, Context, Stream, Event, MemoryBuffer, etc., each managing the relevant resource. For instance:
A NeuromorphContext struct could hold a handle (from the HAL, like a file descriptor or context ID) and implement methods like synchronize() or load_network(module) (to load a compiled network).
A NeuromorphStream struct would encapsulate an execution queue or thread. If the hardware doesn’t have multiple queues, we can simulate streams by maintaining separate command buffers and possibly separate host threads to execute them. When the user (or framework) launches an operation on a given stream, the driver enqueues it and returns control immediately, letting an internal thread actually perform the work. This mimics CUDA’s asynchronous launch semantics.
A DeviceMemory struct would wrap a pointer or handle to memory on the Neuromorph device. Implementing Drop for this struct allows us to automatically free the device memory when it goes out of scope, preventing leaks (safe Rust ensures that this will run if the object is dropped). We might integrate a memory allocator for the device; for example, using an existing GPU memory allocator crate (like gpu-alloc which is designed for Vulkan/Metal) to manage memory regions
docs.rs
. If Neuromorph memory is linear, a simple bump allocator or buddy allocator can be implemented, possibly with pooling to reduce fragmentation. Rust’s strong typing means we can differentiate between host pointers and device pointers at compile time to avoid misuse.
An Module or NetworkHandle struct to represent a neural network loaded onto the device (if applicable). This could store references to device memory for weights and network topology.
FFI for External Integration: To integrate with PyTorch and TensorFlow (which are in C++ and C, respectively), the Rust driver needs to expose a C-compatible interface. We will design a C API that mirrors major operations (similar to how CUDA has a C API). For example, neuromorphInit(int device_id, NeuromorphContext** outCtx) or neuromorphMalloc(NeuromorphContext* ctx, void** devPtr, size_t bytes), etc. These would be extern "C" functions in Rust (in the #[no_mangle] pub extern "C" form), calling into our safe Rust core API. We can use cbindgen to automatically generate the C header file for these functions, ensuring the Rust and C sides stay in sync. This C API becomes the bridge that PyTorch or TensorFlow will use. Importantly, if performance-critical loops are needed (for example, copying large buffers), the Rust code should internally use efficient methods (like using std::ptr::copy_nonoverlapping for memory copy implementations, or calling into optimized SIMD routines if needed). Rust can link to C libraries if necessary for heavy math – e.g., if Neuromorph supports some BLAS-like ops, one could link to an existing matrix library, or use Rust crates like ndarray with BLAS. But for the driver aspects, likely most will be custom logic.
Error Handling and Result Types: In Rust, we will use Result<T, NeuromorphError> for functions internally. But the C API has to translate that to error codes (since C callers expect error codes or status). We can maintain an internal thread-local last_error message or use an error enum that converts to numeric codes. For instance, define NEUROMORPH_SUCCESS = 0 and various error codes for memory allocation failure, invalid argument, etc., similar to CUDA’s cudaError_t. Each extern C function can catch any Rust panic (using std::panic::catch_unwind to avoid unwinding into C) and return an appropriate error code. This ensures robustness across the FFI boundary.
The use of Rust ensures that things like memory leaks or undefined behavior in the driver are minimized. For example, the RAII paradigm (Resource Acquisition Is Initialization) means that when a NeuromorphStream goes out of scope, its Drop implementation can notify the hardware to destroy the stream or free any associated resources. Similarly, if a user forgets to free device memory, the Rust DeviceMemory destructor will handle it when the object is dropped. This is analogous to how C++ smart pointers would work, but with Rust’s stronger guarantees against dangling pointers and data races.
Memory Management Strategy
Memory management in the driver has two aspects: managing Neuromorph device memory and handling host-memory interactions:
Device Memory Allocation: The driver should maintain a memory allocator for the Neuromorph device’s memory space. Many modern accelerators have specialized on-chip memory or attached DRAM. If Neuromorph’s memory is limited, fragmentation could become an issue. A strategy is to implement a simple allocator that can allocate and free blocks, possibly using a free-list or buddy allocator. The gpu-alloc crate demonstrates an approach for Vulkan-like APIs, which we could adapt
docs.rs
. We could also use gpu-allocator which provides a generic GPU memory allocator (used for graphics APIs)
crates.io
. These crates are unsafe by themselves but can be wrapped in a safe interface. Alternatively, if Neuromorph uses pre-allocated memory pools, the driver might simply request a large chunk from the OS (or the device driver) and then suballocate from it internally.
Unified Memory or Not: Determine if Neuromorph supports unified memory addressing (like NVIDIA’s UVM where CPU and GPU share an address space). If not, we have distinct host and device addresses. The Rust API should reflect that distinction clearly, e.g., by using separate types or at least documentation to indicate a pointer is device-side. If unified memory is available (for example, some simulators just use main memory), we might still maintain the abstraction of separate memory for compatibility, but under the hood the copy operations could be no-ops or simple memory moves.
Pinned Host Memory: For high throughput, many GPU drivers allow page-locked host memory for zero-copy or faster DMA. If Neuromorph benefits from this (e.g., faster feeding of input spikes from pinned memory), the driver can expose a neuromorphHostAlloc that allocates page-locked memory on the host (via OS calls). This would again be a C-callable function, and in Rust we can use libc::mlock or allocate via mmap with MAP_LOCKED. However, this depends on whether the hardware can directly access host memory or not.
Memory Copy and DMA: Implement functions equivalent to cudaMemcpy, both synchronous and asynchronous (with stream argument). For example, neuromorphMemcpy(void* dst, const void* src, size_t bytes, enum NeuromorphMemcpyKind kind) where kind might be HostToDevice, DeviceToHost, DeviceToDevice. If Neuromorph is a simulation library running on CPU, a HostToDevice copy might simply be a memmove into an internal buffer. If it’s actual hardware, this call needs to trigger a DMA transfer; often that would be done by writing into a driver (perhaps via an ioctl that starts a DMA, or by memory mapping the device memory and doing a local copy for small data). We must ensure that if a copy is issued to an async stream, we don’t block the caller; instead, queue the copy operation and signal completion via an event or by the stream mechanism. This parallels CUDA behavior, where cudaMemcpyAsync on a stream returns immediately and actual copy happens in parallel.
Memory Coherency: If the neuromorphic device computes in an analog or spiking domain, reading back “memory” (like neuron states or synaptic weights) might not be as straightforward as reading GPU memory. But assuming we have a well-defined representation, the driver must provide a way to retrieve results. For example, after running a network, we might need to copy the output spike counts or neuron voltages back to the host so they can be interpreted as the output tensor. The driver might maintain an output buffer in device memory that is written during simulation, which can then be copied back.
In Rust, we will guard these operations with appropriate safety checks. For example, if a buffer length on host is shorter than what we want to copy from device, we will catch that and return an error rather than overflow memory (something that in C could lead to a crash). The Rust driver can use slice lengths to ensure safe copying. Internally, though, copying from device to host might involve unsafe code (since we are dealing with raw pointers and possibly FFI), but we keep those in as few places as possible, reviewed carefully for correctness.
Execution Engine and Scheduling
At the heart of the driver is the execution engine that will run computations on the Neuromorph substrate. Unlike CUDA, where the execution engine is largely in the GPU hardware (the GPU’s warp schedulers, etc.), here the driver might need to do more work:
Neural Network Compilation: If we want PyTorch or TensorFlow to run models on Neuromorph, we need to translate those models into a form the Neuromorph device understands. There are two possible approaches:
Layer-by-Layer Execution: Implement each primitive operation (Conv2D, Dense, ReLU, etc.) directly on Neuromorph. This would mean writing “kernels” for each operation that configure the neuromorphic hardware to execute that operation. For example, to do a matrix multiplication on a spiking chip, one might configure a crossbar of spiking neurons whose steady-state firing rates represent the matrix product. This is complex and might not yield full compatibility, but it allows reuse of PyTorch’s existing per-layer execution schedule (Eager mode: execute one op at a time on device).
Graph Compilation: More akin to how TPUs or Graphcore IPUs work, we capture an entire model (or a subgraph of operations) and compile it into a neuromorphic network. For instance, use PyTorch’s JIT or FX trace to get the compute graph, then convert layers into spiking neural network equivalents. This compiled SNN (possibly running for a number of time steps to produce an output) is then loaded as a single "module" onto Neuromorph and executed as a whole. This approach can be more efficient on event-driven hardware and can leverage the hardware’s parallel nature better. It also more closely resembles how one would normally use a neuromorphic chip (configure once, then run many inferences through it).
Given the goal of transparency, the first approach (op-by-op) provides a closer illusion that PyTorch operations are running natively. However, if Neuromorph cannot truly execute an operation in one step (e.g., ReLU might just be thresholding which is natural for spiking neurons, but matrix multiply is not one step), the driver would internally break it down: maybe run a spiking network for a certain duration to perform the matmul, then read out the result. The second approach might require users to opt-in to a compile step (like using torch.compile or torch.jit.trace explicitly), which is less transparent. Ideally, we can support both: a default eager mode implementation for core ops and an option to do ahead-of-time compilation of whole models for efficiency. The Rust execution engine can be structured to handle both modes:
Provide trait implementations or functions for each supported operation. For example, a Rust function fn conv2d_forward(ctx: &NeuromorphContext, params: ConvParams, input: &DeviceMemory, output: &DeviceMemory) -> Result<(), NeuromorphError> that sets up the neuromorphic cores to perform a convolution. This could involve translating the convolution kernel into a series of synaptic weight values and neuron connections, programming those into the device, feeding the input (perhaps as a temporal spike pattern or as current injections), running the simulation for required ticks, and then copying the result out. All of that would happen under this function before returning. This is complex but would let us register that function as the kernel for PyTorch’s convolution op.
Alternatively, a more generalized approach: have a routine to execute a pre-loaded network. If we compiled a section of the model into an SNN and loaded it (represented by a NetworkHandle), then the PyTorch operator “NeuromorphModuleForward” could simply call something like neuromorphLaunchNetwork(ctx, network_handle, inputs..., outputs...). This would instruct the hardware to run one inference and produce outputs. The driver might handle multiple networks by ID.
Scheduling and Concurrency: The driver must also manage how operations are scheduled:
If only one host thread is driving the device (typical in single-GPU single-thread scenarios), we can have a simple queue per stream. The Rust code can maintain a worker thread (or a thread pool) that pops operations from stream queues and executes them in order. This is similar to how CUDA’s software driver might delegate work to the GPU’s command queue.
If multiple host threads can call the driver (e.g., in PyTorch DataParallel or multi-threaded input pipelines), we need to ensure thread safety. We will use Rust’s concurrency primitives (e.g., Mutex or RwLock around shared device context data). For instance, we might lock the context when launching a new operation to append it to a queue. We also have to guard hardware access: if two threads try to launch different networks simultaneously, and the hardware can’t actually parallelize them, the driver should serialize them or return an error. Possibly, we restrict a context to single-thread use (document it as not thread-safe except via provided synchronization).
Implementing events: we can model an Event as a synchronization point that a stream signals when it reaches that point. In Rust, an Event could be implemented with a std::sync::Condvar or a channel. When one stream records an event, it essentially notes in the queue “signal event X when you get here”. Another stream waiting on that event would block until signaled (or we integrate with the framework’s synchronization calls properly). The actual hardware might not have an event concept, so this is purely a software construct.
The execution engine in Rust can also integrate a simulation loop if Neuromorph is simulated on CPU. For example, if Neuromorph is a software simulator (like perhaps stanta/neuromorph is itself a Rust simulator of spiking neurons), we can link or embed that simulator. In that case, executing a network might mean calling into that simulation library’s API (which steps through time). If the simulation library is event-driven, we could either use it directly or wrap it to look more like a “run X ticks” function call.
Rust Crates, Tools, and Build System Support
To implement and build this driver stack, several Rust and ecosystem tools will be invaluable:
FFI and Bindings: As mentioned, bindgen can generate Rust bindings to any Neuromorph C/C++ hardware API (if one exists). Conversely, cbindgen can generate C headers for our Rust library, which is helpful for exposing the plugin API to TensorFlow or PyTorch. If we need to interface with C++ (especially for PyTorch’s extension), the cxx crate can be used to create safe bridges between Rust and C++ without manual unsafe code. For example, we could write the PyTorch extension in C++ but have it call into Rust via a thin cxx layer, which handles things like mapping Rust std::string to C++ strings, etc.
Build System: We will produce the driver as a dynamic library. For PyTorch, likely a library called libtorch_neuromorph.so (if we use the custom device approach) or a Python wheel that includes the compiled library and Python binding code. We can use Cargo to build the Rust library (with crate-type = ["cdylib"] for a C-compatible dynamic library). If integrating with Python packaging, tools like Maturin or setuptools-rust can help package the Rust library as a Python module (especially if we decide to provide Python bindings for testing or user tools). For TensorFlow’s plugin, we will build a .so that TensorFlow can load at runtime via TF_LoadPluggableDeviceLibrary. This can be done with Cargo as well. We might need to ensure the library name and exported symbols match what TensorFlow expects (usually you export a TF_InitPlugin symbol or similar – according to TensorFlow’s C API for plugins).
tokio or async runtime: If the Neuromorph driver will handle a lot of asynchronous tasks (especially if simulating things in software), using Rust’s async facilities could be beneficial. For instance, we could use an async channel per stream, and spawn tasks for operations. However, integrating that with external C calls (which are synchronous) is tricky, so we might keep it mostly synchronous internally and rely on internal threads for now. Simpler thread spawning via std::thread might suffice for launching a background worker.
Logging and Debugging: It’s helpful to include logging (with log crate and env_logger) to troubleshoot the sequence of operations, especially when plugging into complex frameworks. We can allow an environment variable to enable verbose logging of every call (similar to CUDA’s debugging modes).
Testing: We should create unit tests for the Rust API (e.g., allocate memory, write pattern, copy to device, copy back, verify integrity; simulate a trivial “network” that maybe just echoes input to output, test that path). Additionally, integration tests with PyTorch and TensorFlow (in Python scripts) would validate that a simple model can run end-to-end on Neuromorph backend.
Rust’s ecosystem does not (yet) have as many purpose-built ML driver crates because historically these drivers are C/C++. However, its strong memory safety and concurrency support make it well-suited for writing such low-level systems software. We will be careful to mark unsafe blocks where needed and justify them, and leverage the community crates for any standard functionality (for example, using thiserror or anyhow to manage error types internally, or serde if we need to serialize any configurations).
Integration with PyTorch (Custom Backend)
To make PyTorch use Neuromorph as an accelerator, we need to register it as a new device type in PyTorch and provide implementations for the required tensor operations. PyTorch is primarily written in C++, so our integration strategy is to create a C++ extension that hooks into PyTorch’s dispatcher, and calls our Rust driver via FFI. The high-level steps and design are:
Device Type Registration: PyTorch (as of 1.8+ and improved in later versions) allows out-of-tree backends through “PrivateUse” dispatch keys
i32n.com
. We will use one of these reserved keys (say PrivateUse1) as the identifier for Neuromorph. This involves creating a TensorImpl subclass or using PyTorch’s OpaqueTensorImpl if our storage is not a regular CPU/CUDA storage
i32n.com
. We can follow PyTorch’s extension tutorial, which indicates that adding a new backend means “writing a kernel for each PyTorch operator in C++ and registering them to a dispatch key representing your customized backend.”
i32n.com
. In practice, we don’t need every op – only those that will be used (others can fall back or error out). PyTorch notes there are 1600+ ops, and out-of-tree backends should focus on a core subset since many ops decompose into simpler ones
i32n.com
. We will identify key ops to support (e.g., linear algebra ops, elementwise ops, some reductions) that allow typical neural network layers to run. PyTorch’s mechanism will dispatch to our kernels whenever a tensor of device type “Neuromorph” is used with an operation.
C++ Extension Implementation: We create a C++ library (likely as part of the Python package) that uses TORCH_LIBRARY_IMPL macros to register our custom kernels to PrivateUse1 dispatch key
i32n.com
. Each kernel will be a C++ function that calls into the Rust driver’s C API:
For example, a registration for matrix multiplication might look like:
at::Tensor neuromorph_matmul(const at::Tensor& A, const at::Tensor& B) {
    // assume A, B are Neuromorph tensors (with our dispatch key)
    auto C = at::empty({A.size(0), B.size(1)}, A.options()); // output tensor on Neuromorph
    neuromorphMatMul(/*context*/ get_context(), 
                     A.data_ptr(), B.data_ptr(), C.data_ptr(), 
                     A.size(0), A.size(1), B.size(1));
    return C;
}
TORCH_LIBRARY_IMPL(aten, PrivateUse1, m) {
    m.impl("mm", neuromorph_matmul);
}
In this pseudo-code, neuromorphMatMul is an extern "C" function provided by our Rust driver (exposed via cbindgen). We would have a way to get the current Neuromorph context (perhaps each tensor’s TensorImpl stores a pointer to the device context).
We will do similar for other ops (convolutions, activation functions, etc.). For elementwise ops like ReLU or sigmoid, if the neuromorphic hardware can apply a threshold or function neuron-wise, we might offload it; otherwise, we can simply use a CPU fallback or a trivial kernel that just marks the tensor (if the next op will do actual computing).
Memory management: the data_ptr() of our tensor would likely be a pointer into device memory. Actually, since PyTorch’s Tensor is a wrapper, our OpaqueTensorImpl can hold a pointer value that represents device memory. We might not want to expose raw device pointers to PyTorch (which might assume it’s accessible). Instead, our tensor could have a size but no meaningful data_ptr on CPU. We could override PyTorch’s copy routines to handle copying between Neuromorph and CPU (so that if the user requests .cpu() on a Neuromorph tensor, we invoke our driver to copy data out). This requires implementing to() or copy kernels as well.
Autograd Support: PyTorch’s autograd can be extended to custom backends using an AutogradPrivateUse1 key
i32n.com
i32n.com
. We will need to implement backward kernels for any operation we want to use in training. For example, if we implement a Neuromorph linear layer, we should implement its gradient computation (or mark it as requiring conversion to CPU for backward). In early stages, we might focus on inference (forward-pass) only, and rely on PyTorch’s ability to do autodiff in a device-agnostic way by switching devices (not ideal). Ideally, we implement backward by either:
Deriving analytic gradients if the neuromorphic operation is deterministic and differentiable (some may not be straightforward, e.g., spiking nonlinearity is not differentiable; but surrogate gradient methods exist).
Or by allowing PyTorch to temporarily copy data to CPU, compute gradient, and copy back (which would be slow).
This is a complex area: neuromorphic hardware often uses specialized training algorithms (e.g., surrogate gradients, on-chip learning) which differ from standard backpropagation. For the scope of integration, we might choose initially to support inference and static models, and leave training support for a future iteration (or support only a subset of operations with custom backward).
Usage in PyTorch: Once registered, users (or the integration library) should be able to do:
torch.utils.rename_privateuse1_backend("neuromorph")
device = torch.device("neuromorph:0")
x = torch.randn(…)
x = x.to(device)        # copy input to Neuromorph
model.to(device)        # model parameters on Neuromorph
out = model(x)          # runs model using our kernels
The call to rename_privateuse1_backend is a utility to give a friendly name to the PrivateUse1 device (so it appears as “neuromorph” instead of “privateuse1”)
github.com
. We will provide that or instruct the user to do so. After that, torch.device("neuromorph:0") becomes valid. PyTorch will list the device in torch.device.count as well, which helps transparency. We expect most PyTorch model code to “just work” if our operator coverage is sufficient. If an unsupported op is encountered, PyTorch will throw an error or we can implement a fallback that moves that tensor to CPU, does the op, and moves it back (PyTorch allows such fallbacks, though with performance cost)
i32n.com
. A sophisticated approach is to leverage PyTorch’s graph capture (with torch.compile) – see below.
Testing and Example: A simple example could be adding two tensors. If we register an implementation for the aten::add.Tensor operator for Neuromorph, we can test:
a = torch.ones(5, device="neuromorph:0")
b = torch.ones(5, device="neuromorph:0")
c = a + b  # should invoke our neuromorph add kernel
print(c)   # ideally yields tensor of 2's on neuromorph device
If this works, then we can build up to more complex ops. We should validate that the results match CPU or CUDA results for the same operations.
Alternate PyTorch Integration (Graph Capture): PyTorch 2.0 introduced torch.compile, which can use backend hooks to capture the entire model graph and lower it to alternative execution backends
docs.pytorch.org
medium.com
. We could write a backend function for torch.compile that converts a model’s torch.fx graph to a neuromorphic program. This approach would be similar to how AWS Neuron or PyTorch/XLA works: the backend would trace the model, then our code would take the graph (perhaps in Relay or ONNX form) and compile it for the Neuromorph device. For example, AWS’s Neuron SDK compiles a TorchScript graph to Inferentia instructions offline
docs.aws.amazon.com
. We could attempt something analogous: use our Rust library or an external tool to convert an NN to spiking form (if needed), then load it. The advantage of this method is that we don’t have to implement 1600 tiny ops; we can implement a handful of building blocks and let the compiler allocate neurons, set weights, etc., globally. The disadvantage is it’s not as plug-and-play (users must call torch.compile or otherwise know to use our backend). Given our goal of transparency, the dispatch key approach (making a true device) might be more seamless for users. In fact, we can do both: support eager mode with core kernels, and also offer a graph-compiler path that might be more optimized for certain models. Summary for PyTorch: We will provide a PyTorch extension (likely named torch_neuromorph) that registers the “Neuromorph” device. This extension is essentially the glue: it contains C++ code to interface with PyTorch and call into the Rust driver library (which it links to). Once installed (e.g., via pip wheel), users can move their models to neuromorph:0 and run forward passes. The extension will handle device placement, and each operation will delegate to the Rust driver which in turn commands the Neuromorph hardware or simulator. This design, while requiring substantial development (implementing kernels or a compiler), aligns with how one integrates new hardware in PyTorch
i32n.com
i32n.com
 and has precedent in projects like AMD’s ROCm (which implements a CUDA-like HIP backend under PyTorch) or Graphcore’s PopTorch (which compiles models for IPU) – we will draw from those experiences.
Integration with TensorFlow (Pluggable Device)
TensorFlow has introduced a PluggableDevice mechanism to allow adding new device backends without modifying TensorFlow’s core, via a stable C API
blog.tensorflow.org
blog.tensorflow.org
. We will leverage this to integrate Neuromorph as a device in TensorFlow. In essence, we will create a plugin library that TensorFlow can load at runtime, which registers a device type (say "NEUROMORPH") and provides implementations for device operations using our Rust driver. The major components of this integration are: Figure: TensorFlow PluggableDevice architecture. The device plug-in (left) implements device factory registration, memory allocator, stream executors, and kernel ops via the C APIs, allowing TensorFlow (right) to treat it like a native device
blog.tensorflow.org
blog.tensorflow.org
.
StreamExecutor C API: TensorFlow uses the StreamExecutor interface for device execution and memory management
blog.tensorflow.org
. We need to implement the functions in this C API for Neuromorph. This includes:
Device discovery and enumeration: a function to list available Neuromorph devices on the system (e.g., if multiple chips or boards are present). We query our Rust driver (perhaps it checks /dev entries or a config) and return how many devices, their names, etc.
Device creation (DeviceFactory): when TensorFlow wants to instantiate the device, our plugin provides a factory that creates a StreamExecutor object with function pointers implemented. In C terms, we fill out a struct of function pointers for operations like allocate, deallocate, memcpy, create stream, destroy stream, enqueue operations, etc.
Memory: functions to allocate and free device memory (similar to our neuromorphMalloc/Free in the driver C API) and to allocate host memory for use by the device (if needed). Also, functions for memory copy (host<->device, device<->device). We will hook these to our Rust driver’s implementations.
Streams and events: functions to create a stream, destroy a stream, allocate an event, record an event, wait for an event, etc. Again, these will tie into our Rust streams. For instance, create_stream would call into Rust to create a NeuromorphStream and return an opaque handle (or we store it in a context and return a pointer).
Kernels (Operations): This part involves how individual TensorFlow ops are executed on the device. TensorFlow’s PluggableDevice C API allows registering custom ops and their kernel implementations for the device
blog.tensorflow.org
. For each TensorFlow op that we want to support on Neuromorph, we write a kernel function (in C/C++ or Rust via FFI) that launches the computation on the device. For example, to support MatMul op, we register a kernel with device type "NEUROMORPH". The registration is done via the C API (providing an op name, attribute specs, etc., and function pointers for the compute and optionally prepare/shape functions). We then implement the compute function to call our Rust driver as appropriate. If our Rust driver has a synchronous function to do matmul on Neuromorph, the kernel function can simply call that and block until completion (or, more ideally, launch it asynchronously and immediately return control to TF, but TF might assume kernel compute is done when it returns unless we integrate with its async mechanisms).
Graph optimization: The PluggableDevice API also allows registering a custom Grappler pass
blog.tensorflow.org
. We could use this to optimize subgraphs for neuromorphic execution – for instance, fuse certain ops or convert patterns into a single neuromorphic kernel. This is advanced and optional. A simpler initial approach is to rely on TensorFlow’s existing ops and just implement those kernels.
Rust and the C API: We have two choices for implementing the TF plugin:
Write the plugin in C++ and call Rust driver functions. This might be straightforward since TensorFlow’s provided API headers are in C++ (with C bindings). We can use our Rust C API here as well. For example, the device factory will call neuromorph_init() (Rust) and create contexts; the op kernels will call neuromorph_do_conv() etc.
Write the plugin entirely in Rust by providing extern "C" functions that match TensorFlow’s API signatures. This is doable (especially since TF’s C API is C, not C++). We would use cbindgen to produce a header that matches what TF expects, or manually ensure our exported symbols align. However, given TensorFlow’s API is extensive, it might be simpler to do a thin C++ layer.
For clarity, an approach is:
Rust library: core driver (which we already have) plus a new crate or module for TensorFlow integration that exposes C symbols for TF_InitPlugin and all required callback implementations.
C++ glue (minimal): if needed, to handle any C++ TF structures or to use convenience macros from TF.
TensorFlow expects the plugin library (say libtensorflow_neuromorph.so) to define an initialization function that it will call during tf.config.list_physical_devices() or the first use. In that function, we register our device type. For example, using TF’s C API we would call TF_InitPluggableDevicePlugin(ctx, &device_info, num_devices) providing function pointers. After successful registration:
TensorFlow will recognize a device type "NEUROMORPH". Users can then do tf.device("/physical_device:NEUROMORPH:0") or simply if TF decides to place ops on it by default when available.
We should ensure that when TF asks for device attributes (memory size, etc.), we provide plausible values (even if the hardware doesn’t have a concept of “memory size” in the traditional sense, we can report total synapse memory or similar).
Our plugin will handle memory growth or pre-allocation if needed (TensorFlow might ask for BFC allocator behavior – we can choose to implement a simple malloc/free or a buffered allocator).
Kernel implementation example: Suppose we want to support a Dense layer (MatMul + bias). We would register kernels for MatMul and BiasAdd for device NEUROMORPH. The MatMul kernel function will receive TF_Tensor* inputs and output. These are basically handles to memory. For our device, those tensors reside in Neuromorph memory (if placed on our device). The kernel implementation needs to do the actual multiplication on Neuromorph. We could call into a Rust function neuromorph_matmul(context, A_ptr, B_ptr, C_ptr, M, K, N) which uses the hardware to compute C = A*B (with appropriate dimensions). This might involve configuring a crossbar or running a spiking simulation for a few steps. If the operation can be done quickly, we might do it synchronously and return. If it’s long-running, a consideration is that TensorFlow expects the kernel call to finish relatively quickly. In GPU, the kernel launch is asynchronous but the StreamExecutor may block until completion depending on config. We might have to block in the plugin until the neuromorph computation is done (to keep TensorFlow’s execution model consistent, especially in eager mode). In graph mode, we could potentially return control and use an event to signal completion, but that complicates the plugin logic. Initially, a safe approach is to perform the complete op and then return.
Data format considerations: TensorFlow might pass data in NHWC or NCHW format for images, etc. Our driver might prefer one or the other. We might need to reorder data as it comes in or ensure our ops account for it. This is common in accelerator plugins (e.g., some accelerators only support NHWC, so the plugin inserts transpose ops).
Existing Plugin Examples: We can look to Apple’s tensorflow-metal plugin or AMD’s DirectML or ROCm plugins. For instance, Apple’s Metal plugin registers a GPU device for Mac; AMD’s ROCm (for Linux) was integrated by building TF with ROCm support, but newer approaches include pluggable device for DirectML (on Windows)
developer.apple.com
. These serve as templates for what functions to implement. The TensorFlow repository’s stream_executor has a C API (see stream_executor.h) that defines what’s needed.
By using the PluggableDevice architecture, no changes are required in user code to use Neuromorph in TensorFlow. If our plugin is installed, a standard tf.keras model or tf.Graph computation can be placed on the Neuromorph device and run. For example:
import tensorflow as tf
print(tf.config.list_physical_devices())
# => [PhysicalDevice(name='/physical_device:CPU:0', device_type='CPU'), 
#     PhysicalDevice(name='/physical_device:NEUROMORPH:0', device_type='NEUROMORPH')]
with tf.device("NEUROMORPH:0"):
    # Build and run a simple model
    x = tf.random.normal([1, 784])
    model = tf.keras.Sequential([...])  # define some layers
    y = model(x)
If integrated properly, TensorFlow will route the ops to our device. Each op’s kernel will invoke the Rust driver. We also need to handle TensorFlow’s memory allocator for our device. Typically, TensorFlow might ask the device to provide an allocator object (to manage device tensors). We can implement an allocator that uses neuromorphMalloc and neuromorphFree. Possibly, TensorFlow’s BFC (best-fit circular) allocator can be reused by providing it a custom allocate/deallocate function (this is how GPU allocators work, to reuse and avoid fragmentation). For simplicity, we might allocate device memory on demand and let TF reuse through our interface. One more integration point: TensorFlow ops that are not easily supported. We should probably start by focusing on basic ops and only advertise the device for inference-type computations. If a user tries to do something like tf.linalg.inv on a matrix on Neuromorph, we might not have an implementation. We can either (a) not register that op at all (TF will then automatically copy the tensor to CPU and use CPU kernel, thanks to TensorFlow’s copy-on-device-miss mechanism), or (b) implement a fallback in our kernel that calls CPU under the hood (less ideal). The pluggable device RFC mentions that if no kernel is registered for an op on that device, it can fall back by copying to another device
github.com
. We should confirm this behavior. It might require marking our device as “supports unimplemented ops via copy” or simply rely on TF’s logic. Graph mode vs Eager mode: In TF2, eager mode is default, but graphs are used in tf.function. Our device should work in both. Eager mode will execute op-by-op with our kernels. In graph mode, the runtime might apply additional optimization; we must ensure our device and ops can be serialized in a GraphDef (the pluggable device should handle this if we register everything properly). Summary for TensorFlow: We deliver a tensorflow_neuromorph plugin library implementing the C API. When loaded, it registers a new device type "NEUROMORPH" with StreamExecutor and kernel callbacks. TensorFlow can then place operations on Neuromorph much like it does for GPU: by using the same high-level APIs (Keras, etc.) with just specifying the device. The heavy lifting of actual compute and data movement is handled by our Rust driver underneath. This design follows the pattern described by TensorFlow’s PluggableDevice architecture
blog.tensorflow.org
blog.tensorflow.org
, ensuring we don’t need to maintain a fork of TensorFlow – our device stays as an isolated plugin.
Challenges in Mapping GPU-like APIs to Neuromorphic Hardware
Integrating a neuromorphic accelerator to mimic a CUDA GPU involves several fundamental challenges, as the computational paradigms differ significantly. We must acknowledge these and plan for how to address or mitigate them:
Programming Model Mismatch: GPU APIs assume an imperative, synchronous model of computation (launch kernel, finish, get results) whereas neuromorphic hardware often runs continuously and event-driven. In a spiking neural network, computation is encoded in spike trains over time rather than instant multiply-add operations. Mapping a single GPU kernel (say a matrix multiplication) to an equivalent neuromorphic computation might require running a spiking network for multiple time steps until it converges to the result. This introduces latency and complexity in extracting the result (e.g., reading spike counts or analog voltages). Moreover, standard deep learning operations are deterministic and differentiable, while neuromorphic computation can be stochastic or refractory. We will likely restrict ourselves to using the neuromorphic device in a deterministic inference mode (e.g., rate-coded neurons) for compatibility. We might also need to calibrate or normalize the outputs to match the numeric ranges expected by PyTorch/TF (since spiking outputs might need scaling to correspond to FP32 values). This mismatch is a core reason neuromorphic hardware isn’t directly used by current frameworks
reddit.com
 – most current AI workloads are not designed for event-based processing
reddit.com
. Our solution is to create a bridge: either by modifying the models (e.g., using SNN models) or by internally converting standard models to spiking versions.
Precision and Data Types: GPUs excel at high-precision arithmetic (FP32, FP16, etc.), whereas many neuromorphic systems use low-bit or analog computation. We have to decide how to represent tensors on Neuromorph. Possibly as 16-bit or even 1-bit spike indicators over time. For integration, we may advertise support for certain dtypes (e.g., float32 and float16) but actually implement them in Neuromorph domain with approximation. This could lead to numerical differences. It’s important to highlight that bit-exact parity with CPU/GPU is unlikely – e.g., summing spikes may have noise or slight variation run-to-run. Frameworks generally assume deterministic outputs for given inputs (except GPU nondeterminism in some reductions). We might need to enforce determinism in our driver’s simulation (e.g., use fixed random seed or deterministic neuron update). If the neuromorphic hardware itself is nondeterministic (due to analog variation), that’s a limitation – for training that’s problematic; for inference it might be acceptable if within error bounds.
Operation Coverage: Some operations in ML frameworks have no meaningful equivalent in neuromorphic hardware. For instance, consider an FFT or sorting – a spiking network could in theory do these, but we likely won’t attempt it. We will focus on common neural network ops (convolutions, dense, pooling, activation, batch norm). Even among these, Batch Normalization is tricky – on GPU it’s an arithmetic operation, on Neuromorph, implementing scaling and mean subtraction across a batch might not be straightforward if the hardware processes one sample at a time or doesn’t support arbitrary scalar ops. We might handle it by executing those on CPU or integrating them as part of network compilation (fold batch norm into preceding layer weights, a common optimization). Another example is variable sequence lengths or control flow (like an if or a loop in a TF graph, or an RNN with dynamic timesteps). Neuromorphic hardware naturally handles time sequences well, but integrating that into a static graph execution is complex. We might restrict certain uses or require unrolling.
Concurrency and Streams: GPUs allow overlapping compute and transfer on different streams for performance. If Neuromorph hardware doesn’t support simultaneous computation and communication (many devices might not, they might stall computation while loading new data), our stream abstraction could be limited. We may implement streams but behind the scenes, all work is serialized. This would still functionally work (the program won’t deadlock), but performance might not improve with multiple streams. In the best case, if Neuromorph has a separate I/O interface, we can overlap host-to-device transfers with ongoing neural activity. We should investigate the hardware capabilities. For a simulator, we could run simulation on one thread and do memory copies on another thread, thus achieving overlap.
Synchronization and Timing: Neuromorphic systems are sometimes real-time (they progress with a wall-clock or a fixed step). Ensuring that when the framework thinks a computation is finished, the neuromorphic hardware’s state is ready is critical. We might have to insert barriers after each op (e.g., wait until spikes have settled). This could incur delays. We might need to deliberately run the simulation for a fixed number of ticks for each operation to ensure completion. Additionally, if multiple ops run sequentially on the device, do they feed into each other properly? E.g., output spikes from layer 1 become input to layer 2 – our driver should handle connecting these or reading out and re-uploading if needed.
Resource Management: Neuromorphic hardware often has a fixed number of neurons and synapses available. A large DNN might not fit at once, requiring time-multiplexing. The driver might need to manage resources by partitioning a network across multiple passes or using off-chip memory to store weights not currently in use (similar to GPU memory swapping, but here neuron context swapping). This is an advanced feature; initially, we might assume the network fits. But if not, our memory management strategy should be aware of capacity and return errors or fallback to CPU if out-of-memory (like GPU OOM errors).
Tooling and Debugging: Standard GPU profilers or debuggers (like Nsight) obviously won’t work. We might need custom ways to verify that Neuromorph execution is correct. This is more of a development challenge but worth noting – we’ll need introspection tools (maybe the neuromorph library has ways to dump spike traces, etc.). When integrated in frameworks, we also want to ensure errors in our driver (e.g., a panic or segfault) don’t crash the whole program. Robust error handling and returning of errors to the framework is needed so that, for example, if something fails, TensorFlow can catch it and raise a Python exception rather than just terminating.
Community and Ecosystem Fit: Neuromorphic computing is still a niche in terms of standard deep learning. Many models assume operations like ReLU, matrix multiply, etc., which can be mapped, but the efficiency might be poor compared to GPU unless the model is adapted for spiking representation. One challenge will be demonstrating that using Neuromorph via this driver actually provides a benefit (e.g., lower power consumption) without massive code changes. This is as much a research question as a technical one. But the integration work we propose aligns with trends noted in literature: “Gradient-based training of deep spiking neural networks is now an off-the-shelf technique... open-source tools underwritten by theoretical results. [...] New software APIs for Neuromorphic development are making efficient use of deep learning tools, permitting training of neuromorphic architectures on GPUs, and giving Neuromorphic tech the look and feel of well-understood systems.”
nature.com
nature.com
. Our driver is precisely in line with giving neuromorphic hardware the familiar “look and feel” of GPU programming models, which is seen as key to its broader adoption
nature.com
.
To mitigate these challenges, our development plan will include:
A tight scope initially: focus on inference for feed-forward SNN equivalents of popular DNNs (CNNs, MLPs). Avoid exotic ops.
Validate on small examples and compare outputs to CPU computations to ensure functional correctness within tolerance.
Provide documentation on limitations (e.g., which ops are supported, requirement for static shapes or certain dtypes).
Possibly integrate with high-level libraries for SNN conversion – for example, mention that for best results the model should be converted to spiking form (using something like SNNTorch or others) before using Neuromorph device. But from the integration perspective, we try not to force that, rather do conversion under the hood if possible.
Tools, Crates, and Build Systems for Rust Integration
We have touched on many tools during the design. Here is a consolidated list of notable ones and their roles:
Cargo and Workspaces: We will likely use a Cargo workspace to organize sub-crates: e.g., neuromorph-sys (FFI bindings), neuromorph-driver (safe API), neuromorph-pytorch (maybe containing the C++ extension code or build script), neuromorph-tf (for the TF plugin). Cargo can manage building all of them. For distributing Python packages, we can use Maturin which directly uses Cargo to build a dynamic library and wraps it in a wheel.
cbindgen
github.com
 and bindgen: cbindgen to generate C headers for the Rust driver’s API (for TF plugin and PyTorch C++ calls). bindgen to create Rust bindings to any existing Neuromorph C API, if applicable (for example, if the Neuromorph hardware comes with a C library for low-level control, we include that and call it).
LLVM/Clang: indirectly used via bindgen, but also if we need to compile GPU code or other embedded code. Possibly not needed unless neuromorph requires JIT compiling some code – which is unlikely, it’s more about configuring networks.
PyO3 or pyo3-built: If we want to expose some Rust functionality to Python for ease of testing (outside of PyTorch), PyO3 allows building a Python module in Rust. For example, we could make a small Python API to directly load a spiking network and run it, for debugging. This isn't required for integration, but a nice-to-have for sanity tests.
Bazel: TensorFlow projects often use Bazel. However, with the PluggableDevice, we can avoid touching TF’s build and just build a .so. If necessary, Google provides a CMake build for the plugin (e.g., the DirectML plugin was built with CMake). We can use CMake with ExternalProject_Add to call Cargo, or we can simply provide instructions to build with Cargo and place the .so in an appropriate place. If integrating into an existing Bazel project, there are Bazel rules for Rust (rules_rust) that could build our crates.
CI/CD Tools: For maintaining this project, it will be useful to set up continuous integration to build and run tests across different OSes (especially if Neuromorph hardware only runs on Linux, we target that; but maybe simulation can run on Mac/Windows too). Ensuring that the Rust code and C++ extension compile with each new version of PyTorch/TF is important (both frameworks have frequent releases that might change internal APIs slightly).
Similar Integration References:
Graphcore Poplar and PopTorch: Graphcore’s IPU integration required writing a compiler (Poplar) and a PyTorch frontend (PopTorch)
docs.graphcore.ai
docs.graphcore.ai
. They strove to keep model changes minimal, which is our goal too
docs.graphcore.ai
. We can learn from their approach of wrapping the model (they require calling poptorch.inferenceModel on the model which behind the scenes compiles it for IPU
docs.graphcore.ai
).
Nvidia CUDA/HIP: AMD’s HIP is an implementation that mirrors CUDA API. In PyTorch, AMD uses a separate backend (ATen code compiled for HIP). For us, we mimic CUDA semantics at the higher level but we’re not implementing a full PTX compiler or anything – our Rust driver is the “equivalent” of CUDA driver but specialized.
Intel’s OneAPI and GPUs: Intel GPUs via oneAPI are integrated into TensorFlow through a plugin (they contributed code to use DPC++ underneath). That shows that even large frameworks are moving to plugin architectures which we are following.
AWS Neuron: AWS Inferentia (Neuron) integrates with PyTorch by providing a custom torch_neuron module and using TorchScript tracing
docs.aws.amazon.com
docs.aws.amazon.com
. This is slightly different (they don’t add a device type, instead you compile the model and the resulting torch.jit module internally calls their runtime). This approach was viable because Inferentia is targeted mostly at inference with static graphs. For Neuromorph, if we aim at similar usage (accelerating inference), we could implement a workflow in TensorFlow where the user does model = tf.neuromorph.compile(model) to convert it. But the question specifically wants “transparently or nearly transparently,” so we lean towards true device integration.
In terms of development timeline: we would likely proceed in phases, using these tools:
Prototype basic Rust driver – using perhaps a dummy Neuromorph implementation or very simple simulation just to test the scaffolding (allocate memory, copy, simple “compute”).
PyTorch integration (basic ops) – use a development build of PyTorch and our extension to test moving a tensor and doing an addition or multiplication.
TensorFlow integration – build the plugin and test tf.constant operations on the device.
Expand coverage & incorporate actual Neuromorph logic – connect to real hardware or a detailed simulator, implement more ops (Conv, etc.), ensure correctness on small nets.
Optimization – measure performance and identify if we need to adjust the approach (e.g., use graph compile for speed, tune memory transfers, etc.).
Each step will use the mentioned crates: for example, step 2 needs cxx or bindgen for PyTorch; step 3 heavy use of C API via cbindgen; step 4 might use simulation crates (there are Rust crates for SNN simulation, like spiking-neural-networks on crates.io
crates.io
, though we might directly use the Neuromorph codebase if it's a simulator).
Examples of Similar Backend Integrations
It’s instructive to look at similar projects for inspiration and validation:
NVIDIA CUDA itself: The gold standard, tightly integrated with frameworks. We emulate its structure (driver/runtime separation, streams, events, etc.) to achieve compatibility. Many frameworks have special paths for CUDA (like optimized kernels); we likely won’t reach that level initially, but by presenting as CUDA-like, we might even be able to reuse some GPU-agnostic code. (For example, if our device is seen as just another DeviceType, operations expressed in device-agnostic terms might “just work” if they eventually call our kernels).
Graphcore IPU (PopTorch): Graphcore’s IPU is a novel architecture (many cores, but not a von Neumann GPU). They built PopTorch which compiles PyTorch models to their Poplar engine. They manage to support most of PyTorch by behind-the-scenes graph compilation
docs.graphcore.ai
. One difference is that they don’t pretend an IPU is a CUDA device – they require usage of their poptorch API. This suggests that when the paradigm is too different, a full transparent integration is hard. However, they did integrate with TensorFlow via XLA as a device. For Neuromorph, we might start with a slight user opt-in (like torch.compile or a custom model class) if needed for complex models, but keep aiming for device transparency.
BrainScaleS, Loihi, and other neuromorphic research frameworks: Intel’s Loihi chip uses a custom API (NxSDK and later Lava). Lava is a Python framework that can integrate with PyTorch to some extent (for training SNNs and then deploying to Loihi). IBM’s TrueNorth had Compass simulator with its own workflow. None of these were integrated at the runtime level into TF/PyTorch directly; instead they require exporting models. Our approach, by integrating at runtime, is somewhat pioneering. However, there are efforts like NengoDL which allows simulating SNNs with TensorFlow, and snnTorch which is a PyTorch-based library for spiking networks. Those operate at a higher abstraction (they simulate neuromorphic behavior on CPU/GPU). In contrast, we are connecting actual neuromorphic hardware as a backend. It’s complementary: one could use snnTorch to train a spiking model in PyTorch, then switch the device to Neuromorph for inference – the code would remain in PyTorch, but now execute on real neuromorphic hardware.
Pluggable Devices in the Wild: The PluggableDevice mechanism in TF is quite new (TF 2.5, ~2021). One example is the tensorflow-metal plugin for Mac GPUs by Apple, which uses METAL API under the hood to run TF ops on Apple GPUs. Another is DirectML for Windows GPUs (by Microsoft), which plugs into TF as well
reddit.com
. These examples show that with a proper plugin, TensorFlow can run on very different hardware via a common interface. Neuromorph would be one of the more unusual ones to attempt, but if we implement the interface, it should work. The Nature paper (2025) specifically points out “Inspiration from uptake of tensor processors (GPUs/TPUs) can help neuromorphic tech overcome hurdles… Coupled with high-level APIs (TensorFlow/Keras, PyTorch), neuromorphic processors become accessible to developers”
nature.com
nature.com
. This is exactly the motivation behind our integration plan: make Neuromorph accessible through the same APIs ML developers use, increasing its appeal and ease of use.
AWS Neuron: As detailed earlier, AWS’s Neuron integration compiles models for their Inferentia ASIC and integrates with TensorFlow (as a runtime where you allocate the device and run sessions on it) and PyTorch (via a custom torch_neuron that leverages TorchScript)
docs.aws.amazon.com
. One key takeaway is that AWS did not attempt to implement every operation on Inferentia. They rely on capturing a whole model and using a compiler to support what the hardware can do. We may find ourselves doing something similar: for example, if Neuromorph excels at SNN convolution but cannot easily do a softmax, we might leave softmax on CPU or implement an approximate softmax on the neuromorphic side (maybe by a winner-take-all circuit). Our driver could potentially split execution: run most of the model on Neuromorph, then bring a small part back to CPU if needed. This hybrid execution needs careful handling to not confuse the frameworks (which typically expect an op either fully on one device or another, with explicit copy ops in between).
SYCL and oneAPI in PyTorch: Intel’s GPUs have become accessible in PyTorch by treating them as a new device type (device=‘xpu’ or such in recent versions), backed by the oneAPI Level Zero driver. That integration was done by contributing code to PyTorch (since it’s in-tree for oneAPI devices). For our out-of-tree approach, we follow a similar path but without modifying PyTorch source – thanks to PrivateUse dispatch, which is essentially a placeholder for any custom device.
In conclusion, this plan brings together a comprehensive architecture for Neuromorph drivers in Rust, covering low-level interactions up to high-level framework integration. The focus is on maximal compatibility – making Neuromorph appear as much like a “GPU” to the software as feasible – while respecting the unique aspects of the neuromorphic model. By leveraging Rust, we gain safety and modern concurrency in implementing these drivers. By interfacing with PyTorch and TensorFlow through their extension mechanisms, we ensure Neuromorph can be used with minimal code changes in existing ML projects. This lowers the barrier for researchers and developers to experiment with neuromorphic acceleration, using familiar tools but gaining the benefits (energy efficiency, perhaps speed on spiking workloads) of Neuromorph. Ultimately, the success of such an integration will be measured by how seamlessly one can train or run a model on a standard ML framework and have it execute on Neuromorph. With the design outlined above, and iterating through the challenges, we aim to achieve a result where switching to Neuromorph is almost as simple as switching device="cuda:0" to device="neuromorph:0". This would be a significant step towards bringing neuromorphic hardware into the mainstream of AI development
nature.com
nature.com
, following the example set by GPUs and TPUs in previous years.






Источники
Вы сказали:
Make step-by-step tech requirements to develop   cuda-style drivers to access Neuromorph from standard ML libraries (Pytorch, TensorFlow, etc)
ChatGPT сказал:

Думал на протяжении 1m 29s
Below is a stage-by-stage technical requirements checklist you can use as a master backlog while turning Neuromorph into a “CUDA-style” accelerator that PyTorch and TensorFlow can target. Follow these steps in order; each builds on the previous one, and every deliverable should come with regression tests and CI gates before you move forward.

1. Foundational research & prerequisites
1.1 Read the CUDA driver/runtime docs to internalise core concepts—contexts, streams, events, mem-alloc, kernel launch, error codes, properties, etc. 
docs.nvidia.com

1.2 Study PyTorch’s PrivateUse1 and dispatcher tutorials to understand how an out-of-tree backend plugs into the operator registry. 
raw.githubusercontent.com
raw.githubusercontent.com

1.3 Study TensorFlow’s PluggableDevice + StreamExecutor C API RFCs to see exactly which callbacks you must implement (device discovery, stream, memcpy, etc.). 
raw.githubusercontent.com

1.4 Survey existing examples that solved similar problems (Graphcore PopTorch, AWS Neuron, TF-DirectML) so you can reuse patterns and avoid gotchas. 
docs.graphcore.ai
raw.githubusercontent.com
raw.githubusercontent.com

2. Hardware-abstraction layer (HAL) in Rust
+2.1 Create neuromorph-sys: unsafe FFI bindings to the board/kernel driver or simulator.
+2.2 Expose register access, DMA queues and any irq/event mechanism as thin Rust functions.
2.3 Mirror CUDA’s driver object model: Context, Stream, Event, DeviceMemory, etc.; each wraps a raw handle and implements Drop to release resources automatically.
2.4 Base errors on a small enum NeuromorphError that maps cleanly to numeric C codes (0 = success; negative = recoverable; positive = fatal).

3. Core safe-Rust driver library
3.1 neuromorph crate: safe façade over HAL with RAII types and Result<T, NeuromorphError>.
3.2 Integrate a GPU-style allocator (gpu-allocator or gpu-alloc) to manage on-device RAM and sub-allocation. 
raw.githubusercontent.com

3.3 Implement async command queues: each Stream owns a Rust thread or tokio task that pops requests and drives the device (gives CUDA-like overlap semantics even if Neuromorph runs serially).
3.4 Provide blocking and non-blocking memcpy APIs so higher layers can call neuromorphMemcpyAsync with a Stream*.
3.5 Design a “kernel” concept: load/instantiate an SNN graph (or converted DNN) and launch it for N ticks or a batch—return immediately, signal via Event.

4. Stable C ABI for external frameworks
4.1 Export driver calls with extern "C"; keep the symbol names parallel to CUDA where sensible (neuromorphInit, neuromorphMalloc, neuromorphLaunchKernel, …).
4.2 Use cbindgen to autogenerate a single header so C/C++ callers stay in sync. 
github.com

4.3 Catch panics with std::panic::catch_unwind and translate them to NEUROMORPH_INTERNAL_ERROR.

5. PyTorch backend deliverables
5.1 Allocate one of the three reserved dispatch keys and rename it “neuromorph”. 
raw.githubusercontent.com

5.2 C++ extension (torch_neuromorph.so):
  • Wrap each required ATen operator with a thin C++ → C call into the Rust ABI.
  • Register kernels via TORCH_LIBRARY_IMPL(aten, PrivateUse1, m); provide a CPU fallback to keep models running when an op isn’t implemented. 
raw.githubusercontent.com
raw.githubusercontent.com

5.3 Implement the device guard, generator and AMP hooks; PyTorch’s tutorial shows the macros (C10_REGISTER_GUARD_IMPL, REGISTER_GENERATOR_PRIVATEUSE1). 
raw.githubusercontent.com

5.4 Add a lightweight graph-compiler path under torch.compile for performance, but keep eager-mode kernels for maximum transparency.

6. TensorFlow plugin deliverables
6.1 Write a shared library that implements the StreamExecutor C API struct table (memory, stream, event, timer). 
raw.githubusercontent.com

6.2 Register "NEUROMORPH" devices in TF_InitPlugin, returning one executor per physical board.
6.3 Provide kernels for the op subset you support; TensorFlow will auto-copy tensors to CPU when it hits an unsupported op.
6.4 Borrow layout from the DirectML plugin for build and wheel packaging. 
raw.githubusercontent.com

7. Model-conversion toolchain
7.1 Add a small Rust or Python CLI that converts standard DNN-weight tensors into time-coded SNN weight tables understood by Neuromorph (reuse existing SNN conversion research and libraries such as snnTorch or Norse for reference). 
github.com

7.2 Emit a self-contained binary “kernel module” that the driver can load via neuromorphLoadModule.

8. Testing, validation & CI
8.1 Unit-test every Rust safe wrapper (alloc, free, stream sync) with an in-process simulator.
8.2 Integration tests:
  • PyTorch: run pytest on a CNN unit test list; compare outputs to CPU within tolerance.
  • TensorFlow: run the Keras MNIST example on "NEUROMORPH:0"; verify identical predictions.
8.3 Continuous integration matrix: Linux x86-64, nightly Rust, PyTorch LTS, TF 2.y, plus optional --features=simulator build that doesn’t need hardware.

9. Packaging & distribution
9.1 Cargo‐generated cdylib for the Rust driver.
9.2 Python wheels: use maturin for PyPI (pip install neuromorph-torch / neuromorph-tf).
9.3 System packages: produce .deb/.rpm that drop the udev rules, kernel module and /usr/lib/libneuromorph.so.

10. Documentation & developer kit
10.1 Driver reference manual with the full C API and error codes (use CUDA doc style).
10.2 Tutorial notebooks:
  • “Run ResNet-18 on Neuromorph with PyTorch in five lines”
  • “Deploy a Keras model to Neuromorph via TF plugin”
10.3 Design white-paper comparing Neuromorph SNN runtime to GPU FP16 throughput; cite Loihi/NxSDK and PopTorch experiences for context. 
arxiv.org
docs.graphcore.ai

11. Future roadmap (post-MVP)
Mixed-precision training with surrogate-gradient back-prop directly on Neuromorph.

Multi-device DataParallel & NCCL-style collective ops.

TFRT / PyTorch 3 back-end migration once those runtimes stabilise.

On-chip profiling interface (similar to CUDA Events + Nsight) for power and latency.

With this checklist you have a clear, ordered backlog: finish the HAL, wrap it safely in Rust, publish a C ABI, then plug that ABI into PyTorch’s PrivateUse1 path and TensorFlow’s PluggableDevice path. Each milestone is testable and shippable—once you tick them all off, device="neuromorph:0" will feel as natural as cuda:0.




No file chosenNo file chosen
ChatGPT может допускать ошибки. Проверьте важную информацию. См. настройки cookie-файлов.
