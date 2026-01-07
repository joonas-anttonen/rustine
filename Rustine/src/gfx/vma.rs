#![allow(dead_code)]

use std::sync::Arc;

use crate::gfx::*;
use crate::{vk_call, warning};

pub use ffi::VmaAllocation;
pub use ffi::VmaAllocationInfo;
pub use ffi::vmaDestroyBuffer;
pub use ffi::vmaDestroyImage;

pub struct Allocator {
    pub handle: ffi::VmaAllocator,
    pub device: Arc<Device>,
}

impl Drop for Allocator {
    fn drop(&mut self) {
        warning!("Allocator::drop");
        unsafe {
            ffi::vmaDestroyAllocator(self.handle);
        }
    }
}

impl Allocator {
    pub fn new(instance: &Instance, device: Arc<Device>) -> Result<Arc<Self>> {
        let mut flags = ffi::VmaAllocatorCreateFlags::NONE as u32;
        // If Windows platform, enable external memory handle types
        if cfg!(target_os = "windows") {
            flags = flags | ffi::VmaAllocatorCreateFlags::KHR_EXTERNAL_MEMORY_WIN32_BIT as u32;
        }

        let create_info = ffi::VmaAllocatorCreateInfo {
            flags: flags,
            physicalDevice: device.physical_device().handle(),
            device: device.handle(),
            preferredLargeHeapBlockSize: 0,
            pAllocationCallbacks: std::ptr::null(),
            pDeviceMemoryCallbacks: std::ptr::null(),
            pHeapSizeLimit: std::ptr::null(),
            pVulkanFunctions: std::ptr::null(),
            instance: instance.handle(),
            vulkanApiVersion: MINIMUM_VULKAN_API_VERSION.to_vk_version(),
            pTypeExternalMemoryHandleTypes: std::ptr::null(),
        };

        let mut allocator_handle: ffi::VmaAllocator = std::ptr::null_mut();

        vk_call!(ffi::vmaCreateAllocator(&create_info, &mut allocator_handle))?;

        Ok(Arc::new(Allocator {
            handle: allocator_handle,
            device: Arc::clone(&device),
        }))
    }

    pub fn create_pixel_buffer(
        self: &Arc<Self>,
        format: Format,
        width: u32,
        height: u32,
        usage: ImageUsage,
        aspect: ImageAspect,
        samples: Samples,
    ) -> Result<PixelBuffer> {
        let image_create_info = vulkan::VkImageCreateInfo {
            sType: vulkan::VkStructureType::IMAGE_CREATE_INFO as u32,
            pNext: std::ptr::null(),
            flags: 0,
            imageType: vulkan::VkImageType::X2D,
            format: format.to_vk(),
            extent: vulkan::VkExtent3D {
                width,
                height,
                depth: 1,
            },
            mipLevels: 1,
            arrayLayers: 1,
            samples: samples.0,
            tiling: vulkan::VkImageTiling::OPTIMAL,
            usage: usage.0,
            sharingMode: vulkan::VkSharingMode::EXCLUSIVE,
            queueFamilyIndexCount: 0,
            pQueueFamilyIndices: std::ptr::null(),
            initialLayout: vulkan::VkImageLayout::UNDEFINED,
        };
        let mut image_view_create_info = vulkan::VkImageViewCreateInfo {
            sType: vulkan::VkStructureType::IMAGE_VIEW_CREATE_INFO as u32,
            pNext: std::ptr::null(),
            flags: 0,
            image: std::ptr::null_mut(), // NOTE: image not available yet, will be set in allocate_image
            viewType: vulkan::VkImageViewType::X2D,
            format: format.to_vk(),
            components: vulkan::VkComponentMapping {
                r: vulkan::VkComponentSwizzle::IDENTITY,
                g: vulkan::VkComponentSwizzle::IDENTITY,
                b: vulkan::VkComponentSwizzle::IDENTITY,
                a: vulkan::VkComponentSwizzle::IDENTITY,
            },
            subresourceRange: vulkan::VkImageSubresourceRange {
                aspectMask: aspect.0,
                baseMipLevel: 0,
                levelCount: 1,
                baseArrayLayer: 0,
                layerCount: 1,
            },
        };

        self.allocate_image(&image_create_info, &mut image_view_create_info)
    }

    fn allocate_image(
        self: &Arc<Self>,
        image_create_info: &vulkan::VkImageCreateInfo,
        image_view_create_info: &mut vulkan::VkImageViewCreateInfo,
    ) -> Result<PixelBuffer> {
        let mut image: vulkan::VkImage = std::ptr::null_mut();
        let mut image_view: vulkan::VkImageView = std::ptr::null_mut();
        let mut allocation: ffi::VmaAllocation = std::ptr::null_mut();
        let mut allocation_info: ffi::VmaAllocationInfo = unsafe { std::mem::zeroed() };

        let allocation_create_info = ffi::VmaAllocationCreateInfo {
            flags: 0,
            usage: ffi::VmaMemoryUsage::AUTO,
            requiredFlags: 0,
            preferredFlags: 0,
            memoryTypeBits: 0,
            pool: std::ptr::null_mut(),
            pUserData: std::ptr::null_mut(),
            priority: 0.0,
        };

        vk_call!(ffi::vmaCreateImage(
            self.handle,
            image_create_info,
            &allocation_create_info,
            &mut image,
            &mut allocation,
            &mut allocation_info
        ))?;

        image_view_create_info.image = image; // Set the image now that it's created

        vk_call!(vulkan::vkCreateImageView(
            self.device.handle(),
            image_view_create_info,
            std::ptr::null(),
            &mut image_view
        ))
        .or_else(|err| {
            // If creating the image view fails, clean up the previously
            // created VMA image and allocation to avoid leaking resources.
            unsafe {
                ffi::vmaDestroyImage(self.handle, image, allocation);
            }
            Err(err)
        })?;

        Ok(PixelBuffer::new(
            image_create_info.extent.width,
            image_create_info.extent.height,
            image,
            image_view,
            allocation,
            allocation_info,
            Arc::clone(&self),
        ))
    }

