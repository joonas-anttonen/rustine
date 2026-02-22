#![allow(dead_code)]

use rustine::{
    gfx,
    gui::{self},
    log,
};

use crate::{EditorAction, TextEditor};
use crate::text_editor::TextEditorDebugState;

const EDITOR_EXTRA_LINE_GAP: f32 = 2.0;

struct TextRenderMetrics {
    line_height: f32,
    char_advance: f32,
    caret_width: f32,
    ascender: f32,
    extra_line_gap: f32,
}

struct MyApplicationState {
    pub editor: TextEditor,
    pub metrics: TextRenderMetrics,
    pub show_debug_overlay: bool,
}

impl MyApplicationState {
    pub fn new() -> Self {
        let scale = 1.0;
        let font_id = gfx::fonts::CASKAYDIAMONO_FONT_ID;
        let font_metrics = gfx::fonts::get_font_metrics(font_id).expect("Font metrics exist");
        let single_line = gfx::measure_text("M", scale, font_id);
        let char_advance = single_line.w.max(1.0);
        let line_height = (font_metrics.ascender
            - font_metrics.descender
            + font_metrics.line_gap
            + EDITOR_EXTRA_LINE_GAP)
            .max(1.0);

        MyApplicationState {
            editor: TextEditor::new(String::new()),
            metrics: TextRenderMetrics {
                line_height: line_height.max(1.0),
                char_advance,
                caret_width: 2.0,
                ascender: font_metrics.ascender,
                extra_line_gap: EDITOR_EXTRA_LINE_GAP,
            },
            show_debug_overlay: false,
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
        if Self::handle_debug_shortcut(gui, &event, &self.state) {
            return;
        }

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

        let x = 2.0;
        let text_top = 2.0;
        let text_baseline_origin = text_top + state.metrics.ascender;
        let scale = 1.0;
        let color = 0xFFFFFFFF;
        let font_id = gfx::fonts::CASKAYDIAMONO_FONT_ID;

        let selection_color = 0x3A6EA5CC;
        for segment in state.editor.selection_segments() {
            let segment_baseline_y =
                text_baseline_origin + segment.line as f32 * state.metrics.line_height;
            let selection_rect = gfx::Rectangle {
                x: x + segment.start_column as f32 * state.metrics.char_advance,
                y: segment_baseline_y - state.metrics.ascender,
                w: (segment.end_column - segment.start_column) as f32 * state.metrics.char_advance,
                h: state.metrics.line_height,
            };
            frame.fill_rectangle(&selection_rect, selection_color);
        }

        for (line_index, line_text) in state.editor.text().split('\n').enumerate() {
            let baseline_y = text_baseline_origin + line_index as f32 * state.metrics.line_height;
            frame.push_text(line_text, x, baseline_y, scale, color, font_id);
        }

        let cursor_line = state.editor.cursor_line();
        let cursor_col = state.editor.cursor_column();
        let caret_x = x + cursor_col as f32 * state.metrics.char_advance;
        let caret_baseline_y = text_baseline_origin + cursor_line as f32 * state.metrics.line_height;
        let caret_rect = gfx::Rectangle {
            x: caret_x,
            y: caret_baseline_y - state.metrics.ascender,
            w: state.metrics.caret_width,
            h: state.metrics.line_height,
        };

        frame.fill_rectangle(&caret_rect, 0xFFFFFFFF);

        if state.show_debug_overlay {
            let snapshot = state.editor.debug_state();
            Self::draw_debug_overlay(frame, &snapshot, &state.metrics);
        }
    }
}

impl MyApplication {
    fn handle_debug_shortcut(
        gui: &gui::Gui,
        event: &gui::KeyEvent,
        state: &std::cell::RefCell<MyApplicationState>,
    ) -> bool {
        if event.action != gui::Action::PRESS {
            return false;
        }

        if event.key != gui::Key::F2 {
            return false;
        }

        let mut state = state.borrow_mut();
        state.show_debug_overlay = !state.show_debug_overlay;
        gui.mark_damaged();
        true
    }

