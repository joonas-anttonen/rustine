#![allow(dead_code)]

use std::{
    collections, ptr,
    sync::{Arc, Mutex},
};

use crate::{error, warning};
use crate::{gfx::vulkan_ffi as vk, gfx::*, version::Version};

/// Wraps a Vulkan function call and converts the result to `gfx::Result`.
#[macro_export]
macro_rules! vk_call {
    ($expr:expr) => {{
        let res = unsafe { $expr };
        match crate::gfx::Status::from_code(res) {
            crate::gfx::Status::Success => Ok(()),
            other => Err(other),
        }
    }};
}

/// Converts a reference to a Vulkan `pNext` pointer.
#[macro_export]
macro_rules! vk_next {
    ($ptr:expr) => {
        &$ptr as *const _ as *const _
    };
    (mut $ptr:expr) => {
        &mut $ptr as *mut _ as *mut _
    };
}

/// Represents a Vulkan instance.
pub struct Instance {
    handle: vk::VkInstance,
    debug_messenger: Option<vk::VkDebugUtilsMessengerEXT>,
}

impl Instance {
    pub fn handle(&self) -> vk::VkInstance {
        self.handle
    }
}

/// Represents a Vulkan logical device.
pub struct Device {
    general_queue_family_index: u32,
    transfer_queue_family_index: u32,
    general_queue_handle: Arc<Mutex<vk::VkQueue>>,
    transfer_queue_handle: Arc<Mutex<vk::VkQueue>>,
    handle: vk::VkDevice,
    physical_device: vk::VkPhysicalDevice,
}

impl Device {
    pub fn handle(&self) -> vk::VkDevice {
        self.handle
    }

    pub fn physical_device_handle(&self) -> vk::VkPhysicalDevice {
        self.physical_device
    }

    pub fn general_queue_family_index(&self) -> u32 {
        self.general_queue_family_index
    }
    pub fn transfer_queue_family_index(&self) -> u32 {
        self.transfer_queue_family_index
    }