    fn allocate_external_image(
        self: &Arc<Self>,
        handle: *const std::ffi::c_void,
        image_create_info: &mut vulkan::VkImageCreateInfo,
        image_view_create_info: &mut vulkan::VkImageViewCreateInfo,
    ) -> Result<PixelBuffer> {
        let external_image_create_info = vulkan::VkExternalMemoryImageCreateInfo {
            sType: vulkan::VkStructureType::EXTERNAL_MEMORY_IMAGE_CREATE_INFO as u32,
            pNext: std::ptr::null(),
            handleTypes: vulkan::VkExternalMemoryHandleTypeFlags::D3D11_TEXTURE_BIT as u32,
        };
        image_create_info.pNext = &external_image_create_info
            as *const vulkan::VkExternalMemoryImageCreateInfo
            as *const std::ffi::c_void;

        let import_memory_win32_info = vulkan::VkImportMemoryWin32HandleInfoKHR {
            sType: vulkan::VkStructureType::IMPORT_MEMORY_WIN32_HANDLE_INFO_KHR as u32,
            pNext: std::ptr::null(),
            handleType: vulkan::VkExternalMemoryHandleTypeFlags::D3D11_TEXTURE_BIT as u32,
            handle: handle as *mut std::ffi::c_void,
            name: std::ptr::null(),
        };

        let mut image: vulkan::VkImage = std::ptr::null_mut();
        let mut image_view: vulkan::VkImageView = std::ptr::null_mut();
        let mut allocation: ffi::VmaAllocation = std::ptr::null_mut();
        let mut allocation_info: ffi::VmaAllocationInfo = unsafe { std::mem::zeroed() };

        let allocation_create_info = ffi::VmaAllocationCreateInfo {
            flags: 0,
            usage: ffi::VmaMemoryUsage::AUTO,
            requiredFlags: 0,
            preferredFlags: 0,
            memoryTypeBits: 0,
            pool: std::ptr::null_mut(),
            pUserData: std::ptr::null_mut(),
            priority: 0.0,
        };

        vk_call!(ffi::vmaCreateDedicatedImage(
            self.handle,
            image_create_info,
            &allocation_create_info,
            &import_memory_win32_info as *const vulkan::VkImportMemoryWin32HandleInfoKHR
                as *const std::ffi::c_void,
            &mut image,
            &mut allocation,
            &mut allocation_info
        ))?;

        image_view_create_info.image = image; // Set the image now that it's created

        vk_call!(vulkan::vkCreateImageView(
            self.device.handle(),
            image_view_create_info,
            std::ptr::null(),
            &mut image_view
        ))
        .or_else(|err| {
            // If creating the image view fails, clean up the previously
            // created VMA image and allocation to avoid leaking resources.
            unsafe {
                ffi::vmaDestroyImage(self.handle, image, allocation);
            }
            Err(err)
        })?;

        Ok(PixelBuffer::new(
            image_create_info.extent.width,
            image_create_info.extent.height,
            image,
            image_view,
            allocation,
            allocation_info,
            Arc::clone(&self),
        ))
    }

    pub fn create_external_pixel_buffer(
        self: &Arc<Self>,
        format: Format,
        width: u32,
        height: u32,
        usage: ImageUsage,
        aspect: ImageAspect,
        handle: *const std::ffi::c_void,
    ) -> Result<PixelBuffer> {
        let mut image_create_info = vulkan::VkImageCreateInfo {
            sType: vulkan::VkStructureType::IMAGE_CREATE_INFO as u32,
            pNext: std::ptr::null(),
            flags: 0,
            imageType: vulkan::VkImageType::X2D,
            format: format.to_vk(),
            extent: vulkan::VkExtent3D {
                width,
                height,
                depth: 1,
            },
            mipLevels: 1,
            arrayLayers: 1,
            samples: Samples::X1.0,
            tiling: vulkan::VkImageTiling::OPTIMAL,
            usage: usage.0,
            sharingMode: vulkan::VkSharingMode::EXCLUSIVE,
            queueFamilyIndexCount: 0,
            pQueueFamilyIndices: std::ptr::null(),
            initialLayout: vulkan::VkImageLayout::UNDEFINED,
        };
        let mut image_view_create_info = vulkan::VkImageViewCreateInfo {
            sType: vulkan::VkStructureType::IMAGE_VIEW_CREATE_INFO as u32,
            pNext: std::ptr::null(),
            flags: 0,
            image: std::ptr::null_mut(), // NOTE: image not available yet, will be set in allocate_image
            viewType: vulkan::VkImageViewType::X2D,
            format: format.to_vk(),
            components: vulkan::VkComponentMapping {
                r: vulkan::VkComponentSwizzle::IDENTITY,
                g: vulkan::VkComponentSwizzle::IDENTITY,
                b: vulkan::VkComponentSwizzle::IDENTITY,
                a: vulkan::VkComponentSwizzle::IDENTITY,
            },
            subresourceRange: vulkan::VkImageSubresourceRange {
                aspectMask: aspect.0,
                baseMipLevel: 0,
                levelCount: 1,
                baseArrayLayer: 0,
                layerCount: 1,
            },
        };

        self.allocate_external_image(handle, &mut image_create_info, &mut image_view_create_info)
    }
}

