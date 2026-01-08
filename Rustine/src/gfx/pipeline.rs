#![allow(dead_code)]

use crate::{gfx::vulkan as vk, gfx::*, vk_call, vk_next, warning};
use std::sync::Arc;

pub struct Parameters {
    pub shader: ShaderProgram,
    pub topology: Topology,
    pub winding: Winding,
    pub culling: Culling,
    pub raster: Raster,
    pub samples: Samples,
    pub depth_comparison: Comparison,
    pub depth_write: bool,
    pub depth_test: bool,
    pub bindings: Vec<Binding>,
    pub attributes: Vec<Attribute>,
    pub descriptors: Vec<Descriptor>,
    pub push_constants: Vec<PushConstantRange>,
    pub attachments: Vec<Attachment>,
}

pub struct Pipeline {
    pipeline: vk::VkPipeline,
    pipeline_layout: vk::VkPipelineLayout,
    descriptor_layout: vk::VkDescriptorSetLayout,
    device: Arc<Device>,
}

impl Eq for Pipeline {}
impl PartialEq for Pipeline {
    fn eq(&self, other: &Self) -> bool {
        self.pipeline == other.pipeline
    }
}
impl std::hash::Hash for Pipeline {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.pipeline.hash(state);
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        warning!("Pipeline::drop");
        unsafe {
            vk::vkDestroyPipeline(self.device.handle(), self.pipeline, std::ptr::null());
            vk::vkDestroyPipelineLayout(
                self.device.handle(),
                self.pipeline_layout,
                std::ptr::null(),
            );
            vk::vkDestroyDescriptorSetLayout(
                self.device.handle(),
                self.descriptor_layout,
                std::ptr::null(),
            );
        }
    }
}

impl Pipeline {
    pub fn handle(&self) -> vk::VkPipeline {
        self.pipeline
    }

    pub fn descriptor_layout(&self) -> vk::VkDescriptorSetLayout {
        self.descriptor_layout
    }

    pub fn pipeline_layout(&self) -> vk::VkPipelineLayout {
        self.pipeline_layout
    }

