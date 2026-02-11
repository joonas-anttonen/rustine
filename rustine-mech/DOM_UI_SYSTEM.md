# DOM-Based UI System for Rustine

## Overview

The DOM-based UI system provides an HTML/CSS-like approach to building user interfaces in Rust with Lua scripting. Unlike the immediate-mode system where UI is redrawn every frame, the DOM system maintains a retained tree structure that is only rebuilt when needed.

## Architecture

### Retained Mode vs Immediate Mode

**Immediate Mode (Legacy):**
```
Every Frame:
  Lua render_ui() → draw calls → RenderFrame → GPU
```

**DOM Mode (New):**
```
On Invalidation:
  Lua build_ui() → DOM Tree → Layout Computation → Cache

Every Frame:
  DOM → Hit-Testing → Render with States → RenderFrame → GPU
```

### Benefits of DOM Mode

- ✅ **Efficient Hit-Testing** - Calculate once, use many times
- ✅ **HTML-like Structure** - Familiar tree-based layout
- ✅ **CSS-like Styling** - State-based styles (normal, hover, pressed)
- ✅ **Performance** - Rebuild only on change, not every frame
- ✅ **Separation of Concerns** - Structure vs. Presentation
- ✅ **Easy Debugging** - Inspect the DOM tree

## Core Data Structures

### UiNode (Element Types)

```rust
pub enum UiNode {
    Panel {
        id: Option<ElementId>,
        style: StatefulStyle,
        layout: Layout,
        layout_mode: LayoutMode,  // Vertical, Horizontal, Absolute
        children: Vec<UiNode>,
    },
    Text {
        id: Option<ElementId>,
        text: String,
        style: StatefulStyle,
        layout: Layout,
    },
    Button {
        id: Option<ElementId>,
        text: String,
        style: StatefulStyle,
        layout: Layout,
        on_click: Option<String>,  // Lua callback name
    },
}
```

### Style System

```rust
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

pub struct StatefulStyle {
    pub normal: Style,
    pub hover: Option<Style>,
    pub pressed: Option<Style>,
}
```

Styles cascade: `pressed` merges with `normal`, `hover` merges with `normal`.

### Layout System

```rust
pub struct Layout {
    pub x: f32,
    pub y: f32,
    pub width: Option<f32>,   // None = fill parent
    pub height: Option<f32>,  // None = fit content
    pub min_width: Option<f32>,
    pub min_height: Option<f32>,
}

pub enum LayoutMode {
    Vertical,    // Stack children top-to-bottom
    Horizontal,  // Stack children left-to-right
    Absolute,    // Children position themselves
}
```

### UiDom (Document Object Model)

```rust
pub struct UiDom {
    root: Option<UiNode>,
    element_states: HashMap<ElementId, ElementState>,
    computed_layouts: HashMap<ElementId, ComputedLayout>,
    // Mouse state for hit-testing
}
```

Methods:
- `set_root(node)` - Set the root UI element
- `compute_layout(w, h)` - Calculate positions/sizes
- `update_mouse(x, y, down, pressed)` - Update input state
- `render(frame)` - Draw to RenderFrame
- `was_clicked(id)` - Check if element was clicked
- `get_element_state(id)` - Get hover/pressed state

## Usage Example (Rust)

```rust
// Create a simple DOM
let mut dom = UiDom::new();

let root = UiNode::Panel {
    id: Some("root".to_string()),
    style: StatefulStyle::new(Style {
        bg_color: Some(0x0D1117FF),
        ..Default::default()
    }),
    layout: Layout {
        width: Some(800.0),
        height: Some(600.0),
        ..Default::default()
    },
    layout_mode: LayoutMode::Vertical,
    children: vec![
        UiNode::Text {
            id: Some("title".to_string()),
            text: "Hello World".to_string(),
            style: StatefulStyle::new(Style {
                text_color: Some(0xFFFFFFFF),
                font_size: Some(1.5),
                ..Default::default()
            }),
            layout: Layout {
                height: Some(50.0),
                ..Default::default()
            },
        },
        UiNode::Button {
            id: Some("btn".to_string()),
            text: "Click Me".to_string(),
            style: StatefulStyle {
                normal: Style {
                    bg_color: Some(0x161B22FF),
                    text_color: Some(0x8B949EFF),
                    border_color: Some(0x30363DFF),
                    border_width: Some(1.0),
                    ..Default::default()
                },
                hover: Some(Style {
                    bg_color: Some(0x1F2428FF),
                    text_color: Some(0xFFFFFFFF),
                    ..Default::default()
                }),
                pressed: Some(Style {
                    bg_color: Some(0x2D333BFF),
                    ..Default::default()
                }),
            },
            layout: Layout {
                width: Some(150.0),
                height: Some(40.0),
                ..Default::default()
            },
            on_click: Some("on_button_click".to_string()),
        },
    ],
};

dom.set_root(root);
dom.compute_layout(800.0, 600.0);

// In render loop
dom.update_mouse(mouse_x, mouse_y, mouse_down, mouse_pressed);
dom.render(&mut frame);

// Check for clicks
if dom.was_clicked("btn") {
    lua.call_function("on_button_click", &[]);
}
```

## Integration with Lua (Future)

### Planned API

