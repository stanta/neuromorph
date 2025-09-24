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
#[cfg(feature = "simulator")]
fn test_register_batch_operations() {
    unsafe {
        neuromorphInit();

        // Test batch register read/write
        let mut values = [0u32; 3];
        let offsets = [NEUROMORPH_REG_VERSION, NEUROMORPH_REG_NEURON_COUNT, NEUROMORPH_REG_SYNAPSE_COUNT];

        let result = neuromorphRegisterReadBatch(0, offsets.as_ptr(), values.as_mut_ptr(), 3);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        assert_eq!(values[0], 0x03000000); // Version 3.0

        // Write batch
        let write_values = [0x11111111, 0x22222222, 0x33333333];
        let result = neuromorphRegisterWriteBatch(0, offsets.as_ptr(), write_values.as_ptr(), 3);
        assert_eq!(result, NEUROMORPH_SUCCESS);

        // Read back to verify
        let result = neuromorphRegisterReadBatch(0, offsets.as_ptr(), values.as_mut_ptr(), 3);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        assert_eq!(values[0], 0x11111111);
        assert_eq!(values[1], 0x22222222);
        assert_eq!(values[2], 0x33333333);
    }
}

#[test]
#[cfg(feature = "simulator")]
fn test_dma_queue_operations() {
    unsafe {
        neuromorphInit();

        let mut queue: *mut std::os::raw::c_void = ptr::null_mut();
        let result = neuromorphDmaQueueCreate(0, &mut queue, 0);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        assert!(!queue.is_null());

        // Test DMA submit
        let mut transfer_id: u32 = 0;
        let test_data = [0x42u8; 1024];
        let result = neuromorphDmaSubmit(queue, test_data.as_ptr() as *mut std::os::raw::c_void,
                                       test_data.as_ptr() as *const std::os::raw::c_void,
                                       test_data.len(), &mut transfer_id);
        assert_eq!(result, NEUROMORPH_SUCCESS);

        // Test DMA wait
        let result = neuromorphDmaWait(queue, transfer_id, 1000);
        assert_eq!(result, NEUROMORPH_SUCCESS);

        // Test DMA query
        let mut completed: i32 = 0;
        let result = neuromorphDmaQuery(queue, transfer_id, &mut completed);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        assert_eq!(completed, 1); // Should be completed

        let result = neuromorphDmaQueueDestroy(queue);
        assert_eq!(result, NEUROMORPH_SUCCESS);
    }
}

#[test]
#[cfg(feature = "simulator")]
fn test_irq_operations() {
    unsafe {
        neuromorphInit();

        // Test IRQ registration (simulator just succeeds)
        extern "C" fn dummy_handler(_device: NeuromorphDevice, _irq_source: u32, _user_data: *mut std::os::raw::c_void) {}
        let result = neuromorphIrqRegister(0, 0x1, dummy_handler, ptr::null_mut());
        assert_eq!(result, NEUROMORPH_SUCCESS);

        // Test IRQ enable/disable
        let result = neuromorphIrqEnable(0, 0x1);
        assert_eq!(result, NEUROMORPH_SUCCESS);

        let result = neuromorphIrqDisable(0, 0x1);
        assert_eq!(result, NEUROMORPH_SUCCESS);

        // Test IRQ status
        let mut status: u32 = 0;
        let result = neuromorphIrqGetStatus(0, &mut status);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        assert_eq!(status, 0); // No pending interrupts in simulator

        // Test IRQ clear
        let result = neuromorphIrqClear(0, 0x1);
        assert_eq!(result, NEUROMORPH_SUCCESS);

        // Test IRQ unregister
        let result = neuromorphIrqUnregister(0, 0x1);
        assert_eq!(result, NEUROMORPH_SUCCESS);
    }
}

#[test]
#[cfg(feature = "simulator")]
fn test_hardware_event_operations() {
    unsafe {
        neuromorphInit();

        let mut event: *mut std::os::raw::c_void = ptr::null_mut();
        let result = neuromorphHwEventCreate(0, &mut event, 0);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        assert!(!event.is_null());

        // Test event signal
        let result = neuromorphHwEventSignal(event);
        assert_eq!(result, NEUROMORPH_SUCCESS);

        // Test event wait
        let result = neuromorphHwEventWait(event, 100);
        assert_eq!(result, NEUROMORPH_SUCCESS);

        // Test event reset
        let result = neuromorphHwEventReset(event);
        assert_eq!(result, NEUROMORPH_SUCCESS);

        let result = neuromorphHwEventDestroy(event);
        assert_eq!(result, NEUROMORPH_SUCCESS);
    }
}

#[test]
#[cfg(feature = "simulator")]
fn test_memory_mapping() {
    unsafe {
        neuromorphInit();

        let mut ctx: NeuromorphContext = ptr::null_mut();
        neuromorphCtxCreate(&mut ctx, NEUROMORPH_CTX_SCHED_AUTO, 0);

        let mut dev_ptr: NeuromorphDevicePtr = ptr::null_mut();
        let size = 4096;
        neuromorphMalloc(&mut dev_ptr, size);

        let mut host_ptr: *mut std::os::raw::c_void = ptr::null_mut();
        let result = neuromorphMemMap(&mut host_ptr, dev_ptr, size, 0);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        assert!(!host_ptr.is_null());

        // In simulator, mapped pointer should equal device pointer
        assert_eq!(host_ptr, dev_ptr);

        let result = neuromorphMemUnmap(host_ptr, size);
        assert_eq!(result, NEUROMORPH_SUCCESS);

        neuromorphFree(dev_ptr);
        neuromorphCtxDestroy(ctx);
    }
}

#[test]
#[cfg(feature = "simulator")]
fn test_power_management() {
    unsafe {
        neuromorphInit();

        // Test power state get/set
        let mut power_state: u32 = 0;
        let result = neuromorphPowerGetState(0, &mut power_state);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        assert_eq!(power_state, NEUROMORPH_POWER_STATE_ACTIVE);

        let result = neuromorphPowerSetState(0, NEUROMORPH_POWER_STATE_IDLE);
        assert_eq!(result, NEUROMORPH_SUCCESS);

        let result = neuromorphPowerGetState(0, &mut power_state);
        assert_eq!(result, NEUROMORPH_SUCCESS);
        assert_eq!(power_state, NEUROMORPH_POWER_STATE_IDLE);
    }
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
