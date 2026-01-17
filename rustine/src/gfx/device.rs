#![allow(dead_code)]

use std::{collections, ptr};

use crate::{gfx::vulkan as vk, gfx::*, version::Version};
use crate::{vk_call, vk_next, warning};

/// Represents the type of a physical graphics device.
#[derive(Debug)]
pub enum PhysicalDeviceType {
    Discrete,
    Integrated,
    Virtual,
    Cpu,
    Other,
}

/// Represents a physical graphics device (GPU) in the system.
#[derive(Debug)]
pub struct PhysicalDevice {
    pub name: String,
    pub driver: Version,
    pub api: Version,
    pub device_type: PhysicalDeviceType,
    pub id: u128,
    pub luid: u64,
    pub handle: vk::VkPhysicalDevice,
}

pub struct DeviceProperties {
    uniform_alignment: u64,
}

impl std::fmt::Display for PhysicalDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (API: {}, Driver: {}, Type: {:?}, Id: {:?})",
            self.name, self.api, self.driver, self.device_type, self.id
        )
    }
}

impl PhysicalDevice {
    pub fn handle(&self) -> vk::VkPhysicalDevice {
        self.handle
    }

    pub fn get_surface_capabilities(
        &self,
        surface_handle: vk::VkSurfaceKHR,
    ) -> Result<vk::VkSurfaceCapabilitiesKHR> {
        let mut surface_capabilities: vk::VkSurfaceCapabilitiesKHR = unsafe { std::mem::zeroed() };
        unsafe {
            vk_call!(vk::vkGetPhysicalDeviceSurfaceCapabilitiesKHR(
                self.handle(),
                surface_handle,
                &mut surface_capabilities
            ))?;
        }

        Ok(surface_capabilities)
    }

    pub fn get_surface_formats(
        &self,
        surface: vk::VkSurfaceKHR,
    ) -> Result<Vec<vk::VkSurfaceFormatKHR>> {
        let mut format_count: u32 = 0;
        unsafe {
            vk_call!(vk::vkGetPhysicalDeviceSurfaceFormatsKHR(
                self.handle,
                surface,
                &mut format_count as *mut u32,
                ptr::null_mut(),
            ))?;
        }

        let mut formats: Vec<vk::VkSurfaceFormatKHR> = Vec::with_capacity(format_count as usize);
        unsafe {
            vk_call!(vk::vkGetPhysicalDeviceSurfaceFormatsKHR(
                self.handle,
                surface,
                &mut format_count as *mut u32,
                formats.as_mut_ptr(),
            ))?;

            formats.set_len(format_count as usize);
        }

        Ok(formats)
    }

    pub fn get_surface_present_modes(
        &self,
        surface: vk::VkSurfaceKHR,
    ) -> Result<Vec<vk::VkPresentModeKHR>> {
        let mut mode_count: u32 = 0;
        unsafe {
            vk_call!(vk::vkGetPhysicalDeviceSurfacePresentModesKHR(
                self.handle,
                surface,
                &mut mode_count as *mut u32,
                ptr::null_mut(),
            ))?;
        }

        let mut modes: Vec<vk::VkPresentModeKHR> = Vec::with_capacity(mode_count as usize);
        unsafe {
            vk_call!(vk::vkGetPhysicalDeviceSurfacePresentModesKHR(
                self.handle,
                surface,
                &mut mode_count as *mut u32,
                modes.as_mut_ptr(),
            ))?;

            modes.set_len(mode_count as usize);
        }

        Ok(modes)
    }