#[allow(
    dead_code,
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals
)]
mod ffi {
    use crate::gfx::vulkan::{
        VkBuffer, VkBufferCreateInfo, VkDevice, VkDeviceMemory, VkDeviceSize, VkImage,
        VkImageCreateInfo, VkInstance, VkPhysicalDevice,
    };

    pub type VmaAllocator = *mut std::ffi::c_void;
    pub type VmaAllocation = *mut std::ffi::c_void;
    pub type VmaPool = *mut std::ffi::c_void;

    #[link(name = "rustine-vma", kind = "static")]
    unsafe extern "C" {

        // ========= Allocator ==========

        pub fn vmaCreateAllocator(
            pCreateInfo: *const VmaAllocatorCreateInfo,
            pAllocator: *mut VmaAllocator,
        ) -> i32;
        pub fn vmaDestroyAllocator(allocator: VmaAllocator);

        // ========= Buffer ==========

        pub fn vmaCreateBuffer(
            allocator: VmaAllocator,
            pBufferCreateInfo: *const VkBufferCreateInfo,
            pAllocationCreateInfo: *const VmaAllocationCreateInfo,
            pBuffer: *mut VkBuffer,
            pAllocation: *mut VmaAllocation,
            pAllocationInfo: *mut VmaAllocationInfo,
        ) -> i32;
        pub fn vmaDestroyBuffer(
            allocator: VmaAllocator,
            buffer: VkBuffer,
            allocation: VmaAllocation,
        );

        // ========= Image ==========

        pub fn vmaCreateImage(
            allocator: VmaAllocator,
            pImageCreateInfo: *const VkImageCreateInfo,
            pAllocationCreateInfo: *const VmaAllocationCreateInfo,
            pImage: *mut VkImage,
            pAllocation: *mut VmaAllocation,
            pAllocationInfo: *mut VmaAllocationInfo,
        ) -> i32;
        pub fn vmaCreateDedicatedImage(
            allocator: VmaAllocator,
            pImageCreateInfo: *const VkImageCreateInfo,
            pAllocationCreateInfo: *const VmaAllocationCreateInfo,
            pMemoryAllocateNext: *const std::ffi::c_void,
            pImage: *mut VkImage,
            pAllocation: *mut VmaAllocation,
            pAllocationInfo: *mut VmaAllocationInfo,
        ) -> i32;
        pub fn vmaDestroyImage(allocator: VmaAllocator, image: VkImage, allocation: VmaAllocation);
    }

    pub type PFN_vmaAllocateDeviceMemoryFunction = unsafe extern "C" fn(
        allocator: VmaAllocator,
        memoryType: u32,
        memory: VkDeviceMemory,
        size: VkDeviceSize,
        pUserData: *mut std::ffi::c_void,
    );
    pub type PFN_vmaFreeDeviceMemoryFunction = unsafe extern "C" fn(
        allocator: VmaAllocator,
        memoryType: u32,
        memory: VkDeviceMemory,
        size: VkDeviceSize,
        pUserData: *mut std::ffi::c_void,
    );

    /// Set of callbacks that the library will call for `vkAllocateMemory` and `vkFreeMemory`.
    ///
    /// Provided for informative purpose, e.g. to gather statistics about number of
    /// allocations or total amount of memory allocated in Vulkan.
    ///
    /// Used in `VmaAllocatorCreateInfo::pDeviceMemoryCallbacks`.
    #[repr(C)]
    pub struct VmaDeviceMemoryCallbacks {
        /// Optional, can be null.
        pub pfnAllocate: Option<PFN_vmaAllocateDeviceMemoryFunction>,
        /// Optional, can be null.
        pub pfnFree: Option<PFN_vmaFreeDeviceMemoryFunction>,
        /// Optional, can be null.
        pub pUserData: *mut std::ffi::c_void,
    }

    /// Parameters of `VmaAllocation` objects, that can be retrieved using function vmaGetAllocationInfo().
    ///
    /// There is also an extended version of this structure that carries additional parameters: `VmaAllocationInfo2`.
    #[repr(C)]
    pub struct VmaAllocationInfo {
        /// Memory type index that this allocation was allocated from.
        /// It never changes.
        pub memoryType: u32,
        /// Handle to Vulkan memory object.
        ///
        /// Same memory object can be shared by multiple allocations.
        /// It can change after the allocation is moved during defragmentation.
        pub deviceMemory: VkDeviceMemory,
        /// Offset in `VkDeviceMemory` object to the beginning of this allocation, in bytes.
        /// `(deviceMemory, offset)` pair is unique to this allocation.
        ///
        /// You usually don't need to use this offset. If you create a buffer or an image together with the allocation
        /// using e.g. function vmaCreateBuffer(), vmaCreateImage(), functions that operate on these resources refer to
        /// the beginning of the buffer or image, not entire device memory block. Functions like vmaMapMemory(),
        /// vmaBindBufferMemory() also refer to the beginning of the allocation and apply this offset automatically.
        ///
        /// It can change after the allocation is moved during defragmentation.
        pub offset: VkDeviceSize,
        /// Size of this allocation, in bytes. It never changes.
        ///
        /// **Note:** Allocation size returned in this variable may be greater than the size
        /// requested for the resource e.g. as `VkBufferCreateInfo::size`. Whole size of the
        /// allocation is accessible for operations on memory e.g. using a pointer after
        /// mapping with vmaMapMemory(), but operations on the resource e.g. using
        /// `vkCmdCopyBuffer` must be limited to the size of the resource.
        pub size: VkDeviceSize,
        /// Pointer to the beginning of this allocation as mapped data.
        ///
        /// If the allocation hasn't been mapped using vmaMapMemory() and hasn't been
        /// created with `VMA_ALLOCATION_CREATE_MAPPED_BIT` flag, this value is null.
        /// It can change after call to vmaMapMemory(), vmaUnmapMemory().
        /// It can also change after the allocation is moved during defragmentation.
        pub pMappedData: *mut std::ffi::c_void,
        /// Custom general-purpose pointer that was passed as VmaAllocationCreateInfo::pUserData
        /// or set using vmaSetAllocationUserData().
        ///
        /// It can change after call to vmaSetAllocationUserData() for this allocation.
        pub pUserData: *mut std::ffi::c_void,
        /// Custom allocation name that was set with vmaSetAllocationName().
        ///
        /// It can change after call to vmaSetAllocationName() for this allocation.
        /// Another way to set custom name is to pass it in VmaAllocationCreateInfo::pUserData with
        /// additional flag `VMA_ALLOCATION_CREATE_USER_DATA_COPY_STRING_BIT` set [DEPRECATED].
        pub pName: *const std::ffi::c_char,
    }

