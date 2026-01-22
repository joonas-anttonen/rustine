#![allow(dead_code)]

use crate::{error, vk_call, warning};
use crate::{gfx::pipeline::*, gfx::vulkan as vk, gfx::*};

use std::{collections::HashSet, rc::Rc};

pub struct CommandPool {
    handle: vk::VkCommandPool,
    device: Rc<Device>,
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        warning!("CommandPool::drop");
        unsafe {
            vk::vkDestroyCommandPool(self.device.handle(), self.handle, std::ptr::null());
        }
    }
}

impl CommandPool {
    pub fn new(family_index: u32, device: &Rc<Device>) -> Rc<CommandPool> {
        let command_pool_create_info = vk::VkCommandPoolCreateInfo {
            sType: vk::VkStructureType::COMMAND_POOL_CREATE_INFO as u32,
            pNext: std::ptr::null(),
            flags: vk::VkCommandPoolCreateFlags::TRANSIENT_BIT
                | vk::VkCommandPoolCreateFlags::RESET_COMMAND_BUFFER_BIT,
            queueFamilyIndex: family_index,
        };

        let mut command_pool_handle = vk::VkCommandPool::default();
        unsafe {
            vk_call!(vk::vkCreateCommandPool(
                device.handle(),
                &command_pool_create_info,
                std::ptr::null(),
                &mut command_pool_handle,
            ))
            .map_err(|err| error!("vkCreateCommandPool {:?}", err))
            .unwrap();
        }

        Rc::new(CommandPool {
            handle: command_pool_handle,
            device: Rc::clone(device),
        })
    }

    pub fn allocate_command_buffer(self: &Rc<Self>) -> Result<CommandBuffer> {
        let allocate_info = vk::VkCommandBufferAllocateInfo {
            sType: vk::VkStructureType::COMMAND_BUFFER_ALLOCATE_INFO,
            pNext: std::ptr::null(),
            commandPool: self.handle,
            level: vk::VkCommandBufferLevel::PRIMARY,
            commandBufferCount: 1,
        };

        let mut command_buffer_handle = vk::VkCommandBuffer::default();
        unsafe {
            vk_call!(vk::vkAllocateCommandBuffers(
                self.device.handle(),
                &allocate_info,
                &mut command_buffer_handle
            ))?;
        }

        // Create fence
        let fence_info = vk::VkFenceCreateInfo {
            sType: vk::VkStructureType::FENCE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
        };
        let mut fence_handle = vk::VkFence::default();
        unsafe {
            vk_call!(vk::vkCreateFence(
                self.device.handle(),
                &fence_info,
                std::ptr::null(),
                &mut fence_handle
            ))?;
        }

        let semaphore_info = vk::VkSemaphoreCreateInfo {
            sType: vk::VkStructureType::SEMAPHORE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
        };
        let mut semaphore_handle = vk::VkSemaphore::default();
        unsafe {
            vk_call!(vk::vkCreateSemaphore(
                self.device.handle(),
                &semaphore_info,
                std::ptr::null(),
                &mut semaphore_handle
            ))?;
        }

        Ok(CommandBuffer::new(
            command_buffer_handle,
            fence_handle,
            semaphore_handle,
            Rc::clone(self),
        ))
    }
}

/// Represents a command buffer used for recording graphics commands.
pub struct CommandBuffer {
    memory_buffers_in_use: HashSet<Rc<MemoryBuffer>>,
    pixel_buffers_in_use: HashSet<Rc<PixelBuffer>>,
    samplers_in_use: HashSet<Rc<Sampler>>,
    pipelines_in_use: HashSet<Rc<Pipeline>>,

    handle: vk::VkCommandBuffer,
    fence: vk::VkFence,
    semaphore: vk::VkSemaphore,
    pool: Rc<CommandPool>,
}

impl Drop for CommandBuffer {
    fn drop(&mut self) {
        warning!("CommandBuffer::drop");
        unsafe {
            vk::vkDestroySemaphore(self.pool.device.handle(), self.semaphore, std::ptr::null());
            vk::vkDestroyFence(self.pool.device.handle(), self.fence, std::ptr::null());
            vk::vkFreeCommandBuffers(self.pool.device.handle(), self.pool.handle, 1, &self.handle);
        }
    }
}

