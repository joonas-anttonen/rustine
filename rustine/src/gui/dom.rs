#![allow(dead_code)]

use crate::{gfx, Color, Vector2f};

pub type NodeId = usize;

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
pub enum AlignItems {
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
pub enum JustifyContent {
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

#[derive(Debug, Clone)]
pub struct Dom {
    nodes: Vec<Node>,
    root: NodeId,
}

impl Dom {
    pub fn new() -> Self {
        let mut nodes = Vec::new();
        let root = Self::alloc_node(&mut nodes, NodeKind::Div(Div::default()));
        Self { nodes, root }
    }

    pub fn layout(&mut self, root_size: Vector2f) {
        let rect = LayoutRect {
            position: Vector2f::new(0.0, 0.0),
            size: root_size,
        };
        self.layout_node(self.root, rect);
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id)
    }

    pub fn create_div(&mut self) -> NodeId {
        Self::alloc_node(&mut self.nodes, NodeKind::Div(Div::default()))
    }

    pub fn create_text(&mut self, content: impl Into<String>, font_id: u32, scale: f32) -> NodeId {
        Self::alloc_node(
            &mut self.nodes,
            NodeKind::Text(Text::new(content, font_id, scale)),
        )
    }

    pub fn set_text(&mut self, id: NodeId, content: impl Into<String>) -> bool {
        let Some(node) = self.nodes.get_mut(id) else {
            return false;
        };
        let Some(text) = node.as_text_mut() else {
            return false;
        };
        text.content = content.into();
        true
    }

    pub fn set_text_font(&mut self, id: NodeId, font_id: u32) -> bool {
        let Some(node) = self.nodes.get_mut(id) else {
            return false;
        };
        let Some(text) = node.as_text_mut() else {
            return false;
        };
        text.font_id = font_id;
        true
    }

    pub fn set_text_scale(&mut self, id: NodeId, scale: f32) -> bool {
        let Some(node) = self.nodes.get_mut(id) else {
            return false;
        };
        let Some(text) = node.as_text_mut() else {
            return false;
        };
        text.scale = scale;
        true
    }

    pub fn append_child(&mut self, parent: NodeId, child: NodeId) -> bool {
        if parent == child {
            return false;
        }

        if self.nodes.get(parent).is_none() || self.nodes.get(child).is_none() {
            return false;
        }

        if let Some(old_parent) = self.nodes[child].parent {
            if let Some(old_parent_node) = self.nodes.get_mut(old_parent) {
                old_parent_node.children.retain(|id| *id != child);
            }
        }

        if let Some(parent_node) = self.nodes.get_mut(parent) {
            parent_node.children.push(child);
        }

        self.nodes[child].parent = Some(parent);
        true
    }

    pub fn set_content_size(&mut self, id: NodeId, size: Vector2f) -> bool {
        let Some(node) = self.nodes.get_mut(id) else {
            return false;
        };

        node.content_size = Vector2f::new(size.x.max(0.0), size.y.max(0.0));
        true
    }

    pub fn hit_test(&self, position: Vector2f) -> Option<NodeId> {
        self.hit_test_node(self.root, position)
    }

    fn alloc_node(nodes: &mut Vec<Node>, kind: NodeKind) -> NodeId {
        let id = nodes.len();
        nodes.push(Node {
            parent: None,
            children: Vec::new(),
            kind,
            layout: LayoutRect::default(),
            content_size: Vector2f::default(),
        });
        id
    }

    fn hit_test_node(&self, id: NodeId, position: Vector2f) -> Option<NodeId> {
        let node = self.nodes.get(id)?;
        if !node.layout.contains(position) {
            return None;
        }

        for child_id in node.children.iter().rev().copied() {
            if let Some(hit) = self.hit_test_node(child_id, position) {
                return Some(hit);
            }
        }

        Some(id)
    }

    fn layout_node(&mut self, id: NodeId, rect: LayoutRect) {
        let (children, style) = match self.nodes.get_mut(id) {
            Some(node) => {
                if let NodeKind::Text(text) = &node.kind {
                    node.content_size = Self::measure_text_size(text);
                }
                node.layout = rect;
                let children = std::mem::take(&mut node.children);
                let style = node.style_mut().map(std::mem::take);
                (children, style)
            }
            None => return,
        };

        let Some(style) = style else {
            if let Some(node) = self.nodes.get_mut(id) {
                node.children = children;
            }
            return;
        };

        let content_rect = rect.inset(style.border.add(style.padding));
        let mut flow_children = Vec::new();

        for child_id in children.iter().copied() {
            let (child_style, position_mode, child_content_size) = {
                let Some(node) = self.nodes.get_mut(child_id) else {
                    continue;
                };
                if let NodeKind::Text(text) = &node.kind {
                    node.content_size = Self::measure_text_size(text);
                }
                let Some(style) = node.style() else {
                    continue;
                };
                (style.clone(), style.position.mode, node.content_size)
            };

            match position_mode {
                PositionMode::Flow => flow_children.push(child_id),
                PositionMode::Absolute => {
                    let child_rect =
                        Self::layout_absolute(&content_rect, &child_style, child_content_size);
                    self.layout_node(child_id, child_rect);
                }
            }
        }

        self.layout_flow_children(&content_rect, &style.layout, &flow_children);

        if let Some(node) = self.nodes.get_mut(id) {
            match &mut node.kind {
                NodeKind::Div(div) => {
                    div.style = style;
                }
                NodeKind::Text(text) => {
                    text.style = style;
                }
            }
            node.children = children;
        }
    }

