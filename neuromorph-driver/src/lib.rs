//! Safe Rust driver library for Neuromorph neuromorphic processor
//!
//! This crate provides safe, RAII-based wrappers around the low-level neuromorph-sys
//! FFI bindings, mirroring CUDA's driver object model.

use neuromorph_sys::*;
use std::ptr;
use std::sync::Mutex;
use std::collections::BTreeMap;
use std::sync::mpsc;
use std::thread;
use std::sync::mpsc::{Receiver, Sender};

/// Error types for the Neuromorph driver
///
/// Maps cleanly to numeric C codes:
/// - 0 = success
/// - negative = recoverable errors
/// - positive = fatal errors
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeuromorphError {
    // Success
    Success = 0,

    // Recoverable errors (negative values)
    ErrorInvalidValue = -3,
    ErrorInvalidHandle = -4,
    ErrorInvalidDevice = -5,
    ErrorOutOfMemory = -7,
    ErrorNotReady = -8,
    ErrorTimeout = -9,

    // Fatal errors (positive values)
    ErrorFatalHardwareFailure = 1,
    ErrorFatalInternalError = 4,
    ErrorFatalUnknown = 5,
}

impl NeuromorphError {
    /// Convert to C integer code
    pub fn to_c_int(self) -> i32 {
        self as i32
    }

    /// Convert from C integer code
    pub fn from_c_int(code: i32) -> Option<Self> {
        match code {
            0 => Some(Self::Success),
            -3 => Some(Self::ErrorInvalidValue),
            -4 => Some(Self::ErrorInvalidHandle),
            -5 => Some(Self::ErrorInvalidDevice),
            -7 => Some(Self::ErrorOutOfMemory),
            -8 => Some(Self::ErrorNotReady),
            -9 => Some(Self::ErrorTimeout),
            1 => Some(Self::ErrorFatalHardwareFailure),
            4 => Some(Self::ErrorFatalInternalError),
            5 => Some(Self::ErrorFatalUnknown),
            _ => None,
        }
    }

    /// Check if error is fatal
    pub fn is_fatal(self) -> bool {
        (self as i32) > 0
    }

    /// Check if error is recoverable
    pub fn is_recoverable(self) -> bool {
        (self as i32) < 0
    }

    /// Check if successful
    pub fn is_success(self) -> bool {
        (self as i32) == 0
    }

    /// Get error description
    pub fn description(self) -> &'static str {
        match self {
            Self::Success => "Success",
            Self::ErrorInvalidValue => "Invalid value",
            Self::ErrorInvalidHandle => "Invalid handle",
            Self::ErrorInvalidDevice => "Invalid device",
            Self::ErrorOutOfMemory => "Out of memory",
            Self::ErrorNotReady => "Device not ready",
            Self::ErrorTimeout => "Operation timeout",
            Self::ErrorFatalHardwareFailure => "Fatal hardware failure",
            Self::ErrorFatalInternalError => "Fatal internal error",
            Self::ErrorFatalUnknown => "Fatal unknown error",
        }
    }
}

impl std::fmt::Display for NeuromorphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl std::error::Error for NeuromorphError {}

/// Result type for driver operations
pub type Result<T> = std::result::Result<T, NeuromorphError>;

/// Commands that can be queued in a Stream for asynchronous execution
enum Command {
    MemcpyHostToDevice {
        dst: usize,
        src: usize,
        size: usize,
        kind: NeuromorphMemcpyKind,
    },
    MemcpyDeviceToHost {
        dst: usize,
        src: usize,
        size: usize,
        kind: NeuromorphMemcpyKind,
    },
    LaunchKernel {
        kernel: usize,
        grid_dim: NeuromorphDim3,
        block_dim: NeuromorphDim3,
        args: usize,
        shared_mem: usize,
    },
    Barrier {
        sender: mpsc::Sender<()>,
    },
    Quit,
}

/// Helper function to convert C error codes to Rust errors
fn convert_error(result: neuromorph_sys::NeuromorphResult) -> Result<()> {
    if result == neuromorph_sys::NEUROMORPH_SUCCESS {
        Ok(())
    } else {
        Err(NeuromorphError::from_c_int(result as i32).unwrap_or(NeuromorphError::ErrorFatalUnknown))
    }
}

/// Initialize the Neuromorph driver
///
/// This must be called before any other driver operations.
/// Returns an error if initialization fails.
pub fn init() -> Result<()> {
    unsafe {
        let result = neuromorphInit();
        convert_error(result)
    }
}

/// Get the number of available Neuromorph devices
pub fn device_count() -> Result<i32> {
    unsafe {
        let mut count: i32 = 0;
        let result = neuromorphGetDeviceCount(&mut count);
        convert_error(result)?;
        Ok(count)
    }
}

/// Get properties of a specific device
pub fn device_properties(device: i32) -> Result<NeuromorphDeviceProperties> {
    unsafe {
        let mut props = NeuromorphDeviceProperties::default();
        let result = neuromorphGetDeviceProperties(&mut props, device);
        convert_error(result)?;
        Ok(props)
    }
}

/// Safe wrapper for Neuromorph context
///
/// Represents a Neuromorph execution context, similar to a CUDA context.
/// Automatically destroys the context when dropped.
pub struct Context {
    handle: NeuromorphContext,
}