impl CommandBuffer {
    /// Creates a new command buffer with the given handle, fence, semaphore, and pool.
    pub fn new(
        handle: vk::VkCommandBuffer,
        fence: vk::VkFence,
        semaphore: vk::VkSemaphore,
        pool: Rc<CommandPool>,
    ) -> Self {
        CommandBuffer {
            pixel_buffers_in_use: HashSet::with_capacity(16),
            samplers_in_use: HashSet::with_capacity(16),
            memory_buffers_in_use: HashSet::with_capacity(16),
            pipelines_in_use: HashSet::with_capacity(16),
            handle,
            fence,
            semaphore,
            pool,
        }
    }

    pub fn handle(&self) -> vk::VkCommandBuffer {
        self.handle
    }

    pub fn semaphore(&self) -> vk::VkSemaphore {
        self.semaphore
    }

    pub fn fence(&self) -> vk::VkFence {
        self.fence
    }

    pub fn is_complete(&self) -> bool {
        let status = unsafe { vk::vkGetFenceStatus(self.pool.device.handle(), self.fence) };
        match status {
            vk::VkResult::SUCCESS => true,
            vk::VkResult::NOT_READY => false,
            _ => {
                error!("Failed to get fence status: {:?}", status);
                false
            }
        }
    }

    pub fn wait_for_completion(&self, timeout_ns: u64) -> bool {
        let result = unsafe {
            vk::vkWaitForFences(
                self.pool.device.handle(),
                1,
                &self.fence,
                vk::VK_TRUE,
                timeout_ns,
            )
        };
        match result {
            vk::VkResult::SUCCESS => true,
            vk::VkResult::TIMEOUT => false,
            _ => {
                error!("Failed to wait for fence: {:?}", result);
                false
            }
        }
    }

    pub fn reset(&mut self) {
        self.pipelines_in_use.clear();
        self.pixel_buffers_in_use.clear();
        self.samplers_in_use.clear();
        self.memory_buffers_in_use.clear();

        unsafe {
            vk_call!(vk::vkResetFences(self.pool.device.handle(), 1, &self.fence)).unwrap_or_else(
                |r| {
                    error!("Failed to reset fence: {:?}", r);
                },
            );
            vk_call!(vk::vkResetCommandBuffer(self.handle, 0)).unwrap_or_else(|r| {
                error!("Failed to reset command buffer: {:?}", r);
            });
        }
    }

    /// Begins recording commands into the command buffer.
    /// `vkCmdBeginCommandBuffer`
    pub fn begin(&self) {
        let begin_info = vk::VkCommandBufferBeginInfo {
            sType: vk::VkStructureType::COMMAND_BUFFER_BEGIN_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            pInheritanceInfo: std::ptr::null(),
        };
        unsafe {
            vk_call!(vk::vkBeginCommandBuffer(self.handle, &begin_info)).unwrap_or_else(|r| {
                error!("Failed to begin command buffer: {:?}", r);
            });
        }
    }

    /// Ends recording commands into the command buffer.
    /// `vkCmdEndCommandBuffer`
    pub fn end(&self) {
        unsafe {
            vk_call!(vk::vkEndCommandBuffer(self.handle)).unwrap_or_else(|r| {
                error!("Failed to end command buffer: {:?}", r);
            });
        }
    }

    /// Binds a graphics pipeline to the command buffer
    /// `vkCmdBindPipeline`
    pub fn bind_pipeline(&mut self, pipeline: &Rc<Pipeline>) {
        self.pipelines_in_use.insert(pipeline.clone());

        unsafe {
            vk::vkCmdBindPipeline(
                self.handle,
                vk::VkPipelineBindPoint::GRAPHICS,
                pipeline.handle(),
            );
        }
    }

    pub fn bind_vertex_buffer(&mut self, memory_buffer: &Rc<MemoryBuffer>) {
        self.memory_buffers_in_use.insert(memory_buffer.clone());

        unsafe {
            let offset = vk::VkDeviceSize(0);
            vk::vkCmdBindVertexBuffers(
                self.handle,
                0,
                1,
                &memory_buffer.handle(),
                &[offset] as *const vk::VkDeviceSize,
            );
        }
    }

