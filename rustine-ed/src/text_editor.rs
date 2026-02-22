#![allow(dead_code)]

use crate::PieceTable;
use crate::piecetable::PieceTableDebugState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorAction {
    InsertChar(char),
    InsertNewline,
    InsertSpace,
    Backspace,
    Delete,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    MoveHome,
    MoveEnd,
    Undo,
    Redo,
}

pub(crate) struct TextEditor {
    text_table: PieceTable,
    text_cache: String,
    text_len_chars: usize,
    line_starts: Vec<usize>,
    cursor: usize,
    cursor_line: usize,
    cursor_column: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct TextEditorDebugState {
    pub cursor: usize,
    pub cursor_line: usize,
    pub cursor_column: usize,
    pub text_len_chars: usize,
    pub line_count: usize,
    pub piece_table: PieceTableDebugState,
}

impl TextEditor {
    pub fn new(initial: String) -> Self {
        let cursor = initial.chars().count();
        let mut editor = Self {
            text_table: PieceTable::new(initial),
            text_cache: String::with_capacity(1024),
            text_len_chars: 0,
            line_starts: Vec::new(),
            cursor,
            cursor_line: 0,
            cursor_column: 0,
        };
        editor.rebuild_text_cache();
        editor.sync_cursor_position();
        editor
    }