    fn measure_text_size(text: &Text) -> Vector2f {
        let rect = gfx::measure_text(&text.content, text.scale, text.font_id);
        Vector2f::new(rect.w, rect.h)
    }

    fn layout_flow_children(
        &mut self,
        parent_rect: &LayoutRect,
        layout: &LayoutStyle,
        children: &[NodeId],
    ) {
        if children.is_empty() {
            return;
        }

        let mut fixed_main = 0.0f32;
        let mut fill_count = 0usize;
        let mut child_count = 0usize;

        for child_id in children.iter().copied() {
            let Some(node) = self.nodes.get(child_id) else {
                continue;
            };
            let Some(style) = node.style() else {
                continue;
            };
            let content_size = node.content_size;
            child_count += 1;

            let (main, _) = Self::resolve_child_size(parent_rect, layout, style, content_size, 0.0);
            let main_margin = Self::main_margin(layout, &style.margin);
            if matches!(Self::main_length(layout, &style.size), Length::Fill) {
                fill_count += 1;
            } else {
                fixed_main += main + main_margin;
            }
        }

        let base_gap = layout.gap.max(0.0);
        let gap_total = base_gap * (child_count.saturating_sub(1) as f32);
        let mut available_main = Self::main_size(layout, parent_rect) - fixed_main - gap_total;
        if available_main < 0.0 {
            available_main = 0.0;
        }
        let fill_share = if fill_count > 0 {
            available_main / fill_count as f32
        } else {
            0.0
        };

        let mut main_sizes = Vec::with_capacity(children.len());
        let mut total_main = gap_total;
        for child_id in children.iter().copied() {
            let Some(node) = self.nodes.get(child_id) else {
                continue;
            };
            let Some(style) = node.style() else {
                continue;
            };
            let content_size = node.content_size;
            let (main, _) =
                Self::resolve_child_size(parent_rect, layout, style, content_size, fill_share);
            let main_margin = Self::main_margin(layout, &style.margin);
            total_main += main + main_margin;
            main_sizes.push((child_id, main));
        }

        let mut extra_space = Self::main_size(layout, parent_rect) - total_main;
        if extra_space < 0.0 {
            extra_space = 0.0;
        }

        let child_count = main_sizes.len();
        let (start_offset, extra_gap) = match layout.justify_content {
            JustifyContent::Start => (0.0, 0.0),
            JustifyContent::Center => (extra_space * 0.5, 0.0),
            JustifyContent::End => (extra_space, 0.0),
            JustifyContent::SpaceBetween => {
                if child_count > 1 {
                    (0.0, extra_space / (child_count - 1) as f32)
                } else {
                    (0.0, 0.0)
                }
            }
            JustifyContent::SpaceAround => {
                if child_count == 0 {
                    (0.0, 0.0)
                } else {
                    let gap = extra_space / child_count as f32;
                    (gap * 0.5, gap)
                }
            }
            JustifyContent::SpaceEvenly => {
                if child_count == 0 {
                    (0.0, 0.0)
                } else {
                    let gap = extra_space / (child_count as f32 + 1.0);
                    (gap, gap)
                }
            }
        };

        let gap = base_gap + extra_gap;
        let mut cursor = start_offset;
        for (child_id, main_size) in main_sizes.into_iter() {
            let (child_rect, main_margin) = {
                let Some(node) = self.nodes.get(child_id) else {
                    continue;
                };
                let Some(style) = node.style() else {
                    continue;
                };
                let content_size = node.content_size;
                let (main, cross) =
                    Self::resolve_child_size(parent_rect, layout, style, content_size, fill_share);
                let cross_size = Self::resolve_cross_size(parent_rect, layout, style, cross);
                let margin = style.margin;
                let (pos, size) = match layout.direction {
                    LayoutDirection::Row => {
                        let x = parent_rect.position.x + cursor + margin.left;
                        let y = Self::align_cross(parent_rect, layout, cross_size, &margin);
                        (Vector2f::new(x, y), Vector2f::new(main, cross_size))
                    }
                    LayoutDirection::Column => {
                        let x = Self::align_cross(parent_rect, layout, cross_size, &margin);
                        let y = parent_rect.position.y + cursor + margin.top;
                        (Vector2f::new(x, y), Vector2f::new(cross_size, main))
                    }
                };
                let child_rect = LayoutRect {
                    position: pos,
                    size,
                };
                let main_margin = Self::main_margin(layout, &margin);
                (child_rect, main_margin)
            };
            self.layout_node(child_id, child_rect);
            cursor += main_size + main_margin + gap;
        }
    }

