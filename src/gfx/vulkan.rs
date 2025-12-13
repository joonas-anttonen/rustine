#![allow(dead_code)]

// Safe wrappers and abstractions over Vulkan API
mod vulkan_ffi;

use std::{ffi, fmt, result};

use super::Version;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Result {
    // Success codes
    Success,
    NotReady,
    Timeout,
    EventSet,
    EventReset,
    Incomplete,
    // Positive result codes
    PipelineCompileRequired,
    Suboptimal,
    ThreadIdleKhr,
    ThreadDoneKhr,
    OperationDeferredKhr,
    IncompatibleShaderBinaryExt,
    PipelineBinaryMissingKhr,
    // Error codes
    ErrorOutOfHostMemory,
    ErrorOutOfDeviceMemory,
    ErrorInitializationFailed,
    ErrorDeviceLost,
    ErrorMemoryMapFailed,
    ErrorLayerNotPresent,
    ErrorExtensionNotPresent,
    ErrorFeatureNotPresent,
    ErrorIncompatibleDriver,
    ErrorTooManyObjects,
    ErrorFormatNotSupported,
    ErrorFragmentedPool,
    ErrorUnknown,
    ErrorOutOfPoolMemory,
    ErrorInvalidExternalHandle,
    ErrorFragmentation,
    ErrorInvalidOpaqueCaptureAddress,
    ErrorNotPermitted,
    ErrorSurfaceLostKhr,
    ErrorNativeWindowInUseKhr,
    ErrorOutOfDateKhr,
    ErrorIncompatibleDisplayKhr,
    ErrorValidationFailedExt,
    ErrorInvalidShaderNv,
    ErrorImageUsageNotSupportedKhr,
    ErrorVideoPictureLayoutNotSupportedKhr,
    ErrorVideoProfileOperationNotSupportedKhr,
    ErrorVideoProfileFormatNotSupportedKhr,
    ErrorVideoProfileCodecNotSupportedKhr,
    ErrorVideoStdVersionNotSupportedKhr,
    ErrorInvalidDrmFormatModifierPlaneLayoutExt,
    ErrorFullScreenExclusiveModeLostExt,
    ErrorInvalidVideoStdParametersKhr,
    ErrorCompressionExhaustedExt,
    ErrorNotEnoughSpaceKhr,
    // Unknown error code
    Unknown(i32),
}

