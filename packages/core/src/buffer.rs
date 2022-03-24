use bevy::ecs::prelude::*;
use intrusive_collections::intrusive_adapter;
use intrusive_collections::{rbtree::AtomicLink, KeyAdapter, RBTree};
use std::{convert::From, fs};

#[derive(Component, Default, Debug)]
pub struct TextBuffer {
    file_path: Option<&'static str>,
    pub tree: RBTree<PieceAdapter>,
    original: Vec<u8>,
    encoding: CharacterEncoding,
    info: TextBufferInfo,
}

impl From<&'static str> for TextBuffer {
    fn from(file_path: &'static str) -> TextBuffer {
        let original = fs::read(file_path.clone()).expect("Failed to read file");
        if original.is_empty() {
            return TextBuffer::default();
        }

        let bytes = original.as_slice();
        let encoding = CharacterEncoding::from(bytes);
        let info = TextBufferInfo::new(bytes);

        let mut tree = RBTree::new(PieceAdapter::new());
        let piece = Piece::new(
            0,
            BufferCursor::new(0, 0),
            BufferCursor::new(
                info.line_starts.len() as i32 - 1,
                match info.line_starts.last() {
                    Some(x) => original.len() as i32 - x,
                    None => 0,
                },
            ),
            bytes.len() as i32,
            info.line_starts.len() as i32 - 1,
        );
        tree.insert(Box::new(piece));

        TextBuffer {
            file_path: Some(file_path),
            tree,
            original,
            encoding,
            info,
        }
    }
}

impl TextBuffer {
    pub fn insert(offset: i32, text: &str) {
        todo!();
    }

    pub fn delete(offset: i32, count: i32) {
        todo!();
    }
}

const UTF8_BOM: &str = "\u{feff}";

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

impl From<&[u8]> for CharacterEncoding {
    fn from(s: &[u8]) -> Self {
        if s.starts_with(UTF8_BOM.as_bytes()) {
            CharacterEncoding::Utf8WithBom
        } else {
            CharacterEncoding::Utf8
        }
    }
}

#[derive(Debug)]
pub struct TextBufferInfo {
    line_starts: Vec<i32>,
    line_break_count: LineBreakCount,
    // is_basic_ascii: bool,
    // contains_rtl: bool,
    // contains_unusual_line_terminators: bool,
    // is_basic_ascii: bool,
    // normalize_eol: bool,
}

impl Default for TextBufferInfo {
    fn default() -> TextBufferInfo {
        TextBufferInfo {
            line_starts: vec![0],
            line_break_count: Default::default(),
        }
    }
}

impl TextBufferInfo {
    fn new(bytes: &[u8]) -> TextBufferInfo {
        let mut line_starts = TextBufferInfo::default();

        let mut enumerate = bytes.iter().enumerate();
        while let Some((i, c)) = enumerate.next() {
            match *c as char {
                '\r' => match enumerate.nth(i + 1) {
                    Some((_, c)) => match *c as char {
                        '\r' => {
                            line_starts.line_starts.push(i as i32 + 2);
                            line_starts.line_break_count.crlf += 1;
                        }
                        _ => {
                            line_starts.line_starts.push(i as i32 + 1);
                            line_starts.line_break_count.cr += 1;
                        }
                    },
                    None => {}
                },
                '\n' => {
                    line_starts.line_starts.push(i as i32 + 1);
                    line_starts.line_break_count.lf += 1;
                }
                _ => {}
            }
        }

        line_starts
    }
}

#[derive(Debug, Default)]
struct LineBreakCount {
    cr: i32,
    lf: i32,
    crlf: i32,
}

#[derive(Default, Debug)]
pub struct Piece {
    link: AtomicLink,
    offset: i32,
    start: BufferCursor,
    end: BufferCursor,
    length: i32,
    line_feed_count: i32,
}

intrusive_adapter!(pub PieceAdapter = Box<Piece>: Piece { link: AtomicLink });
impl<'a> KeyAdapter<'a> for PieceAdapter {
    type Key = i32;
    fn get_key(&self, e: &'a Piece) -> i32 {
        e.offset
    }
}

impl Piece {
    pub fn new(offset: i32, start: BufferCursor, end: BufferCursor, length: i32, line_feed_count: i32) -> Self {
        Self {
            offset,
            start,
            end,
            length,
            line_feed_count,
            ..Default::default()
        }
    }
}

#[derive(Default, Debug)]
pub struct BufferCursor {
    line: i32,
    column: i32,
}

impl BufferCursor {
    fn new(line: i32, column: i32) -> Self {
        Self { line, column }
    }
}