    pub fn new(device: Arc<Device>, params: &Parameters) -> Result<Self> {
        // 1. Descriptor Set Layout and Pipeline Layout
        let descriptor_layout =
            Self::create_descriptor_layout(device.handle(), &params.descriptors).unwrap();
        let pipeline_layout = Self::create_pipeline_layout(
            device.handle(),
            descriptor_layout,
            &params.push_constants,
        )
        .unwrap();

        // 2. Input Assembly
        let input_assembly = vk::VkPipelineInputAssemblyStateCreateInfo {
            sType: vk::VkStructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            topology: params.topology.to_vk(),
            primitiveRestartEnable: 0,
        };

        // 3. Rasterization
        let rasterization = vk::VkPipelineRasterizationStateCreateInfo {
            sType: vk::VkStructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            depthClampEnable: 0,
            rasterizerDiscardEnable: 0,
            polygonMode: params.raster.to_vk(),
            cullMode: params.culling.to_vk(),
            frontFace: params.winding.to_vk(),
            depthBiasEnable: 0,
            depthBiasConstantFactor: 0.0,
            depthBiasClamp: 0.0,
            depthBiasSlopeFactor: 0.0,
            lineWidth: 1.0,
        };

        // 4. Depth/Stencil
        let depth_stencil = vk::VkPipelineDepthStencilStateCreateInfo {
            sType: vk::VkStructureType::PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            depthTestEnable: if params.depth_test { 1 } else { 0 },
            depthWriteEnable: if params.depth_write { 1 } else { 0 },
            depthCompareOp: params.depth_comparison.to_vk(),
            depthBoundsTestEnable: 0,
            stencilTestEnable: 0,
            front: vk::VkStencilOpState {
                failOp: vk::VkStencilOp::KEEP,
                passOp: vk::VkStencilOp::KEEP,
                depthFailOp: vk::VkStencilOp::KEEP,
                compareOp: vk::VkCompareOp::ALWAYS,
                compareMask: 0,
                writeMask: 0,
                reference: 0,
            },
            back: vk::VkStencilOpState {
                failOp: vk::VkStencilOp::KEEP,
                passOp: vk::VkStencilOp::KEEP,
                depthFailOp: vk::VkStencilOp::KEEP,
                compareOp: vk::VkCompareOp::ALWAYS,
                compareMask: 0,
                writeMask: 0,
                reference: 0,
            },
            minDepthBounds: 0.0,
            maxDepthBounds: 1.0,
        };

        // 5. Viewport/Scissor
        let viewport_state = vk::VkPipelineViewportStateCreateInfo {
            sType: vk::VkStructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            viewportCount: 1,
            pViewports: std::ptr::null(),
            scissorCount: 1,
            pScissors: std::ptr::null(),
        };

        // 6. Dynamic State
        let dynamic_states = [
            vk::VkDynamicState::VIEWPORT,
            vk::VkDynamicState::SCISSOR,
            vk::VkDynamicState::DEPTH_WRITE_ENABLE,
        ];
        let dynamic_state = vk::VkPipelineDynamicStateCreateInfo {
            sType: vk::VkStructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            dynamicStateCount: dynamic_states.len() as u32,
            pDynamicStates: dynamic_states.as_ptr(),
        };

        // 7. Multisampling
        let multisample = vk::VkPipelineMultisampleStateCreateInfo {
            sType: vk::VkStructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            rasterizationSamples: params.samples.0,
            sampleShadingEnable: 0,
            minSampleShading: 0.0,
            pSampleMask: std::ptr::null(),
            alphaToCoverageEnable: 0,
            alphaToOneEnable: 0,
        };

        // 8. Vertex Input
        let vertex_input_bindings: Vec<vk::VkVertexInputBindingDescription> = params
            .bindings
            .iter()
            .map(|binding| {
                if binding.stride == 0 {
                    panic!("Invalid operation: vertex binding stride cannot be 0");
                }
                vk::VkVertexInputBindingDescription {
                    binding: binding.binding,
                    stride: binding.stride,
                    inputRate: binding.rate.to_vk(),
                }
            })
            .collect();

        let vertex_input_attributes: Vec<vk::VkVertexInputAttributeDescription> = params
            .attributes
            .iter()
            .map(|attr| {
                if attr.format.to_vk() == vk::VkFormat::UNDEFINED {
                    panic!("Invalid operation: vertex attribute format cannot be UNDEFINED");
                }
                vk::VkVertexInputAttributeDescription {
                    location: attr.location,
                    binding: attr.binding,
                    format: attr.format.to_vk(),
                    offset: attr.offset,
                }
            })
            .collect();

        let vertex_input_info = vk::VkPipelineVertexInputStateCreateInfo {
            sType: vk::VkStructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            vertexBindingDescriptionCount: vertex_input_bindings.len() as u32,
            pVertexBindingDescriptions: vertex_input_bindings.as_ptr(),
            vertexAttributeDescriptionCount: vertex_input_attributes.len() as u32,
            pVertexAttributeDescriptions: vertex_input_attributes.as_ptr(),
        };

        // 9. Color Attachments & Blending
        let color_attachment_formats: Vec<vk::VkFormat> = params
            .attachments
            .iter()
            .map(|att| att.format.to_vk())
            .collect();

        let color_attachment_blends: Vec<vk::VkPipelineColorBlendAttachmentState> = params
            .attachments
            .iter()
            .map(|att| att.blend.to_vk())
            .collect();

        let pipeline_rendering = vk::VkPipelineRenderingCreateInfo {
            sType: vk::VkStructureType::PIPELINE_RENDERING_CREATE_INFO,
            pNext: std::ptr::null(),
            viewMask: 0,
            colorAttachmentCount: color_attachment_formats.len() as u32,
            pColorAttachmentFormats: color_attachment_formats.as_ptr(),
            depthAttachmentFormat: if params.depth_test || params.depth_write {
                // Note: You'll need to pass device_depth_format as a parameter or access it somehow
                vk::VkFormat::UNDEFINED // Replace with actual depth format
            } else {
                vk::VkFormat::UNDEFINED
            },
            stencilAttachmentFormat: vk::VkFormat::UNDEFINED,
        };

        let color_blend_state = vk::VkPipelineColorBlendStateCreateInfo {
            sType: vk::VkStructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            logicOpEnable: 0,
            logicOp: vk::VkLogicOp::COPY,
            attachmentCount: color_attachment_blends.len() as u32,
            pAttachments: color_attachment_blends.as_ptr(),
            blendConstants: [0.0, 0.0, 0.0, 0.0],
        };

        // 10. Shader Stages
        let mut shader_modules = Vec::new();
        let shader_stage_create_infos: Vec<vk::VkPipelineShaderStageCreateInfo> = params
            .shader
            .stages
            .iter()
            .map(|stage| {
                let shader_module_create_info = vk::VkShaderModuleCreateInfo {
                    sType: vk::VkStructureType::SHADER_MODULE_CREATE_INFO,
                    pNext: std::ptr::null(),
                    flags: 0,
                    codeSize: stage.bytecode.len(),
                    pCode: stage.bytecode.as_ptr() as *const u32,
                };

                let mut shader_module: vk::VkShaderModule = std::ptr::null_mut();
                unsafe {
                    vk::vkCreateShaderModule(
                        device.handle(),
                        &shader_module_create_info,
                        std::ptr::null(),
                        &mut shader_module,
                    )
                };
                shader_modules.push(shader_module);

                vk::VkPipelineShaderStageCreateInfo {
                    sType: vk::VkStructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                    pNext: std::ptr::null(),
                    flags: 0,
                    stage: stage.stage.to_vk(),
                    module: shader_module,
                    pName: stage.entry_point.as_ptr() as *const i8,
                    pSpecializationInfo: std::ptr::null(),
                }
            })
            .collect();

        // 11. Finally, the pipeline
        let pipeline_info = vk::VkGraphicsPipelineCreateInfo {
            sType: vk::VkStructureType::GRAPHICS_PIPELINE_CREATE_INFO,
            pNext: vk_next!(pipeline_rendering),
            flags: 0,
            stageCount: shader_stage_create_infos.len() as u32,
            pStages: shader_stage_create_infos.as_ptr(),
            pVertexInputState: &vertex_input_info,
            pInputAssemblyState: &input_assembly,
            pTessellationState: std::ptr::null(),
            pViewportState: &viewport_state,
            pRasterizationState: &rasterization,
            pMultisampleState: &multisample,
            pDepthStencilState: &depth_stencil,
            pColorBlendState: &color_blend_state,
            pDynamicState: &dynamic_state,
            layout: pipeline_layout,
            renderPass: std::ptr::null_mut(),
            subpass: 0,
            basePipelineHandle: std::ptr::null_mut(),
            basePipelineIndex: -1,
        };

        let mut pipeline: vk::VkPipeline = std::ptr::null_mut();

        vk_call!(vk::vkCreateGraphicsPipelines(
            device.handle(),
            std::ptr::null_mut(),
            1,
            &pipeline_info,
            std::ptr::null(),
            &mut pipeline,
        ))?;

        // Clean up shader modules
        for shader_module in shader_modules {
            unsafe {
                vk::vkDestroyShaderModule(device.handle(), shader_module, std::ptr::null());
            }
        }

        Ok(Pipeline {
            pipeline,
            pipeline_layout,
            descriptor_layout,
            device: device.clone(),
        })
    }

