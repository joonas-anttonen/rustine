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
    MoveDocumentHome,
    MoveDocumentEnd,
    SelectLeft,
    SelectRight,
    SelectUp,
    SelectDown,
    SelectHome,
    SelectEnd,
    SelectDocumentHome,
    SelectDocumentEnd,
    SelectAll,
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
    selection_anchor: Option<usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct TextEditorDebugState {
    pub cursor: usize,
    pub cursor_line: usize,
    pub cursor_column: usize,
    pub selection_anchor: Option<usize>,
    pub selection_start: Option<usize>,
    pub selection_end: Option<usize>,
    pub text_len_chars: usize,
    pub line_count: usize,
    pub piece_table: PieceTableDebugState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelectionSegment {
    pub line: usize,
    pub start_column: usize,
    pub end_column: usize,
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
            selection_anchor: None,
        };
        editor.rebuild_text_cache();
        editor.sync_cursor_position();
        editor
    }

    pub fn apply_action(&mut self, action: EditorAction) -> bool {
        match action {
            EditorAction::InsertChar(c) => {
                self.insert_text_internal(c.to_string())
            }
            EditorAction::InsertNewline => {
                self.insert_text_internal("\n".to_string())
            }
            EditorAction::InsertSpace => {
                self.insert_text_internal(" ".to_string())
            }
            EditorAction::Backspace => {
                if self.delete_selection() {
                    return true;
                }

                if self.cursor == 0 {
                    return false;
                }

                self.text_table.delete(self.cursor - 1, 1);
                self.cursor -= 1;
                self.refresh_after_text_change();
                true
            }
            EditorAction::Delete => {
                if self.delete_selection() {
                    return true;
                }

                let text_len = self.text_len();
                if self.cursor >= text_len {
                    return false;
                }

                self.text_table.delete(self.cursor, 1);
                self.refresh_after_text_change();
                true
            }
            EditorAction::MoveLeft => {
                if let Some((selection_start, _)) = self.selection_range() {
                    return self.clear_selection_and_set_cursor(selection_start);
                }

                self.move_cursor(self.cursor.saturating_sub(1), false)
            }
            EditorAction::MoveRight => {
                if let Some((_, selection_end)) = self.selection_range() {
                    return self.clear_selection_and_set_cursor(selection_end);
                }

                let text_len = self.text_len();
                if self.cursor >= text_len {
                    return false;
                }
                self.move_cursor(self.cursor + 1, false)
            }
            EditorAction::MoveUp => self.move_cursor_up(false),
            EditorAction::MoveDown => self.move_cursor_down(false),
            EditorAction::MoveHome => {
                let line_start = self.line_starts[self.cursor_line];
                self.move_cursor(line_start, false)
            }
            EditorAction::MoveEnd => {
                let line_end = self.line_end_char_index(self.cursor_line);
                self.move_cursor(line_end, false)
            }
            EditorAction::MoveDocumentHome => self.move_cursor(0, false),
            EditorAction::MoveDocumentEnd => {
                let text_len = self.text_len();
                self.move_cursor(text_len, false)
            }
            EditorAction::SelectLeft => {
                self.move_cursor(self.cursor.saturating_sub(1), true)
            }
            EditorAction::SelectRight => {
                let text_len = self.text_len();
                if self.cursor >= text_len {
                    return false;
                }
                self.move_cursor(self.cursor + 1, true)
            }
            EditorAction::SelectUp => self.move_cursor_up(true),
            EditorAction::SelectDown => self.move_cursor_down(true),
            EditorAction::SelectHome => {
                let line_start = self.line_starts[self.cursor_line];
                self.move_cursor(line_start, true)
            }
            EditorAction::SelectEnd => {
                let line_end = self.line_end_char_index(self.cursor_line);
                self.move_cursor(line_end, true)
            }
            EditorAction::SelectDocumentHome => self.move_cursor(0, true),
            EditorAction::SelectDocumentEnd => {
                let text_len = self.text_len();
                self.move_cursor(text_len, true)
            }
            EditorAction::SelectAll => {
                let text_len = self.text_len();
                if text_len == 0 {
                    return false;
                }

                let changed = self.selection_range() != Some((0, text_len)) || self.cursor != text_len;
                self.selection_anchor = Some(0);
                self.cursor = text_len;
                self.sync_cursor_position();
                changed
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
        self.insert_text_internal(text)
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
        let (selection_start, selection_end) = if let Some((start, end)) = self.selection_range() {
            (Some(start), Some(end))
        } else {
            (None, None)
        };

        TextEditorDebugState {
            cursor: self.cursor,
            cursor_line: self.cursor_line,
            cursor_column: self.cursor_column,
            selection_anchor: self.selection_anchor,
            selection_start,
            selection_end,
            text_len_chars: self.text_len_chars,
            line_count: self.line_starts.len(),
            piece_table: self.text_table.debug_state(),
        }
    }

    pub fn selection_range(&self) -> Option<(usize, usize)> {
        let anchor = self.selection_anchor?;
        if anchor == self.cursor {
            return None;
        }

        if anchor < self.cursor {
            Some((anchor, self.cursor))
        } else {
            Some((self.cursor, anchor))
        }
    }

    pub fn selected_text(&self) -> Option<String> {
        let (start, end) = self.selection_range()?;
        Some(self.text_table.extract_range(start, end - start))
    }

    pub fn selection_segments(&self) -> Vec<SelectionSegment> {
        let Some((selection_start, selection_end)) = self.selection_range() else {
            return Vec::new();
        };

        let (start_line, _) = self.line_and_column_for_cursor(selection_start);
        let (end_line, _) = self.line_and_column_for_cursor(selection_end);

        let mut segments = Vec::new();
        for line in start_line..=end_line {
            let line_start = self.line_starts[line];
            let line_end = self.line_end_char_index(line);

            let segment_start = if line == start_line {
                selection_start
            } else {
                line_start
            };

            let segment_end = if line == end_line {
                selection_end
            } else {
                line_end
            };

            if segment_end <= segment_start {
                continue;
            }

            segments.push(SelectionSegment {
                line,
                start_column: segment_start - line_start,
                end_column: segment_end - line_start,
            });
        }

        segments
    }

    fn text_len(&self) -> usize {
        self.text_len_chars
    }

    fn refresh_after_text_change(&mut self) {
        self.selection_anchor = None;
        self.rebuild_text_cache();
        self.sync_cursor_position();
    }

    fn insert_text_internal(&mut self, text: String) -> bool {
        if text.is_empty() {
            return false;
        }

        let replaced_selection = self.delete_selection();
        let text_len = text.chars().count();
        self.text_table.insert(self.cursor, text);
        self.cursor += text_len;
        self.refresh_after_text_change();
        replaced_selection || text_len > 0
    }

    fn delete_selection(&mut self) -> bool {
        let Some((selection_start, selection_end)) = self.selection_range() else {
            return false;
        };

        self.text_table.delete(selection_start, selection_end - selection_start);
        self.cursor = selection_start;
        self.refresh_after_text_change();
        true
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

        let (line_index, column) = self.line_and_column_for_cursor(self.cursor);
        self.cursor_line = line_index;
        self.cursor_column = column;
    }

    fn line_and_column_for_cursor(&self, cursor: usize) -> (usize, usize) {
        let cursor = cursor.min(self.text_len_chars);

        let line_index = match self.line_starts.binary_search(&cursor) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };

        let line_start = self.line_starts[line_index];
        let line_len = self.line_length(line_index);
        let column = (cursor - line_start).min(line_len);

        (line_index, column)
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

    fn clear_selection_and_set_cursor(&mut self, cursor: usize) -> bool {
        let mut changed = false;
        if self.selection_anchor.take().is_some() {
            changed = true;
        }

        changed || self.move_cursor(cursor, false)
    }

    fn move_cursor(&mut self, cursor: usize, selecting: bool) -> bool {
        let clamped = cursor.min(self.text_len_chars);
        let old_cursor = self.cursor;

        if !selecting {
            if clamped == old_cursor {
                return self.selection_anchor.take().is_some();
            }

            self.selection_anchor = None;
            self.cursor = clamped;
            self.sync_cursor_position();
            return true;
        }

        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(old_cursor);
        }

        if old_cursor == clamped {
            return false;
        }

        self.cursor = clamped;
        self.sync_cursor_position();
        true
    }

    fn move_cursor_up(&mut self, selecting: bool) -> bool {
        if self.cursor_line == 0 {
            return false;
        }

        let target_line = self.cursor_line - 1;
        let target_col = self.cursor_column.min(self.line_length(target_line));
        let target_cursor = self.line_starts[target_line] + target_col;
        self.move_cursor(target_cursor, selecting)
    }

    fn move_cursor_down(&mut self, selecting: bool) -> bool {
        let last_line = self.line_starts.len() - 1;
        if self.cursor_line >= last_line {
            return false;
        }

        let target_line = self.cursor_line + 1;
        let target_col = self.cursor_column.min(self.line_length(target_line));
        let target_cursor = self.line_starts[target_line] + target_col;
        self.move_cursor(target_cursor, selecting)
    }
}

