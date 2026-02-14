//#![allow(dead_code)]

use crate::{
    Vector2f, gfx,
    gui::{
        Align, EdgeSizes, Justify, LayoutDirection, LayoutStyle, Length, PositionMode, Size, Style,
    },
};

pub type NodeId = usize;

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
        let (children, style, prior_content_size) = match self.nodes.get_mut(id) {
            Some(node) => {
                if let NodeKind::Text(text) = &node.kind {
                    node.content_size = Self::measure_text_size(text);
                }
                let prior_content_size = node.content_size;
                node.layout = rect;
                let children = std::mem::take(&mut node.children);
                let style = node.style_mut().map(std::mem::take);
                (children, style, prior_content_size)
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
        let mut needs_flow_relayout = false;
        let mut absolute_children = Vec::new();

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
                PositionMode::Flow => {
                    if matches!(child_style.size.width, Length::Auto)
                        || matches!(child_style.size.height, Length::Auto)
                    {
                        needs_flow_relayout = true;
                    }
                    flow_children.push(child_id)
                }
                PositionMode::Absolute => {
                    absolute_children.push(child_id);
                    let child_rect =
                        Self::layout_absolute(&content_rect, &child_style, child_content_size);
                    self.layout_node(child_id, child_rect);
                }
            }
        }

        self.layout_flow_children(&content_rect, &style.layout, &flow_children);
        if needs_flow_relayout {
            self.layout_flow_children(&content_rect, &style.layout, &flow_children);
        }

        let mut updated_rect = rect;
        if matches!(style.size.width, Length::Auto) || matches!(style.size.height, Length::Auto) {
            let edge = style.border.add(style.padding);
            let mut max_right = rect.position.x + edge.left;
            let mut max_bottom = rect.position.y + edge.top;

            for child_id in children.iter().copied() {
                if let Some(child) = self.nodes.get(child_id) {
                    let extent_x = child.layout.size.x.max(child.content_size.x);
                    let extent_y = child.layout.size.y.max(child.content_size.y);
                    max_right = max_right.max(child.layout.position.x + extent_x);
                    max_bottom = max_bottom.max(child.layout.position.y + extent_y);
                }
            }

            let required_w = (max_right - rect.position.x + edge.right).max(0.0);
            let required_h = (max_bottom - rect.position.y + edge.bottom).max(0.0);

            if matches!(style.size.width, Length::Auto) {
                updated_rect.size.x = updated_rect.size.x.max(required_w);
            }
            if matches!(style.size.height, Length::Auto) {
                updated_rect.size.y = updated_rect.size.y.max(required_h);
            }
        }

        if updated_rect.size.x != rect.size.x || updated_rect.size.y != rect.size.y {
            let updated_content = updated_rect.inset(style.border.add(style.padding));

            for child_id in absolute_children.iter().copied() {
                let (child_style, child_content_size) = {
                    let Some(node) = self.nodes.get_mut(child_id) else {
                        continue;
                    };
                    if let NodeKind::Text(text) = &node.kind {
                        node.content_size = Self::measure_text_size(text);
                    }
                    let Some(style) = node.style() else {
                        continue;
                    };
                    (style.clone(), node.content_size)
                };
                let child_rect =
                    Self::layout_absolute(&updated_content, &child_style, child_content_size);
                self.layout_node(child_id, child_rect);
            }

            self.layout_flow_children(&updated_content, &style.layout, &flow_children);
            if needs_flow_relayout {
                self.layout_flow_children(&updated_content, &style.layout, &flow_children);
            }
        }

        if matches!(style.size.width, Length::Auto) || matches!(style.size.height, Length::Auto) {
            let edge = style.border.add(style.padding);
            let mut max_right = updated_rect.position.x + edge.left;
            let mut max_bottom = updated_rect.position.y + edge.top;

            for child_id in children.iter().copied() {
                if let Some(child) = self.nodes.get(child_id) {
                    let extent_x = child.layout.size.x.max(child.content_size.x);
                    let extent_y = child.layout.size.y.max(child.content_size.y);
                    max_right = max_right.max(child.layout.position.x + extent_x);
                    max_bottom = max_bottom.max(child.layout.position.y + extent_y);
                }
            }

            if let Some(node) = self.nodes.get_mut(id) {
                if let NodeKind::Div(_) = node.kind {
                    let content_w = (max_right - (updated_rect.position.x + edge.left)).max(0.0);
                    let content_h = (max_bottom - (updated_rect.position.y + edge.top)).max(0.0);
                    let merged_w = content_w.max(prior_content_size.x);
                    let merged_h = content_h.max(prior_content_size.y);
                    node.content_size = Vector2f::new(merged_w, merged_h);
                }
            }
        }

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
            node.layout = updated_rect;
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
        let (start_offset, extra_gap) = match layout.justify {
            Justify::Start => (0.0, 0.0),
            Justify::Center => (extra_space * 0.5, 0.0),
            Justify::End => (extra_space, 0.0),
            Justify::SpaceBetween => {
                if child_count > 1 {
                    (0.0, extra_space / (child_count - 1) as f32)
                } else {
                    (0.0, 0.0)
                }
            }
            Justify::SpaceAround => {
                if child_count == 0 {
                    (0.0, 0.0)
                } else {
                    let gap = extra_space / child_count as f32;
                    (gap * 0.5, gap)
                }
            }
            Justify::SpaceEvenly => {
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
        let mut pos = parent_rect.position;
        pos.x = parent_rect.position.x + margin.left;
        pos.y = parent_rect.position.y + margin.top;

        LayoutRect {
            position: pos,
            size: size,
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
            || (matches!(length, Length::Auto) && layout.align == Align::Stretch)
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

    fn main_length(layout: &LayoutStyle, size: &Size) -> Length {
        match layout.direction {
            LayoutDirection::Row => size.width,
            LayoutDirection::Column => size.height,
        }
    }

    fn cross_length(layout: &LayoutStyle, size: &Size) -> Length {
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
            LayoutDirection::Row => match layout.align {
                Align::Start => parent_rect.position.y + margin.top,
                Align::Center => parent_rect.position.y + (parent_rect.size.y - cross_size) * 0.5,
                Align::End => {
                    parent_rect.position.y + parent_rect.size.y - cross_size - margin.bottom
                }
                Align::Stretch => parent_rect.position.y + margin.top,
            },
            LayoutDirection::Column => match layout.align {
                Align::Start => parent_rect.position.x + margin.left,
                Align::Center => parent_rect.position.x + (parent_rect.size.x - cross_size) * 0.5,
                Align::End => {
                    parent_rect.position.x + parent_rect.size.x - cross_size - margin.right
                }
                Align::Stretch => parent_rect.position.x + margin.left,
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

    #[test]
    fn auto_size_uses_content_measurement() {
        let mut dom = Dom::new();
        let root = dom.root();
        let child = dom.create_div();
        assert!(dom.append_child(root, child));

        {
            let root_style = dom.node_mut(root).unwrap().style_mut().unwrap();
            root_style.layout.align = Align::Start;
            root_style.layout.direction = LayoutDirection::Column;
        }

        {
            let child_style = dom.node_mut(child).unwrap().style_mut().unwrap();
            child_style.size = Size::auto();
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
    fn text_children_overflow_parent() {
        let mut dom = Dom::new();
        let root = dom.root();
        let panel = dom.create_div();
        assert!(dom.append_child(root, panel));

        let text_a = dom.create_text(
            "WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW",
            gfx::fonts::CASKAYDIAMONO_FONT_ID,
            1.0,
        );
        let text_b = dom.create_text(
            "WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW",
            gfx::fonts::CASKAYDIAMONO_FONT_ID,
            1.0,
        );
        let text_c = dom.create_text(
            "WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW",
            gfx::fonts::CASKAYDIAMONO_FONT_ID,
            1.0,
        );

        assert!(dom.append_child(panel, text_a));
        assert!(dom.append_child(panel, text_b));
        assert!(dom.append_child(panel, text_c));

        for text_id in [text_a, text_b, text_c] {
            let text_style = dom.node_mut(text_id).unwrap().style_mut().unwrap();
            text_style.size = Size::auto();
        }

        {
            let root_style = dom.node_mut(root).unwrap().style_mut().unwrap();
            root_style.layout.direction = LayoutDirection::Column;
            root_style.layout.align = Align::Start;
            root_style.layout.justify = Justify::Start;
        }

        {
            let panel_style = dom.node_mut(panel).unwrap().style_mut().unwrap();
            panel_style.layout.direction = LayoutDirection::Column;
            panel_style.layout.align = Align::Start;
            panel_style.layout.justify = Justify::Start;
            panel_style.size = Size {
                width: Length::Px(200.0),
                height: Length::Px(60.0),
            };
            panel_style.padding = EdgeSizes::zero();
            panel_style.border = EdgeSizes::zero();
        }

        dom.layout(Vector2f::new(400.0, 200.0));

        let panel_node = dom.node(panel).unwrap();
        let panel_left = panel_node.layout.position.x;
        let panel_top = panel_node.layout.position.y;
        let panel_right = panel_left + panel_node.layout.size.x;
        let panel_bottom = panel_top + panel_node.layout.size.y;
        let epsilon = 0.01;

        let mut overflowed = false;
        for child_id in [text_a, text_b, text_c] {
            let child = dom.node(child_id).unwrap();
            let child_right = child.layout.position.x + child.content_size.x;
            let child_bottom = child.layout.position.y + child.content_size.y;
            if child_right > panel_right + epsilon || child_bottom > panel_bottom + epsilon {
                overflowed = true;
                break;
            }
        }

        assert!(overflowed);
    }

    #[test]
    fn text_children_fit_auto_parent() {
        let mut dom = Dom::new();
        let root = dom.root();
        let panel = dom.create_div();
        assert!(dom.append_child(root, panel));

        let text_a = dom.create_text("Status: OK", gfx::fonts::CASKAYDIAMONO_FONT_ID, 1.0);
        let text_b = dom.create_text("Subsystem: GREEN", gfx::fonts::CASKAYDIAMONO_FONT_ID, 1.0);
        let text_c = dom.create_text("Temp: 72C", gfx::fonts::CASKAYDIAMONO_FONT_ID, 1.0);

        assert!(dom.append_child(panel, text_a));
        assert!(dom.append_child(panel, text_b));
        assert!(dom.append_child(panel, text_c));

        for text_id in [text_a, text_b, text_c] {
            let text_style = dom.node_mut(text_id).unwrap().style_mut().unwrap();
            text_style.size = Size::auto();
        }

        {
            let root_style = dom.node_mut(root).unwrap().style_mut().unwrap();
            root_style.layout.direction = LayoutDirection::Column;
            root_style.layout.align = Align::Start;
            root_style.layout.justify = Justify::Start;
        }

        {
            let panel_style = dom.node_mut(panel).unwrap().style_mut().unwrap();
            panel_style.layout.direction = LayoutDirection::Column;
            panel_style.layout.align = Align::Start;
            panel_style.layout.justify = Justify::Start;
            panel_style.layout.gap = 4.0;
            panel_style.size = Size::auto();
            panel_style.padding = EdgeSizes::zero();
            panel_style.border = EdgeSizes::zero();
        }

        dom.layout(Vector2f::new(400.0, 200.0));

        let panel_node = dom.node(panel).unwrap();
        let panel_left = panel_node.layout.position.x;
        let panel_top = panel_node.layout.position.y;
        let panel_right = panel_left + panel_node.layout.size.x;
        let panel_bottom = panel_top + panel_node.layout.size.y;
        let epsilon = 0.01;

        for child_id in [text_a, text_b, text_c] {
            let child = dom.node(child_id).unwrap();
            let child_right = child.layout.position.x + child.content_size.x;
            let child_bottom = child.layout.position.y + child.content_size.y;
            assert!(child_right <= panel_right + epsilon);
            assert!(child_bottom <= panel_bottom + epsilon);
        }
    }

    #[test]
    fn min_max_constraints_clamp_content() {
        let mut dom = Dom::new();
        let root = dom.root();
        let child = dom.create_div();
        assert!(dom.append_child(root, child));

        {
            let root_style = dom.node_mut(root).unwrap().style_mut().unwrap();
            root_style.layout.align = Align::Start;
            root_style.layout.direction = LayoutDirection::Column;
        }

        {
            let child_style = dom.node_mut(child).unwrap().style_mut().unwrap();
            child_style.size = Size::auto();
            child_style.min_size = Size {
                width: Length::Px(120.0),
                height: Length::Auto,
            };
            child_style.max_size = Size {
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
            root_style.layout.align = Align::Start;
            root_style.layout.justify = Justify::Start;
        }

        {
            let a_style = dom.node_mut(a).unwrap().style_mut().unwrap();
            a_style.size = Size {
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
            b_style.size = Size {
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
            c_style.size = Size {
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
