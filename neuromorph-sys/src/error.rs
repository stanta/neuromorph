//! Error types for Neuromorph FFI
//! 
//! Maps cleanly to numeric C codes:
//! - 0 = success
//! - negative = recoverable errors
//! - positive = fatal errors

use std::os::raw::c_int;

/// Result type for Neuromorph operations
pub type NeuromorphResult = c_int;

/// Neuromorph error codes that map to C integer codes
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeuromorphError {
    // Success
    Success = 0,
    
    // Recoverable errors (negative values)
    ErrorMemoryAllocation = -1,
    ErrorMemoryAlignment = -2,
    ErrorInvalidValue = -3,
    ErrorInvalidHandle = -4,
    ErrorInvalidDevice = -5,
    ErrorInvalidContext = -6,
    ErrorOutOfMemory = -7,
    ErrorNotReady = -8,
    ErrorTimeout = -9,
    ErrorStreamNotReady = -10,
    ErrorEventNotReady = -11,
    ErrorKernelNotFound = -12,
    ErrorInvalidConfiguration = -13,
    ErrorLaunchFailure = -14,
    ErrorLaunchTimeout = -15,
    ErrorLaunchOutOfResources = -16,
    ErrorInvalidDeviceFunction = -17,
    ErrorDeviceNotLicensed = -18,
    ErrorSoftwareValidityNotEstablished = -19,
    ErrorStartupFailure = -20,
    ErrorInsufficientDriver = -21,
    ErrorNoKernelImageForDevice = -22,
    ErrorIncompatibleDriverContext = -23,
    ErrorPeerAccessAlreadyEnabled = -24,
    ErrorPeerAccessNotEnabled = -25,
    ErrorSetOnActiveProcess = -26,
    ErrorOperatingSystem = -27,
    ErrorEccUncorrectable = -28,
    ErrorSharedObjectSymbolNotFound = -29,
    ErrorSharedObjectInitFailed = -30,
    ErrorUnsupportedLimit = -31,
    ErrorDuplicateVariableName = -32,
    ErrorDuplicateTextureName = -33,
    ErrorDuplicateSurfaceName = -34,
    ErrorDevicesUnavailable = -35,
    ErrorArrayIsMapped = -36,
    ErrorAlreadyMapped = -37,
    ErrorNoDevice = -38,
    ErrorNotMapped = -39,
    ErrorNotMappedAsArray = -40,
    ErrorNotMappedAsPointer = -41,
    ErrorAlreadyAcquired = -42,
    ErrorNotPermitted = -43,
    
    // Fatal errors (positive values)
    ErrorFatalHardwareFailure = 1,
    ErrorFatalDriverCorruption = 2,
    ErrorFatalSystemFailure = 3,
    ErrorFatalInternalError = 4,
    ErrorFatalUnknown = 5,
    ErrorFatalInitializationFailure = 6,
    ErrorFatalDeinitialization = 7,
    ErrorFatalProfilerDisabled = 8,
    ErrorFatalProfilerNotInitialized = 9,
    ErrorFatalProfilerAlreadyStarted = 10,
    ErrorFatalProfilerAlreadyStopped = 11,
    ErrorFatalAssert = 12,
    ErrorFatalIllegalInstruction = 13,
    ErrorFatalHardwareStackError = 14,
    ErrorFatalIllegalAddress = 15,
    ErrorFatalInvalidAddressSpace = 16,
    ErrorFatalInvalidPc = 17,
    ErrorFatalNotPermitted = 18,
}

impl NeuromorphError {
    /// Convert to C integer code
    pub fn to_c_int(self) -> c_int {
        self as c_int
    }
    