#[cfg(test)]
mod tests {
    use super::{EditorAction, SelectionSegment, TextEditor};

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
    fn home_end_moves_within_line() {
        let mut editor = TextEditor::new("ab\ncd".to_string());

        assert!(editor.apply_action(EditorAction::MoveDocumentHome));
        assert!(editor.apply_action(EditorAction::MoveRight));
        assert!(editor.apply_action(EditorAction::MoveRight));
        assert!(editor.apply_action(EditorAction::MoveRight));
        assert_eq!(editor.cursor(), 3);
        assert_eq!(editor.cursor_line(), 1);
        assert_eq!(editor.cursor_column(), 0);

        assert!(editor.apply_action(EditorAction::MoveEnd));
        assert_eq!(editor.cursor(), 5);
        assert_eq!(editor.cursor_line(), 1);
        assert_eq!(editor.cursor_column(), 2);

        assert!(editor.apply_action(EditorAction::MoveHome));
        assert_eq!(editor.cursor(), 3);
        assert_eq!(editor.cursor_line(), 1);
        assert_eq!(editor.cursor_column(), 0);
    }

    #[test]
    fn document_home_end_and_horizontal_moves() {
        let mut editor = TextEditor::new("hello".to_string());

        assert!(editor.apply_action(EditorAction::MoveDocumentHome));
        assert_eq!(editor.cursor(), 0);

        assert!(!editor.apply_action(EditorAction::MoveLeft));
        assert!(editor.apply_action(EditorAction::MoveRight));
        assert_eq!(editor.cursor(), 1);

        assert!(editor.apply_action(EditorAction::MoveDocumentEnd));
        assert_eq!(editor.cursor(), 5);
        assert!(!editor.apply_action(EditorAction::MoveRight));
    }

