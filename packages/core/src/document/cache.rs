use crate::document::{
    buffer::BufferCursor,
    node::{NodeLinePosition, NodePosition},
};

#[derive(Debug)]
pub struct PieceTreeSearchCache {
    limit: i32,
    pub positions: Vec<NodePosition>,
    pub line_positions: Vec<NodeLinePosition>,
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
    pub fn get_position(&self, offset: i32) -> Option<&NodePosition> {
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

    pub fn set_position(&mut self, position: NodePosition) {
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
pub struct TextBufferCache {
    pub last_change: BufferCursor,
    last_visited: LineCache,
    pub search: PieceTreeSearchCache,
    pub line_count: i32,
    pub len: i32,
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
