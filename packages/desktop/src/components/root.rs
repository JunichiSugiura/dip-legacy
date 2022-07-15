use crate::components::status_bar;
use bevy::log::info;
use bevy_dioxus::desktop::prelude::*;
use dioxus::prelude::*;
use dip_core::command::{CoreCommand, UICommand};

pub fn Root(cx: Scope) -> Element {
    let window = use_window::<CoreCommand, UICommand>(&cx);

    use_future(&cx, (), |_| {
        let rx = window.receiver();

        async move {
            while let Some(cmd) = rx.receive().await {
                info!("🎨 {:?}", cmd);
            }
        }
    });

    cx.render(rsx! {
        div {
            h1 { "dip: Text Editor" },
            button {
                onclick: |_e| {
                    window.send(CoreCommand::Exit);
                },
                "Exit",
            }
            status_bar::StatusBar {}
        }
    })
}
