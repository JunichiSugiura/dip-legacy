use crate::document::{buffer::BufferCursor, node::Node};
use intrusive_collections::{intrusive_adapter, KeyAdapter, RBTreeAtomicLink};

#[derive(Default, Debug, Clone)]
pub struct Piece {
    pub start: BufferCursor,
    pub end: BufferCursor,
    pub line_feed_count: i32,
    pub len: i32,
}

impl Piece {
    pub fn new(start: BufferCursor, end: BufferCursor, line_feed_count: i32, len: i32) -> Piece {
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
