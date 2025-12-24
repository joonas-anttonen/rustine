#![allow(dead_code)]

// Safe wrappers and abstractions over Vulkan API
mod vulkan_ffi;

use std::{collections, ffi, fmt, result};

use super::Version;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Result {
    Success,
    NotSupported,
    Unknown(i32),
}

impl std::error::Error for Result {}
impl fmt::Display for Result {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Result::Success => write!(f, "Success"),
            Result::NotSupported => write!(f, "Not supported"),
            Result::Unknown(code) => write!(f, "Unknown error: {}", code),
        }
    }
}

fn from_vk_result(code: i32) -> Result {
    match code {
        0 => Result::Success,
        -7 | -8 | -11 => Result::NotSupported, // Extension not present, feature not present, format not supported
        other => Result::Unknown(other),
    }
}

fn panic_if_failed(code: i32, context: &str) {
    if code != 0 {
        panic!("Vulkan panic in {}: {}", context, from_vk_result(code));
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
    destroy_debug_messenger_fn: vulkan_ffi::PFN_vkDestroyDebugUtilsMessengerEXT,
}

impl Drop for Instance {
    fn drop(&mut self) {
        // Destroy debug messenger first if it exists
        if let (Some(messenger), Some(destroy_fn)) =
            (self.debug_messenger, self.destroy_debug_messenger_fn)
        {
            unsafe {
                destroy_fn(self.handle, messenger, std::ptr::null());
            }
        }

        unsafe {
            vulkan_ffi::vkDestroyInstance(self.handle, std::ptr::null());
        }
    }
}

pub fn vk_enumerate_physical_devices(
    instance: &Instance,
) -> result::Result<Vec<super::core::PhysicalDevice>, Result> {
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
                                let null_pos = properties
                                    .deviceName
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

unsafe extern "C" fn vulkan_debug_callback(
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

        let message = ffi::CStr::from_ptr(data.pMessage).to_string_lossy();

        super::super::log::Log::global()
            .logger("Vulkan", "")
            .error(&format!("{}", message));
        0
    }
}

pub fn vk_create_instance(parameters: &super::ApiParameters) -> result::Result<Instance, Result> {
    // 1. Get available instance layers and extensions
    let available_layers: collections::HashSet<String> = vk_enumerate_instance_layer_properties()?
        .iter()
        .cloned()
        .collect();
    let available_extensions: collections::HashSet<String> =
        vk_enumerate_instance_extension_properties()?
            .iter()
            .cloned()
            .collect();

    let mut enabled_layers_cstrings: Vec<ffi::CString> = Vec::new();
    let mut enabled_extensions_cstrings: Vec<ffi::CString> = Vec::new();

    // 2. Add platform-specific surface extensions
    enabled_extensions_cstrings.push(ffi::CString::new("VK_KHR_surface").unwrap());

    match parameters.platform {
        super::parameters::Platform::Windows => {
            enabled_extensions_cstrings.push(ffi::CString::new("VK_KHR_win32_surface").unwrap());
        }
        super::parameters::Platform::X11 => {
            enabled_extensions_cstrings.push(ffi::CString::new("VK_KHR_xlib_surface").unwrap());
            enabled_extensions_cstrings.push(ffi::CString::new("VK_KHR_xcb_surface").unwrap());
        }
        super::parameters::Platform::Wayland => {
            enabled_extensions_cstrings.push(ffi::CString::new("VK_KHR_wayland_surface").unwrap());
        }
        // Error if unsupported platform
        _ => {
            return Err(Result::NotSupported);
        }
    }

    // 3. Add validation and debug utils extension if debugging is requested and available
    let validation_name = "VK_LAYER_KHRONOS_validation";
    let validation_present = available_layers.contains(validation_name);
    let debug_utils_name = "VK_EXT_debug_utils";
    let debug_utils_present = available_extensions.contains(debug_utils_name);
    let debug_utils_create_fn_name = ffi::CString::new("vkCreateDebugUtilsMessengerEXT").unwrap();
    let debug_utils_destroy_fn_name = ffi::CString::new("vkDestroyDebugUtilsMessengerEXT").unwrap();
    let debug_utils_create_fn_present = unsafe {
        vulkan_ffi::vkGetInstanceProcAddr(0, debug_utils_create_fn_name.as_ptr()).is_some()
    };
    let debug_utils_destroy_fn_present = unsafe {
        vulkan_ffi::vkGetInstanceProcAddr(0, debug_utils_destroy_fn_name.as_ptr()).is_some()
    };

    let enable_debugging = parameters.enable_debugging
        && validation_present
        && debug_utils_present
        && debug_utils_create_fn_present
        && debug_utils_destroy_fn_present;
    if enable_debugging {
        enabled_layers_cstrings.push(ffi::CString::new(validation_name).unwrap());
        enabled_extensions_cstrings.push(ffi::CString::new(debug_utils_name).unwrap());
    }

    // 4. Create the Vulkan instance
    let app_name_cstring = ffi::CString::new(parameters.app_name.as_str()).unwrap();
    let engine_name_cstring = ffi::CString::new(parameters.app_engine_name.as_str()).unwrap();
    let enabled_layers_ptrs: Vec<*const ffi::c_char> = enabled_layers_cstrings
        .iter()
        .map(|cs| cs.as_ptr())
        .collect();

    let enabled_extensions_ptrs: Vec<*const ffi::c_char> = enabled_extensions_cstrings
        .iter()
        .map(|cs| cs.as_ptr())
        .collect();

    let app_info = vulkan_ffi::VkApplicationInfo {
        sType: vulkan_ffi::VkStructureType::VK_STRUCTURE_TYPE_APPLICATION_INFO as u32,
        pNext: std::ptr::null(),
        pApplicationName: app_name_cstring.as_ptr(),
        applicationVersion: parameters.app_version.to_vk_version(),
        pEngineName: engine_name_cstring.as_ptr(),
        engineVersion: parameters.app_engine_version.to_vk_version(),
        apiVersion: parameters.required_api_version.to_vk_version(),
    };

    let create_info = vulkan_ffi::VkInstanceCreateInfo {
        sType: vulkan_ffi::VkStructureType::VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO as u32,
        pNext: std::ptr::null(),
        flags: 0,
        pApplicationInfo: &app_info,
        enabledLayerCount: enabled_layers_ptrs.len() as u32,
        ppEnabledLayerNames: if enabled_layers_ptrs.is_empty() {
            std::ptr::null()
        } else {
            enabled_layers_ptrs.as_ptr()
        },
        enabledExtensionCount: enabled_extensions_ptrs.len() as u32,
        ppEnabledExtensionNames: if enabled_extensions_ptrs.is_empty() {
            std::ptr::null()
        } else {
            enabled_extensions_ptrs.as_ptr()
        },
    };

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
            if enable_debugging {
                let create_debug_fn: vulkan_ffi::PFN_vkCreateDebugUtilsMessengerEXT = unsafe {
                    std::mem::transmute(vulkan_ffi::vkGetInstanceProcAddr(
                        instance as u64,
                        debug_utils_create_fn_name.as_ptr(),
                    ))
                };

                let destroy_debug_fn: vulkan_ffi::PFN_vkDestroyDebugUtilsMessengerEXT = unsafe {
                    std::mem::transmute(vulkan_ffi::vkGetInstanceProcAddr(
                        instance as u64,
                        debug_utils_destroy_fn_name.as_ptr(),
                    ))
                };

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
                        pfnUserCallback: Some(vulkan_debug_callback),
                        pUserData: std::ptr::null_mut(),
                    };

                let mut debug_messenger_ptr: vulkan_ffi::VkDebugUtilsMessengerEXT =
                    std::ptr::null_mut();
                panic_if_failed(
                    unsafe {
                        create_debug_fn.unwrap()(
                            instance,
                            &debug_create_info,
                            std::ptr::null(),
                            &mut debug_messenger_ptr,
                        )
                    },
                    "Creating Debug Utils Messenger",
                );

                Ok(Instance {
                    handle: instance,
                    debug_messenger: Some(debug_messenger_ptr),
                    destroy_debug_messenger_fn: destroy_debug_fn,
                })
            } else {
                Ok(Instance {
                    handle: instance,
                    debug_messenger: None,
                    destroy_debug_messenger_fn: None,
                })
            }
        }
        _ => Err(result),
    }
}
