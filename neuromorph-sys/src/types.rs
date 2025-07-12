//! Type definitions for Neuromorph FFI
//! 
//! Mirrors CUDA's driver object model: Context, Stream, Event, DeviceMemory, etc.
//! Each wraps a raw handle and implements Drop for automatic resource cleanup.

use std::os::raw::{c_int, c_uint, c_void, c_char, c_ulong};

/// Opaque handle types - mirror CUDA's design
pub type NeuromorphContext = *mut c_void;
pub type NeuromorphDevice = c_int;
pub type NeuromorphStream = *mut c_void;
pub type NeuromorphEvent = *mut c_void;
pub type NeuromorphDevicePtr = *mut c_void;
pub type NeuromorphKernel = *mut c_void;
pub type NeuromorphModule = *mut c_void;

/// Memory copy kinds
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeuromorphMemcpyKind {
    HostToHost = 0,
    HostToDevice = 1,
    DeviceToHost = 2,
    DeviceToDevice = 3,
}

/// Device properties structure
#[repr(C)]
#[derive(Debug, Clone)]
pub struct NeuromorphDeviceProperties {
    /// Device name
    pub name: [c_char; 256],
    /// Total global memory in bytes
    pub total_global_mem: usize,
    /// Total shared memory per block in bytes  
    pub shared_mem_per_block: usize,
    /// Total registers per block
    pub regs_per_block: c_int,
    /// Warp size in neurons
    pub warp_size: c_int,
    /// Maximum memory pitch
    pub mem_pitch: usize,
    /// Maximum neurons per block
    pub max_neurons_per_block: c_int,
    /// Maximum block dimensions
    pub max_neurons_dim: [c_int; 3],
    /// Maximum grid dimensions
    pub max_grid_size: [c_int; 3],
    /// Clock frequency in kilohertz
    pub clock_rate: c_int,
    /// Total constant memory in bytes
    pub total_const_mem: usize,
    /// Device capability major version
    pub major: c_int,
    /// Device capability minor version
    pub minor: c_int,
    /// Alignment requirement for textures
    pub texture_alignment: usize,
    /// Pitch alignment requirement for texture references
    pub texture_pitch_alignment: usize,
    /// Device can concurrently copy memory and execute a kernel
    pub device_overlap: c_int,
    /// Number of multiprocessors on device
    pub multi_processor_count: c_int,
    /// Specified whether there is a run time limit on kernels
    pub kernel_exec_timeout_enabled: c_int,
    /// Device is integrated as opposed to discrete
    pub integrated: c_int,
    /// Device can map host memory with neuromorphHostAlloc/neuromorphHostGetDevicePointer
    pub can_map_host_memory: c_int,
    /// Compute mode (default, exclusive, prohibited, exclusive process)
    pub compute_mode: c_int,
    /// Maximum 1D texture size
    pub max_texture_1d: c_int,
    /// Maximum 1D mipmapped texture size
    pub max_texture_1d_mipmap: c_int,
    /// Maximum size for 1D textures bound to linear memory
    pub max_texture_1d_linear: c_int,
    /// Maximum 2D texture dimensions
    pub max_texture_2d: [c_int; 2],
    /// Maximum 2D mipmapped texture dimensions
    pub max_texture_2d_mipmap: [c_int; 2],
    /// Maximum dimensions (width, height, pitch) for 2D textures bound to pitched memory
    pub max_texture_2d_linear: [c_int; 3],
    /// Maximum 2D texture dimensions if texture gather operations have to be performed
    pub max_texture_2d_gather: [c_int; 2],
    /// Maximum 3D texture dimensions
    pub max_texture_3d: [c_int; 3],
    /// Maximum 3D texture dimensions if volume textures are disabled
    pub max_texture_3d_alt: [c_int; 3],
    /// Maximum Cubemap texture dimensions
    pub max_texture_cubemap: c_int,
    /// Maximum 1D layered texture dimensions
    pub max_texture_1d_layered: [c_int; 2],
    /// Maximum 2D layered texture dimensions
    pub max_texture_2d_layered: [c_int; 3],
    /// Maximum Cubemap layered texture dimensions
    pub max_texture_cubemap_layered: [c_int; 2],
    /// Maximum 1D surface size
    pub max_surface_1d: c_int,
    /// Maximum 2D surface dimensions
    pub max_surface_2d: [c_int; 2],
    /// Maximum 3D surface dimensions
    pub max_surface_3d: [c_int; 3],
    /// Maximum 1D layered surface dimensions
    pub max_surface_1d_layered: [c_int; 2],
    /// Maximum 2D layered surface dimensions
    pub max_surface_2d_layered: [c_int; 3],
    /// Maximum Cubemap surface dimensions
    pub max_surface_cubemap: c_int,
    /// Maximum Cubemap layered surface dimensions
    pub max_surface_cubemap_layered: [c_int; 2],
    /// Alignment requirements for surfaces
    pub surface_alignment: usize,
    /// Device can possibly execute multiple kernels concurrently
    pub concurrent_kernels: c_int,
    /// Device has ECC support enabled
    pub ecc_enabled: c_int,
    /// PCI bus ID of the device
    pub pci_bus_id: c_int,
    /// PCI device ID of the device
    pub pci_device_id: c_int,
    /// PCI domain ID of the device
    pub pci_domain_id: c_int,
    /// 1 if device is a Tesla device using TCC driver, 0 otherwise
    pub tcc_driver: c_int,
    /// Number of asynchronous engines
    pub async_engine_count: c_int,
    /// Device shares a unified address space with the host
    pub unified_addressing: c_int,
    /// Peak memory clock frequency in kilohertz
    pub memory_clock_rate: c_int,
    /// Global memory bus width in bits
    pub memory_bus_width: c_int,
    /// Size of L2 cache in bytes
    pub l2_cache_size: c_int,
    /// Maximum resident neurons per multiprocessor
    pub max_neurons_per_multiprocessor: c_int,
    /// Device supports stream priorities
    pub stream_priorities_supported: c_int,
    /// Device supports caching globals in L1
    pub global_l1_cache_supported: c_int,
    /// Device supports caching locals in L1
    pub local_l1_cache_supported: c_int,
    /// Shared memory available per multiprocessor in bytes
    pub shared_mem_per_multiprocessor: usize,
    /// 32-bit registers available per multiprocessor
    pub regs_per_multiprocessor: c_int,
    /// Device supports allocating managed memory on this system
    pub managed_memory: c_int,
    /// Device is on a multi-GPU board
    pub is_multi_gpu_board: c_int,
    /// Unique identifier for a group of devices on the same multi-GPU board
    pub multi_gpu_board_group_id: c_int,
    /// Link between the device and the host supports native atomic operations
    pub host_native_atomic_supported: c_int,
    /// Ratio of single precision to double precision performance
    pub single_to_double_precision_perf_ratio: c_int,
    /// Device supports coherently accessing pageable memory
    pub pageable_memory_access: c_int,
    /// Device can coherently access managed memory concurrently with the CPU
    pub concurrent_managed_access: c_int,
    /// Device supports Compute Preemption
    pub compute_preemption_supported: c_int,
    /// Device can access host registered memory at the same virtual address as the CPU
    pub can_use_host_pointer_for_registered_mem: c_int,
    /// neuromorphStreamBatchMemOp and related APIs are supported
    pub cooperative_launch: c_int,
    /// Device supports launching cooperative kernels via neuromorphLaunchCooperativeKernel
    pub cooperative_multi_device_launch: c_int,
    /// The maximum optin shared memory per block
    pub shared_mem_per_block_optin: usize,
    /// Device supports flushing of outstanding remote writes
    pub pageable_memory_access_uses_host_page_tables: c_int,
    /// Device supports coherently accessing pageable memory
    pub direct_managed_mem_access_from_host: c_int,
}