    fn create_descriptor_layout(
        vk_device: vk::VkDevice,
        descriptors: &[Descriptor],
    ) -> Result<vk::VkDescriptorSetLayout> {
        let vk_descriptor_bindings: Vec<vk::VkDescriptorSetLayoutBinding> = descriptors
            .iter()
            .map(|descriptor| {
                if descriptor.descriptor_count == 0 {
                    panic!("Invalid operation: descriptor_count cannot be 0");
                }
                vk::VkDescriptorSetLayoutBinding {
                    binding: descriptor.binding,
                    descriptorType: descriptor.descriptor_type.to_vk(),
                    descriptorCount: descriptor.descriptor_count,
                    stageFlags: descriptor.stage.to_vk(),
                    pImmutableSamplers: std::ptr::null(),
                }
            })
            .collect();

        let layout_info = vk::VkDescriptorSetLayoutCreateInfo {
            sType: vk::VkStructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: vk::VkDescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_BIT,
            bindingCount: vk_descriptor_bindings.len() as u32,
            pBindings: vk_descriptor_bindings.as_ptr(),
        };

        let mut descriptor_set_layout: vk::VkDescriptorSetLayout = std::ptr::null_mut();
        vk_call!(vk::vkCreateDescriptorSetLayout(
            vk_device,
            &layout_info,
            std::ptr::null(),
            &mut descriptor_set_layout,
        ))?;

        Ok(descriptor_set_layout)
    }