    /// Parameters of new `VmaAllocation`.
    ///
    /// To be used with functions like vmaCreateBuffer(), vmaCreateImage(), and many others.
    #[repr(C)]
    pub struct VmaAllocationCreateInfo {
        /// Flags for created allocation. Use `VmaAllocationCreateFlagBits` enum.
        pub flags: u32,
        /// Intended usage of memory.
        ///
        /// You can leave `VMA_MEMORY_USAGE_UNKNOWN` if you specify memory requirements in other way.
        /// If `pool` is not null, this member is ignored.
        pub usage: VmaMemoryUsage,
        /// Flags that must be set in a memory type chosen for an allocation.
        ///
        /// Leave 0 if you specify memory requirements in other way.
        /// If `pool` is not null, this member is ignored.
        pub requiredFlags: u32,
        /// Flags that preferably should be set in a memory type chosen for an allocation.
        ///
        /// Set to 0 if no additional flags are preferred.
        /// If `pool` is not null, this member is ignored.
        pub preferredFlags: u32,
        /// Bitmask containing one bit set for every memory type acceptable for this allocation.
        ///
        /// Value 0 is equivalent to `UINT32_MAX` - it means any memory type is accepted if
        /// it meets other requirements specified by this structure, with no further
        /// restrictions on memory type index.
        /// If `pool` is not null, this member is ignored.
        pub memoryTypeBits: u32,
        /// Pool that this allocation should be created in.
        ///
        /// Leave null to allocate from default pool. If not null, members:
        /// `usage`, `requiredFlags`, `preferredFlags`, `memoryTypeBits` are ignored.
        pub pool: VmaPool,
        /// Custom general-purpose pointer that will be stored in `VmaAllocation`, can be read as VmaAllocationInfo::pUserData and changed using vmaSetAllocationUserData().
        ///
        /// If `VMA_ALLOCATION_CREATE_USER_DATA_COPY_STRING_BIT` is used, it must be either
        /// null or pointer to a null-terminated string. The string will be then copied to
        /// internal buffer, so it doesn't need to be valid after allocation call.
        pub pUserData: *mut std::ffi::c_void,
        /// A floating-point value between 0 and 1, indicating the priority of the allocation relative to other memory allocations.
        ///
        /// It is used only when `VMA_ALLOCATOR_CREATE_EXT_MEMORY_PRIORITY_BIT` flag was used during creation of the `VmaAllocator` object
        /// and this allocation ends up as dedicated or is explicitly forced as dedicated using `VMA_ALLOCATION_CREATE_DEDICATED_MEMORY_BIT`.
        /// Otherwise, it has the priority of a memory block where it is placed and this variable is ignored.
        pub priority: f32,
    }

    /// Description of an allocator to be created.
    #[repr(C)]
    pub struct VmaAllocatorCreateInfo {
        /// Flags for created allocator. Use `VmaAllocatorCreateFlagBits` enum.
        pub flags: u32,
        /// Vulkan physical device.
        /// Must be valid throughout the whole lifetime of the created allocator.
        pub physicalDevice: VkPhysicalDevice,
        /// Vulkan device.
        /// Must be valid throughout the whole lifetime of the created allocator.
        pub device: VkDevice,
        /// Preferred size of a single `VkDeviceMemory` block to be allocated from large heaps > 1 GiB. Optional.
        /// Set to 0 to use default, which is currently 256 MiB.
        pub preferredLargeHeapBlockSize: VkDeviceSize,
        /// Custom CPU memory allocation callbacks. Optional.
        /// Optional, can be null. When specified, will also be used for all CPU-side memory allocations.
        pub pAllocationCallbacks: *const std::ffi::c_void, // VkAllocationCallbacks*
        /// Informative callbacks for `vkAllocateMemory`, `vkFreeMemory`. Optional.
        /// Optional, can be null.
        pub pDeviceMemoryCallbacks: *const VmaDeviceMemoryCallbacks,
        /// Either null or a pointer to an array of limits on maximum number of bytes that can be allocated out of particular Vulkan memory heap.
        ///
        /// If not NULL, it must be a pointer to an array of
        /// `VkPhysicalDeviceMemoryProperties::memoryHeapCount` elements, defining limit on
        /// maximum number of bytes that can be allocated out of particular Vulkan memory heap.
        ///
        /// Any of the elements may be equal to `VK_WHOLE_SIZE`, which means no limit on that heap.
        /// This is also the default in case of `pHeapSizeLimit` = NULL.
        ///
        /// If there is a limit defined for a heap:
        ///
        /// - If user tries to allocate more memory from that heap using this allocator,
        ///   the allocation fails with `VK_ERROR_OUT_OF_DEVICE_MEMORY`.
        /// - If the limit is smaller than heap size reported in `VkMemoryHeap::size`, the
        ///   value of this limit will be reported instead when using vmaGetMemoryProperties().
        ///
        /// **Warning:** Using this feature may not be equivalent to installing a GPU with
        /// smaller amount of memory, because graphics driver doesn't necessarily fail new
        /// allocations with `VK_ERROR_OUT_OF_DEVICE_MEMORY` result when memory capacity is
        /// exceeded. It may return success and just silently migrate some device memory
        /// blocks to system RAM. This driver behavior can also be controlled using
        /// VK_AMD_memory_overallocation_behavior extension.
        pub pHeapSizeLimit: *const VkDeviceSize,

