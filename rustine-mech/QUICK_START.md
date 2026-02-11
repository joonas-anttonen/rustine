# Rustine Lua UI - Quick Start Guide

Get started with Lua UI scripting in 5 minutes!

## Running the Demo

```bash
cd rustine-mech
cargo run
```

**Controls:**
- `R` - Reload the Lua UI script (hot reload)
- `Q` or `Escape` - Quit

## Your First UI

Create `lua/my_ui.lua`:

```lua
function render_ui()
    -- Get window size
    local w, h = ui.get_window_size()
    
    -- Draw background
    ui.rect(0, 0, w, h, 0x0D1117FF)  -- Dark gray
    
    -- Draw text
    ui.text("Hello Rustine!", 50, 50, 1.0, 0xFFFFFFFF, FONT_CASKAYDIA_MONO)
end
```

Change `main.rs` to load your script:
```rust
let script_path = Path::new("lua/my_ui.lua");
```

## Using Widgets

```lua
local ui_lib = require("ui_lib")

function render_ui()
    local w, h = ui.get_window_size()
    
    -- Background
    ui.rect(0, 0, w, h, ui_lib.colors.bg_primary)
    
    -- Button
    local btn = ui_lib.button("Click Me!", 100, 100, 200, 50)
    if btn.clicked then
        log("Button was clicked!")
    end
    
    -- Panel
    ui_lib.panel(100, 200, 400, 300)
    
    -- Label
    ui_lib.label("Status: Ready", 120, 230)
    
    -- Progress bar
    ui_lib.progress_bar(120, 270, 360, 30, 0.75, 1.0)
end
```

## Colors

```lua
-- Use predefined colors
ui_lib.colors.bg_primary      -- Background
ui_lib.colors.text_primary    -- Text
ui_lib.colors.accent_blue     -- Highlights
ui_lib.colors.accent_green    -- Success
ui_lib.colors.accent_orange   -- Warning
ui_lib.colors.accent_red      -- Error

-- Or define your own (0xRRGGBBAA format)
local my_color = 0xFF5733FF  -- Orange with full opacity
```

## Layout

```lua
-- Vertical stack
local stack = ui_lib.vstack(50, 50, 10)  -- x, y, spacing

ui_lib.button("Button 1", 50, stack.next(40), 200, 40)
ui_lib.button("Button 2", 50, stack.next(40), 200, 40)
ui_lib.button("Button 3", 50, stack.next(40), 200, 40)

-- Horizontal stack
local hstack = ui_lib.hstack(50, 200, 10)

ui_lib.button("A", hstack.next(60), 200, 60, 40)
ui_lib.button("B", hstack.next(60), 200, 60, 40)
ui_lib.button("C", hstack.next(60), 200, 60, 40)
```

## State Management

```lua
-- Module-level state persists between frames
local counter = 0

function render_ui()
    local w, h = ui.get_window_size()
    ui.rect(0, 0, w, h, 0x0D1117FF)
    
    -- Button increments counter
    local btn = ui_lib.button("Count: " .. counter, 100, 100, 200, 50)
    if btn.clicked then
        counter = counter + 1
    end
end
```

## Icons (NerdFont)

```lua
-- Use NerdFont symbols for icons
ui.text("", 50, 50, 1.0, 0x3FB950FF, FONT_NERD_SYMBOLS)  -- Check
ui.text("", 80, 50, 1.0, 0xFF7B72FF, FONT_NERD_SYMBOLS)  -- Cross
ui.text("", 110, 50, 1.0, 0x79C0FFFF, FONT_NERD_SYMBOLS) -- Info
ui.text("", 140, 50, 1.0, 0xFFA657FF, FONT_NERD_SYMBOLS) -- Warning

-- Common icons: 
-- 🗀 folder, 📄 file,  git, 📡 wifi,  terminal
```

## Examples

### Simple Info Panel

```lua
function info_panel(x, y, w, h, title, content)
    -- Background
    ui.rect(x, y, w, h, ui_lib.colors.bg_secondary)
    
    -- Border
    ui.rect(x, y, w, 1, ui_lib.colors.border)
    ui.rect(x, y + h - 1, w, 1, ui_lib.colors.border)
    ui.rect(x, y, 1, h, ui_lib.colors.border)
    ui.rect(x + w - 1, y, 1, h, ui_lib.colors.border)
    
    -- Title
    ui.text(title, x + 10, y + 25, 1.0, ui_lib.colors.text_primary, FONT_CASKAYDIA_MONO)
    
    -- Content
    for i, line in ipairs(content) do
        ui.text(line, x + 10, y + 50 + (i - 1) * 25, 1.0, ui_lib.colors.text_secondary, FONT_CASKAYDIA_MONO)
    end
end

function render_ui()
    local w, h = ui.get_window_size()
    ui.rect(0, 0, w, h, ui_lib.colors.bg_primary)
    
    info_panel(20, 20, 400, 200, "System Info", {
        "FPS: 60",
        "Renderer: Vulkan",
        "Resolution: " .. w .. "x" .. h
    })
end
```

### Resource Display

```lua
local resources = {
    {icon = "", name = "Metal", value = 1250, color = 0x79C0FFFF},
    {icon = "", name = "Energy", value = 850, color = 0xFFA657FF},
    {icon = "", name = "Credits", value = 5400, color = 0x3FB950FF},
}

function render_ui()
    local w, h = ui.get_window_size()
    ui.rect(0, 0, w, h, ui_lib.colors.bg_primary)
    
    local stack = ui_lib.vstack(20, 20, 10)
    
    for _, res in ipairs(resources) do
        ui_lib.resource(res.icon, res.name, res.value, 20, stack.next(50), res.color)
    end
end
```

## Debugging

```lua
-- Log to console
log("Debug info: " .. tostring(value))

-- Display errors in UI
function render_ui()
    -- Wrap in pcall for error handling
    local success, err = pcall(function()
        -- Your UI code here
    end)
    
    if not success then
        ui.text("Error: " .. err, 20, 20, 1.0, 0xFF7B72FF, FONT_CASKAYDIA_MONO)
    end
end
```

## Tips

1. **Start Simple** - Begin with basic rectangles and text
2. **Use ui_lib** - Leverage the widget library instead of raw API
3. **Hot Reload** - Press R to see changes instantly
4. **Check Examples** - See `simple_demo.lua` and `game_ui.lua`
5. **Read the Docs** - Full API in `LUA_UI_API.md`

## Next Steps

- Read the full [API Documentation](LUA_UI_API.md)
- Study `lua/game_ui.lua` for a complete example
- Experiment with the widget library in `lua/ui_lib.lua`
- Build your own game UI!

Happy scripting! 🎮✨