    /// Convert from C integer code
    pub fn from_c_int(code: c_int) -> Option<Self> {
        match code {
            0 => Some(Self::Success),
            
            // Recoverable errors
            -1 => Some(Self::ErrorMemoryAllocation),
            -2 => Some(Self::ErrorMemoryAlignment),
            -3 => Some(Self::ErrorInvalidValue),
            -4 => Some(Self::ErrorInvalidHandle),
            -5 => Some(Self::ErrorInvalidDevice),
            -6 => Some(Self::ErrorInvalidContext),
            -7 => Some(Self::ErrorOutOfMemory),
            -8 => Some(Self::ErrorNotReady),
            -9 => Some(Self::ErrorTimeout),
            -10 => Some(Self::ErrorStreamNotReady),
            -11 => Some(Self::ErrorEventNotReady),
            -12 => Some(Self::ErrorKernelNotFound),
            -13 => Some(Self::ErrorInvalidConfiguration),
            -14 => Some(Self::ErrorLaunchFailure),
            -15 => Some(Self::ErrorLaunchTimeout),
            -16 => Some(Self::ErrorLaunchOutOfResources),
            -17 => Some(Self::ErrorInvalidDeviceFunction),
            -18 => Some(Self::ErrorDeviceNotLicensed),
            -19 => Some(Self::ErrorSoftwareValidityNotEstablished),
            -20 => Some(Self::ErrorStartupFailure),
            -21 => Some(Self::ErrorInsufficientDriver),
            -22 => Some(Self::ErrorNoKernelImageForDevice),
            -23 => Some(Self::ErrorIncompatibleDriverContext),
            -24 => Some(Self::ErrorPeerAccessAlreadyEnabled),
            -25 => Some(Self::ErrorPeerAccessNotEnabled),
            -26 => Some(Self::ErrorSetOnActiveProcess),
            -27 => Some(Self::ErrorOperatingSystem),
            -28 => Some(Self::ErrorEccUncorrectable),
            -29 => Some(Self::ErrorSharedObjectSymbolNotFound),
            -30 => Some(Self::ErrorSharedObjectInitFailed),
            -31 => Some(Self::ErrorUnsupportedLimit),
            -32 => Some(Self::ErrorDuplicateVariableName),
            -33 => Some(Self::ErrorDuplicateTextureName),
            -34 => Some(Self::ErrorDuplicateSurfaceName),
            -35 => Some(Self::ErrorDevicesUnavailable),
            -36 => Some(Self::ErrorArrayIsMapped),
            -37 => Some(Self::ErrorAlreadyMapped),
            -38 => Some(Self::ErrorNoDevice),
            -39 => Some(Self::ErrorNotMapped),
            -40 => Some(Self::ErrorNotMappedAsArray),
            -41 => Some(Self::ErrorNotMappedAsPointer),
            -42 => Some(Self::ErrorAlreadyAcquired),
            -43 => Some(Self::ErrorNotMapped),
            
            // Fatal errors
            1 => Some(Self::ErrorFatalHardwareFailure),
            2 => Some(Self::ErrorFatalDriverCorruption),
            3 => Some(Self::ErrorFatalSystemFailure),
            4 => Some(Self::ErrorFatalInternalError),
            5 => Some(Self::ErrorFatalUnknown),
            6 => Some(Self::ErrorFatalInitializationFailure),
            7 => Some(Self::ErrorFatalDeinitialization),
            8 => Some(Self::ErrorFatalProfilerDisabled),
            9 => Some(Self::ErrorFatalProfilerNotInitialized),
            10 => Some(Self::ErrorFatalProfilerAlreadyStarted),
            11 => Some(Self::ErrorFatalProfilerAlreadyStopped),
            12 => Some(Self::ErrorFatalAssert),
            13 => Some(Self::ErrorFatalIllegalInstruction),
            14 => Some(Self::ErrorFatalHardwareStackError),
            15 => Some(Self::ErrorFatalIllegalAddress),
            16 => Some(Self::ErrorFatalInvalidAddressSpace),
            17 => Some(Self::ErrorFatalInvalidPc),
            18 => Some(Self::ErrorFatalNotPermitted),
            
            _ => None,
        }
    }
    
    /// Check if error is fatal
    pub fn is_fatal(self) -> bool {
        (self as c_int) > 0
    }
    
    /// Check if error is recoverable
    pub fn is_recoverable(self) -> bool {
        (self as c_int) < 0
    }
    
    /// Check if successful
    pub fn is_success(self) -> bool {
        (self as c_int) == 0
    }
    
