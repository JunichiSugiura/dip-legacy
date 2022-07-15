mod buffer;
mod cache;
mod info;
mod node;
mod piece;
mod plugin;
mod text_buffer;

pub use crate::document::plugin::DocumentPlugin;
use crate::document::{info::DefaultEOL, text_buffer::TextBuffer};
use bevy::ecs::prelude::*;
use std::fs;

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
