pub mod command;

use bevy::{
    app::{App, AppExit, Plugin},
    ecs::event::{EventReader, EventWriter},
    input::keyboard::KeyboardInput,
    log::{info, LogPlugin},
};
use command::CoreCommand;
use std::fs;

pub struct DipCorePlugin;

#[derive(Copy, Clone, Debug, PartialEq, Hash, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Command,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Normal
    }
}

impl Plugin for DipCorePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(LogPlugin)
            .init_resource::<Mode>()
            .add_system(print_keyboard_event_system)
            .add_system(log_core_command)
            .add_startup_system(load_file);
    }
}

fn load_file() {
    let data = fs::read_to_string("./README.md").expect("Failed to read file");
    println!("############################################");
    println!("# ./README.md");
    println!("############################################\n");
    println!("{}", data);
}

fn print_keyboard_event_system(mut keyboard_input_events: EventReader<KeyboardInput>) {
    for event in keyboard_input_events.iter() {
        info!("{:?}", event);
    }
}

fn log_core_command(mut events: EventReader<CoreCommand>, mut event: EventWriter<AppExit>) {
    for cmd in events.iter() {
        info!("🧠 {:?}", cmd);

        match cmd {
            CoreCommand::Exit => event.send(AppExit),
            _ => {}
        }
    }
}
