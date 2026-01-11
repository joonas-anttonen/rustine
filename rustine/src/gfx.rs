#![allow(dead_code)]

mod allocator;
mod core;
pub use core::DeviceSelector;
mod instance;
pub use instance::Instance;
mod device;
pub use device::*;
pub mod vulkan;
pub use core::Core;
use std::collections;
use std::sync;
pub mod presentation;
pub mod queue;
pub use presentation::AcquireStatus;
pub use queue::Queue;
pub use queue::SubmitStatus;
pub mod command;
pub use command::CommandBuffer;
pub use command::CommandPool;
pub mod buffer;
pub use buffer::MemoryBuffer;
pub use buffer::PixelBuffer;
pub mod compiler;
pub use compiler::*;
pub mod pipeline;

pub mod fonts;

use crate::*;

use crate::version::Version;

pub const MINIMUM_VULKAN_API_VERSION: Version = Version::new(1, 4, 0);

pub struct Image {
    pub width: u32,
    pub height: u32,
    pub id: u32,
    released_images: sync::Weak<sync::Mutex<collections::VecDeque<u32>>>,
}

impl Drop for Image {
    fn drop(&mut self) {
        if let Some(mailbox) = self.released_images.upgrade() {
            if let Ok(mut queue) = mailbox.lock() {
                queue.push_back(self.id);
            }
        }
    }
}

impl Default for Image {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            id: u32::MAX,
            released_images: sync::Weak::new(),
        }
    }
}

impl Image {
    pub(crate) fn new(
        id: u32,
        width: u32,
        height: u32,
        released_images: sync::Weak<sync::Mutex<collections::VecDeque<u32>>>,
    ) -> Self {
        Self {
            width,
            height,
            id,
            released_images,
        }
    }

