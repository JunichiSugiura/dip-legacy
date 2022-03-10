use bevy_app::{App, Plugin};
use bevy_log::info;
use dioxus::{
    bevy::{use_bevy_context, DioxusDesktopPlugin},
    desktop::DesktopConfig,
    prelude::*,
};
use dip_core::{
    command::{CoreCommand, UICommand},
    DipCorePlugin,
};

pub struct DipDesktopPlugin;

impl Plugin for DipDesktopPlugin {
    fn build(&self, app: &mut App) {
        let mut config = DesktopConfig::default().with_default_icon();
        config.with_window(|w| w.with_title("dip"));

        let desktop = DioxusDesktopPlugin::<CoreCommand, UICommand>::new(root, ());

        app.add_plugin(DipCorePlugin)
            .add_plugin(desktop)
            .insert_non_send_resource(config);
    }
}

fn root(cx: Scope) -> Element {
    let ctx = use_bevy_context::<CoreCommand, UICommand>(&cx);

    use_future(&cx, (), |_| {
        let mut rx = ctx.receiver();
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
                    let _res = ctx.send(CoreCommand::Click);
                },
                "Click",
            }
            button {
                onclick: |_e| {
                    let _res = ctx.send(CoreCommand::Exit);
                },
                "Exit",
            }
        }
    })
}