    fn create_pipeline_layout(
        vk_device: vk::VkDevice,
        layout: vk::VkDescriptorSetLayout,
        push_constant_ranges: &[PushConstantRange],
    ) -> Result<vk::VkPipelineLayout> {
        let vk_push_constant_ranges: Vec<vk::VkPushConstantRange> = push_constant_ranges
            .iter()
            .map(|range| {
                if range.size == 0 {
                    panic!("Invalid operation: push constant range size must be greater than 0");
                }
                if range.offset % 4 != 0 {
                    panic!("Invalid operation: push constant range offset must be a multiple of 4");
                }
                vk::VkPushConstantRange {
                    stageFlags: range.stage_flags.to_vk(),
                    offset: range.offset,
                    size: range.size,
                }
            })
            .collect();

        let pipeline_layout_info = vk::VkPipelineLayoutCreateInfo {
            sType: vk::VkStructureType::PIPELINE_LAYOUT_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            setLayoutCount: 1,
            pSetLayouts: &layout,
            pushConstantRangeCount: vk_push_constant_ranges.len() as u32,
            pPushConstantRanges: vk_push_constant_ranges.as_ptr(),
        };

        let mut pipeline_layout: vk::VkPipelineLayout = std::ptr::null_mut();
        vk_call!(vk::vkCreatePipelineLayout(
            vk_device,
            &pipeline_layout_info,
            std::ptr::null(),
            &mut pipeline_layout,
        ))?;

        Ok(pipeline_layout)
    }
}

pub struct Sampler {
    handle: vk::VkSampler,
    device: Arc<Device>,
}

impl Eq for Sampler {}
impl PartialEq for Sampler {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}
impl std::hash::Hash for Sampler {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.handle.hash(state);
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        unsafe {
            vk::vkDestroySampler(self.device.handle(), self.handle, std::ptr::null());
        }
    }
}

impl Sampler {
    pub fn handle(&self) -> vk::VkSampler {
        self.handle
    }

    pub fn new(
        device: Arc<Device>,
        filter: Filter,
        address_mode: SamplerAddressMode,
        border_color: SamplerBorderColor,
    ) -> Result<Self> {
        let sampler_create_info = vk::VkSamplerCreateInfo {
            sType: vk::VkStructureType::SAMPLER_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            magFilter: filter.to_vk(),
            minFilter: filter.to_vk(),
            mipmapMode: vk::VkSamplerMipmapMode::LINEAR,
            addressModeU: address_mode.to_vk(),
            addressModeV: address_mode.to_vk(),
            addressModeW: address_mode.to_vk(),
            mipLodBias: 0.0,
            anisotropyEnable: 0,
            maxAnisotropy: 1.0,
            compareEnable: 0,
            compareOp: vk::VkCompareOp::ALWAYS,
            minLod: 0.0,
            maxLod: 0.0,
            borderColor: border_color.to_vk(),
            unnormalizedCoordinates: 0,
        };

        let mut handle: vk::VkSampler = std::ptr::null_mut();
        vk_call!(vk::vkCreateSampler(
            device.handle(),
            &sampler_create_info,
            std::ptr::null(),
            &mut handle,
        ))
        .unwrap();

        Ok(Sampler { handle, device })
    }
}

pub struct PushConstantRange {
    stage_flags: Stage,
    offset: u32,
    size: u32,
}

pub struct Binding {
    binding: u32,
    stride: u32,
    rate: Rate,
}

pub struct Attribute {
    location: u32,
    binding: u32,
    format: Format,
    offset: u32,
}

pub enum ImageFit {
    None,
    Stretch,
    Center,
    Fill,
    FillAspect,
}

