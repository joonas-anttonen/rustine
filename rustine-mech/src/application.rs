use rustine::{
    Color, Vector2f, gfx,
    gui::{self, dom, style::StyleComputer},
    log,
};

use std::path::Path;

struct MyApplicationState {
    pub runtime: gui::api::LuaRuntime,
}

pub struct MyApplication {
    state: std::cell::RefCell<MyApplicationState>,
}

impl MyApplication {
    pub fn new() -> Self {
        let (runtime, _) = load_lua_runtime(Path::new("ui/main.lua"));

        MyApplication {
            state: std::cell::RefCell::new(MyApplicationState { runtime }),
        }
    }
}

impl Drop for MyApplication {
    fn drop(&mut self) {}
}

impl gui::Application for MyApplication {
    fn startup(&self, _gui: &gui::Gui) {
        log::debug!("Application::startup");
    }

    fn on_key(&self, gui: &gui::Gui, event: gui::KeyEvent) {
        let mut state = self.state.borrow_mut();

        if event.action != gui::Action::PRESS {
            return;
        }

        if event.key == gui::Key::R && event.mods.contains(gui::Mods::CONTROL) {
            let loaded = reload_lua_ui(&mut state);
            if loaded {
                log::info!("Lua UI reloaded: ui/main.lua");
            }
            gui.mark_damaged();
            return;
        }

        // Assume the window is damaged after handling a key press
        gui.mark_damaged();
    }

    fn on_char(&self, gui: &gui::Gui, _c: char) {
        // Assume the window is damaged after handling a character input
        gui.mark_damaged();
    }

    fn on_mouse_enter(&self, gui: &gui::Gui, event: gui::MouseEnterEvent) {
        let mut state = self.state.borrow_mut();
        let (dom, style) = state.runtime.dom_and_style_mut();
        if style.update_mouse_position(event.position, dom) {
            gui.mark_damaged();
        }
    }

    fn on_mouse_leave(&self, gui: &gui::Gui, _event: gui::MouseLeaveEvent) {
        let mut state = self.state.borrow_mut();
        if state.runtime.style_mut().set_hovered(None) {
            gui.mark_damaged();
        }
    }

    fn on_mouse_move(&self, gui: &gui::Gui, event: gui::MouseMoveEvent) {
        let mut state = self.state.borrow_mut();
        let (dom, style) = state.runtime.dom_and_style_mut();
        if style.update_mouse_position(event.position, dom) {
            gui.mark_damaged();
        }
    }

    fn on_mouse_button(&self, gui: &gui::Gui, event: gui::MouseButtonEvent) {
        let mut state = self.state.borrow_mut();
        let mut clicked = None;
        let mut changed = false;

        {
            let (dom, style) = state.runtime.dom_and_style_mut();
            changed |= style.update_mouse_position(event.position, dom);

            if event.button == gui::MouseButton::LEFT {
                if event.action == gui::Action::PRESS {
                    let hovered = style.hovered_node();
                    changed |= style.set_active(hovered);
                } else {
                    let active = style.active_node();
                    let hovered = style.hovered_node();
                    if active.is_some() && active == hovered {
                        clicked = active;
                    }
                    changed |= style.set_active(None);
                }
            }
        }

        if let Some(node_id) = clicked {
            if state.runtime.dispatch_click(node_id) {
                changed = true;
            }
        }

        if changed {
            gui.mark_damaged();
        }
    }

    fn render(&self, gui: &gui::Gui, frame: &mut gfx::RenderFrame) {
        let mut state = self.state.borrow_mut();
        let pixel_size = gui.pixel_size();
        let root_size = Vector2f::new(pixel_size.x as f32, pixel_size.y as f32);
        let root = state.runtime.root();
        let (dom, style) = state.runtime.dom_and_style_mut();

        dom.layout(root_size);
        render_dom(dom, root, style, frame);
    }
}

