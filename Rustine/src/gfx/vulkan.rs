#![allow(dead_code)]

use std::{collections, ptr, result};

use crate::{error, warning};
use crate::{gfx::*, version::Version};

use crate::gfx::vulkan_ffi as ffi;

/// Wraps a Vulkan function call and converts the result to `gfx::Result`.
#[macro_export]
macro_rules! vk_call {
    ($expr:expr) => {{
        let res = unsafe { $expr };
        match crate::gfx::Result::from_code(res) {
            crate::gfx::Result::Success => Ok(()),
            other => Err(other),
        }
    }};
}

/// Represents a Vulkan instance.
pub struct Instance {
    handle: ffi::VkInstance,
    debug_messenger: Option<ffi::VkDebugUtilsMessengerEXT>,
}

impl Instance {
    pub fn handle(&self) -> ffi::VkInstance {
        self.handle
    }
}

/// Represents a Vulkan logical device.
pub struct Device {
    queue: ffi::VkQueue,
    handle: ffi::VkDevice,
    physical_device: ffi::VkPhysicalDevice,
}

impl Device {
    pub fn handle(&self) -> ffi::VkDevice {
        self.handle
    }

    pub fn physical_device_handle(&self) -> ffi::VkPhysicalDevice {
        self.physical_device
    }

    pub fn queue_handle(&self) -> ffi::VkQueue {
        self.queue
    }
}

/// Represents a command pool owned by a single thread.
pub struct CommandPool {
    handle: ffi::VkCommandPool,
    available_command_buffers: collections::VecDeque<CommandBuffer>,
}

impl CommandPool {
    /// Creates a new command pool with pre-allocated command buffers.
    pub fn new(pool: ffi::VkCommandPool, buffers: Vec<CommandBuffer>) -> Self {
        CommandPool {
            handle: pool,
            available_command_buffers: collections::VecDeque::from(buffers),
        }
    }

    /// Checks if the pool has no available command buffers.
    pub fn is_empty(&self) -> bool {
        self.available_command_buffers.is_empty()
    }

    /// Rents a command buffer from the pool, or returns None if empty.
    pub fn rent_buffer(&mut self) -> Option<CommandBuffer> {
        self.available_command_buffers.pop_front()
    }

    /// Returns a command buffer to the pool.
    pub fn return_buffer(&mut self, buffer: CommandBuffer) {
        self.available_command_buffers.push_back(buffer);
    }
}

/// Represents a command buffer allocated from a command pool.
pub struct CommandBuffer {
    handle: ffi::VkCommandBuffer,
    fence: ffi::VkFence,
}

impl CommandBuffer {
    /// Creates a new command buffer with the given handle and fence.
    pub fn new(handle: ffi::VkCommandBuffer, fence: ffi::VkFence) -> Self {
        CommandBuffer { handle, fence }
    }

    /// Begins recording commands into the command buffer.
    pub fn begin(&self) {
        let begin_info = ffi::VkCommandBufferBeginInfo {
            sType: ffi::VkStructureType::COMMAND_BUFFER_BEGIN_INFO as u32,
            pNext: ptr::null(),
            flags: 0,
            pInheritanceInfo: ptr::null(),
        };
        vk_call!(ffi::vkBeginCommandBuffer(self.handle, &begin_info)).unwrap_or_else(|r| {
            error!("Failed to begin command buffer: {:?}", r);
        });
    }