    pub fn get_extensions(&self) -> Result<Vec<std::ffi::CString>> {
        let mut property_count: u32 = 0;
        unsafe {
            vk_call!(vk::vkEnumerateDeviceExtensionProperties(
                self.handle,
                ptr::null(),
                &mut property_count as *mut u32,
                ptr::null_mut(),
            ))?;
        }

        let mut properties: Vec<vk::VkExtensionProperties> =
            Vec::with_capacity(property_count as usize);
        unsafe {
            vk_call!(vk::vkEnumerateDeviceExtensionProperties(
                self.handle,
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

    pub fn get_queue_families(&self) -> Vec<vk::VkQueueFamilyProperties> {
        let mut queue_family_count: u32 = 0;
        unsafe {
            vk::vkGetPhysicalDeviceQueueFamilyProperties(
                self.handle,
                &mut queue_family_count as *mut u32,
                ptr::null_mut(),
            );
        }

        let mut properties: Vec<vk::VkQueueFamilyProperties> =
            Vec::with_capacity(queue_family_count as usize);
        unsafe {
            vk::vkGetPhysicalDeviceQueueFamilyProperties(
                self.handle,
                &mut queue_family_count as *mut u32,
                properties.as_mut_ptr(),
            );
            properties.set_len(queue_family_count as usize);
        }
        properties
    }
}

/// Represents a Vulkan logical device.
pub struct Device {
    device_properties: DeviceProperties,
    general_queue_family_index: u32,
    general_queue_handle: vk::VkQueue,
    handle: vk::VkDevice,
    physical_device: PhysicalDevice,
}

impl Drop for Device {
    fn drop(&mut self) {
        warning!("Device::drop");
        unsafe {
            vk::vkDestroyDevice(self.handle, ptr::null());
        }
    }
}

impl Device {
    pub fn handle(&self) -> vk::VkDevice {
        self.handle
    }

    pub fn physical_device(&self) -> &PhysicalDevice {
        &self.physical_device
    }

    pub fn general_queue_family_index(&self) -> u32 {
        self.general_queue_family_index
    }

    pub fn general_queue(&self) -> &vk::VkQueue {
        &self.general_queue_handle
    }

    pub(crate) fn new(
        parameters: &crate::Parameters,
        physical_device: PhysicalDevice,
    ) -> Result<Device> {
        let available_device_extensions: collections::HashSet<std::ffi::CString> =
            physical_device.get_extensions()?.into_iter().collect();

        let mut enabled_extensions_cstrings: Vec<std::ffi::CString> = Vec::new();
        enabled_extensions_cstrings.push(std::ffi::CString::new("VK_KHR_swapchain").unwrap());

        if parameters.platform == Platform::Windows {
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

        let device_properties = {
            let mut physical_device_properties = unsafe {
                vk::VkPhysicalDeviceProperties2 {
                    sType: vk::VkStructureType::PHYSICAL_DEVICE_PROPERTIES_2,
                    ..std::mem::zeroed()
                }
            };

            unsafe {
                vk::vkGetPhysicalDeviceProperties2(
                    physical_device.handle(),
                    &mut physical_device_properties,
                )
            }

            DeviceProperties {
                uniform_alignment: physical_device_properties
                    .properties
                    .limits
                    .minUniformBufferOffsetAlignment,
            }
        };

        let mut physical_device_vk14_features = unsafe {
            vk::VkPhysicalDeviceVulkan14Features {
                sType: vk::VkStructureType::VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_VULKAN_1_4_FEATURES,
                ..std::mem::zeroed()
            }
        };

        let mut physical_device_synchronization2 = unsafe {
            vk::VkPhysicalDeviceSynchronization2Features {
                sType: vk::VkStructureType::PHYSICAL_DEVICE_SYNCHRONIZATION_2_FEATURES,
                pNext: vk_next!(mut physical_device_vk14_features),
                ..std::mem::zeroed()
            }
        };
        let mut physical_device_dynamic_rendering = unsafe {
            vk::VkPhysicalDeviceDynamicRenderingFeatures {
                sType: vk::VkStructureType::PHYSICAL_DEVICE_DYNAMIC_RENDERING_FEATURES,
                pNext: vk_next!(mut physical_device_synchronization2),
                ..std::mem::zeroed()
            }
        };
        let mut physical_device_features = unsafe {
            vk::VkPhysicalDeviceFeatures2 {
                sType: vk::VkStructureType::PHYSICAL_DEVICE_FEATURES_2,
                pNext: vk_next!(mut physical_device_dynamic_rendering),
                ..std::mem::zeroed()
            }
        };

        unsafe {
            vk::vkGetPhysicalDeviceFeatures2(
                physical_device.handle(),
                &mut physical_device_features,
            );
        }

        // Ensure push descriptor support
        if physical_device_vk14_features.pushDescriptor == vk::VK_FALSE {
            return Err(Status::NotSupported(-1));
        } else {
            // Avoid enabling features we don't use
            physical_device_vk14_features = unsafe {
                vk::VkPhysicalDeviceVulkan14Features {
                    sType:
                        vk::VkStructureType::VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_VULKAN_1_4_FEATURES,
                    ..std::mem::zeroed()
                }
            };
            physical_device_vk14_features.pushDescriptor = vk::VK_TRUE;
        }
        // Ensure synchronization2 support
        if physical_device_synchronization2.synchronization2 == vk::VK_FALSE {
            return Err(Status::NotSupported(-1));
        }
        // Ensure dynamic rendering support
        if physical_device_dynamic_rendering.dynamicRendering == vk::VK_FALSE {
            return Err(Status::NotSupported(-1));
        }

        let queue_family_properties = physical_device.get_queue_families();
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
                sType: vk::VkStructureType::DEVICE_QUEUE_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                queueFamilyIndex: general_queue_family_index,
                queueCount: 1,
                pQueuePriorities: &queue_priority as *const f32,
            };
            infos.push(general_queue_info);

            if transfer_queue_family_index != general_queue_family_index {
                let transfer_queue_info = vk::VkDeviceQueueCreateInfo {
                    sType: vk::VkStructureType::DEVICE_QUEUE_CREATE_INFO,
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
            sType: vk::VkStructureType::DEVICE_CREATE_INFO,
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

        let mut handle = vk::VkDevice::default();
        unsafe {
            vk_call!(vk::vkCreateDevice(
                physical_device.handle(),
                &device_create_info,
                ptr::null(),
                &mut handle,
            ))?;
        }

        let general_queue_handle = unsafe {
            let mut queue_handle = vk::VkQueue::default();
            vk::vkGetDeviceQueue(handle, general_queue_family_index, 0, &mut queue_handle);
            queue_handle
        };

        Ok(Device {
            handle,
            physical_device,
            general_queue_family_index,
            general_queue_handle,
            device_properties,
        })
    }

    pub fn get_swapchain_images(&self, swapchain: vk::VkSwapchainKHR) -> Result<Vec<vk::VkImage>> {
        let mut image_count: u32 = 0;
        unsafe {
            vk_call!(vk::vkGetSwapchainImagesKHR(
                self.handle,
                swapchain,
                &mut image_count as *mut u32,
                ptr::null_mut(),
            ))?;
        }

        let mut images: Vec<vk::VkImage> = Vec::with_capacity(image_count as usize);
        unsafe {
            vk_call!(vk::vkGetSwapchainImagesKHR(
                self.handle,
                swapchain,
                &mut image_count as *mut u32,
                images.as_mut_ptr(),
            ))?;

            images.set_len(image_count as usize);
        }

        Ok(images)
    }
}