    pub fn bind_index_buffer(&mut self, memory_buffer: &Rc<MemoryBuffer>) {
        self.memory_buffers_in_use.insert(memory_buffer.clone());

        unsafe {
            vk::vkCmdBindIndexBuffer(
                self.handle,
                memory_buffer.handle(),
                vk::VkDeviceSize(0),
                vk::VkIndexType::UINT32,
            );
        }
    }

    pub fn begin_rendering(
        &mut self,
        render_area: &Rectangle,
        color_attachments: &[&Rc<PixelBuffer>],
    ) {
        unsafe {
            const MAX_COLOR_ATTACHMENTS: usize = 2;

            let mut vk_color_attachments: [vk::VkRenderingAttachmentInfo; MAX_COLOR_ATTACHMENTS] =
                std::mem::zeroed();

            for i in 0..MAX_COLOR_ATTACHMENTS - 1 {
                self.pixel_buffers_in_use
                    .insert(Rc::clone(color_attachments[i]));

                vk_color_attachments[i] = vk::VkRenderingAttachmentInfo {
                    sType: vk::VkStructureType::RENDERING_ATTACHMENT_INFO,
                    pNext: std::ptr::null(),
                    imageView: color_attachments[i].image_view(),
                    imageLayout: color_attachments[i].layout().to_vk(),
                    resolveMode: vk::VkResolveModeFlags::NONE,
                    resolveImageView: vk::VkImageView::default(),
                    resolveImageLayout: vk::VkImageLayout::UNDEFINED,
                    loadOp: vk::VkAttachmentLoadOp::LOAD,
                    storeOp: vk::VkAttachmentStoreOp::STORE,
                    clearValue: std::mem::zeroed(),
                }
            }

            let rendering_info = vk::VkRenderingInfo {
                sType: vk::VkStructureType::RENDERING_INFO,
                pNext: std::ptr::null(),
                flags: 0,
                renderArea: vk::VkRect2D {
                    offset: vk::VkOffset2D {
                        x: render_area.x as i32,
                        y: render_area.y as i32,
                    },
                    extent: vk::VkExtent2D {
                        width: render_area.w as u32,
                        height: render_area.h as u32,
                    },
                },
                layerCount: 1,
                viewMask: 0,
                colorAttachmentCount: color_attachments.len() as u32,
                pColorAttachments: vk_color_attachments.as_ptr(),
                pDepthAttachment: std::ptr::null(),
                pStencilAttachment: std::ptr::null(),
            };

            vk::vkCmdBeginRendering(self.handle, &rendering_info);
        }
    }

    pub fn set_viewport(&self, viewport: &Rectangle) {
        unsafe {
            let vk_viewport = vk::VkViewport {
                x: viewport.x,
                y: viewport.y,
                width: viewport.w,
                height: viewport.h,
                minDepth: 0.0,
                maxDepth: 1.0,
            };
            vk::vkCmdSetViewport(self.handle, 0, 1, &vk_viewport);
        }
    }

    pub fn set_scissor(&self, scissor: &Rectangle) {
        unsafe {
            let vk_scissor = vk::VkRect2D {
                offset: vk::VkOffset2D {
                    x: scissor.x as i32,
                    y: scissor.y as i32,
                },
                extent: vk::VkExtent2D {
                    width: scissor.w as u32,
                    height: scissor.h as u32,
                },
            };
            vk::vkCmdSetScissor(self.handle, 0, 1, &vk_scissor);
        }
    }

