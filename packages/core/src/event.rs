use std::fmt::Debug;

#[derive(Debug)]
pub struct OpenDocument {
    pub path: String,
}


impl OpenDocument {
    pub fn new(path: String) -> Self {
        Self { path }
    }
}

#[derive(Debug, Default)]
pub struct NewDocument;