    pub fn apply_action(&mut self, action: EditorAction) -> bool {
        match action {
            EditorAction::InsertChar(c) => {
                self.text_table.insert(self.cursor, c.to_string());
                self.cursor += 1;
                self.refresh_after_text_change();
                true
            }
            EditorAction::InsertNewline => {
                self.text_table.insert(self.cursor, "\n".to_string());
                self.cursor += 1;
                self.refresh_after_text_change();
                true
            }
            EditorAction::InsertSpace => {
                self.text_table.insert(self.cursor, " ".to_string());
                self.cursor += 1;
                self.refresh_after_text_change();
                true
            }
            EditorAction::Backspace => {
                if self.cursor == 0 {
                    return false;
                }

                self.text_table.delete(self.cursor - 1, 1);
                self.cursor -= 1;
                self.refresh_after_text_change();
                true
            }
            EditorAction::Delete => {
                let text_len = self.text_len();
                if self.cursor >= text_len {
                    return false;
                }

                self.text_table.delete(self.cursor, 1);
                self.refresh_after_text_change();
                true
            }
            EditorAction::MoveLeft => {
                self.set_cursor(self.cursor.saturating_sub(1))
            }
            EditorAction::MoveRight => {
                let text_len = self.text_len();
                if self.cursor >= text_len {
                    return false;
                }
                self.set_cursor(self.cursor + 1)
            }
            EditorAction::MoveUp => self.move_cursor_up(),
            EditorAction::MoveDown => self.move_cursor_down(),
            EditorAction::MoveHome => self.set_cursor(0),
            EditorAction::MoveEnd => {
                let text_len = self.text_len();
                self.set_cursor(text_len)
            }
            EditorAction::Undo => {
                if self.text_table.undo() {
                    self.refresh_after_text_change();
                    true
                } else {
                    false
                }
            }
            EditorAction::Redo => {
                if self.text_table.redo() {
                    self.refresh_after_text_change();
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn insert_text(&mut self, text: String) -> bool {
        if text.is_empty() {
            return false;
        }

        let text_len = text.chars().count();
        self.text_table.insert(self.cursor, text);
        self.cursor += text_len;
        self.refresh_after_text_change();
        true
    }

    pub fn text(&self) -> &str {
        &self.text_cache
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn cursor_line(&self) -> usize {
        self.cursor_line
    }

    pub fn cursor_column(&self) -> usize {
        self.cursor_column
    }

    pub fn debug_state(&self) -> TextEditorDebugState {
        TextEditorDebugState {
            cursor: self.cursor,
            cursor_line: self.cursor_line,
            cursor_column: self.cursor_column,
            text_len_chars: self.text_len_chars,
            line_count: self.line_starts.len(),
            piece_table: self.text_table.debug_state(),
        }
    }

    fn text_len(&self) -> usize {
        self.text_len_chars
    }

    fn refresh_after_text_change(&mut self) {
        self.rebuild_text_cache();
        self.sync_cursor_position();
    }

    fn rebuild_text_cache(&mut self) {
        self.text_cache.clear();
        self.text_table.get_text(&mut self.text_cache);

        self.line_starts.clear();
        self.line_starts.push(0);

        let mut text_len_chars = 0usize;
        for c in self.text_cache.chars() {
            if c == '\n' {
                self.line_starts.push(text_len_chars + 1);
            }
            text_len_chars += 1;
        }
        self.text_len_chars = text_len_chars;
    }

    fn sync_cursor_position(&mut self) {
        self.cursor = self.cursor.min(self.text_len_chars);

        let line_index = match self.line_starts.binary_search(&self.cursor) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };
        self.cursor_line = line_index;

        let line_start = self.line_starts[line_index];
        let line_len = self.line_length(line_index);
        self.cursor_column = (self.cursor - line_start).min(line_len);
    }

    fn line_end_char_index(&self, line_index: usize) -> usize {
        if line_index + 1 < self.line_starts.len() {
            self.line_starts[line_index + 1] - 1
        } else {
            self.text_len_chars
        }
    }

    fn line_length(&self, line_index: usize) -> usize {
        self.line_end_char_index(line_index) - self.line_starts[line_index]
    }

    fn set_cursor(&mut self, cursor: usize) -> bool {
        let clamped = cursor.min(self.text_len_chars);
        if self.cursor == clamped {
            return false;
        }

        self.cursor = clamped;
        self.sync_cursor_position();
        true
    }

    fn move_cursor_up(&mut self) -> bool {
        if self.cursor_line == 0 {
            return false;
        }

        let target_line = self.cursor_line - 1;
        let target_col = self.cursor_column.min(self.line_length(target_line));
        let target_cursor = self.line_starts[target_line] + target_col;
        self.set_cursor(target_cursor)
    }

    fn move_cursor_down(&mut self) -> bool {
        let last_line = self.line_starts.len() - 1;
        if self.cursor_line >= last_line {
            return false;
        }

        let target_line = self.cursor_line + 1;
        let target_col = self.cursor_column.min(self.line_length(target_line));
        let target_cursor = self.line_starts[target_line] + target_col;
        self.set_cursor(target_cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::{EditorAction, TextEditor};

    #[test]
    fn insert_newline_and_space_actions() {
        let mut editor = TextEditor::new(String::new());

        assert!(editor.apply_action(EditorAction::InsertChar('a')));
        assert!(editor.apply_action(EditorAction::InsertSpace));
        assert!(editor.apply_action(EditorAction::InsertChar('b')));
        assert!(editor.apply_action(EditorAction::InsertNewline));
        assert!(editor.apply_action(EditorAction::InsertChar('c')));

        assert_eq!(editor.text(), "a b\nc");
        assert_eq!(editor.cursor(), 5);
    }

    #[test]
    fn backspace_and_delete_actions() {
        let mut editor = TextEditor::new("abcd".to_string());

        assert!(editor.apply_action(EditorAction::MoveLeft));
        assert!(editor.apply_action(EditorAction::Backspace));
        assert_eq!(editor.text(), "abd");
        assert_eq!(editor.cursor(), 2);

        assert!(editor.apply_action(EditorAction::Delete));
        assert_eq!(editor.text(), "ab");
        assert_eq!(editor.cursor(), 2);

        assert!(!editor.apply_action(EditorAction::Delete));
    }

    #[test]
    fn home_end_and_horizontal_moves() {
        let mut editor = TextEditor::new("hello".to_string());

        assert!(editor.apply_action(EditorAction::MoveHome));
        assert_eq!(editor.cursor(), 0);

        assert!(!editor.apply_action(EditorAction::MoveLeft));
        assert!(editor.apply_action(EditorAction::MoveRight));
        assert_eq!(editor.cursor(), 1);

        assert!(editor.apply_action(EditorAction::MoveEnd));
        assert_eq!(editor.cursor(), 5);
        assert!(!editor.apply_action(EditorAction::MoveRight));
    }

    #[test]
    fn move_up_down_preserves_column() {
        let mut editor = TextEditor::new("12345\n12\n1234".to_string());

        assert!(editor.apply_action(EditorAction::MoveHome));
        for _ in 0..4 {
            assert!(editor.apply_action(EditorAction::MoveRight));
        }
        assert_eq!(editor.cursor(), 4);

        assert!(editor.apply_action(EditorAction::MoveDown));
        assert_eq!(editor.cursor(), 8);

        assert!(editor.apply_action(EditorAction::MoveDown));
        assert_eq!(editor.cursor(), 11);

        assert!(editor.apply_action(EditorAction::MoveUp));
        assert_eq!(editor.cursor(), 8);
        assert_eq!(editor.cursor_line(), 1);
        assert_eq!(editor.cursor_column(), 2);
    }

    #[test]
    fn cursor_line_and_column_track_position() {
        let mut editor = TextEditor::new("ab\ncd".to_string());

        assert_eq!(editor.cursor(), 5);
        assert_eq!(editor.cursor_line(), 1);
        assert_eq!(editor.cursor_column(), 2);

        assert!(editor.apply_action(EditorAction::MoveHome));
        assert_eq!(editor.cursor_line(), 0);
        assert_eq!(editor.cursor_column(), 0);

        assert!(editor.apply_action(EditorAction::MoveRight));
        assert!(editor.apply_action(EditorAction::MoveRight));
        assert_eq!(editor.cursor_line(), 0);
        assert_eq!(editor.cursor_column(), 2);

        assert!(editor.apply_action(EditorAction::MoveRight));
        assert_eq!(editor.cursor_line(), 1);
        assert_eq!(editor.cursor_column(), 0);
    }

    #[test]
    fn insert_text_inserts_multiple_chars() {
        let mut editor = TextEditor::new("ab".to_string());
        assert!(editor.apply_action(EditorAction::MoveHome));
        assert!(editor.apply_action(EditorAction::MoveRight));

        assert!(editor.insert_text("XYZ".to_string()));
        assert_eq!(editor.text(), "aXYZb");
        assert_eq!(editor.cursor(), 4);
    }

    #[test]
    fn undo_and_redo_actions_work() {
        let mut editor = TextEditor::new("abc".to_string());
        assert!(editor.apply_action(EditorAction::InsertChar('d')));
        assert_eq!(editor.text(), "abcd");

        assert!(editor.apply_action(EditorAction::Undo));
        assert_eq!(editor.text(), "abc");

        assert!(editor.apply_action(EditorAction::Redo));
        assert_eq!(editor.text(), "abcd");

        assert!(!editor.apply_action(EditorAction::Redo));
    }
}