impl Context {
    /// Create a new context for the specified device
    pub fn new(device: i32, flags: u32) -> Result<Self> {
        unsafe {
            let mut handle: NeuromorphContext = ptr::null_mut();
            let result = neuromorphCtxCreate(&mut handle, flags, device);
            convert_error(result)?;
            Ok(Context { handle })
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                let _ = neuromorphCtxDestroy(self.handle);
            }
        }
    }
}

/// Safe wrapper for Neuromorph stream
///
/// Represents an asynchronous execution stream, similar to a CUDA stream.
/// Each stream owns a worker thread that processes commands asynchronously,
/// providing CUDA-like overlap semantics even if the underlying device runs serially.
/// Automatically destroys the stream and joins the worker thread when dropped.
pub struct Stream {
    handle: NeuromorphStream,
    sender: Sender<Command>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl Stream {
    /// Create a new stream
    pub fn new() -> Result<Self> {
        unsafe {
            let mut handle: NeuromorphStream = ptr::null_mut();
            let result = neuromorphStreamCreate(&mut handle);
            convert_error(result)?;

            // Create command channel
            let (sender, receiver) = mpsc::channel();

            // Spawn worker thread
            let join_handle = Some(thread::spawn(move || {
                Stream::worker_loop(receiver);
            }));

            Ok(Stream { handle, sender, join_handle })
        }
    }

    /// Worker thread loop that processes commands
    fn worker_loop(receiver: Receiver<Command>) {
        while let Ok(command) = receiver.recv() {
            match command {
                Command::MemcpyHostToDevice { dst, src, size, kind } => {
                    unsafe {
                        let _ = neuromorphMemcpy(
                            dst as *mut std::os::raw::c_void,
                            src as *const std::os::raw::c_void,
                            size,
                            kind,
                        );
                    }
                }
                Command::MemcpyDeviceToHost { dst, src, size, kind } => {
                    unsafe {
                        let _ = neuromorphMemcpy(
                            dst as *mut std::os::raw::c_void,
                            src as *const std::os::raw::c_void,
                            size,
                            kind,
                        );
                    }
                }
                Command::LaunchKernel { kernel, grid_dim, block_dim, args, shared_mem } => {
                    unsafe {
                        let kernel_ptr = kernel as *mut std::os::raw::c_void;
                        let args_ptr = args as *mut *mut std::os::raw::c_void;
                        let _ = neuromorphLaunchKernel(kernel_ptr, grid_dim, block_dim, args_ptr, shared_mem, ptr::null_mut());
                    }
                }
                Command::Barrier { sender } => {
                    let _ = sender.send(());
                }
                Command::Quit => break,
            }
        }
    }

    /// Synchronize with the stream (wait for all operations to complete)
    pub fn synchronize(&self) -> Result<()> {
        // First synchronize the worker thread to ensure all queued commands are submitted
        let (barrier_sender, barrier_receiver) = mpsc::channel();
        self.sender.send(Command::Barrier { sender: barrier_sender }).map_err(|_| NeuromorphError::ErrorFatalInternalError)?;
        barrier_receiver.recv().map_err(|_| NeuromorphError::ErrorFatalInternalError)?;

        // Then synchronize the underlying HAL stream to ensure execution is complete
        unsafe {
            let result = neuromorphStreamSynchronize(self.handle);
            convert_error(result)?;
        }
        Ok(())
    }

    /// Get the raw stream handle for use with low-level functions
    pub fn handle(&self) -> NeuromorphStream {
        self.handle
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        // Send quit command to worker thread
        let _ = self.sender.send(Command::Quit);

        // Join the worker thread
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }

        // Destroy the stream handle
        unsafe {
            if !self.handle.is_null() {
                let _ = neuromorphStreamDestroy(self.handle);
            }
        }
    }
}

/// Safe wrapper for Neuromorph event
///
/// Represents a synchronization event, similar to a CUDA event.
/// Automatically destroys the event when dropped.
pub struct Event {
    handle: NeuromorphEvent,
}

impl Event {
    /// Create a new event
    pub fn new() -> Result<Self> {
        unsafe {
            let mut handle: NeuromorphEvent = ptr::null_mut();
            let result = neuromorphEventCreate(&mut handle);
            convert_error(result)?;
                Ok(Event { handle })
        }
    }

    /// Record an event in a stream
    pub fn record(&self, stream: &Stream) -> Result<()> {
        unsafe {
            let result = neuromorphEventRecord(self.handle, stream.handle());
            convert_error(result)?;
                Ok(())
        }
    }

    /// Synchronize with the event (wait for it to be recorded)
    pub fn synchronize(&self) -> Result<()> {
        unsafe {
            let result = neuromorphEventSynchronize(self.handle);
            convert_error(result)?;
                Ok(())
        }
    }
}

impl Drop for Event {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                let _ = neuromorphEventDestroy(self.handle);
            }
        }
    }
}

/// Safe wrapper for Neuromorph kernel/graph
///
/// Represents a loaded neuromorphic graph/kernel, similar to a CUDA kernel.
/// Automatically unloads the kernel when dropped.
#[derive(Debug)]
pub struct Kernel {
    handle: NeuromorphKernel,
}

