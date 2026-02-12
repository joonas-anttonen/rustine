#![allow(dead_code)]

use crate::Vector2f;
use super::dom::{self, NodeId};
use std::collections::HashMap;

/// Style state for a node based on user interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleState {
    /// Default state.
    Base,
    /// Mouse is hovering over the node.
    Hovered,
    /// Node is being actively interacted with (e.g., mouse button pressed).
    Active,
    /// Node has keyboard focus.
    Focused,
}

/// Container for style rules that can be applied based on state.
#[derive(Debug, Clone, Default)]
pub struct StyleRules {
    pub base: dom::StyleOverride,
    pub hovered: Option<dom::StyleOverride>,
    pub active: Option<dom::StyleOverride>,
    pub focused: Option<dom::StyleOverride>,
}

impl StyleRules {
    pub fn new(base: dom::StyleOverride) -> Self {
        Self {
            base,
            hovered: None,
            active: None,
            focused: None,
        }
    }

    pub fn with_hovered(mut self, override_style: dom::StyleOverride) -> Self {
        self.hovered = Some(override_style);
        self
    }

    pub fn with_active(mut self, override_style: dom::StyleOverride) -> Self {
        self.active = Some(override_style);
        self
    }

    pub fn with_focused(mut self, override_style: dom::StyleOverride) -> Self {
        self.focused = Some(override_style);
        self
    }

    /// Compute the final style override by merging based on the state.
    pub fn compute(&self, state: StyleState) -> dom::StyleOverride {
        let mut result = self.base;

        match state {
            StyleState::Base => {}
            StyleState::Hovered => {
                if let Some(ref override_style) = self.hovered {
                    result.merge_from(override_style);
                }
            }
            StyleState::Active => {
                if let Some(ref override_style) = self.hovered {
                    result.merge_from(override_style);
                }
                if let Some(ref override_style) = self.active {
                    result.merge_from(override_style);
                }
            }
            StyleState::Focused => {
                if let Some(ref override_style) = self.focused {
                    result.merge_from(override_style);
                }
            }
        }

        result
    }
}

/// Tracks input state and computes style overrides for DOM nodes.
///
/// The StyleComputer works on top of the DOM's embedded styles, applying
/// additional overrides based on interaction state (hover, active, focus).
pub struct StyleComputer {
    /// Current mouse position in screen coordinates.
    mouse_position: Vector2f,
    /// Node currently under the mouse cursor.
    hovered_node: Option<NodeId>,
    /// Node being actively pressed.
    active_node: Option<NodeId>,
    /// Node with keyboard focus.
    focused_node: Option<NodeId>,
    /// Style rules per node.
    rules: HashMap<NodeId, StyleRules>,
    /// Computed style overrides cache.
    computed: HashMap<NodeId, dom::StyleOverride>,
}

impl StyleComputer {
    pub fn new() -> Self {
        Self {
            mouse_position: Vector2f::default(),
            hovered_node: None,
            active_node: None,
            focused_node: None,
            rules: HashMap::new(),
            computed: HashMap::new(),
        }
    }

    /// Set style rules for a node.
    pub fn set_rules(&mut self, node_id: NodeId, rules: StyleRules) {
        self.rules.insert(node_id, rules);
        self.invalidate_node(node_id);
    }

    /// Update mouse position and recompute hover state.
    pub fn update_mouse_position(&mut self, position: Vector2f, dom: &dom::Dom) -> bool {
        self.mouse_position = position;
        let new_hovered = self.resolve_interactive_target(dom, position);
        self.set_hovered(new_hovered)
    }

    /// Set hovered node directly (e.g., on mouse leave).
    pub fn set_hovered(&mut self, node_id: Option<NodeId>) -> bool {
        if self.hovered_node != node_id {
            if let Some(old_hovered) = self.hovered_node {
                self.invalidate_node(old_hovered);
            }
            if let Some(new_hovered) = node_id {
                self.invalidate_node(new_hovered);
            }
            self.hovered_node = node_id;
            true
        } else {
            false
        }
    }

    /// Set active node (e.g., on mouse button press).
    pub fn set_active(&mut self, node_id: Option<NodeId>) -> bool {
        if self.active_node != node_id {
            if let Some(old_active) = self.active_node {
                self.invalidate_node(old_active);
            }
            if let Some(new_active) = node_id {
                self.invalidate_node(new_active);
            }
            self.active_node = node_id;
            true
        } else {
            false
        }
    }

    /// Set focused node (e.g., on keyboard focus).
    pub fn set_focused(&mut self, node_id: Option<NodeId>) -> bool {
        if self.focused_node != node_id {
            if let Some(old_focused) = self.focused_node {
                self.invalidate_node(old_focused);
            }
            if let Some(new_focused) = node_id {
                self.invalidate_node(new_focused);
            }
            self.focused_node = node_id;
            true
        } else {
            false
        }
    }

    /// Get the computed style override for a node, or None if no rules exist.
    ///
    /// The returned override should be applied on top of the node's base style.
    pub fn get_override(&mut self, node_id: NodeId) -> Option<&dom::StyleOverride> {
        if !self.computed.contains_key(&node_id) {
            let state = self.compute_state(node_id);
            let rules = self.rules.get(&node_id)?;
            let override_style = rules.compute(state);
            self.computed.insert(node_id, override_style);
        }
        self.computed.get(&node_id)
    }

    /// Apply the computed style override to a mutable style reference.
    pub fn apply_to_style(&mut self, node_id: NodeId, style: &mut dom::Style) {
        if let Some(override_style) = self.get_override(node_id) {
            style.apply_override(override_style);
        }
    }

    /// Invalidate cached style for a node.
    fn invalidate_node(&mut self, node_id: NodeId) {
        self.computed.remove(&node_id);
    }

    fn resolve_interactive_target(
        &self,
        dom: &dom::Dom,
        position: Vector2f,
    ) -> Option<NodeId> {
        let mut target = dom.hit_test(position);
        while let Some(id) = target {
            if self.rules.contains_key(&id) {
                return Some(id);
            }
            target = dom.node(id).and_then(|node| node.parent);
        }
        None
    }

    /// Compute the current state for a node based on input tracking.
    fn compute_state(&self, node_id: NodeId) -> StyleState {
        if Some(node_id) == self.active_node {
            StyleState::Active
        } else if Some(node_id) == self.focused_node {
            StyleState::Focused
        } else if Some(node_id) == self.hovered_node {
            StyleState::Hovered
        } else {
            StyleState::Base
        }
    }

    pub fn hovered_node(&self) -> Option<NodeId> {
        self.hovered_node
    }

    pub fn active_node(&self) -> Option<NodeId> {
        self.active_node
    }

    pub fn focused_node(&self) -> Option<NodeId> {
        self.focused_node
    }
}

impl Default for StyleComputer {
    fn default() -> Self {
        Self::new()
    }
}