    /// Ends recording commands into the command buffer.
    pub fn end(&self) {
        vk_call!(ffi::vkEndCommandBuffer(self.handle)).unwrap_or_else(|r| {
            error!("Failed to end command buffer: {:?}", r);
        });
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        warning!("Instance::drop");

        if let Some(messenger) = self.debug_messenger {
            let debug_utils_destroy_fn_name = c"vkDestroyDebugUtilsMessengerEXT";
            let destroy_debug_fn: ffi::PFN_vkDestroyDebugUtilsMessengerEXT = unsafe {
                std::mem::transmute(ffi::vkGetInstanceProcAddr(
                    self.handle,
                    debug_utils_destroy_fn_name.as_ptr(),
                ))
            };

            unsafe {
                destroy_debug_fn.unwrap()(self.handle, messenger, ptr::null());
            }
        }

        unsafe {
            ffi::vkDestroyInstance(self.handle, ptr::null());
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        warning!("Device::drop");
        unsafe {
            ffi::vkDestroyDevice(self.handle, ptr::null());
        }
    }
}

/// Queries the highest Vulkan API version supported.
pub fn enumerate_instance_version() -> result::Result<Version, Result> {
    let mut api_version: u32 = 0;
    vk_call!(ffi::vkEnumerateInstanceVersion(
        &mut api_version as *mut u32
    ))?;
    Ok(Version::from_vk_version(api_version))
}

/// Queries the available instance layers.
pub fn enumerate_instance_layers() -> result::Result<Vec<std::ffi::CString>, Result> {
    let mut property_count: u32 = 0;
    vk_call!(ffi::vkEnumerateInstanceLayerProperties(
        &mut property_count as *mut u32,
        ptr::null_mut(),
    ))?;

    let mut properties: Vec<ffi::VkLayerProperties> = Vec::with_capacity(property_count as usize);
    vk_call!(ffi::vkEnumerateInstanceLayerProperties(
        &mut property_count as *mut u32,
        properties.as_mut_ptr(),
    ))?;

    unsafe {
        properties.set_len(property_count as usize);
        let layer_names = properties
            .iter()
            .map(|prop| {
                let cstr = std::ffi::CStr::from_ptr(prop.layerName.as_ptr() as *const i8);
                cstr.to_owned()
            })
            .collect();
        Ok(layer_names)
    }
}

/// Queries the available instance extensions.
pub fn enumerate_instance_extensions() -> result::Result<Vec<std::ffi::CString>, Result> {
    let mut property_count: u32 = 0;
    vk_call!(ffi::vkEnumerateInstanceExtensionProperties(
        ptr::null(),
        &mut property_count as *mut u32,
        ptr::null_mut(),
    ))?;

    let mut properties: Vec<ffi::VkExtensionProperties> =
        Vec::with_capacity(property_count as usize);
    vk_call!(ffi::vkEnumerateInstanceExtensionProperties(
        ptr::null(),
        &mut property_count as *mut u32,
        properties.as_mut_ptr(),
    ))?;

    unsafe {
        properties.set_len(property_count as usize);

        let extension_names = properties
            .iter()
            .map(|prop| {
                let cstr = std::ffi::CStr::from_ptr(prop.extensionName.as_ptr() as *const i8);
                cstr.to_owned()
            })
            .collect();
        Ok(extension_names)
    }
}

pub fn enumerate_physical_device_extensions(
    physical_device: ffi::VkPhysicalDevice,
) -> result::Result<Vec<std::ffi::CString>, Result> {
    let mut property_count: u32 = 0;
    vk_call!(ffi::vkEnumerateDeviceExtensionProperties(
        physical_device,
        ptr::null(),
        &mut property_count as *mut u32,
        ptr::null_mut(),
    ))?;

    let mut properties: Vec<ffi::VkExtensionProperties> =
        Vec::with_capacity(property_count as usize);
    vk_call!(ffi::vkEnumerateDeviceExtensionProperties(
        physical_device,
        ptr::null(),
        &mut property_count as *mut u32,
        properties.as_mut_ptr(),
    ))?;

    unsafe {
        properties.set_len(property_count as usize);
        let extension_names = properties
            .iter()
            .map(|prop| {
                let cstr = std::ffi::CStr::from_ptr(prop.extensionName.as_ptr() as *const i8);
                cstr.to_owned()
            })
            .collect();
        Ok(extension_names)
    }
}

pub fn enumerate_physical_device_queue_families(
    physical_device: ffi::VkPhysicalDevice,
) -> Vec<ffi::VkQueueFamilyProperties> {
    let mut queue_family_count: u32 = 0;
    unsafe {
        ffi::vkGetPhysicalDeviceQueueFamilyProperties(
            physical_device,
            &mut queue_family_count as *mut u32,
            ptr::null_mut(),
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

/// Enumerates physical devices (GPUs) available on the system.
pub fn enumerate_physical_devices(
    instance: &Instance,
) -> result::Result<Vec<PhysicalDevice>, Result> {
    let mut device_count: u32 = 0;
    vk_call!(ffi::vkEnumeratePhysicalDevices(
        instance.handle,
        &mut device_count as *mut u32,
        ptr::null_mut(),
    ))?;

    let mut devices: Vec<ffi::VkPhysicalDevice> = Vec::with_capacity(device_count as usize);
    vk_call!(ffi::vkEnumeratePhysicalDevices(
        instance.handle,
        &mut device_count as *mut u32,
        devices.as_mut_ptr(),
    ))?;

    unsafe {
        devices.set_len(device_count as usize);
    }

    let physical_devices: Vec<PhysicalDevice> = devices
        .iter()
        .map(|&device_handle| {
            let mut id_properties = unsafe {
                ffi::VkPhysicalDeviceIDProperties {
                    sType: ffi::VkStructureType::PHYSICAL_DEVICE_ID_PROPERTIES as u32,
                    ..std::mem::zeroed()
                }
            };

            let mut properties = unsafe {
                ffi::VkPhysicalDeviceProperties2 {
                    sType: ffi::VkStructureType::PHYSICAL_DEVICE_PROPERTIES_2 as u32,
                    pNext: &mut id_properties as *mut ffi::VkPhysicalDeviceIDProperties
                        as *mut std::ffi::c_void,
                    ..std::mem::zeroed()
                }
            };
            unsafe {
                ffi::vkGetPhysicalDeviceProperties2(
                    device_handle,
                    &mut properties as *mut ffi::VkPhysicalDeviceProperties2,
                );
            }

            let device_name = {
                let null_pos = properties
                    .properties
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
                handle: device_handle as u64,
            }
        })
        .collect();

    Ok(physical_devices)
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

pub fn create_device(
    parameters: &super::ApiParameters,
    physical_device: ffi::VkPhysicalDevice,
) -> result::Result<Device, Result> {
    let available_device_extensions: collections::HashSet<std::ffi::CString> =
        enumerate_physical_device_extensions(physical_device)?
            .into_iter()
            .collect();

    let mut enabled_extensions_cstrings: Vec<std::ffi::CString> = Vec::new();
    enabled_extensions_cstrings.push(std::ffi::CString::new("VK_KHR_swapchain").unwrap());
    
    if parameters.platform == Platform::Windows {
        let external_memory_win32_name = c"VK_KHR_external_memory_win32";
        if available_device_extensions.contains(external_memory_win32_name) {
            warning!("Enabling external memory Win32 extension");
            enabled_extensions_cstrings
                .push(std::ffi::CString::new("VK_KHR_external_memory_win32").unwrap());
        }
    }

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

    let queue_family_properties = enumerate_physical_device_queue_families(physical_device);
    let general_queue_family_index = queue_family_properties
        .iter()
        .position(|qf| (qf.queueFlags & ffi::VkQueueFlags::GRAPHICS_BIT as u32) != 0)
        .ok_or(Result::NotSupported)?;
    let queue_priority: f32 = 1.0;
    let queue_create_info = ffi::VkDeviceQueueCreateInfo {
        sType: ffi::VkStructureType::DEVICE_QUEUE_CREATE_INFO as u32,
        pNext: ptr::null(),
        flags: 0,
        queueFamilyIndex: general_queue_family_index as u32,
        queueCount: 1,
        pQueuePriorities: &queue_priority as *const f32,
    };
    let device_create_info = ffi::VkDeviceCreateInfo {
        sType: ffi::VkStructureType::DEVICE_CREATE_INFO as u32,
        pNext: &mut physical_device_features as *mut ffi::VkPhysicalDeviceFeatures2
            as *mut std::ffi::c_void,
        flags: 0,
        queueCreateInfoCount: 1,
        pQueueCreateInfos: &queue_create_info as *const ffi::VkDeviceQueueCreateInfo,
        enabledLayerCount: 0,
        ppEnabledLayerNames: ptr::null(),
        enabledExtensionCount: _enabled_extensions_ptrs.len() as u32,
        ppEnabledExtensionNames: _enabled_extensions_ptrs.as_ptr(),
        pEnabledFeatures: ptr::null(),
    };

    let mut device_handle: ffi::VkDevice = ptr::null_mut();
    vk_call!(ffi::vkCreateDevice(
        physical_device,
        &device_create_info,
        ptr::null(),
        &mut device_handle,
    ))?;

    let mut queue_handle: ffi::VkQueue = ptr::null_mut();
    unsafe {
        ffi::vkGetDeviceQueue(
            device_handle,
            general_queue_family_index as u32,
            0,
            &mut queue_handle,
        );
    }

    Ok(Device {
        handle: device_handle,
        physical_device,
        queue: queue_handle,
    })
}

/// Creates a Vulkan instance based on the provided parameters.
pub fn create_instance(parameters: &super::ApiParameters) -> result::Result<Instance, Result> {
    // 1. Get available instance layers and extensions
    let available_layers: collections::HashSet<std::ffi::CString> =
        enumerate_instance_layers()?.into_iter().collect();
    let available_extensions: collections::HashSet<std::ffi::CString> =
        enumerate_instance_extensions()?.into_iter().collect();

    let mut enabled_layers: Vec<std::ffi::CString> = Vec::new();
    let mut enabled_extensions: Vec<std::ffi::CString> = Vec::new();

    // 2. Add platform-specific surface extensions
    enabled_extensions.push(std::ffi::CString::new("VK_KHR_surface").unwrap());

    match parameters.platform {
        Platform::Windows => {
            enabled_extensions.push(std::ffi::CString::new("VK_KHR_win32_surface").unwrap());
        }
        Platform::X11 => {
            enabled_extensions.push(std::ffi::CString::new("VK_KHR_xlib_surface").unwrap());
            enabled_extensions.push(std::ffi::CString::new("VK_KHR_xcb_surface").unwrap());
        }
        Platform::Wayland => {
            enabled_extensions.push(std::ffi::CString::new("VK_KHR_wayland_surface").unwrap());
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
        enabled_layers.push(validation_name.to_owned());
        enabled_extensions.push(debug_utils_name.to_owned());
    }

    // 4. Create the Vulkan instance
    let app_name_cstring = std::ffi::CString::new(parameters.app_name.as_str()).unwrap();
    let engine_name_cstring = std::ffi::CString::new(parameters.app_engine_name.as_str()).unwrap();
    let enabled_layers_ptrs: Vec<*const std::ffi::c_char> =
        enabled_layers.iter().map(|cs| cs.as_ptr()).collect();
    let enabled_extensions_ptrs: Vec<*const std::ffi::c_char> =
        enabled_extensions.iter().map(|cs| cs.as_ptr()).collect();

    let app_info = ffi::VkApplicationInfo {
        sType: ffi::VkStructureType::APPLICATION_INFO as u32,
        pNext: ptr::null(),
        pApplicationName: app_name_cstring.as_ptr(),
        applicationVersion: parameters.app_version.to_vk_version(),
        pEngineName: engine_name_cstring.as_ptr(),
        engineVersion: parameters.app_engine_version.to_vk_version(),
        apiVersion: parameters.required_api_version.to_vk_version(),
    };

    let create_info = ffi::VkInstanceCreateInfo {
        sType: ffi::VkStructureType::INSTANCE_CREATE_INFO as u32,
        pNext: ptr::null(),
        flags: 0,
        pApplicationInfo: &app_info,
        enabledLayerCount: enabled_layers_ptrs.len() as u32,
        ppEnabledLayerNames: if enabled_layers_ptrs.is_empty() {
            ptr::null()
        } else {
            enabled_layers_ptrs.as_ptr()
        },
        enabledExtensionCount: enabled_extensions_ptrs.len() as u32,
        ppEnabledExtensionNames: if enabled_extensions_ptrs.is_empty() {
            ptr::null()
        } else {
            enabled_extensions_ptrs.as_ptr()
        },
    };

    let mut instance_handle: ffi::VkInstance = ptr::null_mut();
    vk_call!(ffi::vkCreateInstance(
        &create_info,
        ptr::null(),
        &mut instance_handle,
    ))?;

    let mut debug_messenger: Option<ffi::VkDebugUtilsMessengerEXT> = None;
    if enable_debugging {
        let debug_utils_create_fn_name = c"vkCreateDebugUtilsMessengerEXT";
        let create_debug_fn: ffi::PFN_vkCreateDebugUtilsMessengerEXT = unsafe {
            std::mem::transmute(ffi::vkGetInstanceProcAddr(
                instance_handle,
                debug_utils_create_fn_name.as_ptr(),
            ))
        };

        if create_debug_fn.is_none() {
            return Err(Result::NotSupported);
        }

        warning!("Debugging Enabled");

        // Create debug messenger info
        let debug_create_info = ffi::VkDebugUtilsMessengerCreateInfoEXT {
            sType: ffi::VkStructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT as u32,
            pNext: ptr::null(),
            flags: 0,
            messageSeverity: ffi::VkDebugUtilsMessageSeverityFlagsEXT::ERROR_BIT_EXT as u32
                | ffi::VkDebugUtilsMessageSeverityFlagsEXT::WARNING_BIT_EXT as u32,
            messageType: ffi::VkDebugUtilsMessageTypeFlagsEXT::GENERAL_BIT_EXT as u32
                | ffi::VkDebugUtilsMessageTypeFlagsEXT::VALIDATION_BIT_EXT as u32,
            pfnUserCallback: Some(vulkan_debug_callback),
            pUserData: ptr::null_mut(),
        };

        let mut debug_messenger_ptr: ffi::VkDebugUtilsMessengerEXT = ptr::null_mut();
        vk_call!(create_debug_fn.unwrap()(
            instance_handle,
            &debug_create_info,
            ptr::null(),
            &mut debug_messenger_ptr,
        ))?;

        debug_messenger = Some(debug_messenger_ptr);
    }

    Ok(Instance {
        handle: instance_handle,
        debug_messenger: debug_messenger,
    })
}
