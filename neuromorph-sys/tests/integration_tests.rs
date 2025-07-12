//! Tests for neuromorph-sys FFI bindings

use neuromorph_sys::*;
use std::ptr;
use std::os::raw::c_int;

#[test]
fn test_init_and_device_count() {
    unsafe {
        // Initialize the driver
        let result = neuromorphInit();
        assert_eq!(result, NEUROMORPH_SUCCESS);
        
        // Get device count
        let mut count: c_int = 0;
        let result = neuromorphGetDeviceCount(&mut count);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        
        // In simulator mode, we should have 2 devices
        #[cfg(feature = "simulator")]
        assert_eq!(count, 2);
        
        // In hardware mode, we expect 0 devices (stub implementation)
        #[cfg(feature = "hardware")]
        assert_eq!(count, 0);
    }
}

#[test]
#[cfg(feature = "simulator")]
fn test_device_properties() {
    unsafe {
        neuromorphInit();
        
        let mut props = NeuromorphDeviceProperties::default();
        let result = neuromorphGetDeviceProperties(&mut props, 0);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        
        // Check some basic properties
        assert!(props.total_global_mem > 0);
        assert!(props.multi_processor_count > 0);
        assert!(props.major >= 3);
    }
}

#[test]
#[cfg(feature = "simulator")]
fn test_context_creation() {
    unsafe {
        neuromorphInit();
        
        let mut ctx: NeuromorphContext = ptr::null_mut();
        let result = neuromorphCtxCreate(&mut ctx, NEUROMORPH_CTX_SCHED_AUTO, 0);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        assert!(!ctx.is_null());
        
        let result = neuromorphCtxDestroy(ctx);
        assert_eq!(result, NEUROMORPH_SUCCESS);
    }
}

#[test]
#[cfg(feature = "simulator")]
fn test_memory_allocation() {
    unsafe {
        neuromorphInit();
        
        let mut ctx: NeuromorphContext = ptr::null_mut();
        neuromorphCtxCreate(&mut ctx, NEUROMORPH_CTX_SCHED_AUTO, 0);
        
        let mut dev_ptr: NeuromorphDevicePtr = ptr::null_mut();
        let size = 1024;
        let result = neuromorphMalloc(&mut dev_ptr, size);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        assert!(!dev_ptr.is_null());
        
        let result = neuromorphFree(dev_ptr);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        
        neuromorphCtxDestroy(ctx);
    }
}

#[test]
#[cfg(feature = "simulator")]
fn test_stream_operations() {
    unsafe {
        neuromorphInit();
        
        let mut ctx: NeuromorphContext = ptr::null_mut();
        neuromorphCtxCreate(&mut ctx, NEUROMORPH_CTX_SCHED_AUTO, 0);
        
        let mut stream: NeuromorphStream = ptr::null_mut();
        let result = neuromorphStreamCreate(&mut stream);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        assert!(!stream.is_null());
        
        let result = neuromorphStreamSynchronize(stream);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        
        let result = neuromorphStreamDestroy(stream);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        
        neuromorphCtxDestroy(ctx);
    }
}

#[test]
#[cfg(feature = "simulator")]
fn test_event_operations() {
    unsafe {
        neuromorphInit();
        
        let mut ctx: NeuromorphContext = ptr::null_mut();
        neuromorphCtxCreate(&mut ctx, NEUROMORPH_CTX_SCHED_AUTO, 0);
        
        let mut event: NeuromorphEvent = ptr::null_mut();
        let result = neuromorphEventCreate(&mut event);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        assert!(!event.is_null());
        
        let result = neuromorphEventSynchronize(event);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        
        let result = neuromorphEventDestroy(event);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        
        neuromorphCtxDestroy(ctx);
    }
}

#[test]
#[cfg(feature = "simulator")]
fn test_register_access() {
    unsafe {
        neuromorphInit();
        
        let mut value: u32 = 0;
        let result = neuromorphRegisterRead(0, NEUROMORPH_REG_VERSION, &mut value);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        assert_eq!(value, 0x03000000); // Version 3.0
        
        let result = neuromorphRegisterWrite(0, NEUROMORPH_REG_CONTROL, 0x12345678);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        
        let result = neuromorphRegisterRead(0, NEUROMORPH_REG_CONTROL, &mut value);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        assert_eq!(value, 0x12345678);
    }
}

#[test]
#[cfg(feature = "simulator")]
fn test_memory_copy() {
    unsafe {
        neuromorphInit();
        
        let mut ctx: NeuromorphContext = ptr::null_mut();
        neuromorphCtxCreate(&mut ctx, NEUROMORPH_CTX_SCHED_AUTO, 0);
        
        // Allocate host and device memory
        let size = 1024;
        let host_data = vec![0x42u8; size];
        let mut host_result = vec![0u8; size];
        
        let mut dev_ptr: NeuromorphDevicePtr = ptr::null_mut();
        neuromorphMalloc(&mut dev_ptr, size);
        
        // Copy host to device
        let result = neuromorphMemcpy(
            dev_ptr,
            host_data.as_ptr() as *const std::os::raw::c_void,
            size,
            NeuromorphMemcpyKind::HostToDevice
        );
        assert_eq!(result, NEUROMORPH_SUCCESS);
        
        // Copy device to host
        let result = neuromorphMemcpy(
            host_result.as_mut_ptr() as *mut std::os::raw::c_void,
            dev_ptr,
            size,
            NeuromorphMemcpyKind::DeviceToHost
        );
        assert_eq!(result, NEUROMORPH_SUCCESS);
        
        // Verify data
        assert_eq!(host_data, host_result);
        
        neuromorphFree(dev_ptr);
        neuromorphCtxDestroy(ctx);
    }
}

#[test]
fn test_error_codes() {
    // Test error code conversions
    assert_eq!(NeuromorphError::Success.to_c_int(), 0);
    assert_eq!(NeuromorphError::ErrorInvalidValue.to_c_int(), -3);
    assert_eq!(NeuromorphError::ErrorFatalHardwareFailure.to_c_int(), 1);
    
    // Test error classification
    assert!(NeuromorphError::Success.is_success());
    assert!(NeuromorphError::ErrorInvalidValue.is_recoverable());
    assert!(NeuromorphError::ErrorFatalHardwareFailure.is_fatal());
    
    // Test error descriptions
    assert_eq!(NeuromorphError::Success.description(), "Success");
    assert_eq!(NeuromorphError::ErrorInvalidValue.description(), "Invalid value");
}

#[test]
fn test_dim3() {
    let dim = NeuromorphDim3::new(10, 20, 30);
    assert_eq!(dim.x, 10);
    assert_eq!(dim.y, 20);
    assert_eq!(dim.z, 30);
    
    let dim_1d = NeuromorphDim3::from_1d(100);
    assert_eq!(dim_1d.x, 100);
    assert_eq!(dim_1d.y, 1);
    assert_eq!(dim_1d.z, 1);
    
    let dim_2d = NeuromorphDim3::from_2d(50, 75);
    assert_eq!(dim_2d.x, 50);
    assert_eq!(dim_2d.y, 75);
    assert_eq!(dim_2d.z, 1);
}