impl std::error::Error for Result {}
impl fmt::Display for Result {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Result::Success => write!(f, "VK_SUCCESS"),
            Result::NotReady => write!(f, "VK_NOT_READY"),
            Result::Timeout => write!(f, "VK_TIMEOUT"),
            Result::EventSet => write!(f, "VK_EVENT_SET"),
            Result::EventReset => write!(f, "VK_EVENT_RESET"),
            Result::Incomplete => write!(f, "VK_INCOMPLETE"),
            Result::PipelineCompileRequired => write!(f, "VK_PIPELINE_COMPILE_REQUIRED"),
            Result::Suboptimal => write!(f, "VK_SUBOPTIMAL_KHR"),
            Result::ThreadIdleKhr => write!(f, "VK_THREAD_IDLE_KHR"),
            Result::ThreadDoneKhr => write!(f, "VK_THREAD_DONE_KHR"),
            Result::OperationDeferredKhr => write!(f, "VK_OPERATION_DEFERRED_KHR"),
            Result::IncompatibleShaderBinaryExt => write!(f, "VK_INCOMPATIBLE_SHADER_BINARY_EXT"),
            Result::PipelineBinaryMissingKhr => write!(f, "VK_PIPELINE_BINARY_MISSING_KHR"),
            Result::ErrorOutOfHostMemory => write!(f, "VK_ERROR_OUT_OF_HOST_MEMORY"),
            Result::ErrorOutOfDeviceMemory => write!(f, "VK_ERROR_OUT_OF_DEVICE_MEMORY"),
            Result::ErrorInitializationFailed => write!(f, "VK_ERROR_INITIALIZATION_FAILED"),
            Result::ErrorDeviceLost => write!(f, "VK_ERROR_DEVICE_LOST"),
            Result::ErrorMemoryMapFailed => write!(f, "VK_ERROR_MEMORY_MAP_FAILED"),
            Result::ErrorLayerNotPresent => write!(f, "VK_ERROR_LAYER_NOT_PRESENT"),
            Result::ErrorExtensionNotPresent => write!(f, "VK_ERROR_EXTENSION_NOT_PRESENT"),
            Result::ErrorFeatureNotPresent => write!(f, "VK_ERROR_FEATURE_NOT_PRESENT"),
            Result::ErrorIncompatibleDriver => write!(f, "VK_ERROR_INCOMPATIBLE_DRIVER"),
            Result::ErrorTooManyObjects => write!(f, "VK_ERROR_TOO_MANY_OBJECTS"),
            Result::ErrorFormatNotSupported => write!(f, "VK_ERROR_FORMAT_NOT_SUPPORTED"),
            Result::ErrorFragmentedPool => write!(f, "VK_ERROR_FRAGMENTED_POOL"),
            Result::ErrorUnknown => write!(f, "VK_ERROR_UNKNOWN"),
            Result::ErrorOutOfPoolMemory => write!(f, "VK_ERROR_OUT_OF_POOL_MEMORY"),
            Result::ErrorInvalidExternalHandle => write!(f, "VK_ERROR_INVALID_EXTERNAL_HANDLE"),
            Result::ErrorFragmentation => write!(f, "VK_ERROR_FRAGMENTATION"),
            Result::ErrorInvalidOpaqueCaptureAddress => {
                write!(f, "VK_ERROR_INVALID_OPAQUE_CAPTURE_ADDRESS")
            }
            Result::ErrorNotPermitted => write!(f, "VK_ERROR_NOT_PERMITTED"),
            Result::ErrorSurfaceLostKhr => write!(f, "VK_ERROR_SURFACE_LOST_KHR"),
            Result::ErrorNativeWindowInUseKhr => write!(f, "VK_ERROR_NATIVE_WINDOW_IN_USE_KHR"),
            Result::ErrorOutOfDateKhr => write!(f, "VK_ERROR_OUT_OF_DATE_KHR"),
            Result::ErrorIncompatibleDisplayKhr => write!(f, "VK_ERROR_INCOMPATIBLE_DISPLAY_KHR"),
            Result::ErrorValidationFailedExt => write!(f, "VK_ERROR_VALIDATION_FAILED_EXT"),
            Result::ErrorInvalidShaderNv => write!(f, "VK_ERROR_INVALID_SHADER_NV"),
            Result::ErrorImageUsageNotSupportedKhr => {
                write!(f, "VK_ERROR_IMAGE_USAGE_NOT_SUPPORTED_KHR")
            }
            Result::ErrorVideoPictureLayoutNotSupportedKhr => {
                write!(f, "VK_ERROR_VIDEO_PICTURE_LAYOUT_NOT_SUPPORTED_KHR")
            }
            Result::ErrorVideoProfileOperationNotSupportedKhr => {
                write!(f, "VK_ERROR_VIDEO_PROFILE_OPERATION_NOT_SUPPORTED_KHR")
            }
            Result::ErrorVideoProfileFormatNotSupportedKhr => {
                write!(f, "VK_ERROR_VIDEO_PROFILE_FORMAT_NOT_SUPPORTED_KHR")
            }
            Result::ErrorVideoProfileCodecNotSupportedKhr => {
                write!(f, "VK_ERROR_VIDEO_PROFILE_CODEC_NOT_SUPPORTED_KHR")
            }
            Result::ErrorVideoStdVersionNotSupportedKhr => {
                write!(f, "VK_ERROR_VIDEO_STD_VERSION_NOT_SUPPORTED_KHR")
            }
            Result::ErrorInvalidDrmFormatModifierPlaneLayoutExt => {
                write!(f, "VK_ERROR_INVALID_DRM_FORMAT_MODIFIER_PLANE_LAYOUT_EXT")
            }
            Result::ErrorFullScreenExclusiveModeLostExt => {
                write!(f, "VK_ERROR_FULL_SCREEN_EXCLUSIVE_MODE_LOST_EXT")
            }
            Result::ErrorInvalidVideoStdParametersKhr => {
                write!(f, "VK_ERROR_INVALID_VIDEO_STD_PARAMETERS_KHR")
            }
            Result::ErrorCompressionExhaustedExt => write!(f, "VK_ERROR_COMPRESSION_EXHAUSTED_EXT"),
            Result::ErrorNotEnoughSpaceKhr => write!(f, "VK_ERROR_NOT_ENOUGH_SPACE_KHR"),
            Result::Unknown(code) => write!(f, "Unknown VkResult: {}", code),
        }
    }
}

