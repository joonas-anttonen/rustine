-- UI Widget Library for Rustine
-- Provides declarative widget system built on top of low-level UI primitives

local M = {}

-- Color utilities
M.colors = {
    -- GitHub Dark theme colors
    bg_primary = 0x0D1117FF,
    bg_secondary = 0x161B22FF,
    bg_tertiary = 0x21262DFF,
    
    text_primary = 0xFFFFFFFF,
    text_secondary = 0x8B949EFF,
    text_link = 0x79C0FFFF,
    
    accent_blue = 0x79C0FFFF,
    accent_green = 0x3FB950FF,
    accent_orange = 0xFFA657FF,
    accent_red = 0xFF7B72FF,
    accent_purple = 0xD2A8FFFF,
    
    border = 0x30363DFF,
    hover = 0x1F2428FF,
    pressed = 0x2D333BFF,
}

-- Convert hex string to color
function M.hex(color_string)
    return tonumber(color_string, 16)
end

-- Lerp between two colors (for animations)
function M.lerp_color(a, b, t)
    local a_r = bit.rshift(bit.band(a, 0xFF000000), 24)
    local a_g = bit.rshift(bit.band(a, 0x00FF0000), 16)
    local a_b = bit.rshift(bit.band(a, 0x0000FF00), 8)
    local a_a = bit.band(a, 0x000000FF)
    
    local b_r = bit.rshift(bit.band(b, 0xFF000000), 24)
    local b_g = bit.rshift(bit.band(b, 0x00FF0000), 16)
    local b_b = bit.rshift(bit.band(b, 0x0000FF00), 8)
    local b_a = bit.band(b, 0x000000FF)
    
    local r = math.floor(a_r + (b_r - a_r) * t)
    local g = math.floor(a_g + (b_g - a_g) * t)
    local b = math.floor(a_b + (b_b - a_b) * t)
    local a = math.floor(a_a + (b_a - a_a) * t)
    
    return bit.lshift(r, 24) + bit.lshift(g, 16) + bit.lshift(b, 8) + a
end

-- Button widget
function M.button(text, x, y, w, h)
    local hovered = ui.is_rect_hovered(x, y, w, h)
    local pressed = hovered and ui.is_mouse_down()
    
    local bg_color = M.colors.bg_secondary
    if pressed then
        bg_color = M.colors.pressed
    elseif hovered then
        bg_color = M.colors.hover
    end
    
    -- Draw button background
    ui.rect(x, y, w, h, bg_color)
    
    -- Draw border
    ui.rect(x, y, w, 1, M.colors.border)           -- top
    ui.rect(x, y + h - 1, w, 1, M.colors.border)   -- bottom
    ui.rect(x, y, 1, h, M.colors.border)           -- left
    ui.rect(x + w - 1, y, 1, h, M.colors.border)   -- right
    
    -- Draw text (centered)
    local text_color = hovered and M.colors.text_primary or M.colors.text_secondary
    local text_x = x + w / 2 - (#text * 8) / 2  -- Approximate centering
    local text_y = y + h / 2 + 6  -- Vertical center (approximate)
    ui.text(text, text_x, text_y, 1.0, text_color, FONT_CASKAYDIA_MONO)
    
    -- Return interaction state
    return {
        hovered = hovered,
        pressed = pressed,
        clicked = hovered and ui.is_mouse_pressed()
    }
end

-- Panel widget
function M.panel(x, y, w, h, color)
    color = color or M.colors.bg_secondary
    ui.rect(x, y, w, h, color)
end

-- Text label
function M.label(text, x, y, color, font)
    color = color or M.colors.text_primary
    font = font or FONT_CASKAYDIA_MONO
    ui.text(text, x, y, 1.0, color, font)
end

-- Progress bar
function M.progress_bar(x, y, w, h, value, max_value)
    max_value = max_value or 1.0
    local progress = math.min(1.0, math.max(0.0, value / max_value))
    
    -- Background
    ui.rect(x, y, w, h, M.colors.bg_tertiary)
    
    -- Fill
    local fill_w = math.floor(w * progress)
    if fill_w > 0 then
        ui.rect(x, y, fill_w, h, M.colors.accent_green)
    end
    
    -- Border
    ui.rect(x, y, w, 1, M.colors.border)
    ui.rect(x, y + h - 1, w, 1, M.colors.border)
    ui.rect(x, y, 1, h, M.colors.border)
    ui.rect(x + w - 1, y, 1, h, M.colors.border)
end

-- Resource display (for game UI)
function M.resource(icon, label, value, x, y, color)
    color = color or M.colors.accent_green
    
    -- Icon (using NerdFont symbols)
    ui.text(icon, x, y + 16, 1.0, color, FONT_NERD_SYMBOLS)
    
    -- Label
    ui.text(label, x + 30, y + 16, 1.0, M.colors.text_secondary, FONT_CASKAYDIA_MONO)
    
    -- Value
    local value_str = tostring(math.floor(value))
    ui.text(value_str, x + 150, y + 16, 1.0, M.colors.text_primary, FONT_CASKAYDIA_MONO)
end

-- Layout helper: vertical stack
function M.vstack(x, y, spacing)
    local current_y = y
    return {
        next = function(height)
            local result_y = current_y
            current_y = current_y + height + spacing
            return result_y
        end,
        get_y = function()
            return current_y
        end
    }
end

-- Layout helper: horizontal stack
function M.hstack(x, y, spacing)
    local current_x = x
    return {
        next = function(width)
            local result_x = current_x
            current_x = current_x + width + spacing
            return result_x
        end,
        get_x = function()
            return current_x
        end
    }
end

return M
