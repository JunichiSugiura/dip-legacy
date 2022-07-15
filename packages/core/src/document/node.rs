use crate::document::{
    buffer::{Buffer, BufferCursor},
    piece::Piece,
};
use intrusive_collections::RBTreeAtomicLink;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone)]
pub struct NodePosition {
    pub node: Node,
    remainder: i32,
    pub node_start_offset: i32,
}

impl NodePosition {
    pub fn new(node: Node, remainder: i32, node_start_offset: i32) -> NodePosition {
        NodePosition {
            node,
            remainder,
            node_start_offset,
        }
    }
}

#[derive(Debug)]
pub struct NodeLinePosition {
    pub node: Node,
    pub node_start_offset: i32,
    node_start_line_number: i32,
}

#[derive(Default, Debug, Clone)]
pub struct Node {
    pub link: RBTreeAtomicLink,

    pub piece: Piece,
    pub r#type: NodeType,

    pub left_len: i32,
    pub left_line_feed_count: i32,
}

impl Node {
    pub fn is_original(&self) -> bool {
        self.r#type == NodeType::Original
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Original,
    Changed,
}

impl Default for NodeType {
    fn default() -> NodeType {
        NodeType::Changed
    }
}

impl Node {
    pub fn from_original_buffer(buffer: &Buffer) -> Node {
        let grapheme_len = buffer.value.graphemes(true).count() as i32;
        let line_starts_len = buffer.line_starts.len() as i32;
        let end_line = if line_starts_len == 0 {
            0
        } else {
            line_starts_len - 1
        };

        Node {
            piece: Piece::new(
                BufferCursor::default(),
                BufferCursor::new(end_line, grapheme_len),
                line_starts_len,
                grapheme_len,
            ),
            r#type: NodeType::Original,
            ..Default::default()
        }
    }

    pub fn from_changed_buffer(buffer: &Buffer, start_offset: i32, start: BufferCursor) -> Node {
        let end_offset = buffer.value.graphemes(true).count() as i32;
        let line_starts_len = buffer.line_starts.len() as i32;
        let end_line = if line_starts_len == 0 {
            0
        } else {
            line_starts_len - 1
        };
        let end_column = end_offset - buffer.line_starts[end_line as usize];
        let end = BufferCursor::new(end_line as i32, end_column);

        Node {
            piece: Piece::new(
                start,
                end,
                buffer.get_line_feed_count(&start, &end),
                end_offset - start_offset,
            ),
            r#type: NodeType::Changed,
            ..Default::default()
        }
    }

    pub fn total_len(&self) -> i32 {
        self.left_len + self.piece.len
    }

    pub fn start_with_lf_from_string(value: &String) -> bool {
        return value.as_str().graphemes(true).last() == Some("\n");
    }

    pub fn end_with_cr_from_string(value: &String) -> bool {
        match value.graphemes(true).last() {
            Some(c) => match c {
                "\r" => true,
                _ => false,
            },
            None => false,
        }
    }

    pub fn start_with_lf(&self, buffer: &Buffer) -> bool {
        if buffer.line_starts.len() == 0 {
            return false;
        } else {
            if self.piece.start.line == buffer.line_starts.len() as i32 - 1 {
                return false;
            }

            let next_line_offset = buffer.line_starts[self.piece.start.line as usize + 1];
            let start_offset =
                buffer.line_starts[self.piece.start.line as usize] + self.piece.start.column;
            if next_line_offset > start_offset + 1 {
                return false;
            }

            return buffer.value.graphemes(true).nth(start_offset as usize) == Some("\n");
        }
    }
}
