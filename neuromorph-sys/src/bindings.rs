//! C-style function bindings for neuromorph driver
//! 
//! These functions expose register access, DMA queues, and IRQ/event mechanisms
//! as thin Rust functions that can be called from C code.

use std::os::raw::{c_int, c_uint, c_void};
use crate::types::*;
use crate::error::*;

/// Register access functions - expose direct hardware register manipulation

/// Read from a device register
/// 
/// # Safety
/// device must be valid, register_offset must be within valid range
pub unsafe fn neuromorphRegisterRead(
    device: NeuromorphDevice,
    register_offset: c_uint,
    value: *mut c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphRegisterRead(device, register_offset, value)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphRegisterRead(device, register_offset, value)
    }
}

/// Write to a device register
/// 
/// # Safety
/// device must be valid, register_offset must be within valid range
pub unsafe fn neuromorphRegisterWrite(
    device: NeuromorphDevice,
    register_offset: c_uint,
    value: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphRegisterWrite(device, register_offset, value)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphRegisterWrite(device, register_offset, value)
    }
}

/// Read from multiple registers atomically
/// 
/// # Safety
/// All pointers must be valid, count must match array sizes
pub unsafe fn neuromorphRegisterReadBatch(
    device: NeuromorphDevice,
    register_offsets: *const c_uint,
    values: *mut c_uint,
    count: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphRegisterReadBatch(device, register_offsets, values, count)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphRegisterReadBatch(device, register_offsets, values, count)
    }
}

/// Write to multiple registers atomically
/// 
/// # Safety
/// All pointers must be valid, count must match array sizes
pub unsafe fn neuromorphRegisterWriteBatch(
    device: NeuromorphDevice,
    register_offsets: *const c_uint,
    values: *const c_uint,
    count: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphRegisterWriteBatch(device, register_offsets, values, count)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphRegisterWriteBatch(device, register_offsets, values, count)
    }
}

/// DMA queue management functions

/// Create a DMA queue for asynchronous memory transfers
/// 
/// # Safety
/// queue must be a valid pointer
pub unsafe fn neuromorphDmaQueueCreate(
    device: NeuromorphDevice,
    queue: *mut *mut c_void,
    priority: c_int
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphDmaQueueCreate(device, queue, priority)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphDmaQueueCreate(device, queue, priority)
    }
}

/// Destroy a DMA queue
/// 
/// # Safety
/// queue must be a valid DMA queue handle
pub unsafe fn neuromorphDmaQueueDestroy(queue: *mut c_void) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphDmaQueueDestroy(queue)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphDmaQueueDestroy(queue)
    }
}

/// Submit a DMA transfer to the queue
/// 
/// # Safety
/// All pointers must be valid, src and dst must be valid for size bytes
pub unsafe fn neuromorphDmaSubmit(
    queue: *mut c_void,
    dst: *mut c_void,
    src: *const c_void,
    size: usize,
    transfer_id: *mut c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphDmaSubmit(queue, dst, src, size, transfer_id)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphDmaSubmit(queue, dst, src, size, transfer_id)
    }
}

/// Wait for a DMA transfer to complete
/// 
/// # Safety
/// queue must be valid, transfer_id must be from a previous neuromorphDmaSubmit call
pub unsafe fn neuromorphDmaWait(
    queue: *mut c_void,
    transfer_id: c_uint,
    timeout_ms: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphDmaWait(queue, transfer_id, timeout_ms)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphDmaWait(queue, transfer_id, timeout_ms)
    }
}

/// Check if a DMA transfer is complete (non-blocking)
/// 
/// # Safety
/// queue must be valid, transfer_id must be from a previous neuromorphDmaSubmit call
pub unsafe fn neuromorphDmaQuery(
    queue: *mut c_void,
    transfer_id: c_uint,
    completed: *mut c_int
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphDmaQuery(queue, transfer_id, completed)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphDmaQuery(queue, transfer_id, completed)
    }
}

/// IRQ/Event mechanism functions

/// Interrupt handler function type
pub type NeuromorphIrqHandler = unsafe extern "C" fn(
    device: NeuromorphDevice,
    irq_source: c_uint,
    user_data: *mut c_void
);

