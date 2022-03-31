use bevy::ecs::prelude::*;
use intrusive_collections::intrusive_adapter;
use intrusive_collections::{rbtree::AtomicLink, KeyAdapter, RBTree};
use std::{convert::From, fs, str};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Component, Default, Debug)]
pub struct TextBuffer {
    file_path: Option<&'static str>,
    tree: RBTree<NodeAdapter>,
    original: String,
    info: TextBufferInfo,
    last_change_buffer_pos: BufferCursor, // TODO: move to TextBufferCache
}

impl From<&'static str> for TextBuffer {
    fn from(file_path: &'static str) -> TextBuffer {
        let mut buffer = TextBuffer::default();
        let text = fs::read_to_string(file_path.clone()).expect("Failed to read file");
        buffer.original = text.clone();
        buffer.info = TextBufferInfo::new(&text);

        if text.is_empty() {
            return buffer;
        } else {
            buffer.insert(0, &text);
            buffer
        }
    }
}

impl TextBuffer {
    pub fn insert(&mut self, offset: i32, text: &str) {
        if self.tree.is_empty() {
            let end_index = if self.info.line_starts.len() == 0 {
                0
            } else {
                self.info.line_starts.len() as i32 - 1
            };
            let start = BufferCursor::default();
            let end = BufferCursor::new(
                end_index,
                match self.info.line_starts.last() {
                    Some(x) => text.len() as i32 - x,
                    None => 0,
                },
            );
            let piece = Piece::new(
                offset,
                text.to_string(),
                start,
                end,
                text.len() as i32,
                get_line_feed_count(&self.info, &self.original, &start, &end),
            );
            let node = Node::from(piece);
            self.tree.insert(Box::new(node));
        } else {
            let position = self.node_at(offset);
            let mut text = text.to_string();
            match self.tree.front().get() {
                Some(node) => {
                    if node.piece.offset == 0
                        && node.piece.end.line == self.last_change_buffer_pos.line
                        && node.piece.end.column == self.last_change_buffer_pos.column
                        && position.node_start_offset + node.piece.len == offset
                    {
                        self.append_to_node(node.clone(), &mut text);
                        // self.compute_buffer_metadata();
                        return;
                    }
                }
                None => {}
            }
        }
    }

    fn append_to_node(&mut self, node: Node, text: &mut String) {
        if self.adjust_cr_from_next(node, text) {
            text.push_str("\n");
        }
    }

    fn node_at(&self, mut offset: i32) -> NodePosition {
        /* let cache = self.search_cache.get(offset); */
        /* if (cache) { */
        /*     NodePosition::new(cache.cursor, cache.node_start_offset, offset - cache.node_start_offset); */
        /* } */

        let mut node_start_offset = 0;
        let mut res = None;
        let mut c = self.tree.find(&offset);

        while !c.is_null() {
            match c.get() {
                Some(node) => {
                    if node.size_left > offset {
                        c.move_prev();
                    } else if node.size_left + node.piece.len >= offset {
                        node_start_offset += node.size_left;
                        let position =
                            NodePosition::new(offset - node.size_left, node_start_offset);
                        // self.search_cache.set(res);
                        res = Some(position);
                        break;
                    } else {
                        offset -= node.size_left + node.piece.len;
                        node_start_offset += node.size_left + node.piece.len;
                        c.move_next();
                    }
                }
                None => {}
            }
        }

        res.expect("Tree must NOT be empty when calling node_at method")
    }

    pub fn delete(&self, _offset: i32, _count: i32) {
        todo!("delete");
    }

    fn adjust_cr_from_next(&mut self, node: Node, value: &mut String) -> bool {
        if !(self.should_check_crlf() && self.end_with_cr(value)) {
            return false
        }

        let mut cursor = self.tree.find_mut(&node.piece.offset);
        match cursor.as_cursor().get() {
            Some(node) => {
                cursor.as_cursor().move_next();
                if start_with_lf(node, &self.info, &self.original) {
                    value.push_str("\n");

                    if node.piece.len == 1 {
                        cursor.remove();
                    } else {
                        match cursor.get() {
                            Some(node) => {
                                let piece = Piece::new(
                                    node.piece.offset,
                                    value.to_string(),
                                    BufferCursor::new(node.piece.start.line + 1, 0),
                                    node.piece.end,
                                    value.len() as i32,
                                    get_line_feed_count(&self.info, &self.original, &node.piece.start, &node.piece.end),
                                );
                                let node = Node::from(piece);
                                cursor.replace_with(Box::new(node)).unwrap();

                                // update_tree_metadata(this, nextNode, -1, -1);
                            }
                            None => {}
                        }
                    }
                    return true;
                } else {
                    return false;
                }
            }
            None => return false,
        }
    }

    fn end_with_cr(&self, text: &String) -> bool {
        match text.graphemes(true).last() {
            Some(c) => match c {
                "\r" => true,
                _ => false,
            },
            None => false,
        }
    }

    fn should_check_crlf(&self) -> bool {
        return !(self.info.eos_normalized && self.info.eol == EOL::LF);
    }

    pub fn to_string(&self) -> String {
        let mut text = String::new();
        for node in self.tree.iter() {
            text.insert_str(node.piece.offset as usize, node.piece.text.as_str());
        }
        text
    }
}

const UTF8_BOM: &str = "\u{feff}";

struct NodePosition {
    remainder: i32,
    node_start_offset: i32,
}

