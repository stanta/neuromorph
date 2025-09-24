//! Safe Rust driver library for Neuromorph neuromorphic processor
//!
//! This crate provides safe, RAII-based wrappers around the low-level neuromorph-sys
//! FFI bindings, mirroring CUDA's driver object model.

use neuromorph_sys::*;
use std::ptr;

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
}
