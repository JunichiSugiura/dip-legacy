use bevy::ecs::prelude::*;
use intrusive_collections::intrusive_adapter;
use intrusive_collections::{
    rbtree::{AtomicLinkOps, RBTreeOps},
    KeyAdapter, RBTree, RBTreeAtomicLink,
};
use std::{convert::From, fs, ptr::NonNull, str};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Component, Debug)]
pub struct Document {
    file_path: Option<&'static str>,
    text_buffer: TextBuffer,
}

impl Document {
    pub fn new(default_eol: DefaultEOL) -> Document {
        Document {
            file_path: None,
            text_buffer: TextBuffer::new(default_eol),
        }
    }

    pub fn from_file(file_path: &'static str, default_eol: DefaultEOL) -> Document {
        let original = fs::read_to_string(file_path.clone()).expect("Failed to read file");
        let text_buffer = TextBuffer::from_string(&original, default_eol);

        Document {
            file_path: Some(file_path),
            text_buffer,
        }
    }
}

#[derive(Default, Debug)]
pub struct TextBuffer {
    tree: RBTree<NodeAdapter>,
    info: TextBufferInfo,

    original: Buffer,
    changed: Option<Buffer>,

    cache: TextBufferCache,
}

impl TextBuffer {
    fn new(default_eol: DefaultEOL) -> TextBuffer {
        TextBuffer {
            info: TextBufferInfo::new(default_eol),
            ..Default::default()
        }
    }

    fn from_string(original: &str, default_eol: DefaultEOL) -> TextBuffer {
        let original = original.to_string();
        let (info, line_starts) = TextBufferInfo::from_string(&original.clone(), default_eol);

        let mut text_buffer = TextBuffer {
            info,
            original: Buffer::new(original.clone(), line_starts),
            ..Default::default()
        };

        let node = Node::from_original_buffer(&text_buffer.original);
        text_buffer.tree.insert(Box::new(node));

        text_buffer
    }
}

impl TextBuffer {
    pub fn insert(&mut self, offset: i32, value: &'static str) {
        let value = value.to_string();

        if self.changed.is_none() {
            // first insert
            self.add_node(&value);
        } else {
            let NodePosition {
                node,
                node_start_offset,
                ..
            } = self.get_node_position(offset);
            if node.is_original()
                && node.piece.end.line == self.cache.last_change.line
                && node.piece.end.column == self.cache.last_change.column
                && node_start_offset + node.piece.len == offset
            {
                // changed node
                self.append(node, value);
            } else if node_start_offset == offset {
                self.insert_left(node, value);
                self.validate_search_cache(offset);
            } else if node_start_offset + node.piece.len > offset {
                self.insert_middle(node, value);
            } else {
                // original node
                self.insert_right(node, value.to_string());
            }
        }

        self.compute_buffer_metadata();
    }

    fn append(&mut self, mut node: Node, mut value: String) {
        if self.adjust_cr_from_next(node.clone(), &mut value) {
            value.push_str("\n");
        }

        self.changed().value.push_str(&value);

        let start_offset = node.total_len();
        let mut line_starts = Buffer::get_line_starts(&value);
        for line_start in line_starts.iter_mut() {
            *line_start += start_offset;
        }

        let hit_crlf = self.info.should_check_crlf()
            && Node::start_with_lf_from_string(&mut value)
            && self.end_with_cr(&node);
        if hit_crlf {
            let buffer = self.get_buffer(&node);
            let prev_start_offset = buffer.line_starts[buffer.line_starts.len() - 2];
            if node.is_original() {
                self.original.line_starts.pop();
            } else {
                self.changed_mut().line_starts.pop();
            }

            self.cache.last_change = BufferCursor::new(
                self.cache.last_change.line - 1,
                start_offset - prev_start_offset,
            );
        }
        line_starts.remove(0);
        let mut buffer = self.get_buffer(&node);
        buffer.line_starts.extend_from_slice(&line_starts[1..]);

        self.original
            .line_starts
            .extend_from_slice(&line_starts[1..]);

        let end_index = self.original.line_starts.len() as i32 - 1;
        let end_column = self.original.value.graphemes(true).count() as i32
            - self
                .original
                .line_starts
                .get(end_index as usize)
                .expect("end index is out of line starts index");
        let new_end = BufferCursor::new(end_index, end_column);
        let old_line_feed_count = node.piece.line_feed_count;
        let new_line_feed_count = buffer.get_line_feed_count(&node.piece.start, &new_end);
        let value_len = value.graphemes(true).count() as i32;

        node.piece = Piece::new(
            node.piece.start,
            new_end,
            new_line_feed_count,
            node.piece.len + value_len,
        );

        self.cache.last_change = new_end;
        self.update_tree_metadata(&node, value_len, new_line_feed_count - old_line_feed_count);
    }

