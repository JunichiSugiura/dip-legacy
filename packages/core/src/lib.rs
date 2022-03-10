pub mod command;

use bevy_app::{App, AppExit, Plugin};
use bevy_ecs::event::{EventReader, EventWriter};
use bevy_log::{info, LogPlugin};
use command::{CoreCommand, UICommand};

pub struct DipCorePlugin;

impl Plugin for DipCorePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(LogPlugin)
            .add_startup_system(test_ui_command)
            .add_system(log_core_command);
    }
}

fn test_ui_command(mut event: EventWriter<UICommand>) {
    event.send(UICommand::Test);
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
