#![allow(dead_code)]

use rustine::{
    gfx,
    gui::{self},
    log,
};

struct MyApplicationState {
    pub editor: TextEditor,
}

pub struct MyApplication {
    state: std::cell::RefCell<MyApplicationState>,
}

impl MyApplication {
    pub fn new() -> Self {
        MyApplication {
            state: std::cell::RefCell::new(MyApplicationState {
                editor: TextEditor {
                    input_buffer: String::new(),
                },
            }),
        }
    }
}

impl Drop for MyApplication {
    fn drop(&mut self) {}
}

// "Operators" in Markdown
// #        headings
// >        blockquote
// `        inline code
// -        unordered list
// ---      horizontal rule
// !        image
// [        link-text
// (        link-url
//
// Ignored for now:
// *        italic
// **       bold

struct TextEditor {
    input_buffer: String,
}

impl TextEditor {
    pub fn push_input(&mut self, c: char) {
        self.input_buffer.push(c);
    }
}

impl gui::Application for MyApplication {
    fn startup(&self, _gui: &gui::Gui) {
        log::info!("MyApplication::startup");
    }

    fn on_key(&self, gui: &gui::Gui, event: gui::KeyEvent) {
        let mut _state = self.state.borrow_mut();

        if event.action != gui::Action::PRESS {
            return;
        }

        if event.key == gui::Key::DELETE || event.key == gui::Key::BACKSPACE {
            log::info!("Received DELETE action with key: {:?}", event.key);
            _state.editor.input_buffer.pop();
        }

        // Assume the window is damaged after handling a key press
        gui.mark_damaged();
    }

    fn on_char(&self, gui: &gui::Gui, c: char) {
        // Assume the window is damaged after handling a character input
        gui.mark_damaged();

        if c.is_alphanumeric() || c.is_ascii_punctuation() || c == ' ' || c == '\n' {
            // Log the received character input
            //log::info!("Received char input: '{}'", c);

            let mut state = self.state.borrow_mut();
            state.editor.push_input(c);
        } else {
            // Log non-printable character inputs with their Unicode code point
            //log::info!("Received non-printable char input: U+{:04X}", _c as u32);
        }
    }

    fn on_mouse_enter(&self, _gui: &gui::Gui, _event: gui::MouseEnterEvent) {
        let mut _state = self.state.borrow_mut();
    }

    fn on_mouse_leave(&self, _gui: &gui::Gui, _event: gui::MouseLeaveEvent) {}

    fn on_mouse_move(&self, _gui: &gui::Gui, _event: gui::MouseMoveEvent) {
        let mut _state = self.state.borrow_mut();
    }

    fn on_mouse_button(&self, _gui: &gui::Gui, _event: gui::MouseButtonEvent) {
        let mut _state = self.state.borrow_mut();
    }

    fn render(&self, _gui: &gui::Gui, frame: &mut gfx::RenderFrame) {
        let state = self.state.borrow_mut();

        let rect = gfx::Rectangle {
            x: 0.0,
            y: 0.0,
            w: frame.size.x as f32,
            h: frame.size.y as f32,
        };
        let color = 0x1B232F_FF;
        frame.fill_rectangle(&rect, color);

        let x = 10.0;
        let y = 20.0;
        let scale = 1.0;
        let color = 0xFFFFFFFF;
        let font_id = gfx::fonts::CASKAYDIAMONO_FONT_ID;

        frame.push_text(
            state.editor.input_buffer.as_str(),
            x,
            y,
            scale,
            color,
            font_id,
        );
    }
}


