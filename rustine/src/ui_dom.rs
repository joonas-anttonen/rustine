#![allow(dead_code)]

//! UI DOM (Document Object Model) system
//! 
//! Provides a retained-mode UI architecture where Lua builds a tree structure
//! that Rust maintains for efficient hit-testing and rendering.

use crate::gfx;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

/// Unique identifier for UI elements
pub type ElementId = String;

/// Layout mode for containers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// Stack children vertically
    Vertical,
    /// Stack children horizontally
    Horizontal,
    /// Absolute positioning (children specify their own positions)
    Absolute,
}

/// Style properties for an element
#[derive(Debug, Clone)]
pub struct Style {
    pub bg_color: Option<u32>,
    pub text_color: Option<u32>,
    pub border_color: Option<u32>,
    pub border_width: Option<f32>,
    pub padding: Option<f32>,
    pub margin: Option<f32>,
    pub font_size: Option<f32>,
    pub font_id: Option<u32>,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            bg_color: None,
            text_color: None,
            border_color: None,
            border_width: None,
            padding: None,
            margin: None,
            font_size: None,
            font_id: None,
        }
    }
}

impl Style {
    /// Merge this style with another, using the other's values when present
    pub fn merge(&self, other: &Style) -> Style {
        Style {
            bg_color: other.bg_color.or(self.bg_color),
            text_color: other.text_color.or(self.text_color),
            border_color: other.border_color.or(self.border_color),
            border_width: other.border_width.or(self.border_width),
            padding: other.padding.or(self.padding),
            margin: other.margin.or(self.margin),
            font_size: other.font_size.or(self.font_size),
            font_id: other.font_id.or(self.font_id),
        }
    }
}

/// State-dependent styles (normal, hover, pressed)
#[derive(Debug, Clone)]
pub struct StatefulStyle {
    pub normal: Style,
    pub hover: Option<Style>,
    pub pressed: Option<Style>,
}

impl StatefulStyle {
    pub fn new(normal: Style) -> Self {
        Self {
            normal,
            hover: None,
            pressed: None,
        }
    }

    /// Get the appropriate style based on element state
    pub fn get_style(&self, hovered: bool, pressed: bool) -> Style {
        if pressed {
            if let Some(ref pressed_style) = self.pressed {
                return self.normal.merge(pressed_style);
            }
        }
        if hovered {
            if let Some(ref hover_style) = self.hover {
                return self.normal.merge(hover_style);
            }
        }
        self.normal.clone()
    }
}

/// Layout properties
#[derive(Debug, Clone)]
pub struct Layout {
    pub x: f32,
    pub y: f32,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub min_width: Option<f32>,
    pub min_height: Option<f32>,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: None,
            height: None,
            min_width: None,
            min_height: None,
        }
    }
}

/// Computed layout after layout pass
#[derive(Debug, Clone, Copy)]
pub struct ComputedLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// UI element state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementState {
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
}

impl Default for ElementState {
    fn default() -> Self {
        Self {
            hovered: false,
            pressed: false,
            focused: false,
        }
    }
}

/// UI node types
#[derive(Debug, Clone)]
pub enum UiNode {
    /// Container panel
    Panel {
        id: Option<ElementId>,
        style: StatefulStyle,
        layout: Layout,
        layout_mode: LayoutMode,
        children: Vec<Rc<RefCell<UiNode>>>,
    },
    /// Text element
    Text {
        id: Option<ElementId>,
        text: String,
        style: StatefulStyle,
        layout: Layout,
    },
    /// Button element
    Button {
        id: Option<ElementId>,
        text: String,
        style: StatefulStyle,
        layout: Layout,
        on_click: Option<String>, // Lua callback function name
    },
}

impl UiNode {
    pub fn id(&self) -> Option<&str> {
        match self {
            UiNode::Panel { id, .. } => id.as_deref(),
            UiNode::Text { id, .. } => id.as_deref(),
            UiNode::Button { id, .. } => id.as_deref(),
        }
    }

    pub fn layout(&self) -> &Layout {
        match self {
            UiNode::Panel { layout, .. } => layout,
            UiNode::Text { layout, .. } => layout,
            UiNode::Button { layout, .. } => layout,
        }
    }

    pub fn layout_mut(&mut self) -> &mut Layout {
        match self {
            UiNode::Panel { layout, .. } => layout,
            UiNode::Text { layout, .. } => layout,
            UiNode::Button { layout, .. } => layout,
        }
    }

    pub fn children(&self) -> Vec<Rc<RefCell<UiNode>>> {
        match self {
            UiNode::Panel { children, .. } => children.clone(),
            _ => vec![],
        }
    }

