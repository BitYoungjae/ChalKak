use super::ToolPoint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFontFamily {
    Sans,
    Serif,
}

impl TextFontFamily {
    pub const fn cairo_font_name(self) -> &'static str {
        match self {
            Self::Sans => "Sans",
            Self::Serif => "Serif",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextOptions {
    pub color_r: u8,
    pub color_g: u8,
    pub color_b: u8,
    pub size: u8,
    pub weight: u16,
    pub family: TextFontFamily,
}

impl Default for TextOptions {
    fn default() -> Self {
        Self {
            color_r: 0,
            color_g: 0,
            color_b: 0,
            size: 16,
            weight: 500,
            family: TextFontFamily::Sans,
        }
    }
}

impl TextOptions {
    pub fn set_color(&mut self, color_r: u8, color_g: u8, color_b: u8) {
        self.color_r = color_r;
        self.color_g = color_g;
        self.color_b = color_b;
    }

    pub fn set_size(&mut self, size: u8) {
        self.size = clamp_text_size(size);
    }

    pub fn set_weight(&mut self, weight: u16) {
        self.weight = clamp_text_weight(weight);
    }

    pub fn set_family(&mut self, family: TextFontFamily) {
        self.family = family;
    }
}

const fn clamp_text_size(size: u8) -> u8 {
    if size == 0 {
        1
    } else {
        size
    }
}

const fn clamp_text_weight(weight: u16) -> u16 {
    if weight < 100 {
        100
    } else if weight > 1000 {
        1000
    } else {
        weight
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextElement {
    pub id: u64,
    pub x: i32,
    pub y: i32,
    pub content: String,
    cursor_chars: usize,
    pub options: TextOptions,
}

impl TextElement {
    pub fn new(id: u64, anchor: ToolPoint, options: TextOptions) -> Self {
        Self {
            id,
            x: anchor.x,
            y: anchor.y,
            content: String::new(),
            cursor_chars: 0,
            options,
        }
    }

    pub fn with_text(
        id: u64,
        anchor: ToolPoint,
        text: impl Into<String>,
        options: TextOptions,
    ) -> Self {
        let content = text.into();
        let cursor_chars = content.chars().count();
        Self {
            id,
            x: anchor.x,
            y: anchor.y,
            content,
            cursor_chars,
            options,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        let byte_index = self.byte_index_for_cursor(self.cursor_chars);
        self.content.insert(byte_index, c);
        self.cursor_chars = self.cursor_chars.saturating_add(1);
    }

    pub fn delete_backward(&mut self) -> bool {
        if self.cursor_chars == 0 {
            return false;
        }
        let end = self.byte_index_for_cursor(self.cursor_chars);
        let start = self.byte_index_for_cursor(self.cursor_chars.saturating_sub(1));
        if start >= end || end > self.content.len() {
            return false;
        }
        self.content.drain(start..end);
        self.cursor_chars = self.cursor_chars.saturating_sub(1);
        true
    }

    pub fn insert_newline(&mut self) {
        let byte_index = self.byte_index_for_cursor(self.cursor_chars);
        self.content.insert(byte_index, '\n');
        self.cursor_chars = self.cursor_chars.saturating_add(1);
    }

    pub fn cursor_chars(&self) -> usize {
        self.cursor_chars.min(self.content.chars().count())
    }

    pub fn move_cursor_left(&mut self) -> bool {
        if self.cursor_chars() == 0 {
            return false;
        }
        self.cursor_chars = self.cursor_chars().saturating_sub(1);
        true
    }

    pub fn move_cursor_right(&mut self) -> bool {
        let max_chars = self.content.chars().count();
        if self.cursor_chars() >= max_chars {
            return false;
        }
        self.cursor_chars = self.cursor_chars().saturating_add(1);
        true
    }

    pub fn move_cursor_up(&mut self) -> bool {
        self.move_cursor_vertically(-1)
    }

    pub fn move_cursor_down(&mut self) -> bool {
        self.move_cursor_vertically(1)
    }

    pub fn move_cursor_to_end(&mut self) {
        self.cursor_chars = self.content.chars().count();
    }

    fn byte_index_for_cursor(&self, cursor_chars: usize) -> usize {
        let cursor_chars = cursor_chars.min(self.content.chars().count());
        self.content
            .char_indices()
            .nth(cursor_chars)
            .map(|(index, _)| index)
            .unwrap_or(self.content.len())
    }

    fn cursor_line_column(&self) -> (usize, usize) {
        let mut line = 0_usize;
        let mut column = 0_usize;
        let cursor = self.cursor_chars();
        for (index, ch) in self.content.chars().enumerate() {
            if index >= cursor {
                break;
            }
            if ch == '\n' {
                line = line.saturating_add(1);
                column = 0;
            } else {
                column = column.saturating_add(1);
            }
        }
        (line, column)
    }

    fn cursor_index_for_line_column(&self, target_line: usize, target_column: usize) -> usize {
        let mut line = 0_usize;
        let mut column = 0_usize;
        for (index, ch) in self.content.chars().enumerate() {
            if line == target_line && column == target_column {
                return index;
            }
            if ch == '\n' {
                if line == target_line {
                    return index;
                }
                line = line.saturating_add(1);
                column = 0;
            } else {
                column = column.saturating_add(1);
            }
        }
        self.content.chars().count()
    }

    fn move_cursor_vertically(&mut self, delta_lines: i32) -> bool {
        let lines = self.content.split('\n').collect::<Vec<_>>();
        if lines.len() <= 1 {
            return false;
        }
        let (line, column) = self.cursor_line_column();
        let target_line = if delta_lines < 0 {
            line.saturating_sub(delta_lines.unsigned_abs() as usize)
        } else {
            line.saturating_add(delta_lines as usize)
        }
        .min(lines.len().saturating_sub(1));
        if target_line == line {
            return false;
        }

        let target_column = column.min(lines[target_line].chars().count());
        let next_cursor = self.cursor_index_for_line_column(target_line, target_column);
        if next_cursor == self.cursor_chars() {
            return false;
        }
        self.cursor_chars = next_cursor;
        true
    }
}
