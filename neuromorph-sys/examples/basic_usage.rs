//! Simple example demonstrating neuromorph-sys usage

use neuromorph_sys::*;
use std::ptr;
use std::os::raw::c_int;

fn main() {
    unsafe {
        // Initialize the neuromorph driver
        println!("Initializing Neuromorph driver...");
        let result = neuromorphInit();
        if result != NEUROMORPH_SUCCESS {
            eprintln!("Failed to initialize driver: {}", result);
            return;
        }
        println!("Driver initialized successfully!");
        
        // Get device count
        let mut device_count: c_int = 0;
        let result = neuromorphGetDeviceCount(&mut device_count);
        if result != NEUROMORPH_SUCCESS {
            eprintln!("Failed to get device count: {}", result);
            return;
        }
        println!("Found {} neuromorphic device(s)", device_count);
        
        if device_count == 0 {
            println!("No devices available. This is expected when using hardware feature without actual hardware.");
            return;
        }
        
        // Get properties for the first device
        let mut props = NeuromorphDeviceProperties::default();
        let result = neuromorphGetDeviceProperties(&mut props, 0);
        if result != NEUROMORPH_SUCCESS {
            eprintln!("Failed to get device properties: {}", result);
            return;
        }
        
        // Print device properties
        let device_name = std::ffi::CStr::from_ptr(props.name.as_ptr())
            .to_string_lossy();
        println!("Device 0: {}", device_name);
        println!("  Total memory: {} MB", props.total_global_mem / (1024 * 1024));
        println!("  Multiprocessors: {}", props.multi_processor_count);
        println!("  Max neurons per block: {}", props.max_neurons_per_block);
        println!("  Clock rate: {} MHz", props.clock_rate / 1000);
        println!("  Compute capability: {}.{}", props.major, props.minor);
        
        // Create a context
        println!("\nCreating context...");
        let mut ctx: NeuromorphContext = ptr::null_mut();
        let result = neuromorphCtxCreate(&mut ctx, NEUROMORPH_CTX_SCHED_AUTO, 0);
        if result != NEUROMORPH_SUCCESS {
            eprintln!("Failed to create context: {}", result);
            return;
        }
        println!("Context created successfully!");
        
        // Create a stream
        println!("Creating stream...");
        let mut stream: NeuromorphStream = ptr::null_mut();
        let result = neuromorphStreamCreate(&mut stream);
        if result != NEUROMORPH_SUCCESS {
            eprintln!("Failed to create stream: {}", result);
            neuromorphCtxDestroy(ctx);
            return;
        }
        println!("Stream created successfully!");
        
        // Allocate device memory
        println!("Allocating device memory...");
        let size = 1024 * 1024; // 1MB
        let mut dev_ptr: NeuromorphDevicePtr = ptr::null_mut();
        let result = neuromorphMalloc(&mut dev_ptr, size);
        if result != NEUROMORPH_SUCCESS {
            eprintln!("Failed to allocate device memory: {}", result);
            neuromorphStreamDestroy(stream);
            neuromorphCtxDestroy(ctx);
            return;
        }
        println!("Allocated {} bytes of device memory", size);
        
        // Test register access
        println!("\nTesting register access...");
        let mut version: u32 = 0;
        let result = neuromorphRegisterRead(0, NEUROMORPH_REG_VERSION, &mut version);
        if result == NEUROMORPH_SUCCESS {
            println!("Device version register: 0x{:08X}", version);
        }
        
        let mut neuron_count: u32 = 0;
        let result = neuromorphRegisterRead(0, NEUROMORPH_REG_NEURON_COUNT, &mut neuron_count);
        if result == NEUROMORPH_SUCCESS {
            println!("Device neuron count: {}", neuron_count);
        }
        
        let mut synapse_count: u32 = 0;
        let result = neuromorphRegisterRead(0, NEUROMORPH_REG_SYNAPSE_COUNT, &mut synapse_count);
        if result == NEUROMORPH_SUCCESS {
            println!("Device synapse count: {}", synapse_count);
        }
        
        // Create and test events
        println!("\nTesting events...");
        let mut event: NeuromorphEvent = ptr::null_mut();
        let result = neuromorphEventCreate(&mut event);
        if result == NEUROMORPH_SUCCESS {
            println!("Event created successfully!");
            
            // Record event in stream
            let result = neuromorphEventRecord(event, stream);
            if result == NEUROMORPH_SUCCESS {
                println!("Event recorded in stream");
            }
            
            // Synchronize with event
            let result = neuromorphEventSynchronize(event);
            if result == NEUROMORPH_SUCCESS {
                println!("Event synchronized successfully");
            }
            
            neuromorphEventDestroy(event);
        }
        
        // Test memory copy
        println!("\nTesting memory operations...");
        let host_data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
        let mut host_result = vec![0u8; 1024];
        
        // Copy host to device
        let result = neuromorphMemcpy(
            dev_ptr,
            host_data.as_ptr() as *const std::os::raw::c_void,
            1024,
            NeuromorphMemcpyKind::HostToDevice
        );
        if result == NEUROMORPH_SUCCESS {
            println!("Host to device copy successful");
            
            // Copy device to host
            let result = neuromorphMemcpy(
                host_result.as_mut_ptr() as *mut std::os::raw::c_void,
                dev_ptr,
                1024,
                NeuromorphMemcpyKind::DeviceToHost
            );
            if result == NEUROMORPH_SUCCESS {
                println!("Device to host copy successful");
                
                // Verify data integrity
                if host_data == host_result {
                    println!("Data integrity verified!");
                } else {
                    println!("Data corruption detected!");
                }
            }
        }
        
        // Synchronize stream
        println!("\nSynchronizing stream...");
        let result = neuromorphStreamSynchronize(stream);
        if result == NEUROMORPH_SUCCESS {
            println!("Stream synchronized successfully");
        }
        
        // Clean up resources
        println!("\nCleaning up resources...");
        neuromorphFree(dev_ptr);
        neuromorphStreamDestroy(stream);
        neuromorphCtxDestroy(ctx);
        
        println!("Example completed successfully!");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_example_init() {
        unsafe {
            let result = neuromorphInit();
            assert!(result == NEUROMORPH_SUCCESS);
        }
    }
}