pub enum SamplerAddressMode {
    Repeat,
    MirroredRepeat,
    ClampToEdge,
    ClampToBorder,
    MirrorClampToEdge,
}

impl SamplerAddressMode {
    pub fn to_vk(&self) -> vk::VkSamplerAddressMode {
        match self {
            SamplerAddressMode::Repeat => vk::VkSamplerAddressMode::REPEAT,
            SamplerAddressMode::MirroredRepeat => vk::VkSamplerAddressMode::MIRRORED_REPEAT,
            SamplerAddressMode::ClampToEdge => vk::VkSamplerAddressMode::CLAMP_TO_EDGE,
            SamplerAddressMode::ClampToBorder => vk::VkSamplerAddressMode::CLAMP_TO_BORDER,
            SamplerAddressMode::MirrorClampToEdge => vk::VkSamplerAddressMode::MIRROR_CLAMP_TO_EDGE,
        }
    }
}

pub enum SamplerBorderColor {
    FloatTransparentBlack,
    IntTransparentBlack,
    FloatOpaqueBlack,
    IntOpaqueBlack,
    FloatOpaqueWhite,
    IntOpaqueWhite,
}

impl SamplerBorderColor {
    pub fn to_vk(&self) -> vk::VkBorderColor {
        match self {
            SamplerBorderColor::FloatTransparentBlack => vk::VkBorderColor::FLOAT_TRANSPARENT_BLACK,
            SamplerBorderColor::IntTransparentBlack => vk::VkBorderColor::INT_TRANSPARENT_BLACK,
            SamplerBorderColor::FloatOpaqueBlack => vk::VkBorderColor::FLOAT_OPAQUE_BLACK,
            SamplerBorderColor::IntOpaqueBlack => vk::VkBorderColor::INT_OPAQUE_BLACK,
            SamplerBorderColor::FloatOpaqueWhite => vk::VkBorderColor::FLOAT_OPAQUE_WHITE,
            SamplerBorderColor::IntOpaqueWhite => vk::VkBorderColor::INT_OPAQUE_WHITE,
        }
    }
}

pub enum Filter {
    Nearest,
    Linear,
}

impl Filter {
    pub fn to_vk(&self) -> vk::VkFilter {
        match self {
            Filter::Nearest => vk::VkFilter::NEAREST,
            Filter::Linear => vk::VkFilter::LINEAR,
        }
    }
}

impl SamplerMipmapMode {
    pub fn to_vk(&self) -> vk::VkSamplerMipmapMode {
        match self {
            SamplerMipmapMode::Nearest => vk::VkSamplerMipmapMode::NEAREST,
            SamplerMipmapMode::Linear => vk::VkSamplerMipmapMode::LINEAR,
        }
    }
}

pub enum SamplerMipmapMode {
    Nearest,
    Linear,
}

pub enum DescriptorType {
    Sampler,
    SampledImage,
    StorageImage,
    UniformBuffer,
    StorageBuffer,
    InputAttachment,
}

pub struct Descriptor {
    binding: u32,
    descriptor_type: DescriptorType,
    descriptor_count: u32,
    stage: Stage,
}

impl Descriptor {
    pub fn new(binding: u32, descriptor_type: DescriptorType, stage: Stage) -> Self {
        Descriptor {
            binding,
            descriptor_type,
            descriptor_count: 1,
            stage,
        }
    }
}

impl DescriptorType {
    pub fn to_vk(&self) -> vk::VkDescriptorType {
        match self {
            &DescriptorType::Sampler => vk::VkDescriptorType::SAMPLER,
            &DescriptorType::SampledImage => vk::VkDescriptorType::SAMPLED_IMAGE,
            &DescriptorType::StorageImage => vk::VkDescriptorType::STORAGE_IMAGE,
            &DescriptorType::UniformBuffer => vk::VkDescriptorType::UNIFORM_BUFFER,
            &DescriptorType::StorageBuffer => vk::VkDescriptorType::STORAGE_BUFFER,
            &DescriptorType::InputAttachment => vk::VkDescriptorType::INPUT_ATTACHMENT,
        }
    }
}

pub struct Attachment {
    format: Format,
    blend: AttachmentBlend,
}