    pub fn push_pixel_descriptor(
        &mut self,
        pipeline: &Rc<Pipeline>,
        pixel_buffer_binding: u32,
        pixel_buffer: &Rc<PixelBuffer>,
        sampler_binding: u32,
        sampler: &Rc<Sampler>,
    ) {
        // We trust that the pipeline is already tracked via bind_pipeline
        self.pixel_buffers_in_use.insert(Rc::clone(&pixel_buffer));
        self.samplers_in_use.insert(Rc::clone(&sampler));

        let descriptor_image_info = vk::VkDescriptorImageInfo {
            sampler: vk::VkSampler::default(),
            imageView: pixel_buffer.image_view(),
            imageLayout: pixel_buffer.layout().to_vk(),
        };
        let descriptor_sampler_info = vk::VkDescriptorImageInfo {
            sampler: sampler.handle(),
            imageView: vk::VkImageView::default(),
            imageLayout: vk::VkImageLayout::UNDEFINED,
        };
        let descriptor_writes: [vk::VkWriteDescriptorSet; 2] = [
            vk::VkWriteDescriptorSet {
                sType: vk::VkStructureType::WRITE_DESCRIPTOR_SET,
                pNext: std::ptr::null(),
                dstSet: std::ptr::null_mut(),
                dstBinding: pixel_buffer_binding,
                dstArrayElement: 0,
                descriptorCount: 1,
                descriptorType: vk::VkDescriptorType::SAMPLED_IMAGE,
                pImageInfo: &descriptor_image_info,
                pBufferInfo: std::ptr::null(),
                pTexelBufferView: std::ptr::null(),
            },
            vk::VkWriteDescriptorSet {
                sType: vk::VkStructureType::WRITE_DESCRIPTOR_SET,
                pNext: std::ptr::null(),
                dstSet: std::ptr::null_mut(),
                dstBinding: sampler_binding,
                dstArrayElement: 0,
                descriptorCount: 1,
                descriptorType: vk::VkDescriptorType::SAMPLER,
                pImageInfo: &descriptor_sampler_info,
                pBufferInfo: std::ptr::null(),
                pTexelBufferView: std::ptr::null(),
            },
        ];

        unsafe {
            vk::vkCmdPushDescriptorSet(
                self.handle,
                vk::VkPipelineBindPoint::GRAPHICS,
                pipeline.pipeline_layout(),
                0,
                2,
                descriptor_writes.as_ptr(),
            );
        }
    }

    pub fn end_rendering(&self) {
        unsafe {
            vk::vkCmdEndRendering(self.handle);
        }
    }

    /// `vkCmdDraw`
    pub fn draw(
        &self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        unsafe {
            vk::vkCmdDraw(
                self.handle,
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
            );
        }
    }

    pub fn draw_indexed(
        &self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) {
        unsafe {
            vk::vkCmdDrawIndexed(
                self.handle,
                index_count,
                instance_count,
                first_index,
                vertex_offset,
                first_instance,
            );
        }
    }

    pub fn push_constants<T>(&self, pipeline: &Rc<Pipeline>, stage: Stage, data: &T) {
        unsafe {
            vk::vkCmdPushConstants(
                self.handle,
                pipeline.pipeline_layout(),
                vk::VkShaderStageFlags(stage.0),
                0,
                std::mem::size_of::<T>() as u32,
                data as *const T as *const std::ffi::c_void,
            );
        }
    }

    /// Copies data from a memory buffer to a pixel buffer image.
    ///
    /// Always uses full image extents.
    ///
    /// Requires that the destination image is in `TRANSFER_DST_OPTIMAL` layout.
    pub fn copy_buffer_to_image(&mut self, src: &Rc<MemoryBuffer>, dst: &Rc<PixelBuffer>) {
        self.memory_buffers_in_use.insert(Rc::clone(src));
        self.pixel_buffers_in_use.insert(Rc::clone(dst));

        unsafe {
            vk::vkCmdCopyBufferToImage(
                self.handle,
                src.handle(),
                dst.image(),
                dst.layout().to_vk(),
                1,
                &vk::VkBufferImageCopy {
                    bufferOffset: vk::VkDeviceSize(0),
                    bufferRowLength: 0,
                    bufferImageHeight: 0,
                    imageSubresource: vk::VkImageSubresourceLayers {
                        aspectMask: vk::VkImageAspectFlags::COLOR_BIT,
                        mipLevel: 0,
                        baseArrayLayer: 0,
                        layerCount: 1,
                    },
                    imageOffset: vk::VkOffset3D { x: 0, y: 0, z: 0 },
                    imageExtent: vk::VkExtent3D {
                        width: dst.width(),
                        height: dst.height(),
                        depth: 1,
                    },
                },
            );
        }
    }