        /// Pointers to Vulkan functions. Can be null.
        /// For details, see the VMA documentation on Vulkan function pointers.
        pub pVulkanFunctions: *const std::ffi::c_void, // VmaVulkanFunctions*
        /// Handle to Vulkan instance object.
        /// Starting from version 3.0.0 this member is no longer optional, it must be set!
        pub instance: VkInstance,
        /// Vulkan version that the application uses. Optional.
        ///
        /// It must be a value in the format as created by macro `VK_MAKE_VERSION` or a constant like: `VK_API_VERSION_1_1`, `VK_API_VERSION_1_0`.
        /// The patch version number specified is ignored. Only the major and minor versions are considered.
        /// Only versions 1.0...1.4 are supported by the current implementation.
        /// Leaving it initialized to zero is equivalent to `VK_API_VERSION_1_0`.
        /// It must match the Vulkan version used by the application and supported on the selected physical device,
        /// so it must be no higher than `VkApplicationInfo::apiVersion` passed to `vkCreateInstance`
        /// and no higher than `VkPhysicalDeviceProperties::apiVersion` found on the physical device used.
        pub vulkanApiVersion: u32,

        /// Either null or a pointer to an array of external memory handle types for each Vulkan memory type.
        ///
        /// If not NULL, it must be a pointer to an array of `VkPhysicalDeviceMemoryProperties::memoryTypeCount`
        /// elements, defining external memory handle types of particular Vulkan memory type,
        /// to be passed using `VkExportMemoryAllocateInfoKHR`.
        ///
        /// Any of the elements may be equal to 0, which means not to use `VkExportMemoryAllocateInfoKHR` on this memory type.
        /// This is also the default in case of `pTypeExternalMemoryHandleTypes` = NULL.
        pub pTypeExternalMemoryHandleTypes: *const u32, // VkExternalMemoryHandleTypeFlagsKHR*
    }

