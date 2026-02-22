#![allow(dead_code)]

use std::{collections, ptr};

use crate::{drop, vk_call, vk_next};
use crate::{gfx::vulkan as vk, gfx::*, version::Version};

fn extend_surface_extensions_for_platform(
    available_extensions: &collections::HashSet<std::ffi::CString>,
    enabled_extensions: &mut Vec<std::ffi::CString>,
    platform: Platform,
) -> bool {
    match platform {
        Platform::Windows => {
            let win32_surface = c"VK_KHR_win32_surface";
            if available_extensions.contains(win32_surface) {
                enabled_extensions.push(win32_surface.to_owned());
                true
            } else {
                false
            }
        }
        Platform::Wayland => {
            let wayland_surface = c"VK_KHR_wayland_surface";
            if available_extensions.contains(wayland_surface) {
                enabled_extensions.push(wayland_surface.to_owned());
                true
            } else {
                false
            }
        }
        Platform::X11 => {
            let xlib_surface = c"VK_KHR_xlib_surface";
            let xcb_surface = c"VK_KHR_xcb_surface";
            let mut any = false;

            if available_extensions.contains(xlib_surface) {
                enabled_extensions.push(xlib_surface.to_owned());
                any = true;
            }
            if available_extensions.contains(xcb_surface) {
                enabled_extensions.push(xcb_surface.to_owned());
                any = true;
            }

            any
        }
        Platform::MacOS => false,
    }
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
                crate::log::Log::global().append(
                    crate::log::Severity::Error,
                    std::ffi::CStr::from_ptr(data.pMessage)
                        .to_str()
                        .unwrap_or("Invalid UTF-8"),
                    "Vulkan",
                );
            }
        }
        vk::VK_FALSE
    }
}

/// Represents a Vulkan instance.
pub struct Instance {
    handle: vk::VkInstance,
    debug_messenger: Option<vk::VkDebugUtilsMessengerEXT>,
    surface_platform_hint: Platform,
}

