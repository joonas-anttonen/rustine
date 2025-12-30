#![allow(dead_code)]

use std::{collections, ptr};

use crate::{error, warning};
use crate::{gfx::*, version::Version};

use crate::gfx::vulkan_ffi as ffi;

/// Wraps a Vulkan function call and converts the result to `gfx::Result`.
#[macro_export]
macro_rules! vk_call {
    ($expr:expr) => {{
        let res = unsafe { $expr };
        match crate::gfx::Outcome::from_code(res) {
            crate::gfx::Outcome::Success => Ok(()),
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

pub enum SubmitStatus {
    Success,
    Timeout,
    Error(crate::gfx::Outcome),
}

pub struct Queue {
    handle: ffi::VkQueue,
    family_index: u32,
    device: Arc<vulkan::Device>,
}

impl Drop for Queue {
    fn drop(&mut self) {
        warning!("Queue::drop");
    }
}

impl Queue {
    pub fn new(handle: ffi::VkQueue, family_index: u32, device: Arc<vulkan::Device>) -> Arc<Self> {
        Arc::new(Queue {
            handle,
            family_index,
            device,
        })
    }

    pub fn allocate_command_pool(self: &Arc<Self>) -> Arc<CommandPool> {
        let command_pool_create_info = ffi::VkCommandPoolCreateInfo {
            sType: ffi::VkStructureType::COMMAND_POOL_CREATE_INFO as u32,
            pNext: ptr::null(),
            flags: ffi::VkCommandPoolCreateFlags::TRANSIENT_BIT
                | ffi::VkCommandPoolCreateFlags::RESET_COMMAND_BUFFER_BIT,
            queueFamilyIndex: self.family_index,
        };

        let mut command_pool_handle: ffi::VkCommandPool = ptr::null_mut();
        vk_call!(ffi::vkCreateCommandPool(
            self.device.handle(),
            &command_pool_create_info,
            ptr::null(),
            &mut command_pool_handle,
        ))
        .map_err(|err| error!("vkCreateCommandPool {:?}", err))
        .unwrap();

        Arc::new(CommandPool {
            handle: command_pool_handle,
            device: Arc::clone(&self.device),
        })
    }

    pub fn wait_idle(self: &Arc<Self>) {
        vk_call!(ffi::vkQueueWaitIdle(self.handle)).expect("vkQueueWaitIdle");
    }

    pub fn submit(self: &Arc<Self>, commands: &CommandBuffer) {
        let submit_info = vulkan_ffi::VkSubmitInfo {
            sType: vulkan_ffi::VkStructureType::SUBMIT_INFO as u32,
            pNext: std::ptr::null(),
            waitSemaphoreCount: 0,
            pWaitSemaphores: std::ptr::null(),
            pWaitDstStageMask: std::ptr::null(),
            commandBufferCount: 1,
            pCommandBuffers: &commands.handle,
            signalSemaphoreCount: 0,
            pSignalSemaphores: std::ptr::null(),
        };

        vk_call!(vulkan_ffi::vkQueueSubmit(
            self.handle,
            1,
            &submit_info,
            commands.fence,
        ))
        .expect("vkQueueSubmit failures should be handled");
    }

    pub fn submit_with_keyed_mutex(
        self: &Arc<Self>,
        commands: &CommandBuffer,
        shared_pixel_buffer: &PixelBuffer,
        acquire_key: u64,
        release_key: u64,
    ) -> SubmitStatus {
        let timeout = 10u32;

        let keyed_mutex_acquire_release_info = vulkan_ffi::VkWin32KeyedMutexAcquireReleaseInfoKHR {
            sType: vulkan_ffi::VkStructureType::WIN32_KEYED_MUTEX_ACQUIRE_RELEASE_INFO_KHR as u32,
            pNext: std::ptr::null(),
            acquireCount: 1,
            pAcquireSyncs: &shared_pixel_buffer.device_memory(),
            pAcquireKeys: &acquire_key,
            pAcquireTimeouts: &timeout,
            releaseCount: 1,
            pReleaseSyncs: &shared_pixel_buffer.device_memory(),
            pReleaseKeys: &release_key,
        };
        let submit_info = vulkan_ffi::VkSubmitInfo {
            sType: vulkan_ffi::VkStructureType::SUBMIT_INFO as u32,
            pNext: &keyed_mutex_acquire_release_info as *const _ as *const _,
            waitSemaphoreCount: 0,
            pWaitSemaphores: std::ptr::null(),
            pWaitDstStageMask: std::ptr::null(),
            commandBufferCount: 1,
            pCommandBuffers: &commands.handle,
            signalSemaphoreCount: 0,
            pSignalSemaphores: std::ptr::null(),
        };
        let result = vk_call!(vulkan_ffi::vkQueueSubmit(
            self.handle,
            1,
            &submit_info,
            commands.fence,
        ));
        self.wait_idle();

        match result {
            Ok(()) => SubmitStatus::Success,
            Err(err) => {
                if let crate::gfx::Outcome::Timeout(_) = err {
                    SubmitStatus::Timeout
                } else {
                    SubmitStatus::Error(err)
                }
            }
        }
    }
}

/// Represents a Vulkan logical device.
pub struct Device {
    general_queue_family_index: u32,
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

    pub fn create_general_queue(self: &Arc<Self>) -> Arc<Queue> {
        let mut queue_handle: ffi::VkQueue = ptr::null_mut();
        unsafe {
            ffi::vkGetDeviceQueue(
                self.handle,
                self.general_queue_family_index,
                0,
                &mut queue_handle,
            );
        }

        Queue::new(
            queue_handle,
            self.general_queue_family_index,
            Arc::clone(self),
        )
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
pub fn enumerate_instance_version() -> Result<Version> {
    let mut api_version: u32 = 0;
    vk_call!(ffi::vkEnumerateInstanceVersion(
        &mut api_version as *mut u32
    ))?;
    Ok(Version::from_vk_version(api_version))
}

/// Queries the available instance layers.
pub fn enumerate_instance_layers() -> Result<Vec<std::ffi::CString>> {
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
pub fn enumerate_instance_extensions() -> Result<Vec<std::ffi::CString>> {
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
) -> Result<Vec<std::ffi::CString>> {
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
pub fn enumerate_physical_devices(instance: &Instance) -> Result<Vec<PhysicalDevice>> {
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
        ffi::VK_FALSE
    }
}

pub fn create_device(
    parameters: &super::StartupParameters,
    physical_device: ffi::VkPhysicalDevice,
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

    let mut physical_device_features = unsafe {
        let mut physical_device_synchronization2 = ffi::VkPhysicalDeviceSynchronization2Features {
            sType: ffi::VkStructureType::PHYSICAL_DEVICE_SYNCHRONIZATION_2_FEATURES as u32,
            ..std::mem::zeroed()
        };
        let mut physical_device_dynamic_rendering = ffi::VkPhysicalDeviceDynamicRenderingFeatures {
            sType: ffi::VkStructureType::PHYSICAL_DEVICE_DYNAMIC_RENDERING_FEATURES as u32,
            pNext: &mut physical_device_synchronization2
                as *mut ffi::VkPhysicalDeviceSynchronization2Features
                as *mut std::ffi::c_void,
            ..std::mem::zeroed()
        };
        let mut physical_device_features = ffi::VkPhysicalDeviceFeatures2 {
            sType: ffi::VkStructureType::PHYSICAL_DEVICE_FEATURES_2 as u32,
            pNext: &mut physical_device_dynamic_rendering
                as *mut ffi::VkPhysicalDeviceDynamicRenderingFeatures
                as *mut std::ffi::c_void,
            ..std::mem::zeroed()
        };

        ffi::vkGetPhysicalDeviceFeatures2(physical_device, &mut physical_device_features);

        // Ensure synchronization2 support
        if physical_device_synchronization2.synchronization2 == ffi::VK_FALSE {
            return Err(Outcome::NotSupported(-1));
        }
        // Ensure dynamic rendering support
        if physical_device_dynamic_rendering.dynamicRendering == ffi::VK_FALSE {
            return Err(Outcome::NotSupported(-1));
        }

        physical_device_features
    };

    let queue_family_properties = enumerate_physical_device_queue_families(physical_device);
    let general_queue_family_index = queue_family_properties
        .iter()
        .position(|qf| (qf.queueFlags & ffi::VkQueueFlags::GRAPHICS_BIT as u32) != 0)
        .map(|idx| idx as u32)
        .ok_or(Outcome::NotSupported(-1))?;
    let queue_priority: f32 = 1.0;
    let queue_create_info = ffi::VkDeviceQueueCreateInfo {
        sType: ffi::VkStructureType::DEVICE_QUEUE_CREATE_INFO as u32,
        pNext: ptr::null(),
        flags: 0,
        queueFamilyIndex: general_queue_family_index,
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

    Ok(Device {
        handle: device_handle,
        physical_device,
        general_queue_family_index,
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
            return Err(Outcome::NotSupported(-1));
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

    let app_info = ffi::VkApplicationInfo {
        sType: ffi::VkStructureType::APPLICATION_INFO as u32,
        pNext: ptr::null(),
        pApplicationName: app_name_cstring.as_ptr(),
        applicationVersion: parameters.host_version.to_vk_version(),
        pEngineName: engine_name_cstring.as_ptr(),
        engineVersion: Version::new(1, 0, 0).to_vk_version(),
        apiVersion: MINIMUM_VULKAN_API_VERSION.to_vk_version(),
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
            return Err(Outcome::NotSupported(-1));
        }

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
