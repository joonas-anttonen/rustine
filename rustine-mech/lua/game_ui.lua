-- Example Game UI for Rustine Mech
-- Resource extraction/economy game UI

local ui_lib = require("ui_lib")

-- Game state (would be updated from Rust)
local game_state = {
    resources = {
        metal = 1250,
        energy = 850,
        credits = 5400,
    },
    production = {
        metal_rate = 12.5,
        energy_rate = 8.3,
        credits_rate = 15.0,
    },
    buildings = {
        extractors = 5,
        refineries = 3,
        power_plants = 4,
    },
    selected_building = nil,
}

-- Animation state
local button_states = {}
local time = 0

-- Main UI render function (called every frame)
function render_ui()
    local w, h = ui.get_window_size()
    
    -- Update time for animations
    time = time + 0.016  -- ~60 FPS
    
    -- Background
    ui.rect(0, 0, w, h, ui_lib.colors.bg_primary)
    
    -- Top bar with resources
    render_top_bar(w)
    
    -- Main content area
    render_main_content(w, h)
    
    -- Bottom control panel
    render_control_panel(w, h)
end

function render_top_bar(w)
    local bar_height = 60
    ui_lib.panel(0, 0, w, bar_height, ui_lib.colors.bg_secondary)
    
    -- Resource displays
    local stack = ui_lib.hstack(20, 10, 40)
    
    ui_lib.resource("", "Metal", game_state.resources.metal, stack.next(200), 10, ui_lib.colors.accent_blue)
    ui_lib.resource("", "Energy", game_state.resources.energy, stack.next(200), 10, ui_lib.colors.accent_orange)
    ui_lib.resource("", "Credits", game_state.resources.credits, stack.next(200), 10, ui_lib.colors.accent_green)
    
    -- Production rates (smaller text below)
    ui.text("+/" .. game_state.production.metal_rate, 70, 45, 0.8, ui_lib.colors.text_secondary, FONT_CASKAYDIA_MONO)
    ui.text("+/" .. game_state.production.energy_rate, 310, 45, 0.8, ui_lib.colors.text_secondary, FONT_CASKAYDIA_MONO)
    ui.text("+/" .. game_state.production.credits_rate, 550, 45, 0.8, ui_lib.colors.text_secondary, FONT_CASKAYDIA_MONO)
end

function render_main_content(w, h)
    local content_y = 60
    local content_h = h - 60 - 200
    
    -- Left sidebar - Building list
    render_building_sidebar(20, content_y + 20, 300, content_h - 40)
    
    -- Main view area (would show game world/map)
    local main_x = 340
    local main_w = w - main_x - 20
    ui_lib.panel(main_x, content_y + 20, main_w, content_h - 40, ui_lib.colors.bg_tertiary)
    ui_lib.label("[ MAIN VIEW ]", main_x + 20, content_y + 50, ui_lib.colors.text_secondary)
end

function render_building_sidebar(x, y, w, h)
    ui_lib.panel(x, y, w, h, ui_lib.colors.bg_secondary)
    
    -- Title
    ui_lib.label("BUILDINGS", x + 10, y + 25, ui_lib.colors.text_primary)
    
    -- Building buttons
    local stack = ui_lib.vstack(x + 10, y + 50, 10)
    
    local buildings = {
        {name = "Extractors", count = game_state.buildings.extractors, icon = ""},
        {name = "Refineries", count = game_state.buildings.refineries, icon = ""},
        {name = "Power Plants", count = game_state.buildings.power_plants, icon = ""},
    }
    
    for i, building in ipairs(buildings) do
        local btn_y = stack.next(40)
        local btn_text = building.icon .. " " .. building.name .. " (" .. building.count .. ")"
        
        local state = ui_lib.button(btn_text, x + 10, btn_y, w - 20, 35)
        
        if state.clicked then
            game_state.selected_building = building.name
            log("Selected: " .. building.name)
        end
    end
end

function render_control_panel(w, h)
    local panel_height = 180
    local panel_y = h - panel_height
    
    ui_lib.panel(0, panel_y, w, panel_height, ui_lib.colors.bg_secondary)
    
    -- Panel title
    ui_lib.label("CONTROL PANEL", 20, panel_y + 25, ui_lib.colors.text_primary)
    
    -- Action buttons
    local stack = ui_lib.hstack(20, panel_y + 50, 15)
    
    local buttons = {
        {text = " Build", width = 150},
        {text = " Upgrade", width = 150},
        {text = " Demolish", width = 150},
        {text = " Research", width = 150},
    }
    
    for i, btn_def in ipairs(buttons) do
        local btn_x = stack.next(btn_def.width)
        local state = ui_lib.button(btn_def.text, btn_x, panel_y + 50, btn_def.width, 40)
        
        if state.clicked then
            log("Clicked: " .. btn_def.text)
        end
    end
    
    -- Status/info area
    ui_lib.panel(20, panel_y + 110, w - 40, 50, ui_lib.colors.bg_tertiary)
    
    if game_state.selected_building then
        ui_lib.label("Selected: " .. game_state.selected_building, 30, panel_y + 140, ui_lib.colors.accent_green)
    else
        ui_lib.label("No building selected", 30, panel_y + 140, ui_lib.colors.text_secondary)
    end
end

-- Update function (called by Rust to update game state)
function update_game_state(resources, production, buildings)
    game_state.resources = resources or game_state.resources
    game_state.production = production or game_state.production
    game_state.buildings = buildings or game_state.buildings
end

-- Initialize UI
log("Game UI initialized")