impl Attachment {
    pub fn new(format: Format, blend: AttachmentBlend) -> Self {
        Attachment { format, blend }
    }
}

pub struct AttachmentBlend {
    blend_enable: u32,
    src_color_blend_factor: BlendFactor,
    dst_color_blend_factor: BlendFactor,
    color_blend_op: BlendOp,
    src_alpha_blend_factor: BlendFactor,
    dst_alpha_blend_factor: BlendFactor,
    alpha_blend_op: BlendOp,
    color_write_mask: ColorComponent,
}

impl AttachmentBlend {
    pub fn to_vk(&self) -> vk::VkPipelineColorBlendAttachmentState {
        vk::VkPipelineColorBlendAttachmentState {
            blend_enable: self.blend_enable,
            src_color_blend_factor: self.src_color_blend_factor.to_vk(),
            dst_color_blend_factor: self.dst_color_blend_factor.to_vk(),
            color_blend_op: self.color_blend_op.to_vk(),
            src_alpha_blend_factor: self.src_alpha_blend_factor.to_vk(),
            dst_alpha_blend_factor: self.dst_alpha_blend_factor.to_vk(),
            alpha_blend_op: self.alpha_blend_op.to_vk(),
            color_write_mask: self.color_write_mask.to_vk(),
        }
    }

    pub fn straight_alpha_blend() -> Self {
        Self {
            blend_enable: 1,
            src_color_blend_factor: BlendFactor::One,
            dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::OneMinusSrcAlpha,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorComponent::RGBA,
        }
    }
}

pub enum ColorComponent {
    R,
    G,
    B,
    A,
    RGBA,
}

impl ColorComponent {
    pub fn to_vk(&self) -> vk::VkColorComponentFlags {
        match self {
            ColorComponent::R => vk::VkColorComponentFlags::R_BIT,
            ColorComponent::G => vk::VkColorComponentFlags::G_BIT,
            ColorComponent::B => vk::VkColorComponentFlags::B_BIT,
            ColorComponent::A => vk::VkColorComponentFlags::A_BIT,
            ColorComponent::RGBA => {
                vk::VkColorComponentFlags::R_BIT
                    | vk::VkColorComponentFlags::G_BIT
                    | vk::VkColorComponentFlags::B_BIT
                    | vk::VkColorComponentFlags::A_BIT
            }
        }
    }
}

pub enum BlendFactor {
    Zero,
    One,
    SrcColor,
    OneMinusSrcColor,
    DstColor,
    OneMinusDstColor,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstAlpha,
    OneMinusDstAlpha,
    ConstantColor,
    OneMinusConstantColor,
    ConstantAlpha,
    OneMinusConstantAlpha,
    SrcAlphaSaturate,
    Src1Color,
    OneMinusSrc1Color,
    Src1Alpha,
    OneMinusSrc1Alpha,
}

impl BlendFactor {
    pub fn to_vk(&self) -> vk::VkBlendFactor {
        match self {
            &BlendFactor::Zero => vk::VkBlendFactor::ZERO,
            &BlendFactor::One => vk::VkBlendFactor::ONE,
            &BlendFactor::SrcColor => vk::VkBlendFactor::SRC_COLOR,
            &BlendFactor::OneMinusSrcColor => vk::VkBlendFactor::ONE_MINUS_SRC_COLOR,
            &BlendFactor::DstColor => vk::VkBlendFactor::DST_COLOR,
            &BlendFactor::OneMinusDstColor => vk::VkBlendFactor::ONE_MINUS_DST_COLOR,
            &BlendFactor::SrcAlpha => vk::VkBlendFactor::SRC_ALPHA,
            &BlendFactor::OneMinusSrcAlpha => vk::VkBlendFactor::ONE_MINUS_SRC_ALPHA,
            &BlendFactor::DstAlpha => vk::VkBlendFactor::DST_ALPHA,
            &BlendFactor::OneMinusDstAlpha => vk::VkBlendFactor::ONE_MINUS_DST_ALPHA,
            &BlendFactor::ConstantColor => vk::VkBlendFactor::CONSTANT_COLOR,
            &BlendFactor::OneMinusConstantColor => vk::VkBlendFactor::ONE_MINUS_CONSTANT_COLOR,
            &BlendFactor::ConstantAlpha => vk::VkBlendFactor::CONSTANT_ALPHA,
            &BlendFactor::OneMinusConstantAlpha => vk::VkBlendFactor::ONE_MINUS_CONSTANT_ALPHA,
            &BlendFactor::SrcAlphaSaturate => vk::VkBlendFactor::SRC_ALPHA_SATURATE,
            &BlendFactor::Src1Color => vk::VkBlendFactor::SRC1_COLOR,
            &BlendFactor::OneMinusSrc1Color => vk::VkBlendFactor::ONE_MINUS_SRC1_COLOR,
            &BlendFactor::Src1Alpha => vk::VkBlendFactor::SRC1_ALPHA,
            &BlendFactor::OneMinusSrc1Alpha => vk::VkBlendFactor::ONE_MINUS_SRC1_ALPHA,
        }
    }
}

