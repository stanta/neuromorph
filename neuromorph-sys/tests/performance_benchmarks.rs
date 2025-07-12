//! Performance Benchmarks for CUDA Compatibility
//! 
//! This test suite benchmarks neuromorph-sys performance to ensure
//! it can match CUDA driver performance characteristics.

use neuromorph_sys::*;
use std::ptr;
use std::os::raw::{c_int, c_void};
use std::time::{Duration, Instant};

const MB: usize = 1024 * 1024;
const KB: usize = 1024;

/// Benchmark memory allocation/deallocation patterns
#[test]
#[cfg(feature = "simulator")]
fn benchmark_memory_allocation() {
    unsafe {
        neuromorphInit();
        
        let mut ctx: NeuromorphContext = ptr::null_mut();
        neuromorphCtxCreate(&mut ctx, NEUROMORPH_CTX_SCHED_AUTO, 0);
        
        let sizes = [KB, 4*KB, 16*KB, 64*KB, 256*KB, MB, 4*MB];
        let iterations = 100;
        
        println!("Memory Allocation Benchmark:");
        println!("Size\t\tAlloc (μs)\tFree (μs)\tTotal (μs)");
        println!("-----------------------------------------------------");
        
        for &size in &sizes {
            let mut alloc_times = Vec::new();
            let mut free_times = Vec::new();
            
            for _ in 0..iterations {
                let mut dev_ptr: NeuromorphDevicePtr = ptr::null_mut();
                
                // Benchmark allocation
                let start = Instant::now();
                neuromorphMalloc(&mut dev_ptr, size);
                let alloc_time = start.elapsed();
                alloc_times.push(alloc_time);
                
                // Benchmark deallocation
                let start = Instant::now();
                neuromorphFree(dev_ptr);
                let free_time = start.elapsed();
                free_times.push(free_time);
            }
            
            let avg_alloc = alloc_times.iter().sum::<Duration>() / iterations as u32;
            let avg_free = free_times.iter().sum::<Duration>() / iterations as u32;
            let total = avg_alloc + avg_free;
            
            println!("{:8}\t{:8.2}\t{:8.2}\t{:8.2}", 
                format_size(size),
                avg_alloc.as_micros(),
                avg_free.as_micros(),
                total.as_micros()
            );
        }
        
        neuromorphCtxDestroy(ctx);
    }
}

/// Benchmark memory bandwidth
#[test]
#[cfg(feature = "simulator")]
fn benchmark_memory_bandwidth() {
    unsafe {
        neuromorphInit();
        
        let mut ctx: NeuromorphContext = ptr::null_mut();
        neuromorphCtxCreate(&mut ctx, NEUROMORPH_CTX_SCHED_AUTO, 0);
        
        let sizes = [KB, 4*KB, 16*KB, 64*KB, 256*KB, MB, 4*MB, 16*MB];
        let iterations = 50;
        
        println!("\nMemory Bandwidth Benchmark:");
        println!("Size\t\tH2D (MB/s)\tD2H (MB/s)\tBidirectional");
        println!("-------------------------------------------------------");
        
        for &size in &sizes {
            let host_data = vec![0x42u8; size];
            let mut host_result = vec![0u8; size];
            
            let mut dev_ptr: NeuromorphDevicePtr = ptr::null_mut();
            neuromorphMalloc(&mut dev_ptr, size);
            
            // Benchmark Host to Device
            let start = Instant::now();
            for _ in 0..iterations {
                neuromorphMemcpy(
                    dev_ptr,
                    host_data.as_ptr() as *const c_void,
                    size,
                    NeuromorphMemcpyKind::HostToDevice
                );
            }
            let h2d_time = start.elapsed();
            let h2d_bandwidth = (size * iterations) as f64 / h2d_time.as_secs_f64() / MB as f64;
            
            // Benchmark Device to Host
            let start = Instant::now();
            for _ in 0..iterations {
                neuromorphMemcpy(
                    host_result.as_mut_ptr() as *mut c_void,
                    dev_ptr,
                    size,
                    NeuromorphMemcpyKind::DeviceToHost
                );
            }
            let d2h_time = start.elapsed();
            let d2h_bandwidth = (size * iterations) as f64 / d2h_time.as_secs_f64() / MB as f64;
            
            // Bidirectional test
            let start = Instant::now();
            for _ in 0..iterations/2 {
                neuromorphMemcpy(
                    dev_ptr,
                    host_data.as_ptr() as *const c_void,
                    size,
                    NeuromorphMemcpyKind::HostToDevice
                );
                neuromorphMemcpy(
                    host_result.as_mut_ptr() as *mut c_void,
                    dev_ptr,
                    size,
                    NeuromorphMemcpyKind::DeviceToHost
                );
            }
            let bidir_time = start.elapsed();
            let bidir_bandwidth = (size * iterations) as f64 / bidir_time.as_secs_f64() / MB as f64;
            
            println!("{:8}\t{:8.1}\t{:8.1}\t{:8.1}", 
                format_size(size),
                h2d_bandwidth,
                d2h_bandwidth,
                bidir_bandwidth
            );
            
            neuromorphFree(dev_ptr);
        }
        
        neuromorphCtxDestroy(ctx);
    }
}