    fn insert_left(&mut self, _node: Node, _value: String) {
        todo!("insert_middle");
    }

    fn insert_middle(&mut self, _node: Node, _value: String) {
        todo!("insert_middle");
    }

    fn insert_right(&mut self, node: Node, mut value: String) {
        if self.adjust_cr_from_next(node, &mut value) {
            value += "\n";
        }

        todo!("insert_right");

        // let piece = self.new_node(&value);
        // const newNode = this.rbInsertRight(node, newPieces[0]);
        // let tmpNode = newNode;

        // for (let k = 1; k < newPieces.length; k++) {
        // 	tmpNode = this.rbInsertRight(tmpNode, newPieces[k]);
        // }

        // self.validate_crlf_with_prev_node(newNode);
    }

    fn get_node_position(&mut self, mut offset: i32) -> NodePosition {
        let cache = self.cache.search.get_position(offset);
        if let Some(cache) = cache {
            NodePosition::new(
                cache.node.clone(),
                cache.node_start_offset,
                offset - cache.node_start_offset,
            );
        }

        let mut node_start_offset = 0;
        let mut res = None;
        let mut cursor = self.tree.front();

        while let Some(node) = cursor.get() {
            if node.left_len > offset {
                cursor.move_prev();
            } else if node.total_len() >= offset {
                node_start_offset += node.left_len;
                let position =
                    NodePosition::new(node.clone(), offset - node.left_len, node_start_offset);
                res = Some(position.clone());
                self.cache.search.set_position(position);
                break;
            } else {
                offset -= node.total_len();
                node_start_offset += node.total_len();
                cursor.move_next();
            }
        }

        res.expect("Tree must NOT be empty when calling node_at method")
    }

    pub fn delete(&self, _offset: i32, _count: i32) {
        todo!("delete");
    }

    fn add_node(&mut self, value: &String) {
        let (mut changed, mut start_offset) = match &self.changed {
            Some(b) => (b.clone(), b.value.graphemes(true).count() as i32),
            None => (Buffer::from(value.clone()), 0),
        };
        let line_starts = Buffer::get_line_starts(value);

        match changed.line_starts.last() {
            Some(changed_line_starts) => {
                if *changed_line_starts == start_offset as i32
                    && start_offset != 0
                    && Node::start_with_lf_from_string(value)
                    && Node::end_with_cr_from_string(value)
                {
                    self.cache.last_change.column += 1;

                    for (i, x) in line_starts.iter().enumerate() {
                        if i != 0 {
                            changed.line_starts.push(x + start_offset + 1);
                        }
                    }

                    start_offset += 1;
                    changed.value.push_str(&format!("_{}", value));
                }
            }
            None => {
                if start_offset != 0 {
                    for (i, x) in line_starts.iter().enumerate() {
                        if i != 0 {
                            changed.line_starts.push(x + start_offset);
                        }
                    }
                }
                changed.value.push_str(&value);
            }
        }

        let node = Node::from_changed_buffer(&changed, start_offset, self.cache.last_change);
        self.changed = Some(changed);
        self.recompute_tree_metadata(&node);
        self.tree.insert(Box::new(node.clone()));

        self.cache.last_change = node.piece.end;
    }

