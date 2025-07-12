//! Unsafe FFI bindings for Neuromorph neuromorphic processor driver
//! 
//! This crate provides low-level, unsafe bindings to the Neuromorph hardware
//! or simulator, mirroring CUDA's driver API structure.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::{c_int, c_void, c_uint};

// Re-export modules
pub mod error;
pub mod types;
pub mod bindings;

// Re-export commonly used types
pub use error::*;
pub use types::*;
pub use bindings::*;

/// Initialize the Neuromorph driver
/// 
/// # Safety
/// This function must be called before any other neuromorph functions.
/// It initializes the underlying hardware or simulator.
pub unsafe fn neuromorphInit() -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphInit()
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphInit()
    }
}

/// Get the number of available Neuromorph devices
/// 
/// # Safety
/// Must be called after neuromorphInit()
pub unsafe fn neuromorphGetDeviceCount(count: *mut c_int) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphGetDeviceCount(count)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphGetDeviceCount(count)
    }
}

/// Get device properties
/// 
/// # Safety
/// Device must be a valid device ordinal
pub unsafe fn neuromorphGetDeviceProperties(
    prop: *mut NeuromorphDeviceProperties,
    device: c_int
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphGetDeviceProperties(prop, device)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphGetDeviceProperties(prop, device)
    }
}

/// Create a Neuromorph context for the specified device
/// 
/// # Safety
/// Device must be a valid device ordinal
pub unsafe fn neuromorphCtxCreate(
    pctx: *mut NeuromorphContext,
    flags: c_uint,
    device: c_int
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphCtxCreate(pctx, flags, device)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphCtxCreate(pctx, flags, device)
    }
}

/// Destroy a Neuromorph context
/// 
/// # Safety
/// Context must be a valid context handle
pub unsafe fn neuromorphCtxDestroy(ctx: NeuromorphContext) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphCtxDestroy(ctx)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphCtxDestroy(ctx)
    }
}

/// Allocate device memory
/// 
/// # Safety
/// Size must be non-zero, devPtr must be a valid pointer
pub unsafe fn neuromorphMalloc(devPtr: *mut NeuromorphDevicePtr, size: usize) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphMalloc(devPtr, size)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphMalloc(devPtr, size)
    }
}

/// Free device memory
/// 
/// # Safety
/// devPtr must be a valid device pointer allocated with neuromorphMalloc
pub unsafe fn neuromorphFree(devPtr: NeuromorphDevicePtr) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphFree(devPtr)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphFree(devPtr)
    }
}

/// Copy memory synchronously
/// 
/// # Safety
/// dst and src must be valid pointers, count must be valid size
pub unsafe fn neuromorphMemcpy(
    dst: *mut c_void,
    src: *const c_void,
    count: usize,
    kind: NeuromorphMemcpyKind
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphMemcpy(dst, src, count, kind)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphMemcpy(dst, src, count, kind)
    }
}

/// Copy memory asynchronously
/// 
/// # Safety
/// dst and src must be valid pointers, stream must be valid
pub unsafe fn neuromorphMemcpyAsync(
    dst: *mut c_void,
    src: *const c_void,
    count: usize,
    kind: NeuromorphMemcpyKind,
    stream: NeuromorphStream
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphMemcpyAsync(dst, src, count, kind, stream)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphMemcpyAsync(dst, src, count, kind, stream)
    }
}

/// Create a stream
/// 
/// # Safety
/// phStream must be a valid pointer
pub unsafe fn neuromorphStreamCreate(phStream: *mut NeuromorphStream) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphStreamCreate(phStream)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphStreamCreate(phStream)
    }
}

/// Destroy a stream
/// 
/// # Safety
/// hStream must be a valid stream handle
pub unsafe fn neuromorphStreamDestroy(hStream: NeuromorphStream) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphStreamDestroy(hStream)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphStreamDestroy(hStream)
    }
}

/// Synchronize with a stream
/// 
/// # Safety
/// hStream must be a valid stream handle
pub unsafe fn neuromorphStreamSynchronize(hStream: NeuromorphStream) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphStreamSynchronize(hStream)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphStreamSynchronize(hStream)
    }
}

/// Create an event
/// 
/// # Safety
/// phEvent must be a valid pointer
pub unsafe fn neuromorphEventCreate(phEvent: *mut NeuromorphEvent) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphEventCreate(phEvent)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphEventCreate(phEvent)
    }
}

/// Destroy an event
/// 
/// # Safety
/// hEvent must be a valid event handle
pub unsafe fn neuromorphEventDestroy(hEvent: NeuromorphEvent) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphEventDestroy(hEvent)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphEventDestroy(hEvent)
    }
}

/// Record an event in a stream
/// 
/// # Safety
/// hEvent and hStream must be valid handles
pub unsafe fn neuromorphEventRecord(
    hEvent: NeuromorphEvent,
    hStream: NeuromorphStream
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphEventRecord(hEvent, hStream)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphEventRecord(hEvent, hStream)
    }
}

/// Wait for an event to complete
/// 
/// # Safety
/// hEvent must be a valid event handle
pub unsafe fn neuromorphEventSynchronize(hEvent: NeuromorphEvent) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphEventSynchronize(hEvent)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphEventSynchronize(hEvent)
    }
}

/// Launch a neuromorphic kernel
/// 
/// # Safety
/// All parameters must be valid
pub unsafe fn neuromorphLaunchKernel(
    kernel: NeuromorphKernel,
    grid_dim: NeuromorphDim3,
    block_dim: NeuromorphDim3,
    args: *mut *mut c_void,
    shared_mem: usize,
    stream: NeuromorphStream
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphLaunchKernel(kernel, grid_dim, block_dim, args, shared_mem, stream)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphLaunchKernel(kernel, grid_dim, block_dim, args, shared_mem, stream)
    }
}

// Platform-specific implementations
#[cfg(feature = "simulator")]
mod simulator;

#[cfg(feature = "hardware")]
mod hardware;
