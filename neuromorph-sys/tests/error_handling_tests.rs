//! Error Handling Compatibility Tests
//! 
//! This test suite ensures neuromorph-sys error handling matches CUDA patterns

use neuromorph_sys::*;
use std::ptr;
use std::os::raw::c_int;

/// Test CUDA-style error code patterns
#[test]
fn test_cuda_error_code_compatibility() {
    // CUDA uses 0 for success, positive for errors
    // We use 0 for success, negative for recoverable, positive for fatal
    assert_eq!(NEUROMORPH_SUCCESS, 0, "Success should be 0 like CUDA_SUCCESS");
    
    // Test specific error mappings that should match CUDA patterns
    assert!(NEUROMORPH_ERROR_INVALID_VALUE != 0, "Error codes should be non-zero");
    assert!(NEUROMORPH_ERROR_OUT_OF_MEMORY != 0, "OOM should be non-zero");
    assert!(NEUROMORPH_ERROR_INVALID_DEVICE != 0, "Invalid device should be non-zero");
    
    println!("✓ Error codes follow CUDA patterns");
}

/// Test error propagation in typical CUDA workflows
#[test]
fn test_cuda_error_propagation() {
    unsafe {
        // Test uninitialized driver access (should fail gracefully)
        let mut count: c_int = 0;
        let result = neuromorphGetDeviceCount(&mut count);
        
        // This might succeed in simulator mode, but should not crash
        if result != NEUROMORPH_SUCCESS {
            if let Some(error) = NeuromorphError::from_c_int(result) {
                println!("Expected error before init: {}", error.description());
            }
        }
        
        // Initialize properly
        neuromorphInit();
        
        // Test invalid device access
        let mut props = NeuromorphDeviceProperties::default();
        let result = neuromorphGetDeviceProperties(&mut props, 999); // Invalid device
        assert_ne!(result, NEUROMORPH_SUCCESS, "Should fail for invalid device");
        
        if let Some(error) = NeuromorphError::from_c_int(result) {
            assert!(error.is_recoverable(), "Invalid device should be recoverable error");
        }
        
        println!("✓ Error propagation works correctly");
    }
}

/// Test error handling in memory operations
#[test]
#[cfg(feature = "simulator")]
fn test_memory_error_handling() {
    unsafe {
        neuromorphInit();
        
        let mut ctx: NeuromorphContext = ptr::null_mut();
        neuromorphCtxCreate(&mut ctx, NEUROMORPH_CTX_SCHED_AUTO, 0);
        
        // Test invalid memory operations
        let result = neuromorphFree(ptr::null_mut());
        assert_ne!(result, NEUROMORPH_SUCCESS, "Should fail for null pointer");
        
        // Test allocation with invalid size
        let mut dev_ptr: NeuromorphDevicePtr = ptr::null_mut();
        let result = neuromorphMalloc(&mut dev_ptr, 0);
        assert_ne!(result, NEUROMORPH_SUCCESS, "Should fail for zero size");
        
        // Test memory copy with null pointers
        let result = neuromorphMemcpy(
            ptr::null_mut(),
            ptr::null(),
            1024,
            NeuromorphMemcpyKind::HostToDevice
        );
        assert_ne!(result, NEUROMORPH_SUCCESS, "Should fail for null pointers");
        
        neuromorphCtxDestroy(ctx);
        
        println!("✓ Memory error handling verified");
    }
}

/// Test error handling in stream operations
#[test]
#[cfg(feature = "simulator")]
fn test_stream_error_handling() {
    unsafe {
        neuromorphInit();
        
        let mut ctx: NeuromorphContext = ptr::null_mut();
        neuromorphCtxCreate(&mut ctx, NEUROMORPH_CTX_SCHED_AUTO, 0);
        
        // Test operations on invalid streams
        let result = neuromorphStreamSynchronize(ptr::null_mut());
        assert_ne!(result, NEUROMORPH_SUCCESS, "Should fail for null stream");
        
        let result = neuromorphStreamDestroy(ptr::null_mut());
        assert_ne!(result, NEUROMORPH_SUCCESS, "Should fail for null stream");
        
        neuromorphCtxDestroy(ctx);
        
        println!("✓ Stream error handling verified");
    }
}

