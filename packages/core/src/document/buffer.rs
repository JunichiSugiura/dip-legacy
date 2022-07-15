use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone)]
pub struct Buffer {
    pub value: String,
    pub line_starts: Vec<i32>,
}

impl Default for Buffer {
    fn default() -> Buffer {
        Buffer {
            value: Default::default(),
            line_starts: vec![0],
        }
    }
}

impl From<String> for Buffer {
    fn from(value: String) -> Buffer {
        let line_starts = Buffer::get_line_starts(&value.clone());
        Buffer { value, line_starts }
    }
}

impl Buffer {
    pub fn new(value: String, line_starts: Vec<i32>) -> Buffer {
        Buffer { value, line_starts }
    }

    pub fn offset(&self, cursor: BufferCursor) -> i32 {
        self.line_starts.get(cursor.line as usize).unwrap_or(&0) + cursor.column
    }

    pub fn get_line_starts(value: &String) -> Vec<i32> {
        let mut line_starts = vec![0];

        let enumerate = &mut value.grapheme_indices(true).enumerate();
        while let Some((i, (grapheme_index, c))) = enumerate.next() {
            match c {
                "\r" => {
                    enumerate.nth(i + 1);
                    line_starts.push(grapheme_index as i32 + 1);
                }
                "\n" => {
                    line_starts.push(grapheme_index as i32 + 1);
                }
                _ => {}
            }
        }

        line_starts
    }

    pub fn get_line_feed_count(&self, start: &BufferCursor, end: &BufferCursor) -> i32 {
        if end.column == 0 || end.line == self.line_starts.len() as i32 - 1 {
            return end.line - start.line;
        }

        let end_line = end.line as usize;
        let next_line_start_offset = self.line_starts[end_line + 1];
        let end_offset = self.line_starts[end_line] + end.column;
        if next_line_start_offset > end_offset + 1 {
            return end.line - start.line;
        }

        let previous_grapheme_offset = end_offset as usize - 1;
        if self.value.graphemes(true).collect::<Vec<&str>>()[previous_grapheme_offset] == "\r" {
            return end.line - start.line + 1;
        } else {
            return end.line - start.line;
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct BufferCursor {
    pub line: i32,
    pub column: i32,
}

impl BufferCursor {
    pub fn new(line: i32, column: i32) -> Self {
        Self { line, column }
    }
}