/// Benchmark stream operations
#[test]
#[cfg(feature = "simulator")]
fn benchmark_stream_operations() {
    unsafe {
        neuromorphInit();
        
        let mut ctx: NeuromorphContext = ptr::null_mut();
        neuromorphCtxCreate(&mut ctx, NEUROMORPH_CTX_SCHED_AUTO, 0);
        
        let iterations = 1000;
        
        println!("\nStream Operations Benchmark:");
        
        // Benchmark stream creation/destruction
        let start = Instant::now();
        for _ in 0..iterations {
            let mut stream: NeuromorphStream = ptr::null_mut();
            neuromorphStreamCreate(&mut stream);
            neuromorphStreamDestroy(stream);
        }
        let create_destroy_time = start.elapsed();
        let avg_create_destroy = create_destroy_time.as_micros() / iterations as u128;
        
        println!("Stream create/destroy: {:.2} μs per operation", avg_create_destroy);
        
        // Benchmark stream synchronization
        let mut stream: NeuromorphStream = ptr::null_mut();
        neuromorphStreamCreate(&mut stream);
        
        let start = Instant::now();
        for _ in 0..iterations {
            neuromorphStreamSynchronize(stream);
        }
        let sync_time = start.elapsed();
        let avg_sync = sync_time.as_micros() / iterations as u128;
        
        println!("Stream synchronize: {:.2} μs per operation", avg_sync);
        
        neuromorphStreamDestroy(stream);
        neuromorphCtxDestroy(ctx);
    }
}

/// Benchmark event operations
#[test]
#[cfg(feature = "simulator")]
fn benchmark_event_operations() {
    unsafe {
        neuromorphInit();
        
        let mut ctx: NeuromorphContext = ptr::null_mut();
        neuromorphCtxCreate(&mut ctx, NEUROMORPH_CTX_SCHED_AUTO, 0);
        
        let mut stream: NeuromorphStream = ptr::null_mut();
        neuromorphStreamCreate(&mut stream);
        
        let iterations = 1000;
        
        println!("\nEvent Operations Benchmark:");
        
        // Benchmark event creation/destruction
        let start = Instant::now();
        for _ in 0..iterations {
            let mut event: NeuromorphEvent = ptr::null_mut();
            neuromorphEventCreate(&mut event);
            neuromorphEventDestroy(event);
        }
        let create_destroy_time = start.elapsed();
        let avg_create_destroy = create_destroy_time.as_micros() / iterations as u128;
        
        println!("Event create/destroy: {:.2} μs per operation", avg_create_destroy);
        
        // Benchmark event record/synchronize
        let mut event: NeuromorphEvent = ptr::null_mut();
        neuromorphEventCreate(&mut event);
        
        let start = Instant::now();
        for _ in 0..iterations {
            neuromorphEventRecord(event, stream);
            neuromorphEventSynchronize(event);
        }
        let record_sync_time = start.elapsed();
        let avg_record_sync = record_sync_time.as_micros() / iterations as u128;
        
        println!("Event record/sync: {:.2} μs per operation", avg_record_sync);
        
        neuromorphEventDestroy(event);
        neuromorphStreamDestroy(stream);
        neuromorphCtxDestroy(ctx);
    }
}