impl Drop for Instance {
    fn drop(&mut self) {
        drop!("Instance::drop");

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

impl Instance {
    pub fn handle(&self) -> vk::VkInstance {
        self.handle
    }

    pub fn surface_platform_hint(&self) -> Platform {
        self.surface_platform_hint
    }

    pub fn create_wayland_surface(
        &self,
        wl_output: *const std::ffi::c_void,
        wl_surface: *const std::ffi::c_void,
    ) -> vk::VkSurfaceKHR {
        let create_info = vk::VkWaylandSurfaceCreateInfoKHR {
            sType: vk::VkStructureType::WAYLAND_SURFACE_CREATE_INFO_KHR,
            pNext: ptr::null(),
            flags: 0,
            display: wl_output,
            surface: wl_surface,
        };

        let mut surface_handle = vk::VkSurfaceKHR::default();
        unsafe {
            vk_call!(vk::vkCreateWaylandSurfaceKHR(
                self.handle,
                &create_info,
                ptr::null(),
                &mut surface_handle,
            ))
            .unwrap();
        }

        surface_handle
    }

    pub fn destroy_surface(&self, surface: vk::VkSurfaceKHR) {
        unsafe {
            vk::vkDestroySurfaceKHR(self.handle, surface, ptr::null());
        }
    }

    /// Creates a Vulkan instance based on the provided parameters.
    pub(crate) fn new(parameters: &crate::Parameters) -> Result<Instance> {
        // 1. Get available instance layers and extensions
        let available_layers: collections::HashSet<std::ffi::CString> =
            enumerate_instance_layers()?.into_iter().collect();
        let available_extensions: collections::HashSet<std::ffi::CString> =
            enumerate_instance_extensions()?.into_iter().collect();

        let mut enabled_layers: Vec<std::ffi::CString> = Vec::new();
        let mut enabled_extensions: Vec<std::ffi::CString> = Vec::new();

        // 2. Add platform surface extensions (preferred platform first, then fallback)
        let surface_name = c"VK_KHR_surface";
        if !available_extensions.contains(surface_name) {
            return Err(Status::NotSupported(-1));
        }
        enabled_extensions.push(surface_name.to_owned());

        let platform_candidates: Vec<Platform> = match parameters.platform {
            Platform::Wayland => vec![Platform::Wayland, Platform::X11],
            Platform::X11 => vec![Platform::X11, Platform::Wayland],
            Platform::Windows => vec![Platform::Windows],
            Platform::MacOS => vec![Platform::MacOS],
        };

        let mut platform_extension_added = false;
        let mut selected_surface_platform: Option<Platform> = None;
        for platform in platform_candidates {
            let added = extend_surface_extensions_for_platform(
                &available_extensions,
                &mut enabled_extensions,
                platform,
            );
            platform_extension_added |= added;

            if added && selected_surface_platform.is_none() {
                selected_surface_platform = Some(platform);
            }
        }

        if !platform_extension_added {
            return Err(Status::NotSupported(-1));
        }

        // 3. Add validation and debug utils extension if debugging is requested and available
        let validation_name = c"VK_LAYER_KHRONOS_validation";
        let validation_present = available_layers.contains(validation_name);
        let debug_utils_name = c"VK_EXT_debug_utils";
        let debug_utils_present = available_extensions.contains(debug_utils_name);

        let enable_debugging = parameters.debugging && validation_present && debug_utils_present;
        if enable_debugging {
            enabled_layers.push(validation_name.to_owned());
            enabled_extensions.push(debug_utils_name.to_owned());
        }

        // 4. Create the Vulkan instance
        let app_name_cstring = std::ffi::CString::new(parameters.app_name.as_str()).unwrap();
        let engine_name_cstring = std::ffi::CString::new("Rustine").unwrap();
        let enabled_layers_ptrs: Vec<*const std::ffi::c_char> =
            enabled_layers.iter().map(|cs| cs.as_ptr()).collect();
        let enabled_extensions_ptrs: Vec<*const std::ffi::c_char> =
            enabled_extensions.iter().map(|cs| cs.as_ptr()).collect();

        let app_info = vk::VkApplicationInfo {
            sType: vk::VkStructureType::APPLICATION_INFO,
            pNext: ptr::null(),
            pApplicationName: app_name_cstring.as_ptr(),
            applicationVersion: parameters.app_version.to_vk_version(),
            pEngineName: engine_name_cstring.as_ptr(),
            engineVersion: Version::new(1, 0, 0).to_vk_version(),
            apiVersion: MINIMUM_VULKAN_API_VERSION.to_vk_version(),
        };

        let create_info = vk::VkInstanceCreateInfo {
            sType: vk::VkStructureType::INSTANCE_CREATE_INFO,
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

        let mut handle = vk::VkInstance::default();
        unsafe {
            vk_call!(vk::vkCreateInstance(&create_info, ptr::null(), &mut handle))?;
        }

        let mut debug_messenger: Option<vk::VkDebugUtilsMessengerEXT> = None;
        if enable_debugging {
            let debug_utils_create_fn_name = c"vkCreateDebugUtilsMessengerEXT";
            let create_debug_fn: vk::PFN_vkCreateDebugUtilsMessengerEXT = unsafe {
                std::mem::transmute(vk::vkGetInstanceProcAddr(
                    handle,
                    debug_utils_create_fn_name.as_ptr(),
                ))
            };

            if create_debug_fn.is_none() {
                return Err(Status::NotSupported(-1));
            }

            // Create debug messenger info
            let debug_create_info = vk::VkDebugUtilsMessengerCreateInfoEXT {
                sType: vk::VkStructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
                pNext: ptr::null(),
                flags: 0,
                messageSeverity: vk::VkDebugUtilsMessageSeverityFlagsEXT::ERROR_BIT_EXT
                    | vk::VkDebugUtilsMessageSeverityFlagsEXT::WARNING_BIT_EXT,
                messageType: vk::VkDebugUtilsMessageTypeFlagsEXT::GENERAL_BIT_EXT
                    | vk::VkDebugUtilsMessageTypeFlagsEXT::VALIDATION_BIT_EXT,
                pfnUserCallback: Some(vulkan_debug_callback),
                pUserData: ptr::null_mut(),
            };

            let mut debug_messenger_ptr = vk::VkDebugUtilsMessengerEXT::default();
            unsafe {
                vk_call!(create_debug_fn.unwrap()(
                    handle,
                    &debug_create_info,
                    ptr::null(),
                    &mut debug_messenger_ptr,
                ))?;
            }

            debug_messenger = Some(debug_messenger_ptr);
        }

        Ok(Instance {
            handle,
            debug_messenger,
            surface_platform_hint: selected_surface_platform.unwrap(),
        })
    }

    /// Enumerates physical devices (GPUs) available on the system.
    pub fn enumerate_physical_devices(&self) -> Result<Vec<PhysicalDevice>> {
        let mut device_count: u32 = 0;
        let devices = unsafe {
            vk_call!(vk::vkEnumeratePhysicalDevices(
                self.handle,
                &mut device_count as *mut u32,
                ptr::null_mut(),
            ))?;

            let mut devices: Vec<vk::VkPhysicalDevice> = Vec::with_capacity(device_count as usize);
            vk_call!(vk::vkEnumeratePhysicalDevices(
                self.handle,
                &mut device_count as *mut u32,
                devices.as_mut_ptr(),
            ))?;

            devices.set_len(device_count as usize);
            devices
        };

        let physical_devices: Vec<PhysicalDevice> = devices
            .iter()
            .map(|&device_handle| {
                let mut id_properties = unsafe {
                    vk::VkPhysicalDeviceIDProperties {
                        sType: vk::VkStructureType::PHYSICAL_DEVICE_ID_PROPERTIES,
                        ..std::mem::zeroed()
                    }
                };

                let mut properties = unsafe {
                    vk::VkPhysicalDeviceProperties2 {
                        sType: vk::VkStructureType::PHYSICAL_DEVICE_PROPERTIES_2,
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
                    handle: device_handle,
                }
            })
            .collect();

        Ok(physical_devices)
    }
}

/// Queries the highest Vulkan API version supported.
fn enumerate_instance_version() -> Result<Version> {
    let mut api_version: u32 = 0;
    unsafe {
        vk_call!(vk::vkEnumerateInstanceVersion(&mut api_version as *mut u32))?;
    }
    Ok(Version::from_vk_version(api_version))
}

/// Queries the available instance layers.
fn enumerate_instance_layers() -> Result<Vec<std::ffi::CString>> {
    unsafe {
        let mut property_count: u32 = 0;
        vk_call!(vk::vkEnumerateInstanceLayerProperties(
            &mut property_count as *mut u32,
            ptr::null_mut(),
        ))?;

        let mut properties: Vec<vk::VkLayerProperties> =
            Vec::with_capacity(property_count as usize);
        vk_call!(vk::vkEnumerateInstanceLayerProperties(
            &mut property_count as *mut u32,
            properties.as_mut_ptr(),
        ))?;

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
fn enumerate_instance_extensions() -> Result<Vec<std::ffi::CString>> {
    unsafe {
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