    /// It is always safe to add an image to the released images queue.
    /// If the image is used after this, any resources will get reallocated.
    pub fn soft_drop(&mut self) {
        if let Some(mailbox) = self.released_images.upgrade() {
            if let Ok(mut queue) = mailbox.lock() {
                queue.push_back(self.id);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum Fit {
    NONE,
    /// Fits the content such that it fills the container, preserving aspect ratio.
    ///
    /// Will crop the content if necessary.
    FILL_KEEP_ASPECT,
    /// Fits the content such that it is fully visible within the container, preserving aspect ratio.
    FIT_KEEP_ASPECT,
    CENTER,
}

#[derive(Debug, Clone)]
pub struct Rectangle {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rectangle {
    pub fn position(&self) -> Vector2f {
        Vector2f::new(self.x, self.y)
    }

    pub fn extent(&self) -> Vector2f {
        Vector2f::new(self.w, self.h)
    }

    pub fn left(&self) -> f32 {
        self.x
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn top(&self) -> f32 {
        self.y
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
}

/// A single draw command: draw a range of indices from the vertex/index buffer with optional scissor.
#[derive(Debug, Clone)]
pub struct DrawCommand {
    /// Starting index in the index buffer.
    pub index_offset: u32,
    /// Number of indices to draw.
    pub index_count: u32,
    /// Optional image/texture ID to bind.
    pub image_id: Option<u32>,
    pub image_fallback_id: Option<u32>,
    /// Scissor rectangle for this draw; None means fullscreen.
    pub scissor: Option<Rectangle>,
}

impl DrawCommand {
    pub fn new(
        index_offset: u32,
        index_count: u32,
        image_id: Option<u32>,
        image_fallback_id: Option<u32>,
    ) -> Self {
        Self {
            index_offset,
            index_count,
            image_id,
            image_fallback_id,
            scissor: None,
        }
    }

    pub fn with_scissor(mut self, scissor: Rectangle) -> Self {
        self.scissor = Some(scissor);
        self
    }
}

/// A batch of draw commands sharing common state (all use same vertex/index buffers and pipeline).
#[derive(Debug, Clone)]
pub struct DrawBatch {
    pub commands: Vec<DrawCommand>,
}

impl DrawBatch {
    pub fn new() -> Self {
        Self {
            commands: Vec::with_capacity(256),
        }
    }

    pub fn push_command(&mut self, cmd: DrawCommand) {
        self.commands.push(cmd);
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct GpuVertex {
    pub position: Vector2f,
    pub texture: Vector2f,
    pub color: u32,
}

/// Descriptor for an image quad that requires dynamic fitting based on actual pixel buffer size.
#[derive(Debug, Clone)]
pub struct ImageDescriptor {
    pub image_id: u32,
    pub fallback_id: Option<u32>,
    pub layout: Rectangle,
    pub fit: Fit,
    pub color: u32,
    /// Index into vertices where this quad starts (4 vertices per quad).
    pub vertex_offset: u32,
}

/// Computes fitted quad positions and UVs based on image dimensions, layout, and fit mode.
/// Returns (positions, uvs) where positions are the 4 corners and uvs are texture coordinates.
fn compute_fit(
    img_w: f32,
    img_h: f32,
    layout: &Rectangle,
    fit: Fit,
) -> ([Vector2f; 4], [Vector2f; 4]) {
    let layout_w = layout.w;
    let layout_h = layout.h;

    let mut pos_origin = Vector2f::new(layout.x, layout.y);
    let mut size = Vector2f::new(layout_w, layout_h);
    let mut uv_min = Vector2f::new(0.0, 0.0);
    let mut uv_max = Vector2f::new(1.0, 1.0);

    match fit {
        Fit::NONE => {}
        Fit::CENTER => {
            size = Vector2f::new(img_w, img_h);
            pos_origin.x += (layout_w - size.x) * 0.5;
            pos_origin.y += (layout_h - size.y) * 0.5;
        }
        Fit::FIT_KEEP_ASPECT => {
            let scale = (layout_w / img_w).min(layout_h / img_h);
            size = Vector2f::new(img_w * scale, img_h * scale);
            pos_origin.x += (layout_w - size.x) * 0.5;
            pos_origin.y += (layout_h - size.y) * 0.5;
        }
        Fit::FILL_KEEP_ASPECT => {
            let scale = (layout_w / img_w).max(layout_h / img_h);
            let scaled_w = img_w * scale;
            let scaled_h = img_h * scale;

            let excess_w = (scaled_w - layout_w).max(0.0) * 0.5;
            let excess_h = (scaled_h - layout_h).max(0.0) * 0.5;

            uv_min.x = (excess_w / scaled_w).clamp(0.0, 0.5);
            uv_max.x = 1.0 - uv_min.x;
            uv_min.y = (excess_h / scaled_h).clamp(0.0, 0.5);
            uv_max.y = 1.0 - uv_min.y;
        }
    }

    let positions = [
        pos_origin,
        Vector2f::new(pos_origin.x + size.x, pos_origin.y),
        Vector2f::new(pos_origin.x + size.x, pos_origin.y + size.y),
        Vector2f::new(pos_origin.x, pos_origin.y + size.y),
    ];

    let uvs = [
        Vector2f::new(uv_min.x, uv_min.y),
        Vector2f::new(uv_max.x, uv_min.y),
        Vector2f::new(uv_max.x, uv_max.y),
        Vector2f::new(uv_min.x, uv_max.y),
    ];

    (positions, uvs)
}

/// Pre-computed render frame: all vertices and indices are pre-built by UI thread.
#[derive(Clone)]
pub struct RenderFrame {
    pub vertices: Vec<GpuVertex>,
    pub indices: Vec<u32>,
    pub batches: Vec<DrawBatch>,
    /// Descriptors for quads that need dynamic refitting based on pixel buffer dimensions.
    pub image_descriptors: Vec<ImageDescriptor>,
}

impl RenderFrame {
    pub fn new() -> Self {
        Self {
            vertices: Vec::with_capacity(65536),
            indices: Vec::with_capacity(65536),
            batches: Vec::with_capacity(16),
            image_descriptors: Vec::with_capacity(64),
        }
    }

    pub fn push_batch(&mut self, batch: DrawBatch) {
        self.batches.push(batch);
    }

    pub fn push_quad(
        &mut self,
        positions: [Vector2f; 4],
        uvs: [Vector2f; 4],
        color: u32,
        image_id: Option<u32>,
        image_fallback_id: Option<u32>,
    ) {
        let vertex_base = self.vertices.len() as u32;
        let index_offset = self.indices.len() as u32;

        self.vertices.extend_from_slice(&[
            GpuVertex {
                position: positions[0],
                texture: uvs[0],
                color,
            },
            GpuVertex {
                position: positions[1],
                texture: uvs[1],
                color,
            },
            GpuVertex {
                position: positions[2],
                texture: uvs[2],
                color,
            },
            GpuVertex {
                position: positions[3],
                texture: uvs[3],
                color,
            },
        ]);

        self.indices.extend_from_slice(&[
            vertex_base,
            vertex_base + 1,
            vertex_base + 2,
            vertex_base,
            vertex_base + 2,
            vertex_base + 3,
        ]);

        if self.batches.is_empty() {
            self.batches.push(DrawBatch::new());
        }

        let batch = self.batches.last_mut().expect("batch exists");

        let can_merge = batch
            .commands
            .last()
            .map(|cmd| cmd.image_id == image_id && cmd.image_fallback_id == image_fallback_id)
            .unwrap_or(false);

        if can_merge {
            if let Some(cmd) = batch.commands.last_mut() {
                cmd.index_count += 6;
            }
        } else {
            batch.push_command(DrawCommand::new(
                index_offset,
                6,
                image_id,
                image_fallback_id,
            ));
        }
    }

    pub fn push_image(
        &mut self,
        image: &Image,
        fallback_image: Option<&Image>,
        layout: Rectangle,
        fit: Fit,
        color: u32,
    ) {
        if layout.w <= 0.0 || layout.h <= 0.0 {
            return;
        }

        let vertex_offset = self.vertices.len() as u32;

        // Store descriptor for dynamic fitting during render
        self.image_descriptors.push(ImageDescriptor {
            image_id: image.id,
            fallback_id: fallback_image.map(|f| f.id),
            layout: layout.clone(),
            fit,
            color,
            vertex_offset,
        });

        let img_w = image.width.max(1) as f32;
        let img_h = image.height.max(1) as f32;

        let (positions, uvs) = compute_fit(img_w, img_h, &layout, Fit::NONE); // No fitting during push_image
        let fallback_id = fallback_image.map(|f| f.id);

        self.push_quad(positions, uvs, color, Some(image.id), fallback_id);
    }

    /// Render text using the embedded bitmap fonts.
    ///
    /// Returns the final area occupied by the text.
    pub fn push_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        scale: f32,
        color: u32,
        font_id: u32,
    ) -> Rectangle {
        let font_size = fonts::get_font_size(font_id);
        let mut cursor_x = x;
        let mut cursor_y = y;
        let mut min_x = x;
        let mut max_x = x;
        let mut min_y = y;
        let mut max_y = y;

        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = x;
                cursor_y += font_size * scale;
                continue;
            }

            if let Some(metrics) = fonts::get_glyph_metrics(font_id, ch) {
                let glyph_w = metrics.width as f32;
                let glyph_h = metrics.height as f32;

                let u0 = metrics.u0;
                let v0 = metrics.v0;
                let u1 = metrics.u1;
                let v1 = metrics.v1;

                // offset_y is the distance from baseline to top of glyph (negative means above baseline)
                // We want to position glyphs so cursor_y is the baseline
                let x0 = cursor_x + metrics.offset_x as f32 * scale;
                let y1 = cursor_y - metrics.offset_y as f32 * scale;
                let x1 = x0 + glyph_w * scale;
                let y0 = y1 - glyph_h * scale;

                min_x = min_x.min(x0);
                max_x = max_x.max(x1);
                min_y = min_y.min(y0);
                max_y = max_y.max(y1);

                let positions = [
                    Vector2f::new(x0, y0),
                    Vector2f::new(x1, y0),
                    Vector2f::new(x1, y1),
                    Vector2f::new(x0, y1),
                ];

                let uvs = [
                    Vector2f::new(u0, v0),
                    Vector2f::new(u1, v0),
                    Vector2f::new(u1, v1),
                    Vector2f::new(u0, v1),
                ];

                self.push_quad(positions, uvs, color, Some(font_id), None);

                cursor_x += metrics.advance_width as f32 * scale;
            } else {
                // Invalid character, take the first character in the font and push_quad filling that space
                if let Some(all_metrics) = fonts::get_all_metrics(font_id) {
                    if let Some((_, first_metrics)) = all_metrics.first() {
                        let glyph_w = first_metrics.width as f32;
                        let glyph_h = first_metrics.height as f32;

                        let x0 = cursor_x + first_metrics.offset_x as f32 * scale;
                        let y1 = cursor_y - first_metrics.offset_y as f32 * scale;
                        let x1 = x0 + glyph_w * scale;
                        let y0 = y1 - glyph_h * scale;

                        min_x = min_x.min(x0);
                        max_x = max_x.max(x1);
                        min_y = min_y.min(y0);
                        max_y = max_y.max(y1);

                        let padding = 1.0 * scale;
                        let positions = [
                            Vector2f::new(x0 + padding, y0 + padding),
                            Vector2f::new(x1 - padding, y0 + padding),
                            Vector2f::new(x1 - padding, y1 - padding),
                            Vector2f::new(x0 + padding, y1 - padding),
                        ];

                        let uvs = [
                            Vector2f::new(0.0, 0.0),
                            Vector2f::new(0.0, 0.0),
                            Vector2f::new(0.0, 0.0),
                            Vector2f::new(0.0, 0.0),
                        ];

                        let error_color = 0xFF0000FFu32;
                        self.push_quad(positions, uvs, error_color, None, None);

                        cursor_x += first_metrics.advance_width as f32 * scale;
                    }
                }
            }
        }

        max_x = max_x.max(cursor_x);

        Rectangle {
            x: min_x,
            y: min_y,
            w: max_x - min_x,
            h: max_y - min_y,
        }
    }

    /// Draw a filled rectangle with the given color.
    ///
    /// The rectangle is defined by its outer corners. Draws a single quad covering the entire area.
    pub fn fill_rectangle(&mut self, rect: &Rectangle, color: u32) {
        if rect.w <= 0.0 || rect.h <= 0.0 {
            return;
        }

        let positions = [
            Vector2f::new(rect.x, rect.y),
            Vector2f::new(rect.x + rect.w, rect.y),
            Vector2f::new(rect.x + rect.w, rect.y + rect.h),
            Vector2f::new(rect.x, rect.y + rect.h),
        ];

        let uvs = [
            Vector2f::new(0.0, 0.0),
            Vector2f::new(0.0, 0.0),
            Vector2f::new(0.0, 0.0),
            Vector2f::new(0.0, 0.0),
        ];

        self.push_quad(positions, uvs, color, None, None);
    }

    /// Draw a rectangle outline with the given color and thickness.
    ///
    /// The rectangle is defined by its outer corners. The thickness grows inward, meaning the input
    /// rectangle defines the outer boundary and the stroke is drawn on the interior.
    ///
    /// The rectangle is decomposed into 4 quads representing the edges:
    /// - Top edge covers both top corners
    /// - Bottom edge covers both bottom corners
    /// - Left and right edges fill the space between top and bottom
    ///
    /// This ensures no overlap at corners, which is important when using transparent colors.
    pub fn draw_rectangle(&mut self, rect: &Rectangle, thickness: f32, color: u32) {
        if rect.w <= 0.0 || rect.h <= 0.0 || thickness <= 0.0 {
            return;
        }

        let thickness = thickness.min(rect.w / 2.0).min(rect.h / 2.0);

        let x0 = rect.x;
        let y0 = rect.y;
        let x1 = rect.x + rect.w;
        let y1 = rect.y + rect.h;
        let xi0 = x0 + thickness;
        let yi0 = y0 + thickness;
        let xi1 = x1 - thickness;
        let yi1 = y1 - thickness;

        let uvs = [
            Vector2f::new(0.0, 0.0),
            Vector2f::new(0.0, 0.0),
            Vector2f::new(0.0, 0.0),
            Vector2f::new(0.0, 0.0),
        ];

        // Top edge (covers both top corners)
        let top_positions = [
            Vector2f::new(x0, y0),
            Vector2f::new(x1, y0),
            Vector2f::new(x1, yi0),
            Vector2f::new(x0, yi0),
        ];
        self.push_quad(top_positions, uvs, color, None, None);

        // Bottom edge (covers both bottom corners)
        let bottom_positions = [
            Vector2f::new(x0, yi1),
            Vector2f::new(x1, yi1),
            Vector2f::new(x1, y1),
            Vector2f::new(x0, y1),
        ];
        self.push_quad(bottom_positions, uvs, color, None, None);

        // Left edge (between top and bottom)
        let left_positions = [
            Vector2f::new(x0, yi0),
            Vector2f::new(xi0, yi0),
            Vector2f::new(xi0, yi1),
            Vector2f::new(x0, yi1),
        ];
        self.push_quad(left_positions, uvs, color, None, None);

        // Right edge (between top and bottom)
        let right_positions = [
            Vector2f::new(xi1, yi0),
            Vector2f::new(x1, yi0),
            Vector2f::new(x1, yi1),
            Vector2f::new(xi1, yi1),
        ];
        self.push_quad(right_positions, uvs, color, None, None);
    }
}

#[derive(Debug)]
pub struct Extent2D {
    pub width: u32,
    pub height: u32,
}

use vulkan as vk;

#[allow(non_snake_case, non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    U32,
    R32G32_SFLOAT,
    R32G32B32_SFLOAT,
    R8G8B8A8_UNORM,
    B8G8R8A8_UNORM,
    D32_SFLOAT,
    D24_UNORM_S8_UINT,
    D32_SFLOAT_S8_UINT,
}
impl Format {
    pub fn to_vk(&self) -> vk::VkFormat {
        match self {
            Format::U32 => vk::VkFormat::R32_UINT,
            Format::R32G32_SFLOAT => vk::VkFormat::R32G32_SFLOAT,
            Format::R32G32B32_SFLOAT => vk::VkFormat::R32G32B32_SFLOAT,
            Format::R8G8B8A8_UNORM => vk::VkFormat::R8G8B8A8_UNORM,
            Format::B8G8R8A8_UNORM => vk::VkFormat::B8G8R8A8_UNORM,
            Format::D32_SFLOAT => vk::VkFormat::D32_SFLOAT,
            Format::D24_UNORM_S8_UINT => vk::VkFormat::D24_UNORM_S8_UINT,
            Format::D32_SFLOAT_S8_UINT => vk::VkFormat::D32_SFLOAT_S8_UINT,
        }
    }
    pub fn from_vk(format: vk::VkFormat) -> Self {
        match format {
            vk::VkFormat::R32_UINT => Format::U32,
            vk::VkFormat::R32G32_SFLOAT => Format::R32G32_SFLOAT,
            vk::VkFormat::R32G32B32_SFLOAT => Format::R32G32B32_SFLOAT,
            vk::VkFormat::R8G8B8A8_UNORM => Format::R8G8B8A8_UNORM,
            vk::VkFormat::B8G8R8A8_UNORM => Format::B8G8R8A8_UNORM,
            vk::VkFormat::D32_SFLOAT => Format::D32_SFLOAT,
            vk::VkFormat::D24_UNORM_S8_UINT => Format::D24_UNORM_S8_UINT,
            vk::VkFormat::D32_SFLOAT_S8_UINT => Format::D32_SFLOAT_S8_UINT,
            _ => panic!("Unsupported VkFormat: {:?}", format),
        }
    }
}

pub struct Layout(vk::VkImageLayout);
impl Copy for Layout {}
impl Clone for Layout {
    fn clone(&self) -> Self {
        *self
    }
}
impl PartialEq for Layout {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for Layout {}
impl Layout {
    /// Careful with this layout: when transitioning from UNDEFINED, the contents of the image are not guaranteed to be preserved.
    pub const UNDEFINED: Self = Self(vk::VkImageLayout::UNDEFINED);
    pub const GENERAL: Self = Self(vk::VkImageLayout::GENERAL);
    pub const COLOR_ATTACHMENT: Self = Self(vk::VkImageLayout::COLOR_ATTACHMENT_OPTIMAL);
    pub const DEPTH_ATTACHMENT: Self = Self(vk::VkImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
    pub const TRANSFER_SRC: Self = Self(vk::VkImageLayout::TRANSFER_SRC_OPTIMAL);
    pub const TRANSFER_DST: Self = Self(vk::VkImageLayout::TRANSFER_DST_OPTIMAL);
    pub const SHADER_READ_ONLY: Self = Self(vk::VkImageLayout::SHADER_READ_ONLY_OPTIMAL);
    pub const PRESENT_SRC_KHR: Self = Self(vk::VkImageLayout::PRESENT_SRC_KHR);

    pub fn to_vk(&self) -> vk::VkImageLayout {
        self.0
    }
    pub fn from_vk(layout: vk::VkImageLayout) -> Self {
        Self(layout)
    }
    /// Returns the raw u32 value for atomic storage.
    pub fn to_raw_u32(&self) -> u32 {
        // VkImageLayout is VkImageLayout(u32), so we need to reconstruct from the vk::VkImageLayout
        match self.0 {
            vk::VkImageLayout::UNDEFINED => 0,
            vk::VkImageLayout::GENERAL => 1,
            vk::VkImageLayout::COLOR_ATTACHMENT_OPTIMAL => 2,
            vk::VkImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => 3,
            vk::VkImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL => 4,
            vk::VkImageLayout::SHADER_READ_ONLY_OPTIMAL => 5,
            vk::VkImageLayout::TRANSFER_SRC_OPTIMAL => 6,
            vk::VkImageLayout::TRANSFER_DST_OPTIMAL => 7,
            _ => 0,
        }
    }
    /// Converts a raw u32 value back to Layout (from atomic storage).
    pub fn from_raw_u32(val: u32) -> Self {
        match val {
            0 => Self::UNDEFINED,
            1 => Self::GENERAL,
            2 => Self::COLOR_ATTACHMENT,
            3 => Self::DEPTH_ATTACHMENT,
            4 => Self(vk::VkImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL),
            5 => Self::SHADER_READ_ONLY,
            6 => Self::TRANSFER_SRC,
            7 => Self::TRANSFER_DST,
            _ => Self::UNDEFINED,
        }
    }
}

/// Represents the sample count flags for a pixel buffer (image).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Samples(u32);
impl Samples {
    pub const X1: Self = Self(vulkan::VkSampleCountFlags::X1_BIT as u32);
    pub const X2: Self = Self(vulkan::VkSampleCountFlags::X2_BIT as u32);
    pub const X4: Self = Self(vulkan::VkSampleCountFlags::X4_BIT as u32);
    pub const X8: Self = Self(vulkan::VkSampleCountFlags::X8_BIT as u32);
    pub const X16: Self = Self(vulkan::VkSampleCountFlags::X16_BIT as u32);
    pub const X32: Self = Self(vulkan::VkSampleCountFlags::X32_BIT as u32);
    pub const X64: Self = Self(vulkan::VkSampleCountFlags::X64_BIT as u32);
}
impl std::ops::BitOr for Samples {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Represents the aspect flags for a pixel buffer (image).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageAspect(u32);
impl ImageAspect {
    pub const COLOR: Self = Self(vulkan::VkImageAspectFlags::COLOR_BIT as u32);
    pub const DEPTH: Self = Self(vulkan::VkImageAspectFlags::DEPTH_BIT as u32);
    pub const STENCIL: Self = Self(vulkan::VkImageAspectFlags::STENCIL_BIT as u32);

    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}
impl std::ops::BitOr for ImageAspect {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Represents the usage flags for a pixel buffer (image).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageUsage(vulkan::VkImageUsageFlags);
impl ImageUsage {
    pub const TRANSFER_SRC: Self = Self(vulkan::VkImageUsageFlags::TRANSFER_SRC_BIT);
    pub const TRANSFER_DST: Self = Self(vulkan::VkImageUsageFlags::TRANSFER_DST_BIT);
    pub const SAMPLED: Self = Self(vulkan::VkImageUsageFlags::SAMPLED_BIT);
    pub const STORAGE: Self = Self(vulkan::VkImageUsageFlags::STORAGE_BIT);
    pub const COLOR_ATTACHMENT: Self = Self(vulkan::VkImageUsageFlags::COLOR_ATTACHMENT_BIT);
    pub const DEPTH_ATTACHMENT: Self =
        Self(vulkan::VkImageUsageFlags::DEPTH_STENCIL_ATTACHMENT_BIT);
}
impl std::ops::BitOr for ImageUsage {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Represents the result of a graphics operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Success,
    NotImplemented(i32),
    InvalidOperation(i32),
    NotSupported(i32),
    Timeout(i32),
    OutOfDate(i32),
    Unknown(i32),
}

impl std::error::Error for Status {}
impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Success => write!(f, "Success"),
            Status::InvalidOperation(code) => write!(f, "Invalid operation: {}", code),
            Status::NotSupported(code) => write!(f, "Not supported: {}", code),
            Status::NotImplemented(code) => write!(f, "Not implemented: {}", code),
            Status::Timeout(code) => write!(f, "Timeout: {}", code),
            Status::OutOfDate(code) => write!(f, "Out of date: {}", code),
            Status::Unknown(code) => write!(f, "Unknown error: {}", code),
        }
    }
}

impl Status {
    pub fn to_code(&self) -> i32 {
        match self {
            Status::Success => 0,
            Status::InvalidOperation(code) => *code,
            Status::NotImplemented(code) => *code,
            Status::NotSupported(code) => *code,
            Status::Timeout(code) => *code,
            Status::OutOfDate(code) => *code,
            Status::Unknown(code) => *code,
        }
    }

    pub fn from_code(code: i32) -> Self {
        match code {
            0 => Status::Success,
            -7 | -8 | -11 => Status::NotSupported(code), // Extension not present, feature not present, format not supported
            -4 => Status::InvalidOperation(code),        // Invalid operation
            -1 => Status::NotImplemented(code),          // Not implemented
            2 => Status::Timeout(code),                  // Timeout
            -1000001004 => Status::OutOfDate(code),      // Out of date
            other => Status::Unknown(other),
        }
    }
}

/// Represents the result of a graphics operation.
pub type Result<T> = std::result::Result<T, Status>;