/// Register an interrupt handler
/// 
/// # Safety
/// handler must be a valid function pointer, user_data lifetime must exceed the registration
pub unsafe fn neuromorphIrqRegister(
    device: NeuromorphDevice,
    irq_mask: c_uint,
    handler: NeuromorphIrqHandler,
    user_data: *mut c_void
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphIrqRegister(device, irq_mask, handler, user_data)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphIrqRegister(device, irq_mask, handler, user_data)
    }
}

/// Unregister an interrupt handler
/// 
/// # Safety
/// device must be valid, irq_mask must match a previous registration
pub unsafe fn neuromorphIrqUnregister(
    device: NeuromorphDevice,
    irq_mask: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphIrqUnregister(device, irq_mask)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphIrqUnregister(device, irq_mask)
    }
}

/// Enable interrupts for specified sources
/// 
/// # Safety
/// device must be valid
pub unsafe fn neuromorphIrqEnable(
    device: NeuromorphDevice,
    irq_mask: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphIrqEnable(device, irq_mask)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphIrqEnable(device, irq_mask)
    }
}

/// Disable interrupts for specified sources
/// 
/// # Safety
/// device must be valid
pub unsafe fn neuromorphIrqDisable(
    device: NeuromorphDevice,
    irq_mask: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphIrqDisable(device, irq_mask)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphIrqDisable(device, irq_mask)
    }
}

/// Get pending interrupt status
/// 
/// # Safety
/// device must be valid, status must be a valid pointer
pub unsafe fn neuromorphIrqGetStatus(
    device: NeuromorphDevice,
    status: *mut c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphIrqGetStatus(device, status)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphIrqGetStatus(device, status)
    }
}

/// Clear interrupt status
/// 
/// # Safety
/// device must be valid
pub unsafe fn neuromorphIrqClear(
    device: NeuromorphDevice,
    irq_mask: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphIrqClear(device, irq_mask)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphIrqClear(device, irq_mask)
    }
}

/// Hardware event management

/// Create a hardware event object
/// 
/// # Safety
/// event must be a valid pointer
pub unsafe fn neuromorphHwEventCreate(
    device: NeuromorphDevice,
    event: *mut *mut c_void,
    auto_reset: c_int
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphHwEventCreate(device, event, auto_reset)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphHwEventCreate(device, event, auto_reset)
    }
}

/// Destroy a hardware event object
/// 
/// # Safety
/// event must be a valid hardware event handle
pub unsafe fn neuromorphHwEventDestroy(event: *mut c_void) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphHwEventDestroy(event)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphHwEventDestroy(event)
    }
}

/// Signal a hardware event
/// 
/// # Safety
/// event must be a valid hardware event handle
pub unsafe fn neuromorphHwEventSignal(event: *mut c_void) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphHwEventSignal(event)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphHwEventSignal(event)
    }
}

/// Wait for a hardware event to be signaled
/// 
/// # Safety
/// event must be a valid hardware event handle
pub unsafe fn neuromorphHwEventWait(
    event: *mut c_void,
    timeout_ms: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphHwEventWait(event, timeout_ms)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphHwEventWait(event, timeout_ms)
    }
}

/// Reset a hardware event to unsignaled state
/// 
/// # Safety
/// event must be a valid hardware event handle
pub unsafe fn neuromorphHwEventReset(event: *mut c_void) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphHwEventReset(event)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphHwEventReset(event)
    }
}

/// Memory mapping functions for direct hardware access

/// Map device memory to host virtual address space
/// 
/// # Safety
/// All parameters must be valid, mapped memory must be unmapped when done
pub unsafe fn neuromorphMemMap(
    host_ptr: *mut *mut c_void,
    device_ptr: NeuromorphDevicePtr,
    size: usize,
    flags: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphMemMap(host_ptr, device_ptr, size, flags)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphMemMap(host_ptr, device_ptr, size, flags)
    }
}

/// Unmap previously mapped device memory
///
/// # Safety
/// host_ptr must be from a previous neuromorphMemMap call
pub unsafe fn neuromorphMemUnmap(
    host_ptr: *mut c_void,
    size: usize
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphMemUnmap(host_ptr, size)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphMemUnmap(host_ptr, size)
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
        crate::simulator::neuromorphLaunchKernel(kernel, grid_dim, block_dim, args, shared_mem, stream)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphLaunchKernel(kernel, grid_dim, block_dim, args, shared_mem, stream)
    }
}