    fn handle_command_shortcut(
        gui: &gui::Gui,
        event: &gui::KeyEvent,
        state: &std::cell::RefCell<MyApplicationState>,
    ) -> bool {
        if event.action != gui::Action::PRESS || !Self::has_primary_modifier(&event.mods) {
            return false;
        }

        match event.key {
            gui::Key::A => {
                let mut state = state.borrow_mut();
                if state.editor.apply_action(EditorAction::SelectAll) {
                    gui.mark_damaged();
                }
                true
            }
            gui::Key::C => {
                let text = {
                    let state = state.borrow();
                    state
                        .editor
                        .selected_text()
                        .unwrap_or_else(|| state.editor.text().to_string())
                };
                let _ = gui.set_clipboard_text(&text);
                true
            }
            gui::Key::X => {
                let selected_text = {
                    let state = state.borrow();
                    state.editor.selected_text()
                };

                if let Some(text) = selected_text {
                    let _ = gui.set_clipboard_text(&text);
                    let mut state = state.borrow_mut();
                    if state.editor.apply_action(EditorAction::Delete) {
                        gui.mark_damaged();
                    }
                }
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

        let selecting = event.mods.contains(gui::Mods::SHIFT);
        let primary_modifier = Self::has_primary_modifier(&event.mods);

        match event.key {
            gui::Key::ENTER => Some(EditorAction::InsertNewline),
            gui::Key::SPACE => Some(EditorAction::InsertSpace),
            gui::Key::BACKSPACE => Some(EditorAction::Backspace),
            gui::Key::DELETE => Some(EditorAction::Delete),
            gui::Key::LEFT => Some(if selecting {
                EditorAction::SelectLeft
            } else {
                EditorAction::MoveLeft
            }),
            gui::Key::RIGHT => Some(if selecting {
                EditorAction::SelectRight
            } else {
                EditorAction::MoveRight
            }),
            gui::Key::UP => Some(if selecting {
                EditorAction::SelectUp
            } else {
                EditorAction::MoveUp
            }),
            gui::Key::DOWN => Some(if selecting {
                EditorAction::SelectDown
            } else {
                EditorAction::MoveDown
            }),
            gui::Key::HOME => Some(if primary_modifier {
                if selecting {
                    EditorAction::SelectDocumentHome
                } else {
                    EditorAction::MoveDocumentHome
                }
            } else {
                if selecting {
                    EditorAction::SelectHome
                } else {
                    EditorAction::MoveHome
                }
            }),
            gui::Key::END => Some(if primary_modifier {
                if selecting {
                    EditorAction::SelectDocumentEnd
                } else {
                    EditorAction::MoveDocumentEnd
                }
            } else {
                if selecting {
                    EditorAction::SelectEnd
                } else {
                    EditorAction::MoveEnd
                }
            }),
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

    fn draw_debug_overlay(
        frame: &mut gfx::RenderFrame,
        snapshot: &TextEditorDebugState,
        metrics: &TextRenderMetrics,
    ) {
        let max_piece_lines = 12usize;
        let mut lines = Vec::new();

        lines.push(format!(
            "cursor={} (line {}, col {})",
            snapshot.cursor, snapshot.cursor_line, snapshot.cursor_column
        ));
        lines.push(format!(
            "selection={:?}..{:?} anchor={:?}",
            snapshot.selection_start, snapshot.selection_end, snapshot.selection_anchor
        ));
        lines.push(format!(
            "text_len={} lines={}",
            snapshot.text_len_chars, snapshot.line_count
        ));
        lines.push(format!("line_height={} extra_gap={}", metrics.line_height, metrics.extra_line_gap));
        lines.push(format!(
            "pieces={} doc_len={} undo={} redo={}",
            snapshot.piece_table.piece_count,
            snapshot.piece_table.document_len,
            snapshot.piece_table.undo_depth,
            snapshot.piece_table.redo_depth
        ));

        for (i, piece) in snapshot
            .piece_table
            .pieces
            .iter()
            .take(max_piece_lines)
            .enumerate()
        {
            lines.push(format!(
                "#{i:02} {} s:{} l:{} \"{}\"",
                piece.buffer, piece.start, piece.length, piece.preview
            ));
        }

        if snapshot.piece_table.pieces.len() > max_piece_lines {
            lines.push(format!(
                "... {} more pieces",
                snapshot.piece_table.pieces.len() - max_piece_lines
            ));
        }

        let max_chars = lines.iter().map(|line| line.chars().count()).max().unwrap_or(0);
        let padding = 0.0f32;
        let line_height = metrics.line_height.max(1.0);
        let width = (max_chars as f32 * metrics.char_advance) + padding * 2.0;
        let height = (lines.len() as f32 * line_height) + padding * 2.0;
        let x = (frame.size.x as f32 - width - 10.0).max(10.0);
        let y = 10.0;

        let bg = gfx::Rectangle {
            x,
            y,
            w: width,
            h: height,
        };
        frame.fill_rectangle(&bg, 0x000000D0);

        let text = lines.join("\n");
        frame.push_text(
            &text,
            x + padding,
            y + padding + line_height,
            1.0,
            0xFFFFFFFF,
            gfx::fonts::CASKAYDIAMONO_FONT_ID,
        );
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
