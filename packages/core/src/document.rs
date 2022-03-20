use crate::event::{NewDocument, NewDocumentWithPath};
use bevy::{app::prelude::*, ecs::prelude::*, log::info};
use intrusive_collections::intrusive_adapter;
use intrusive_collections::{rbtree::AtomicLink, KeyAdapter, RBTree};
use std::fs;

struct Piece {
    link: AtomicLink,
    value: i32,
}

intrusive_adapter!(PieceAdapter = Box<Piece>: Piece { link: AtomicLink });
impl<'a> KeyAdapter<'a> for PieceAdapter {
    type Key = i32;
    fn get_key(&self, e: &'a Piece) -> i32 {
        e.value
    }
}

#[derive(Default)]
pub struct DocumentPlugin;

static HANDLE_NEW_DOCUMENT: &str = "handle_new_document";
static CHANGE_DETECTION: &str = "change_detection";

impl Plugin for DocumentPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NewDocument>()
            .add_event::<NewDocumentWithPath>()
            .add_stage_after(
                CoreStage::Update,
                CHANGE_DETECTION,
                SystemStage::single_threaded(),
            )
            .add_startup_system(setup)
            .add_system(handle_new_document.label(HANDLE_NEW_DOCUMENT))
            .add_system(handle_new_document_with_path.before(HANDLE_NEW_DOCUMENT))
            .add_system_to_stage(CHANGE_DETECTION, log_text_buffer);
    }
}

#[derive(Component)]
struct TextBuffer {
    tree: RBTree<PieceAdapter>,
}

impl TextBuffer {
    fn new(tree: RBTree<PieceAdapter>) -> Self {
        Self { tree }
    }
}

fn setup(mut new_doc: EventWriter<NewDocumentWithPath>) {
    new_doc.send(NewDocumentWithPath::new("./README.md".into()));
}

fn handle_new_document(mut events: EventReader<NewDocument>, mut commands: Commands) {
    for e in events.iter() {
        info!("\n{}", e.data);
        let tree = RBTree::new(PieceAdapter::new());

        commands.spawn().insert(TextBuffer::new(tree));
    }
}

fn handle_new_document_with_path(
    mut events: EventReader<NewDocumentWithPath>,
    mut new_doc: EventWriter<NewDocument>,
) {
    for e in events.iter() {
        let data = fs::read_to_string(e.path.clone()).expect("Failed to read file");
        new_doc.send(NewDocument::new(data));
    }
}

fn log_text_buffer(q: Query<&TextBuffer, Changed<TextBuffer>>) {
    for b in q.iter() {
        info!("tree: {}", b.tree.is_empty());
    }
}