    /// Blits an image from a source to a destination with the specified filter.
    ///
    /// Always uses full image extents.
    ///
    /// Requires that the source image is in `TRANSFER_SRC_OPTIMAL` layout and the destination image is in `TRANSFER_DST_OPTIMAL` layout.
    pub fn blit(&mut self, src: &Rc<PixelBuffer>, dst: &Rc<PixelBuffer>, filter: Filter) {
        self.pixel_buffers_in_use.insert(Rc::clone(src));
        self.pixel_buffers_in_use.insert(Rc::clone(dst));

        self.blit_raw(src.image(), src.size(), dst.image(), dst.size(), filter);
    }

    /// Blits an image from a source to a destination with the specified filter.
    ///
    /// Always uses full image extents.
    ///
    /// Requires that the source image is in `TRANSFER_SRC_OPTIMAL` layout and the destination image is in `TRANSFER_DST_OPTIMAL` layout.
    pub fn blit_to_present(
        &mut self,
        src: &Rc<PixelBuffer>,
        dst: &presentation::PresentationImage,
        filter: Filter,
    ) {
        self.pixel_buffers_in_use.insert(Rc::clone(src));

        self.blit_raw(src.image(), src.size(), dst.image, dst.size(), filter);
    }

    /// Blits an image from a source to a destination with the specified filter.
    ///
    /// Always uses full image extents.
    ///
    /// Requires that the source image is in `TRANSFER_SRC_OPTIMAL` layout and the destination image is in `TRANSFER_DST_OPTIMAL` layout.
    fn blit_raw(
        &self,
        src: vk::VkImage,
        src_size: Vector2i,
        dst: vk::VkImage,
        dst_size: Vector2i,
        filter: Filter,
    ) {
        let blit_region = vk::VkImageBlit {
            srcSubresource: vk::VkImageSubresourceLayers {
                aspectMask: vk::VkImageAspectFlags::COLOR_BIT,
                mipLevel: 0,
                baseArrayLayer: 0,
                layerCount: 1,
            },
            srcOffsets: [
                vk::VkOffset3D { x: 0, y: 0, z: 0 },
                vk::VkOffset3D {
                    x: src_size.x,
                    y: src_size.y,
                    z: 1,
                },
            ],
            dstSubresource: vk::VkImageSubresourceLayers {
                aspectMask: vk::VkImageAspectFlags::COLOR_BIT,
                mipLevel: 0,
                baseArrayLayer: 0,
                layerCount: 1,
            },
            dstOffsets: [
                vk::VkOffset3D { x: 0, y: 0, z: 0 },
                vk::VkOffset3D {
                    x: dst_size.x,
                    y: dst_size.y,
                    z: 1,
                },
            ],
        };

        unsafe {
            vk::vkCmdBlitImage(
                self.handle,
                src,
                vk::VkImageLayout::TRANSFER_SRC_OPTIMAL,
                dst,
                vk::VkImageLayout::TRANSFER_DST_OPTIMAL,
                1,
                &blit_region,
                filter.to_vk(),
            )
        };
    }

    fn barrier_stage_mask(layout: vk::VkImageLayout) -> vk::VkPipelineStageFlags2 {
        match layout {
            vk::VkImageLayout::UNDEFINED => vk::VkPipelineStageFlags2::TOP_OF_PIPE_BIT,
            vk::VkImageLayout::TRANSFER_DST_OPTIMAL => vk::VkPipelineStageFlags2::ALL_TRANSFER_BIT,
            vk::VkImageLayout::TRANSFER_SRC_OPTIMAL => vk::VkPipelineStageFlags2::ALL_TRANSFER_BIT,
            vk::VkImageLayout::COLOR_ATTACHMENT_OPTIMAL => {
                vk::VkPipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT_BIT
            }
            vk::VkImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => {
                vk::VkPipelineStageFlags2::EARLY_FRAGMENT_TESTS_BIT
                    | vk::VkPipelineStageFlags2::LATE_FRAGMENT_TESTS_BIT
            }
            vk::VkImageLayout::SHADER_READ_ONLY_OPTIMAL => {
                vk::VkPipelineStageFlags2::FRAGMENT_SHADER_BIT
            }
            vk::VkImageLayout::PRESENT_SRC_KHR => vk::VkPipelineStageFlags2::BOTTOM_OF_PIPE_BIT,
            // Something else: you get everything
            _ => vk::VkPipelineStageFlags2::ALL_COMMANDS_BIT,
        }
    }