impl Kernel {
    /// Load a kernel from binary data
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        unsafe {
            let mut handle: NeuromorphKernel = ptr::null_mut();
            let result = neuromorphGraphLoad(&mut handle, data.as_ptr() as *const std::os::raw::c_void, data.len());
            convert_error(result)?;
            Ok(Kernel { handle })
        }
    }

    /// Launch the kernel asynchronously for a specified number of ticks
    ///
    /// This method returns immediately after queuing the launch. The kernel will
    /// simulate execution for the specified number of ticks and signal completion
    /// via the provided event.
    ///
    /// # Parameters
    /// - `ticks`: Number of simulation ticks to run
    /// - `stream`: Stream to launch on
    /// - `completion_event`: Optional event to record when launch completes
    pub fn launch(&self, ticks: u32, stream: &Stream, completion_event: Option<&Event>) -> Result<()> {
        // For now, simulate a simple kernel launch
        // In a real implementation, this would configure the neuromorphic processor
        // with the graph and start execution

        unsafe {
            // Use a dummy grid/block configuration for now
            let grid_dim = NeuromorphDim3 { x: 1, y: 1, z: 1 };
            let block_dim = NeuromorphDim3 { x: 1, y: 1, z: 1 };

            // Prepare arguments (ticks parameter)
            let args = vec![&ticks as *const u32 as *mut std::os::raw::c_void];

            let result = neuromorphLaunchKernel(
                self.handle,
                grid_dim,
                block_dim,
                args.as_ptr() as *mut *mut std::os::raw::c_void,
                0, // shared memory
                stream.handle(),
            );
            convert_error(result)?;

            // Record completion event if provided
            if let Some(event) = completion_event {
                event.record(stream)?;
            }

            Ok(())
        }
    }

    /// Get the raw kernel handle for use with low-level functions
    pub fn handle(&self) -> NeuromorphKernel {
        self.handle
    }
}

impl Drop for Kernel {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                let _ = neuromorphGraphUnload(self.handle);
            }
        }
    }
}

/// GPU-style memory allocator for managing on-device RAM and sub-allocation
///
/// This allocator provides sub-allocation capabilities on top of neuromorph device memory,
/// similar to how GPU memory allocators manage VRAM with efficient sub-allocation.
/// It uses a first-fit allocation strategy with alignment support and automatic coalescing
/// of free blocks to minimize fragmentation.
///
/// Note: A custom allocator implementation was chosen over gpu-alloc/gpu-allocator crates
/// for tighter control over allocation semantics and better integration with the neuromorph
/// HAL simulator. The allocator provides equivalent GPU-style sub-allocation functionality.
///
/// # Examples
///
/// ```
/// use neuromorph_driver::{init, MemoryAllocator};
///
/// init().unwrap();
/// let mut allocator = MemoryAllocator::new(4096).unwrap();
///
/// // Allocate a 1024-byte block with 8-byte alignment
/// let block = allocator.allocate(1024, 8).unwrap();
/// assert_eq!(block.size(), 1024);
/// assert_eq!(block.offset(), 0);
///
/// // The block is automatically freed when it goes out of scope
/// ```
pub struct MemoryAllocator {
    device_memory: DeviceMemory,
    state: Mutex<AllocatorState>,
}

struct AllocatorState {
    allocated_blocks: BTreeMap<usize, usize>, // offset -> size
    free_blocks: BTreeMap<usize, usize>, // offset -> size
}

impl MemoryAllocator {
    /// Create a new memory allocator with a device memory pool
    pub fn new(pool_size: usize) -> Result<Self> {
        let device_memory = DeviceMemory::new(pool_size)?;
        let mut free_blocks = BTreeMap::new();
        free_blocks.insert(0, pool_size);

        Ok(MemoryAllocator {
            device_memory,
            state: Mutex::new(AllocatorState {
                allocated_blocks: BTreeMap::new(),
                free_blocks,
            }),
        })
    }

    /// Allocate a sub-region from the memory pool with alignment
    pub fn allocate(&self, size: usize, align: usize) -> Result<MemoryBlock> {
        if size == 0 || align == 0 {
            return Err(NeuromorphError::ErrorInvalidValue);
        }

        let mut state = self.state.lock().unwrap();

        // Find a suitable free block
        for (&offset, &block_size) in &state.free_blocks.clone() {
            let aligned_offset = Self::align_up(offset, align);
            let padding = aligned_offset - offset;

            if aligned_offset + size <= offset + block_size {
                // Found a suitable block
                state.free_blocks.remove(&offset);

                // Add back the remaining space
                if padding > 0 {
                    state.free_blocks.insert(offset, padding);
                }
                let remaining = (offset + block_size) - (aligned_offset + size);
                if remaining > 0 {
                    state.free_blocks.insert(aligned_offset + size, remaining);
                }

                // Record allocation
                state.allocated_blocks.insert(aligned_offset, size);

                return Ok(MemoryBlock {
                    allocator: self as *const MemoryAllocator,
                    offset: aligned_offset,
                    size,
                });
            }
        }

        Err(NeuromorphError::ErrorOutOfMemory)
    }

    /// Deallocate a memory block
    pub(crate) fn deallocate(&self, offset: usize, size: usize) {
        let mut state = self.state.lock().unwrap();
        // Remove from allocated
        state.allocated_blocks.remove(&offset);

        // Add to free blocks and coalesce
        state.free_blocks.insert(offset, size);
        self.coalesce_free_blocks(&mut state);
    }

