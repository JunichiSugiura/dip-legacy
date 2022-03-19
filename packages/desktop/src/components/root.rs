use crate::components::status_bar;
use bevy::log::info;
use dioxus::{bevy::prelude::*, prelude::*};
use dip_core::command::{CoreCommand, UICommand};

pub fn Root(cx: Scope) -> Element {
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
                    window.send(CoreCommand::Exit).unwrap();
                },
                "Exit",
            }
            status_bar::StatusBar {}
        }
    })
}
