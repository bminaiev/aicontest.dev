use std::sync::Arc;

use egui::Context;
use futures::{
    channel::mpsc::{self, UnboundedReceiver},
    SinkExt, StreamExt,
};

use wasm_bindgen::{
    prelude::{wasm_bindgen, Closure},
    JsCast,
};
use wasm_bindgen_futures::{spawn_local, JsFuture};

pub struct TemplateApp {
    // Example stuff:
    label: String,

    value: f32,
    receiver: UnboundedReceiver<String>,
    last_msg: String,
}

use gloo_timers::future::TimeoutFuture;
use web_sys::{CloseEvent, MessageEvent, WebSocket};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

fn reconnect(url: String) {
    log("Connection closed, reconnecting...");

    let ws = WebSocket::new(&url).unwrap();

    let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
        match e.data().dyn_into::<js_sys::JsString>() {
            Ok(data) => {
                let message = data.to_string();
                log(&format!("Received message: {}", message));
            }
            Err(err) => {
                log("Received non-string message: {err:?}");
            }
        }
    }) as Box<dyn FnMut(MessageEvent)>);

    let url = Arc::new(url);

    let onclose_callback = Closure::wrap(Box::new(move |_: CloseEvent| {
        // TODO: wait a bit before reconnecting
        reconnect((*url).clone());
    }) as Box<dyn FnMut(CloseEvent)>);

    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));

    onmessage_callback.forget();
    onclose_callback.forget();
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        let (mut sender, receiver) = mpsc::unbounded::<String>();

        let ctx = cc.egui_ctx.clone();

        spawn_local(async move {
            let url = "ws://127.0.0.1:7878";
            // let url = "wss://echo.websocket.events";

            reconnect(url.to_owned());

            for x in 1.. {
                sender.send(format!("hello{x}")).await.unwrap();
                ctx.request_repaint();
                TimeoutFuture::new(1000).await;
            }
            // while let Some(message) = receiver.next().await {
            //     web_sys::console::log_1(&message.into());
            // }
        });

        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
            receiver,
            last_msg: "".to_owned(),
        }
    }
}

impl eframe::App for TemplateApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(Some(new_msg)) = self.receiver.try_next() {
            self.last_msg = new_msg;
        }

        let Self {
            label,
            value,
            receiver,
            last_msg,
        } = self;

        // Examples of how to create different panels and windows.
        // Pick whichever suits you.
        // Tip: a good default choice is to just keep the `CentralPanel`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("Side Panel");

            ui.horizontal(|ui| {
                ui.label(format!("Write something: {last_msg}"));
                ui.text_edit_singleline(label);
            });

            ui.add(egui::Slider::new(value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                *value += 1.0;
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label("powered by ");
                    ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                    ui.label(" and ");
                    ui.hyperlink_to(
                        "eframe",
                        "https://github.com/emilk/egui/tree/master/crates/eframe",
                    );
                    ui.label(".");
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's

            ui.heading("eframe template");
            ui.hyperlink("https://github.com/emilk/eframe_template");
            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/master/",
                "Source code."
            ));
            egui::warn_if_debug_build(ui);
        });

        if false {
            egui::Window::new("Window").show(ctx, |ui| {
                ui.label("Windows can be moved by dragging them.");
                ui.label("They are automatically sized based on contents.");
                ui.label("You can turn on resizing and scrolling if you like.");
                ui.label("You would normally choose either panels OR windows.");
            });
        }
    }
}