    fn layout_absolute(
        parent_rect: &LayoutRect,
        style: &Style,
        content_size: Vector2f,
    ) -> LayoutRect {
        let size = Self::resolve_size(parent_rect, style, content_size, 0.0);
        let margin = &style.margin;
        let anchors = style.position.anchors;
        let mut pos = parent_rect.position;
        let mut actual_size = size;

        if anchors.left && anchors.right {
            actual_size.x = (parent_rect.size.x - margin.left - margin.right).max(0.0);
            pos.x = parent_rect.position.x + margin.left;
        } else if anchors.left {
            pos.x = parent_rect.position.x + margin.left;
        } else if anchors.right {
            pos.x = parent_rect.position.x + parent_rect.size.x - margin.right - actual_size.x;
        } else {
            pos.x = parent_rect.position.x + margin.left;
        }

        if anchors.top && anchors.bottom {
            actual_size.y = (parent_rect.size.y - margin.top - margin.bottom).max(0.0);
            pos.y = parent_rect.position.y + margin.top;
        } else if anchors.top {
            pos.y = parent_rect.position.y + margin.top;
        } else if anchors.bottom {
            pos.y = parent_rect.position.y + parent_rect.size.y - margin.bottom - actual_size.y;
        } else {
            pos.y = parent_rect.position.y + margin.top;
        }

        LayoutRect {
            position: pos,
            size: actual_size,
        }
    }

    fn resolve_size(
        parent_rect: &LayoutRect,
        style: &Style,
        content_size: Vector2f,
        fill_share: f32,
    ) -> Vector2f {
        let edge_x = style.padding.horizontal() + style.border.horizontal();
        let edge_y = style.padding.vertical() + style.border.vertical();
        let content_w = Self::resolve_length(
            style.size.width,
            parent_rect.size.x,
            fill_share,
            edge_x,
            content_size.x,
        );
        let content_h = Self::resolve_length(
            style.size.height,
            parent_rect.size.y,
            fill_share,
            edge_y,
            content_size.y,
        );
        let clamped = Self::apply_constraints(
            Vector2f::new(content_w, content_h),
            parent_rect,
            style,
            Vector2f::new(edge_x, edge_y),
        );
        Vector2f::new((clamped.x + edge_x).max(0.0), (clamped.y + edge_y).max(0.0))
    }

    fn resolve_child_size(
        parent_rect: &LayoutRect,
        layout: &LayoutStyle,
        style: &Style,
        content_size: Vector2f,
        fill_share: f32,
    ) -> (f32, f32) {
        let size = Self::resolve_size(parent_rect, style, content_size, fill_share);
        match layout.direction {
            LayoutDirection::Row => (size.x, size.y),
            LayoutDirection::Column => (size.y, size.x),
        }
    }

    fn resolve_cross_size(
        parent_rect: &LayoutRect,
        layout: &LayoutStyle,
        style: &Style,
        cross: f32,
    ) -> f32 {
        let available = Self::cross_size(layout, parent_rect);
        let cross_margin = Self::cross_margin(layout, &style.margin);
        let mut resolved = cross;
        let length = Self::cross_length(layout, &style.size);
        if matches!(length, Length::Fill)
            || (matches!(length, Length::Auto) && layout.align_items == AlignItems::Stretch)
        {
            resolved = (available - cross_margin).max(0.0);
        }

        resolved
    }

    fn resolve_length(
        length: Length,
        available: f32,
        fill_share: f32,
        edge_sum: f32,
        content_value: f32,
    ) -> f32 {
        match length {
            Length::Auto => content_value.max(0.0),
            Length::Px(px) => px.max(0.0),
            Length::Percent(pct) => (available * pct).max(0.0),
            Length::Fill => (fill_share - edge_sum).max(0.0),
        }
    }

    fn apply_constraints(
        content: Vector2f,
        parent_rect: &LayoutRect,
        style: &Style,
        edge: Vector2f,
    ) -> Vector2f {
        let min_w = Self::resolve_constraint(style.min_size.width, parent_rect.size.x, edge.x);
        let min_h = Self::resolve_constraint(style.min_size.height, parent_rect.size.y, edge.y);
        let max_w = Self::resolve_constraint(style.max_size.width, parent_rect.size.x, edge.x);
        let max_h = Self::resolve_constraint(style.max_size.height, parent_rect.size.y, edge.y);

        let mut width = content.x;
        let mut height = content.y;
        if let Some(min) = min_w {
            width = width.max(min);
        }
        if let Some(min) = min_h {
            height = height.max(min);
        }
        if let Some(max) = max_w {
            width = width.min(max);
        }
        if let Some(max) = max_h {
            height = height.min(max);
        }
        Vector2f::new(width, height)
    }

    fn resolve_constraint(length: Length, available: f32, edge_sum: f32) -> Option<f32> {
        match length {
            Length::Auto => None,
            Length::Px(px) => Some(px.max(0.0)),
            Length::Percent(pct) => Some((available * pct).max(0.0)),
            Length::Fill => Some((available - edge_sum).max(0.0)),
        }
    }
    fn main_size(layout: &LayoutStyle, rect: &LayoutRect) -> f32 {
        match layout.direction {
            LayoutDirection::Row => rect.size.x,
            LayoutDirection::Column => rect.size.y,
        }
    }