```lua
-- Define styles
local button_style = {
    normal = {
        bg_color = 0x161B22FF,
        text_color = 0x8B949EFF,
        border_color = 0x30363DFF,
        border_width = 1.0,
    },
    hover = {
        bg_color = 0x1F2428FF,
        text_color = 0xFFFFFFFF,
    },
    pressed = {
        bg_color = 0x2D333BFF,
    }
}

-- Build UI structure
function build_ui()
    return {
        type = "panel",
        id = "root",
        style = { normal = { bg_color = 0x0D1117FF } },
        layout = "vertical",
        children = {
            {
                type = "text",
                id = "title",
                text = "Hello World",
                style = {
                    normal = {
                        text_color = 0xFFFFFFFF,
                        font_size = 1.5,
                    }
                },
                height = 50,
            },
            {
                type = "button",
                id = "start_btn",
                text = "Start Game",
                style = button_style,
                width = 150,
                height = 40,
                on_click = "on_start_click",
            },
        }
    }
end

-- Event callback
function on_start_click()
    log("Game started!")
end
```

## State Management

### Element States

```rust
pub struct ElementState {
    pub hovered: bool,   // Mouse is over element
    pub pressed: bool,   // Mouse button down over element
    pub focused: bool,   // Element has keyboard focus (future)
}
```

States are computed automatically during hit-testing based on mouse position and bounds.

### DOM Invalidation

DOM is rebuilt when:
1. Script is reloaded (R key)
2. Mode is changed to DOM (D key)
3. Explicit invalidation is requested

## Layout Algorithm

1. **Start at root** with window dimensions
2. **For each node:**
   - Use specified width/height or fill parent
   - Compute absolute position from parent + offset
3. **Layout children** based on `layout_mode`:
   - **Vertical:** Stack children vertically, advance Y
   - **Horizontal:** Stack children horizontally, advance X
   - **Absolute:** Children use their own x/y
4. **Store computed layout** for hit-testing

## Rendering Algorithm

1. **Traverse DOM tree** depth-first
2. **For each node:**
   - Get element state (hover/press)
   - Get appropriate style variant
   - Render based on type:
     - **Panel:** Fill background, draw border
     - **Text:** Render text at position
     - **Button:** Fill background, draw border, center text
3. **Render children** recursively

## Performance Characteristics

| Operation | Complexity | Frequency |
|-----------|------------|-----------|
| DOM Build | O(n) nodes | On invalidation |
| Layout Compute | O(n) nodes | On invalidation |
| Hit-Testing | O(n) nodes | Per frame |
| Rendering | O(n) nodes | Per frame |

Where n = number of nodes in DOM tree.

**Key Optimization:** Layout and structure are computed once, not every frame!

## Comparison: Immediate vs DOM

| Aspect | Immediate Mode | DOM Mode |
|--------|---------------|----------|
| Rebuild | Every frame | On invalidation |
| Hit-Testing | Manual | Automatic |
| Structure | Implicit (call order) | Explicit (tree) |
| State | Manual tracking | Automatic |
| Performance | Slower for complex UI | Faster for complex UI |
| Debugging | Hard to inspect | Easy (inspect tree) |
| Best For | Simple overlays | Complex interfaces |

## Current Status

### Implemented ✅

- UiNode enum (Panel, Text, Button)
- Style system with state variants
- Layout computation (Vertical, Horizontal, Absolute)
- Hit-testing with automatic state updates
- DOM rendering to RenderFrame
- Button click detection
- Lua callback integration
- Mode toggle (D key)

### Planned 🔄

- Lua table parsing (currently hardcoded in Rust)
- More node types (Image, Input, List, etc.)
- Flexbox-like layout engine
- Animation/transition support
- Accessibility features
- DOM inspection tools

## Testing

### Controls

- `D` - Toggle between Immediate and DOM mode
- `R` - Reload Lua script (rebuilds DOM)
- `Q` - Quit

### Example Test

1. Run rustine-mech
2. See immediate mode UI
3. Press `D` → Switch to DOM mode
4. See "DOM UI System" title and "Click Me" button
5. Hover button → Background changes (hover state working!)
6. Click button → Console logs callback (event working!)
7. Press `R` → Reload, DOM rebuilds
8. Press `D` → Back to immediate mode

## Migration Guide

### From Immediate to DOM

**Before (Immediate):**
```lua
function render_ui()
    ui.rect(100, 100, 200, 50, 0x161B22FF)
    ui.text("Click Me", 150, 125, 1.0, 0xFFFFFFFF, FONT_CASKAYDIA_MONO)
end
```

**After (DOM - future):**
```lua
function build_ui()
    return {
        type = "button",
        x = 100, y = 100,
        width = 200, height = 50,
        text = "Click Me",
        style = button_style,
        on_click = "on_click",
    }
end
```

### Benefits of Migration

- Automatic hover/press states
- Easier maintenance (structure separate from rendering)
- Better performance for complex UIs
- Easier to add animations
- Built-in hit-testing

## Future Enhancements

1. **Lua Table Parsing** - Parse DOM from Lua-returned tables
2. **More Node Types:**
   - Image (with 9-patch support)
   - TextInput (editable text)
   - List/ScrollView (scrollable containers)
   - Canvas (custom drawing)
3. **Layout Improvements:**
   - Flexbox-like system
   - Grid layout
   - Constraints (min/max width/height)
4. **Styling:**
   - CSS-like selectors
   - Style inheritance
   - Themes
5. **Animation:**
   - Transition between states
   - Keyframe animations
   - Easing functions
6. **Accessibility:**
   - Screen reader support
   - Keyboard navigation
   - Focus management

## Conclusion

The DOM-based UI system provides a modern, efficient approach to building complex user interfaces in Rustine. It combines the familiarity of HTML/CSS with the performance of a retained-mode system, while maintaining seamless integration with Lua scripting.

The system is production-ready for basic use cases and provides a solid foundation for future enhancements.
