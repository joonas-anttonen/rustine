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
            layout = { direction = "column", align = "start", justify = "start", gap = 12 },
            padding = 12,
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
                children = {
                    ui.label("Click me!", { style = { foreground = palette.fg } }),
                },
                on_click = function()
                    ui.log("Button clicked!")
                end
            }),
            ui.div({
                style = {
                    width = "fill",
                    height = 1,
                    background = palette.border,
                },
            }),
            ui.label("This is a simple UI built with Rustine's Lua API.", {
                style = { foreground = palette.fg } }),
            ui.div({
                style = {
                    width = "fill",
                    height = 1,
                    background = palette.border,
                },
            }),
            -- Here goes an imaginary list of items, for demonstration purposes
            ui.list({
                item_template = function(index)
                    return ui.div({
                        style = {
                            layout = { direction = "row", align = "center", justify = "space-between", gap = 8 },
                            padding = 8,
                            background = "#2e3a4b",
                            border = 1,
                            border_color = "#1B232F",
                        },
                        children = {
                            ui.label("Item " .. index, { style = { foreground = palette.fg } }),
                        }
                    })
                end,
                count = 3,
                style = {
                    layout = { direction = "column", align = "start", justify = "start", gap = 12 },
                },
            }),
        },
    })
)