    fn cross_size(layout: &LayoutStyle, rect: &LayoutRect) -> f32 {
        match layout.direction {
            LayoutDirection::Row => rect.size.y,
            LayoutDirection::Column => rect.size.x,
        }
    }

    fn main_length(layout: &LayoutStyle, size: &Size2) -> Length {
        match layout.direction {
            LayoutDirection::Row => size.width,
            LayoutDirection::Column => size.height,
        }
    }

    fn cross_length(layout: &LayoutStyle, size: &Size2) -> Length {
        match layout.direction {
            LayoutDirection::Row => size.height,
            LayoutDirection::Column => size.width,
        }
    }

    fn main_margin(layout: &LayoutStyle, margin: &EdgeSizes) -> f32 {
        match layout.direction {
            LayoutDirection::Row => margin.left + margin.right,
            LayoutDirection::Column => margin.top + margin.bottom,
        }
    }

    fn cross_margin(layout: &LayoutStyle, margin: &EdgeSizes) -> f32 {
        match layout.direction {
            LayoutDirection::Row => margin.top + margin.bottom,
            LayoutDirection::Column => margin.left + margin.right,
        }
    }

    fn align_cross(
        parent_rect: &LayoutRect,
        layout: &LayoutStyle,
        cross_size: f32,
        margin: &EdgeSizes,
    ) -> f32 {
        match layout.direction {
            LayoutDirection::Row => match layout.align_items {
                AlignItems::Start => parent_rect.position.y + margin.top,
                AlignItems::Center => {
                    parent_rect.position.y + (parent_rect.size.y - cross_size) * 0.5
                }
                AlignItems::End => {
                    parent_rect.position.y + parent_rect.size.y - cross_size - margin.bottom
                }
                AlignItems::Stretch => parent_rect.position.y + margin.top,
            },
            LayoutDirection::Column => match layout.align_items {
                AlignItems::Start => parent_rect.position.x + margin.left,
                AlignItems::Center => {
                    parent_rect.position.x + (parent_rect.size.x - cross_size) * 0.5
                }
                AlignItems::End => {
                    parent_rect.position.x + parent_rect.size.x - cross_size - margin.right
                }
                AlignItems::Stretch => parent_rect.position.x + margin.left,
            },
        }
    }
}

impl Default for Dom {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub kind: NodeKind,
    pub layout: LayoutRect,
    pub content_size: Vector2f,
}

impl Node {
    pub fn as_div(&self) -> Option<&Div> {
        match &self.kind {
            NodeKind::Div(div) => Some(div),
            NodeKind::Text(_) => None,
        }
    }

    pub fn as_div_mut(&mut self) -> Option<&mut Div> {
        match &mut self.kind {
            NodeKind::Div(div) => Some(div),
            NodeKind::Text(_) => None,
        }
    }

    pub fn as_text(&self) -> Option<&Text> {
        match &self.kind {
            NodeKind::Text(text) => Some(text),
            NodeKind::Div(_) => None,
        }
    }

    pub fn as_text_mut(&mut self) -> Option<&mut Text> {
        match &mut self.kind {
            NodeKind::Text(text) => Some(text),
            NodeKind::Div(_) => None,
        }
    }

    pub fn style(&self) -> Option<&Style> {
        match &self.kind {
            NodeKind::Div(div) => Some(&div.style),
            NodeKind::Text(text) => Some(&text.style),
        }
    }