pub enum BlendOp {
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

impl BlendOp {
    pub fn to_vk(&self) -> vk::VkBlendOp {
        match self {
            BlendOp::Add => vk::VkBlendOp::ADD,
            BlendOp::Subtract => vk::VkBlendOp::SUBTRACT,
            BlendOp::ReverseSubtract => vk::VkBlendOp::REVERSE_SUBTRACT,
            BlendOp::Min => vk::VkBlendOp::MIN,
            BlendOp::Max => vk::VkBlendOp::MAX,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rate {
    Vertex,
    Instance,
}

impl Rate {
    pub fn to_vk(&self) -> vk::VkVertexInputRate {
        match self {
            Rate::Vertex => vk::VkVertexInputRate::VERTEX,
            Rate::Instance => vk::VkVertexInputRate::INSTANCE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    Never,
    Less,
    Equal,
    LessOrEqual,
    Greater,
    NotEqual,
    GreaterOrEqual,
    Always,
}

impl Comparison {
    pub fn to_vk(&self) -> vk::VkCompareOp {
        match self {
            Comparison::Never => vk::VkCompareOp::NEVER,
            Comparison::Less => vk::VkCompareOp::LESS,
            Comparison::Equal => vk::VkCompareOp::EQUAL,
            Comparison::LessOrEqual => vk::VkCompareOp::LESS_OR_EQUAL,
            Comparison::Greater => vk::VkCompareOp::GREATER,
            Comparison::NotEqual => vk::VkCompareOp::NOT_EQUAL,
            Comparison::GreaterOrEqual => vk::VkCompareOp::GREATER_OR_EQUAL,
            Comparison::Always => vk::VkCompareOp::ALWAYS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topology {
    Points,
    Lines,
    Triangles,
}

impl Topology {
    pub fn to_vk(&self) -> vk::VkPrimitiveTopology {
        match self {
            Topology::Points => vk::VkPrimitiveTopology::POINT_LIST,
            Topology::Lines => vk::VkPrimitiveTopology::LINE_LIST,
            Topology::Triangles => vk::VkPrimitiveTopology::TRIANGLE_LIST,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Winding {
    CounterClockwise,
    Clockwise,
}

impl Winding {
    pub fn to_vk(&self) -> vk::VkFrontFace {
        match self {
            Winding::CounterClockwise => vk::VkFrontFace::COUNTER_CLOCKWISE,
            Winding::Clockwise => vk::VkFrontFace::CLOCKWISE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Culling {
    None,
    Front,
    Back,
    FrontAndBack,
}

impl Culling {
    pub fn to_vk(&self) -> vk::VkCullModeFlags {
        match self {
            Culling::None => vk::VkCullModeFlags::NONE,
            Culling::Front => vk::VkCullModeFlags::FRONT_BIT,
            Culling::Back => vk::VkCullModeFlags::BACK_BIT,
            Culling::FrontAndBack => vk::VkCullModeFlags::FRONT_AND_BACK,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Raster {
    Fill,
    Line,
    Point,
}

impl Raster {
    pub fn to_vk(&self) -> vk::VkPolygonMode {
        match self {
            Raster::Fill => vk::VkPolygonMode::FILL,
            Raster::Line => vk::VkPolygonMode::LINE,
            Raster::Point => vk::VkPolygonMode::POINT,
        }
    }
}
