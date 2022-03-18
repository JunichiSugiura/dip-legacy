use bevy::{
    app::{App, Plugin},
    log::info,
};
use dioxus::{bevy::prelude::*, prelude::*};
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
            .add_plugin(DioxusDesktopPlugin::<CoreCommand, UICommand>::new(root, ()))
            .insert_non_send_resource(config);
    }
}

fn root(cx: Scope) -> Element {
    let window = use_bevy_window::<CoreCommand, UICommand>(&cx);

    use_future(&cx, (), |_| {
        let mut rx = window.receiver();

        async move {
            while let Ok(cmd) = rx.recv().await {
                info!("🎨 {:?}", cmd);
            }
        }
    });

    cx.render(rsx! {
        div {
            h1 { "dip: Text Editor" },
            button {
                onclick: |_e| {
                    window.send(CoreCommand::Click).unwrap();
                },
                "Click",
            }
            button {
                onclick: |_e| {
                    window.send(CoreCommand::Exit).unwrap();
                },
                "Exit",
            }
        }
    })
}
