#![allow(dead_code)]

mod input;
pub use input::*;

mod core;
pub use core::*;
pub mod api;
pub mod dom;
pub mod style;

use crate::gfx::{self, presentation, vulkan as vk};
use crate::{Color, debug};

use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct Style {
    pub layout: LayoutStyle,
    pub size: Size,
    pub min_size: Size,
    pub max_size: Size,
    pub position: PositionMode,
    pub x: f32,
    pub y: f32,
    pub padding: EdgeSizes,
    pub margin: EdgeSizes,
    pub foreground: Color,
    pub background: Color,
    pub border_color: Color,
    pub border: EdgeSizes,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::default(),
            size: Size::fill(),
            min_size: Size::auto(),
            max_size: Size::auto(),
            position: PositionMode::Flow,
            x: 0.0,
            y: 0.0,
            padding: EdgeSizes::zero(),
            margin: EdgeSizes::zero(),
            foreground: Color::from_u32(0xFFFF_FFFF),
            background: Color::transparent(),
            border_color: Color::from_u32(0xFFFF_FFFF),
            border: EdgeSizes::zero(),
        }
    }
}

impl Style {
    pub fn apply_override(&mut self, override_style: &StyleOverride) {
        if let Some(layout) = override_style.layout {
            self.layout = layout;
        }
        if let Some(size) = override_style.size {
            self.size = size;
        }
        if let Some(min_size) = override_style.min_size {
            self.min_size = min_size;
        }
        if let Some(max_size) = override_style.max_size {
            self.max_size = max_size;
        }
        if let Some(position) = override_style.position_mode {
            self.position = position;
        }
        if let Some(x) = override_style.x {
            self.x = x;
        }
        if let Some(y) = override_style.y {
            self.y = y;
        }
        if let Some(padding) = override_style.padding {
            self.padding = padding;
        }
        if let Some(margin) = override_style.margin {
            self.margin = margin;
        }
        if let Some(foreground) = override_style.foreground {
            self.foreground = foreground;
        }
        if let Some(background) = override_style.background {
            self.background = background;
        }
        if let Some(border_color) = override_style.border_color {
            self.border_color = border_color;
        }
        if let Some(border) = override_style.border {
            self.border = border;
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StyleOverride {
    pub layout: Option<LayoutStyle>,
    pub size: Option<Size>,
    pub min_size: Option<Size>,
    pub max_size: Option<Size>,
    pub position_mode: Option<PositionMode>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub padding: Option<EdgeSizes>,
    pub margin: Option<EdgeSizes>,
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    pub border: Option<EdgeSizes>,
}

impl StyleOverride {
    pub fn merge_from(&mut self, other: &StyleOverride) {
        if other.layout.is_some() {
            self.layout = other.layout;
        }
        if other.size.is_some() {
            self.size = other.size;
        }
        if other.min_size.is_some() {
            self.min_size = other.min_size;
        }
        if other.max_size.is_some() {
            self.max_size = other.max_size;
        }
        if other.position_mode.is_some() {
            self.position_mode = other.position_mode;
        }
        if other.x.is_some() {
            self.x = other.x;
        }
        if other.y.is_some() {
            self.y = other.y;
        }
        if other.padding.is_some() {
            self.padding = other.padding;
        }
        if other.margin.is_some() {
            self.margin = other.margin;
        }
        if other.foreground.is_some() {
            self.foreground = other.foreground;
        }
        if other.background.is_some() {
            self.background = other.background;
        }
        if other.border_color.is_some() {
            self.border_color = other.border_color;
        }
        if other.border.is_some() {
            self.border = other.border;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LayoutStyle {
    pub direction: LayoutDirection,
    pub align: Align,
    pub justify: Justify,
    pub gap: f32,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            direction: LayoutDirection::Column,
            align: Align::Stretch,
            justify: Justify::Start,
            gap: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EdgeSizes {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl EdgeSizes {
    pub fn zero() -> Self {
        Self {
            left: 0.0,
            right: 0.0,
            top: 0.0,
            bottom: 0.0,
        }
    }

    pub fn add(self, other: Self) -> Self {
        Self {
            left: self.left + other.left,
            right: self.right + other.right,
            top: self.top + other.top,
            bottom: self.bottom + other.bottom,
        }
    }

    pub fn horizontal(self) -> f32 {
        self.left + self.right
    }

    pub fn vertical(self) -> f32 {
        self.top + self.bottom
    }
}

impl Default for EdgeSizes {
    fn default() -> Self {
        Self::zero()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Size {
    pub width: Length,
    pub height: Length,
}

impl Size {
    pub fn fill() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    pub fn auto() -> Self {
        Self {
            width: Length::Auto,
            height: Length::Auto,
        }
    }
}

impl Default for Size {
    fn default() -> Self {
        Self::fill()
    }
}

/// Direction for layout flow and sizing.
///
/// The direction sets the main axis (flow direction). The cross axis is the
/// perpendicular axis used for alignment and stretching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutDirection {
    /// Lay out children horizontally; main axis is left-to-right.
    Row,
    /// Lay out children vertically; main axis is top-to-bottom.
    Column,
}

/// Cross-axis alignment of children inside a layout.
///
/// The cross axis is perpendicular to the main axis: for `Row` it is vertical,
/// and for `Column` it is horizontal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// Align to the start edge of the cross axis.
    Start,
    /// Center along the cross axis.
    Center,
    /// Align to the end edge of the cross axis.
    End,
    /// Stretch to fill the available cross-axis space.
    Stretch,
}

/// Main-axis distribution of children inside a layout.
///
/// The main axis is the flow direction: `Row` uses the horizontal axis, and
/// `Column` uses the vertical axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Justify {
    /// Pack toward the start edge of the main axis.
    Start,
    /// Center along the main axis.
    Center,
    /// Pack toward the end edge of the main axis.
    End,
    /// Evenly distribute extra space between children.
    SpaceBetween,
    /// Distribute extra space around children (half space at edges).
    SpaceAround,
    /// Distribute extra space evenly, including edges.
    SpaceEvenly,
}

/// Length representation for sizes and constraints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    /// Use the content's intrinsic size.
    Auto,
    /// Fixed size in pixels.
    Px(f32),
    /// Percentage of the available parent size.
    Percent(f32),
    /// Take remaining space after fixed sizes and gaps.
    Fill,
}

/// How a node participates in layout positioning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionMode {
    /// Participate in flow layout with siblings.
    Flow,
    /// Positioned relative to the parent using anchors.
    Absolute,
}
