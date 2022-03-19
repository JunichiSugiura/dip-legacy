use bevy::app::App;
use dip_desktop::prelude::DipDesktopPlugin;

fn main() {
    App::new().add_plugin(DipDesktopPlugin).run();
}
