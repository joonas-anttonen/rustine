use rustine::gfx::{Rectangle, RenderFrame};

/// Shared list selection state.
#[derive(Debug, Clone, Default)]
pub struct ListState {
    pub selected: Option<usize>,
    /// Starting Y offset for rendering. Adjusted based on selected item to keep it in view.
    pub scroll_offset: f32,
    pub count: usize,
}

impl ListState {
    pub fn new() -> Self {
        Self {
            selected: None,
            scroll_offset: 0.0,
            count: 0,
        }
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

    /// Update scroll offset to ensure the selected item is visible in the viewport.
    ///
    /// Given the content height and line height, adjusts `scroll_offset` so that
    /// the selected item is brought into view. The selected item will only trigger
    /// scrolling if it goes completely outside the viewport.
    pub fn update_scroll(&mut self, line_height: f32, content_height: f32) {
        let Some(selected_idx) = self.selected else {
            return;
        };

        let item_top = selected_idx as f32 * line_height;
        let item_bottom = item_top + line_height;

        let viewport_top = self.scroll_offset;
        let viewport_bottom = self.scroll_offset + content_height;

        // Only scroll if the item is completely outside the viewport
        // If item is completely above viewport, scroll to show it at the top
        if item_bottom <= viewport_top {
            self.scroll_offset = item_top.max(0.0);
        }
        // If item is completely below viewport, scroll to show it at the bottom
        else if item_top >= viewport_bottom {
            self.scroll_offset = (item_bottom - content_height).max(0.0);
        }
        // Otherwise item is at least partially visible, don't change scroll_offset
    }
}

/// Render a vertical list with shared selection/highlight handling.
///
/// The caller is responsible for drawing item contents inside `render_item`.
/// The `start_y` parameter is adjusted by the list's scroll offset.
pub fn render_list<F>(
    frame: &mut RenderFrame,
    content: Rectangle,
    line_height: f32,
    highlight_color: u32,
    state: &ListState,
    mut render_item: F,
) where
    F: FnMut(&mut RenderFrame, usize, f32, bool),
{
    // Push scissor to clip content to the viewport
    frame.push_scissor(content);

    let adjusted_y = content.y - state.scroll_offset;

    for idx in 0..state.count {
        let y = adjusted_y + idx as f32 * line_height;

        // Skip items that are completely outside the viewport
        if y + line_height < content.y || y > content.y + content.h {
            continue;
        }

        let is_selected = state.selected == Some(idx);

        if is_selected {
            frame.fill_rectangle(
                &Rectangle {
                    x: content.x,
                    y,
                    w: content.w,
                    h: line_height,
                },
                highlight_color,
            );
        }

        render_item(frame, idx, y, is_selected);
    }

    // Draw scrollbar if not all items fit in view
    let total_height = state.count as f32 * line_height;
    if total_height > content.h {
        const SCROLLBAR_WIDTH: f32 = 2.0;
        const SCROLLBAR_MARGIN: f32 = 2.0;
        const SCROLLBAR_COLOR: u32 = 0xFFFFFFFFu32;

        let scrollbar_x = content.x + content.w - SCROLLBAR_WIDTH - SCROLLBAR_MARGIN;

        // Calculate scrollbar position and height
        let scrollbar_height = (content.h / total_height) * content.h;
        let scrollbar_y = content.y + (state.scroll_offset / total_height) * content.h;
        frame.fill_rectangle(
            &Rectangle {
                x: scrollbar_x,
                y: scrollbar_y,
                w: SCROLLBAR_WIDTH,
                h: scrollbar_height,
            },
            SCROLLBAR_COLOR,
        );
    }

    // Pop scissor to restore previous clipping state
    frame.pop_scissor();
}
