use std::fmt::Debug;

#[derive(Debug)]
pub struct OpenDocument {
    pub path: &'static str,
}

impl OpenDocument {
    pub fn new(path: &'static str) -> Self {
        Self { path }
    }
}

#[derive(Debug, Default)]
pub struct NewDocument;
