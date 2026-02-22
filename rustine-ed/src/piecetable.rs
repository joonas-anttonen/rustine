#![allow(dead_code)]

#[derive(Clone)]
enum BufferType {
    Original,
    Add(String),
}

struct Piece {
    buffer_type: BufferType,
    start: usize,
    length: usize,
}

enum OperationKind {
    Insert,
    Delete,
}

#[derive(Debug, Clone)]
pub(crate) struct PieceDebugEntry {
    pub buffer: &'static str,
    pub start: usize,
    pub length: usize,
    pub preview: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PieceTableDebugState {
    pub document_len: usize,
    pub piece_count: usize,
    pub undo_depth: usize,
    pub redo_depth: usize,
    pub pieces: Vec<PieceDebugEntry>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InsertClass {
    Word,
    Whitespace,
    Punctuation,
    Newline,
}

struct Operation {
    kind: OperationKind,
    position: usize,
    text: String,
}

pub(crate) struct PieceTable {
    original: String,
    pieces: Vec<Piece>,
    undo_stack: Vec<Operation>,
    redo_stack: Vec<Operation>,
}

impl PieceTable {
    pub fn new(original: String) -> Self {
        let pieces = vec![Piece {
            buffer_type: BufferType::Original,
            start: 0,
            length: original.len(),
        }];
        Self {
            original,
            pieces,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn get_text(&self, result: &mut String) {
        for piece in &self.pieces {
            match &piece.buffer_type {
                BufferType::Original => {
                    let slice = &self.original[piece.start..piece.start + piece.length];
                    result.push_str(slice);
                }
                BufferType::Add(text) => {
                    result.extend(text.chars().skip(piece.start).take(piece.length));
                }
            }
        }
    }

    pub fn debug_state(&self) -> PieceTableDebugState {
        let mut pieces = Vec::with_capacity(self.pieces.len());

        for piece in &self.pieces {
            match &piece.buffer_type {
                BufferType::Original => {
                    let segment = &self.original[piece.start..piece.start + piece.length];
                    pieces.push(PieceDebugEntry {
                        buffer: "orig",
                        start: piece.start,
                        length: piece.length,
                        preview: Self::debug_preview(segment, 24),
                    });
                }
                BufferType::Add(text) => {
                    let segment: String = text.chars().skip(piece.start).take(piece.length).collect();
                    pieces.push(PieceDebugEntry {
                        buffer: "add",
                        start: piece.start,
                        length: piece.length,
                        preview: Self::debug_preview(&segment, 24),
                    });
                }
            }
        }

        let document_len = self.pieces.iter().map(|piece| piece.length).sum();

        PieceTableDebugState {
            document_len,
            piece_count: self.pieces.len(),
            undo_depth: self.undo_stack.len(),
            redo_depth: self.redo_stack.len(),
            pieces,
        }
    }

    fn debug_preview(text: &str, max_chars: usize) -> String {
        let mut preview = String::new();
        let mut count = 0usize;

        for c in text.chars() {
            if count >= max_chars {
                preview.push('…');
                break;
            }

            match c {
                '\n' => preview.push_str("\\n"),
                '\r' => preview.push_str("\\r"),
                '\t' => preview.push_str("\\t"),
                _ => preview.push(c),
            }
            count += 1;
        }

        preview
    }

    pub fn extract_range(&self, position: usize, length: usize) -> String {
        let mut result = String::with_capacity(length);
        let mut cursor = 0usize;
        let end_position = position
            .checked_add(length)
            .expect("extract range end overflows usize");

        for piece in &self.pieces {
            let piece_start = cursor;
            let piece_end = piece_start + piece.length;

            if piece_end <= position || piece_start >= end_position {
                // This piece is completely outside the target range, skip it
            } else {
                // This piece overlaps with the target range, extract the relevant portion
                let extract_start = position.max(piece_start);
                let extract_end = end_position.min(piece_end);
                let extract_offset = extract_start - piece_start;
                let extract_length = extract_end - extract_start;

                match &piece.buffer_type {
                    BufferType::Original => {
                        let slice = &self.original[piece.start + extract_offset
                            ..piece.start + extract_offset + extract_length];
                        result.push_str(slice);
                    }
                    BufferType::Add(text) => {
                        result.extend(
                            text.chars()
                                .skip(piece.start + extract_offset)
                                .take(extract_length),
                        );
                    }
                }
            }

            cursor = piece_end;
        }

        result
    }

    pub fn insert(&mut self, position: usize, text: String) {
        self.insert_internal(position, text, true);
    }

    fn coalesce_pieces(&mut self) {
        let mut merged: Vec<Piece> = Vec::with_capacity(self.pieces.len());

        for piece in self.pieces.drain(..) {
            if piece.length == 0 {
                continue;
            }

            if let Some(last) = merged.last_mut() {
                match (&mut last.buffer_type, &piece.buffer_type) {
                    (BufferType::Original, BufferType::Original)
                        if last.start + last.length == piece.start =>
                    {
                        last.length += piece.length;
                        continue;
                    }
                    (BufferType::Add(last_text), BufferType::Add(next_text))
                        if Self::can_merge_add_piece_boundary(
                            last_text,
                            last.start,
                            last.length,
                            next_text,
                            piece.start,
                            piece.length,
                        ) =>
                    {
                        let mut combined = String::new();
                        combined.extend(last_text.chars().skip(last.start).take(last.length));
                        combined.extend(next_text.chars().skip(piece.start).take(piece.length));
                        *last_text = combined;
                        last.start = 0;
                        last.length = last_text.chars().count();
                        continue;
                    }
                    _ => {}
                }
            }

            merged.push(piece);
        }

        if merged.is_empty() {
            merged.push(Piece {
                buffer_type: BufferType::Original,
                start: 0,
                length: 0,
            });
        }

        self.pieces = merged;
    }

    fn push_undo_operation(&mut self, operation: Operation) {
        if let Some(last) = self.undo_stack.last_mut() {
            match (&last.kind, &operation.kind) {
                (OperationKind::Insert, OperationKind::Insert) => {
                    let last_len = last.text.chars().count();
                    if operation.position == last.position + last_len
                        && Self::should_merge_insert_operations(&last.text, &operation.text)
                    {
                        last.text.push_str(&operation.text);
                        return;
                    }
                }
                (OperationKind::Delete, OperationKind::Delete) => {
                    let op_len = operation.text.chars().count();

                    if operation.position == last.position {
                        last.text.push_str(&operation.text);
                        return;
                    }

                    if operation.position + op_len == last.position {
                        let mut merged_text = operation.text;
                        merged_text.push_str(&last.text);
                        last.position = operation.position;
                        last.text = merged_text;
                        return;
                    }
                }
                _ => {}
            }
        }

        self.undo_stack.push(operation);
    }

    fn classify_insert_char(c: char) -> InsertClass {
        if c == '\n' {
            InsertClass::Newline
        } else if c.is_alphanumeric() || c == '_' {
            InsertClass::Word
        } else if c.is_whitespace() {
            InsertClass::Whitespace
        } else {
            InsertClass::Punctuation
        }
    }

    fn classify_insert_text(text: &str) -> Option<InsertClass> {
        let mut chars = text.chars();
        let first = chars.next()?;
        let class = Self::classify_insert_char(first);
        if class == InsertClass::Newline {
            return Some(InsertClass::Newline);
        }

        for c in chars {
            if Self::classify_insert_char(c) != class {
                return None;
            }
        }

        Some(class)
    }

    fn should_merge_insert_operations(previous: &str, next: &str) -> bool {
        let Some(previous_class) = Self::classify_insert_text(previous) else {
            return false;
        };
        let Some(next_class) = Self::classify_insert_text(next) else {
            return false;
        };

        if previous_class == InsertClass::Newline || next_class == InsertClass::Newline {
            return false;
        }

        previous_class == next_class
    }

    fn can_merge_add_piece_boundary(
        left_text: &str,
        left_start: usize,
        left_length: usize,
        right_text: &str,
        right_start: usize,
        right_length: usize,
    ) -> bool {
        let left_ends_with_newline = left_text
            .chars()
            .skip(left_start)
            .take(left_length)
            .last()
            .is_some_and(|c| c == '\n');

        if left_ends_with_newline {
            return false;
        }

        let right_starts_with_newline = right_text
            .chars()
            .skip(right_start)
            .take(right_length)
            .next()
            .is_some_and(|c| c == '\n');

        !right_starts_with_newline
    }

    fn split_insert_into_pieces(text: String) -> Vec<Piece> {
        let mut pieces = Vec::new();
        let mut current = String::new();
        let mut current_len = 0usize;

        for c in text.chars() {
            current.push(c);
            current_len += 1;

            if c == '\n' {
                pieces.push(Piece {
                    buffer_type: BufferType::Add(std::mem::take(&mut current)),
                    start: 0,
                    length: current_len,
                });
                current_len = 0;
            }
        }

        if current_len > 0 {
            pieces.push(Piece {
                buffer_type: BufferType::Add(current),
                start: 0,
                length: current_len,
            });
        }

        pieces
    }

    fn merge_adjacent_pieces(&mut self, left_index: usize) -> bool {
        let right_index = left_index + 1;
        if right_index >= self.pieces.len() {
            return false;
        }

        let should_merge = match (
            &self.pieces[left_index].buffer_type,
            &self.pieces[right_index].buffer_type,
        ) {
            (BufferType::Original, BufferType::Original) => {
                self.pieces[left_index].start + self.pieces[left_index].length
                    == self.pieces[right_index].start
            }
            (BufferType::Add(left_text), BufferType::Add(right_text)) => {
                Self::can_merge_add_piece_boundary(
                    left_text,
                    self.pieces[left_index].start,
                    self.pieces[left_index].length,
                    right_text,
                    self.pieces[right_index].start,
                    self.pieces[right_index].length,
                )
            }
            _ => false,
        };

        if !should_merge {
            return false;
        }

        let right_piece = self.pieces.remove(right_index);
        let left_piece = &mut self.pieces[left_index];

        match (&mut left_piece.buffer_type, right_piece.buffer_type) {
            (BufferType::Original, BufferType::Original) => {
                left_piece.length += right_piece.length;
            }
            (BufferType::Add(left_text), BufferType::Add(right_text)) => {
                let mut combined = String::new();
                combined.extend(
                    left_text
                        .chars()
                        .skip(left_piece.start)
                        .take(left_piece.length),
                );
                combined.extend(
                    right_text
                        .chars()
                        .skip(right_piece.start)
                        .take(right_piece.length),
                );
                *left_text = combined;
                left_piece.start = 0;
                left_piece.length = left_text.chars().count();
            }
            _ => unreachable!("merge precondition violated"),
        }

        true
    }

    fn coalesce_around(&mut self, mut index: usize) {
        if index >= self.pieces.len() {
            return;
        }

        if index > 0 && self.merge_adjacent_pieces(index - 1) {
            index -= 1;
        }

        self.merge_adjacent_pieces(index);
    }

    fn insert_internal(&mut self, position: usize, text: String, record_undo: bool) {
        if text.is_empty() {
            return;
        }

        if record_undo {
            self.redo_stack.clear();
            let operation = Operation {
                kind: OperationKind::Insert,
                position,
                text: text.clone(),
            };
            self.push_undo_operation(operation);
        }

        let new_pieces = Self::split_insert_into_pieces(text);
        if new_pieces.is_empty() {
            return;
        }

        if self.pieces.is_empty() {
            self.pieces = new_pieces;
            return;
        }

        let mut cursor = 0usize;
        for index in 0..self.pieces.len() {
            let piece = &self.pieces[index];
            let piece_start = cursor;
            let piece_end = piece_start + piece.length;

            if position < piece_end {
                if position == piece_start {
                    self.pieces.splice(index..index, new_pieces);
                    self.coalesce_pieces();
                    return;
                }

                let left_len = position - piece_start;
                let right_len = piece_end - position;
                let split_buffer_type = self.pieces[index].buffer_type.clone();

                self.pieces[index].length = left_len;

                let right_piece = Piece {
                    buffer_type: split_buffer_type,
                    start: self.pieces[index].start + left_len,
                    length: right_len,
                };

                let inserted_count = new_pieces.len();
                self.pieces.splice(index + 1..index + 1, new_pieces);
                self.pieces.insert(index + 1 + inserted_count, right_piece);
                self.coalesce_pieces();
                return;
            }

            if position == piece_end {
                self.pieces.splice(index + 1..index + 1, new_pieces);
                self.coalesce_pieces();
                return;
            }

            cursor = piece_end;
        }

        self.pieces.extend(new_pieces);
        self.coalesce_pieces();
    }

    pub fn delete(&mut self, position: usize, length: usize) {
        self.delete_internal(position, length, true);
    }

    fn delete_internal(&mut self, position: usize, length: usize, record_undo: bool) {
        let current_len = self.pieces.iter().map(|piece| piece.length).sum::<usize>();
        let delete_end = position
            .checked_add(length)
            .expect("delete range end overflows usize");
        assert!(
            delete_end <= current_len,
            "delete range {position}..{} out of bounds for length {current_len}",
            delete_end
        );
        if length == 0 {
            return;
        }

        if record_undo {
            self.redo_stack.clear();
            let operation = Operation {
                kind: OperationKind::Delete,
                position,
                text: self.extract_range(position, length),
            };
            self.push_undo_operation(operation);
        }

        let delete_start = position;
        let mut cursor = 0usize;
        let mut new_pieces = Vec::with_capacity(self.pieces.len());

        for piece in &self.pieces {
            let piece_start = cursor;
            let piece_end = piece_start + piece.length;

            if piece_end <= delete_start || piece_start >= delete_end {
                new_pieces.push(Piece {
                    buffer_type: piece.buffer_type.clone(),
                    start: piece.start,
                    length: piece.length,
                });
            } else {
                if piece_start < delete_start {
                    let left_len = delete_start - piece_start;
                    new_pieces.push(Piece {
                        buffer_type: piece.buffer_type.clone(),
                        start: piece.start,
                        length: left_len,
                    });
                }

                if piece_end > delete_end {
                    let removed_prefix = delete_end - piece_start;
                    let right_len = piece_end - delete_end;
                    new_pieces.push(Piece {
                        buffer_type: piece.buffer_type.clone(),
                        start: piece.start + removed_prefix,
                        length: right_len,
                    });
                }
            }

            cursor = piece_end;
        }

        self.pieces = new_pieces;
        self.coalesce_pieces();
    }

    pub fn undo(&mut self) -> bool {
        if let Some(operation) = self.undo_stack.pop() {
            match operation.kind {
                OperationKind::Insert => {
                    let length = operation.text.chars().count();
                    self.delete_internal(operation.position, length, false);
                }
                OperationKind::Delete => {
                    self.insert_internal(operation.position, operation.text.clone(), false);
                }
            }

            self.redo_stack.push(operation);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(operation) = self.redo_stack.pop() {
            match operation.kind {
                OperationKind::Insert => {
                    self.insert_internal(operation.position, operation.text.clone(), false);
                }
                OperationKind::Delete => {
                    let length = operation.text.chars().count();
                    self.delete_internal(operation.position, length, false);
                }
            }

            self.undo_stack.push(operation);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGINAL_TEXT: &str = "Hello, World!";

    fn make_piece_table() -> PieceTable {
        PieceTable::new(ORIGINAL_TEXT.chars().collect())
    }

    fn assert_original_stable(piece_table: &PieceTable) {
        assert_eq!(piece_table.original, ORIGINAL_TEXT);
    }

    fn assert_text(piece_table: &PieceTable, expected: &str) {
        let mut result = String::new();
        piece_table.get_text(&mut result);
        assert_eq!(result, expected);
    }

    fn assert_newline_piece_boundaries(piece_table: &PieceTable) {
        for piece in &piece_table.pieces {
            if let BufferType::Add(text) = &piece.buffer_type {
                let segment: String = text.chars().skip(piece.start).take(piece.length).collect();
                let newline_count = segment.chars().filter(|&c| c == '\n').count();
                assert!(
                    newline_count <= 1,
                    "piece contains multiple newlines: {segment:?}"
                );
                if newline_count == 1 {
                    assert!(
                        segment.ends_with('\n'),
                        "piece with newline must end with newline: {segment:?}"
                    );
                }
            }
        }
    }

    fn assert_undo_to_original(piece_table: &mut PieceTable) {
        piece_table.undo();
        assert_text(piece_table, ORIGINAL_TEXT);
    }

    #[test]
    fn extract_range() {
        let piece_table = make_piece_table();
        let extracted = piece_table.extract_range(7, 5);
        assert_eq!(extracted, "World");
    }

    #[test]
    fn contiguous_insert_merges() {
        let mut piece_table = make_piece_table();

        piece_table.insert(13, "R".to_string());
        piece_table.insert(14, "u".to_string());
        piece_table.insert(15, "s".to_string());
        piece_table.insert(16, "t".to_string());
        assert_text(&piece_table, "Hello, World!Rust");

        assert_original_stable(&piece_table);
        assert_eq!(piece_table.pieces.len(), 2);
        assert_eq!(piece_table.undo_stack.len(), 1);

        piece_table.undo();
        assert_text(&piece_table, ORIGINAL_TEXT);
    }

    #[test]
    fn insert_coalescing_stops_at_space_boundary() {
        let mut piece_table = make_piece_table();

        piece_table.insert(13, "h".to_string());
        piece_table.insert(14, "i".to_string());
        piece_table.insert(15, " ".to_string());
        piece_table.insert(16, "t".to_string());
        piece_table.insert(17, "h".to_string());
        piece_table.insert(18, "e".to_string());
        piece_table.insert(19, "r".to_string());
        piece_table.insert(20, "e".to_string());

        assert_eq!(piece_table.undo_stack.len(), 3);
        assert_text(&piece_table, "Hello, World!hi there");

        piece_table.undo();
        assert_text(&piece_table, "Hello, World!hi ");

        piece_table.undo();
        assert_text(&piece_table, "Hello, World!hi");

        piece_table.undo();
        assert_text(&piece_table, ORIGINAL_TEXT);
    }

    #[test]
    fn insert_coalescing_stops_at_newline_boundary() {
        let mut piece_table = make_piece_table();

        piece_table.insert(13, "a".to_string());
        piece_table.insert(14, "b".to_string());
        piece_table.insert(15, "\n".to_string());
        piece_table.insert(16, "c".to_string());
        piece_table.insert(17, "d".to_string());

        assert_eq!(piece_table.undo_stack.len(), 3);
        assert_text(&piece_table, "Hello, World!ab\ncd");

        piece_table.undo();
        assert_text(&piece_table, "Hello, World!ab\n");

        piece_table.undo();
        assert_text(&piece_table, "Hello, World!ab");

        piece_table.undo();
        assert_text(&piece_table, ORIGINAL_TEXT);
    }

    #[test]
    fn contiguous_delete_merges() {
        let mut piece_table = make_piece_table();
        piece_table.delete(0, 7);
        piece_table.delete(0, 6);

        assert_original_stable(&piece_table);
        assert_eq!(piece_table.pieces.len(), 1);

        assert_text(&piece_table, "");
        assert_undo_to_original(&mut piece_table);
    }

    #[test]
    fn backward_contiguous_delete_merges_on_original() {
        let mut piece_table = make_piece_table();
        piece_table.delete(6, 1);
        piece_table.delete(5, 1);

        assert_original_stable(&piece_table);
        assert_eq!(piece_table.undo_stack.len(), 1);
        assert_text(&piece_table, "HelloWorld!");

        assert_undo_to_original(&mut piece_table);
    }

    #[test]
    fn backward_contiguous_delete_merges_on_inserted_segment() {
        let mut piece_table = make_piece_table();
        piece_table.insert(7, "Rustine ".to_string());
        piece_table.delete(14, 1);
        piece_table.delete(13, 1);

        assert_original_stable(&piece_table);
        assert_eq!(piece_table.undo_stack.len(), 2);
        assert_text(&piece_table, "Hello, RustinWorld!");

        piece_table.undo();
        assert_text(&piece_table, "Hello, Rustine World!");
    }

    #[test]
    fn non_contiguous_deletes_do_not_merge() {
        let mut piece_table = make_piece_table();
        piece_table.delete(0, 1);
        piece_table.delete(2, 1);

        assert_original_stable(&piece_table);
        assert_eq!(piece_table.undo_stack.len(), 2);
        assert_text(&piece_table, "elo, World!");

        piece_table.undo();
        assert_text(&piece_table, "ello, World!");

        piece_table.undo();
        assert_text(&piece_table, ORIGINAL_TEXT);
    }

    #[test]
    fn start_insert() {
        let mut piece_table = make_piece_table();
        piece_table.insert(0, "Say: ".to_string());

        assert_original_stable(&piece_table);
        assert_eq!(piece_table.pieces.len(), 2);

        assert_text(&piece_table, &format!("Say: {ORIGINAL_TEXT}"));
        assert_undo_to_original(&mut piece_table);
    }

    #[test]
    fn middle_insert() {
        let mut piece_table = make_piece_table();
        piece_table.insert(5, ", Rustine".to_string());

        assert_original_stable(&piece_table);
        assert_eq!(piece_table.pieces.len(), 3);

        assert_text(&piece_table, "Hello, Rustine, World!");
        assert_undo_to_original(&mut piece_table);
    }

    #[test]
    fn end_insert() {
        let mut piece_table = make_piece_table();
        piece_table.insert(13, " Goodbye.".to_string());

        assert_original_stable(&piece_table);
        assert_eq!(piece_table.pieces.len(), 2);

        assert_text(&piece_table, "Hello, World! Goodbye.");
        assert_undo_to_original(&mut piece_table);
    }

    #[test]
    fn start_delete() {
        let mut piece_table = make_piece_table();
        piece_table.delete(0, 7);

        assert_original_stable(&piece_table);

        assert_text(&piece_table, "World!");
        assert_undo_to_original(&mut piece_table);
    }

    #[test]
    fn middle_delete() {
        let mut piece_table = make_piece_table();
        piece_table.delete(5, 2);

        assert_original_stable(&piece_table);

        assert_text(&piece_table, "HelloWorld!");
        assert_undo_to_original(&mut piece_table);
    }

    #[test]
    fn end_delete() {
        let mut piece_table = make_piece_table();
        piece_table.delete(7, 6);

        assert_original_stable(&piece_table);

        assert_text(&piece_table, "Hello, ");
        assert_undo_to_original(&mut piece_table);
    }

    #[test]
    fn full_delete() {
        let mut piece_table = make_piece_table();
        piece_table.delete(0, ORIGINAL_TEXT.chars().count());

        assert_original_stable(&piece_table);

        assert_text(&piece_table, "");
        assert_undo_to_original(&mut piece_table);
    }

    #[test]
    fn delete_inserted_segment() {
        let mut piece_table = make_piece_table();
        piece_table.insert(7, "Rustine ".to_string());
        piece_table.delete(7, 8);

        assert_original_stable(&piece_table);

        assert_text(&piece_table, ORIGINAL_TEXT);

        piece_table.undo();
        assert_text(&piece_table, "Hello, Rustine World!");
    }

    #[test]
    fn delete_across_original_into_inserted() {
        let mut piece_table = make_piece_table();
        piece_table.insert(7, "Rustine ".to_string());
        piece_table.delete(5, 6);

        assert_original_stable(&piece_table);

        assert_text(&piece_table, "Helloine World!");

        piece_table.undo();
        assert_text(&piece_table, "Hello, Rustine World!");
    }

    #[test]
    fn repeated_undo_reverts_lifo_chain() {
        let mut piece_table = make_piece_table();

        piece_table.insert(0, "Say: ".to_string());
        assert_text(&piece_table, "Say: Hello, World!");

        piece_table.delete(5, 7);
        assert_text(&piece_table, "Say: World!");

        piece_table.insert(11, " Goodbye.".to_string());
        assert_text(&piece_table, "Say: World! Goodbye.");

        piece_table.undo();
        assert_text(&piece_table, "Say: World!");

        piece_table.undo();
        assert_text(&piece_table, "Say: Hello, World!");

        piece_table.undo();
        assert_text(&piece_table, ORIGINAL_TEXT);

        piece_table.undo();
        assert_text(&piece_table, ORIGINAL_TEXT);
    }

    #[test]
    fn redo_reapplies_undone_operations_in_order() {
        let mut piece_table = make_piece_table();

        piece_table.insert(0, "Say: ".to_string());
        piece_table.delete(5, 7);
        piece_table.insert(11, " Goodbye.".to_string());
        assert_text(&piece_table, "Say: World! Goodbye.");

        assert!(piece_table.undo());
        assert_text(&piece_table, "Say: World!");

        assert!(piece_table.undo());
        assert_text(&piece_table, "Say: Hello, World!");

        assert!(piece_table.redo());
        assert_text(&piece_table, "Say: World!");

        assert!(piece_table.redo());
        assert_text(&piece_table, "Say: World! Goodbye.");

        assert!(!piece_table.redo());
    }

    #[test]
    fn redo_stack_clears_after_new_edit() {
        let mut piece_table = make_piece_table();

        piece_table.insert(13, "!".to_string());
        assert!(piece_table.undo());
        assert_text(&piece_table, ORIGINAL_TEXT);

        piece_table.insert(0, "X".to_string());
        assert!(!piece_table.redo());
        assert_text(&piece_table, "XHello, World!");
    }

    #[test]
    fn unicode_insert_delete_and_undo() {
        let mut piece_table = make_piece_table();
        let unicode = " ääkköset 🚀";
        piece_table.insert(13, unicode.to_string());

        assert_original_stable(&piece_table);
        assert_text(&piece_table, "Hello, World! ääkköset 🚀");

        piece_table.delete(13, unicode.chars().count());
        assert_text(&piece_table, ORIGINAL_TEXT);

        piece_table.undo();
        assert_text(&piece_table, "Hello, World! ääkköset 🚀");
    }

    #[test]
    fn unicode_extract_range_from_inserted_text() {
        let mut piece_table = make_piece_table();
        piece_table.insert(7, "你好🌍 ".to_string());

        assert_original_stable(&piece_table);
        assert_text(&piece_table, "Hello, 你好🌍 World!");

        let extracted = piece_table.extract_range(7, 3);
        assert_eq!(extracted, "你好🌍");
    }

    #[test]
    fn multiline_insert_and_delete() {
        let mut piece_table = PieceTable::new("line1\nline2\nline3".to_string());
        piece_table.insert(5, "\ninserted".to_string());

        assert_text(&piece_table, "line1\ninserted\nline2\nline3");

        piece_table.delete(5, "\ninserted".chars().count());
        assert_text(&piece_table, "line1\nline2\nline3");

        piece_table.undo();
        assert_text(&piece_table, "line1\ninserted\nline2\nline3");
    }

    #[test]
    fn multiline_extract_range_spans_newline() {
        let piece_table = PieceTable::new("alpha\nbeta\ngamma".to_string());
        let extracted = piece_table.extract_range(3, 6);
        assert_eq!(extracted, "ha\nbet");
    }

    #[test]
    fn unicode_original_buffer_insert_delete_extract() {
        let original = "Hei ääkköset 🚀 maailma";
        let mut piece_table = PieceTable::new(original.to_string());
        let word = "ääkköset ";
        let word_start = original.find(word).unwrap();

        let extracted = piece_table.extract_range(word_start, word.len());
        assert_eq!(extracted, word);

        piece_table.delete(word_start, word.len());
        assert_text(&piece_table, "Hei 🚀 maailma");

        piece_table.undo();
        assert_text(&piece_table, original);

        piece_table.insert(word_start, "[X]".to_string());
        assert_text(&piece_table, "Hei [X]ääkköset 🚀 maailma");

        piece_table.undo();
        assert_text(&piece_table, original);
    }

    #[test]
    fn empty_original_multiline_insert_splits_on_newlines() {
        let mut piece_table = PieceTable::new(String::new());
        piece_table.insert(0, "alpha\nbeta\ngamma".to_string());

        assert_text(&piece_table, "alpha\nbeta\ngamma");
        assert_eq!(piece_table.pieces.len(), 3);

        piece_table.insert(6, "X".to_string());
        assert_text(&piece_table, "alpha\nXbeta\ngamma");

        piece_table.undo();
        assert_text(&piece_table, "alpha\nbeta\ngamma");
    }

    #[test]
    fn multiline_piece_count_stays_stable_across_line_edits() {
        let mut piece_table = PieceTable::new(String::new());
        piece_table.insert(0, "alpha\nbeta\ngamma".to_string());

        assert_eq!(piece_table.pieces.len(), 3);

        piece_table.insert(8, "X".to_string());
        assert_eq!(piece_table.pieces.len(), 3);

        piece_table.delete(1, 1);
        assert_eq!(piece_table.pieces.len(), 3);

        let mut current_text = String::new();
        piece_table.get_text(&mut current_text);
        let third_line_start = current_text
            .match_indices('\n')
            .nth(1)
            .expect("expected second newline")
            .0
            + 1;

        piece_table.insert(third_line_start + 2, "Y".to_string());
        assert_eq!(piece_table.pieces.len(), 3);

        piece_table.undo();
        assert_eq!(piece_table.pieces.len(), 3);

        piece_table.undo();
        assert_eq!(piece_table.pieces.len(), 3);

        piece_table.undo();
        assert_eq!(piece_table.pieces.len(), 3);
    }

    #[test]
    fn cross_line_boundary_edits_preserve_newline_piece_boundaries() {
        let mut piece_table = PieceTable::new(String::new());
        piece_table.insert(0, "alpha\nbeta\ngamma".to_string());
        assert_newline_piece_boundaries(&piece_table);

        piece_table.delete(5, 2);
        assert_text(&piece_table, "alphaeta\ngamma");
        assert_newline_piece_boundaries(&piece_table);

        piece_table.insert(5, "\nb".to_string());
        assert_text(&piece_table, "alpha\nbeta\ngamma");
        assert_newline_piece_boundaries(&piece_table);

        piece_table.delete(10, 1);
        assert_text(&piece_table, "alpha\nbetagamma");
        assert_newline_piece_boundaries(&piece_table);

        piece_table.insert(10, "\n".to_string());
        assert_text(&piece_table, "alpha\nbeta\ngamma");
        assert_newline_piece_boundaries(&piece_table);
    }

    #[test]
    fn unicode_cross_line_boundary_edits_preserve_newline_piece_boundaries() {
        let mut piece_table = PieceTable::new(String::new());
        piece_table.insert(0, "ää\n🚀\n你好".to_string());
        assert_newline_piece_boundaries(&piece_table);

        piece_table.delete(2, 2);
        assert_text(&piece_table, "ää\n你好");
        assert_newline_piece_boundaries(&piece_table);

        piece_table.insert(2, "\n🚀".to_string());
        assert_text(&piece_table, "ää\n🚀\n你好");
        assert_newline_piece_boundaries(&piece_table);

        piece_table.delete(4, 1);
        assert_text(&piece_table, "ää\n🚀你好");
        assert_newline_piece_boundaries(&piece_table);

        piece_table.insert(4, "\n".to_string());
        assert_text(&piece_table, "ää\n🚀\n你好");
        assert_newline_piece_boundaries(&piece_table);
    }
}
