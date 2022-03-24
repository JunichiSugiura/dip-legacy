use crate::{command::UICommand, event::*, buffer::TextBuffer};
use bevy::{app::prelude::*, ecs::prelude::*};

#[derive(Default)]
pub struct DocumentPlugin;

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
            .add_system(new_document)
            .add_system(open_document)
            .add_system_to_stage(CHANGE_DETECTION, send_document_added);
    }
}

fn setup(mut new_doc: EventWriter<OpenDocument>) {
    new_doc.send(OpenDocument::new("./README.md".into()));
}

fn new_document(
    mut events: EventReader<NewDocument>,
    mut commands: Commands,
) {
    for _ in events.iter() {
        let buffer = TextBuffer::default();
        commands
            .spawn()
            .insert(buffer);
    }
}

fn open_document(
    mut events: EventReader<OpenDocument>,
    mut commands: Commands
) {
    for e in events.iter() {
        let buffer = TextBuffer::from(e.path.as_str());
        commands
            .spawn()
            .insert(buffer);
    }
}

fn send_document_added(q: Query<&TextBuffer, Added<TextBuffer>>, mut ui: EventWriter<UICommand>) {
    for _text_buffer in q.iter() {
        ui.send(UICommand::DocumentAdded);
    }
}