    /// Align offset up to the given alignment
    fn align_up(offset: usize, align: usize) -> usize {
        (offset + align - 1) & !(align - 1)
    }

    /// Coalesce adjacent free blocks
    fn coalesce_free_blocks(&self, state: &mut AllocatorState) {
        let mut coalesced = BTreeMap::new();
        let mut prev_offset = None;
        let mut prev_size = 0;

        for (&offset, &size) in &state.free_blocks {
            if let Some(prev) = prev_offset {
                if prev + prev_size == offset {
                    // Coalesce
                    prev_size += size;
                    continue;
                } else {
                    coalesced.insert(prev, prev_size);
                }
            }
            prev_offset = Some(offset);
            prev_size = size;
        }

        if let Some(prev) = prev_offset {
            coalesced.insert(prev, prev_size);
        }

        state.free_blocks = coalesced;
    }

    /// Get the base device pointer
    pub fn base_ptr(&self) -> NeuromorphDevicePtr {
        self.device_memory.handle()
    }
}

impl Drop for MemoryAllocator {
    fn drop(&mut self) {
        // DeviceMemory will be dropped automatically
    }
}

/// Memory block representing a sub-allocated region
///
/// Represents a contiguous region of device memory allocated from a MemoryAllocator.
/// The block automatically deallocates itself when dropped, following RAII principles.
///
/// Memory blocks can be used for data transfer operations similar to DeviceMemory,
/// but they represent sub-allocated regions rather than whole device memory allocations.
#[derive(Debug)]
pub struct MemoryBlock {
    allocator: *const MemoryAllocator,
    offset: usize,
    size: usize,
}

impl MemoryBlock {
    /// Get the device pointer for this block (base + offset)
    pub fn device_ptr(&self) -> NeuromorphDevicePtr {
        unsafe {
            if self.allocator.is_null() {
                ptr::null_mut()
            } else {
                let base = (*self.allocator).base_ptr();
                (base as usize + self.offset) as NeuromorphDevicePtr
            }
        }
    }

    /// Get the offset within the pool
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Get the size of this block
    pub fn size(&self) -> usize {
        self.size
    }

    /// Copy data from host to this memory block
    pub fn copy_from_host(&self, host_data: &[u8], kind: NeuromorphMemcpyKind) -> Result<()> {
        unsafe {
            let result = neuromorphMemcpy(
                self.device_ptr() as *mut std::os::raw::c_void,
                host_data.as_ptr() as *const std::os::raw::c_void,
                host_data.len(),
                kind,
            );
            convert_error(result)
        }
    }

    /// Copy data from this memory block to host
    pub fn copy_to_host(&self, host_data: &mut [u8], kind: NeuromorphMemcpyKind) -> Result<()> {
        unsafe {
            let result = neuromorphMemcpy(
                host_data.as_mut_ptr() as *mut std::os::raw::c_void,
                self.device_ptr() as *const std::os::raw::c_void,
                host_data.len(),
                kind,
            );
            convert_error(result)
        }
    }
}

impl Drop for MemoryBlock {
    fn drop(&mut self) {
        if !self.allocator.is_null() {
            unsafe {
                (*self.allocator).deallocate(self.offset, self.size);
            }
        }
    }
}

/// Safe wrapper for device memory allocation
///
/// Represents allocated device memory, similar to CUDA device memory.
/// Automatically frees the memory when dropped.
pub struct DeviceMemory {
    handle: NeuromorphDevicePtr,
    size: usize,
}

impl DeviceMemory {
    /// Allocate device memory
    pub fn new(size: usize) -> Result<Self> {
        unsafe {
            let mut handle: NeuromorphDevicePtr = ptr::null_mut();
            let result = neuromorphMalloc(&mut handle, size);
            convert_error(result)?;
                Ok(DeviceMemory { handle, size })
        }
    }

    /// Get the size of the allocated memory
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get the raw device pointer handle
    pub fn handle(&self) -> NeuromorphDevicePtr {
        self.handle
    }

    /// Copy data from host to device
    pub fn copy_from_host(&self, host_data: &[u8], kind: NeuromorphMemcpyKind) -> Result<()> {
        unsafe {
            let result = neuromorphMemcpy(
                self.handle as *mut std::os::raw::c_void,
                host_data.as_ptr() as *const std::os::raw::c_void,
                host_data.len(),
                kind,
            );
            convert_error(result)?;
                Ok(())
        }
    }

    /// Copy data from device to host
    pub fn copy_to_host(&self, host_data: &mut [u8], kind: NeuromorphMemcpyKind) -> Result<()> {
        unsafe {
            let result = neuromorphMemcpy(
                host_data.as_mut_ptr() as *mut std::os::raw::c_void,
                self.handle as *const std::os::raw::c_void,
                host_data.len(),
                kind,
            );
            convert_error(result)?;
                Ok(())
        }
    }

    /// Copy data from host to device asynchronously
    pub fn copy_from_host_async(&self, host_data: &[u8], stream: &Stream, kind: NeuromorphMemcpyKind) -> Result<()> {
        stream.sender.send(Command::MemcpyHostToDevice {
            dst: self.handle as usize,
            src: host_data.as_ptr() as usize,
            size: host_data.len(),
            kind,
        }).map_err(|_| NeuromorphError::ErrorFatalInternalError)?;
        Ok(())
    }