fn load_lua_runtime(path: &Path) -> (gui::api::LuaRuntime, bool) {
    match gui::api::LuaRuntime::from_file(path) {
        Ok(runtime) => {
            log::info!("{}", runtime.dom());
            (runtime, true)
        }
        Err(err) => {
            log::warning!("Failed to load Lua UI: {err}");
            let mut runtime = gui::api::LuaRuntime::new_empty();
            let root = runtime.root();
            build_error_ui(runtime.dom_mut(), root, err.to_string());
            (runtime, false)
        }
    }
}

fn reload_lua_ui(state: &mut MyApplicationState) -> bool {
    let (runtime, loaded) = load_lua_runtime(Path::new("ui/main.lua"));
    state.runtime = runtime;
    loaded
}

fn build_error_ui(dom: &mut dom::Dom, root: dom::NodeId, error: String) {
    let panel_width = 600.0;
    let panel_padding = 16.0;
    let error_scale = 0.95;
    let error_wrap_width = panel_width - panel_padding * 2.0;
    let root_style = dom.node_mut(root).unwrap().style_mut().unwrap();
    root_style.layout = gui::LayoutStyle {
        direction: gui::LayoutDirection::Column,
        align: gui::Align::Center,
        justify: gui::Justify::Center,
        gap: 12.0,
    };
    root_style.padding = edge_all(24.0);
    root_style.background = Color::from_u32(0x0E1117FF);
    root_style.size = gui::Size::fill();

    let panel = add_div(dom, root, |style| {
        style.layout = gui::LayoutStyle {
            direction: gui::LayoutDirection::Column,
            align: gui::Align::Stretch,
            justify: gui::Justify::Start,
            gap: 8.0,
        };
        style.size = gui::Size {
            width: gui::Length::Auto,
            height: gui::Length::Auto,
        };
        style.padding = edge_all(panel_padding);
        style.background = Color::from_u32(0x161B22FF);
        style.border = edge_all(1.0);
        style.border_color = Color::from_u32(0x30363DFF);
    });

    add_text(dom, panel, "Lua UI failed to load", 1.3, |style| {
        style.size = gui::Size::auto();
        style.foreground = Color::from_u32(0xF85149FF);
    });

    add_text(dom, panel, "Check ui/main.lua and reload.", 1.0, |style| {
        style.size = gui::Size::auto();
        style.foreground = Color::from_u32(0xC9D1D9FF);
    });

    let error_block = add_div(dom, panel, |style| {
        style.layout = gui::LayoutStyle {
            direction: gui::LayoutDirection::Column,
            align: gui::Align::Stretch,
            justify: gui::Justify::Start,
            gap: 4.0,
        };
        style.size = gui::Size::auto();
    });

    for line in gfx::wrap_text_lines(
        &error,
        error_wrap_width,
        error_scale,
        gfx::fonts::CASKAYDIAMONO_FONT_ID,
    ) {
        let content = if line.is_empty() { " " } else { line.as_str() };
        add_text(dom, error_block, content, error_scale, |style| {
            style.size = gui::Size::auto();
            style.foreground = Color::from_u32(0x8B949EFF);
        });
    }
}

fn add_div(
    dom: &mut dom::Dom,
    parent: dom::NodeId,
    f: impl FnOnce(&mut gui::Style),
) -> dom::NodeId {
    let id = dom.create_div();
    if let Some(node) = dom.node_mut(id) {
        if let Some(div) = node.as_div_mut() {
            f(&mut div.style);
        }
    }
    dom.append_child(parent, id);
    id
}

fn add_text(
    dom: &mut dom::Dom,
    parent: dom::NodeId,
    content: impl Into<String>,
    scale: f32,
    f: impl FnOnce(&mut gui::Style),
) -> dom::NodeId {
    let id = dom.create_text(content, gfx::fonts::CASKAYDIAMONO_FONT_ID, scale);
    if let Some(node) = dom.node_mut(id) {
        if let Some(text) = node.as_text_mut() {
            f(&mut text.style);
        }
    }
    dom.append_child(parent, id);
    id
}

fn edge_all(value: f32) -> gui::EdgeSizes {
    gui::EdgeSizes {
        left: value,
        right: value,
        top: value,
        bottom: value,
    }
}