/// Test error recovery patterns
#[test]
#[cfg(feature = "simulator")]
fn test_error_recovery() {
    unsafe {
        neuromorphInit();
        
        let mut ctx: NeuromorphContext = ptr::null_mut();
        neuromorphCtxCreate(&mut ctx, NEUROMORPH_CTX_SCHED_AUTO, 0);
        
        // Simulate error followed by recovery
        let mut dev_ptr: NeuromorphDevicePtr = ptr::null_mut();
        
        // First attempt with invalid size (should fail)
        let result = neuromorphMalloc(&mut dev_ptr, 0);
        assert_ne!(result, NEUROMORPH_SUCCESS);
        
        // Recovery attempt with valid size (should succeed)
        let result = neuromorphMalloc(&mut dev_ptr, 1024);
        assert_eq!(result, NEUROMORPH_SUCCESS, "Recovery should work after error");
        assert!(!dev_ptr.is_null());
        
        neuromorphFree(dev_ptr);
        neuromorphCtxDestroy(ctx);
        
        println!("✓ Error recovery patterns work");
    }
}

/// Test error classification (recoverable vs fatal)
#[test]
fn test_error_classification() {
    let recoverable_errors = [
        NeuromorphError::ErrorInvalidValue,
        NeuromorphError::ErrorOutOfMemory,
        NeuromorphError::ErrorInvalidDevice,
        NeuromorphError::ErrorInvalidContext,
        NeuromorphError::ErrorInvalidHandle,
    ];
    
    let fatal_errors = [
        NeuromorphError::ErrorFatalHardwareFailure,
        NeuromorphError::ErrorFatalDriverCorruption,
        NeuromorphError::ErrorFatalSystemFailure,
    ];
    
    for error in &recoverable_errors {
        assert!(error.is_recoverable(), "{:?} should be recoverable", error);
        assert!(!error.is_fatal(), "{:?} should not be fatal", error);
        assert!(error.to_c_int() < 0, "{:?} should have negative error code", error);
    }
    
    for error in &fatal_errors {
        assert!(error.is_fatal(), "{:?} should be fatal", error);
        assert!(!error.is_recoverable(), "{:?} should not be recoverable", error);
        assert!(error.to_c_int() > 0, "{:?} should have positive error code", error);
    }
    
    println!("✓ Error classification correct");
}

/// Test error string descriptions
#[test]
fn test_error_descriptions() {
    let test_cases = [
        (NeuromorphError::Success, "Success"),
        (NeuromorphError::ErrorInvalidValue, "Invalid value"),
        (NeuromorphError::ErrorOutOfMemory, "Out of memory"),
        (NeuromorphError::ErrorInvalidDevice, "Invalid device"),
    ];
    
    for (error, expected) in &test_cases {
        let description = error.description();
        assert_eq!(description, *expected, "Wrong description for {:?}", error);
        assert!(!description.is_empty(), "Description should not be empty");
    }
    
    println!("✓ Error descriptions are correct");
}

/// Test error code round-trip conversion
#[test]
fn test_error_roundtrip() {
    let errors = [
        NeuromorphError::Success,
        NeuromorphError::ErrorInvalidValue,
        NeuromorphError::ErrorOutOfMemory,
        NeuromorphError::ErrorFatalHardwareFailure,
    ];
    
    for original_error in &errors {
        let c_code = original_error.to_c_int();
        if let Some(converted_error) = NeuromorphError::from_c_int(c_code) {
            assert_eq!(*original_error, converted_error, "Round-trip conversion failed for {:?}", original_error);
        } else {
            panic!("Failed to convert back from C code {} for {:?}", c_code, original_error);
        }
    }
    
    println!("✓ Error round-trip conversion works");
}

