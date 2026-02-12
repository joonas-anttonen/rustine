use rustine::{
    Color, Vector2f, gfx,
    gui::{self, dom, style::{StyleComputer, StyleRules}},
    log,
};

struct MyApplicationState {
    dom: dom::Dom,
    root: dom::NodeId,
    style: StyleComputer,
}

pub struct MyApplication {
    state: std::cell::RefCell<MyApplicationState>,
}

impl MyApplication {
    pub fn new() -> Self {
        let (dom, root, style) = match gui::lua::load_dom_from_file("ui/main.lua") {
            Ok(lua_dom) => (lua_dom.dom, lua_dom.root, lua_dom.style),
            Err(err) => {
                log::warning!("Failed to load Lua UI: {err}");
                let mut dom = dom::Dom::new();
                let root = dom.root();
                let mut style = StyleComputer::new();
                build_mock_ui(&mut dom, root, &mut style);
                (dom, root, style)
            }
        };

        MyApplication {
            state: std::cell::RefCell::new(MyApplicationState { dom, root, style }),
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
        let mut _state = self.state.borrow_mut();

        if event.key == gui::Key::UNKNOWN {
            log::warning!("Application::on_key: {:?} {:?}", event.key, event.action);
        }

        if event.action != gui::Action::PRESS {
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
        let MyApplicationState { dom, style, .. } = &mut *state;
        if style.update_mouse_position(event.position, dom) {
            gui.mark_damaged();
        }
    }

    fn on_mouse_leave(&self, gui: &gui::Gui, _event: gui::MouseLeaveEvent) {
        let mut state = self.state.borrow_mut();
        let MyApplicationState { style, .. } = &mut *state;
        if style.set_hovered(None) {
            gui.mark_damaged();
        }
    }

    fn on_mouse_move(&self, gui: &gui::Gui, event: gui::MouseMoveEvent) {
        let mut state = self.state.borrow_mut();
        let MyApplicationState { dom, style, .. } = &mut *state;
        if style.update_mouse_position(event.position, dom) {
            gui.mark_damaged();
        }
    }

    fn on_mouse_button(&self, gui: &gui::Gui, event: gui::MouseButtonEvent) {
        let mut state = self.state.borrow_mut();
        let MyApplicationState { dom, style, .. } = &mut *state;
        let mut changed = style.update_mouse_position(event.position, dom);

        if event.button == gui::MouseButton::LEFT {
            if event.action == gui::Action::PRESS {
                let hovered = style.hovered_node();
                changed |= style.set_active(hovered);
            } else {
                changed |= style.set_active(None);
            }
        }

        if changed {
            gui.mark_damaged();
        }
    }

    fn render(&self, gui: &gui::Gui, frame: &mut gfx::RenderFrame) {
        let mut state = self.state.borrow_mut();
        let MyApplicationState { dom, root, style } = &mut *state;
        let pixel_size = gui.pixel_size();
        let root_size = Vector2f::new(pixel_size.x as f32, pixel_size.y as f32);

        dom.layout(root_size);
        render_dom(dom, *root, style, frame);
    }
}

fn build_mock_ui(dom: &mut dom::Dom, root: dom::NodeId, style: &mut StyleComputer) {
    let root_style = dom.node_mut(root).unwrap().style_mut().unwrap();
    root_style.layout = dom::LayoutStyle {
        direction: dom::LayoutDirection::Column,
        align_items: dom::AlignItems::Stretch,
        justify_content: dom::JustifyContent::Start,
        gap: 16.0,
    };
    root_style.padding = edge_all(16.0);
    root_style.background = Color::from_u32(0x0D1117FF);
    root_style.size = dom::Size2::fill();

    let top_bar = add_div(dom, root, |style| {
        style.layout = dom::LayoutStyle {
            direction: dom::LayoutDirection::Row,
            align_items: dom::AlignItems::Center,
            justify_content: dom::JustifyContent::Start,
            gap: 12.0,
        };
        style.size = dom::Size2 {
            width: dom::Length::Fill,
            height: dom::Length::Px(72.0),
        };
        style.padding = edge_all(12.0);
        style.background = Color::from_u32(0x161B22FF);
        style.border = edge_all(1.0);
        style.border_color = Color::from_u32(0x30363DFF);
    });

    let title_block = add_div(dom, top_bar, |style| {
        style.layout = dom::LayoutStyle {
            direction: dom::LayoutDirection::Row,
            align_items: dom::AlignItems::Center,
            justify_content: dom::JustifyContent::Start,
            gap: 8.0,
        };
        style.size = dom::Size2 {
            width: dom::Length::Px(180.0),
            height: dom::Length::Fill,
        };
        style.padding = edge_all(8.0);
    });

    add_text(dom, title_block, "MECH OPS", 1.4, |style| {
        style.size = dom::Size2::auto();
        style.foreground = Color::from_u32(0xF0F6FCFF);
    });

    let economy_row = add_div(dom, top_bar, |style| {
        style.layout = dom::LayoutStyle {
            direction: dom::LayoutDirection::Row,
            align_items: dom::AlignItems::Center,
            justify_content: dom::JustifyContent::Start,
            gap: 8.0,
        };
        style.size = dom::Size2 {
            width: dom::Length::Px(520.0),
            height: dom::Length::Fill,
        };
    });

    for (idx, label) in ["ECO GRID", "SUPPLY", "SALVAGE"].iter().copied().enumerate() {
        let card = add_div(dom, economy_row, |style| {
            style.size = dom::Size2 {
                width: dom::Length::Px(160.0),
                height: dom::Length::Fill,
            };
            style.padding = edge_all(10.0);
            style.background = Color::from_u32(0x21262DFF);
            style.border = edge_all(1.0);
            style.border_color = Color::from_u32(0x30363DFF);
        });

        let text = format!("{} {}", idx + 1, label);
        add_text(dom, card, text, 1.0, |style| {
            style.size = dom::Size2::auto();
            style.foreground = Color::from_u32(0xC9D1D9FF);
        });
    }

    add_div(dom, top_bar, |style| {
        style.size = dom::Size2 {
            width: dom::Length::Fill,
            height: dom::Length::Fill,
        };
    });

    let button_row = add_div(dom, top_bar, |style| {
        style.layout = dom::LayoutStyle {
            direction: dom::LayoutDirection::Row,
            align_items: dom::AlignItems::Center,
            justify_content: dom::JustifyContent::End,
            gap: 8.0,
        };
        style.size = dom::Size2 {
            width: dom::Length::Px(420.0),
            height: dom::Length::Fill,
        };
    });

    for label in ["Launch", "Diagnostics", "Power"].iter().copied() {
        let button = add_div(dom, button_row, |style| {
            style.layout = dom::LayoutStyle {
                direction: dom::LayoutDirection::Row,
                align_items: dom::AlignItems::Center,
                justify_content: dom::JustifyContent::Center,
                gap: 6.0,
            };
            style.size = dom::Size2 {
                width: dom::Length::Px(120.0),
                height: dom::Length::Px(36.0),
            };
            style.padding = edge_all(6.0);
            style.background = Color::from_u32(0x5d6c80FF); // #5d6c80
            style.border = edge_all(1.0);
            style.border_color = Color::from_u32(0x30363DFF);
        });

        register_hover_active_background(
            style,
            button,
            Color::from_u32(0x6B7C93FF),
            Color::from_u32(0x4F5F74FF),
        );

        add_text(dom, button, label, 1.0, |style| {
            style.size = dom::Size2::auto(); 
            style.foreground = Color::from_u32(0xF0F6FCFF); // #F0F6FCFF
        });
    }

    let main_row = add_div(dom, root, |style| {
        style.layout = dom::LayoutStyle {
            direction: dom::LayoutDirection::Row,
            align_items: dom::AlignItems::Stretch,
            justify_content: dom::JustifyContent::Start,
            gap: 16.0,
        };
        style.size = dom::Size2 {
            width: dom::Length::Fill,
            height: dom::Length::Fill,
        };
    });

    let left_panel = add_div(dom, main_row, |style| {
        style.layout = dom::LayoutStyle {
            direction: dom::LayoutDirection::Column,
            align_items: dom::AlignItems::Stretch,
            justify_content: dom::JustifyContent::Start,
            gap: 10.0,
        };
        style.size = dom::Size2 {
            width: dom::Length::Fill,
            height: dom::Length::Fill,
        };
        style.padding = edge_all(12.0);
        style.background = Color::from_u32(0x161B22FF);
        style.border = edge_all(1.0);
        style.border_color = Color::from_u32(0x30363DFF);
    });

    let left_header = add_div(dom, left_panel, |style| {
        style.size = dom::Size2 {
            width: dom::Length::Px(240.0),
            height: dom::Length::Px(20.0),
        };
        style.padding = edge_all(4.0);
        style.background = Color::from_u32(0x21262DFF);
    });

    add_text(dom, left_header, "Inventory", 1.1, |style| {
        style.size = dom::Size2::auto();
        style.foreground = Color::from_u32(0xF0F6FCFF);
    });

    for label in [
        "Hydraulic Core",
        "Servo Array",
        "Reactor Feed",
        "Armor Plating",
        "Sensor Suite",
        "Cooling Loop",
    ]
    .iter()
    .copied()
    {
        let row = add_div(dom, left_panel, |style| {
            style.layout = dom::LayoutStyle {
                direction: dom::LayoutDirection::Row,
                align_items: dom::AlignItems::Center,
                justify_content: dom::JustifyContent::Start,
                gap: 8.0,
            };
            style.size = dom::Size2 {
                width: dom::Length::Fill,
                height: dom::Length::Px(36.0),
            };
            style.padding = edge_all(8.0);
            style.background = Color::from_u32(0x0D1117FF);
            style.border = edge_all(1.0);
            style.border_color = Color::from_u32(0x30363DFF);
        });

        register_hover_active_background(
            style,
            row,
            Color::from_u32(0x161B22FF),
            Color::from_u32(0x0B1016FF),
        );

        add_text(dom, row, label, 1.0, |style| {
            style.size = dom::Size2::auto();
            style.foreground = Color::from_u32(0xC9D1D9FF);
        });
    }

    let right_panel = add_div(dom, main_row, |style| {
        style.layout = dom::LayoutStyle {
            direction: dom::LayoutDirection::Column,
            align_items: dom::AlignItems::Stretch,
            justify_content: dom::JustifyContent::Start,
            gap: 12.0,
        };
        style.size = dom::Size2 {
            width: dom::Length::Px(320.0),
            height: dom::Length::Fill,
        };
        style.padding = edge_all(12.0);
        style.background = Color::from_u32(0x161B22FF);
        style.border = edge_all(1.0);
        style.border_color = Color::from_u32(0x30363DFF);
    });

    let right_header = add_div(dom, right_panel, |style| {
        style.size = dom::Size2 {
            width: dom::Length::Px(180.0),
            height: dom::Length::Px(20.0),
        };
        style.padding = edge_all(4.0);
        style.background = Color::from_u32(0x21262DFF);
    });

    add_text(dom, right_header, "Telemetry", 1.1, |style| {
        style.size = dom::Size2::auto();
        style.foreground = Color::from_u32(0xF0F6FCFF);
    });

    for (idx, progress) in [0.25, 0.6, 0.85].iter().copied().enumerate() {
        let item = add_div(dom, right_panel, |style| {
            style.layout = dom::LayoutStyle {
                direction: dom::LayoutDirection::Column,
                align_items: dom::AlignItems::Stretch,
                justify_content: dom::JustifyContent::Start,
                gap: 6.0,
            };
            style.size = dom::Size2 {
                width: dom::Length::Fill,
                height: dom::Length::Px(64.0),
            };
            style.padding = edge_all(8.0);
            style.background = Color::from_u32(0x0D1117FF);
            style.border = edge_all(1.0);
            style.border_color = Color::from_u32(0x30363DFF);
        });

        let label = match idx {
            0 => "Power Core",
            1 => "Signal Link",
            _ => "Reactor Flow",
        };
        add_text(dom, item, label, 1.0, |style| {
            style.size = dom::Size2::auto();
            style.foreground = Color::from_u32(0xC9D1D9FF);
        });

        add_div(dom, item, |style| {
            let width = 0.55 + (idx as f32) * 0.1;
            style.size = dom::Size2 {
                width: dom::Length::Percent(width),
                height: dom::Length::Px(10.0),
            };
            style.background = Color::from_u32(0x30363DFF);
        });

        let bar = add_div(dom, item, |style| {
            style.size = dom::Size2 {
                width: dom::Length::Fill,
                height: dom::Length::Px(14.0),
            };
            style.background = Color::from_u32(0x21262DFF);
            style.border = edge_all(1.0);
            style.border_color = Color::from_u32(0x30363DFF);
        });

        add_div(dom, bar, |style| {
            style.position = dom::PositionStyle {
                mode: dom::PositionMode::Absolute,
                anchors: dom::Anchors {
                    left: true,
                    right: false,
                    top: true,
                    bottom: true,
                },
            };
            style.size = dom::Size2 {
                width: dom::Length::Percent(progress),
                height: dom::Length::Px(1.0),
            };
            style.margin = edge_all(1.0);
            style.background = Color::from_u32(0x238636FF);
        });
    }

    let bottom_bar = add_div(dom, root, |style| {
        style.layout = dom::LayoutStyle {
            direction: dom::LayoutDirection::Row,
            align_items: dom::AlignItems::Stretch,
            justify_content: dom::JustifyContent::Start,
            gap: 12.0,
        };
        style.size = dom::Size2 {
            width: dom::Length::Fill,
            height: dom::Length::Px(160.0),
        };
        style.padding = edge_all(12.0);
        style.background = Color::from_u32(0x161B22FF);
        style.border = edge_all(1.0);
        style.border_color = Color::from_u32(0x30363DFF);
    });

    let build_list = add_div(dom, bottom_bar, |style| {
        style.layout = dom::LayoutStyle {
            direction: dom::LayoutDirection::Column,
            align_items: dom::AlignItems::Stretch,
            justify_content: dom::JustifyContent::Start,
            gap: 8.0,
        };
        style.size = dom::Size2 {
            width: dom::Length::Px(300.0),
            height: dom::Length::Fill,
        };
    });

    add_text(dom, build_list, "Build Queue", 1.2, |style| {
        style.size = dom::Size2::auto();
        style.foreground = Color::from_u32(0xF0F6FCFF);
    });

    for label in ["Atlas Frame", "Artemis Core", "Helios Array", "Raptor Gear"]
        .iter()
        .copied()
    {
        let row = add_div(dom, build_list, |style| {
            style.layout = dom::LayoutStyle {
                direction: dom::LayoutDirection::Row,
                align_items: dom::AlignItems::Center,
                justify_content: dom::JustifyContent::Start,
                gap: 8.0,
            };
            style.size = dom::Size2 {
                width: dom::Length::Fill,
                height: dom::Length::Px(36.0),
            };
            style.padding = edge_all(8.0);
            style.background = Color::from_u32(0x21262DFF);
            style.border = edge_all(1.0);
            style.border_color = Color::from_u32(0x30363DFF);
        });

        register_hover_active_background(
            style,
            row,
            Color::from_u32(0x2A2F37FF),
            Color::from_u32(0x1B1F25FF),
        );

        add_text(dom, row, label, 1.0, |style| {
            style.size = dom::Size2::auto();
            style.foreground = Color::from_u32(0xF0F6FCFF);
        });
    }

    add_div(dom, bottom_bar, |style| {
        style.size = dom::Size2 {
            width: dom::Length::Fill,
            height: dom::Length::Fill,
        };
        style.background = Color::from_u32(0x161B22FF);
        style.border = edge_all(1.0);
        style.border_color = Color::from_u32(0x30363DFF);
    });
}

fn add_div(
    dom: &mut dom::Dom,
    parent: dom::NodeId,
    f: impl FnOnce(&mut dom::Style),
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
    f: impl FnOnce(&mut dom::Style),
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

fn edge_all(value: f32) -> dom::EdgeSizes {
    dom::EdgeSizes {
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

fn register_hover_active_background(
    style: &mut StyleComputer,
    node_id: dom::NodeId,
    hovered: Color,
    active: Color,
) {
    let hovered = dom::StyleOverride {
        background: Some(hovered),
        ..Default::default()
    };
    let active = dom::StyleOverride {
        background: Some(active),
        ..Default::default()
    };
    style.set_rules(
        node_id,
        StyleRules::new(dom::StyleOverride::default())
            .with_hovered(hovered)
            .with_active(active),
    );
}

fn draw_text(
    frame: &mut gfx::RenderFrame,
    rect: &gfx::Rectangle,
    style: &dom::Style,
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

fn draw_style(frame: &mut gfx::RenderFrame, rect: &gfx::Rectangle, style: &dom::Style) {
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

fn inset_rectangle(rect: &gfx::Rectangle, border: dom::EdgeSizes) -> gfx::Rectangle {
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
    border: dom::EdgeSizes,
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
