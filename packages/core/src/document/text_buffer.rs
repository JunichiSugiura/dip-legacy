use crate::document::{
    buffer::{Buffer, BufferCursor},
    cache::TextBufferCache,
    info::{DefaultEOL, TextBufferInfo},
    node::{Node, NodePosition},
    piece::{NodeAdapter, Piece},
};
use intrusive_collections::RBTree;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Default, Debug)]
pub struct TextBuffer {
    tree: RBTree<NodeAdapter>,
    info: TextBufferInfo,

    original: Buffer,
    changed: Option<Buffer>,

    cache: TextBufferCache,
}

impl TextBuffer {
    pub fn new(default_eol: DefaultEOL) -> TextBuffer {
        TextBuffer {
            info: TextBufferInfo::new(default_eol),
            ..Default::default()
        }
    }

    pub fn from_string(original: &str, default_eol: DefaultEOL) -> TextBuffer {
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
        self.tree.insert(Box::new(node.clone()));
        self.recompute_tree_metadata(&node);

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
            cursor.move_parent();
        }
    }

    fn recompute_tree_metadata(&mut self, node: &Node) {
        // let delta = 0;
        // let lf_delta = 0;
        // if (node === self.tree.root) {
        //     return;
        // }

        // // go upwards till the node whose left subtree is changed.
        // while (x !== tree.root && x === x.parent.right) {
        //     x = x.parent;
        // }

        // if (x === tree.root) {
        //     // well, it means we add a node to the end (inorder)
        //     return;
        // }

        // // x is the node whose right subtree is changed.
        // x = x.parent;

        // delta = calculateSize(x.left) - x.size_left;
        // lf_delta = calculateLF(x.left) - x.lf_left;
        // x.size_left += delta;
        // x.lf_left += lf_delta;

        // // go upwards till root. O(logN)
        // while (x !== tree.root && (delta !== 0 || lf_delta !== 0)) {
        //     if (x.parent.left === x) {
        //         x.parent.size_left += delta;
        //         x.parent.lf_left += lf_delta;
        //     }

        //     x = x.parent;
        // }

        // original
        // let mut cursor = self.tree.find(&node.total_len());
        // let current = cursor.get().expect("Node provided is null");
        // if self.is_root(&current) {
        //     return;
        // }

        // while let Some(node) = cursor.peek_parent().peek_right().get() {
        //     if current.total_len() == node.total_len() {
        //         cursor.move_parent();
        //     }
        // }

        // if self.is_root(current) {
        //     return;
        // }

        // cursor.move_parent();

        // let left = cursor.peek_left().get();
        // if let Some(node) = cursor.get() {
        //     let delta = self.calculate_size(left) - node.left_len;
        //     let line_feed_count_delta = self.calculate_line_feed_count(left) - node.left_len;

        //     let mut new_node = node.clone();
        //     new_node.left_len += delta;
        //     new_node.left_line_feed_count += line_feed_count_delta;

        //     {
        //         let mut cursor = self.tree.find_mut(&node.total_len());
        //         cursor
        //             .replace_with(Box::new(new_node))
        //             .expect("Faild to replace with new node");
        //     }

        //     if self.is_root(node) || delta == 0 || line_feed_count_delta == 0 {
        //         return;
        //     }

        //     while let Some(parent) = cursor.peek_parent().get() {
        //         cursor.move_parent();
        //         if let Some(parent_left) = cursor.peek_parent().peek_left().get() {
        //             if parent.total_len() == parent_left.total_len() {
        //                 let mut new_node = parent.clone();
        //                 new_node.left_len += delta;
        //                 new_node.left_line_feed_count += line_feed_count_delta;

        //                 let mut cursor = self.tree.find_mut(&parent.total_len());
        //                 cursor.replace_with(Box::new(new_node));
        //             }
        //         }
        //     }
        // }
    }

    fn is_root(&self, node: &Node) -> bool {
        let root = self.tree.front().get().expect("Tree is empty");
        root.total_len() == node.total_len()
    }

    fn calculate_size(&self, node: Option<&Node>) -> i32 {
        match node {
            Some(node) => {
                let cursor = self.tree.find(&node.total_len());
                node.left_len + node.piece.len + self.calculate_size(cursor.peek_right().get())
            }
            None => 0,
        }
    }

    fn calculate_line_feed_count(&self, node: Option<&Node>) -> i32 {
        match node {
            Some(node) => {
                let cursor = self.tree.find(&node.total_len());
                node.left_line_feed_count
                    + node.piece.line_feed_count
                    + self.calculate_size(cursor.peek_right().get())
            }
            None => 0,
        }
    }

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

const CHANGE_NODE_DOES_NOT_EXIST: &str = "Change node doesn't exist";

#[cfg(test)]
mod inserts_and_deletes {
    use crate::document::{info::DefaultEOL, text_buffer::TextBuffer};

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
