use std::fmt::Debug;

#[derive(Debug)]
pub struct NewDocumentWithPath {
    pub path: String,
}

impl NewDocumentWithPath {
    pub fn new(path: String) -> Self {
        Self { path }
    }
}

#[derive(Debug)]
pub struct NewDocument {
    pub data: String,
}

impl NewDocument {
    pub fn new(data: String) -> Self {
        Self { data }
    }
}
