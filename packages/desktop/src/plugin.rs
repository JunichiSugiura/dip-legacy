use crate::components::root;
use bevy::{
    app::{App, Plugin},
    window::WindowDescriptor,
};
use bevy_dioxus::desktop::prelude::*;
use dip_core::{
    command::{CoreCommand, UICommand},
    DipCorePlugin,
};

pub struct DipDesktopPlugin;

impl Plugin for DipDesktopPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(DipCorePlugin)
            .insert_resource(WindowDescriptor {
                title: "dip".to_string(),
                ..Default::default()
            })
            .add_plugin(DioxusPlugin::<EmptyGlobalState, CoreCommand, UICommand>::new(root::Root));
    }
}