fn render_dom(
    dom: &dom::Dom,
    node_id: dom::NodeId,
    style: &mut StyleComputer,
    frame: &mut gfx::RenderFrame,
) {
    let Some(node) = dom.node(node_id) else {
        return;
    };

    let rect = gfx::Rectangle {
        x: node.layout.position.x,
        y: node.layout.position.y,
        w: node.layout.size.x,
        h: node.layout.size.y,
    };

    if let Some(node_style) = node.style() {
        let mut resolved_style = node_style.clone();
        style.apply_to_style(node_id, &mut resolved_style);

        draw_style(frame, &rect, &resolved_style);

        if let Some(text) = node.as_text() {
            draw_text(frame, &rect, &resolved_style, text);
        }
    }

    for child_id in node.children.iter().copied() {
        render_dom(dom, child_id, style, frame);
    }
}

fn draw_text(
    frame: &mut gfx::RenderFrame,
    rect: &gfx::Rectangle,
    style: &gui::Style,
    text: &dom::Text,
) {
    if style.foreground.a <= 0.0 || text.content.is_empty() {
        return;
    }

    let inner = inset_rectangle(rect, style.border);
    let content_rect = inset_rectangle(&inner, style.padding);
    let bounds = gfx::measure_text(&text.content, text.scale, text.font_id);
    let baseline_x = content_rect.x;
    let baseline_y = content_rect.y - bounds.y;
    frame.push_text(
        &text.content,
        baseline_x,
        baseline_y,
        text.scale,
        style.foreground.to_u32(),
        text.font_id,
    );
}

fn draw_style(frame: &mut gfx::RenderFrame, rect: &gfx::Rectangle, style: &gui::Style) {
    if style.background.a > 0.0 {
        let inner = inset_rectangle(rect, style.border);
        frame.fill_rectangle(&inner, style.background.to_u32());
    }

    let border = style.border;
    if style.border_color.a > 0.0
        && (border.left > 0.0 || border.right > 0.0 || border.top > 0.0 || border.bottom > 0.0)
    {
        draw_border(frame, rect, border, style.border_color.to_u32());
    }
}

fn inset_rectangle(rect: &gfx::Rectangle, border: gui::EdgeSizes) -> gfx::Rectangle {
    let w = (rect.w - border.left - border.right).max(0.0);
    let h = (rect.h - border.top - border.bottom).max(0.0);
    gfx::Rectangle {
        x: rect.x + border.left,
        y: rect.y + border.top,
        w,
        h,
    }
}

fn draw_border(
    frame: &mut gfx::RenderFrame,
    rect: &gfx::Rectangle,
    border: gui::EdgeSizes,
    color: u32,
) {
    let top_h = border.top.max(0.0);
    let bottom_h = border.bottom.max(0.0);
    let left_w = border.left.max(0.0);
    let right_w = border.right.max(0.0);
    let mid_h = (rect.h - top_h - bottom_h).max(0.0);

    if top_h > 0.0 {
        frame.fill_rectangle(
            &gfx::Rectangle {
                x: rect.x,
                y: rect.y,
                w: rect.w,
                h: top_h,
            },
            color,
        );
    }

    if bottom_h > 0.0 {
        frame.fill_rectangle(
            &gfx::Rectangle {
                x: rect.x,
                y: rect.y + rect.h - bottom_h,
                w: rect.w,
                h: bottom_h,
            },
            color,
        );
    }

    if left_w > 0.0 && mid_h > 0.0 {
        frame.fill_rectangle(
            &gfx::Rectangle {
                x: rect.x,
                y: rect.y + top_h,
                w: left_w,
                h: mid_h,
            },
            color,
        );
    }

    if right_w > 0.0 && mid_h > 0.0 {
        frame.fill_rectangle(
            &gfx::Rectangle {
                x: rect.x + rect.w - right_w,
                y: rect.y + top_h,
                w: right_w,
                h: mid_h,
            },
            color,
        );
    }
}