impl NodePosition {
    fn new(remainder: i32, node_start_offset: i32) -> NodePosition {
        NodePosition {
            remainder,
            node_start_offset,
        }
    }
}
#[derive(Debug, Default)]
pub struct TextBufferInfo {
    encoding: CharacterEncoding,
    line_starts: Vec<i32>,
    line_break_count: LineBreakCount,
    eol: EOL,
    eos_normalized: bool,
    // contains_rtl: bool,
    // contains_unusual_line_terminators: bool,
    is_ascii: bool,
    // normalize_eol: bool,
}

impl TextBufferInfo {
    fn new(text: &String) -> TextBufferInfo {
        let mut info = TextBufferInfo::default();
        info.encoding = CharacterEncoding::from(text);
        info.is_ascii = true;

        let enumerate = &mut text.graphemes(true).enumerate();
        while let Some((i, c)) = enumerate.next() {
            match c {
                "\r" => match enumerate.nth(i + 1) {
                    Some((_, c)) => match c {
                        "\n" => {
                            info.eol = EOL::CRLF;
                            info.line_starts.push(i as i32 + 2);
                            info.line_break_count.crlf += 1;
                        }
                        _ => {
                            info.eol = EOL::CR;
                            info.line_starts.push(i as i32 + 1);
                            info.line_break_count.cr += 1;
                        }
                    },
                    None => {}
                },
                "\n" => {
                    info.line_starts.push(i as i32 + 1);
                    info.line_break_count.lf += 1;
                }
                _ => {
                    if !c.is_ascii() {
                        info.is_ascii = false
                    }
                }
            }
        }

        info
    }
}

#[derive(Debug, PartialEq)]
enum EOL {
    LF,
    CR,
    CRLF,
}

impl Default for EOL {
    fn default() -> EOL {
        EOL::LF
    }
}

#[derive(Debug)]
enum CharacterEncoding {
    Utf8,
    Utf8WithBom,
}

impl Default for CharacterEncoding {
    fn default() -> CharacterEncoding {
        CharacterEncoding::Utf8
    }
}

impl From<&String> for CharacterEncoding {
    fn from(s: &String) -> Self {
        if s.starts_with(UTF8_BOM) {
            CharacterEncoding::Utf8WithBom
        } else {
            CharacterEncoding::Utf8
        }
    }
}

#[derive(Debug, Default)]
struct LineBreakCount {
    cr: i32,
    lf: i32,
    crlf: i32,
}

#[derive(Default, Debug, Clone)]
pub struct Node {
    link: AtomicLink,
    size_left: i32,
    left_lf: i32,
    piece: Piece,
}

#[derive(Default, Debug, Clone)]
struct Piece {
    offset: i32,
    text: String,
    start: BufferCursor,
    end: BufferCursor,
    len: i32,
    line_feed_count: i32,
}

impl Piece {
    fn new(
        offset: i32,
        text: String,
        start: BufferCursor,
        end: BufferCursor,
        len: i32,
        line_feed_count: i32,
    ) -> Piece {
        Piece {
            offset,
            text,
            start,
            end,
            len,
            line_feed_count,
        }
    }
}

intrusive_adapter!(pub NodeAdapter = Box<Node>: Node { link: AtomicLink });
impl<'a> KeyAdapter<'a> for NodeAdapter {
    type Key = i32;
    fn get_key(&self, e: &'a Node) -> i32 {
        e.piece.offset
    }
}

impl From<Piece> for Node {
    fn from(piece: Piece) -> Self {
        Self {
            piece,
            ..Default::default()
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct BufferCursor {
    line: i32,
    column: i32,
}

impl BufferCursor {
    fn new(line: i32, column: i32) -> Self {
        Self { line, column }
    }
}

fn start_with_lf(node: &Node, info: &TextBufferInfo, original: &String) -> bool {
    if node.piece.line_feed_count == 0 {
        return false;
    } else {
        if node.piece.start.line == info.line_starts.len() as i32 - 1 {
            return false;
        }
        let next_line_offset = info.line_starts[node.piece.start.line as usize + 1];
        let start_offset =
            info.line_starts[node.piece.start.line as usize] + node.piece.start.column;
        if next_line_offset > start_offset + 1 {
            return false;
        }

        return original.graphemes(true).nth(start_offset as usize) == Some("\n");
    }
}

fn get_line_feed_count(
    info: &TextBufferInfo,
    original: &String,
    start: &BufferCursor,
    end: &BufferCursor,
) -> i32 {
    if end.column == 0 {
        return 0;
    }

    if end.line == info.line_starts.len() as i32 - 1 {
        return end.line - start.line;
    }

    let next_line_start_offset = info.line_starts[end.line as usize + 1];
    let end_offset = info.line_starts[end.line as usize] + end.column;
    if next_line_start_offset > end_offset + 1 {
        return end.line - start.line;
    }

    let previous_char_offset = end_offset as usize - 1;
    if original.graphemes(true).collect::<Vec<&str>>()[previous_char_offset] == "\r" {
        return end.line - start.line + 1;
    } else {
        return end.line - start.line;
    }
}

#[cfg(test)]
mod inserts_and_deletes {
    use crate::buffer::TextBuffer;
    #[test]
    fn basic_insert_and_delete() {
        let mut buffer = TextBuffer::default();
        buffer.insert(0, "This is a document with some text.");
        assert_eq!(buffer.to_string(), "This is a document with some text.");

        buffer.insert(34, "This is some more text to insert at offset 34.");
        assert_eq!(
            buffer.to_string(),
            "This is a document with some text.This is some more text to insert at offset 34."
        );

/*         buffer.delete(42, 5); */
/*         assert_eq!( */
/*             buffer.to_string(), */
/*             "This is a document with some text.This is more text to insert at offset 34." */
/*         ); */
    }
}