/// 3D dimensions structure
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NeuromorphDim3 {
    pub x: c_uint,
    pub y: c_uint,
    pub z: c_uint,
}

impl NeuromorphDim3 {
    pub fn new(x: c_uint, y: c_uint, z: c_uint) -> Self {
        Self { x, y, z }
    }
    
    pub fn from_1d(x: c_uint) -> Self {
        Self::new(x, 1, 1)
    }
    
    pub fn from_2d(x: c_uint, y: c_uint) -> Self {
        Self::new(x, y, 1)
    }
}

impl Default for NeuromorphDim3 {
    fn default() -> Self {
        Self::new(1, 1, 1)
    }
}

/// Kernel launch parameters
#[repr(C)]
#[derive(Debug)]
pub struct NeuromorphKernelNodeParams {
    pub func: NeuromorphKernel,
    pub grid_dim: NeuromorphDim3,
    pub block_dim: NeuromorphDim3,
    pub shared_mem_bytes: c_uint,
    pub kern_params: *mut *mut c_void,
    pub extra: *mut *mut c_void,
}

/// Memory allocation type
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeuromorphMemoryType {
    Host = 1,
    Device = 2,
    Array = 3,
    Unified = 4,
}

/// Memory pool properties
#[repr(C)]
#[derive(Debug)]
pub struct NeuromorphMemPoolProps {
    pub alloc_type: NeuromorphMemoryType,
    pub handle_types: c_int,
    pub location: NeuromorphMemLocation,
    pub win32_security_attributes: *mut c_void,
    pub reserved: [c_ulong; 64],
}