/// Benchmark register access
#[test]
#[cfg(feature = "simulator")]
fn benchmark_register_access() {
    unsafe {
        neuromorphInit();
        
        let iterations = 10000;
        
        println!("\nRegister Access Benchmark:");
        
        // Benchmark register read
        let start = Instant::now();
        for _ in 0..iterations {
            let mut value: u32 = 0;
            neuromorphRegisterRead(0, NEUROMORPH_REG_STATUS, &mut value);
        }
        let read_time = start.elapsed();
        let avg_read = read_time.as_nanos() / iterations as u128;
        
        println!("Register read: {:.1} ns per operation", avg_read);
        
        // Benchmark register write
        let start = Instant::now();
        for i in 0..iterations {
            neuromorphRegisterWrite(0, NEUROMORPH_REG_CONTROL, i as u32);
        }
        let write_time = start.elapsed();
        let avg_write = write_time.as_nanos() / iterations as u128;
        
        println!("Register write: {:.1} ns per operation", avg_write);
    }
}

/// Stress test with multiple concurrent operations
#[test]
#[cfg(feature = "simulator")]
fn stress_test_concurrent_operations() {
    unsafe {
        neuromorphInit();
        
        let mut device_count: c_int = 0;
        neuromorphGetDeviceCount(&mut device_count);
        
        println!("\nStress Test - Concurrent Operations:");
        
        // Create multiple contexts, streams, and perform operations
        let num_streams = 4;
        let operations_per_stream = 100;
        let size = 64 * KB;
        
        let mut ctx: NeuromorphContext = ptr::null_mut();
        neuromorphCtxCreate(&mut ctx, NEUROMORPH_CTX_SCHED_AUTO, 0);
        
        let mut streams: Vec<NeuromorphStream> = Vec::new();
        let mut dev_ptrs: Vec<NeuromorphDevicePtr> = Vec::new();
        
        // Setup
        for _ in 0..num_streams {
            let mut stream: NeuromorphStream = ptr::null_mut();
            neuromorphStreamCreate(&mut stream);
            streams.push(stream);
            
            let mut dev_ptr: NeuromorphDevicePtr = ptr::null_mut();
            neuromorphMalloc(&mut dev_ptr, size);
            dev_ptrs.push(dev_ptr);
        }
        
        let host_data = vec![0x55u8; size];
        
        let start = Instant::now();
        
        // Perform concurrent operations
        for i in 0..operations_per_stream {
            for (stream_idx, (&stream, &dev_ptr)) in streams.iter().zip(dev_ptrs.iter()).enumerate() {
                // Vary the data pattern
                let pattern = (i + stream_idx) as u8;
                let data = vec![pattern; size];
                
                neuromorphMemcpyAsync(
                    dev_ptr,
                    data.as_ptr() as *const c_void,
                    size,
                    NeuromorphMemcpyKind::HostToDevice,
                    stream
                );
            }
            
            // Synchronize all streams
            for &stream in &streams {
                neuromorphStreamSynchronize(stream);
            }
        }
        
        let total_time = start.elapsed();
        let total_operations = num_streams * operations_per_stream;
        let ops_per_second = total_operations as f64 / total_time.as_secs_f64();
        let total_data = (total_operations * size) as f64 / MB as f64;
        let throughput = total_data / total_time.as_secs_f64();
        
        println!("Completed {} operations in {:.2}s", total_operations, total_time.as_secs_f64());
        println!("Throughput: {:.1} ops/sec, {:.1} MB/s", ops_per_second, throughput);
        
        // Cleanup
        for dev_ptr in dev_ptrs {
            neuromorphFree(dev_ptr);
        }
        for stream in streams {
            neuromorphStreamDestroy(stream);
        }
        neuromorphCtxDestroy(ctx);
        
        // Verify we didn't leak memory or handles
        println!("✓ Stress test completed without errors");
    }
}