    #[test]
    fn move_up_down_preserves_column() {
        let mut editor = TextEditor::new("12345\n12\n1234".to_string());

        assert!(editor.apply_action(EditorAction::MoveDocumentHome));
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

        assert!(editor.apply_action(EditorAction::MoveDocumentHome));
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
        assert!(editor.apply_action(EditorAction::MoveDocumentHome));
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

    #[test]
    fn shift_moves_create_and_extend_selection() {
        let mut editor = TextEditor::new("hello".to_string());
        assert!(editor.apply_action(EditorAction::MoveDocumentHome));

        assert!(editor.apply_action(EditorAction::SelectRight));
        assert!(editor.apply_action(EditorAction::SelectRight));
        assert_eq!(editor.selection_range(), Some((0, 2)));
        assert_eq!(editor.selected_text().as_deref(), Some("he"));

        assert!(editor.apply_action(EditorAction::MoveRight));
        assert_eq!(editor.cursor(), 2);
        assert_eq!(editor.selection_range(), None);
    }

    #[test]
    fn typing_replaces_selection() {
        let mut editor = TextEditor::new("hello".to_string());
        assert!(editor.apply_action(EditorAction::MoveDocumentHome));
        assert!(editor.apply_action(EditorAction::SelectRight));
        assert!(editor.apply_action(EditorAction::SelectRight));

        assert!(editor.apply_action(EditorAction::InsertChar('X')));
        assert_eq!(editor.text(), "Xllo");
        assert_eq!(editor.cursor(), 1);
        assert_eq!(editor.selection_range(), None);
    }

    #[test]
    fn backspace_and_delete_remove_selection() {
        let mut editor = TextEditor::new("abcde".to_string());
        assert!(editor.apply_action(EditorAction::MoveDocumentHome));
        assert!(editor.apply_action(EditorAction::SelectRight));
        assert!(editor.apply_action(EditorAction::SelectRight));
        assert!(editor.apply_action(EditorAction::SelectRight));

        assert!(editor.apply_action(EditorAction::Backspace));
        assert_eq!(editor.text(), "de");
        assert_eq!(editor.cursor(), 0);
        assert_eq!(editor.selection_range(), None);

        assert!(editor.apply_action(EditorAction::SelectRight));
        assert!(editor.apply_action(EditorAction::Delete));
        assert_eq!(editor.text(), "e");
    }

    #[test]
    fn select_all_selects_full_document() {
        let mut editor = TextEditor::new("abc".to_string());
        assert!(editor.apply_action(EditorAction::SelectAll));
        assert_eq!(editor.selection_range(), Some((0, 3)));
        assert_eq!(editor.selected_text().as_deref(), Some("abc"));
    }

    #[test]
    fn selection_segments_cover_multiline_ranges() {
        let mut editor = TextEditor::new("ab\ncd\nef".to_string());
        assert!(editor.apply_action(EditorAction::MoveDocumentHome));
        assert!(editor.apply_action(EditorAction::SelectRight));
        assert!(editor.apply_action(EditorAction::SelectRight));
        assert!(editor.apply_action(EditorAction::SelectRight));
        assert!(editor.apply_action(EditorAction::SelectRight));
        assert!(editor.apply_action(EditorAction::SelectRight));

        assert_eq!(
            editor.selection_segments(),
            vec![
                SelectionSegment {
                    line: 0,
                    start_column: 0,
                    end_column: 2,
                },
                SelectionSegment {
                    line: 1,
                    start_column: 0,
                    end_column: 2,
                },
            ]
        );
    }
}