/// Memory location
#[repr(C)]
#[derive(Debug)]
pub struct NeuromorphMemLocation {
    pub location_type: NeuromorphMemoryType,
    pub id: c_int,
}

/// Stream callback function type
pub type NeuromorphStreamCallback = unsafe extern "C" fn(
    stream: NeuromorphStream,
    status: NeuromorphResult,
    user_data: *mut c_void,
);

/// Host function type for streams
pub type NeuromorphHostFn = unsafe extern "C" fn(user_data: *mut c_void);

/// Context creation flags
pub const NEUROMORPH_CTX_SCHED_AUTO: c_uint = 0x00;
pub const NEUROMORPH_CTX_SCHED_SPIN: c_uint = 0x01;
pub const NEUROMORPH_CTX_SCHED_YIELD: c_uint = 0x02;
pub const NEUROMORPH_CTX_SCHED_BLOCKING_SYNC: c_uint = 0x04;
pub const NEUROMORPH_CTX_MAP_HOST: c_uint = 0x08;
pub const NEUROMORPH_CTX_LMEM_RESIZE_TO_MAX: c_uint = 0x10;

/// Event creation flags
pub const NEUROMORPH_EVENT_DEFAULT: c_uint = 0x00;
pub const NEUROMORPH_EVENT_BLOCKING_SYNC: c_uint = 0x01;
pub const NEUROMORPH_EVENT_DISABLE_TIMING: c_uint = 0x02;
pub const NEUROMORPH_EVENT_INTERPROCESS: c_uint = 0x04;

/// Stream creation flags
pub const NEUROMORPH_STREAM_DEFAULT: c_uint = 0x00;
pub const NEUROMORPH_STREAM_NON_BLOCKING: c_uint = 0x01;

use crate::error::*;

impl Default for NeuromorphDeviceProperties {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

impl Default for NeuromorphKernelNodeParams {
    fn default() -> Self {
        Self {
            func: std::ptr::null_mut(),
            grid_dim: NeuromorphDim3::default(),
            block_dim: NeuromorphDim3::default(),
            shared_mem_bytes: 0,
            kern_params: std::ptr::null_mut(),
            extra: std::ptr::null_mut(),
        }
    }
}

impl Default for NeuromorphMemPoolProps {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

impl Default for NeuromorphMemLocation {
    fn default() -> Self {
        Self {
            location_type: NeuromorphMemoryType::Device,
            id: 0,
        }
    }
}