    fn barrier_access_mask(layout: vk::VkImageLayout) -> vk::VkAccessFlags2 {
        match layout {
            vk::VkImageLayout::UNDEFINED => vk::VkAccessFlags2::NONE,
            vk::VkImageLayout::PRESENT_SRC_KHR => vk::VkAccessFlags2::NONE,
            vk::VkImageLayout::TRANSFER_DST_OPTIMAL => vk::VkAccessFlags2::TRANSFER_WRITE_BIT,
            vk::VkImageLayout::TRANSFER_SRC_OPTIMAL => vk::VkAccessFlags2::TRANSFER_READ_BIT,
            vk::VkImageLayout::COLOR_ATTACHMENT_OPTIMAL => {
                vk::VkAccessFlags2::COLOR_ATTACHMENT_WRITE_BIT
            }
            vk::VkImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => {
                vk::VkAccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE_BIT
            }
            vk::VkImageLayout::SHADER_READ_ONLY_OPTIMAL => vk::VkAccessFlags2::SHADER_READ_BIT,
            // Something else: you get everything
            _ => {
                vk::VkAccessFlags2::MEMORY_READ_BIT
                    | vk::VkAccessFlags2::MEMORY_WRITE_BIT
                    | vk::VkAccessFlags2::TRANSFER_READ_BIT
                    | vk::VkAccessFlags2::TRANSFER_WRITE_BIT
                    | vk::VkAccessFlags2::COLOR_ATTACHMENT_READ_BIT
                    | vk::VkAccessFlags2::COLOR_ATTACHMENT_WRITE_BIT
                    | vk::VkAccessFlags2::DEPTH_STENCIL_ATTACHMENT_READ_BIT
                    | vk::VkAccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE_BIT
                    | vk::VkAccessFlags2::SHADER_READ_BIT
                    | vk::VkAccessFlags2::SHADER_WRITE_BIT
            }
        }
    }

    pub fn full_barrier(&self) {
        let memory_barrier = vk::VkMemoryBarrier2 {
            sType: vk::VkStructureType::MEMORY_BARRIER_2,
            pNext: std::ptr::null(),
            srcStageMask: vk::VkPipelineStageFlags2::ALL_COMMANDS_BIT,
            srcAccessMask: vk::VkAccessFlags2::NONE,
            dstStageMask: vk::VkPipelineStageFlags2::ALL_COMMANDS_BIT,
            dstAccessMask: vk::VkAccessFlags2::NONE,
        };
        let dependency_info = vk::VkDependencyInfo {
            sType: vk::VkStructureType::DEPENDENCY_INFO,
            pNext: std::ptr::null(),
            dependencyFlags: 0,
            memoryBarrierCount: 1,
            pMemoryBarriers: &memory_barrier,
            bufferMemoryBarrierCount: 0,
            pBufferMemoryBarriers: std::ptr::null(),
            imageMemoryBarrierCount: 0,
            pImageMemoryBarriers: std::ptr::null(),
        };
        unsafe {
            vk::vkCmdPipelineBarrier2(self.handle, &dependency_info);
        }
    }

