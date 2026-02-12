-- Retained-mode DOM with HTML-like structure and CSS-like styling

-- rustine.ui

-- Define reusable styles
local panel_style = {
    normal = {
        bg_color = 0x161B22FF,
        border_color = 0x30363DFF,
        border_width = 1.0,
        padding = 10.0,
    }
}

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

local title_style = {
    normal = {
        text_color = 0xFFFFFFFF,
        font_size = 1.2,
        font_id = FONT_CASKAYDIA_MONO,
    }
}

local text_style = {
    normal = {
        text_color = 0x8B949EFF,
        font_size = 1.0,
        font_id = FONT_CASKAYDIA_MONO,
    }
}

-- Build the UI DOM structure
function build_ui()
    local w, h = ui.get_window_size()

    -- Root panel (background)
    local root = dom.panel({
        id = "root",
        layout = "vertical",
        style = {
            normal = {
                bg_color = 0x0D1117FF,
            }
        },
        width = w,
        height = h,
        children = {
            -- Title
            dom.text({
                id = "title",
                text = "Rustine Mech - DOM UI System",
                style = title_style,
                height = 40,
            }),

            -- Subtitle
            dom.text({
                id = "subtitle",
                text = "Press R to reload, Q to quit",
                style = text_style,
                height = 30,
            }),

            -- Content panel
            dom.panel({
                id = "content",
                x = 20,
                y = 100,
                width = 400,
                height = 250,
                style = panel_style,
                layout = "vertical",
                children = {
                    dom.text({
                        text = "System Information",
                        style = title_style,
                        height = 35,
                    }),
                    dom.text({
                        text = "Window: " .. w .. " x " .. h,
                        style = text_style,
                        height = 25,
                    }),
                    dom.text({
                        text = "Renderer: Vulkan",
                        style = text_style,
                        height = 25,
                    }),
                    dom.text({
                        text = "UI Mode: DOM (Retained)",
                        style = text_style,
                        height = 25,
                    }),
                }
            }),

            -- Buttons panel
            dom.panel({
                id = "buttons",
                x = 440,
                y = 100,
                width = 200,
                height = 250,
                style = panel_style,
                layout = "vertical",
                children = {
                    dom.button({
                        id = "btn1",
                        text = "Button 1",
                        style = button_style,
                        height = 40,
                        on_click = "on_button1_click",
                    }),
                    dom.button({
                        id = "btn2",
                        text = "Button 2",
                        style = button_style,
                        y = 10, -- margin between buttons
                        height = 40,
                        on_click = "on_button2_click",
                    }),
                    dom.button({
                        id = "btn3",
                        text = "Button 3",
                        style = button_style,
                        y = 10,
                        height = 40,
                        on_click = "on_button3_click",
                    }),
                }
            }),
        }
    })

    return root
end

-- Event handlers
function on_button1_click()
    log("Button 1 clicked!")
end

function on_button2_click()
    log("Button 2 clicked!")
end

function on_button3_click()
    log("Button 3 clicked!")
end

-- Note: The actual DOM building and rendering happens in Rust
-- This function just defines the structure
log("DOM UI demo loaded")
