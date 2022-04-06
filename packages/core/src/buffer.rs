use bevy::ecs::prelude::*;
use intrusive_collections::intrusive_adapter;
use intrusive_collections::{KeyAdapter, RBTree, RBTreeAtomicLink};
use std::{convert::From, fs, str};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Component, Default, Debug)]
pub struct TextBuffer {
    file_path: Option<&'static str>,
    tree: RBTree<NodeAdapter>,
    original: String,
    info: TextBufferInfo,
    last_change_buffer_position: BufferCursor, // TODO: move to TextBufferCache
}

impl TextBuffer {
    pub fn new(file_path: &'static str, default_eol: DefaultEOL) -> TextBuffer {
        let original = fs::read_to_string(file_path.clone()).expect("Failed to read file");
        let (info, line_starts) =
            TextBufferInfo::new_with_line_starts(original.clone().as_str(), default_eol);
        let mut buffer = TextBuffer {
            file_path: Some(file_path),
            tree: RBTree::<NodeAdapter>::default(),
            original: original.clone(),
            info,
            last_change_buffer_position: BufferCursor::default(),
        };

        if buffer.original.is_empty() {
            return buffer;
        }

        let piece = Piece::new(0, original, line_starts);
        let node = Node::from(piece);
        buffer.tree.insert(Box::new(node));

        buffer
    }
}

impl TextBuffer {
    pub fn insert(&mut self, offset: i32, value: &str) {
        if self.tree.is_empty() {
            let line_starts = TextBufferInfo::get_line_start_slice(&value.to_string());
            let piece = Piece::new(offset, value.to_string(), line_starts);
            let node = Node::from(piece);
            self.tree.insert(Box::new(node));
        } else {
            let position = self.get_node_position(offset);
            let node = self
                .tree
                .find(&position.node_key)
                .get()
                .expect("Cannot find node")
                .clone();

            if node.piece.offset == 0
                && node.piece.end.line == self.last_change_buffer_position.line
                && node.piece.end.column == self.last_change_buffer_position.column
                && position.node_start_offset + node.piece.len() == offset
            {
                self.append(node, value.to_string());
            } else if position.node_start_offset == offset {
                // self.insert_left(node, value);
                // self.search_cache.validate(offset);
            } else if position.node_start_offset + node.piece.len() > offset {
                // self.insert_middle(node, value);
            } else {
                // self.insert_right(node, value);
            }

            // self.compute_buffer_metadata();
        }
    }

    fn append(&mut self, mut node: Node, mut value: String) {
        if self.adjust_cr_from_next(node.clone(), &mut value) {
            value.push_str("\n");
        }

        let start_offset = node.total_size();
        let mut line_starts = TextBufferInfo::get_line_start_slice(&mut value);
        for line_start in line_starts.iter_mut() {
            *line_start += start_offset;
        }

        let hit_crlf = self.should_check_crlf()
            && Node::start_with_lf_from_string(&mut value)
            && self.end_with_cr(&node);
        if hit_crlf {
            let prev_start_offset = node.piece.line_starts[node.piece.line_starts.len() - 2];
            node.piece.line_starts.pop();
            // last_change_buffer_position is already wrong */
            self.last_change_buffer_position = BufferCursor::new(
                self.last_change_buffer_position.line - 1,
                start_offset - prev_start_offset,
            );
        }
        line_starts.remove(0);

        // this._buffers[0].lineStarts = (<number[]>this._buffers[0].lineStarts).concat(<number[]>lineStarts.slice(1));
        /* const endIndex = this._buffers[0].lineStarts.length - 1; */
        /* const endColumn = this._buffers[0].buffer.length - this._buffers[0].lineStarts[endIndex]; */
        /* const newEnd = { line: endIndex, column: endColumn }; */
        /* const newLength = node.piece.length + value.length; */
        /* const oldLineFeedCnt = node.piece.lineFeedCnt; */
        /* const newLineFeedCnt = this.getLineFeedCnt(0, node.piece.start, newEnd); */
        /* const lf_delta = newLineFeedCnt - oldLineFeedCnt; */

        // node.piece = Piece::new(
        // 	node.piece.bufferIndex,
        // 	node.piece.start,
        // 	newEnd,
        // 	newLineFeedCnt,
        // 	newLength
        // );

        /* this._lastChangeBufferPos = newEnd; */
        /* updateTreeMetadata(this, node, value.length, lf_delta); */
    }