    pub fn transfer_barrier(&self) {
        let memory_barrier = vk::VkMemoryBarrier2 {
            sType: vk::VkStructureType::MEMORY_BARRIER_2,
            pNext: std::ptr::null(),
            srcStageMask: vk::VkPipelineStageFlags2::ALL_TRANSFER_BIT,
            srcAccessMask: vk::VkAccessFlags2::TRANSFER_WRITE_BIT,
            dstStageMask: vk::VkPipelineStageFlags2::ALL_TRANSFER_BIT,
            dstAccessMask: vk::VkAccessFlags2::TRANSFER_READ_BIT,
        };
        let dependency_info = vk::VkDependencyInfo {
            sType: vk::VkStructureType::DEPENDENCY_INFO,
            pNext: std::ptr::null(),
            dependencyFlags: 0,
            memoryBarrierCount: 1,
            pMemoryBarriers: &memory_barrier,
            bufferMemoryBarrierCount: 0,
            pBufferMemoryBarriers: std::ptr::null(),
            imageMemoryBarrierCount: 0,
            pImageMemoryBarriers: std::ptr::null(),
        };
        unsafe {
            vk::vkCmdPipelineBarrier2(self.handle, &dependency_info);
        }
    }

    pub fn present_image_barrier(
        &self,
        image: &presentation::PresentationImage,
        old_layout: Layout,
        new_layout: Layout,
    ) {
        self.raw_image_barrier(image.image, old_layout, new_layout);
    }

    pub fn layout_barrier(&mut self, buffer: &Rc<PixelBuffer>, new_layout: Layout) {
        let old_layout = buffer.layout();
        self.pixel_buffers_in_use.insert(Rc::clone(buffer));
        self.raw_image_barrier(buffer.image(), old_layout, new_layout);
        buffer.set_layout(new_layout);
    }

    fn raw_image_barrier(&self, image: vk::VkImage, old_layout: Layout, new_layout: Layout) {
        let image_memory_barrier = vk::VkImageMemoryBarrier2 {
            sType: vk::VkStructureType::IMAGE_MEMORY_BARRIER_2,
            pNext: std::ptr::null(),
            srcStageMask: Self::barrier_stage_mask(old_layout.to_vk()),
            srcAccessMask: Self::barrier_access_mask(old_layout.to_vk()),
            dstStageMask: Self::barrier_stage_mask(new_layout.to_vk()),
            dstAccessMask: Self::barrier_access_mask(new_layout.to_vk()),
            oldLayout: old_layout.to_vk(),
            newLayout: new_layout.to_vk(),
            srcQueueFamilyIndex: vk::VK_QUEUE_FAMILY_IGNORED,
            dstQueueFamilyIndex: vk::VK_QUEUE_FAMILY_IGNORED,
            image,
            subresourceRange: vk::VkImageSubresourceRange {
                // TODO: Support more aspects
                aspectMask: vk::VkImageAspectFlags::COLOR_BIT as u32,
                baseMipLevel: 0,
                levelCount: 1,
                baseArrayLayer: 0,
                layerCount: 1,
            },
        };
        let dependency_info = vk::VkDependencyInfo {
            sType: vk::VkStructureType::DEPENDENCY_INFO,
            pNext: std::ptr::null(),
            dependencyFlags: 0,
            memoryBarrierCount: 0,
            pMemoryBarriers: std::ptr::null(),
            bufferMemoryBarrierCount: 0,
            pBufferMemoryBarriers: std::ptr::null(),
            imageMemoryBarrierCount: 1,
            pImageMemoryBarriers: &image_memory_barrier,
        };

        unsafe {
            vk::vkCmdPipelineBarrier2(self.handle, &dependency_info);
        }
    }

    /// Clears the given pixel buffer to the specified color.
    /// Current layout of `buffer` must be `SHARED_PRESENT_KHR`, `GENERAL` or `TRANSFER_DST_OPTIMAL`.
    pub fn clear_pixel_buffer(&mut self, buffer: &Rc<PixelBuffer>, color: &[f32; 4]) {
        self.pixel_buffers_in_use.insert(Rc::clone(buffer));

        let clear_color = vk::VkClearColorValue { float32: *color };
        let image_subresource_range = vk::VkImageSubresourceRange {
            aspectMask: vk::VkImageAspectFlags::COLOR_BIT as u32,
            baseMipLevel: 0,
            levelCount: 1,
            baseArrayLayer: 0,
            layerCount: 1,
        };
        unsafe {
            vk::vkCmdClearColorImage(
                self.handle,
                buffer.image(),
                buffer.layout().to_vk(),
                &clear_color,
                1,
                &image_subresource_range,
            );
        }
    }
}