fn from_vk_result(code: i32) -> Result {
    match code {
        0 => Result::Success,
        1 => Result::NotReady,
        2 => Result::Timeout,
        3 => Result::EventSet,
        4 => Result::EventReset,
        5 => Result::Incomplete,
        1000297000 => Result::PipelineCompileRequired,
        1000001003 => Result::Suboptimal,
        1000268000 => Result::ThreadIdleKhr,
        1000268001 => Result::ThreadDoneKhr,
        1000268002 => Result::OperationDeferredKhr,
        1000482000 => Result::IncompatibleShaderBinaryExt,
        1000483000 => Result::PipelineBinaryMissingKhr,
        -1 => Result::ErrorOutOfHostMemory,
        -2 => Result::ErrorOutOfDeviceMemory,
        -3 => Result::ErrorInitializationFailed,
        -4 => Result::ErrorDeviceLost,
        -5 => Result::ErrorMemoryMapFailed,
        -6 => Result::ErrorLayerNotPresent,
        -7 => Result::ErrorExtensionNotPresent,
        -8 => Result::ErrorFeatureNotPresent,
        -9 => Result::ErrorIncompatibleDriver,
        -10 => Result::ErrorTooManyObjects,
        -11 => Result::ErrorFormatNotSupported,
        -12 => Result::ErrorFragmentedPool,
        -13 => Result::ErrorUnknown,
        -1000069000 => Result::ErrorOutOfPoolMemory,
        -1000072003 => Result::ErrorInvalidExternalHandle,
        -1000161000 => Result::ErrorFragmentation,
        -1000257000 => Result::ErrorInvalidOpaqueCaptureAddress,
        -1000174001 => Result::ErrorNotPermitted,
        -1000000000 => Result::ErrorSurfaceLostKhr,
        -1000000001 => Result::ErrorNativeWindowInUseKhr,
        -1000001004 => Result::ErrorOutOfDateKhr,
        -1000003001 => Result::ErrorIncompatibleDisplayKhr,
        -1000011001 => Result::ErrorValidationFailedExt,
        -1000012000 => Result::ErrorInvalidShaderNv,
        -1000023000 => Result::ErrorImageUsageNotSupportedKhr,
        -1000023001 => Result::ErrorVideoPictureLayoutNotSupportedKhr,
        -1000023002 => Result::ErrorVideoProfileOperationNotSupportedKhr,
        -1000023003 => Result::ErrorVideoProfileFormatNotSupportedKhr,
        -1000023004 => Result::ErrorVideoProfileCodecNotSupportedKhr,
        -1000023005 => Result::ErrorVideoStdVersionNotSupportedKhr,
        -1000158000 => Result::ErrorInvalidDrmFormatModifierPlaneLayoutExt,
        -1000255000 => Result::ErrorFullScreenExclusiveModeLostExt,
        -1000299000 => Result::ErrorInvalidVideoStdParametersKhr,
        -1000338000 => Result::ErrorCompressionExhaustedExt,
        -1000483000 => Result::ErrorNotEnoughSpaceKhr,
        other => Result::Unknown(other),
    }
}

pub fn vk_enumerate_instance_version() -> result::Result<Version, Result> {
    let mut api_version: u32 = 0;
    let result = unsafe {
        from_vk_result(vulkan_ffi::vkEnumerateInstanceVersion(
            &mut api_version as *mut u32,
        ))
    };

    match result {
        Result::Success => Ok(Version::from_vk_version(api_version)),
        _ => Err(result),
    }
}

pub fn vk_enumerate_instance_layer_properties() -> result::Result<Vec<String>, Result> {
    let mut property_count: u32 = 0;
    let result = unsafe {
        from_vk_result(vulkan_ffi::vkEnumerateInstanceLayerProperties(
            &mut property_count as *mut u32,
            std::ptr::null_mut(),
        ))
    };

    match result {
        Result::Success => {
            let mut properties: Vec<vulkan_ffi::VkLayerProperties> =
                Vec::with_capacity(property_count as usize);
            let result = unsafe {
                from_vk_result(vulkan_ffi::vkEnumerateInstanceLayerProperties(
                    &mut property_count as *mut u32,
                    properties.as_mut_ptr(),
                ))
            };

            match result {
                Result::Success => {
                    unsafe {
                        properties.set_len(property_count as usize);
                    }
                    let layer_names = properties
                        .iter()
                        .map(|prop| {
                            let cstr = unsafe {
                                std::ffi::CStr::from_ptr(prop.layerName.as_ptr() as *const i8)
                            };
                            cstr.to_string_lossy().into_owned()
                        })
                        .collect();
                    Ok(layer_names)
                }
                _ => Err(result),
            }
        }
        _ => Err(result),
    }
}

