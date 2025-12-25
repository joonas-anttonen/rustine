#![allow(dead_code)]

use std::{collections, result};

use crate::{debug, error, warning};
use crate::{gfx::*, version::Version};

use crate::gfx::vulkan_ffi as ffi;

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
        make_result(ffi::vkEnumerateInstanceVersion(
            &mut api_version as *mut u32,
        ))
    };

    match result {
        Result::Success => Ok(Version::from_vk_version(api_version)),
        _ => Err(result),
    }
}

/// Queries the available instance layers.
pub fn vk_enumerate_instance_layer_properties() -> result::Result<Vec<std::ffi::CString>, Result> {
    let mut property_count: u32 = 0;
    let result = unsafe {
        make_result(ffi::vkEnumerateInstanceLayerProperties(
            &mut property_count as *mut u32,
            std::ptr::null_mut(),
        ))
    };

    match result {
        Result::Success => {
            let mut properties: Vec<ffi::VkLayerProperties> =
                Vec::with_capacity(property_count as usize);
            let result = unsafe {
                make_result(ffi::vkEnumerateInstanceLayerProperties(
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
pub fn vk_enumerate_instance_extension_properties() -> result::Result<Vec<std::ffi::CString>, Result>
{
    let mut property_count: u32 = 0;
    let result = unsafe {
        make_result(ffi::vkEnumerateInstanceExtensionProperties(
            std::ptr::null(),
            &mut property_count as *mut u32,
            std::ptr::null_mut(),
        ))
    };

    match result {
        Result::Success => {
            let mut properties: Vec<ffi::VkExtensionProperties> =
                Vec::with_capacity(property_count as usize);
            let result = unsafe {
                make_result(ffi::vkEnumerateInstanceExtensionProperties(
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

pub fn vk_enumerate_physical_device_extensions(
    physical_device: ffi::VkPhysicalDevice,
) -> result::Result<Vec<std::ffi::CString>, Result> {
    let mut property_count: u32 = 0;
    let result = unsafe {
        make_result(ffi::vkEnumerateDeviceExtensionProperties(
            physical_device,
            std::ptr::null(),
            &mut property_count as *mut u32,
            std::ptr::null_mut(),
        ))
    };

    match result {
        Result::Success => {
            let mut properties: Vec<ffi::VkExtensionProperties> =
                Vec::with_capacity(property_count as usize);
            let result = unsafe {
                make_result(ffi::vkEnumerateDeviceExtensionProperties(
                    physical_device,
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

pub fn vk_enumerate_physical_device_queue_families(
    physical_device: ffi::VkPhysicalDevice,
) -> Vec<ffi::VkQueueFamilyProperties> {
    let mut queue_family_count: u32 = 0;
    unsafe {
        ffi::vkGetPhysicalDeviceQueueFamilyProperties(
            physical_device,
            &mut queue_family_count as *mut u32,
            std::ptr::null_mut(),
        );
    }

    let mut properties: Vec<ffi::VkQueueFamilyProperties> =
        Vec::with_capacity(queue_family_count as usize);
    unsafe {
        ffi::vkGetPhysicalDeviceQueueFamilyProperties(
            physical_device,
            &mut queue_family_count as *mut u32,
            properties.as_mut_ptr(),
        );
        properties.set_len(queue_family_count as usize);
    }
    properties
}

/// Represents a Vulkan instance.
pub struct Instance {
    handle: ffi::VkInstance,
    debug_messenger: Option<ffi::VkDebugUtilsMessengerEXT>,
}

pub struct Device {
    handle: ffi::VkDevice,
    physical_device: ffi::VkPhysicalDevice,
}

impl Drop for Instance {
    fn drop(&mut self) {
        warning!("Instance::drop");

        if let Some(messenger) = self.debug_messenger {
            let debug_utils_destroy_fn_name = c"vkDestroyDebugUtilsMessengerEXT";
            let destroy_debug_fn: ffi::PFN_vkDestroyDebugUtilsMessengerEXT = unsafe {
                std::mem::transmute(ffi::vkGetInstanceProcAddr(
                    self.handle as u64,
                    debug_utils_destroy_fn_name.as_ptr(),
                ))
            };

            unsafe {
                destroy_debug_fn.unwrap()(self.handle, messenger, std::ptr::null());
            }
        }

        unsafe {
            ffi::vkDestroyInstance(self.handle, std::ptr::null());
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        warning!("Device::drop");
        unsafe {
            ffi::vkDestroyDevice(self.handle, std::ptr::null());
        }
    }
}

/// Enumerates physical devices (GPUs) available on the system.
pub fn vk_enumerate_physical_devices(
    instance: &Instance,
) -> result::Result<Vec<PhysicalDevice>, Result> {
    let mut device_count: u32 = 0;
    let result = unsafe {
        make_result(ffi::vkEnumeratePhysicalDevices(
            instance.handle,
            &mut device_count as *mut u32,
            std::ptr::null_mut(),
        ))
    };

    match result {
        Result::Success => {
            let mut devices: Vec<u64> = Vec::with_capacity(device_count as usize);
            let result = unsafe {
                make_result(ffi::vkEnumeratePhysicalDevices(
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
                            let mut id_properties: ffi::VkPhysicalDeviceIDProperties =
                                unsafe { std::mem::zeroed() };
                            id_properties.sType = ffi::VkStructureType::VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_ID_PROPERTIES as u32;

                            let mut properties: ffi::VkPhysicalDeviceProperties2 =
                                unsafe { std::mem::zeroed() };
                            properties.sType = ffi::VkStructureType::VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_PROPERTIES_2 as u32;
                            properties.pNext = &mut id_properties as *mut ffi::VkPhysicalDeviceIDProperties as *mut std::ffi::c_void;
                            unsafe {
                                ffi::vkGetPhysicalDeviceProperties2(
                                    device_handle,
                                    &mut properties as *mut ffi::VkPhysicalDeviceProperties2,
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
                                handle: device_handle,
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
    callback_data: *const ffi::VkDebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::ffi::c_void,
) -> u32 {
    if callback_data.is_null() {
        return 0;
    }
    unsafe {
        let data = &*callback_data;
        if data.pMessage.is_null() {
            return 0;
        }

        let message = std::ffi::CStr::from_ptr(data.pMessage).to_string_lossy();
        error!("Vulkan: {}", message);
        0
    }
}

pub fn vk_create_device(physical_device: ffi::VkPhysicalDevice) -> result::Result<Device, Result> {
    let _available_device_extensions: collections::HashSet<std::ffi::CString> =
        vk_enumerate_physical_device_extensions(physical_device)?
            .into_iter()
            .collect();

    let mut enabled_extensions_cstrings: Vec<std::ffi::CString> = Vec::new();
    enabled_extensions_cstrings.push(std::ffi::CString::new("VK_KHR_swapchain").unwrap());

    let _enabled_extensions_ptrs: Vec<*const std::ffi::c_char> = enabled_extensions_cstrings
        .iter()
        .map(|cs| cs.as_ptr())
        .collect();

    let mut physical_device_dynamic_rendering = unsafe {
        ffi::VkPhysicalDeviceDynamicRenderingFeatures {
            sType: ffi::VkStructureType::PHYSICAL_DEVICE_DYNAMIC_RENDERING_FEATURES as u32,
            ..std::mem::zeroed()
        }
    };
    let mut physical_device_features = unsafe {
        ffi::VkPhysicalDeviceFeatures2 {
            sType: ffi::VkStructureType::PHYSICAL_DEVICE_FEATURES_2 as u32,
            pNext: &mut physical_device_dynamic_rendering
                as *mut ffi::VkPhysicalDeviceDynamicRenderingFeatures
                as *mut std::ffi::c_void,
            ..std::mem::zeroed()
        }
    };
    unsafe {
        ffi::vkGetPhysicalDeviceFeatures2(physical_device, &mut physical_device_features);
    }

    let queue_family_properties = vk_enumerate_physical_device_queue_families(physical_device);
    let general_queue_family_index = queue_family_properties
        .iter()
        .position(|qf| (qf.queueFlags & ffi::VkQueueFlagBits::VK_QUEUE_GRAPHICS_BIT as u32) != 0)
        .ok_or(Result::NotSupported)?;
    let queue_priority: f32 = 1.0;
    let queue_create_info = ffi::VkDeviceQueueCreateInfo {
        sType: ffi::VkStructureType::VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO as u32,
        pNext: std::ptr::null(),
        flags: 0,
        queueFamilyIndex: general_queue_family_index as u32,
        queueCount: 1,
        pQueuePriorities: &queue_priority as *const f32,
    };
    let device_create_info = ffi::VkDeviceCreateInfo {
        sType: ffi::VkStructureType::VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO as u32,
        pNext: &mut physical_device_features as *mut ffi::VkPhysicalDeviceFeatures2
            as *mut std::ffi::c_void,
        flags: 0,
        queueCreateInfoCount: 1,
        pQueueCreateInfos: &queue_create_info as *const ffi::VkDeviceQueueCreateInfo,
        enabledLayerCount: 0,
        ppEnabledLayerNames: std::ptr::null(),
        enabledExtensionCount: _enabled_extensions_ptrs.len() as u32,
        ppEnabledExtensionNames: _enabled_extensions_ptrs.as_ptr(),
        pEnabledFeatures: std::ptr::null(),
    };

    let mut device_handle: ffi::VkDevice = std::ptr::null_mut();
    let result = unsafe {
        make_result(ffi::vkCreateDevice(
            physical_device,
            &device_create_info,
            std::ptr::null(),
            &mut device_handle,
        ))
    };
    match result {
        Result::Success => Ok(Device {
            handle: device_handle,
            physical_device,
        }),
        _ => Err(result),
    }
}

/// Creates a Vulkan instance based on the provided parameters.
pub fn vk_create_instance(parameters: &super::ApiParameters) -> result::Result<Instance, Result> {
    // 1. Get available instance layers and extensions
    let available_layers: collections::HashSet<std::ffi::CString> =
        vk_enumerate_instance_layer_properties()?
            .into_iter()
            .collect();
    let available_extensions: collections::HashSet<std::ffi::CString> =
        vk_enumerate_instance_extension_properties()?
            .into_iter()
            .collect();

    let mut enabled_layers_cstrings: Vec<std::ffi::CString> = Vec::new();
    let mut enabled_extensions_cstrings: Vec<std::ffi::CString> = Vec::new();

    // 2. Add platform-specific surface extensions
    enabled_extensions_cstrings.push(std::ffi::CString::new("VK_KHR_surface").unwrap());

    match parameters.platform {
        super::Platform::Windows => {
            enabled_extensions_cstrings
                .push(std::ffi::CString::new("VK_KHR_win32_surface").unwrap());
        }
        super::Platform::X11 => {
            enabled_extensions_cstrings
                .push(std::ffi::CString::new("VK_KHR_xlib_surface").unwrap());
            enabled_extensions_cstrings.push(std::ffi::CString::new("VK_KHR_xcb_surface").unwrap());
        }
        super::Platform::Wayland => {
            enabled_extensions_cstrings
                .push(std::ffi::CString::new("VK_KHR_wayland_surface").unwrap());
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
    let app_name_cstring = std::ffi::CString::new(parameters.app_name.as_str()).unwrap();
    let engine_name_cstring = std::ffi::CString::new(parameters.app_engine_name.as_str()).unwrap();
    let enabled_layers_ptrs: Vec<*const std::ffi::c_char> = enabled_layers_cstrings
        .iter()
        .map(|cs| cs.as_ptr())
        .collect();

    let enabled_extensions_ptrs: Vec<*const std::ffi::c_char> = enabled_extensions_cstrings
        .iter()
        .map(|cs| cs.as_ptr())
        .collect();

    let app_info = ffi::VkApplicationInfo {
        sType: ffi::VkStructureType::VK_STRUCTURE_TYPE_APPLICATION_INFO as u32,
        pNext: std::ptr::null(),
        pApplicationName: app_name_cstring.as_ptr(),
        applicationVersion: parameters.app_version.to_vk_version(),
        pEngineName: engine_name_cstring.as_ptr(),
        engineVersion: parameters.app_engine_version.to_vk_version(),
        apiVersion: parameters.required_api_version.to_vk_version(),
    };

    let create_info = ffi::VkInstanceCreateInfo {
        sType: ffi::VkStructureType::VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO as u32,
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

    let mut instance: ffi::VkInstance = std::ptr::null_mut();
    let result = unsafe {
        make_result(ffi::vkCreateInstance(
            &create_info,
            std::ptr::null(),
            &mut instance,
        ))
    };

    match result {
        Result::Success => {
            if enable_debugging {
                let debug_utils_create_fn_name = c"vkCreateDebugUtilsMessengerEXT";
                let create_debug_fn: ffi::PFN_vkCreateDebugUtilsMessengerEXT = unsafe {
                    std::mem::transmute(ffi::vkGetInstanceProcAddr(
                        instance as u64,
                        debug_utils_create_fn_name.as_ptr(),
                    ))
                };

                if create_debug_fn.is_none() {
                    return Err(Result::NotSupported);
                }

                warning!("Debugging Enabled");

                // Create debug messenger info
                let debug_create_info = ffi::VkDebugUtilsMessengerCreateInfoEXT {
                        sType: ffi::VkStructureType::VK_STRUCTURE_TYPE_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT as u32,
                        pNext: std::ptr::null(),
                        flags: 0,
                        messageSeverity:
                            ffi::VkDebugUtilsMessageSeverityFlagBitsEXT::VK_DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT as u32 |
                            ffi::VkDebugUtilsMessageSeverityFlagBitsEXT::VK_DEBUG_UTILS_MESSAGE_SEVERITY_WARNING_BIT_EXT as u32,
                        messageType:
                            ffi::VkDebugUtilsMessageTypeFlagBitsEXT::VK_DEBUG_UTILS_MESSAGE_TYPE_GENERAL_BIT_EXT as u32 |
                            ffi::VkDebugUtilsMessageTypeFlagBitsEXT::VK_DEBUG_UTILS_MESSAGE_TYPE_VALIDATION_BIT_EXT as u32,
                        pfnUserCallback: Some(vulkan_debug_callback),
                        pUserData: std::ptr::null_mut(),
                    };

                let mut debug_messenger_ptr: ffi::VkDebugUtilsMessengerEXT = std::ptr::null_mut();
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
