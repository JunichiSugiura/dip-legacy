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

#[derive(Debug)]
pub struct NewDocument {
    pub data: Vec<u8>,
}

impl NewDocument {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}