    /// Copy data from device to host asynchronously
    pub fn copy_to_host_async(&self, host_data: &mut [u8], stream: &Stream, kind: NeuromorphMemcpyKind) -> Result<()> {
        stream.sender.send(Command::MemcpyDeviceToHost {
            dst: host_data.as_mut_ptr() as usize,
            src: self.handle as usize,
            size: host_data.len(),
            kind,
        }).map_err(|_| NeuromorphError::ErrorFatalInternalError)?;
        Ok(())
    }
}

impl Drop for DeviceMemory {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                let _ = neuromorphFree(self.handle);
            }
        }
    }
}

/// Launch a neuromorphic kernel asynchronously on the stream
///
/// # Safety
/// kernel must be a valid kernel handle, all parameters must be valid
pub unsafe fn launch_kernel(
    kernel: NeuromorphKernel,
    grid_dim: NeuromorphDim3,
    block_dim: NeuromorphDim3,
    args: *mut *mut std::os::raw::c_void,
    shared_mem: usize,
    stream: &Stream,
) -> Result<()> {
    stream.sender.send(Command::LaunchKernel {
        kernel: kernel as usize,
        grid_dim,
        block_dim,
        args: args as usize,
        shared_mem,
    }).map_err(|_| NeuromorphError::ErrorFatalInternalError)?;
    Ok(())
}

/// Perform a synchronous (blocking) memory copy operation
///
/// This function performs a memory copy operation and blocks until completion.
/// It is equivalent to calling neuromorphMemcpy directly.
///
/// # Safety
/// dst and src must be valid pointers for the specified size and kind
pub unsafe fn memcpy_sync(
    dst: *mut std::os::raw::c_void,
    src: *const std::os::raw::c_void,
    size: usize,
    kind: NeuromorphMemcpyKind,
) -> Result<()> {
    let result = neuromorph_sys::neuromorphMemcpy(dst, src, size, kind);
    convert_error(result)
}

