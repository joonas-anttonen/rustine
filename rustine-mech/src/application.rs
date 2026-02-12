use rustine::{
    Color, Vector2f, gfx,
    gui::{self, dom},
    log,
};

struct MyApplicationState {
    dom: dom::Dom,
    root: dom::NodeId,
}

pub struct MyApplication {
    state: std::cell::RefCell<MyApplicationState>,
}

impl MyApplication {
    pub fn new() -> Self {
        let mut dom = dom::Dom::new();
        let root = dom.root();
        build_mock_ui(&mut dom, root);

        MyApplication {
            state: std::cell::RefCell::new(MyApplicationState { dom, root }),
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

    fn render(&self, gui: &gui::Gui, frame: &mut gfx::RenderFrame) {
        let mut state = self.state.borrow_mut();
        let pixel_size = gui.pixel_size();
        let root_size = Vector2f::new(pixel_size.x as f32, pixel_size.y as f32);

        state.dom.layout(root_size);
        render_dom(&state.dom, state.root, frame);
    }
}

fn build_mock_ui(dom: &mut dom::Dom, root: dom::NodeId) {
    let root_style = dom.node_mut(root).unwrap().style_mut().unwrap();
    root_style.layout = dom::LayoutStyle {
        direction: dom::LayoutDirection::Column,
        align_items: dom::AlignItems::Stretch,
        justify_content: dom::JustifyContent::Start,
        gap: 16.0,
    };
    root_style.padding = edge_all(16.0);
    root_style.background = Color::from_u32(0x101418FF);
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
        style.background = Color::from_u32(0x1A2128FF);
        style.border = edge_all(1.0);
        style.border_color = Color::from_u32(0x2B3640FF);
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

    for _ in 0..3 {
        add_div(dom, economy_row, |style| {
            style.size = dom::Size2 {
                width: dom::Length::Px(160.0),
                height: dom::Length::Fill,
            };
            style.padding = edge_all(10.0);
            style.background = Color::from_u32(0x232C35FF);
            style.border = edge_all(1.0);
            style.border_color = Color::from_u32(0x36424EFF);
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

    for _ in 0..3 {
        add_div(dom, button_row, |style| {
            style.size = dom::Size2 {
                width: dom::Length::Px(120.0),
                height: dom::Length::Px(36.0),
            };
            style.background = Color::from_u32(0x32404BFF);
            style.border = edge_all(1.0);
            style.border_color = Color::from_u32(0x4A5B67FF);
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
        style.background = Color::from_u32(0x171D22FF);
        style.border = edge_all(1.0);
        style.border_color = Color::from_u32(0x2B343DFF);
    });

    add_div(dom, left_panel, |style| {
        style.size = dom::Size2 {
            width: dom::Length::Px(240.0),
            height: dom::Length::Px(20.0),
        };
        style.background = Color::from_u32(0x2C3842FF);
    });

    for _ in 0..6 {
        add_div(dom, left_panel, |style| {
            style.size = dom::Size2 {
                width: dom::Length::Fill,
                height: dom::Length::Px(36.0),
            };
            style.background = Color::from_u32(0x1F2931FF);
            style.border = edge_all(1.0);
            style.border_color = Color::from_u32(0x2C3842FF);
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
        style.background = Color::from_u32(0x1A2128FF);
        style.border = edge_all(1.0);
        style.border_color = Color::from_u32(0x2B3640FF);
    });

    add_div(dom, right_panel, |style| {
        style.size = dom::Size2 {
            width: dom::Length::Px(180.0),
            height: dom::Length::Px(20.0),
        };
        style.background = Color::from_u32(0x2C3842FF);
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
            style.background = Color::from_u32(0x212A33FF);
            style.border = edge_all(1.0);
            style.border_color = Color::from_u32(0x2F3B46FF);
        });

        add_div(dom, item, |style| {
            let width = 0.55 + (idx as f32) * 0.1;
            style.size = dom::Size2 {
                width: dom::Length::Percent(width),
                height: dom::Length::Px(10.0),
            };
            style.background = Color::from_u32(0x3B4753FF);
        });

        let bar = add_div(dom, item, |style| {
            style.size = dom::Size2 {
                width: dom::Length::Fill,
                height: dom::Length::Px(14.0),
            };
            style.background = Color::from_u32(0x2B3640FF);
            style.border = edge_all(1.0);
            style.border_color = Color::from_u32(0x3A4652FF);
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
            style.background = Color::from_u32(0x6BCB77FF);
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
        style.background = Color::from_u32(0x1A2128FF);
        style.border = edge_all(1.0);
        style.border_color = Color::from_u32(0x2B3640FF);
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

    for _ in 0..4 {
        add_div(dom, build_list, |style| {
            style.size = dom::Size2 {
                width: dom::Length::Fill,
                height: dom::Length::Px(36.0),
            };
            style.background = Color::from_u32(0x32404BFF);
            style.border = edge_all(1.0);
            style.border_color = Color::from_u32(0x4A5B67FF);
        });
    }

    add_div(dom, bottom_bar, |style| {
        style.size = dom::Size2 {
            width: dom::Length::Fill,
            height: dom::Length::Fill,
        };
        style.background = Color::from_u32(0x202831FF);
        style.border = edge_all(1.0);
        style.border_color = Color::from_u32(0x2C3842FF);
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

fn edge_all(value: f32) -> dom::EdgeSizes {
    dom::EdgeSizes {
        left: value,
        right: value,
        top: value,
        bottom: value,
    }
}

fn render_dom(dom: &dom::Dom, node_id: dom::NodeId, frame: &mut gfx::RenderFrame) {
    let Some(node) = dom.node(node_id) else {
        return;
    };

    if let Some(style) = node.style() {
        let rect = gfx::Rectangle {
            x: node.layout.position.x,
            y: node.layout.position.y,
            w: node.layout.size.x,
            h: node.layout.size.y,
        };
        draw_style(frame, &rect, style);
    }

    for child_id in node.children.iter().copied() {
        render_dom(dom, child_id, frame);
    }
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