pub fn vk_enumerate_instance_extension_properties() -> result::Result<Vec<String>, Result> {
    let mut property_count: u32 = 0;
    let result = unsafe {
        from_vk_result(vulkan_ffi::vkEnumerateInstanceExtensionProperties(
            std::ptr::null(),
            &mut property_count as *mut u32,
            std::ptr::null_mut(),
        ))
    };

    match result {
        Result::Success => {
            let mut properties: Vec<vulkan_ffi::VkExtensionProperties> =
                Vec::with_capacity(property_count as usize);
            let result = unsafe {
                from_vk_result(vulkan_ffi::vkEnumerateInstanceExtensionProperties(
                    std::ptr::null(),
                    &mut property_count as *mut u32,
                    properties.as_mut_ptr(),
                ))
            };

            match result {
                Result::Success => {
                    unsafe {
                        properties.set_len(property_count as usize);
                    }
                    let extension_names = properties
                        .iter()
                        .map(|prop| {
                            let cstr = unsafe {
                                std::ffi::CStr::from_ptr(prop.extensionName.as_ptr() as *const i8)
                            };
                            cstr.to_string_lossy().into_owned()
                        })
                        .collect();
                    Ok(extension_names)
                }
                _ => Err(result),
            }
        }
        _ => Err(result),
    }
}

pub struct Instance {
    handle: vulkan_ffi::VkInstance,
    debug_messenger: Option<vulkan_ffi::VkDebugUtilsMessengerEXT>,
    destroy_debug_messenger_fn: Option<vulkan_ffi::PFN_vkDestroyDebugUtilsMessengerEXT>,
}

impl Drop for Instance {
    fn drop(&mut self) {
        // Destroy debug messenger first if it exists
        if let (Some(messenger), Some(destroy_fn)) = (self.debug_messenger, self.destroy_debug_messenger_fn) {
            unsafe {
                if let Some(destroy) = destroy_fn {
                    destroy(self.handle, messenger, std::ptr::null());
                }
            }
        }
        
        unsafe {
            vulkan_ffi::vkDestroyInstance(self.handle, std::ptr::null());
        }
    }
}

pub fn vk_enumerate_physical_devices(instance: &Instance) -> result::Result<Vec<super::core::PhysicalDevice>, Result> {
    let mut device_count: u32 = 0;
    let result = unsafe {
        from_vk_result(vulkan_ffi::vkEnumeratePhysicalDevices(
            instance.handle,
            &mut device_count as *mut u32,
            std::ptr::null_mut(),
        ))
    };

    match result {
        Result::Success => {
            let mut devices: Vec<u64> = Vec::with_capacity(device_count as usize);
            let result = unsafe {
                from_vk_result(vulkan_ffi::vkEnumeratePhysicalDevices(
                    instance.handle,
                    &mut device_count as *mut u32,
                    devices.as_mut_ptr(),
                ))
            };

            match result {
                Result::Success => {
                    unsafe {
                        devices.set_len(device_count as usize);
                    }

                    // Query properties for each physical device
                    let physical_devices: Vec<super::core::PhysicalDevice> = devices
                        .iter()
                        .map(|&device_handle| {
                            let mut properties = vulkan_ffi::VkPhysicalDeviceProperties {
                                apiVersion: 0,
                                driverVersion: 0,
                                vendorID: 0,
                                deviceID: 0,
                                deviceType: 0,
                                deviceName: [0; 256],
                                pipelineCacheUUID: [0; 16],
                                limits: unsafe { std::mem::zeroed() },
                                sparseProperties: unsafe { std::mem::zeroed() },
                            };

                            unsafe {
                                vulkan_ffi::vkGetPhysicalDeviceProperties(
                                    device_handle,
                                    &mut properties as *mut vulkan_ffi::VkPhysicalDeviceProperties,
                                );
                            }

                            // Extract device name
                            let device_name = {
                                let null_pos = properties.deviceName
                                    .iter()
                                    .position(|&c| c == 0)
                                    .unwrap_or(properties.deviceName.len());
                                let name_bytes = &properties.deviceName[..null_pos];
                                String::from_utf8_lossy(name_bytes).into_owned()
                            };

                            // Convert device type
                            let device_type = match properties.deviceType {
                                1 => super::core::PhysicalDeviceType::Integrated,
                                2 => super::core::PhysicalDeviceType::Discrete,
                                3 => super::core::PhysicalDeviceType::Virtual,
                                4 => super::core::PhysicalDeviceType::Cpu,
                                _ => super::core::PhysicalDeviceType::Other,
                            };

                            // Use pipelineCacheUUID as unique ID
                            let id = u128::from_le_bytes(properties.pipelineCacheUUID);

                            super::core::PhysicalDevice {
                                name: device_name,
                                driver: Version::from_vk_version(properties.driverVersion),
                                api: Version::from_vk_version(properties.apiVersion),
                                device_type,
                                id,
                            }
                        })
                        .collect();

                    Ok(physical_devices)
                }
                _ => Err(result),
            }
        }
        _ => Err(result),
    }
}

