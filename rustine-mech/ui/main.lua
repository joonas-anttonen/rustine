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
