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

// Register access functions

/// Read from a device register
///
/// # Safety
/// Device must be valid, value must be a valid pointer
pub unsafe fn neuromorphRegisterRead(
    device: NeuromorphDevice,
    register_offset: c_uint,
    value: *mut c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphRegisterRead(device, register_offset, value)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphRegisterRead(device, register_offset, value)
    }
}

/// Write to a device register
///
/// # Safety
/// Device must be valid
pub unsafe fn neuromorphRegisterWrite(
    device: NeuromorphDevice,
    register_offset: c_uint,
    value: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphRegisterWrite(device, register_offset, value)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphRegisterWrite(device, register_offset, value)
    }
}

/// Read multiple device registers in batch
///
/// # Safety
/// Device must be valid, arrays must be valid and count must match
pub unsafe fn neuromorphRegisterReadBatch(
    device: NeuromorphDevice,
    register_offsets: *const c_uint,
    values: *mut c_uint,
    count: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphRegisterReadBatch(device, register_offsets, values, count)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphRegisterReadBatch(device, register_offsets, values, count)
    }
}

/// Write multiple device registers in batch
///
/// # Safety
/// Device must be valid, arrays must be valid and count must match
pub unsafe fn neuromorphRegisterWriteBatch(
    device: NeuromorphDevice,
    register_offsets: *const c_uint,
    values: *const c_uint,
    count: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphRegisterWriteBatch(device, register_offsets, values, count)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphRegisterWriteBatch(device, register_offsets, values, count)
    }
}

// DMA queue functions

/// Create a DMA queue for the device
///
/// # Safety
/// Device must be valid, queue must be a valid pointer
pub unsafe fn neuromorphDmaQueueCreate(
    device: NeuromorphDevice,
    queue: *mut *mut c_void,
    priority: c_int
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphDmaQueueCreate(device, queue, priority)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphDmaQueueCreate(device, queue, priority)
    }
}

/// Destroy a DMA queue
///
/// # Safety
/// Queue must be a valid DMA queue handle
pub unsafe fn neuromorphDmaQueueDestroy(queue: *mut c_void) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphDmaQueueDestroy(queue)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphDmaQueueDestroy(queue)
    }
}

/// Submit a DMA transfer
///
/// # Safety
/// Queue must be valid, pointers must be valid, transfer_id must be a valid pointer
pub unsafe fn neuromorphDmaSubmit(
    queue: *mut c_void,
    dst: *mut c_void,
    src: *const c_void,
    size: usize,
    transfer_id: *mut c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphDmaSubmit(queue, dst, src, size, transfer_id)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphDmaSubmit(queue, dst, src, size, transfer_id)
    }
}

/// Wait for DMA transfer completion
///
/// # Safety
/// Queue must be valid
pub unsafe fn neuromorphDmaWait(
    queue: *mut c_void,
    transfer_id: c_uint,
    timeout_ms: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphDmaWait(queue, transfer_id, timeout_ms)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphDmaWait(queue, transfer_id, timeout_ms)
    }
}

/// Query DMA transfer status
///
/// # Safety
/// Queue must be valid, completed must be a valid pointer
pub unsafe fn neuromorphDmaQuery(
    queue: *mut c_void,
    transfer_id: c_uint,
    completed: *mut c_int
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphDmaQuery(queue, transfer_id, completed)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphDmaQuery(queue, transfer_id, completed)
    }
}

// IRQ functions

/// Register an interrupt handler
///
/// # Safety
/// Device must be valid, handler must be a valid function pointer
pub unsafe fn neuromorphIrqRegister(
    device: NeuromorphDevice,
    irq_mask: c_uint,
    handler: NeuromorphIrqHandler,
    user_data: *mut c_void
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphIrqRegister(device, irq_mask, handler, user_data)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphIrqRegister(device, irq_mask, handler, user_data)
    }
}

/// Unregister an interrupt handler
///
/// # Safety
/// Device must be valid
pub unsafe fn neuromorphIrqUnregister(
    device: NeuromorphDevice,
    irq_mask: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphIrqUnregister(device, irq_mask)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphIrqUnregister(device, irq_mask)
    }
}