    fn adjust_cr_from_next(&mut self, node: Node, value: &mut String) -> bool {
        if !(self.info.should_check_crlf() && Node::end_with_cr_from_string(value)) {
            return false;
        }

        let buffer = self.get_buffer(&node);
        let mut cursor = self.tree.find_mut(&node.total_len());
        match cursor.as_cursor().get() {
            Some(node) => {
                cursor.as_cursor().move_next();
                if node.start_with_lf(&buffer) {
                    value.push_str("\n");

                    if node.piece.len == 1 {
                        cursor.remove();
                    } else {
                        match cursor.as_cursor().get() {
                            Some(node) => {
                                let mut node = node.clone();
                                let start = BufferCursor::new(node.piece.start.line + 1, 0);
                                node.piece = Piece::new(
                                    start,
                                    node.piece.end,
                                    buffer.get_line_feed_count(&start, &node.piece.end),
                                    node.piece.len - 1,
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
        let cursor = self.tree.find(&node.total_len());
        if cursor.is_null() || node.piece.line_feed_count == 0 {
            return false;
        }

        let node = cursor.get().expect("Cursor is null");
        if node.piece.line_feed_count < 1 {
            return false;
        }

        let buffer = self.get_buffer(node);
        match buffer.value.graphemes(true).last() {
            Some("\r") => true,
            Some(_) => false,
            None => false,
        }
    }

    fn get_buffer(&self, node: &Node) -> Buffer {
        if node.is_original() {
            self.original.clone()
        } else {
            self.changed()
        }
    }

    fn changed(&self) -> Buffer {
        self.changed
            .as_ref()
            .expect(CHANGE_NODE_DOES_NOT_EXIST)
            .clone()
    }

    fn changed_mut(&mut self) -> &mut Buffer {
        self.changed.as_mut().expect(CHANGE_NODE_DOES_NOT_EXIST)
    }

    // fn node_char_code_at(&self, node: Node, _offset: i32) -> Option<i32> {
    //     if node.piece.line_feed_count < 1 {
    //         return None;
    //     }
    //     // let buffer = this._buffers[node.piece.bufferIndex];
    //     todo!("node_char_code_at");
    //     // let buffer = self.changed[node.offset..];
    //     // let new_offset = self.get_offset_in_buffer(node.piece.bufferIndex, node.piece.start) + offset;
    //     // return buffer.buffer.charCodeAt(new_offset);
    // }

    fn compute_buffer_metadata(&mut self) {
        let mut cursor = self.tree.front();
        let mut line_feed_count = 0;
        let mut grapheme_len = 0;

        while let Some(node) = cursor.get() {
            line_feed_count += node.left_line_feed_count + node.piece.line_feed_count;
            grapheme_len += node.left_len + node.piece.len;
            cursor.move_next();
        }

        self.cache.line_count = line_feed_count;
        self.cache.len = grapheme_len;
        self.validate_search_cache(grapheme_len);
    }

    fn update_tree_metadata(&mut self, node: &Node, delta: i32, line_feed_count_delta: i32) {
        let mut cursor = self.tree.find_mut(&node.total_len());
        cursor.move_parent();
        while let Some(node) = cursor.get() {
            let mut node = node.clone();
            node.left_len += delta;
            node.left_line_feed_count += line_feed_count_delta;

            cursor
                .replace_with(Box::new(node))
                .expect("Failed to replace parent node meta data");
            cursor.move_parent()
        }
    }

    fn recompute_tree_metadata(&mut self, node: &Node) {}

    pub fn to_string(&self) -> String {
        let mut text = String::new();
        for node in self.tree.iter() {
            let s = self.get_node_content(node);
            text.push_str(&s);
        }
        text
    }

    fn get_node_content(&self, node: &Node) -> String {
        let node = self
            .tree
            .find(&node.total_len())
            .get()
            .expect("Cannot find node in tree");
        let buffer = self.get_buffer(node);
        let start_offset = buffer.offset(node.piece.start);
        let end_offset = buffer.offset(node.piece.end);

        let graphemes = &mut buffer.value.graphemes(true);
        let mut text = String::new();
        while let Some((i, g)) = graphemes.enumerate().next() {
            if start_offset <= i as i32 && i as i32 <= end_offset {
                text.push_str(g);
            }
        }

        text
    }

    fn validate_search_cache(&mut self, offset: i32) {
        self.cache.search.positions.retain(|p| {
            let cursor = self.tree.find(&p.node.total_len());
            !(cursor.peek_parent().is_null() || p.node_start_offset >= offset)
        });
        self.cache.search.line_positions.retain(|p| {
            let cursor = self.tree.find(&p.node.total_len());
            !(cursor.peek_parent().is_null() || p.node_start_offset >= offset)
        });
    }
}

#[derive(Debug, Clone)]
struct Buffer {
    value: String,
    line_starts: Vec<i32>,
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
    fn new(value: String, line_starts: Vec<i32>) -> Buffer {
        Buffer { value, line_starts }
    }

    fn offset(&self, cursor: BufferCursor) -> i32 {
        self.line_starts.get(cursor.line as usize).unwrap_or(&0) + cursor.column
    }

    fn get_line_starts(value: &String) -> Vec<i32> {
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

    fn get_line_feed_count(&self, start: &BufferCursor, end: &BufferCursor) -> i32 {
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

const UTF8_BOM: &str = "\u{feff}";
const CHANGE_NODE_DOES_NOT_EXIST: &str = "Change node doesn't exist";

#[derive(Default, Debug)]
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
    fn new(default_eol: DefaultEOL) -> TextBufferInfo {
        let eol = match default_eol {
            DefaultEOL::LF => EOL::LF,
            DefaultEOL::CRLF => EOL::CRLF,
        };

        TextBufferInfo {
            encoding: CharacterEncoding::Utf8,
            eol,
            ..Default::default()
        }
    }

    fn from_string(value: &String, default_eol: DefaultEOL) -> (TextBufferInfo, Vec<i32>) {
        let encoding = CharacterEncoding::from(value);
        let mut line_starts = vec![0];
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

    fn should_check_crlf(&self) -> bool {
        return !(self.eos_normalized && self.eol == EOL::LF);
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

#[derive(Debug)]
struct PieceTreeSearchCache {
    limit: i32,
    positions: Vec<NodePosition>,
    line_positions: Vec<NodeLinePosition>,
}

impl Default for PieceTreeSearchCache {
    fn default() -> PieceTreeSearchCache {
        PieceTreeSearchCache {
            limit: 1,
            positions: vec![],
            line_positions: vec![],
        }
    }
}

impl PieceTreeSearchCache {
    fn get_position(&self, offset: i32) -> Option<&NodePosition> {
        let mut res = None;
        for p in self.positions.iter().rev() {
            if p.node_start_offset <= offset && p.node_start_offset + p.node.piece.len >= offset {
                res = Some(p);
                break;
            }
        }

        res
    }

    // fn get_line_position(&self, line_number: i32) -> Option<&NodeLinePosition> {
    //     let mut res = None;
    //     for p in self.line_positions.iter().rev() {
    //         if p.node_start_line_number < line_number
    //             && p.node_start_line_number + p.node.piece.line_feed_count >= line_number
    //         {
    //             res = Some(p);
    //             break;
    //         }
    //     }

    //     res
    // }

    fn is_limit(&self) -> bool {
        self.positions.len() + self.line_positions.len() >= self.limit as usize
    }

    fn set_position(&mut self, position: NodePosition) {
        if self.is_limit() {
            self.positions.remove(0);
        }
        self.positions.push(position);
    }

    // fn set_line_position(&mut self, line_position: NodeLinePosition) {
    //     if self.is_limit() {
    //         self.line_positions.remove(0);
    //     }
    //     self.line_positions.push(line_position);
    // }
}

#[derive(Default, Debug)]
struct TextBufferCache {
    last_change: BufferCursor,
    last_visited: LineCache,
    search: PieceTreeSearchCache,
    line_count: i32,
    len: i32,
}

#[derive(Default, Debug)]
struct LineCache {
    line_number: i32,
    value: String,
}

impl LineCache {
    fn new(line_number: i32, value: String) -> LineCache {
        LineCache { line_number, value }
    }
}

#[derive(Debug, Clone)]
struct NodePosition {
    node: Node,
    remainder: i32,
    node_start_offset: i32,
}

impl NodePosition {
    fn new(node: Node, remainder: i32, node_start_offset: i32) -> NodePosition {
        NodePosition {
            node,
            remainder,
            node_start_offset,
        }
    }
}

#[derive(Debug)]
struct NodeLinePosition {
    node: Node,
    node_start_offset: i32,
    node_start_line_number: i32,
}

#[derive(Default, Debug, Clone)]
pub struct Node {
    link: RBTreeAtomicLink,

    piece: Piece,
    r#type: NodeType,

    left_len: i32,
    left_line_feed_count: i32,
}

impl Node {
    fn is_original(&self) -> bool {
        self.r#type == NodeType::Original
    }
}

#[derive(Debug, Clone, PartialEq)]
enum NodeType {
    Original,
    Changed,
}

impl Default for NodeType {
    fn default() -> NodeType {
        NodeType::Changed
    }
}

impl Node {
    fn from_original_buffer(buffer: &Buffer) -> Node {
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

    fn from_changed_buffer(buffer: &Buffer, start_offset: i32, start: BufferCursor) -> Node {
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

    fn total_len(&self) -> i32 {
        self.left_len + self.piece.len
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

    fn start_with_lf(&self, buffer: &Buffer) -> bool {
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

#[derive(Default, Debug, Clone)]
pub struct Piece {
    start: BufferCursor,
    end: BufferCursor,
    line_feed_count: i32,
    len: i32,
}

impl Piece {
    fn new(start: BufferCursor, end: BufferCursor, line_feed_count: i32, len: i32) -> Piece {
        Piece {
            start,
            end,
            line_feed_count,
            len,
        }
    }
}

intrusive_adapter!(pub NodeAdapter = Box<Node>: Node { link: RBTreeAtomicLink });
impl<'a> KeyAdapter<'a> for NodeAdapter {
    type Key = i32;
    fn get_key(&self, node: &'a Node) -> i32 {
        node.total_len()
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
    use crate::buffer::{DefaultEOL, TextBuffer};
    #[test]
    fn basic_insert_and_delete() {
        let mut buffer =
            TextBuffer::from_string("This is a document with some text.", DefaultEOL::LF);
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

    #[test]
    fn more_inserts() {
        let mut buffer = TextBuffer::new(DefaultEOL::LF);

        buffer.insert(0, "AAA");
        assert_eq!(buffer.to_string(), "AAA");
        // buffer.insert(0, "BBB");
        // assert_eq!(buffer.to_string(), "BBBAAA");
        // buffer.insert(6, "CCC");
        // assert_eq!(buffer.to_string(), "BBBAAACCC");
        // buffer.insert(5, "DDD");
        // assert_eq!(buffer.to_string(), "BBBAADDDACCC");
    }
}
