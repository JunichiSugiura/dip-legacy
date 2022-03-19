use dioxus::prelude::*;
use dip_core::command::{CoreCommand, UICommand};
use dip_core::ModeType;

pub fn StatusBar(cx: Scope) -> Element {
    let window = use_bevy_window::<CoreCommand, UICommand>(&cx);
    let mode_type = use_state(&cx, || ModeType::Normal);

    use_future(&cx, (), |_| {
        let mut rx = window.receiver();
        let mode_type = mode_type.clone();

        async move {
            while let Ok(cmd) = rx.recv().await {
                match cmd {
                    UICommand::ModeChange(m) => {
                        *mode_type.make_mut() = m.0;
                    }
                }
            }
        }
    });

    cx.render(rsx! {
        div {
            div { [format_args!("Mode: {mode_type:?}")] }
        }
    })
}
