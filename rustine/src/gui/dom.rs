#![allow(dead_code)]

use crate::{Color, Vector2f};

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

	fn layout_node(&mut self, id: NodeId, rect: LayoutRect) {
		let (children, style) = match self.nodes.get(id) {
			Some(node) => (node.children.clone(), node.style().cloned()),
			None => return,
		};

		if let Some(node) = self.nodes.get_mut(id) {
			node.layout = rect;
		}

		let Some(style) = style else {
			return;
		};

		let content_rect = rect.inset(style.border.add(style.padding));
		let flow_children: Vec<NodeId> = children
			.iter()
			.copied()
			.filter(|child_id| {
				self.nodes
					.get(*child_id)
					.and_then(|node| node.style())
					.map(|child_style| child_style.position.mode == PositionMode::Flow)
					.unwrap_or(false)
			})
			.collect();

		for child_id in children.iter().copied() {
			let child_style = match self
				.nodes
				.get(child_id)
				.and_then(|node| node.style())
			{
				Some(style) => style.clone(),
				None => continue,
			};
			let child_content_size = self
				.nodes
				.get(child_id)
				.map(|node| node.content_size)
				.unwrap_or_default();

			if child_style.position.mode == PositionMode::Absolute {
				let child_rect =
					Self::layout_absolute(&content_rect, &child_style, child_content_size);
				self.layout_node(child_id, child_rect);
			}
		}

		self.layout_flow_children(&content_rect, &style.layout, &flow_children);
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
		let mut child_styles = Vec::with_capacity(children.len());

		for child_id in children.iter().copied() {
			let style = match self
				.nodes
				.get(child_id)
				.and_then(|node| node.style())
			{
				Some(style) => style.clone(),
				None => continue,
			};
			let content_size = self
				.nodes
				.get(child_id)
				.map(|node| node.content_size)
				.unwrap_or_default();

			let (main, _) =
				Self::resolve_child_size(parent_rect, layout, &style, content_size, 0.0);
			let main_margin = Self::main_margin(layout, &style.margin);
			if matches!(Self::main_length(layout, &style.size), Length::Fill) {
				fill_count += 1;
			} else {
				fixed_main += main + main_margin;
			}
			child_styles.push((child_id, style, content_size));
		}

		let base_gap = layout.gap.max(0.0);
		let gap_total = base_gap * (child_styles.len().saturating_sub(1) as f32);
		let mut available_main = Self::main_size(layout, parent_rect) - fixed_main - gap_total;
		if available_main < 0.0 {
			available_main = 0.0;
		}
		let fill_share = if fill_count > 0 {
			available_main / fill_count as f32
		} else {
			0.0
		};

		let mut main_sizes = Vec::with_capacity(child_styles.len());
		let mut total_main = gap_total;
		for (_, style, content_size) in child_styles.iter() {
			let (main, _) = Self::resolve_child_size(
				parent_rect,
				layout,
				style,
				*content_size,
				fill_share,
			);
			let main_margin = Self::main_margin(layout, &style.margin);
			total_main += main + main_margin;
			main_sizes.push(main);
		}

		let mut extra_space = Self::main_size(layout, parent_rect) - total_main;
		if extra_space < 0.0 {
			extra_space = 0.0;
		}

		let (start_offset, extra_gap) = match layout.justify_content {
			JustifyContent::Start => (0.0, 0.0),
			JustifyContent::Center => (extra_space * 0.5, 0.0),
			JustifyContent::End => (extra_space, 0.0),
			JustifyContent::SpaceBetween => {
				if child_styles.len() > 1 {
					(0.0, extra_space / (child_styles.len() - 1) as f32)
				} else {
					(0.0, 0.0)
				}
			}
			JustifyContent::SpaceAround => {
				if child_styles.is_empty() {
					(0.0, 0.0)
				} else {
					let gap = extra_space / child_styles.len() as f32;
					(gap * 0.5, gap)
				}
			}
			JustifyContent::SpaceEvenly => {
				if child_styles.is_empty() {
					(0.0, 0.0)
				} else {
					let gap = extra_space / (child_styles.len() as f32 + 1.0);
					(gap, gap)
				}
			}
		};

		let gap = base_gap + extra_gap;
		let mut cursor = start_offset;
		for ((child_id, style, content_size), main_size) in
			child_styles.iter().zip(main_sizes.iter())
		{
			let (main, cross) = Self::resolve_child_size(
				parent_rect,
				layout,
				style,
				*content_size,
				fill_share,
			);
			let cross_size = Self::resolve_cross_size(parent_rect, layout, style, cross);
			let margin = &style.margin;
			let (pos, size) = match layout.direction {
				LayoutDirection::Row => {
					let x = parent_rect.position.x + cursor + margin.left;
					let y = Self::align_cross(parent_rect, layout, cross_size, margin);
					(
						Vector2f::new(x, y),
						Vector2f::new(main, cross_size),
					)
				}
				LayoutDirection::Column => {
					let x = Self::align_cross(parent_rect, layout, cross_size, margin);
					let y = parent_rect.position.y + cursor + margin.top;
					(
						Vector2f::new(x, y),
						Vector2f::new(cross_size, main),
					)
				}
			};
			let child_rect = LayoutRect { position: pos, size };
			self.layout_node(*child_id, child_rect);

			let main_margin = Self::main_margin(layout, margin);
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
		Vector2f::new(
			(clamped.x + edge_x).max(0.0),
			(clamped.y + edge_y).max(0.0),
		)
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
		}
	}

	pub fn as_div_mut(&mut self) -> Option<&mut Div> {
		match &mut self.kind {
			NodeKind::Div(div) => Some(div),
		}
	}

	pub fn style(&self) -> Option<&Style> {
		self.as_div().map(|div| &div.style)
	}

	pub fn style_mut(&mut self) -> Option<&mut Style> {
		self.as_div_mut().map(|div| &mut div.style)
	}
}

#[derive(Debug, Clone)]
pub enum NodeKind {
	Div(Div),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutDirection {
	Row,
	Column,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignItems {
	Start,
	Center,
	End,
	Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustifyContent {
	Start,
	Center,
	End,
	SpaceBetween,
	SpaceAround,
	SpaceEvenly,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
	Auto,
	Px(f32),
	Percent(f32),
	Fill,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionMode {
	Flow,
	Absolute,
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
			position: Vector2f::new(self.position.x + padding.left, self.position.y + padding.top),
			size: Vector2f::new(width, height),
		}
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
}


