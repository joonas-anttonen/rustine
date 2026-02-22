#![allow(dead_code)]

use rustine::{
    gfx,
    gui::{self},
    log,
};

use crate::{EditorAction, TextEditor};

struct TextRenderMetrics {
    line_height: f32,
    char_advance: f32,
    caret_width: f32,
}

struct MyApplicationState {
    pub editor: TextEditor,
    pub metrics: TextRenderMetrics,
}

impl MyApplicationState {
    pub fn new() -> Self {
        let scale = 1.0;
        let font_id = gfx::fonts::CASKAYDIAMONO_FONT_ID;
        let single_line = gfx::measure_text("M", scale, font_id);
        let two_lines = gfx::measure_text("M\nM", scale, font_id);
        let char_advance = single_line.w.max(1.0);
        let line_height = (two_lines.h - single_line.h).max(1.0);

        MyApplicationState {
            editor: TextEditor::new(String::new()),
            metrics: TextRenderMetrics {
                line_height: line_height.max(1.0),
                char_advance,
                caret_width: 2.0,
            },
        }
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
        if Self::handle_command_shortcut(gui, &event, &self.state) {
            return;
        }

        if let Some(action) = Self::key_event_to_action(&event) {
            let mut state = self.state.borrow_mut();
            if state.editor.apply_action(action) {
                gui.mark_damaged();
            }
        }
    }

    fn on_char(&self, gui: &gui::Gui, c: char) {
        if let Some(action) = Self::char_to_action(c) {
            let mut state = self.state.borrow_mut();
            if state.editor.apply_action(action) {
                gui.mark_damaged();
            }
        }
    }

    fn on_mouse_enter(&self, _gui: &gui::Gui, _event: gui::MouseEnterEvent) {
        let _state = self.state.borrow_mut();
    }

    fn on_mouse_leave(&self, _gui: &gui::Gui, _event: gui::MouseLeaveEvent) {}

    fn on_mouse_move(&self, _gui: &gui::Gui, _event: gui::MouseMoveEvent) {
        let _state = self.state.borrow_mut();
    }

    fn on_mouse_button(&self, _gui: &gui::Gui, _event: gui::MouseButtonEvent) {
        let _state = self.state.borrow_mut();
    }

    fn render(&self, _gui: &gui::Gui, frame: &mut gfx::RenderFrame) {
        let state = self.state.borrow();

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

        let text = state.editor.text();
        frame.push_text(text, x, y, scale, color, font_id);

        let cursor_line = state.editor.cursor_line();
        let cursor_col = state.editor.cursor_column();
        let caret_x = x + cursor_col as f32 * state.metrics.char_advance;
        let caret_baseline_y = y + cursor_line as f32 * state.metrics.line_height;
        let caret_rect = gfx::Rectangle {
            x: caret_x,
            y: caret_baseline_y - state.metrics.line_height,
            w: state.metrics.caret_width,
            h: state.metrics.line_height,
        };

        frame.fill_rectangle(&caret_rect, 0xFFFFFFFF);
    }
}

impl MyApplication {
    fn handle_command_shortcut(
        gui: &gui::Gui,
        event: &gui::KeyEvent,
        state: &std::cell::RefCell<MyApplicationState>,
    ) -> bool {
        if event.action != gui::Action::PRESS || !Self::has_primary_modifier(&event.mods) {
            return false;
        }

        match event.key {
            gui::Key::C => {
                let text = {
                    let state = state.borrow();
                    state.editor.text().to_string()
                };
                let _ = gui.set_clipboard_text(&text);
                true
            }
            gui::Key::V => {
                if let Some(text) = gui.clipboard_text() {
                    let mut state = state.borrow_mut();
                    if state.editor.insert_text(text) {
                        gui.mark_damaged();
                    }
                }
                true
            }
            gui::Key::Z => {
                let action = if event.mods.contains(gui::Mods::SHIFT) {
                    EditorAction::Redo
                } else {
                    EditorAction::Undo
                };

                let mut state = state.borrow_mut();
                if state.editor.apply_action(action) {
                    gui.mark_damaged();
                }
                true
            }
            gui::Key::Y => {
                let mut state = state.borrow_mut();
                if state.editor.apply_action(EditorAction::Redo) {
                    gui.mark_damaged();
                }
                true
            }
            _ => false,
        }
    }

    fn key_event_to_action(event: &gui::KeyEvent) -> Option<EditorAction> {
        if !matches!(event.action, gui::Action::PRESS | gui::Action::REPEAT) {
            return None;
        }

        match event.key {
            gui::Key::ENTER => Some(EditorAction::InsertNewline),
            gui::Key::SPACE => Some(EditorAction::InsertSpace),
            gui::Key::BACKSPACE => Some(EditorAction::Backspace),
            gui::Key::DELETE => Some(EditorAction::Delete),
            gui::Key::LEFT => Some(EditorAction::MoveLeft),
            gui::Key::RIGHT => Some(EditorAction::MoveRight),
            gui::Key::UP => Some(EditorAction::MoveUp),
            gui::Key::DOWN => Some(EditorAction::MoveDown),
            gui::Key::HOME => Some(EditorAction::MoveHome),
            gui::Key::END => Some(EditorAction::MoveEnd),
            _ => None,
        }
    }

    fn has_primary_modifier(mods: &gui::Mods) -> bool {
        mods.contains(gui::Mods::CONTROL) || mods.contains(gui::Mods::SUPER)
    }

    fn char_to_action(c: char) -> Option<EditorAction> {
        if !c.is_control() && !c.is_whitespace() {
            return Some(EditorAction::InsertChar(c));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_to_action_accepts_currency_symbols() {
        assert!(matches!(
            MyApplication::char_to_action('€'),
            Some(EditorAction::InsertChar('€'))
        ));
        assert!(matches!(
            MyApplication::char_to_action('£'),
            Some(EditorAction::InsertChar('£'))
        ));
    }

    #[test]
    fn char_to_action_keeps_whitespace_filtered() {
        assert!(MyApplication::char_to_action(' ').is_none());
        assert!(MyApplication::char_to_action('\n').is_none());
        assert!(MyApplication::char_to_action('\t').is_none());
    }
}