/// Perform an asynchronous (non-blocking) memory copy operation on a stream
///
/// This function queues a memory copy operation on the specified stream and returns
/// immediately. The operation will be executed asynchronously by the stream's worker thread.
/// Use stream.synchronize() to wait for completion.
///
/// # Safety
/// dst and src must be valid pointers for the specified size and kind,
/// stream must be a valid Stream handle
pub unsafe fn memcpy_async(
    dst: *mut std::os::raw::c_void,
    src: *const std::os::raw::c_void,
    size: usize,
    kind: NeuromorphMemcpyKind,
    stream: &Stream,
) -> Result<()> {
    let result = neuromorph_sys::neuromorphMemcpyAsync(dst, src, size, kind, stream.handle());
    convert_error(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_and_device_count() {
        assert!(init().is_ok());
        let count = device_count().unwrap();
        assert!(count >= 0);
    }

    #[test]
    fn test_context_creation() {
        init().unwrap();
        let context = Context::new(0, 0).unwrap();
        // Context will be automatically dropped
    }

    #[test]
    fn test_stream_operations() {
        init().unwrap();
        let stream = Stream::new().unwrap();
        assert!(stream.synchronize().is_ok());
        // Stream will be automatically dropped
    }

    #[test]
    fn test_stream_async_execution() {
        init().unwrap();
        let stream = Stream::new().unwrap();
        let memory = DeviceMemory::new(1024).unwrap();
        
        let test_data = vec![0x42u8; 512];
        
        // Queue multiple operations
        assert!(memory.copy_from_host_async(&test_data, &stream, NeuromorphMemcpyKind::HostToDevice).is_ok());
        
        let mut result_data = vec![0u8; 512];
        assert!(memory.copy_to_host_async(&mut result_data, &stream, NeuromorphMemcpyKind::DeviceToHost).is_ok());
        
        // Synchronize to ensure all operations complete
        assert!(stream.synchronize().is_ok());
        
        assert_eq!(test_data, result_data);
    }

    #[test]
    fn test_stream_concurrency() {
        init().unwrap();
        let stream1 = Stream::new().unwrap();
        let stream2 = Stream::new().unwrap();
        
        let memory1 = DeviceMemory::new(1024).unwrap();
        let memory2 = DeviceMemory::new(1024).unwrap();
        
        let data1 = vec![1u8; 512];
        let data2 = vec![2u8; 512];
        
        // Queue operations on both streams
        memory1.copy_from_host_async(&data1, &stream1, NeuromorphMemcpyKind::HostToDevice).unwrap();
        memory2.copy_from_host_async(&data2, &stream2, NeuromorphMemcpyKind::HostToDevice).unwrap();
        
        // Synchronize both
        stream1.synchronize().unwrap();
        stream2.synchronize().unwrap();
        
        // Verify results
        let mut result1 = vec![0u8; 512];
        let mut result2 = vec![0u8; 512];
        
        memory1.copy_to_host(&mut result1, NeuromorphMemcpyKind::DeviceToHost).unwrap();
        memory2.copy_to_host(&mut result2, NeuromorphMemcpyKind::DeviceToHost).unwrap();
        
        assert_eq!(data1, result1);
        assert_eq!(data2, result2);
    }

    #[test]
    fn test_event_operations() {
        init().unwrap();
        let event = Event::new().unwrap();
        assert!(event.synchronize().is_ok());
        // Event will be automatically dropped
    }

    #[test]
    fn test_memory_allocation() {
        init().unwrap();
        let memory = DeviceMemory::new(1024).unwrap();
        assert_eq!(memory.size(), 1024);
        // Memory will be automatically freed
    }

    #[test]
    fn test_memory_copy() {
        init().unwrap();
        let memory = DeviceMemory::new(1024).unwrap();

        let test_data = vec![0x42u8; 512];
        assert!(memory.copy_from_host(&test_data, NeuromorphMemcpyKind::HostToDevice).is_ok());

        let mut result_data = vec![0u8; 512];
        assert!(memory.copy_to_host(&mut result_data, NeuromorphMemcpyKind::DeviceToHost).is_ok());

        assert_eq!(test_data, result_data);
    }

    #[test]
    fn test_error_enum_conversion() {
        // Test to_c_int conversion
        assert_eq!(NeuromorphError::Success.to_c_int(), 0);
        assert_eq!(NeuromorphError::ErrorInvalidValue.to_c_int(), -3);
        assert_eq!(NeuromorphError::ErrorFatalHardwareFailure.to_c_int(), 1);

        // Test from_c_int conversion
        assert_eq!(NeuromorphError::from_c_int(0), Some(NeuromorphError::Success));
        assert_eq!(NeuromorphError::from_c_int(-3), Some(NeuromorphError::ErrorInvalidValue));
        assert_eq!(NeuromorphError::from_c_int(-999), None); // Unknown error code

        // Test error classification
        assert!(NeuromorphError::Success.is_success());
        assert!(!NeuromorphError::Success.is_fatal());
        assert!(!NeuromorphError::Success.is_recoverable());

        assert!(NeuromorphError::ErrorInvalidValue.is_recoverable());
        assert!(!NeuromorphError::ErrorInvalidValue.is_fatal());
        assert!(!NeuromorphError::ErrorInvalidValue.is_success());

        assert!(NeuromorphError::ErrorFatalHardwareFailure.is_fatal());
        assert!(!NeuromorphError::ErrorFatalHardwareFailure.is_recoverable());
        assert!(!NeuromorphError::ErrorFatalHardwareFailure.is_success());

        // Test descriptions
        assert_eq!(NeuromorphError::Success.description(), "Success");
        assert_eq!(NeuromorphError::ErrorInvalidValue.description(), "Invalid value");
        assert_eq!(NeuromorphError::ErrorFatalHardwareFailure.description(), "Fatal hardware failure");
    }

    #[test]
    fn test_device_properties() {
        init().unwrap();
        let props = device_properties(0).unwrap();

        // Basic validation that properties are reasonable
        assert!(props.total_global_mem > 0);
        assert!(props.multi_processor_count > 0);
    }

    #[test]
    fn test_event_recording() {
        init().unwrap();
        let stream = Stream::new().unwrap();
        let event = Event::new().unwrap();

        // Record event in stream
        assert!(event.record(&stream).is_ok());

        // Synchronize should work (event was recorded)
        assert!(event.synchronize().is_ok());
    }

    #[test]
    fn test_async_memory_copy() {
        init().unwrap();
        let memory = DeviceMemory::new(1024).unwrap();
        let stream = Stream::new().unwrap();

        let test_data = vec![0xABu8; 512];

        // Async copy from host to device
        assert!(memory.copy_from_host_async(&test_data, &stream, NeuromorphMemcpyKind::HostToDevice).is_ok());

        // Synchronize stream to ensure copy completes
        assert!(stream.synchronize().is_ok());

        // Async copy from device to host
        let mut result_data = vec![0u8; 512];
        assert!(memory.copy_to_host_async(&mut result_data, &stream, NeuromorphMemcpyKind::DeviceToHost).is_ok());

        // Synchronize to ensure copy completes
        assert!(stream.synchronize().is_ok());

        // Verify data
        assert_eq!(test_data, result_data);
    }

    #[test]
    fn test_error_display() {
        let error = NeuromorphError::ErrorInvalidValue;
        assert_eq!(format!("{}", error), "Invalid value");
        assert_eq!(format!("{:?}", error), "ErrorInvalidValue");
    }

    #[test]
    fn test_memory_allocator_creation() {
        init().unwrap();
        let allocator = MemoryAllocator::new(4096).unwrap();
        // MemoryAllocator will be dropped automatically
    }


    #[test]
    fn test_memory_allocator_out_of_memory() {
        init().unwrap();
        let mut allocator = MemoryAllocator::new(1024).unwrap();

        // Try to allocate more than available
        let result = allocator.allocate(2048, 8);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), NeuromorphError::ErrorOutOfMemory);
    }

    #[test]
    fn test_memory_sub_allocation_basic() {
        init().unwrap();
        let mut allocator = MemoryAllocator::new(4096).unwrap();

        // Allocate a block
        let block = allocator.allocate(1024, 8).unwrap();
        assert_eq!(block.size(), 1024);
        assert_eq!(block.offset(), 0);

        // Allocate another block
        let block2 = allocator.allocate(512, 16).unwrap();
        assert_eq!(block2.size(), 512);
        // Should be aligned to 16 bytes
        assert_eq!(block2.offset(), 1024);

        // Check device pointers are different
        assert_ne!(block.device_ptr(), block2.device_ptr());
    }

    #[test]
    fn test_memory_sub_allocation_alignment() {
        init().unwrap();
        let mut allocator = MemoryAllocator::new(4096).unwrap();

        // Test various alignments
        let block1 = allocator.allocate(100, 64).unwrap();
        assert_eq!(block1.offset() % 64, 0);

        let block2 = allocator.allocate(200, 128).unwrap();
        assert_eq!(block2.offset() % 128, 0);

        let block3 = allocator.allocate(50, 256).unwrap();
        assert_eq!(block3.offset() % 256, 0);
    }

    #[test]
    fn test_memory_sub_allocation_fragmentation() {
        init().unwrap();
        let mut allocator = MemoryAllocator::new(2048).unwrap();

        // Allocate multiple blocks
        let block1 = allocator.allocate(256, 8).unwrap(); // offset 0
        let block2 = allocator.allocate(256, 8).unwrap(); // offset 256
        let block3 = allocator.allocate(256, 8).unwrap(); // offset 512

        // Drop middle block to create fragmentation
        drop(block2);

        // Allocate a block that should fit in the freed space
        let block4 = allocator.allocate(256, 8).unwrap();
        assert_eq!(block4.offset(), 256); // Should reuse the freed space

        // Allocate another block
        let block5 = allocator.allocate(256, 8).unwrap();
        assert_eq!(block5.offset(), 768); // Should be after block3
    }

    #[test]
    fn test_memory_sub_allocation_max_utilization() {
        init().unwrap();
        let mut allocator = MemoryAllocator::new(1024).unwrap();

        // Allocate exactly the pool size
        let block = allocator.allocate(1024, 1).unwrap();
        assert_eq!(block.size(), 1024);
        assert_eq!(block.offset(), 0);

        // Should fail to allocate more
        let result = allocator.allocate(1, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_sub_allocation_with_padding() {
        init().unwrap();
        let mut allocator = MemoryAllocator::new(1024).unwrap();

        // Allocate with alignment that requires padding
        let block1 = allocator.allocate(100, 8).unwrap(); // offset 0
        let block2 = allocator.allocate(100, 64).unwrap(); // needs alignment to 64

        // Should have padding between blocks
        assert!(block2.offset() >= block1.offset() + block1.size());
        assert_eq!(block2.offset() % 64, 0);
    }

    #[test]
    fn test_memory_block_operations() {
        init().unwrap();
        let mut allocator = MemoryAllocator::new(4096).unwrap();
        let block = allocator.allocate(1024, 8).unwrap();

        // Test basic properties
        assert_eq!(block.size(), 1024);
        assert!(block.offset() >= 0);

        // Test device pointer calculation
        let base_ptr = allocator.base_ptr();
        let expected_ptr = (base_ptr as usize + block.offset()) as NeuromorphDevicePtr;
        assert_eq!(block.device_ptr(), expected_ptr);
    }

    #[test]
    fn test_memory_allocator_reuse_after_deallocation() {
        init().unwrap();
        let mut allocator = MemoryAllocator::new(2048).unwrap();

        // Allocate and deallocate
        {
            let block = allocator.allocate(512, 8).unwrap();
            assert_eq!(block.offset(), 0);
        } // block is dropped here

        // Should be able to allocate again in the same space
        let block2 = allocator.allocate(512, 8).unwrap();
        assert_eq!(block2.offset(), 0); // Should reuse the space
    }

    #[test]
    fn test_memory_sub_allocation_edge_cases() {
        init().unwrap();
        let mut allocator = MemoryAllocator::new(512).unwrap();

        // Test zero-sized allocation (should fail)
        let result = allocator.allocate(0, 8);
        assert!(result.is_err()); // Zero size should fail

        // Test allocation larger than pool
        let result = allocator.allocate(1024, 8);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), NeuromorphError::ErrorOutOfMemory);

        // Test allocation that fits exactly after alignment
        let block = allocator.allocate(512, 1).unwrap();
        assert_eq!(block.size(), 512);
        assert_eq!(block.offset(), 0);
    }

    #[test]
    fn test_memory_coalescing() {
        init().unwrap();
        let mut allocator = MemoryAllocator::new(1024).unwrap();

        // Allocate three blocks
        let block1 = allocator.allocate(200, 8).unwrap(); // 0-200
        let block2 = allocator.allocate(200, 8).unwrap(); // 200-400
        let block3 = allocator.allocate(200, 8).unwrap(); // 400-600

        // Free middle block
        drop(block2);

        // Free first block
        drop(block1);

        // Should coalesce free blocks
        // Now try to allocate a larger block that spans the coalesced space
        let block4 = allocator.allocate(400, 8).unwrap();
        assert_eq!(block4.offset(), 0); // Should start at beginning
        assert_eq!(block4.size(), 400);
    }

    #[test]
    fn test_memcpy_sync() {
        init().unwrap();
        let memory = DeviceMemory::new(1024).unwrap();

        let test_data = vec![0xABu8; 512];

        // Copy from host to device synchronously
        unsafe {
            memcpy_sync(
                memory.handle() as *mut std::os::raw::c_void,
                test_data.as_ptr() as *const std::os::raw::c_void,
                test_data.len(),
                NeuromorphMemcpyKind::HostToDevice,
            ).unwrap();
        }

        // Copy from device to host synchronously
        let mut result_data = vec![0u8; 512];
        unsafe {
            memcpy_sync(
                result_data.as_mut_ptr() as *mut std::os::raw::c_void,
                memory.handle() as *const std::os::raw::c_void,
                result_data.len(),
                NeuromorphMemcpyKind::DeviceToHost,
            ).unwrap();
        }

        // Verify data
        assert_eq!(test_data, result_data);
    }

    #[test]
    fn test_memcpy_async() {
        init().unwrap();
        let memory = DeviceMemory::new(1024).unwrap();
        let stream = Stream::new().unwrap();

        let test_data = vec![0xCDu8; 512];

        // Copy from host to device asynchronously
        unsafe {
            memcpy_async(
                memory.handle() as *mut std::os::raw::c_void,
                test_data.as_ptr() as *const std::os::raw::c_void,
                test_data.len(),
                NeuromorphMemcpyKind::HostToDevice,
                &stream,
            ).unwrap();
        }

        // Synchronize to ensure completion
        stream.synchronize().unwrap();

        // Copy from device to host asynchronously
        let mut result_data = vec![0u8; 512];
        unsafe {
            memcpy_async(
                result_data.as_mut_ptr() as *mut std::os::raw::c_void,
                memory.handle() as *const std::os::raw::c_void,
                result_data.len(),
                NeuromorphMemcpyKind::DeviceToHost,
                &stream,
            ).unwrap();
        }

        // Synchronize to ensure completion
        stream.synchronize().unwrap();

        // Verify data
        assert_eq!(test_data, result_data);
    }

    #[test]
    fn test_memcpy_async_concurrent() {
        init().unwrap();
        let memory1 = DeviceMemory::new(1024).unwrap();
        let memory2 = DeviceMemory::new(1024).unwrap();
        let stream1 = Stream::new().unwrap();
        let stream2 = Stream::new().unwrap();

        let data1 = vec![1u8; 512];
        let data2 = vec![2u8; 512];

        // Queue async copies on both streams
        unsafe {
            memcpy_async(
                memory1.handle() as *mut std::os::raw::c_void,
                data1.as_ptr() as *const std::os::raw::c_void,
                data1.len(),
                NeuromorphMemcpyKind::HostToDevice,
                &stream1,
            ).unwrap();

            memcpy_async(
                memory2.handle() as *mut std::os::raw::c_void,
                data2.as_ptr() as *const std::os::raw::c_void,
                data2.len(),
                NeuromorphMemcpyKind::HostToDevice,
                &stream2,
            ).unwrap();
        }

        // Synchronize both streams
        stream1.synchronize().unwrap();
        stream2.synchronize().unwrap();

        // Verify results
        let mut result1 = vec![0u8; 512];
        let mut result2 = vec![0u8; 512];

        unsafe {
            memcpy_sync(
                result1.as_mut_ptr() as *mut std::os::raw::c_void,
                memory1.handle() as *const std::os::raw::c_void,
                result1.len(),
                NeuromorphMemcpyKind::DeviceToHost,
            ).unwrap();

            memcpy_sync(
                result2.as_mut_ptr() as *mut std::os::raw::c_void,
                memory2.handle() as *const std::os::raw::c_void,
                result2.len(),
                NeuromorphMemcpyKind::DeviceToHost,
            ).unwrap();
        }

        assert_eq!(data1, result1);
        assert_eq!(data2, result2);
    }

    #[test]
    fn test_kernel_load_and_launch() {
        init().unwrap();
        let stream = Stream::new().unwrap();
        let event = Event::new().unwrap();

        // Create some dummy graph data (in a real implementation, this would be a compiled SNN/DNN graph)
        let graph_data = vec![0xAAu8; 1024];

        // Load the kernel
        let kernel = Kernel::from_bytes(&graph_data).unwrap();

        // Launch the kernel for 100 ticks
        assert!(kernel.launch(100, &stream, Some(&event)).is_ok());

        // Wait for completion
        assert!(event.synchronize().is_ok());

        // Kernel will be automatically unloaded
    }

    #[test]
    fn test_kernel_launch_without_event() {
        init().unwrap();
        let stream = Stream::new().unwrap();

        let graph_data = vec![0xBBu8; 512];
        let kernel = Kernel::from_bytes(&graph_data).unwrap();

        // Launch without completion event
        assert!(kernel.launch(50, &stream, None).is_ok());

        // Synchronize stream to ensure completion
        assert!(stream.synchronize().is_ok());
    }

    #[test]
    fn test_kernel_invalid_data() {
        init().unwrap();

        // Try to load empty data (should fail)
        let result = Kernel::from_bytes(&[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), NeuromorphError::ErrorInvalidValue);
    }

    #[test]
    fn test_kernel_multiple_launches() {
        init().unwrap();
        let stream = Stream::new().unwrap();

        let graph_data = vec![0xCCu8; 256];
        let kernel = Kernel::from_bytes(&graph_data).unwrap();

        // Launch multiple times
        for ticks in [10, 20, 30] {
            let event = Event::new().unwrap();
            assert!(kernel.launch(ticks, &stream, Some(&event)).is_ok());
            assert!(event.synchronize().is_ok());
        }
    }

}