    pub fn has_children(&self) -> bool {
        match self {
            UiNode::Panel { children, .. } => !children.is_empty(),
            _ => false,
        }
    }
}

/// UI Document Object Model
pub struct UiDom {
    root: Option<UiNode>,
    element_states: HashMap<ElementId, ElementState>,
    computed_layouts: HashMap<ElementId, ComputedLayout>,
    mouse_x: f32,
    mouse_y: f32,
    mouse_down: bool,
    mouse_pressed: bool,
}

impl UiDom {
    pub fn new() -> Self {
        Self {
            root: None,
            element_states: HashMap::new(),
            computed_layouts: HashMap::new(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_down: false,
            mouse_pressed: false,
        }
    }

    /// Set the root node of the DOM
    pub fn set_root(&mut self, root: UiNode) {
        self.root = Some(root);
        self.invalidate_layout();
    }

    /// Get the root node
    pub fn root(&self) -> Option<&UiNode> {
        self.root.as_ref()
    }

    /// Update mouse state
    pub fn update_mouse(&mut self, x: f32, y: f32, down: bool, pressed: bool) {
        self.mouse_x = x;
        self.mouse_y = y;
        self.mouse_down = down;
        self.mouse_pressed = pressed;

        // Update hover states
        self.update_hover_states();
    }

    /// Invalidate layout (triggers recomputation)
    pub fn invalidate_layout(&mut self) {
        self.computed_layouts.clear();
    }

    /// Compute layout for the entire tree
    pub fn compute_layout(&mut self, window_width: f32, window_height: f32) {
        self.computed_layouts.clear();
        
        if let Some(ref root) = self.root {
            self.compute_node_layout_recursive(root, 0.0, 0.0, window_width, window_height);
        }
    }

    /// Recursively compute layout for a node and its children
    fn compute_node_layout_recursive(
        &mut self,
        node: &UiNode,
        parent_x: f32,
        parent_y: f32,
        available_width: f32,
        available_height: f32,
    ) {
        let layout = node.layout();
        let x = parent_x + layout.x;
        let y = parent_y + layout.y;
        
        let width = layout.width.unwrap_or(available_width);
        let height = layout.height.unwrap_or(available_height);

        // Store computed layout
        if let Some(id) = node.id() {
            self.computed_layouts.insert(
                id.to_string(),
                ComputedLayout {
                    x,
                    y,
                    width,
                    height,
                },
            );
        }

        // Compute children layouts
        if let UiNode::Panel { layout_mode, children, .. } = node {
            match layout_mode {
                LayoutMode::Vertical => {
                    let mut current_y = 0.0;
                    for child_rc in children {
                        let mut child = child_rc.borrow_mut();
                        child.layout_mut().y = current_y;
                        
                        // Need to drop the mutable borrow before recursive call
                        let child_id = child.id().map(|s| s.to_string());
                        drop(child);
                        
                        // Now we can recursively compute without holding the borrow
                        self.compute_node_layout_recursive(&child_rc.borrow(), x, y, width, height - current_y);
                        
                        // Get child's computed height
                        if let Some(child_id) = child_id {
                            if let Some(computed) = self.computed_layouts.get(&child_id) {
                                current_y += computed.height;
                            }
                        }
                    }
                }
                LayoutMode::Horizontal => {
                    let mut current_x = 0.0;
                    for child_rc in children {
                        let mut child = child_rc.borrow_mut();
                        child.layout_mut().x = current_x;
                        
                        // Need to drop the mutable borrow before recursive call
                        let child_id = child.id().map(|s| s.to_string());
                        drop(child);
                        
                        // Now we can recursively compute without holding the borrow
                        self.compute_node_layout_recursive(&child_rc.borrow(), x, y, width - current_x, height);
                        
                        // Get child's computed width
                        if let Some(child_id) = child_id {
                            if let Some(computed) = self.computed_layouts.get(&child_id) {
                                current_x += computed.width;
                            }
                        }
                    }
                }
                LayoutMode::Absolute => {
                    for child_rc in children {
                        self.compute_node_layout_recursive(&child_rc.borrow(), x, y, width, height);
                    }
                }
            }
        }
    }

    /// Update hover states based on mouse position
    fn update_hover_states(&mut self) {
        self.element_states.clear();
        
        if let Some(ref root) = self.root {
            self.update_node_hover_state(root);
        }
    }

    /// Recursively update hover state for a node
    fn update_node_hover_state(&mut self, node: &UiNode) {
        if let Some(id) = node.id() {
            if let Some(computed) = self.computed_layouts.get(id) {
                let hovered = self.mouse_x >= computed.x
                    && self.mouse_x <= computed.x + computed.width
                    && self.mouse_y >= computed.y
                    && self.mouse_y <= computed.y + computed.height;

                let pressed = hovered && self.mouse_down;

                self.element_states.insert(
                    id.to_string(),
                    ElementState {
                        hovered,
                        pressed,
                        focused: false,
                    },
                );
            }
        }

        // Recurse into children
        for child_rc in node.children() {
            self.update_node_hover_state(&child_rc.borrow());
        }
    }

    /// Get the state of an element
    pub fn get_element_state(&self, id: &str) -> ElementState {
        self.element_states
            .get(id)
            .copied()
            .unwrap_or_default()
    }

    /// Get computed layout for an element
    pub fn get_computed_layout(&self, id: &str) -> Option<ComputedLayout> {
        self.computed_layouts.get(id).copied()
    }

    /// Render the DOM tree to a RenderFrame
    pub fn render(&self, frame: &mut gfx::RenderFrame) {
        if let Some(ref root) = self.root {
            self.render_node(root, frame);
        }
    }

    /// Recursively render a node
    fn render_node(&self, node: &UiNode, frame: &mut gfx::RenderFrame) {
        let state = node.id()
            .and_then(|id| self.element_states.get(id))
            .copied()
            .unwrap_or_default();

        let computed = node.id()
            .and_then(|id| self.computed_layouts.get(id))
            .copied();

        if let Some(layout) = computed {
            match node {
                UiNode::Panel { style, children, .. } => {
                    let current_style = style.get_style(state.hovered, state.pressed);
                    
                    // Draw background
                    if let Some(bg_color) = current_style.bg_color {
                        frame.fill_rectangle(
                            &gfx::Rectangle {
                                x: layout.x,
                                y: layout.y,
                                w: layout.width,
                                h: layout.height,
                            },
                            bg_color,
                        );
                    }

                    // Draw border
                    if let (Some(border_color), Some(border_width)) =
                        (current_style.border_color, current_style.border_width)
                    {
                        let rect = gfx::Rectangle {
                            x: layout.x,
                            y: layout.y,
                            w: layout.width,
                            h: layout.height,
                        };
                        frame.draw_rectangle(&rect, border_width, border_color);
                    }

                    // Render children
                    for child_rc in children {
                        self.render_node(&child_rc.borrow(), frame);
                    }
                }
                UiNode::Text { text, style, .. } => {
                    let current_style = style.get_style(state.hovered, state.pressed);
                    let color = current_style.text_color.unwrap_or(0xFFFFFFFF);
                    let scale = current_style.font_size.unwrap_or(1.0);
                    let font_id = current_style.font_id.unwrap_or(gfx::fonts::CASKAYDIAMONO_FONT_ID);

                    frame.push_text(text, layout.x, layout.y + 20.0, scale, color, font_id);
                }
                UiNode::Button { text, style, .. } => {
                    let current_style = style.get_style(state.hovered, state.pressed);

                    // Draw background
                    if let Some(bg_color) = current_style.bg_color {
                        frame.fill_rectangle(
                            &gfx::Rectangle {
                                x: layout.x,
                                y: layout.y,
                                w: layout.width,
                                h: layout.height,
                            },
                            bg_color,
                        );
                    }

                    // Draw border
                    if let (Some(border_color), Some(border_width)) =
                        (current_style.border_color, current_style.border_width)
                    {
                        let rect = gfx::Rectangle {
                            x: layout.x,
                            y: layout.y,
                            w: layout.width,
                            h: layout.height,
                        };
                        frame.draw_rectangle(&rect, border_width, border_color);
                    }

                    // Draw text (centered)
                    let text_color = current_style.text_color.unwrap_or(0xFFFFFFFF);
                    let font_size = current_style.font_size.unwrap_or(1.0);
                    let font_id = current_style.font_id.unwrap_or(gfx::fonts::CASKAYDIAMONO_FONT_ID);
                    
                    // Approximate centering
                    let text_x = layout.x + layout.width / 2.0 - (text.len() as f32 * 8.0 * font_size) / 2.0;
                    let text_y = layout.y + layout.height / 2.0 + 6.0;
                    
                    frame.push_text(text, text_x, text_y, font_size, text_color, font_id);
                }
            }
        }
    }

    /// Check if an element was clicked
    pub fn was_clicked(&self, id: &str) -> bool {
        if let Some(state) = self.element_states.get(id) {
            state.hovered && self.mouse_pressed
        } else {
            false
        }
    }
}

impl Default for UiDom {
    fn default() -> Self {
        Self::new()
    }
}