    /// Flags for created `VmaAllocator`.
    #[repr(u32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum VmaAllocatorCreateFlags {
        NONE = 0,
        /// Allocator and all objects created from it will not be synchronized internally, so you must guarantee they are used from only one thread at a time or synchronized externally by you.
        ///
        /// Using this flag may increase performance because internal mutexes are not used.
        EXTERNALLY_SYNCHRONIZED_BIT = 0x00000001,
        /// Enables usage of VK_KHR_dedicated_allocation extension.
        ///
        /// The flag works only if VmaAllocatorCreateInfo::vulkanApiVersion `== VK_API_VERSION_1_0`.
        /// When it is `VK_API_VERSION_1_1`, the flag is ignored because the extension has been promoted to Vulkan 1.1.
        ///
        /// Using this extension will automatically allocate dedicated blocks of memory for
        /// some buffers and images instead of suballocating place for them out of bigger
        /// memory blocks (as if you explicitly used `VMA_ALLOCATION_CREATE_DEDICATED_MEMORY_BIT`
        /// flag) when it is recommended by the driver. It may improve performance on some
        /// GPUs.
        ///
        /// You may set this flag only if you found out that following device extensions are
        /// supported, you enabled them while creating Vulkan device passed as
        /// VmaAllocatorCreateInfo::device, and you want them to be used internally by this
        /// library:
        ///
        /// - VK_KHR_get_memory_requirements2 (device extension)
        /// - VK_KHR_dedicated_allocation (device extension)
        ///
        /// When this flag is set, you can experience following warnings reported by Vulkan
        /// validation layer. You can ignore them.
        ///
        /// > vkBindBufferMemory(): Binding memory to buffer 0x2d but vkGetBufferMemoryRequirements() has not been called on that buffer.
        KHR_DEDICATED_ALLOCATION_BIT = 0x00000002,
        /// Enables usage of VK_KHR_bind_memory2 extension.
        ///
        /// The flag works only if VmaAllocatorCreateInfo::vulkanApiVersion `== VK_API_VERSION_1_0`.
        /// When it is `VK_API_VERSION_1_1`, the flag is ignored because the extension has been promoted to Vulkan 1.1.
        ///
        /// You may set this flag only if you found out that this device extension is supported,
        /// you enabled it while creating Vulkan device passed as VmaAllocatorCreateInfo::device,
        /// and you want it to be used internally by this library.
        ///
        /// The extension provides functions `vkBindBufferMemory2KHR` and `vkBindImageMemory2KHR`,
        /// which allow to pass a chain of `pNext` structures while binding.
        /// This flag is required if you use `pNext` parameter in vmaBindBufferMemory2() or vmaBindImageMemory2().
        KHR_BIND_MEMORY2_BIT = 0x00000004,
        /// Enables usage of VK_EXT_memory_budget extension.
        ///
        /// You may set this flag only if you found out that this device extension is supported,
        /// you enabled it while creating Vulkan device passed as VmaAllocatorCreateInfo::device,
        /// and you want it to be used internally by this library, along with another instance extension
        /// VK_KHR_get_physical_device_properties2, which is required by it (or Vulkan 1.1, where this extension is promoted).
        ///
        /// The extension provides query for current memory usage and budget, which will probably
        /// be more accurate than an estimation used by the library otherwise.
        EXT_MEMORY_BUDGET_BIT = 0x00000008,
        /// Enables usage of VK_AMD_device_coherent_memory extension.
        ///
        /// You may set this flag only if you:
        ///
        /// - found out that this device extension is supported and enabled it while creating Vulkan device passed as VmaAllocatorCreateInfo::device,
        /// - checked that `VkPhysicalDeviceCoherentMemoryFeaturesAMD::deviceCoherentMemory` is true and set it while creating the Vulkan device,
        /// - want it to be used internally by this library.
        ///
        /// The extension and accompanying device feature provide access to memory types with
        /// `VK_MEMORY_PROPERTY_DEVICE_COHERENT_BIT_AMD` and `VK_MEMORY_PROPERTY_DEVICE_UNCACHED_BIT_AMD` flags.
        /// They are useful mostly for writing breadcrumb markers - a common method for debugging GPU crash/hang/TDR.
        ///
        /// When the extension is not enabled, such memory types are still enumerated, but their usage is illegal.
        /// To protect from this error, if you don't create the allocator with this flag, it will refuse to allocate any memory or create a custom pool in such memory type,
        /// returning `VK_ERROR_FEATURE_NOT_PRESENT`.
        AMD_DEVICE_COHERENT_MEMORY_BIT = 0x00000010,
        /// Enables usage of "buffer device address" feature, which allows you to use function
        /// `vkGetBufferDeviceAddress*` to get raw GPU pointer to a buffer and pass it for usage inside a shader.
        ///
        /// You may set this flag only if you:
        ///
        /// 1. (For Vulkan version < 1.2) Found as available and enabled device extension
        /// VK_KHR_buffer_device_address.
        /// This extension is promoted to core Vulkan 1.2.
        /// 2. Found as available and enabled device feature `VkPhysicalDeviceBufferDeviceAddressFeatures::bufferDeviceAddress`.
        ///
        /// When this flag is set, you can create buffers with `VK_BUFFER_USAGE_SHADER_DEVICE_ADDRESS_BIT` using VMA.
        /// The library automatically adds `VK_MEMORY_ALLOCATE_DEVICE_ADDRESS_BIT` to
        /// allocated memory blocks wherever it might be needed.
        ///
        /// For more information, see the VMA documentation on enabling buffer device address.
        BUFFER_DEVICE_ADDRESS_BIT = 0x00000020,
        /// Enables usage of VK_EXT_memory_priority extension in the library.
        ///
        /// You may set this flag only if you found available and enabled this device extension,
        /// along with `VkPhysicalDeviceMemoryPriorityFeaturesEXT::memoryPriority == VK_TRUE`,
        /// while creating Vulkan device passed as VmaAllocatorCreateInfo::device.
        ///
        /// When this flag is used, VmaAllocationCreateInfo::priority and VmaPoolCreateInfo::priority
        /// are used to set priorities of allocated Vulkan memory. Without it, these variables are ignored.
        ///
        /// A priority must be a floating-point value between 0 and 1, indicating the priority of the allocation relative to other memory allocations.
        /// Larger values are higher priority. The granularity of the priorities is implementation-dependent.
        /// It is automatically passed to every call to `vkAllocateMemory` done by the library using structure `VkMemoryPriorityAllocateInfoEXT`.
        /// The value to be used for default priority is 0.5.
        /// For more details, see the documentation of the VK_EXT_memory_priority extension.
        EXT_MEMORY_PRIORITY_BIT = 0x00000040,
        /// Enables usage of VK_KHR_maintenance4 extension in the library.
        ///
        /// You may set this flag only if you found available and enabled this device extension,
        /// while creating Vulkan device passed as VmaAllocatorCreateInfo::device.
        KHR_MAINTENANCE4_BIT = 0x00000080,
        /// Enables usage of VK_KHR_maintenance5 extension in the library.
        ///
        /// You should set this flag if you found available and enabled this device extension,
        /// while creating Vulkan device passed as VmaAllocatorCreateInfo::device.
        KHR_MAINTENANCE5_BIT = 0x00000100,