pub fn vk_create_instance(parameters: &super::ApiParameters) -> result::Result<Instance, Result> {
    // Get available instance layers and extensions
    let available_layers = vk_enumerate_instance_layer_properties()?;
    let available_extensions = vk_enumerate_instance_extension_properties()?;

    // Keep owned CString objects alive for the duration of the FFI call
    let mut enabled_layers_owned: Vec<ffi::CString> = Vec::new();
    if parameters.enable_debugging {
        // Enable standard validation layer if available
        let layer_name = "VK_LAYER_KHRONOS_validation";
        if available_layers.iter().any(|layer| layer == layer_name) {
            enabled_layers_owned.push(
                ffi::CString::new(layer_name)
                    .map_err(|_| Result::ErrorInitializationFailed)?
            );
        } else {
            return Err(Result::ErrorLayerNotPresent);
        }
    }

    // Create pointers from owned strings
    let enabled_layers: Vec<*const ffi::c_char> = enabled_layers_owned
        .iter()
        .map(|cs| cs.as_ptr())
        .collect();

    // Store extension names as &CStr references
    let mut enabled_extensions_list: Vec<&ffi::CStr> = Vec::new();

    // Add platform-specific surface extensions
    if cfg!(target_os = "windows") {
        enabled_extensions_list.push(c"VK_KHR_surface");
        enabled_extensions_list.push(c"VK_KHR_win32_surface");
    } else if cfg!(target_os = "linux") {
        enabled_extensions_list.push(c"VK_KHR_surface");
        enabled_extensions_list.push(c"VK_KHR_xlib_surface");
        enabled_extensions_list.push(c"VK_KHR_xcb_surface");
        enabled_extensions_list.push(c"VK_KHR_wayland_surface");
    }

    // Add debug utils extension if debugging is enabled and available
    if parameters.enable_debugging {
        if available_extensions.iter().any(|ext| ext == "VK_EXT_debug_utils") {
            enabled_extensions_list.push(c"VK_EXT_debug_utils");
        }
    }

    // Verify that required extensions are available
    for ext_cstr in &enabled_extensions_list {
        let ext_str = ext_cstr.to_string_lossy();
        let ext_available = available_extensions.iter().any(|e| e == ext_str.as_ref());
        if !ext_available {
            return Err(Result::ErrorExtensionNotPresent);
        }
    }

    // Create pointer array for FFI call
    let enabled_extensions: Vec<*const ffi::c_char> = enabled_extensions_list
        .iter()
        .map(|cs| cs.as_ptr())
        .collect();

    // Create VkApplicationInfo
    let app_name_cstring = ffi::CString::new(parameters.app_name.as_str())
        .map_err(|_| Result::ErrorInitializationFailed)?;
    let engine_name_cstring = ffi::CString::new(parameters.app_engine_name.as_str())
        .map_err(|_| Result::ErrorInitializationFailed)?;

    let app_info = vulkan_ffi::VkApplicationInfo {
        sType: vulkan_ffi::VkStructureType::VK_STRUCTURE_TYPE_APPLICATION_INFO as u32,
        pNext: std::ptr::null(),
        pApplicationName: app_name_cstring.as_ptr(),
        applicationVersion: parameters.app_version.to_vk_version(),
        pEngineName: engine_name_cstring.as_ptr(),
        engineVersion: parameters.app_engine_version.to_vk_version(),
        apiVersion: parameters.required_api_version.to_vk_version(),
    };

    // Create VkInstanceCreateInfo
    let create_info = vulkan_ffi::VkInstanceCreateInfo {
        sType: vulkan_ffi::VkStructureType::VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO as u32,
        pNext: std::ptr::null(),
        flags: 0,
        pApplicationInfo: &app_info,
        enabledLayerCount: enabled_layers.len() as u32,
        ppEnabledLayerNames: if enabled_layers.is_empty() {
            std::ptr::null()
        } else {
            enabled_layers.as_ptr()
        },
        enabledExtensionCount: enabled_extensions.len() as u32,
        ppEnabledExtensionNames: if enabled_extensions.is_empty() {
            std::ptr::null()
        } else {
            enabled_extensions.as_ptr()
        },
    };

    // Create the Vulkan instance
    let mut instance: vulkan_ffi::VkInstance = std::ptr::null_mut();
    let result = unsafe {
        from_vk_result(vulkan_ffi::vkCreateInstance(
            &create_info,
            std::ptr::null(),
            &mut instance,
        ))
    };

    match result {
        Result::Success => {
            let mut debug_messenger = None;
            let mut destroy_debug_messenger_fn = None;

            // Set up debug messenger if debugging is enabled
            if parameters.enable_debugging {
                // Get function pointers
                let create_debug_fn: vulkan_ffi::PFN_vkCreateDebugUtilsMessengerEXT = unsafe {
                    std::mem::transmute(vulkan_ffi::vkGetInstanceProcAddr(
                        instance as u64,
                        c"vkCreateDebugUtilsMessengerEXT".as_ptr() as *const u8,
                    ))
                };

                let destroy_debug_fn: vulkan_ffi::PFN_vkDestroyDebugUtilsMessengerEXT = unsafe {
                    std::mem::transmute(vulkan_ffi::vkGetInstanceProcAddr(
                        instance as u64,
                        c"vkDestroyDebugUtilsMessengerEXT".as_ptr() as *const u8,
                    ))
                };

                if let (Some(create_fn), Some(destroy_fn)) = (create_debug_fn, destroy_debug_fn) {
                    // Define the debug callback
                    unsafe extern "C" fn debug_callback(
                        _message_severity: u32,
                        _message_type: u32,
                        callback_data: *const vulkan_ffi::VkDebugUtilsMessengerCallbackDataEXT,
                        _user_data: *mut ffi::c_void,
                    ) -> u32 {
                        if callback_data.is_null() {
                            return 0;
                        }
                        unsafe {
                            let data = &*callback_data;
                            if data.pMessage.is_null() {
                                return 0;
                            }

                            let message = ffi::CStr::from_ptr(data.pMessage)
                                .to_string_lossy();

                            super::super::log::Log::global().logger("Vulkan", "")
                                .error(&format!("{}", message));          
                            0
                        }       
                    }

                    // Create debug messenger info
                    let debug_create_info = vulkan_ffi::VkDebugUtilsMessengerCreateInfoEXT {
                        sType: vulkan_ffi::VkStructureType::VK_STRUCTURE_TYPE_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT as u32,
                        pNext: std::ptr::null(),
                        flags: 0,
                        messageSeverity: 
                            vulkan_ffi::VkDebugUtilsMessageSeverityFlagBitsEXT::VK_DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT as u32 |
                            vulkan_ffi::VkDebugUtilsMessageSeverityFlagBitsEXT::VK_DEBUG_UTILS_MESSAGE_SEVERITY_WARNING_BIT_EXT as u32,
                        messageType:
                            vulkan_ffi::VkDebugUtilsMessageTypeFlagBitsEXT::VK_DEBUG_UTILS_MESSAGE_TYPE_GENERAL_BIT_EXT as u32 |
                            vulkan_ffi::VkDebugUtilsMessageTypeFlagBitsEXT::VK_DEBUG_UTILS_MESSAGE_TYPE_VALIDATION_BIT_EXT as u32,
                        pfnUserCallback: Some(debug_callback),
                        pUserData: std::ptr::null_mut(),
                    };

                    let mut messenger: vulkan_ffi::VkDebugUtilsMessengerEXT = std::ptr::null_mut();
                    let create_result = unsafe {
                        from_vk_result(create_fn(
                            instance,
                            &debug_create_info,
                            std::ptr::null(),
                            &mut messenger,
                        ))
                    };

                    if create_result == Result::Success {
                        debug_messenger = Some(messenger);
                        destroy_debug_messenger_fn = Some(destroy_fn);
                    }
                }
            }

            Ok(Instance {
                handle: instance,
                debug_messenger,
                destroy_debug_messenger_fn: destroy_debug_messenger_fn.map(Some),
            })
        }
        _ => Err(result),
    }
}
