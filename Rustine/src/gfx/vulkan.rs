#![allow(dead_code)]

// Safe wrappers and abstractions over Vulkan API
mod vulkan_ffi;

use std::{collections, ffi, result};

use crate::{error, warning};
use crate::{gfx::*, version::Version};

fn make_result(code: i32) -> Result {
    match code {
        0 => Result::Success,
        -7 | -8 | -11 => Result::NotSupported, // Extension not present, feature not present, format not supported
        other => Result::Unknown(other),
    }
}

fn panic_if_failed(code: i32, context: &str) {
    if code != 0 {
        panic!("Vulkan panic in {}: {}", context, make_result(code));
    }
}

/// Queries the highest Vulkan API version supported.
pub fn vk_enumerate_instance_version() -> result::Result<Version, Result> {
    let mut api_version: u32 = 0;
    let result = unsafe {
        make_result(vulkan_ffi::vkEnumerateInstanceVersion(
            &mut api_version as *mut u32,
        ))
    };

    match result {
        Result::Success => Ok(Version::from_vk_version(api_version)),
        _ => Err(result),
    }
}

/// Queries the available instance layers.
pub fn vk_enumerate_instance_layer_properties() -> result::Result<Vec<ffi::CString>, Result> {
    let mut property_count: u32 = 0;
    let result = unsafe {
        make_result(vulkan_ffi::vkEnumerateInstanceLayerProperties(
            &mut property_count as *mut u32,
            std::ptr::null_mut(),
        ))
    };

    match result {
        Result::Success => {
            let mut properties: Vec<vulkan_ffi::VkLayerProperties> =
                Vec::with_capacity(property_count as usize);
            let result = unsafe {
                make_result(vulkan_ffi::vkEnumerateInstanceLayerProperties(
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
                            cstr.to_owned()
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

/// Queries the available instance extensions.
pub fn vk_enumerate_instance_extension_properties() -> result::Result<Vec<ffi::CString>, Result> {
    let mut property_count: u32 = 0;
    let result = unsafe {
        make_result(vulkan_ffi::vkEnumerateInstanceExtensionProperties(
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
                make_result(vulkan_ffi::vkEnumerateInstanceExtensionProperties(
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
                            cstr.to_owned()
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

/// Represents a Vulkan instance.
pub struct Instance {
    handle: vulkan_ffi::VkInstance,
    debug_messenger: Option<vulkan_ffi::VkDebugUtilsMessengerEXT>,
}

impl Drop for Instance {
    fn drop(&mut self) {
        warning!("Instance::drop");

        if let Some(messenger) = self.debug_messenger {
            let debug_utils_destroy_fn_name = c"vkDestroyDebugUtilsMessengerEXT";
            let destroy_debug_fn: vulkan_ffi::PFN_vkDestroyDebugUtilsMessengerEXT = unsafe {
                std::mem::transmute(vulkan_ffi::vkGetInstanceProcAddr(
                    self.handle as u64,
                    debug_utils_destroy_fn_name.as_ptr(),
                ))
            };

            unsafe {
                destroy_debug_fn.unwrap()(self.handle, messenger, std::ptr::null());
            }
        }

        unsafe {
            vulkan_ffi::vkDestroyInstance(self.handle, std::ptr::null());
        }
    }
}

/// Enumerates physical devices (GPUs) available on the system.
pub fn vk_enumerate_physical_devices(
    instance: &Instance,
) -> result::Result<Vec<PhysicalDevice>, Result> {
    let mut device_count: u32 = 0;
    let result = unsafe {
        make_result(vulkan_ffi::vkEnumeratePhysicalDevices(
            instance.handle,
            &mut device_count as *mut u32,
            std::ptr::null_mut(),
        ))
    };

    match result {
        Result::Success => {
            let mut devices: Vec<u64> = Vec::with_capacity(device_count as usize);
            let result = unsafe {
                make_result(vulkan_ffi::vkEnumeratePhysicalDevices(
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
                    let physical_devices: Vec<PhysicalDevice> = devices
                        .iter()
                        .map(|&device_handle| {
                            let mut id_properties: vulkan_ffi::VkPhysicalDeviceIDProperties =
                                unsafe { std::mem::zeroed() };
                            id_properties.sType = vulkan_ffi::VkStructureType::VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_ID_PROPERTIES as u32;

                            let mut properties: vulkan_ffi::VkPhysicalDeviceProperties2 =
                                unsafe { std::mem::zeroed() };
                            properties.sType = vulkan_ffi::VkStructureType::VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_PROPERTIES_2 as u32;
                            properties.pNext = &mut id_properties as *mut vulkan_ffi::VkPhysicalDeviceIDProperties as *mut ffi::c_void;
                            unsafe {
                                vulkan_ffi::vkGetPhysicalDeviceProperties2(
                                    device_handle,
                                    &mut properties as *mut vulkan_ffi::VkPhysicalDeviceProperties2,
                                );
                            }

                            let device_name = {
                                let null_pos = properties.properties
                                    .deviceName
                                    .iter()
                                    .position(|&c| c == 0)
                                    .unwrap_or(properties.properties.deviceName.len());
                                let name_bytes = &properties.properties.deviceName[..null_pos];
                                String::from_utf8_lossy(name_bytes).into_owned()
                            };

                            let device_type = match properties.properties.deviceType {
                                1 => PhysicalDeviceType::Integrated,
                                2 => PhysicalDeviceType::Discrete,
                                3 => PhysicalDeviceType::Virtual,
                                4 => PhysicalDeviceType::Cpu,
                                _ => PhysicalDeviceType::Other,
                            };

                            let id = u128::from_le_bytes(id_properties.deviceUUID);
                            let luid = if id_properties.deviceLUIDValid != 0 {
                                u64::from_le_bytes(id_properties.deviceLUID)
                            } else {
                                0
                            };

                            PhysicalDevice {
                                name: device_name,
                                driver: Version::from_vk_version(properties.properties.driverVersion),
                                api: Version::from_vk_version(properties.properties.apiVersion),
                                device_type,
                                id,
                                luid,
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
        error!("Vulkan: {}", message);
        0
    }
}

/// Creates a Vulkan instance based on the provided parameters.
pub fn vk_create_instance(parameters: &super::ApiParameters) -> result::Result<Instance, Result> {
    // 1. Get available instance layers and extensions
    let available_layers: collections::HashSet<ffi::CString> =
        vk_enumerate_instance_layer_properties()?
            .iter()
            .cloned()
            .collect();
    let available_extensions: collections::HashSet<ffi::CString> =
        vk_enumerate_instance_extension_properties()?
            .iter()
            .cloned()
            .collect();

    let mut enabled_layers_cstrings: Vec<ffi::CString> = Vec::new();
    let mut enabled_extensions_cstrings: Vec<ffi::CString> = Vec::new();

    // 2. Add platform-specific surface extensions
    enabled_extensions_cstrings.push(ffi::CString::new("VK_KHR_surface").unwrap());

    match parameters.platform {
        super::Platform::Windows => {
            enabled_extensions_cstrings.push(ffi::CString::new("VK_KHR_win32_surface").unwrap());
        }
        super::Platform::X11 => {
            enabled_extensions_cstrings.push(ffi::CString::new("VK_KHR_xlib_surface").unwrap());
            enabled_extensions_cstrings.push(ffi::CString::new("VK_KHR_xcb_surface").unwrap());
        }
        super::Platform::Wayland => {
            enabled_extensions_cstrings.push(ffi::CString::new("VK_KHR_wayland_surface").unwrap());
        }
        // Error if unsupported platform
        _ => {
            return Err(Result::NotSupported);
        }
    }

    // 3. Add validation and debug utils extension if debugging is requested and available
    let validation_name = c"VK_LAYER_KHRONOS_validation";
    let validation_present = available_layers.contains(validation_name);
    let debug_utils_name = c"VK_EXT_debug_utils";
    let debug_utils_present = available_extensions.contains(debug_utils_name);

    let enable_debugging = parameters.enable_debugging && validation_present && debug_utils_present;
    if enable_debugging {
        enabled_layers_cstrings.push(validation_name.to_owned());
        enabled_extensions_cstrings.push(debug_utils_name.to_owned());
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
        make_result(vulkan_ffi::vkCreateInstance(
            &create_info,
            std::ptr::null(),
            &mut instance,
        ))
    };

    match result {
        Result::Success => {
            if enable_debugging {
                let debug_utils_create_fn_name = c"vkCreateDebugUtilsMessengerEXT";
                let create_debug_fn: vulkan_ffi::PFN_vkCreateDebugUtilsMessengerEXT = unsafe {
                    std::mem::transmute(vulkan_ffi::vkGetInstanceProcAddr(
                        instance as u64,
                        debug_utils_create_fn_name.as_ptr(),
                    ))
                };

                if create_debug_fn.is_none() {
                    return Err(Result::NotSupported);
                }

                warning!("Debugging Enabled");

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
                })
            } else {
                Ok(Instance {
                    handle: instance,
                    debug_messenger: None,
                })
            }
        }
        _ => Err(result),
    }
}