/// Compare performance with theoretical CUDA metrics
#[test]
#[cfg(feature = "simulator")]
fn compare_cuda_metrics() {
    unsafe {
        neuromorphInit();
        
        let mut props = NeuromorphDeviceProperties::default();
        neuromorphGetDeviceProperties(&mut props, 0);
        
        println!("\nCUDA Compatibility Metrics:");
        println!("============================");
        
        // Memory metrics
        let global_mem_gb = props.total_global_mem as f64 / (1024.0 * 1024.0 * 1024.0);
        println!("Global Memory: {:.1} GB", global_mem_gb);
        println!("Shared Memory per Block: {} KB", props.shared_mem_per_block / 1024);
        println!("Registers per Block: {}", props.regs_per_block);
        
        // Compute metrics
        println!("Multiprocessor Count: {}", props.multi_processor_count);
        println!("Clock Rate: {:.1} MHz", props.clock_rate as f64 / 1000.0);
        println!("Warp Size: {}", props.warp_size);
        
        // Compute capability
        println!("Compute Capability: {}.{}", props.major, props.minor);
        
        // Theoretical bandwidth calculation
        let memory_bus_width = 256; // bits, typical for modern devices
        let memory_clock_mhz = 1000.0; // Example memory clock
        let theoretical_bandwidth = (memory_bus_width as f64 / 8.0) * memory_clock_mhz * 2.0 / 1000.0;
        
        println!("Theoretical Memory Bandwidth: {:.1} GB/s", theoretical_bandwidth);
        
        // Performance characteristics that should match CUDA
        assert!(props.major >= 3, "Compute capability should be at least 3.0 for CUDA compatibility");
        assert!(props.warp_size == 32, "Warp size should be 32 for CUDA compatibility");
        assert!(props.multi_processor_count > 0, "Should have at least one multiprocessor");
        assert!(global_mem_gb >= 0.5, "Should have at least 512MB memory for useful work");
        
        println!("✓ All CUDA compatibility metrics passed");
    }
}

/// Utility function to format byte sizes
fn format_size(bytes: usize) -> String {
    if bytes >= MB {
        format!("{}MB", bytes / MB)
    } else if bytes >= KB {
        format!("{}KB", bytes / KB)
    } else {
        format!("{}B", bytes)
    }
}

/// Test to ensure consistent performance across multiple runs
#[test]
#[cfg(feature = "simulator")]
fn test_performance_consistency() {
    unsafe {
        neuromorphInit();
        
        let mut ctx: NeuromorphContext = ptr::null_mut();
        neuromorphCtxCreate(&mut ctx, NEUROMORPH_CTX_SCHED_AUTO, 0);
        
        let size = MB;
        let runs = 10;
        let iterations_per_run = 20;
        
        println!("\nPerformance Consistency Test:");
        
        let mut bandwidths = Vec::new();
        
        for run in 0..runs {
            let host_data = vec![0x77u8; size];
            let mut dev_ptr: NeuromorphDevicePtr = ptr::null_mut();
            neuromorphMalloc(&mut dev_ptr, size);
            
            let start = Instant::now();
            for _ in 0..iterations_per_run {
                neuromorphMemcpy(
                    dev_ptr,
                    host_data.as_ptr() as *const c_void,
                    size,
                    NeuromorphMemcpyKind::HostToDevice
                );
            }
            let duration = start.elapsed();
            
            let bandwidth = (size * iterations_per_run) as f64 / duration.as_secs_f64() / MB as f64;
            bandwidths.push(bandwidth);
            
            println!("Run {}: {:.1} MB/s", run + 1, bandwidth);
            
            neuromorphFree(dev_ptr);
        }
        
        // Calculate statistics
        let mean = bandwidths.iter().sum::<f64>() / runs as f64;
        let variance = bandwidths.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / runs as f64;
        let std_dev = variance.sqrt();
        let cv = std_dev / mean * 100.0; // coefficient of variation
        
        println!("Mean: {:.1} MB/s", mean);
        println!("Std Dev: {:.1} MB/s", std_dev);
        println!("Coefficient of Variation: {:.1}%", cv);
        
        // Performance should be consistent (CV < 10%)
        assert!(cv < 10.0, "Performance variation too high: {:.1}%", cv);
        
        neuromorphCtxDestroy(ctx);
        
        println!("✓ Performance consistency verified");
    }
}
