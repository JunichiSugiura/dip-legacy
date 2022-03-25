use bevy::ecs::prelude::*;
use std::fmt::Debug;

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum DipStage {
    Notify,
}

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

#[derive(Debug)]
pub struct DocumentInsert {
    pub entity: Entity,
    pub offset: i32,
    pub text: &'static str,
}

impl DocumentInsert {
    pub fn new(entity: Entity, offset: i32, text: &'static str) -> DocumentInsert {
        DocumentInsert {
            entity,
            offset,
            text,
        }
    }
}
