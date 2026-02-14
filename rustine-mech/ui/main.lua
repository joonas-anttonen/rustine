--[[
-- Style definitions
local button_base = {
    layout = { direction = "row", align = "center", justify = "center", gap = 6 },
    size = { width = 120, height = 36 },
    padding = 6,
    background = "#5D6C80",
    border = 1,
    border_color = "#30363D",
}

local button_style = {
    normal = button_base,
    hover = ui.compose(button_base, { background = "#6B7C93" }),
    press = ui.compose(button_base, { background = "#4F5F74" }),
}

local card_style = {
    normal = {
        layout = { direction = "column", align = "stretch", justify = "start" },
        padding = 12,
        background = "#161B22",
        border = 1,
        border_color = "#30363D",
    },
}

local state = {
    launch_clicks = 0,
}

-- Widget helpers
local function button(label, on_click)
    return ui.div({
        style = button_style,
        children = {
            ui.label(label, { style = { foreground = "#F0F6FC" } }),
        },
        on_click = on_click,
    })
end

local header_style = ui.compose(card_style.normal, {
    layout = { direction = "row", align = "center", justify = "start", gap = 12 },
    size = { width = "fill", height = 72 },
})

local header_label = ui.label("MECH OPS", { scale = 1.4, style = { foreground = "#F0F6FC" } })

local root = ui.div({
    style = {
        layout = { direction = "column", align = "stretch", justify = "start", gap = 16 },
        size = { width = "fill", height = "fill" },
        padding = 16,
        background = "#0D1117",
    },
    children = {
        ui.div({
            style = header_style,
            children = {
                header_label,
                ui.spacer(),
                button("Launch", function()
                    state.launch_clicks = state.launch_clicks + 1
                    ui.log("Launch clicked! count=" .. tostring(state.launch_clicks))
                    ui.set_text(header_label, "MECH OPS (" .. tostring(state.launch_clicks) .. ")")
                end),
                button("Diagnostics"),
                button("Power"),
            },
        }),
        ui.div({
            style = {
                layout = { direction = "row", align = "stretch", justify = "start", gap = 16 },
                size = { width = "fill", height = "fill" },
            },
            children = {
                ui.div({
                    style = card_style.normal,
                    children = {
                        ui.label("SYSTEM STATUS", { style = { foreground = "#C9D1D9" } }),
                        ui.label("ONLINE", { scale = 1.2, style = { foreground = "#58A6FF" } }),
                    },
                }),
                ui.div({
                    style = {
                        layout = {
                            direction = "column",
                            align = "stretch",
                            justify = "start",
                            gap = 12,
                        },
                        size = { width = "fill", height = "fill" },
                        padding = 12,
                        background = "#0B0F14",
                        border = 1,
                        border_color = "#21262D",
                    },
                    children = {
                        ui.label("Telemetry feed online. Awaiting next command.",
                            { style = { foreground = "#8B949E" } }),
                    },
                }),
            },
        }),
    },
})

ui.dom(root)
]] --

local palette = {
    bg = "#1B232F",
    bg_button = "#2489db",
    fg = "#FFFFFF",
    border = "#2e3a4b",
    hover = "#2a9ae0",
    press = "#1f78c1",
}

ui.dom(
    ui.div({
        style = {
            layout = { direction = "column", align = "center", justify = "center", gap = 12 },
            size = "fill",
            background = palette.bg,
        },
        children = {
            ui.label("Lua UI", { scale = 1.5, style = { foreground = palette.fg } }),
            ui.button({
                style = {
                    background = palette.bg_button,
                    border_color = palette.border,
                    hover = { background = palette.hover },
                    press = { background = palette.press },
                },
                content = {
                    ui.label("Click me!", { style = { foreground = palette.fg } }),
                },
                on_click = function()
                    ui.log("Button clicked!")
                end
            }),
        },
    })
)
