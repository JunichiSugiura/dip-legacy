use crate::{
    buffer::{DefaultEOL, TextBuffer},
    command::UICommand,
    event::*,
};
use bevy::{app::prelude::*, ecs::prelude::*};

#[derive(Default)]
pub struct DocumentPlugin;

impl Plugin for DocumentPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NewDocument>()
            .add_event::<OpenDocument>()
            .add_event::<DocumentInsert>()
            .add_stage_after(
                CoreStage::Update,
                DipStage::Notify,
                SystemStage::single_threaded(),
            )
            .add_startup_system(debug_setup)
            .add_system(new_document)
            .add_system(open_document)
            // .add_system(insert)
            .add_system_to_stage(DipStage::Notify, send_document_added);
    }
}

fn debug_setup(mut new_doc: EventWriter<OpenDocument>) {
    new_doc.send(OpenDocument::new("./README.md".into()));
}

// fn insert(mut events: EventReader<DocumentInsert>, q: Query<(Entity, &TextBuffer)>) {
//     for e in events.iter() {
//         for (id, b) in q.iter() {
//             if id == e.entity {
//                 b.insert(e.offset, e.text);
//             }
//         }
//     }
// }

fn new_document(mut events: EventReader<NewDocument>, mut commands: Commands) {
    for _ in events.iter() {
        let buffer = TextBuffer::default();
        commands.spawn().insert(buffer);
    }
}

fn open_document(mut events: EventReader<OpenDocument>, mut commands: Commands) {
    for e in events.iter() {
        let buffer = TextBuffer::new(e.path, DefaultEOL::LF);
        commands.spawn().insert(buffer);
    }
}

fn send_document_added(q: Query<&TextBuffer, Added<TextBuffer>>, mut ui: EventWriter<UICommand>) {
    for _text_buffer in q.iter() {
        ui.send(UICommand::DocumentAdded);
    }
}