    pub fn general_queue(&self) -> Arc<Mutex<vk::VkQueue>> {
        Arc::clone(&self.general_queue_handle)
    }
    pub fn transfer_queue(&self) -> Arc<Mutex<vk::VkQueue>> {
        Arc::clone(&self.transfer_queue_handle)
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        warning!("Instance::drop");

        if let Some(messenger) = self.debug_messenger {
            let debug_utils_destroy_fn_name = c"vkDestroyDebugUtilsMessengerEXT";
            let destroy_debug_fn: vk::PFN_vkDestroyDebugUtilsMessengerEXT = unsafe {
                std::mem::transmute(vk::vkGetInstanceProcAddr(
                    self.handle,
                    debug_utils_destroy_fn_name.as_ptr(),
                ))
            };

            unsafe {
                destroy_debug_fn.unwrap()(self.handle, messenger, ptr::null());
            }
        }

        unsafe {
            vk::vkDestroyInstance(self.handle, ptr::null());
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        warning!("Device::drop");
        unsafe {
            vk::vkDestroyDevice(self.handle, ptr::null());
        }
    }
}

/// Queries the highest Vulkan API version supported.
pub fn enumerate_instance_version() -> Result<Version> {
    let mut api_version: u32 = 0;
    vk_call!(vk::vkEnumerateInstanceVersion(&mut api_version as *mut u32))?;
    Ok(Version::from_vk_version(api_version))
}

/// Queries the available instance layers.
pub fn enumerate_instance_layers() -> Result<Vec<std::ffi::CString>> {
    let mut property_count: u32 = 0;
    vk_call!(vk::vkEnumerateInstanceLayerProperties(
        &mut property_count as *mut u32,
        ptr::null_mut(),
    ))?;

    let mut properties: Vec<vk::VkLayerProperties> = Vec::with_capacity(property_count as usize);
    vk_call!(vk::vkEnumerateInstanceLayerProperties(
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
pub fn enumerate_instance_extensions() -> Result<Vec<std::ffi::CString>> {
    let mut property_count: u32 = 0;
    vk_call!(vk::vkEnumerateInstanceExtensionProperties(
        ptr::null(),
        &mut property_count as *mut u32,
        ptr::null_mut(),
    ))?;

    let mut properties: Vec<vk::VkExtensionProperties> =
        Vec::with_capacity(property_count as usize);
    vk_call!(vk::vkEnumerateInstanceExtensionProperties(
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

pub fn enumerate_physical_device_surface_formats(
    physical_device: vk::VkPhysicalDevice,
    surface: vk::VkSurfaceKHR,
) -> Result<Vec<vk::VkSurfaceFormatKHR>> {
    let mut format_count: u32 = 0;
    vk_call!(vk::vkGetPhysicalDeviceSurfaceFormatsKHR(
        physical_device,
        surface,
        &mut format_count as *mut u32,
        ptr::null_mut(),
    ))?;

    let mut formats: Vec<vk::VkSurfaceFormatKHR> = Vec::with_capacity(format_count as usize);
    vk_call!(vk::vkGetPhysicalDeviceSurfaceFormatsKHR(
        physical_device,
        surface,
        &mut format_count as *mut u32,
        formats.as_mut_ptr(),
    ))?;

    unsafe {
        formats.set_len(format_count as usize);
    }

    Ok(formats)
}

pub fn enumerate_physical_device_surface_present_modes(
    physical_device: vk::VkPhysicalDevice,
    surface: vk::VkSurfaceKHR,
) -> Result<Vec<vk::VkPresentModeKHR>> {
    let mut mode_count: u32 = 0;
    vk_call!(vk::vkGetPhysicalDeviceSurfacePresentModesKHR(
        physical_device,
        surface,
        &mut mode_count as *mut u32,
        ptr::null_mut(),
    ))?;

    let mut modes: Vec<vk::VkPresentModeKHR> = Vec::with_capacity(mode_count as usize);
    vk_call!(vk::vkGetPhysicalDeviceSurfacePresentModesKHR(
        physical_device,
        surface,
        &mut mode_count as *mut u32,
        modes.as_mut_ptr(),
    ))?;

    unsafe {
        modes.set_len(mode_count as usize);
    }

    Ok(modes)
}

pub fn enumerate_swapchain_images(
    device: vk::VkDevice,
    swapchain: vk::VkSwapchainKHR,
) -> Result<Vec<vk::VkImage>> {
    let mut image_count: u32 = 0;
    vk_call!(vk::vkGetSwapchainImagesKHR(
        device,
        swapchain,
        &mut image_count as *mut u32,
        ptr::null_mut(),
    ))?;

    let mut images: Vec<vk::VkImage> = Vec::with_capacity(image_count as usize);
    vk_call!(vk::vkGetSwapchainImagesKHR(
        device,
        swapchain,
        &mut image_count as *mut u32,
        images.as_mut_ptr(),
    ))?;

    unsafe {
        images.set_len(image_count as usize);
    }

    Ok(images)
}

pub fn enumerate_physical_device_extensions(
    physical_device: vk::VkPhysicalDevice,
) -> Result<Vec<std::ffi::CString>> {
    let mut property_count: u32 = 0;
    vk_call!(vk::vkEnumerateDeviceExtensionProperties(
        physical_device,
        ptr::null(),
        &mut property_count as *mut u32,
        ptr::null_mut(),
    ))?;

    let mut properties: Vec<vk::VkExtensionProperties> =
        Vec::with_capacity(property_count as usize);
    vk_call!(vk::vkEnumerateDeviceExtensionProperties(
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
    physical_device: vk::VkPhysicalDevice,
) -> Vec<vk::VkQueueFamilyProperties> {
    let mut queue_family_count: u32 = 0;
    unsafe {
        vk::vkGetPhysicalDeviceQueueFamilyProperties(
            physical_device,
            &mut queue_family_count as *mut u32,
            ptr::null_mut(),
        );
    }

    let mut properties: Vec<vk::VkQueueFamilyProperties> =
        Vec::with_capacity(queue_family_count as usize);
    unsafe {
        vk::vkGetPhysicalDeviceQueueFamilyProperties(
            physical_device,
            &mut queue_family_count as *mut u32,
            properties.as_mut_ptr(),
        );
        properties.set_len(queue_family_count as usize);
    }
    properties
}

/// Enumerates physical devices (GPUs) available on the system.
pub fn enumerate_physical_devices(instance: &Instance) -> Result<Vec<PhysicalDevice>> {
    let mut device_count: u32 = 0;
    vk_call!(vk::vkEnumeratePhysicalDevices(
        instance.handle,
        &mut device_count as *mut u32,
        ptr::null_mut(),
    ))?;

    let mut devices: Vec<vk::VkPhysicalDevice> = Vec::with_capacity(device_count as usize);
    vk_call!(vk::vkEnumeratePhysicalDevices(
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
                vk::VkPhysicalDeviceIDProperties {
                    sType: vk::VkStructureType::PHYSICAL_DEVICE_ID_PROPERTIES as u32,
                    ..std::mem::zeroed()
                }
            };

            let mut properties = unsafe {
                vk::VkPhysicalDeviceProperties2 {
                    sType: vk::VkStructureType::PHYSICAL_DEVICE_PROPERTIES_2 as u32,
                    pNext: vk_next!(mut id_properties),
                    ..std::mem::zeroed()
                }
            };
            unsafe {
                vk::vkGetPhysicalDeviceProperties2(
                    device_handle,
                    &mut properties as *mut vk::VkPhysicalDeviceProperties2,
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
    callback_data: *const vk::VkDebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::ffi::c_void,
) -> u32 {
    unsafe {
        if !callback_data.is_null() {
            let data = &*callback_data;
            if !data.pMessage.is_null() {
                error!(
                    "{}",
                    std::ffi::CStr::from_ptr(data.pMessage).to_string_lossy()
                );
            }
        }
        vk::VK_FALSE
    }
}

pub fn create_device(
    parameters: &super::StartupParameters,
    physical_device: vk::VkPhysicalDevice,
) -> Result<Device> {
    let available_device_extensions: collections::HashSet<std::ffi::CString> =
        enumerate_physical_device_extensions(physical_device)?
            .into_iter()
            .collect();

    let mut enabled_extensions_cstrings: Vec<std::ffi::CString> = Vec::new();
    enabled_extensions_cstrings.push(std::ffi::CString::new("VK_KHR_swapchain").unwrap());

    if parameters.host_platform == Platform::Windows {
        let external_memory_win32_name = c"VK_KHR_external_memory_win32";
        let keyed_mutex_win32_name = c"VK_KHR_win32_keyed_mutex";
        if available_device_extensions.contains(external_memory_win32_name) {
            warning!("Enabling VK_KHR_external_memory_win32");
            enabled_extensions_cstrings
                .push(std::ffi::CString::new(external_memory_win32_name.to_owned()).unwrap());
        }
        if available_device_extensions.contains(keyed_mutex_win32_name) {
            warning!("Enabling VK_KHR_win32_keyed_mutex");
            enabled_extensions_cstrings
                .push(std::ffi::CString::new(keyed_mutex_win32_name.to_owned()).unwrap());
        }
    }

    let _enabled_extensions_ptrs: Vec<*const std::ffi::c_char> = enabled_extensions_cstrings
        .iter()
        .map(|cs| cs.as_ptr())
        .collect();

    let physical_device_features = unsafe {
        let mut physical_device_synchronization2 = vk::VkPhysicalDeviceSynchronization2Features {
            sType: vk::VkStructureType::PHYSICAL_DEVICE_SYNCHRONIZATION_2_FEATURES as u32,
            ..std::mem::zeroed()
        };
        let mut physical_device_dynamic_rendering = vk::VkPhysicalDeviceDynamicRenderingFeatures {
            sType: vk::VkStructureType::PHYSICAL_DEVICE_DYNAMIC_RENDERING_FEATURES as u32,
            pNext: vk_next!(mut physical_device_synchronization2),
            ..std::mem::zeroed()
        };
        let mut physical_device_features = vk::VkPhysicalDeviceFeatures2 {
            sType: vk::VkStructureType::PHYSICAL_DEVICE_FEATURES_2 as u32,
            pNext: vk_next!(mut physical_device_dynamic_rendering),
            ..std::mem::zeroed()
        };

        vk::vkGetPhysicalDeviceFeatures2(physical_device, &mut physical_device_features);

        // Ensure synchronization2 support
        if physical_device_synchronization2.synchronization2 == vk::VK_FALSE {
            return Err(Status::NotSupported(-1));
        }
        // Ensure dynamic rendering support
        if physical_device_dynamic_rendering.dynamicRendering == vk::VK_FALSE {
            return Err(Status::NotSupported(-1));
        }

        physical_device_features
    };

    let queue_family_properties = enumerate_physical_device_queue_families(physical_device);
    let general_queue_family_index = queue_family_properties
        .iter()
        .position(|qf| (qf.queueFlags & vk::VkQueueFlags::GRAPHICS_BIT as u32) != 0)
        .map(|idx| idx as u32)
        .ok_or(Status::NotSupported(-1))?;
    let transfer_queue_family_index = queue_family_properties
        .iter()
        .position(|qf| {
            (qf.queueFlags & vk::VkQueueFlags::TRANSFER_BIT as u32) != 0
                && (qf.queueFlags & vk::VkQueueFlags::GRAPHICS_BIT as u32) == 0
        })
        .map(|idx| idx as u32)
        .unwrap_or(general_queue_family_index);

    let queue_priority: f32 = 0.5;
    let queue_create_infos: Vec<vk::VkDeviceQueueCreateInfo> = {
        let mut infos: Vec<vk::VkDeviceQueueCreateInfo> = Vec::new();

        let general_queue_info = vk::VkDeviceQueueCreateInfo {
            sType: vk::VkStructureType::DEVICE_QUEUE_CREATE_INFO as u32,
            pNext: ptr::null(),
            flags: 0,
            queueFamilyIndex: general_queue_family_index,
            queueCount: 1,
            pQueuePriorities: &queue_priority as *const f32,
        };
        infos.push(general_queue_info);

        if transfer_queue_family_index != general_queue_family_index {
            let transfer_queue_info = vk::VkDeviceQueueCreateInfo {
                sType: vk::VkStructureType::DEVICE_QUEUE_CREATE_INFO as u32,
                pNext: ptr::null(),
                flags: 0,
                queueFamilyIndex: transfer_queue_family_index,
                queueCount: 1,
                pQueuePriorities: &queue_priority as *const f32,
            };
            infos.push(transfer_queue_info);
        }

        infos
    };

    let device_create_info = vk::VkDeviceCreateInfo {
        sType: vk::VkStructureType::DEVICE_CREATE_INFO as u32,
        pNext: vk_next!(physical_device_features),
        flags: 0,
        queueCreateInfoCount: queue_create_infos.len() as u32,
        pQueueCreateInfos: queue_create_infos.as_ptr(),
        enabledLayerCount: 0,
        ppEnabledLayerNames: ptr::null(),
        enabledExtensionCount: _enabled_extensions_ptrs.len() as u32,
        ppEnabledExtensionNames: _enabled_extensions_ptrs.as_ptr(),
        pEnabledFeatures: ptr::null(),
    };

    let mut device_handle: vk::VkDevice = ptr::null_mut();
    vk_call!(vk::vkCreateDevice(
        physical_device,
        &device_create_info,
        ptr::null(),
        &mut device_handle,
    ))?;

    let general_queue_handle = unsafe {
        let mut queue_handle: vk::VkQueue = ptr::null_mut();
        vk::vkGetDeviceQueue(
            device_handle,
            general_queue_family_index,
            0,
            &mut queue_handle,
        );
        Arc::new(Mutex::new(queue_handle))
    };
    let transfer_queue_handle = if transfer_queue_family_index != general_queue_family_index {
        unsafe {
            let mut queue_handle: vk::VkQueue = ptr::null_mut();
            vk::vkGetDeviceQueue(
                device_handle,
                transfer_queue_family_index,
                0,
                &mut queue_handle,
            );
            Arc::new(Mutex::new(queue_handle))
        }
    } else {
        Arc::clone(&general_queue_handle)
    };

    Ok(Device {
        handle: device_handle,
        physical_device,
        general_queue_family_index,
        transfer_queue_family_index,
        general_queue_handle,
        transfer_queue_handle,
    })
}

/// Creates a Vulkan instance based on the provided parameters.
pub fn create_instance(parameters: &super::StartupParameters) -> Result<Instance> {
    // 1. Get available instance layers and extensions
    let available_layers: collections::HashSet<std::ffi::CString> =
        enumerate_instance_layers()?.into_iter().collect();
    let available_extensions: collections::HashSet<std::ffi::CString> =
        enumerate_instance_extensions()?.into_iter().collect();

    let mut enabled_layers: Vec<std::ffi::CString> = Vec::new();
    let mut enabled_extensions: Vec<std::ffi::CString> = Vec::new();

    // 2. Add platform-specific surface extensions
    warning!("Enabling VK_KHR_surface");
    enabled_extensions.push(std::ffi::CString::new("VK_KHR_surface").unwrap());

    match parameters.host_platform {
        Platform::Windows => {
            warning!("Enabling VK_KHR_win32_surface");
            enabled_extensions.push(std::ffi::CString::new("VK_KHR_win32_surface").unwrap());
        }
        Platform::X11 => {
            warning!("Enabling VK_KHR_xlib_surface");
            warning!("Enabling VK_KHR_xcb_surface");
            enabled_extensions.push(std::ffi::CString::new("VK_KHR_xlib_surface").unwrap());
            enabled_extensions.push(std::ffi::CString::new("VK_KHR_xcb_surface").unwrap());
        }
        Platform::Wayland => {
            warning!("Enabling VK_KHR_wayland_surface");
            enabled_extensions.push(std::ffi::CString::new("VK_KHR_wayland_surface").unwrap());
        }
        // Error if unsupported platform
        _ => {
            return Err(Status::NotSupported(-1));
        }
    }

    // 3. Add validation and debug utils extension if debugging is requested and available
    let validation_name = c"VK_LAYER_KHRONOS_validation";
    let validation_present = available_layers.contains(validation_name);
    let debug_utils_name = c"VK_EXT_debug_utils";
    let debug_utils_present = available_extensions.contains(debug_utils_name);

    let enable_debugging = parameters.enable_debugging && validation_present && debug_utils_present;
    if enable_debugging {
        warning!("Enabling VK_LAYER_KHRONOS_validation");
        warning!("Enabling VK_EXT_debug_utils");
        enabled_layers.push(validation_name.to_owned());
        enabled_extensions.push(debug_utils_name.to_owned());
    }

    // 4. Create the Vulkan instance
    let app_name_cstring = std::ffi::CString::new(parameters.host_name.as_str()).unwrap();
    let engine_name_cstring = std::ffi::CString::new("Rustine").unwrap();
    let enabled_layers_ptrs: Vec<*const std::ffi::c_char> =
        enabled_layers.iter().map(|cs| cs.as_ptr()).collect();
    let enabled_extensions_ptrs: Vec<*const std::ffi::c_char> =
        enabled_extensions.iter().map(|cs| cs.as_ptr()).collect();

    let app_info = vk::VkApplicationInfo {
        sType: vk::VkStructureType::APPLICATION_INFO as u32,
        pNext: ptr::null(),
        pApplicationName: app_name_cstring.as_ptr(),
        applicationVersion: parameters.host_version.to_vk_version(),
        pEngineName: engine_name_cstring.as_ptr(),
        engineVersion: Version::new(1, 0, 0).to_vk_version(),
        apiVersion: MINIMUM_VULKAN_API_VERSION.to_vk_version(),
    };

    let create_info = vk::VkInstanceCreateInfo {
        sType: vk::VkStructureType::INSTANCE_CREATE_INFO as u32,
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

    let mut instance_handle: vk::VkInstance = ptr::null_mut();
    vk_call!(vk::vkCreateInstance(
        &create_info,
        ptr::null(),
        &mut instance_handle,
    ))?;

    let mut debug_messenger: Option<vk::VkDebugUtilsMessengerEXT> = None;
    if enable_debugging {
        let debug_utils_create_fn_name = c"vkCreateDebugUtilsMessengerEXT";
        let create_debug_fn: vk::PFN_vkCreateDebugUtilsMessengerEXT = unsafe {
            std::mem::transmute(vk::vkGetInstanceProcAddr(
                instance_handle,
                debug_utils_create_fn_name.as_ptr(),
            ))
        };

        if create_debug_fn.is_none() {
            return Err(Status::NotSupported(-1));
        }

        // Create debug messenger info
        let debug_create_info = vk::VkDebugUtilsMessengerCreateInfoEXT {
            sType: vk::VkStructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT as u32,
            pNext: ptr::null(),
            flags: 0,
            messageSeverity: vk::VkDebugUtilsMessageSeverityFlagsEXT::ERROR_BIT_EXT
                | vk::VkDebugUtilsMessageSeverityFlagsEXT::WARNING_BIT_EXT,
            messageType: vk::VkDebugUtilsMessageTypeFlagsEXT::GENERAL_BIT_EXT
                | vk::VkDebugUtilsMessageTypeFlagsEXT::VALIDATION_BIT_EXT,
            pfnUserCallback: Some(vulkan_debug_callback),
            pUserData: ptr::null_mut(),
        };

        let mut debug_messenger_ptr: vk::VkDebugUtilsMessengerEXT = ptr::null_mut();
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