/// Load a neuromorphic graph/kernel from binary data
///
/// # Safety
/// data must be a valid pointer to graph binary data, size must be correct
pub unsafe fn neuromorphGraphLoad(
    graph: *mut NeuromorphKernel,
    data: *const c_void,
    size: usize
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphGraphLoad(graph, data, size)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphGraphLoad(graph, data, size)
    }
}

/// Unload a neuromorphic graph/kernel
///
/// # Safety
/// graph must be a valid kernel handle
pub unsafe fn neuromorphGraphUnload(graph: NeuromorphKernel) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphGraphUnload(graph)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphGraphUnload(graph)
    }
}

/// Power management functions

/// Set device power state
/// 
/// # Safety
/// device must be valid
pub unsafe fn neuromorphPowerSetState(
    device: NeuromorphDevice,
    power_state: c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphPowerSetState(device, power_state)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphPowerSetState(device, power_state)
    }
}

/// Get device power state
/// 
/// # Safety
/// device must be valid, power_state must be a valid pointer
pub unsafe fn neuromorphPowerGetState(
    device: NeuromorphDevice,
    power_state: *mut c_uint
) -> NeuromorphResult {
    #[cfg(feature = "simulator")]
    {
        crate::simulator::neuromorphPowerGetState(device, power_state)
    }
    #[cfg(feature = "hardware")]
    {
        crate::hardware::neuromorphPowerGetState(device, power_state)
    }
}

/// Constants for register offsets (device-specific)
pub const NEUROMORPH_REG_CONTROL: c_uint = 0x0000;
pub const NEUROMORPH_REG_STATUS: c_uint = 0x0004;
pub const NEUROMORPH_REG_INTERRUPT_MASK: c_uint = 0x0008;
pub const NEUROMORPH_REG_INTERRUPT_STATUS: c_uint = 0x000C;
pub const NEUROMORPH_REG_DMA_CONTROL: c_uint = 0x0010;
pub const NEUROMORPH_REG_DMA_STATUS: c_uint = 0x0014;
pub const NEUROMORPH_REG_MEMORY_BASE: c_uint = 0x0020;
pub const NEUROMORPH_REG_MEMORY_SIZE: c_uint = 0x0024;
pub const NEUROMORPH_REG_NEURON_COUNT: c_uint = 0x0030;
pub const NEUROMORPH_REG_SYNAPSE_COUNT: c_uint = 0x0034;
pub const NEUROMORPH_REG_CLOCK_FREQ: c_uint = 0x0040;
pub const NEUROMORPH_REG_VERSION: c_uint = 0x00FC;

/// IRQ source masks
pub const NEUROMORPH_IRQ_DMA_COMPLETE: c_uint = 0x0001;
pub const NEUROMORPH_IRQ_KERNEL_COMPLETE: c_uint = 0x0002;
pub const NEUROMORPH_IRQ_ERROR: c_uint = 0x0004;
pub const NEUROMORPH_IRQ_OVERFLOW: c_uint = 0x0008;
pub const NEUROMORPH_IRQ_POWER_STATE: c_uint = 0x0010;
pub const NEUROMORPH_IRQ_THERMAL: c_uint = 0x0020;
pub const NEUROMORPH_IRQ_ALL: c_uint = 0xFFFF;

/// Power states
pub const NEUROMORPH_POWER_STATE_ACTIVE: c_uint = 0;
pub const NEUROMORPH_POWER_STATE_IDLE: c_uint = 1;
pub const NEUROMORPH_POWER_STATE_SLEEP: c_uint = 2;
pub const NEUROMORPH_POWER_STATE_OFF: c_uint = 3;

/// Memory mapping flags
pub const NEUROMORPH_MEM_MAP_READ: c_uint = 0x01;
pub const NEUROMORPH_MEM_MAP_WRITE: c_uint = 0x02;
pub const NEUROMORPH_MEM_MAP_CACHED: c_uint = 0x04;
pub const NEUROMORPH_MEM_MAP_COHERENT: c_uint = 0x08;