    fn get_node_position(&self, mut offset: i32) -> NodePosition {
        /* let cache = self.search_cache.get(offset); */
        /* if (cache) { */
        /*     NodePosition::new(cache.cursor, cache.node_start_offset, offset - cache.node_start_offset); */
        /* } */

        let mut node_start_offset = 0;
        let mut res = None;
        let mut cursor = self.tree.front();

        while !cursor.is_null() {
            let node = cursor.get().expect("Cursor is null");

            if node.left_size > offset {
                cursor.move_prev();
            } else if node.total_size() >= offset {
                node_start_offset += node.left_size;
                let position = NodePosition::new(
                    node.total_size(),
                    offset - node.left_size,
                    node_start_offset,
                );
                // self.search_cache.set(res);
                res = Some(position);
                break;
            } else {
                offset -= node.total_size();
                node_start_offset += node.total_size();
                cursor.move_next();
            }
        }

        res.expect("Tree must NOT be empty when calling node_at method")
    }

    pub fn delete(&self, _offset: i32, _count: i32) {
        todo!("delete");
    }

    fn adjust_cr_from_next(&mut self, node: Node, value: &mut String) -> bool {
        if !(self.should_check_crlf() && Node::end_with_cr_from_string(value)) {
            return false;
        }

        let mut cursor = self.tree.find_mut(&node.piece.offset);
        match cursor.as_cursor().get() {
            Some(node) => {
                cursor.as_cursor().move_next();
                if node.start_with_lf(&self.original) {
                    value.push_str("\n");

                    if node.piece.len() == 1 {
                        cursor.remove();
                    } else {
                        match cursor.get() {
                            Some(node) => {
                                let mut node = node.clone();
                                node.piece = Piece::new(
                                    node.piece.offset,
                                    value.to_string(),
                                    node.piece.line_starts,
                                );
                                cursor.replace_with(Box::new(node.clone())).unwrap();

                                self.update_tree_metadata(&node, -1, -1);
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

    fn end_with_cr(&self, node: &Node) -> bool {
        let cursor = self.tree.find(&node.total_size());
        if cursor.is_null() || node.piece.line_feed_count == 0 {
            return false;
        }

        let node = cursor.get().expect("Cursor is null");
        match node.piece.value.graphemes(true).last() {
            Some("\r") => true,
            Some(_) => false,
            None => false,
        }
    }

    fn update_tree_metadata(&mut self, node: &Node, delta: i32, line_feed_count_delta: i32) {
        while let Some(key) = node.parent_key {
            let mut cursor = self.tree.find_mut(&key);
            let mut node = node.clone();
            node.left_size += delta;
            node.left_lf += line_feed_count_delta;
            cursor
                .replace_with(Box::new(node))
                .expect("Failed to replace parent node meta data");
        }
    }

    fn should_check_crlf(&self) -> bool {
        return !(self.info.eos_normalized && self.info.eol == EOL::LF);
    }

    pub fn to_string(&self) -> String {
        let mut text = String::new();
        for node in self.tree.iter() {
            text.insert_str(node.piece.offset as usize, node.piece.value.as_str());
        }
        text
    }
}

const UTF8_BOM: &str = "\u{feff}";

struct NodePosition {
    node_key: i32,
    remainder: i32,
    node_start_offset: i32,
}

impl NodePosition {
    fn new(node_key: i32, remainder: i32, node_start_offset: i32) -> NodePosition {
        NodePosition {
            node_key,
            remainder,
            node_start_offset,
        }
    }
}
#[derive(Debug, Default)]
pub struct TextBufferInfo {
    encoding: CharacterEncoding,
    eol: EOL,
    eos_normalized: bool,
    // contains_rtl: bool,
    // contains_unusual_line_terminators: bool,
    is_ascii: bool,
    // normalize_eol: bool,
}

impl TextBufferInfo {
    fn new_with_line_starts(value: &str, default_eol: DefaultEOL) -> (TextBufferInfo, Vec<i32>) {
        let encoding = CharacterEncoding::from(value);
        let mut line_starts = vec![];
        let mut line_break_count = LineBreakCount::default();
        let mut is_ascii = true;

        let enumerate = &mut value.grapheme_indices(true).enumerate();
        while let Some((i, (grapheme_index, c))) = enumerate.next() {
            match c {
                "\r" => match enumerate.nth(i + 1) {
                    Some((_, (grapheme_index, "\n"))) => {
                        line_starts.push(grapheme_index as i32);
                        line_break_count.crlf += 1;
                    }
                    Some(_) => {
                        line_starts.push(grapheme_index as i32);
                        line_break_count.cr += 1;
                    }
                    None => {
                        line_starts.push(grapheme_index as i32);
                        line_break_count.cr += 1;
                    }
                },
                "\n" => {
                    line_starts.push(grapheme_index as i32);
                    line_break_count.lf += 1;
                }
                _ => {
                    if !c.is_ascii() {
                        is_ascii = false
                    }
                }
            }
        }

        let total_eol_count = line_break_count.cr + line_break_count.lf + line_break_count.crlf;
        let total_cr_count = line_break_count.cr + line_break_count.crlf;

        let eol = match (total_eol_count, default_eol) {
            (x, default_eol) if x == 0 && default_eol == DefaultEOL::LF => EOL::LF,
            (x, _) if x == 0 || total_cr_count > total_eol_count / 2 => EOL::CRLF,
            _ => EOL::LF,
        };

        let info = TextBufferInfo {
            encoding,
            eol,
            eos_normalized: false,
            is_ascii,
        };

        (info, line_starts)
    }

    fn get_line_start_slice(value: &String) -> Vec<i32> {
        let mut line_start_slice = vec![];

        let enumerate = &mut value.grapheme_indices(true).enumerate();
        while let Some((i, (grapheme_index, c))) = enumerate.next() {
            match c {
                "\r" => {
                    enumerate.nth(i + 1);
                    line_start_slice.push(grapheme_index as i32 + 1);
                }
                "\n" => {
                    line_start_slice.push(grapheme_index as i32 + 1);
                }
                _ => {}
            }
        }

        line_start_slice
    }
}

#[derive(PartialEq)]
pub enum DefaultEOL {
    LF = 1,
    CRLF = 2,
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

impl From<&str> for CharacterEncoding {
    fn from(s: &str) -> Self {
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
    link: RBTreeAtomicLink,
    left_size: i32,
    left_lf: i32,
    parent_key: Option<i32>,
    piece: Piece,
}

impl Node {
    fn total_size(&self) -> i32 {
        self.left_size + self.piece.len()
    }

    fn start_with_lf(&self, original: &String) -> bool {
        if self.piece.line_feed_count == 0 {
            return false;
        } else {
            if self.piece.start.line == self.piece.line_starts.len() as i32 - 1 {
                return false;
            }

            let next_line_offset = self.piece.line_starts[self.piece.start.line as usize + 1];
            let start_offset =
                self.piece.line_starts[self.piece.start.line as usize] + self.piece.start.column;
            if next_line_offset > start_offset + 1 {
                return false;
            }

            return original.graphemes(true).nth(start_offset as usize) == Some("\n");
        }
    }

    fn start_with_lf_from_string(value: &String) -> bool {
        return value.as_str().graphemes(true).last() == Some("\n");
    }

    fn end_with_cr_from_string(value: &String) -> bool {
        match value.graphemes(true).last() {
            Some(c) => match c {
                "\r" => true,
                _ => false,
            },
            None => false,
        }
    }
}

#[derive(Default, Debug, Clone)]
struct Piece {
    offset: i32,
    value: String,
    start: BufferCursor,
    end: BufferCursor,
    line_starts: Vec<i32>,
    line_feed_count: i32,
}

impl Piece {
    fn new(offset: i32, value: String, line_starts: Vec<i32>) -> Piece {
        let end_index = if line_starts.len() == 0 {
            0
        } else {
            line_starts.len() as i32 - 1
        };
        let start = BufferCursor::default();
        let end = BufferCursor::new(
            end_index,
            match line_starts.last() {
                Some(x) => value.len() as i32 - x,
                None => 0,
            },
        );
        let line_feed_count =
            Piece::get_line_feed_count(&line_starts, &value.clone(), &start, &end);

        Piece {
            offset,
            value,
            start,
            end,
            line_starts,
            line_feed_count,
        }
    }

    fn len(&self) -> i32 {
        self.value.len() as i32
    }

    fn get_line_feed_count(
        line_starts: &[i32],
        original: &String,
        start: &BufferCursor,
        end: &BufferCursor,
    ) -> i32 {
        if end.column == 0 {
            return 0;
        }

        if end.line == line_starts.len() as i32 - 1 {
            return end.line - start.line;
        }

        let next_line_start_offset = line_starts[end.line as usize + 1];
        let end_offset = line_starts[end.line as usize] + end.column;
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
}

intrusive_adapter!(pub NodeAdapter = Box<Node>: Node { link: RBTreeAtomicLink });
impl<'a> KeyAdapter<'a> for NodeAdapter {
    type Key = i32;
    fn get_key(&self, node: &'a Node) -> i32 {
        node.total_size()
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
