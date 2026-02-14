-- UI API utilities and helpers

--- Combine two style tables, with overrides taking precedence
--- @param base table The base style table
--- @param overrides table The style properties to override
--- @return table
function ui.compose(base, overrides)
    local result = {}
    for k, v in pairs(base) do
        result[k] = v
    end
    for k, v in pairs(overrides) do
        result[k] = v
    end
    return result
end

--- Create a horizontal box container
--- @param opts table Options for the hbox, including:
---   - align: Alignment of children ('start', 'center', 'end', 'stretch')
---   - justify: Justification of children ('start', 'center', 'end', 'space-between', 'space-around', 'space-evenly')
---   - gap: Spacing between children in pixels
---   - size: Table with width and height ('auto', 'fill', or specific pixel values)
---   - style: Additional styles to apply to the container
--- @return table
function ui.hbox(opts)
    opts = opts or {}
    local style = opts.style or {}
    local layout = {
        direction = "row",
        align = opts.align or "stretch",
        justify = opts.justify or "start",
        gap = opts.gap or 0,
    }

    return ui.div(ui.compose(style, {
        layout = layout,
        width = opts.width or "auto",
        height = opts.height or "fill",
    }))
end

--- Create a vertical box container
--- @param opts table Options for the vbox, including:
---   - align: Alignment of children ('start', 'center', 'end', 'stretch')
---   - justify: Justification of children ('start', 'center', 'end', 'space-between', 'space-around', 'space-evenly')
---   - gap: Spacing between children in pixels
---   - size: Table with width and height ('auto', 'fill', or specific pixel values)
---   - style: Additional styles to apply to the container
--- @return table
function ui.vbox(opts)
    opts = opts or {}
    local style = opts.style or {}
    local layout = {
        direction = "column",
        align = opts.align or "stretch",
        justify = opts.justify or "start",
        gap = opts.gap or 0,
    }

    return ui.div(ui.compose(style, {
        layout = layout,
        width = opts.width or "auto",
        height = opts.height or "fill",
    }))
end

-- Create a spacer element
function ui.spacer(opts)
    opts = opts or {}
    return ui.div({
        style = {
            size = opts.size or { width = "fill", height = "fill" },
            background = opts.background,
        },
    })
end

-- Create a label element
function ui.label(text, opts)
    opts = opts or {}
    return ui.text(ui.compose({
        text = text,
        font_scale = opts.scale or 1.0,
    }, opts))
end

--- Create a button-like div.
--- @param options table Options for the button, including:
---   - style: Additional styles to apply to the button.
---   - content: Child elements to include inside the button.
---   - on_click: Callback function to invoke when the button is clicked
--- @return table
function ui.button(options)
    return ui.div({
        style = ui.compose({
            layout = { direction = "row", align = "center", justify = "center", gap = 0 },
            width = 128,
            height = 32,
            padding = 2,
            background = "#5D6C80",
            border = 2,
            border_color = "#30363D",
        }, options.style),
        children = options.content,
        on_click = options.on_click,
    })
end

--- Log a message to the UI log
--- @param text string The message to log
function ui.log(text)
    log(text)
end