        /// Enables usage of VK_KHR_external_memory_win32 extension in the library.
        ///
        /// You should set this flag if you found available and enabled this device extension,
        /// while creating Vulkan device passed as VmaAllocatorCreateInfo::device.
        /// For more information, see the VMA documentation on API interoperability.
        KHR_EXTERNAL_MEMORY_WIN32_BIT = 0x00000200,
    }

    impl std::ops::BitOr for VmaAllocatorCreateFlags {
        type Output = u32;

        fn bitor(self, rhs: Self) -> u32 {
            (self as u32) | (rhs as u32)
        }
    }

    /// Intended usage of the allocated memory.
    #[repr(u32)]
    pub enum VmaMemoryUsage {
        /// No intended memory usage specified.
        /// Use other members of VmaAllocationCreateInfo to specify your requirements.
        UNKNOWN = 0,
        /// **Deprecated:** Obsolete, preserved for backward compatibility.
        /// Prefers `VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT`.
        /*GPU_ONLY = 1,
        /// **Deprecated:** Obsolete, preserved for backward compatibility.
        /// Guarantees `VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT` and `VK_MEMORY_PROPERTY_HOST_COHERENT_BIT`.
        CPU_ONLY = 2,
        /// **Deprecated:** Obsolete, preserved for backward compatibility.
        /// Guarantees `VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT`, prefers `VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT`.
        CPU_TO_GPU = 3,
        /// **Deprecated:** Obsolete, preserved for backward compatibility.
        /// Guarantees `VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT`, prefers `VK_MEMORY_PROPERTY_HOST_CACHED_BIT`.
        GPU_TO_CPU = 4,
        /// **Deprecated:** Obsolete, preserved for backward compatibility.
        /// Prefers not `VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT`.
        CPU_COPY = 5,*/
        /// Lazily allocated GPU memory having `VK_MEMORY_PROPERTY_LAZILY_ALLOCATED_BIT`.
        /// Exists mostly on mobile platforms. Using it on desktop PC or other GPUs with no such memory type present will fail the allocation.
        ///
        /// Usage: Memory for transient attachment images (color attachments, depth attachments etc.), created with `VK_IMAGE_USAGE_TRANSIENT_ATTACHMENT_BIT`.
        ///
        /// Allocations with this usage are always created as dedicated - it implies `VMA_ALLOCATION_CREATE_DEDICATED_MEMORY_BIT`.
        GPU_LAZILY_ALLOCATED = 6,
        /// Selects best memory type automatically.
        /// This flag is recommended for most common use cases.
        ///
        /// When using this flag, if you want to map the allocation (using vmaMapMemory() or `VMA_ALLOCATION_CREATE_MAPPED_BIT`),
        /// you must pass one of the flags: `VMA_ALLOCATION_CREATE_HOST_ACCESS_SEQUENTIAL_WRITE_BIT` or `VMA_ALLOCATION_CREATE_HOST_ACCESS_RANDOM_BIT`
        /// in VmaAllocationCreateInfo::flags.
        ///
        /// It can be used only with functions that let the library know `VkBufferCreateInfo` or `VkImageCreateInfo`, e.g.
        /// vmaCreateBuffer(), vmaCreateImage(), vmaFindMemoryTypeIndexForBufferInfo(), vmaFindMemoryTypeIndexForImageInfo()
        /// and not with generic memory allocation functions.
        AUTO = 7,
        /// Selects best memory type automatically with preference for GPU (device) memory.
        ///
        /// When using this flag, if you want to map the allocation (using vmaMapMemory() or `VMA_ALLOCATION_CREATE_MAPPED_BIT`),
        /// you must pass one of the flags: `VMA_ALLOCATION_CREATE_HOST_ACCESS_SEQUENTIAL_WRITE_BIT` or `VMA_ALLOCATION_CREATE_HOST_ACCESS_RANDOM_BIT`
        /// in VmaAllocationCreateInfo::flags.
        ///
        /// It can be used only with functions that let the library know `VkBufferCreateInfo` or `VkImageCreateInfo`, e.g.
        /// vmaCreateBuffer(), vmaCreateImage(), vmaFindMemoryTypeIndexForBufferInfo(), vmaFindMemoryTypeIndexForImageInfo()
        /// and not with generic memory allocation functions.
        AUTO_PREFER_DEVICE = 8,
        /// Selects best memory type automatically with preference for CPU (host) memory.
        ///
        /// When using this flag, if you want to map the allocation (using vmaMapMemory() or `VMA_ALLOCATION_CREATE_MAPPED_BIT`),
        /// you must pass one of the flags: `VMA_ALLOCATION_CREATE_HOST_ACCESS_SEQUENTIAL_WRITE_BIT` or `VMA_ALLOCATION_CREATE_HOST_ACCESS_RANDOM_BIT`
        /// in VmaAllocationCreateInfo::flags.
        ///
        /// It can be used only with functions that let the library know `VkBufferCreateInfo` or `VkImageCreateInfo`, e.g.
        /// vmaCreateBuffer(), vmaCreateImage(), vmaFindMemoryTypeIndexForBufferInfo(), vmaFindMemoryTypeIndexForImageInfo()
        /// and not with generic memory allocation functions.
        AUTO_PREFER_HOST = 9,
    }

    /// Flags to be passed as VmaAllocationCreateInfo::flags.
    #[repr(u32)]
    pub enum VmaAllocationCreateFlags {
        /// Set this flag if the allocation should have its own memory block.
        ///
        /// Use it for special, big resources, like fullscreen images used as attachments.
        ///
        /// If you use this flag while creating a buffer or an image, `VkMemoryDedicatedAllocateInfo`
        /// structure is applied if possible.
        DEDICATED_MEMORY_BIT = 0x00000001,
        /// Set this flag to only try to allocate from existing `VkDeviceMemory` blocks and never create new such block.
        ///
        /// If new allocation cannot be placed in any of the existing blocks, allocation
        /// fails with `VK_ERROR_OUT_OF_DEVICE_MEMORY` error.
        ///
        /// You should not use `DEDICATED_MEMORY_BIT` and
        /// `NEVER_ALLOCATE_BIT` at the same time. It makes no sense.
        NEVER_ALLOCATE_BIT = 0x00000002,
        /// Set this flag to use a memory that will be persistently mapped and retrieve pointer to it.
        ///
        /// Pointer to mapped memory will be returned through VmaAllocationInfo::pMappedData.
        ///
        /// It is valid to use this flag for allocation made from memory type that is not
        /// `HOST_VISIBLE`. This flag is then ignored and memory is not mapped. This is
        /// useful if you need an allocation that is efficient to use on GPU
        /// (`DEVICE_LOCAL`) and still want to map it directly if possible on platforms that
        /// support it (e.g. Intel GPU).
        MAPPED_BIT = 0x00000004,
        /// **Deprecated:** Preserved for backward compatibility. Consider using vmaSetAllocationName() instead.
        ///
        /// Set this flag to treat VmaAllocationCreateInfo::pUserData as pointer to a
        /// null-terminated string. Instead of copying pointer value, a local copy of the
        /// string is made and stored in allocation's `pName`. The string is automatically
        /// freed together with the allocation. It is also used in vmaBuildStatsString().
        USER_DATA_COPY_STRING_BIT = 0x00000020,
        /// Allocation will be created from upper stack in a double stack pool.
        ///
        /// This flag is only allowed for custom pools created with `VMA_POOL_CREATE_LINEAR_ALGORITHM_BIT` flag.
        UPPER_ADDRESS_BIT = 0x00000040,
        /// Create both buffer/image and allocation, but don't bind them together.
        /// It is useful when you want to bind yourself to do some more advanced binding, e.g. using some extensions.
        /// The flag is meaningful only with functions that bind by default: vmaCreateBuffer(), vmaCreateImage().
        /// Otherwise it is ignored.
        ///
        /// If you want to make sure the new buffer/image is not tied to the new memory allocation
        /// through `VkMemoryDedicatedAllocateInfoKHR` structure in case the allocation ends up in its own memory block,
        /// use also flag `CAN_ALIAS_BIT`.
        DONT_BIND_BIT = 0x00000080,
        /// Create allocation only if additional device memory required for it, if any, won't exceed
        /// memory budget. Otherwise return `VK_ERROR_OUT_OF_DEVICE_MEMORY`.
        WITHIN_BUDGET_BIT = 0x00000100,
        /// Set this flag if the allocated memory will have aliasing resources.
        ///
        /// Usage of this flag prevents supplying `VkMemoryDedicatedAllocateInfoKHR` when `DEDICATED_MEMORY_BIT` is specified.
        /// Otherwise created dedicated memory will not be suitable for aliasing resources, resulting in Vulkan Validation Layer errors.
        CAN_ALIAS_BIT = 0x00000200,
        /// Requests possibility to map the allocation (using vmaMapMemory() or `MAPPED_BIT`).
        ///
        /// - If you use `VMA_MEMORY_USAGE_AUTO` or other `VMA_MEMORY_USAGE_AUTO*` value,
        ///   you must use this flag to be able to map the allocation. Otherwise, mapping is incorrect.
        /// - If you use other value of `VmaMemoryUsage`, this flag is ignored and mapping is always possible in memory types that are `HOST_VISIBLE`.
        ///   This includes allocations created in custom memory pools.
        ///
        /// Declares that mapped memory will only be written sequentially, e.g. using `memcpy()` or a loop writing number-by-number,
        /// never read or accessed randomly, so a memory type can be selected that is uncached and write-combined.
        ///
        /// **Warning:** Violating this declaration may work correctly, but will likely be very slow.
        /// Watch out for implicit reads introduced by doing e.g. `pMappedData[i] += x;`
        /// Better prepare your data in a local variable and `memcpy()` it to the mapped pointer all at once.
        HOST_ACCESS_SEQUENTIAL_WRITE_BIT = 0x00000400,
        /// Requests possibility to map the allocation (using vmaMapMemory() or `MAPPED_BIT`).
        ///
        /// - If you use `VMA_MEMORY_USAGE_AUTO` or other `VMA_MEMORY_USAGE_AUTO*` value,
        ///   you must use this flag to be able to map the allocation. Otherwise, mapping is incorrect.
        /// - If you use other value of `VmaMemoryUsage`, this flag is ignored and mapping is always possible in memory types that are `HOST_VISIBLE`.
        ///   This includes allocations created in custom memory pools.
        ///
        /// Declares that mapped memory can be read, written, and accessed in random order,
        /// so a `HOST_CACHED` memory type is preferred.
        HOST_ACCESS_RANDOM_BIT = 0x00000800,
        /// Together with `HOST_ACCESS_SEQUENTIAL_WRITE_BIT` or `HOST_ACCESS_RANDOM_BIT`,
        /// it says that despite request for host access, a not-`HOST_VISIBLE` memory type can be selected
        /// if it may improve performance.
        ///
        /// By using this flag, you declare that you will check if the allocation ended up in a `HOST_VISIBLE` memory type
        /// (e.g. using vmaGetAllocationMemoryProperties()) and if not, you will create some "staging" buffer and
        /// issue an explicit transfer to write/read your data.
        /// To prepare for this possibility, don't forget to add appropriate flags like
        /// `VK_BUFFER_USAGE_TRANSFER_DST_BIT`, `VK_BUFFER_USAGE_TRANSFER_SRC_BIT` to the parameters of created buffer or image.
        HOST_ACCESS_ALLOW_TRANSFER_INSTEAD_BIT = 0x00001000,
        /// Allocation strategy that chooses smallest possible free range for the allocation
        /// to minimize memory usage and fragmentation, possibly at the expense of allocation time.
        STRATEGY_MIN_MEMORY_BIT = 0x00010000,
        /// Allocation strategy that chooses first suitable free range for the allocation -
        /// not necessarily in terms of the smallest offset but the one that is easiest and fastest to find
        /// to minimize allocation time, possibly at the expense of allocation quality.
        STRATEGY_MIN_TIME_BIT = 0x00020000,
        /// Allocation strategy that chooses always the lowest offset in available space.
        /// This is not the most efficient strategy but achieves highly packed data.
        /// Used internally by defragmentation, not recommended in typical usage.
        STRATEGY_MIN_OFFSET_BIT = 0x00040000,
    }
}
