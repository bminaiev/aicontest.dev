use std::sync::Arc;

use egui::Context;
use futures::{
    channel::mpsc::{self, UnboundedReceiver, UnboundedSender},
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

fn reconnect(url: String, sender: Arc<UnboundedSender<String>>, ctx: Arc<Context>) {
    log("Connection closed, reconnecting...");

    let ws = WebSocket::new(&url).unwrap();

    let onmessage_callback = Closure::wrap(Box::new({
        let sender = sender.clone();
        let ctx = ctx.clone();
        move |e: MessageEvent| match e.data().dyn_into::<js_sys::JsString>() {
            Ok(data) => {
                let message = data.to_string();
                log(&format!("Received message: {}", message));
                match sender.unbounded_send(message.into()) {
                    Ok(()) => {}
                    Err(err) => {
                        log(&format!("Error sending message: {err:?}"));
                    }
                }
                // This is too early
                ctx.request_repaint();
            }
            Err(err) => {
                log("Received non-string message: {err:?}");
            }
        }
    }) as Box<dyn FnMut(MessageEvent)>);

    let url = Arc::new(url);

    let onclose_callback = Closure::wrap(Box::new(move |_: CloseEvent| {
        // TODO: wait a bit before reconnecting
        reconnect((*url).clone(), sender.clone(), ctx.clone());
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

            reconnect(
                url.to_owned(),
                Arc::new(sender.clone()),
                Arc::new(ctx.clone()),
            );

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

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("{last_msg}"));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("eframe template");

            if !last_msg.is_empty() {
                let available_size = ui.available_size();
                log(&format!("Size: {available_size:?}"));
            }

            egui::warn_if_debug_build(ui);
        });
    }
}
