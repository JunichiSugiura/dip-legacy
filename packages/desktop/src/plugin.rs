use crate::components::root;
use bevy::app::{App, Plugin};
use dioxus::bevy::prelude::*;
use dip_core::{
    command::{CoreCommand, UICommand},
    DipCorePlugin,
};

pub struct DipDesktopPlugin;

impl Plugin for DipDesktopPlugin {
    fn build(&self, app: &mut App) {
        let mut config = DesktopConfig::default().with_default_icon();
        config.with_window(|w| w.with_title("dip"));

        app.add_plugin(DipCorePlugin)
            .add_plugin(DioxusDesktopPlugin::<CoreCommand, UICommand>::new(
                root::Root,
                (),
            ))
            .insert_non_send_resource(config);
    }
}
