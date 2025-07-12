# neuromorph-sys

Unsafe FFI bindings for the Neuromorph neuromorphic processor driver.

This crate provides low-level, unsafe bindings to the Neuromorph hardware or simulator, mirroring CUDA's driver API structure for compatibility with existing ML frameworks.

## Features

### 2.1 ✅ Unsafe FFI bindings to board/kernel driver or simulator

- Complete FFI interface with C-compatible function signatures
- Simulator backend for development and testing
- Hardware backend stub for future real hardware integration

### 2.2 ✅ Register access, DMA queues and IRQ/event mechanisms

- Direct register read/write functions with batch operations
- DMA queue management for asynchronous memory transfers
- IRQ registration and handling with configurable masks
- Hardware event creation and synchronization

### 2.3 ✅ CUDA-style driver object model

- **Context**: Device context management with RAII
- **Stream**: Asynchronous command streams for overlap
- **Event**: Synchronization primitives with timestamps
- **DeviceMemory**: GPU-style memory allocation and management
- All objects implement automatic resource cleanup via Drop

### 2.4 ✅ Clean error mapping to C codes

- `NeuromorphError` enum with comprehensive error types
- 0 = success, negative = recoverable, positive = fatal
- Automatic conversion between Rust enums and C integer codes
- Descriptive error messages for debugging

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
neuromorph-sys = { path = "../neuromorph-sys", features = ["simulator"] }
```

### Basic Example

```rust
use neuromorph_sys::*;
use std::ptr;

unsafe {
    // Initialize driver
    assert_eq!(neuromorphInit(), NEUROMORPH_SUCCESS);

    // Get device count
    let mut count = 0;
    neuromorphGetDeviceCount(&mut count);
    println!("Found {} devices", count);

    // Create context
    let mut ctx = ptr::null_mut();
    neuromorphCtxCreate(&mut ctx, NEUROMORPH_CTX_SCHED_AUTO, 0);

    // Allocate memory
    let mut dev_ptr = ptr::null_mut();
    neuromorphMalloc(&mut dev_ptr, 1024);

    // Clean up
    neuromorphFree(dev_ptr);
    neuromorphCtxDestroy(ctx);
}
```

## Architecture

```
neuromorph-sys/
├── src/
│   ├── lib.rs          # Main FFI interface
│   ├── error.rs        # Error types and C code mapping
│   ├── types.rs        # CUDA-style type definitions
│   ├── bindings.rs     # Register/DMA/IRQ function bindings
│   ├── simulator.rs    # Software simulation backend
│   └── hardware.rs     # Hardware driver backend (stub)
├── examples/
│   └── basic_usage.rs  # Usage demonstration
├── tests/
│   └── integration_tests.rs  # FFI integration tests
└── build.rs            # Build script for C header generation
```

## Features

- **`simulator`** (default): Use software simulation backend
- **`hardware`**: Use real hardware backend (requires hardware drivers)

## API Overview

### Core Driver Functions

- `neuromorphInit()` - Initialize driver
- `neuromorphGetDeviceCount()` - Query available devices
- `neuromorphGetDeviceProperties()` - Get device capabilities

### Context Management

- `neuromorphCtxCreate()` - Create device context
- `neuromorphCtxDestroy()` - Destroy context

### Memory Management

- `neuromorphMalloc()` - Allocate device memory
- `neuromorphFree()` - Free device memory
- `neuromorphMemcpy()` - Synchronous memory copy
- `neuromorphMemcpyAsync()` - Asynchronous memory copy

### Stream Operations

- `neuromorphStreamCreate()` - Create command stream
- `neuromorphStreamDestroy()` - Destroy stream
- `neuromorphStreamSynchronize()` - Wait for stream completion

### Event Synchronization

- `neuromorphEventCreate()` - Create synchronization event
- `neuromorphEventDestroy()` - Destroy event
- `neuromorphEventRecord()` - Record event in stream
- `neuromorphEventSynchronize()` - Wait for event

### Hardware Access

- `neuromorphRegisterRead/Write()` - Direct register access
- `neuromorphDmaSubmit/Wait()` - DMA queue operations
- `neuromorphIrqRegister/Enable()` - Interrupt management
- `neuromorphMemMap/Unmap()` - Memory mapping
- `neuromorphPowerSetState()` - Power management

## Error Handling

All functions return `NeuromorphResult` (i32) following CUDA conventions:

- `0` = Success
- `< 0` = Recoverable errors (retry possible)
- `> 0` = Fatal errors (requires restart)

```rust
match neuromorphMalloc(&mut ptr, size) {
    NEUROMORPH_SUCCESS => println!("Success!"),
    NEUROMORPH_ERROR_OUT_OF_MEMORY => println!("Retry with smaller size"),
    code if code > 0 => println!("Fatal error: {}", code),
    code => println!("Recoverable error: {}", code),
}
```

## Thread Safety

This crate provides unsafe FFI bindings. Thread safety must be ensured by:

- Proper synchronization when sharing contexts/streams between threads
- Using separate contexts per thread when possible
- Protecting shared resources with mutexes at higher levels

## Next Steps

This crate implements step 2.1-2.4 of the neuromorphic driver roadmap. Next phases:

1. **Safe Rust wrapper** (`neuromorph` crate) with RAII types
2. **PyTorch integration** via PrivateUse1 dispatch
3. **TensorFlow plugin** via PluggableDevice API
4. **Model conversion toolchain** for SNN deployment

## Testing

Run tests with different backends:

```bash
# Test with simulator (default)
cargo test

# Test hardware backend (requires hardware)
cargo test --no-default-features --features hardware

# Run examples
cargo run --example basic_usage
```

## Build

The crate automatically generates:

- C header file (`neuromorph.h`) for external integration
- Hardware bindings (if hardware headers available)
- Documentation and tests

```bash
cargo build    # Build with simulator
cargo doc      # Generate documentation
cargo test     # Run test suite
```
