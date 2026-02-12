-- UI API utilities and helpers

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

function ui.hbox(opts)
    opts = opts or {}
    local style = opts.style or {}
    local layout = {
        direction = "row",
        align_items = opts.align or "stretch",
        justify_content = opts.justify or "start",
        gap = opts.gap or 0,
    }

    return ui.div(ui.compose(style, {
        layout = layout,
        size = opts.size or { width = "fill", height = "auto" },
    }))
end

function ui.vbox(opts)
    opts = opts or {}
    local style = opts.style or {}
    local layout = {
        direction = "column",
        align_items = opts.align or "stretch",
        justify_content = opts.justify or "start",
        gap = opts.gap or 0,
    }

    return ui.div(ui.compose(style, {
        layout = layout,
        size = opts.size or { width = "auto", height = "fill" },
    }))
end

function ui.spacer(opts)
    opts = opts or {}
    return ui.div({
        style = {
            size = opts.size or { width = "fill", height = "fill" },
            background = opts.background,
        },
    })
end

function ui.label(text, opts)
    opts = opts or {}
    return ui.text(ui.compose({
        text = text,
        font_scale = opts.scale or 1.0,
    }, opts))
end
