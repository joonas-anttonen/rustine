use rustine::gfx::{Rectangle, RenderFrame};

/// Shared list selection state.
#[derive(Debug, Clone, Default)]
pub struct ListState {
    pub selected: Option<usize>,
}

impl ListState {
    pub fn new() -> Self {
        Self { selected: None }
    }

    pub fn with_selected(selected: Option<usize>) -> Self {
        Self { selected }
    }

    /// Move selection to the previous item, wrapping to the end.
    pub fn select_prev(&mut self, len: usize) {
        if len == 0 {
            self.selected = None;
            return;
        }

        self.selected = Some(match self.selected {
            Some(idx) if idx > 0 => idx - 1,
            Some(_) | None => len - 1,
        });
    }

    /// Move selection to the next item, wrapping to the start.
    pub fn select_next(&mut self, len: usize) {
        if len == 0 {
            self.selected = None;
            return;
        }

        self.selected = Some(match self.selected {
            Some(idx) if idx + 1 < len => idx + 1,
            Some(_) | None => 0,
        });
    }

    /// Ensure the selection stays within bounds.
    pub fn clamp(&mut self, len: usize) {
        if len == 0 {
            self.selected = None;
            return;
        }

        if let Some(idx) = self.selected {
            if idx >= len {
                self.selected = Some(len - 1);
            }
        }
    }
}

/// Render a vertical list with shared selection/highlight handling.
///
/// The caller is responsible for drawing item contents inside `render_item`.
pub fn render_list<F>(
    frame: &mut RenderFrame,
    content_x: f32,
    content_w: f32,
    start_y: f32,
    line_height: f32,
    highlight_color: u32,
    selected: Option<usize>,
    count: usize,
    mut render_item: F,
) where
    F: FnMut(&mut RenderFrame, usize, f32, bool),
{
    for idx in 0..count {
        let y = start_y + idx as f32 * line_height;
        let is_selected = selected == Some(idx);

        if is_selected {
            frame.fill_rectangle(
                &Rectangle {
                    x: content_x,
                    y,
                    w: content_w,
                    h: line_height,
                },
                highlight_color,
            );
        }

        render_item(frame, idx, y, is_selected);
    }
}
