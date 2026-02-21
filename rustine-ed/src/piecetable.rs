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

struct Operation {
    kind: OperationKind,
    position: usize,
    text: String,
}

struct PieceTable {
    original: String,
    pieces: Vec<Piece>,
    undo_stack: Vec<Operation>,
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
        }
    }

    pub fn get_text(&self) -> String {
        let mut result = String::new();
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
                    (BufferType::Add(last_text), BufferType::Add(next_text)) => {
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
                    if operation.position == last.position + last_len {
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
            (BufferType::Add(_), BufferType::Add(_)) => true,
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
        let len = text.chars().count();
        if len == 0 {
            return;
        }

        let current_len = self.pieces.iter().map(|piece| piece.length).sum::<usize>();
        assert!(
            position <= current_len,
            "insert position {position} out of bounds for length {current_len}"
        );

        let existing_at_position: String = self.get_text().chars().skip(position).take(len).collect();
        if existing_at_position == text {
            return;
        }

        if record_undo {
            let operation = Operation {
                kind: OperationKind::Insert,
                position,
                text: text.clone(),
            };
            self.push_undo_operation(operation);
        }

        let new_piece = Piece {
            buffer_type: BufferType::Add(text),
            start: 0,
            length: len,
        };

        if self.pieces.is_empty() {
            self.pieces.push(new_piece);
            return;
        }

        let mut cursor = 0usize;
        for index in 0..self.pieces.len() {
            let piece = &self.pieces[index];
            let piece_start = cursor;
            let piece_end = piece_start + piece.length;

            if position < piece_end {
                if position == piece_start {
                    self.pieces.insert(index, new_piece);
                    self.coalesce_around(index);
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

                self.pieces.insert(index + 1, new_piece);
                self.pieces.insert(index + 2, right_piece);
                self.coalesce_around(index + 1);
                return;
            }

            if position == piece_end {
                self.pieces.insert(index + 1, new_piece);
                self.coalesce_around(index + 1);
                return;
            }

            cursor = piece_end;
        }

        self.pieces.push(new_piece);
        self.coalesce_around(self.pieces.len() - 1);
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
            let operation = Operation {
                kind: OperationKind::Delete,
                position,
                text: self
                    .get_text()
                    .chars()
                    .skip(position)
                    .take(length)
                    .collect(),
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

    pub fn undo(&mut self) {
        if let Some(operation) = self.undo_stack.pop() {
            match operation.kind {
                OperationKind::Insert => {
                    let length = operation.text.chars().count();
                    self.delete_internal(operation.position, length, false);
                }
                OperationKind::Delete => {
                    self.insert_internal(operation.position, operation.text, false);
                }
            }
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
        assert_eq!(piece_table.get_text(), expected);
    }

    fn assert_undo_to_original(piece_table: &mut PieceTable) {
        piece_table.undo();
        assert_text(piece_table, ORIGINAL_TEXT);
    }

    #[test]
    fn contiguous_insert_merges() {
        let mut piece_table = make_piece_table();
        piece_table.insert(0, "Say: ".to_string());
        piece_table.insert(5, "Hello, ".to_string());

        assert_original_stable(&piece_table);
        assert_eq!(piece_table.pieces.len(), 2);

        assert_text(&piece_table, "Say: Hello, World!");
        assert_undo_to_original(&mut piece_table);
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
}