use bevy::{
    app::{App, Plugin},
    ecs::{component::Component, system::Commands},
};
use intrusive_collections::intrusive_adapter;
use intrusive_collections::{rbtree::AtomicLink, Bound, KeyAdapter, RBTree};
use std::fs;

struct Element {
    link: AtomicLink,
    value: i32,
}

intrusive_adapter!(ElementAdapter = Box<Element>: Element { link: AtomicLink });
impl<'a> KeyAdapter<'a> for ElementAdapter {
    type Key = i32;
    fn get_key(&self, e: &'a Element) -> i32 {
        e.value
    }
}

#[derive(Default)]
pub struct DocumentPlugin;

impl Plugin for DocumentPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(load_file);
    }
}

#[derive(Component)]
struct TextBuffer(RBTree<ElementAdapter>);

fn load_file(mut commands: Commands) {
    let data = fs::read_to_string("./README.md").expect("Failed to read file");
    println!("############################################");
    println!("# ./README.md");
    println!("############################################\n");
    println!("{}", data);

    let mut tree = RBTree::new(ElementAdapter::new());

    commands.spawn().insert(TextBuffer(tree));
}
