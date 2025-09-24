//! Safe Rust driver library for Neuromorph neuromorphic processor
//!
//! This crate provides safe, RAII-based wrappers around the low-level neuromorph-sys
//! FFI bindings, mirroring CUDA's driver object model.

use neuromorph_sys::*;
use std::ptr;

/// Result type for driver operations
pub type Result<T> = std::result::Result<T, NeuromorphError>;

/// Helper function to convert C error codes to Rust errors
fn convert_error(result: neuromorph_sys::NeuromorphResult) -> Result<()> {
    if result == neuromorph_sys::NEUROMORPH_SUCCESS {
        Ok(())
    } else {
        Err(NeuromorphError::from_c_int(result).unwrap_or(NeuromorphError::ErrorFatalUnknown))
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
/// Automatically destroys the stream when dropped.
pub struct Stream {
    handle: NeuromorphStream,
}

impl Stream {
    /// Create a new stream
    pub fn new() -> Result<Self> {
        unsafe {
            let mut handle: NeuromorphStream = ptr::null_mut();
            let result = neuromorphStreamCreate(&mut handle);
            convert_error(result)?;
            Ok(Stream { handle })
        }
    }

    /// Synchronize with the stream (wait for all operations to complete)
    pub fn synchronize(&self) -> Result<()> {
        unsafe {
            let result = neuromorphStreamSynchronize(self.handle);
            convert_error(result)
        }
    }

    /// Get the raw stream handle for use with low-level functions
    pub fn handle(&self) -> NeuromorphStream {
        self.handle
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
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
        unsafe {
            let result = neuromorphMemcpyAsync(
                self.handle as *mut std::os::raw::c_void,
                host_data.as_ptr() as *const std::os::raw::c_void,
                host_data.len(),
                kind,
                stream.handle(),
            );
            convert_error(result)?;
                Ok(())
        }
    }

    /// Copy data from device to host asynchronously
    pub fn copy_to_host_async(&self, host_data: &mut [u8], stream: &Stream, kind: NeuromorphMemcpyKind) -> Result<()> {
        unsafe {
            let result = neuromorphMemcpyAsync(
                host_data.as_mut_ptr() as *mut std::os::raw::c_void,
                self.handle as *const std::os::raw::c_void,
                host_data.len(),
                kind,
                stream.handle(),
            );
            convert_error(result)?;
                Ok(())
        }
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

/// Launch a neuromorphic kernel
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
    let result = neuromorphLaunchKernel(kernel, grid_dim, block_dim, args, shared_mem, stream.handle());
    convert_error(result)?;
        Ok(())
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
}
