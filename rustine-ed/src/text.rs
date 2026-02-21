/*#![allow(dead_code)]

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

struct Piece {
    text: String,
    char_position: usize,
    char_offset: usize,
    char_length: usize,
    byte_offset: usize,
    byte_length: usize,
    next: Option<usize>,
}

fn byte_offsets_for_char_indices(
    text: &str,
    first_char_index: usize,
    second_char_index: usize,
) -> (usize, usize) {
    assert!(first_char_index <= second_char_index);

    let mut first_byte = None;
    let mut second_byte = None;

    for (char_index, (byte_index, _)) in text.char_indices().enumerate() {
        if first_byte.is_none() && char_index == first_char_index {
            first_byte = Some(byte_index);
        }

        if second_byte.is_none() && char_index == second_char_index {
            second_byte = Some(byte_index);
            break;
        }
    }

    let text_len = text.len();
    (
        first_byte.unwrap_or(text_len),
        second_byte.unwrap_or(text_len),
    )
}

impl Piece {
    pub fn new(text: String, char_position: usize, char_offset: usize, char_length: usize) -> Self {
        let (byte_offset, byte_end) =
            byte_offsets_for_char_indices(&text, char_offset, char_offset + char_length);
        let byte_length = byte_end - byte_offset;

        Self {
            text,
            char_position,
            char_offset,
            char_length,
            byte_offset,
            byte_length,
            next: None,
        }
    }

    /// Return the length of the text in `char`s.
    pub fn len(&self) -> usize {
        self.text.chars().count()
    }

    /// Return the text as a `&str`, respecting the `char_offset` and `char_length`.
    pub fn as_str(&self) -> &str {
        &self.text[self.byte_offset..self.byte_offset + self.byte_length]
    }
}

pub(crate) struct TextEditor {
    base_text: String,
    pieces: Vec<Piece>,
}

impl TextEditor {
    pub(crate) fn new(initial: String) -> Self {
        let chars_len = initial.chars().count();
        Self {
            base_text: initial,
            pieces: vec![Piece::new(initial, 0, 0, chars_len)],
        }
    }

    pub fn get_text(&self, target: &mut String) {
        for piece in &self.pieces {
            target.push_str(piece.as_str());
        }
    }

    pub fn insert(&mut self, position: usize, input: &str) {
        let char_len = input.chars().count();

        if position == 0 {
            // Insert at the beginning of the text
            self.pieces
                .insert(0, Piece::new(input.to_string(), 0, 0, char_len));
            return;
        }

        for piece in &mut self.pieces {
            if position <= piece.char_position + piece.char_length {
                // Insert the input into the piece's text at the correct byte offset
                let insert_char_offset = position - piece.char_position;
                let (byte_offset, _) = byte_offsets_for_char_indices(
                    &piece.text,
                    insert_char_offset,
                    insert_char_offset,
                );
                piece.text.insert_str(byte_offset, input);
                piece.char_length += char_len;
                piece.byte_length += input.len();
                return;
            }
        }

        panic!("Insertion position should be within the text");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASIC_TEXT: &str = "Hello, World!";
    const MULTI_LINE_TEXT: &str =
        "Hello, World!\nThis is a multi-line text.\nIt has several lines.";
    const UNICODE_TEXT: &str = "Hello, 世界! Emoji: ";

    #[test]
    fn construct_editor() {
        let editor = TextEditor::new(BASIC_TEXT.to_string());

        assert_eq!(editor.pieces.first().unwrap().text, BASIC_TEXT);
    }

    #[test]
    fn piece_char_iteration() {
        let editor = TextEditor::new(UNICODE_TEXT.to_string());

        let mut chars = Vec::new();
        for piece in &editor.pieces {
            for c in piece.as_str().chars() {
                chars.push(c);
            }
        }

        assert_eq!(chars.iter().collect::<String>(), UNICODE_TEXT);
    }

    #[test]
    fn piece_char_offset() {
        let unicode = "Hello, 世界!";
        let char_offset = 7; // Start at the '世' character
        let char_length = 2; // Include '世' and '界'

        let piece = Piece::new(unicode.to_string(), 0, char_offset, char_length);
        assert_eq!(piece.as_str(), "世界");

        let char_offset = 0; // Start at the beginning
        let char_length = 5; // Include "Hello"
        let piece = Piece::new(unicode.to_string(), 0, char_offset, char_length);
        assert_eq!(piece.as_str(), "Hello");
    }

    #[test]
    fn editor_insert_beginning() {
        let basic_text = "Hello, World!";

        let mut editor = TextEditor::new(basic_text.to_string());
        editor.insert(0, "Greetings! ");

        let mut result = String::new();
        editor.get_text(&mut result);
        assert_eq!(result, "Greetings! Hello, World!");

        assert_eq!(editor.pieces.len(), 1);
    }

    #[test]
    fn editor_insert_middle() {
        let basic_text = "Hello, World!";

        let mut editor = TextEditor::new(basic_text.to_string());
        editor.insert(7, "Beautiful ");

        let mut result = String::new();
        editor.get_text(&mut result);
        assert_eq!(result, "Hello, Beautiful World!");
    }

    #[test]
    fn editor_insert_end() {
        let basic_text = "Hello, World!";

        let mut editor = TextEditor::new(basic_text.to_string());
        editor.insert(basic_text.chars().count(), " Goodbye!");

        let mut result = String::new();
        editor.get_text(&mut result);
        assert_eq!(result, "Hello, World! Goodbye!");
    }

    #[test]
    fn editor_insert_unicode() {
        let unicode_text = "Hello, 世界!";

        let mut editor = TextEditor::new(unicode_text.to_string());
        editor.insert(7, "Beautiful ");

        let mut result = String::new();
        editor.get_text(&mut result);

        assert_eq!(result, "Hello, Beautiful 世界!");
    }
}*/
