pub mod command;

use bevy::{
    app::{App, AppExit, CoreStage, Plugin},
    core::CorePlugin,
    ecs::{
        component::Component,
        event::{EventReader, EventWriter},
        query::{Changed, With},
        system::{Commands, Query},
    },
    input::keyboard::{KeyCode, KeyboardInput},
    log::{debug, LogPlugin},
};
use command::{CoreCommand, UICommand};
use leafwing_input_manager::prelude::*;
use std::fs;

pub struct DipCorePlugin;

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub enum ModeType {
    Normal,
    Insert,
    Command,
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Mode(pub ModeType);

impl Default for Mode {
    fn default() -> Self {
        Mode(ModeType::Normal)
    }
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug)]
enum Action {
    InsertMode,
    NormalMode,
}

impl Plugin for DipCorePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(LogPlugin::default())
            .add_plugin(InputManagerPlugin::<Action>::default())
            .add_plugin(CorePlugin::default())
            .add_startup_system(spawn_user)
            .add_system(handle_app_exit)
            .add_system(change_mode)
            .add_system(log_core_command)
            .add_system(log_keyboard_event_system)
            .add_startup_system(load_file)
            .add_system_to_stage(CoreStage::PostUpdate, send_mode_change);
    }
}

#[derive(Component)]
struct User;

fn spawn_user(mut commands: Commands) {
    commands
        .spawn()
        .insert(User)
        .insert_bundle(InputManagerBundle::<Action> {
            action_state: ActionState::default(),
            input_map: InputMap::new([
                (Action::InsertMode, KeyCode::I),
                (Action::NormalMode, KeyCode::Escape),
            ]),
        })
        .insert(Mode::default());
}

fn handle_app_exit(mut events: EventReader<CoreCommand>, mut exit: EventWriter<AppExit>) {
    for cmd in events.iter() {
        match cmd {
            CoreCommand::Exit => exit.send(AppExit),
            _ => {}
        }
    }
}

fn load_file() {
    let data = fs::read_to_string("./README.md").expect("Failed to read file");
    println!("############################################");
    println!("# ./README.md");
    println!("############################################\n");
    println!("{}", data);
}

fn log_keyboard_event_system(mut events: EventReader<KeyboardInput>) {
    for event in events.iter() {
        debug!("{:?}", event);
    }
}

fn log_core_command(mut events: EventReader<CoreCommand>, mut event: EventWriter<AppExit>) {
    for cmd in events.iter() {
        debug!("🧠 {:?}", cmd);

        match cmd {
            CoreCommand::Exit => event.send(AppExit),
            _ => {}
        }
    }
}

fn change_mode(mut query: Query<(&ActionState<Action>, &mut Mode), With<User>>) {
    let (action_state, mut mode) = query.single_mut();
    match mode.0 {
        ModeType::Normal => {
            if action_state.just_pressed(&Action::InsertMode) {
                mode.0 = ModeType::Insert;
            }
        }
        ModeType::Insert => {
            if action_state.just_pressed(&Action::NormalMode) {
                mode.0 = ModeType::Normal;
            }
        }
        ModeType::Command => {}
    }
}

fn send_mode_change(mut ui: EventWriter<UICommand>, query: Query<&Mode, Changed<Mode>>) {
    for mode in query.iter() {
        ui.send(UICommand::ModeChange(*mode));
    }
}