/// Enable interrupts
///
/// # Safety
/// Device must be valid
pub unsafe fn neuromorphIrqEnable(
    device: NeuromorphDevice,
    irq_mask: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphIrqEnable(device, irq_mask)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphIrqEnable(device, irq_mask)
    }
}

/// Disable interrupts
///
/// # Safety
/// Device must be valid
pub unsafe fn neuromorphIrqDisable(
    device: NeuromorphDevice,
    irq_mask: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphIrqDisable(device, irq_mask)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphIrqDisable(device, irq_mask)
    }
}

/// Get interrupt status
///
/// # Safety
/// Device must be valid, status must be a valid pointer
pub unsafe fn neuromorphIrqGetStatus(
    device: NeuromorphDevice,
    status: *mut c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphIrqGetStatus(device, status)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphIrqGetStatus(device, status)
    }
}

/// Clear interrupt status
///
/// # Safety
/// Device must be valid
pub unsafe fn neuromorphIrqClear(
    device: NeuromorphDevice,
    irq_mask: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphIrqClear(device, irq_mask)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphIrqClear(device, irq_mask)
    }
}

// Hardware event functions

/// Create a hardware event
///
/// # Safety
/// Device must be valid, event must be a valid pointer
pub unsafe fn neuromorphHwEventCreate(
    device: NeuromorphDevice,
    event: *mut *mut c_void,
    auto_reset: c_int
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphHwEventCreate(device, event, auto_reset)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphHwEventCreate(device, event, auto_reset)
    }
}

/// Destroy a hardware event
///
/// # Safety
/// Event must be a valid hardware event handle
pub unsafe fn neuromorphHwEventDestroy(event: *mut c_void) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphHwEventDestroy(event)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphHwEventDestroy(event)
    }
}

/// Signal a hardware event
///
/// # Safety
/// Event must be a valid hardware event handle
pub unsafe fn neuromorphHwEventSignal(event: *mut c_void) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphHwEventSignal(event)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphHwEventSignal(event)
    }
}

/// Wait for a hardware event
///
/// # Safety
/// Event must be a valid hardware event handle
pub unsafe fn neuromorphHwEventWait(
    event: *mut c_void,
    timeout_ms: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphHwEventWait(event, timeout_ms)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphHwEventWait(event, timeout_ms)
    }
}

/// Reset a hardware event
///
/// # Safety
/// Event must be a valid hardware event handle
pub unsafe fn neuromorphHwEventReset(event: *mut c_void) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphHwEventReset(event)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphHwEventReset(event)
    }
}

// Memory mapping functions

/// Map device memory to host address space
///
/// # Safety
/// host_ptr and device_ptr must be valid pointers
pub unsafe fn neuromorphMemMap(
    host_ptr: *mut *mut c_void,
    device_ptr: NeuromorphDevicePtr,
    size: usize,
    flags: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphMemMap(host_ptr, device_ptr, size, flags)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphMemMap(host_ptr, device_ptr, size, flags)
    }
}

/// Unmap device memory from host address space
///
/// # Safety
/// host_ptr must be a valid mapped pointer
pub unsafe fn neuromorphMemUnmap(
    host_ptr: *mut c_void,
    size: usize
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphMemUnmap(host_ptr, size)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphMemUnmap(host_ptr, size)
    }
}

// Power management functions

/// Set device power state
///
/// # Safety
/// Device must be valid
pub unsafe fn neuromorphPowerSetState(
    device: NeuromorphDevice,
    power_state: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphPowerSetState(device, power_state)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphPowerSetState(device, power_state)
    }
}

/// Get device power state
///
/// # Safety
/// Device must be valid, power_state must be a valid pointer
pub unsafe fn neuromorphPowerGetState(
    device: NeuromorphDevice,
    power_state: *mut c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        simulator::neuromorphPowerGetState(device, power_state)
    }
    #[cfg(feature = "hardware")]
    {
        hardware::neuromorphPowerGetState(device, power_state)
    }
}

// Platform-specific implementations
#[cfg(feature = "simulator")]
mod simulator;

#[cfg(feature = "hardware")]
mod hardware;
