-- Simple Demo UI - Minimal example without mouse input
-- This demonstrates basic Lua UI rendering

function render_ui()
    local w, h = ui.get_window_size()
    
    -- Background
    ui.rect(0, 0, w, h, 0x0D1117FF)
    
    -- Title
    ui.text("Rustine Mech - Lua UI System", 20, 40, 1.2, 0xFFFFFFFF, FONT_CASKAYDIA_MONO)
    ui.text("Press R to reload this script, Q to quit", 20, 70, 1.0, 0x8B949EFF, FONT_CASKAYDIA_MONO)
    
    -- Draw some UI elements to demonstrate capabilities
    draw_demo_panels(w, h)
end

function draw_demo_panels(w, h)
    -- Color palette
    local bg_secondary = 0x161B22FF
    local bg_tertiary = 0x21262DFF
    local text_primary = 0xFFFFFFFF
    local text_secondary = 0x8B949EFF
    local accent_blue = 0x79C0FFFF
    local accent_green = 0x3FB950FF
    local accent_orange = 0xFFA657FF
    local border = 0x30363DFF
    
    -- Panel 1: Info panel
    local panel_x = 20
    local panel_y = 120
    local panel_w = 400
    local panel_h = 250
    
    ui.rect(panel_x, panel_y, panel_w, panel_h, bg_secondary)
    ui.rect(panel_x, panel_y, panel_w, 1, border)
    ui.rect(panel_x, panel_y + panel_h - 1, panel_w, 1, border)
    ui.rect(panel_x, panel_y, 1, panel_h, border)
    ui.rect(panel_x + panel_w - 1, panel_y, 1, panel_h, border)
    
    ui.text("System Information", panel_x + 15, panel_y + 35, 1.0, text_primary, FONT_CASKAYDIA_MONO)
    ui.text("Window Size: " .. w .. " x " .. h, panel_x + 15, panel_y + 70, 1.0, text_secondary, FONT_CASKAYDIA_MONO)
    ui.text("Renderer: Vulkan", panel_x + 15, panel_y + 100, 1.0, text_secondary, FONT_CASKAYDIA_MONO)
    ui.text("UI: Lua Scripting", panel_x + 15, panel_y + 130, 1.0, text_secondary, FONT_CASKAYDIA_MONO)
    
    -- Status indicators with NerdFont icons
    ui.text("", panel_x + 15, panel_y + 180, 1.0, accent_green, FONT_NERD_SYMBOLS)
    ui.text("Lua Engine: Ready", panel_x + 45, panel_y + 180, 1.0, accent_green, FONT_CASKAYDIA_MONO)
    
    ui.text("", panel_x + 15, panel_y + 210, 1.0, accent_blue, FONT_NERD_SYMBOLS)
    ui.text("Graphics: Active", panel_x + 45, panel_y + 210, 1.0, accent_blue, FONT_CASKAYDIA_MONO)
    
    -- Panel 2: Progress bars
    local panel2_x = 440
    local panel2_y = 120
    local panel2_w = w - panel2_x - 20
    local panel2_h = 250
    
    ui.rect(panel2_x, panel2_y, panel2_w, panel2_h, bg_secondary)
    ui.rect(panel2_x, panel2_y, panel2_w, 1, border)
    ui.rect(panel2_x, panel2_y + panel2_h - 1, panel2_w, 1, border)
    ui.rect(panel2_x, panel2_y, 1, panel2_h, border)
    ui.rect(panel2_x + panel2_w - 1, panel2_y, 1, panel2_h, border)
    
    ui.text("Resource Demo", panel2_x + 15, panel2_y + 35, 1.0, text_primary, FONT_CASKAYDIA_MONO)
    
    -- Progress bar function
    local function progress_bar(x, y, width, height, value, color, label)
        -- Background
        ui.rect(x, y, width, height, bg_tertiary)
        -- Fill
        local fill_w = math.floor(width * value)
        if fill_w > 0 then
            ui.rect(x, y, fill_w, height, color)
        end
        -- Border
        ui.rect(x, y, width, 1, border)
        ui.rect(x, y + height - 1, width, 1, border)
        ui.rect(x, y, 1, height, border)
        ui.rect(x + width - 1, y, 1, height, border)
        -- Label
        ui.text(label, x + 5, y + 20, 0.9, text_primary, FONT_CASKAYDIA_MONO)
        ui.text(math.floor(value * 100) .. "%", x + width - 50, y + 20, 0.9, text_secondary, FONT_CASKAYDIA_MONO)
    end
    
    local bar_width = panel2_w - 30
    progress_bar(panel2_x + 15, panel2_y + 60, bar_width, 30, 0.75, accent_green, "Metal")
    progress_bar(panel2_x + 15, panel2_y + 110, bar_width, 30, 0.45, accent_orange, "Energy")
    progress_bar(panel2_x + 15, panel2_y + 160, bar_width, 30, 0.90, accent_blue, "Credits")
    
    -- Bottom status bar
    local status_y = h - 40
    ui.rect(0, status_y, w, 40, bg_tertiary)
    ui.text("Ready - Edit lua/simple_demo.lua to customize this UI", 20, status_y + 25, 1.0, text_secondary, FONT_CASKAYDIA_MONO)
end

-- Initialize
log("Simple demo UI loaded")
