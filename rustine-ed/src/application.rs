#![allow(dead_code)]

use rustine::{
    gfx,
    gui::{self},
    log,
};

use crate::PieceTable;

struct MyApplicationState {
    pub text_table: PieceTable,
    pub text_cache: String,
}

impl MyApplicationState {
    pub fn new() -> Self {
        MyApplicationState {
            text_table: PieceTable::new(String::new()),
            text_cache: String::with_capacity(1024),
        }
    }

    pub fn update_text_cache(&mut self) {
        self.text_cache.clear();
        self.text_table.get_text(&mut self.text_cache);
    }
}

pub struct MyApplication {
    state: std::cell::RefCell<MyApplicationState>,
}

impl MyApplication {
    pub fn new() -> Self {
        MyApplication {
            state: std::cell::RefCell::new(MyApplicationState::new()),
        }
    }
}

impl Drop for MyApplication {
    fn drop(&mut self) {}
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
            //_state.editor.input_buffer.pop();
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

            let mut _state = self.state.borrow_mut();
            //state.editor.push_input(c);
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
        let mut state = self.state.borrow_mut();
        state.update_text_cache();

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

        frame.push_text(&state.text_cache, x, y, scale, color, font_id);
    }
}
