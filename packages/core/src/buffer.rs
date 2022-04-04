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
            last_change_buffer_pos: BufferCursor::default(),
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
        // First insert
        if self.tree.is_empty() {
            let line_starts = TextBufferInfo::get_line_start_slice(&value.to_string());
            let piece = Piece::new(offset, value.to_string(), line_starts);
            let node = Node::from(piece);
            self.tree.insert(Box::new(node));
        } else {
            let position = self.node_at(offset);
            let mut value = value.to_string();
            match self.tree.find(&position.node_key).get() {
                Some(node) => {
                    if node.piece.offset == 0
                        && node.piece.end.line == self.last_change_buffer_pos.line
                        && node.piece.end.column == self.last_change_buffer_pos.column
                        && position.node_start_offset + node.piece.len() == offset
                    {
                        self.append_to_node(node.clone(), &mut value);
                        // self.compute_buffer_metadata();
                        return;
                    }
                }
                None => {}
            }
        }
    }

    fn append_to_node(&mut self, node: Node, value: &mut String) {
        if self.adjust_cr_from_next(node.clone(), value) {
            value.push_str("\n");
        }

        let start_offset = node.total_size();
        let mut line_starts = TextBufferInfo::get_line_start_slice(value);
        for line_start in line_starts.iter_mut() {
            *line_start += start_offset;
        }

        /* const hitCRLF = this.shouldCheckCRLF() && this.startWithLF(value) && this.endWithCR(node); */
        /* if (hitCRLF) { */
        /* const prevStartOffset = this._buffers[0].lineStarts[this._buffers[0].lineStarts.length - 2]; */
        /* (<number[]>this._buffers[0].lineStarts).pop(); */
        /* // _lastChangeBufferPos is already wrong */
        /* this._lastChangeBufferPos = { line: this._lastChangeBufferPos.line - 1, column: startOffset - prevStartOffset }; */
        /* } */

        /* this._buffers[0].lineStarts = (<number[]>this._buffers[0].lineStarts).concat(<number[]>lineStarts.slice(1)); */
        /* const endIndex = this._buffers[0].lineStarts.length - 1; */
        /* const endColumn = this._buffers[0].buffer.length - this._buffers[0].lineStarts[endIndex]; */
        /* const newEnd = { line: endIndex, column: endColumn }; */
        /* const newLength = node.piece.length + value.length; */
        /* const oldLineFeedCnt = node.piece.lineFeedCnt; */
        /* const newLineFeedCnt = this.getLineFeedCnt(0, node.piece.start, newEnd); */
        /* const lf_delta = newLineFeedCnt - oldLineFeedCnt; */

        /* node.piece = Piece::new( */
        /* 	node.piece.bufferIndex, */
        /* 	node.piece.start, */
        /* 	newEnd, */
        /* 	newLineFeedCnt, */
        /* 	newLength */
        /* ); */

        /* this._lastChangeBufferPos = newEnd; */
        /* updateTreeMetadata(this, node, value.length, lf_delta); */
    }

    fn node_at(&self, mut offset: i32) -> NodePosition {
        /* let cache = self.search_cache.get(offset); */
        /* if (cache) { */
        /*     NodePosition::new(cache.cursor, cache.node_start_offset, offset - cache.node_start_offset); */
        /* } */

        let mut node_start_offset = 0;
        let mut res = None;
        let mut c = self.tree.front();

        while !c.is_null() {
            match c.get() {
                Some(node) => {
                    if node.left_size > offset {
                        c.move_prev();
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

    fn adjust_cr_from_next(&mut self, _node: Node, value: &mut String) -> bool {
        if !(self.should_check_crlf() && self.end_with_cr(value)) {
            return false;
        }

        todo!("adjust_cr_from_next");
        // let mut cursor = self.tree.find_mut(&node.piece.offset);
        // match cursor.as_cursor().get() {
        //     Some(node) => {
        //         cursor.as_cursor().move_next();
        //         if start_with_lf(node, &self.info, &self.original) {
        //             value.push_str("\n");

        //             if node.piece.len() == 1 {
        //                 cursor.remove();
        //             } else {
        //                 match cursor.get() {
        //                     Some(node) => {
        //                         let piece = Piece::new(
        //                             node.piece.offset,
        //                             value.to_string(),
        //                             line_starts,
        //                             &self.info,
        //                         );
        //                         let node = Node::from(piece);
        //                         cursor.replace_with(Box::new(node)).unwrap();

        //                         // update_tree_metadata(this, nextNode, -1, -1);
        //                     }
        //                     None => {}
        //                 }
        //             }
        //             return true;
        //         } else {
        //             return false;
        //         }
        //     }
        //     None => return false,
        // }
    }

    fn end_with_cr(&self, value: &String) -> bool {
        match value.graphemes(true).last() {
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
                    Some((_, (grapheme_index, c))) => match c {
                        "\n" => {
                            line_starts.push(grapheme_index as i32);
                            line_break_count.crlf += 1;
                        }
                        _ => {
                            line_starts.push(grapheme_index as i32);
                            line_break_count.cr += 1;
                        }
                    },
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
    link: AtomicLink,
    left_size: i32,
    left_lf: i32,
    piece: Piece,
}

impl Node {
    fn total_size(&self) -> i32 {
        self.left_size + self.piece.len()
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

intrusive_adapter!(pub NodeAdapter = Box<Node>: Node { link: AtomicLink });
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

// fn start_with_lf(node: &Node, info: &TextBufferInfo, original: &String) -> bool {
//     if node.piece.line_feed_count == 0 {
//         return false;
//     } else {
//         if node.piece.start.line == info.line_starts.len() as i32 - 1 {
//             return false;
//         }
//         let next_line_offset = info.line_starts[node.piece.start.line as usize + 1];
//         let start_offset =
//             info.line_starts[node.piece.start.line as usize] + node.piece.start.column;
//         if next_line_offset > start_offset + 1 {
//             return false;
//         }

//         return original.graphemes(true).nth(start_offset as usize) == Some("\n");
//     }
// }

#[cfg(test)]
mod inserts_and_deletes {
    use crate::buffer::TextBuffer;
    #[test]
    fn basic_insert_and_delete() {
        let mut buffer = TextBuffer::default();
        buffer.insert(0, "T\nh\r\nis is a document with some text.");
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