/// Test that error handling doesn't leak resources
#[test]
#[cfg(feature = "simulator")]
fn test_error_resource_cleanup() {
    unsafe {
        neuromorphInit();
        
        let mut ctx: NeuromorphContext = ptr::null_mut();
        neuromorphCtxCreate(&mut ctx, NEUROMORPH_CTX_SCHED_AUTO, 0);
        
        // Create resources that we'll intentionally cause errors with
        let mut stream: NeuromorphStream = ptr::null_mut();
        neuromorphStreamCreate(&mut stream);
        
        let mut event: NeuromorphEvent = ptr::null_mut();
        neuromorphEventCreate(&mut event);
        
        let mut dev_ptr: NeuromorphDevicePtr = ptr::null_mut();
        neuromorphMalloc(&mut dev_ptr, 1024);
        
        // Cause some errors (these should not leak resources)
        neuromorphMemcpy(ptr::null_mut(), ptr::null(), 0, NeuromorphMemcpyKind::HostToDevice);
        neuromorphStreamSynchronize(ptr::null_mut());
        neuromorphEventRecord(ptr::null_mut(), stream);
        
        // Clean up properly (should still work despite errors above)
        let result = neuromorphFree(dev_ptr);
        assert_eq!(result, NEUROMORPH_SUCCESS, "Cleanup should work after errors");
        
        let result = neuromorphEventDestroy(event);
        assert_eq!(result, NEUROMORPH_SUCCESS, "Event cleanup should work");
        
        let result = neuromorphStreamDestroy(stream);
        assert_eq!(result, NEUROMORPH_SUCCESS, "Stream cleanup should work");
        
        let result = neuromorphCtxDestroy(ctx);
        assert_eq!(result, NEUROMORPH_SUCCESS, "Context cleanup should work");
        
        println!("✓ No resource leaks during error conditions");
    }
}

/// Comprehensive error handling test combining all patterns
#[test]
#[cfg(feature = "simulator")]
fn test_comprehensive_error_handling() {
    unsafe {
        println!("Running comprehensive error handling test...");
        
        // 1. Test pre-initialization errors
        let mut count: c_int = 0;
        let result = neuromorphGetDeviceCount(&mut count);
        if result != NEUROMORPH_SUCCESS {
            if let Some(error) = NeuromorphError::from_c_int(result) {
                println!("✓ Pre-init error handled: {}", error.description());
            }
        }
        
        // 2. Initialize and test normal operation
        let result = neuromorphInit();
        assert_eq!(result, NEUROMORPH_SUCCESS);
        println!("✓ Driver initialized successfully");
        
        // 3. Test device enumeration errors
        let mut props = NeuromorphDeviceProperties::default();
        let result = neuromorphGetDeviceProperties(&mut props, -1); // Invalid device
        assert_ne!(result, NEUROMORPH_SUCCESS);
        if let Some(error) = NeuromorphError::from_c_int(result) {
            println!("✓ Invalid device error: {}", error.description());
        }
        
        // 4. Test context errors
        let mut ctx: NeuromorphContext = ptr::null_mut();
        let result = neuromorphCtxCreate(&mut ctx, 0xFFFFFFFF, 999); // Invalid flags and device
        if result != NEUROMORPH_SUCCESS {
            if let Some(error) = NeuromorphError::from_c_int(result) {
                println!("✓ Context creation error: {}", error.description());
            }
        }
        
        // Create valid context for remaining tests
        neuromorphCtxCreate(&mut ctx, NEUROMORPH_CTX_SCHED_AUTO, 0);
        
        // 5. Test memory errors
        let mut dev_ptr: NeuromorphDevicePtr = ptr::null_mut();
        let result = neuromorphMalloc(&mut dev_ptr, 0); // Invalid size
        assert_ne!(result, NEUROMORPH_SUCCESS);
        if let Some(error) = NeuromorphError::from_c_int(result) {
            println!("✓ Memory allocation error: {}", error.description());
        }
        
        // 6. Test stream errors
        let result = neuromorphStreamSynchronize(ptr::null_mut());
        assert_ne!(result, NEUROMORPH_SUCCESS);
        if let Some(error) = NeuromorphError::from_c_int(result) {
            println!("✓ Stream error: {}", error.description());
        }
        
        // 7. Test event errors
        let result = neuromorphEventSynchronize(ptr::null_mut());
        assert_ne!(result, NEUROMORPH_SUCCESS);
        if let Some(error) = NeuromorphError::from_c_int(result) {
            println!("✓ Event error: {}", error.description());
        }
        
        // 8. Test successful operations after errors
        neuromorphMalloc(&mut dev_ptr, 1024);
        assert!(!dev_ptr.is_null());
        neuromorphFree(dev_ptr);
        
        let mut stream: NeuromorphStream = ptr::null_mut();
        neuromorphStreamCreate(&mut stream);
        neuromorphStreamDestroy(stream);
        
        println!("✓ Recovery after errors successful");
        
        // 9. Clean shutdown
        neuromorphCtxDestroy(ctx);
        
        println!("🎉 Comprehensive error handling test passed!");
    }
}