    pub fn style_mut(&mut self) -> Option<&mut Style> {
        match &mut self.kind {
            NodeKind::Div(div) => Some(&mut div.style),
            NodeKind::Text(text) => Some(&mut text.style),
        }
    }
}

#[derive(Debug, Clone)]
pub enum NodeKind {
    Div(Div),
    Text(Text),
}

#[derive(Debug, Clone)]
pub struct Div {
    pub style: Style,
}

impl Default for Div {
    fn default() -> Self {
        Self {
            style: Style::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Text {
    pub content: String,
    pub font_id: u32,
    pub scale: f32,
    pub style: Style,
}

impl Text {
    pub fn new(content: impl Into<String>, font_id: u32, scale: f32) -> Self {
        Self {
            content: content.into(),
            font_id,
            scale,
            style: Style::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Style {
    pub layout: LayoutStyle,
    pub size: Size2,
    pub min_size: Size2,
    pub max_size: Size2,
    pub position: PositionStyle,
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
            size: Size2::fill(),
            min_size: Size2::auto(),
            max_size: Size2::auto(),
            position: PositionStyle::default(),
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
    pub fn anchored(mut self, anchors: Anchors) -> Self {
        self.position.mode = PositionMode::Absolute;
        self.position.anchors = anchors;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LayoutStyle {
    pub direction: LayoutDirection,
    pub align_items: AlignItems,
    pub justify_content: JustifyContent,
    pub gap: f32,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            direction: LayoutDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            gap: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Size2 {
    pub width: Length,
    pub height: Length,
}

impl Size2 {
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

impl Default for Size2 {
    fn default() -> Self {
        Self::fill()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PositionStyle {
    pub mode: PositionMode,
    pub anchors: Anchors,
}

impl Default for PositionStyle {
    fn default() -> Self {
        Self {
            mode: PositionMode::Flow,
            anchors: Anchors::none(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchors {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

impl Anchors {
    pub fn none() -> Self {
        Self {
            left: false,
            right: false,
            top: false,
            bottom: false,
        }
    }

    pub fn fill() -> Self {
        Self {
            left: true,
            right: true,
            top: true,
            bottom: true,
        }
    }

    pub fn horizontal() -> Self {
        Self {
            left: true,
            right: true,
            top: false,
            bottom: false,
        }
    }

    pub fn vertical() -> Self {
        Self {
            left: false,
            right: false,
            top: true,
            bottom: true,
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
pub struct LayoutRect {
    pub position: Vector2f,
    pub size: Vector2f,
}

impl Default for LayoutRect {
    fn default() -> Self {
        Self {
            position: Vector2f::default(),
            size: Vector2f::default(),
        }
    }
}

impl LayoutRect {
    pub fn inset(&self, padding: EdgeSizes) -> Self {
        let width = (self.size.x - padding.left - padding.right).max(0.0);
        let height = (self.size.y - padding.top - padding.bottom).max(0.0);
        Self {
            position: Vector2f::new(
                self.position.x + padding.left,
                self.position.y + padding.top,
            ),
            size: Vector2f::new(width, height),
        }
    }

    pub fn contains(&self, point: Vector2f) -> bool {
        point.x >= self.position.x
            && point.y >= self.position.y
            && point.x < self.position.x + self.size.x
            && point.y < self.position.y + self.size.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ExpectedStyle {
        direction: LayoutDirection,
        align_items: AlignItems,
        justify_content: JustifyContent,
        gap: f32,
        size: Size2,
        min_size: Size2,
        max_size: Size2,
        position: PositionStyle,
        padding: EdgeSizes,
        margin: EdgeSizes,
        border: EdgeSizes,
    }

    fn apply_expected_style(dom: &mut Dom, id: NodeId, expected: &ExpectedStyle) {
        let style = dom.node_mut(id).unwrap().style_mut().unwrap();
        style.layout.direction = expected.direction;
        style.layout.align_items = expected.align_items;
        style.layout.justify_content = expected.justify_content;
        style.layout.gap = expected.gap;
        style.size = expected.size;
        style.min_size = expected.min_size;
        style.max_size = expected.max_size;
        style.position = expected.position;
        style.padding = expected.padding;
        style.margin = expected.margin;
        style.border = expected.border;
    }

    fn assert_style(node: &Node, expected: &ExpectedStyle) {
        let style = node.style().unwrap();
        assert_eq!(style.layout.direction, expected.direction);
        assert_eq!(style.layout.align_items, expected.align_items);
        assert_eq!(style.layout.justify_content, expected.justify_content);
        assert_eq!(style.layout.gap, expected.gap);
        assert_eq!(style.size.width, expected.size.width);
        assert_eq!(style.size.height, expected.size.height);
        assert_eq!(style.min_size.width, expected.min_size.width);
        assert_eq!(style.min_size.height, expected.min_size.height);
        assert_eq!(style.max_size.width, expected.max_size.width);
        assert_eq!(style.max_size.height, expected.max_size.height);
        assert_eq!(style.position.mode, expected.position.mode);
        assert_eq!(style.position.anchors.left, expected.position.anchors.left);
        assert_eq!(
            style.position.anchors.right,
            expected.position.anchors.right
        );
        assert_eq!(style.position.anchors.top, expected.position.anchors.top);
        assert_eq!(
            style.position.anchors.bottom,
            expected.position.anchors.bottom
        );
        assert_eq!(style.padding.left, expected.padding.left);
        assert_eq!(style.padding.right, expected.padding.right);
        assert_eq!(style.padding.top, expected.padding.top);
        assert_eq!(style.padding.bottom, expected.padding.bottom);
        assert_eq!(style.margin.left, expected.margin.left);
        assert_eq!(style.margin.right, expected.margin.right);
        assert_eq!(style.margin.top, expected.margin.top);
        assert_eq!(style.margin.bottom, expected.margin.bottom);
        assert_eq!(style.border.left, expected.border.left);
        assert_eq!(style.border.right, expected.border.right);
        assert_eq!(style.border.top, expected.border.top);
        assert_eq!(style.border.bottom, expected.border.bottom);
    }

    fn assert_layout(node: &Node, position: Vector2f, size: Vector2f) {
        assert_eq!(node.layout.position.x, position.x);
        assert_eq!(node.layout.position.y, position.y);
        assert_eq!(node.layout.size.x, size.x);
        assert_eq!(node.layout.size.y, size.y);
    }

    #[test]
    fn auto_size_uses_content_measurement() {
        let mut dom = Dom::new();
        let root = dom.root();
        let child = dom.create_div();
        assert!(dom.append_child(root, child));

        {
            let root_style = dom.node_mut(root).unwrap().style_mut().unwrap();
            root_style.layout.align_items = AlignItems::Start;
            root_style.layout.direction = LayoutDirection::Column;
        }

        {
            let child_style = dom.node_mut(child).unwrap().style_mut().unwrap();
            child_style.size = Size2::auto();
            child_style.padding = EdgeSizes {
                left: 5.0,
                right: 5.0,
                top: 5.0,
                bottom: 5.0,
            };
            child_style.border = EdgeSizes {
                left: 2.0,
                right: 2.0,
                top: 2.0,
                bottom: 2.0,
            };
        }

        dom.set_content_size(child, Vector2f::new(30.0, 40.0));
        dom.layout(Vector2f::new(200.0, 200.0));

        let child_node = dom.node(child).unwrap();
        assert_eq!(child_node.layout.size.x, 30.0 + 10.0 + 4.0);
        assert_eq!(child_node.layout.size.y, 40.0 + 10.0 + 4.0);
    }

    #[test]
    fn min_max_constraints_clamp_content() {
        let mut dom = Dom::new();
        let root = dom.root();
        let child = dom.create_div();
        assert!(dom.append_child(root, child));

        {
            let root_style = dom.node_mut(root).unwrap().style_mut().unwrap();
            root_style.layout.align_items = AlignItems::Start;
            root_style.layout.direction = LayoutDirection::Column;
        }

        {
            let child_style = dom.node_mut(child).unwrap().style_mut().unwrap();
            child_style.size = Size2::auto();
            child_style.min_size = Size2 {
                width: Length::Px(120.0),
                height: Length::Auto,
            };
            child_style.max_size = Size2 {
                width: Length::Auto,
                height: Length::Px(80.0),
            };
        }

        dom.set_content_size(child, Vector2f::new(100.0, 100.0));
        dom.layout(Vector2f::new(200.0, 200.0));

        let child_node = dom.node(child).unwrap();
        assert_eq!(child_node.layout.size.x, 120.0);
        assert_eq!(child_node.layout.size.y, 80.0);
    }

    #[test]
    fn absolute_anchor_fill_respects_margins() {
        let mut dom = Dom::new();
        let root = dom.root();
        let child = dom.create_div();
        assert!(dom.append_child(root, child));

        {
            let root_style = dom.node_mut(root).unwrap().style_mut().unwrap();
            root_style.layout.direction = LayoutDirection::Column;
            root_style.layout.align_items = AlignItems::Start;
        }

        {
            let child_style = dom.node_mut(child).unwrap().style_mut().unwrap();
            child_style.position = PositionStyle {
                mode: PositionMode::Absolute,
                anchors: Anchors::fill(),
            };
            child_style.margin = EdgeSizes {
                left: 10.0,
                right: 20.0,
                top: 5.0,
                bottom: 15.0,
            };
        }

        dom.layout(Vector2f::new(200.0, 100.0));

        let child_node = dom.node(child).unwrap();
        assert_eq!(child_node.layout.position.x, 10.0);
        assert_eq!(child_node.layout.position.y, 5.0);
        assert_eq!(child_node.layout.size.x, 200.0 - 10.0 - 20.0);
        assert_eq!(child_node.layout.size.y, 100.0 - 5.0 - 15.0);
    }

    #[test]
    fn dom_structure_is_consistent() {
        let mut dom = Dom::new();
        let root = dom.root();
        let child_a = dom.create_div();
        let child_b = dom.create_div();
        assert!(dom.append_child(root, child_a));
        assert!(dom.append_child(root, child_b));

        let root_node = dom.node(root).unwrap();
        assert_eq!(root_node.children.len(), 2);
        assert!(root_node.children.contains(&child_a));
        assert!(root_node.children.contains(&child_b));

        let child_node = dom.node(child_a).unwrap();
        assert_eq!(child_node.parent, Some(root));
    }

    #[test]
    fn append_child_reparents_and_removes_from_old_parent() {
        let mut dom = Dom::new();
        let root = dom.root();
        let parent_a = dom.create_div();
        let parent_b = dom.create_div();
        let child = dom.create_div();
        assert!(dom.append_child(root, parent_a));
        assert!(dom.append_child(root, parent_b));
        assert!(dom.append_child(parent_a, child));

        assert!(dom.append_child(parent_b, child));

        let parent_a_node = dom.node(parent_a).unwrap();
        assert!(!parent_a_node.children.contains(&child));

        let parent_b_node = dom.node(parent_b).unwrap();
        assert!(parent_b_node.children.contains(&child));

        let child_node = dom.node(child).unwrap();
        assert_eq!(child_node.parent, Some(parent_b));
    }

    #[test]
    fn deep_layout_preserves_structure_and_styles() {
        let mut dom = Dom::new();
        let root = dom.root();
        let a = dom.create_div();
        let b = dom.create_div();
        let c = dom.create_div();
        let d = dom.create_div();
        let e = dom.create_div();
        let b1 = dom.create_div();
        let root_sibling = dom.create_div();

        assert!(dom.append_child(root, a));
        assert!(dom.append_child(a, b));
        assert!(dom.append_child(b, c));
        assert!(dom.append_child(c, d));
        assert!(dom.append_child(d, e));
        assert!(dom.append_child(b, b1));
        assert!(dom.append_child(root, root_sibling));

        let expected_root = ExpectedStyle {
            direction: LayoutDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::SpaceBetween,
            gap: 4.0,
            size: Size2::fill(),
            min_size: Size2::auto(),
            max_size: Size2::auto(),
            position: PositionStyle::default(),
            padding: EdgeSizes {
                left: 3.0,
                right: 6.0,
                top: 2.0,
                bottom: 5.0,
            },
            margin: EdgeSizes::zero(),
            border: EdgeSizes {
                left: 1.0,
                right: 2.0,
                top: 1.5,
                bottom: 2.5,
            },
        };

        let expected_a = ExpectedStyle {
            direction: LayoutDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Start,
            gap: 1.0,
            size: Size2 {
                width: Length::Percent(0.5),
                height: Length::Px(120.0),
            },
            min_size: Size2 {
                width: Length::Px(30.0),
                height: Length::Auto,
            },
            max_size: Size2 {
                width: Length::Auto,
                height: Length::Px(140.0),
            },
            position: PositionStyle::default(),
            padding: EdgeSizes::zero(),
            margin: EdgeSizes {
                left: 4.0,
                right: 5.0,
                top: 6.0,
                bottom: 7.0,
            },
            border: EdgeSizes::zero(),
        };

        let expected_b = ExpectedStyle {
            direction: LayoutDirection::Column,
            align_items: AlignItems::End,
            justify_content: JustifyContent::Center,
            gap: 2.5,
            size: Size2 {
                width: Length::Fill,
                height: Length::Auto,
            },
            min_size: Size2::auto(),
            max_size: Size2::auto(),
            position: PositionStyle::default(),
            padding: EdgeSizes {
                left: 2.0,
                right: 2.0,
                top: 2.0,
                bottom: 2.0,
            },
            margin: EdgeSizes::zero(),
            border: EdgeSizes::zero(),
        };

        let expected_c = ExpectedStyle {
            direction: LayoutDirection::Row,
            align_items: AlignItems::Start,
            justify_content: JustifyContent::End,
            gap: 0.0,
            size: Size2 {
                width: Length::Px(64.0),
                height: Length::Px(48.0),
            },
            min_size: Size2::auto(),
            max_size: Size2 {
                width: Length::Px(80.0),
                height: Length::Px(60.0),
            },
            position: PositionStyle::default(),
            padding: EdgeSizes::zero(),
            margin: EdgeSizes::zero(),
            border: EdgeSizes::zero(),
        };

        let expected_d = ExpectedStyle {
            direction: LayoutDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceAround,
            gap: 3.0,
            size: Size2 {
                width: Length::Auto,
                height: Length::Auto,
            },
            min_size: Size2::auto(),
            max_size: Size2::auto(),
            position: PositionStyle::default(),
            padding: EdgeSizes::zero(),
            margin: EdgeSizes::zero(),
            border: EdgeSizes {
                left: 1.0,
                right: 1.0,
                top: 1.0,
                bottom: 1.0,
            },
        };

        let expected_e = ExpectedStyle {
            direction: LayoutDirection::Row,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::SpaceEvenly,
            gap: 5.0,
            size: Size2 {
                width: Length::Percent(0.25),
                height: Length::Percent(0.4),
            },
            min_size: Size2::auto(),
            max_size: Size2::auto(),
            position: PositionStyle::default(),
            padding: EdgeSizes::zero(),
            margin: EdgeSizes {
                left: 1.0,
                right: 2.0,
                top: 3.0,
                bottom: 4.0,
            },
            border: EdgeSizes::zero(),
        };

        let expected_b1 = ExpectedStyle {
            direction: LayoutDirection::Column,
            align_items: AlignItems::Start,
            justify_content: JustifyContent::Start,
            gap: 0.0,
            size: Size2 {
                width: Length::Px(40.0),
                height: Length::Px(10.0),
            },
            min_size: Size2::auto(),
            max_size: Size2::auto(),
            position: PositionStyle {
                mode: PositionMode::Absolute,
                anchors: Anchors::horizontal(),
            },
            padding: EdgeSizes::zero(),
            margin: EdgeSizes::zero(),
            border: EdgeSizes::zero(),
        };

        let expected_root_sibling = ExpectedStyle {
            direction: LayoutDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            gap: 0.5,
            size: Size2 {
                width: Length::Px(90.0),
                height: Length::Px(60.0),
            },
            min_size: Size2::auto(),
            max_size: Size2::auto(),
            position: PositionStyle::default(),
            padding: EdgeSizes::zero(),
            margin: EdgeSizes::zero(),
            border: EdgeSizes::zero(),
        };

        apply_expected_style(&mut dom, root, &expected_root);
        apply_expected_style(&mut dom, a, &expected_a);
        apply_expected_style(&mut dom, b, &expected_b);
        apply_expected_style(&mut dom, c, &expected_c);
        apply_expected_style(&mut dom, d, &expected_d);
        apply_expected_style(&mut dom, e, &expected_e);
        apply_expected_style(&mut dom, b1, &expected_b1);
        apply_expected_style(&mut dom, root_sibling, &expected_root_sibling);

        dom.set_content_size(d, Vector2f::new(24.0, 18.0));
        dom.set_content_size(e, Vector2f::new(10.0, 12.0));
        dom.set_content_size(b1, Vector2f::new(8.0, 6.0));
        dom.layout(Vector2f::new(320.0, 240.0));

        let root_node = dom.node(root).unwrap();
        assert_eq!(root_node.children, vec![a, root_sibling]);
        assert_style(root_node, &expected_root);
        assert_layout(
            root_node,
            Vector2f::new(0.0, 0.0),
            Vector2f::new(320.0, 240.0),
        );

        let a_node = dom.node(a).unwrap();
        assert_eq!(a_node.parent, Some(root));
        assert_eq!(a_node.children, vec![b]);
        assert_style(a_node, &expected_a);
        assert_layout(a_node, Vector2f::new(8.0, 9.5), Vector2f::new(154.0, 120.0));

        let b_node = dom.node(b).unwrap();
        assert_eq!(b_node.parent, Some(a));
        assert_eq!(b_node.children, vec![c, b1]);
        assert_style(b_node, &expected_b);
        assert_layout(b_node, Vector2f::new(8.0, 67.5), Vector2f::new(154.0, 4.0));

        let c_node = dom.node(c).unwrap();
        assert_eq!(c_node.parent, Some(b));
        assert_eq!(c_node.children, vec![d]);
        assert_style(c_node, &expected_c);
        assert_layout(c_node, Vector2f::new(96.0, 69.5), Vector2f::new(64.0, 48.0));

        let d_node = dom.node(d).unwrap();
        assert_eq!(d_node.parent, Some(c));
        assert_eq!(d_node.children, vec![e]);
        assert_style(d_node, &expected_d);

        let e_node = dom.node(e).unwrap();
        assert_eq!(e_node.parent, Some(d));
        assert!(e_node.children.is_empty());
        assert_style(e_node, &expected_e);

        let b1_node = dom.node(b1).unwrap();
        assert_eq!(b1_node.parent, Some(b));
        assert!(b1_node.children.is_empty());
        assert_style(b1_node, &expected_b1);
        assert_layout(
            b1_node,
            Vector2f::new(10.0, 69.5),
            Vector2f::new(150.0, 10.0),
        );

        let sibling_node = dom.node(root_sibling).unwrap();
        assert_eq!(sibling_node.parent, Some(root));
        assert!(sibling_node.children.is_empty());
        assert_style(sibling_node, &expected_root_sibling);
        assert_layout(
            sibling_node,
            Vector2f::new(4.0, 172.5),
            Vector2f::new(90.0, 60.0),
        );
    }

    #[test]
    fn hit_test_returns_deepest_child() {
        let mut dom = Dom::new();
        let root = dom.root();
        let a = dom.create_div();
        let b = dom.create_div();
        let c = dom.create_div();

        assert!(dom.append_child(root, a));
        assert!(dom.append_child(a, b));
        assert!(dom.append_child(b, c));

        {
            let root_style = dom.node_mut(root).unwrap().style_mut().unwrap();
            root_style.layout.direction = LayoutDirection::Column;
            root_style.layout.align_items = AlignItems::Start;
            root_style.layout.justify_content = JustifyContent::Start;
        }

        {
            let a_style = dom.node_mut(a).unwrap().style_mut().unwrap();
            a_style.size = Size2 {
                width: Length::Px(80.0),
                height: Length::Px(80.0),
            };
            a_style.margin = EdgeSizes {
                left: 10.0,
                right: 0.0,
                top: 10.0,
                bottom: 0.0,
            };
        }

        {
            let b_style = dom.node_mut(b).unwrap().style_mut().unwrap();
            b_style.size = Size2 {
                width: Length::Px(40.0),
                height: Length::Px(40.0),
            };
            b_style.margin = EdgeSizes {
                left: 5.0,
                right: 0.0,
                top: 5.0,
                bottom: 0.0,
            };
        }

        {
            let c_style = dom.node_mut(c).unwrap().style_mut().unwrap();
            c_style.size = Size2 {
                width: Length::Px(10.0),
                height: Length::Px(10.0),
            };
            c_style.margin = EdgeSizes {
                left: 2.0,
                right: 0.0,
                top: 2.0,
                bottom: 0.0,
            };
        }

        dom.layout(Vector2f::new(200.0, 200.0));

        let hit_c = dom.hit_test(Vector2f::new(18.0, 18.0));
        assert_eq!(hit_c, Some(c));

        let hit_b = dom.hit_test(Vector2f::new(16.0, 16.0));
        assert_eq!(hit_b, Some(b));

        let hit_a = dom.hit_test(Vector2f::new(11.0, 11.0));
        assert_eq!(hit_a, Some(a));

        let hit_none = dom.hit_test(Vector2f::new(150.0, 150.0));
        assert_eq!(hit_none, Some(root));
    }
}
