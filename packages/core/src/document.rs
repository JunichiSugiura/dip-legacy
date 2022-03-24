use crate::{command::UICommand, event::{NewDocument, OpenDocument}, buffer::TextBuffer};
use bevy::{app::prelude::*, ecs::prelude::*};
use std::fs;

#[derive(Default)]
pub struct DocumentPlugin;

static HANDLE_NEW_DOCUMENT: &str = "handle_new_document";
static CHANGE_DETECTION: &str = "change_detection";

impl Plugin for DocumentPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NewDocument>()
            .add_event::<OpenDocument>()
            .add_stage_after(
                CoreStage::Update,
                CHANGE_DETECTION,
                SystemStage::single_threaded(),
            )
            .add_startup_system(setup)
            .add_system(new_document.label(HANDLE_NEW_DOCUMENT))
            .add_system_to_stage(CHANGE_DETECTION, send_document_added)
            .add_system(open_document.before(HANDLE_NEW_DOCUMENT));
    }
}

fn setup(mut new_doc: EventWriter<OpenDocument>) {
    new_doc.send(OpenDocument::new("./README.md".into()));
}

fn new_document(mut events: EventReader<NewDocument>, mut commands: Commands) {
    for e in events.iter() {
        let text_buffer = TextBuffer::new(e.data.clone());
        commands
            .spawn()
            .insert(text_buffer);
    }
}

fn send_document_added(q: Query<&TextBuffer, Added<TextBuffer>>, mut ui: EventWriter<UICommand>) {
    for _text_buffer in q.iter() {
        ui.send(UICommand::DocumentAdded);
    }
}

fn open_document(
    mut events: EventReader<OpenDocument>,
    mut new_doc: EventWriter<NewDocument>,
) {
    for e in events.iter() {
        let bytes = fs::read(e.path.clone()).expect("Failed to read file");
        new_doc.send(NewDocument::new(bytes));
    }
}