    /// Get error description
    pub fn description(self) -> &'static str {
        match self {
            Self::Success => "Success",
            
            // Recoverable errors
            Self::ErrorMemoryAllocation => "Memory allocation failed",
            Self::ErrorMemoryAlignment => "Memory alignment error",
            Self::ErrorInvalidValue => "Invalid value",
            Self::ErrorInvalidHandle => "Invalid handle",
            Self::ErrorInvalidDevice => "Invalid device",
            Self::ErrorInvalidContext => "Invalid context",
            Self::ErrorOutOfMemory => "Out of memory",
            Self::ErrorNotReady => "Device not ready",
            Self::ErrorTimeout => "Operation timeout",
            Self::ErrorStreamNotReady => "Stream not ready",
            Self::ErrorEventNotReady => "Event not ready",
            Self::ErrorKernelNotFound => "Kernel not found",
            Self::ErrorInvalidConfiguration => "Invalid configuration",
            Self::ErrorLaunchFailure => "Kernel launch failed",
            Self::ErrorLaunchTimeout => "Kernel launch timeout",
            Self::ErrorLaunchOutOfResources => "Kernel launch out of resources",
            Self::ErrorInvalidDeviceFunction => "Invalid device function",
            Self::ErrorDeviceNotLicensed => "Device not licensed",
            Self::ErrorSoftwareValidityNotEstablished => "Software validity not established",
            Self::ErrorStartupFailure => "Startup failure",
            Self::ErrorInsufficientDriver => "Insufficient driver version",
            Self::ErrorNoKernelImageForDevice => "No kernel image for device",
            Self::ErrorIncompatibleDriverContext => "Incompatible driver context",
            Self::ErrorPeerAccessAlreadyEnabled => "Peer access already enabled",
            Self::ErrorPeerAccessNotEnabled => "Peer access not enabled",
            Self::ErrorSetOnActiveProcess => "Cannot set on active process",
            Self::ErrorOperatingSystem => "Operating system error",
            Self::ErrorEccUncorrectable => "Uncorrectable ECC error",
            Self::ErrorSharedObjectSymbolNotFound => "Shared object symbol not found",
            Self::ErrorSharedObjectInitFailed => "Shared object initialization failed",
            Self::ErrorUnsupportedLimit => "Unsupported limit",
            Self::ErrorDuplicateVariableName => "Duplicate variable name",
            Self::ErrorDuplicateTextureName => "Duplicate texture name",
            Self::ErrorDuplicateSurfaceName => "Duplicate surface name",
            Self::ErrorDevicesUnavailable => "All devices unavailable",
            Self::ErrorArrayIsMapped => "Array is mapped",
            Self::ErrorAlreadyMapped => "Already mapped",
            Self::ErrorNoDevice => "No device available",
            Self::ErrorNotMapped => "Not mapped",
            Self::ErrorNotMappedAsArray => "Not mapped as array",
            Self::ErrorNotMappedAsPointer => "Not mapped as pointer",
            Self::ErrorAlreadyAcquired => "Already acquired",
            
            // Fatal errors
            Self::ErrorFatalHardwareFailure => "Fatal hardware failure",
            Self::ErrorFatalDriverCorruption => "Fatal driver corruption",
            Self::ErrorFatalSystemFailure => "Fatal system failure",
            Self::ErrorFatalInternalError => "Fatal internal error",
            Self::ErrorFatalUnknown => "Fatal unknown error",
            Self::ErrorFatalInitializationFailure => "Fatal initialization failure",
            Self::ErrorFatalDeinitialization => "Fatal deinitialization error",
            Self::ErrorFatalProfilerDisabled => "Fatal profiler disabled",
            Self::ErrorFatalProfilerNotInitialized => "Fatal profiler not initialized",
            Self::ErrorFatalProfilerAlreadyStarted => "Fatal profiler already started",
            Self::ErrorFatalProfilerAlreadyStopped => "Fatal profiler already stopped",
            Self::ErrorFatalAssert => "Fatal assertion failed",
            Self::ErrorFatalIllegalInstruction => "Fatal illegal instruction",
            Self::ErrorFatalHardwareStackError => "Fatal hardware stack error",
            Self::ErrorFatalIllegalAddress => "Fatal illegal address",
            Self::ErrorFatalInvalidAddressSpace => "Fatal invalid address space",
            Self::ErrorFatalInvalidPc => "Fatal invalid program counter",
            Self::ErrorFatalNotPermitted => "Fatal operation not permitted",
            Self::ErrorNotPermitted => "Operation not permitted",
        }
    }
}

impl std::fmt::Display for NeuromorphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl std::error::Error for NeuromorphError {}

/// Helper macro to return success
#[macro_export]
macro_rules! neuromorph_success {
    () => {
        $crate::NeuromorphError::Success.to_c_int()
    };
}

/// Helper macro to return error
#[macro_export]
macro_rules! neuromorph_error {
    ($err:expr) => {
        $err.to_c_int()
    };
}

/// Constants for C compatibility
pub const NEUROMORPH_SUCCESS: c_int = 0;
pub const NEUROMORPH_ERROR_MEMORY_ALLOCATION: c_int = -1;
pub const NEUROMORPH_ERROR_INVALID_VALUE: c_int = -3;
pub const NEUROMORPH_ERROR_INVALID_HANDLE: c_int = -4;
pub const NEUROMORPH_ERROR_INVALID_DEVICE: c_int = -5;
pub const NEUROMORPH_ERROR_OUT_OF_MEMORY: c_int = -7;
pub const NEUROMORPH_ERROR_FATAL_HARDWARE_FAILURE: c_int = 1;
pub const NEUROMORPH_ERROR_FATAL_INTERNAL_ERROR: c_int = 4;
