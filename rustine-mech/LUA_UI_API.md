# Rustine Lua UI Scripting

This document describes the Lua UI scripting system for Rustine, enabling declarative UI definition with reactive state management.

## Overview

The Lua UI system allows you to define game UIs entirely in Lua scripts, using a simple drawing API that integrates with Rustine's command-buffer-based rendering. UIs can respond to mouse interactions (hover, press, click) and be hot-reloaded during development.

## Architecture

```
┌─────────────────┐
│   Lua Script    │ <- Define UI in Lua
│   (game_ui.lua) │
└────────┬────────┘
         │
         v
┌─────────────────┐
│   lua_ui.rs     │ <- Rust<->Lua bindings
│   (FFI layer)   │
└────────┬────────┘
         │
         v
┌─────────────────┐
│   RenderFrame   │ <- Command buffer
│   (gfx.rs)      │
└─────────────────┘
```

## Core API

### Drawing Functions

#### `ui.rect(x, y, w, h, color)`
Draw a filled rectangle.
- `x, y`: Position (top-left corner)
- `w, h`: Size
- `color`: 32-bit RGBA color (0xRRGGBBAA format)

```lua
-- Draw a blue rectangle
ui.rect(100, 100, 200, 50, 0x79C0FFFF)
```

#### `ui.text(text, x, y, scale, color, font_id)`
Draw text at a position.
- `text`: String to display
- `x, y`: Position (baseline)
- `scale`: Text scale (1.0 = normal)
- `color`: Text color
- `font_id`: Font constant (FONT_CASKAYDIA_MONO, etc.)

```lua
ui.text("Hello World", 20, 40, 1.0, 0xFFFFFFFF, FONT_CASKAYDIA_MONO)
```

#### `ui.push_scissor(x, y, w, h)` / `ui.pop_scissor()`
Push/pop scissor rectangles for clipping.

```lua
ui.push_scissor(0, 0, 300, 400)
-- Everything drawn here is clipped to the scissor
ui.pop_scissor()
```

### Input Functions

#### `ui.get_window_size()`
Returns window width and height.

```lua
local w, h = ui.get_window_size()
```

#### `ui.get_mouse_pos()`
Returns mouse x, y position.

```lua
local mx, my = ui.get_mouse_pos()
```

#### `ui.is_mouse_down()`
Returns true if left mouse button is currently pressed.

```lua
if ui.is_mouse_down() then
    -- Mouse button is held
end
```

#### `ui.is_mouse_pressed()`
Returns true if left mouse button was just pressed this frame.

```lua
if ui.is_mouse_pressed() then
    -- Handle click
end
```

#### `ui.is_mouse_released()`
Returns true if left mouse button was just released this frame.

#### `ui.is_rect_hovered(x, y, w, h)`
Returns true if mouse is over the given rectangle.

```lua
local hovered = ui.is_rect_hovered(100, 100, 200, 50)
```

### Font Constants

- `FONT_PROGGY_CLEAN` (0) - 16pt, Latin1
- `FONT_DEPARTURE_MONO` (1) - 22pt, Latin1
- `FONT_CASKAYDIA_MONO` (2) - 16pt, Unicode + NerdFont symbols
- `FONT_NERD_SYMBOLS` (3) - 22pt, NerdFont icons only

## Widget Library (`ui_lib.lua`)

The `ui_lib.lua` module provides higher-level widgets built on the core API.

### Colors

```lua
local ui_lib = require("ui_lib")

-- GitHub Dark theme colors
ui_lib.colors.bg_primary      -- 0x0D1117FF
ui_lib.colors.bg_secondary    -- 0x161B22FF
ui_lib.colors.bg_tertiary     -- 0x21262DFF
ui_lib.colors.text_primary    -- 0xFFFFFFFF
ui_lib.colors.text_secondary  -- 0x8B949EFF
ui_lib.colors.accent_blue     -- 0x79C0FFFF
ui_lib.colors.accent_green    -- 0x3FB950FF
-- ... and more
```

### Widgets

#### Button

```lua
local state = ui_lib.button("Click Me", x, y, width, height)

if state.clicked then
    -- Handle button click
end

-- state.hovered - true if mouse is over button
-- state.pressed - true if button is being pressed
```

#### Panel

```lua
ui_lib.panel(x, y, width, height, color)
-- color is optional, defaults to bg_secondary
```

#### Label

```lua
ui_lib.label("Status: Ready", x, y, color, font)
-- color and font are optional
```

#### Progress Bar

```lua
ui_lib.progress_bar(x, y, width, height, current_value, max_value)
-- Shows a filled progress bar
```

#### Resource Display (for game UIs)

```lua
ui_lib.resource("", "Metal", 1250, x, y, ui_lib.colors.accent_blue)
-- Icon (NerdFont), label, value, position, color
```

### Layout Helpers

#### Vertical Stack

```lua
local stack = ui_lib.vstack(x, y, spacing)

local y1 = stack.next(height1)  -- Returns y position, advances stack
local y2 = stack.next(height2)
local y3 = stack.next(height3)
```

#### Horizontal Stack

```lua
local stack = ui_lib.hstack(x, y, spacing)

local x1 = stack.next(width1)
local x2 = stack.next(width2)
```

## Example: Simple UI

```lua
function render_ui()
    local w, h = ui.get_window_size()
    
    -- Background
    ui.rect(0, 0, w, h, 0x0D1117FF)
    
    -- Title
    ui.text("My Game", 20, 40, 1.5, 0xFFFFFFFF, FONT_CASKAYDIA_MONO)
    
    -- Button
    local btn_state = ui_lib.button("Start Game", w/2 - 75, h/2, 150, 40)
    if btn_state.clicked then
        log("Game started!")
    end
end
```

## Example: Game UI with State

See `lua/game_ui.lua` for a complete example of a resource management game UI with:
- Resource displays (metal, energy, credits)
- Building list with buttons
- Interactive control panel
- State management

## Integration in Rust

```rust
use rustine::{lua::LuaEngine, lua_ui};

// Create Lua engine
let mut lua = LuaEngine::new();
lua.register_log_function();
lua_ui::register_ui_functions(&mut lua);

// Load UI script
lua.execute(&script).unwrap();

// In render loop:
lua_ui::set_current_frame(frame);
lua.call_function("render_ui", &[]).unwrap();
lua_ui::clear_current_frame();
```

## Hot Reload

Press `R` to reload the UI script without restarting the application (in rustine-mech).

## Best Practices

1. **Organize with modules**: Split large UIs into separate Lua files using `require()`
2. **Cache expensive calculations**: Store layout calculations in module-level variables
3. **Use widget library**: Build on `ui_lib.lua` rather than raw API calls
4. **Minimize Lua<->Rust calls**: Batch state updates from Rust side
5. **Profile rendering**: Keep `render_ui()` fast (< 1ms for 60 FPS)

## Limitations

- No keyboard input forwarding yet (planned)
- No image rendering from Lua yet (planned)
- Mouse scroll events not exposed yet (planned)
- Single-threaded rendering (Lua state is not thread-safe)

## Future Enhancements

- [ ] Animation/tweening support
- [ ] Input text fields
- [ ] Dropdown menus
- [ ] Drag and drop
- [ ] Scrollable containers
- [ ] Flexbox/grid layout system
- [ ] Theme system
- [ ] Widget state persistence
