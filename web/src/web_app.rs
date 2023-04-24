use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
};

use egui::{pos2, vec2, Align2, Context, FontId, Pos2, RichText, Rounding, Shape, Stroke};
use futures::{
    channel::mpsc::{self, UnboundedReceiver, UnboundedSender},
    SinkExt, StreamExt,
};

use game_common::{
    game_state::{GameState, Player},
    point::Point,
};
use instant::{Duration, Instant, SystemTime};
use wasm_bindgen::{
    prelude::{wasm_bindgen, Closure},
    JsCast,
};
use wasm_bindgen_futures::{spawn_local, JsFuture};

pub struct TemplateApp {
    // Example stuff:
    label: String,

    value: f32,
    receiver: UnboundedReceiver<StateWithTime>,
    state_approximator: StateApproximator,

    counter: u64,
    start: Instant,
    updates_got: u64,
}

use gloo_timers::future::TimeoutFuture;
use web_sys::{CloseEvent, MessageEvent, WebSocket};

use crate::state_approximator::{StateApproximator, StateWithTime};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

fn reconnect(url: String, sender: Arc<UnboundedSender<StateWithTime>>, ctx: Arc<Context>) {
    log("Connection closed, reconnecting...");

    let ws = WebSocket::new(&url).unwrap();

    let onmessage_callback = Closure::wrap(Box::new({
        let sender = sender.clone();
        let ctx = ctx.clone();
        move |e: MessageEvent| match e.data().dyn_into::<js_sys::JsString>() {
            Ok(data) => {
                let message: String = data.to_string().into();
                match GameState::from_string(&message) {
                    Ok(state) => {
                        let state = StateWithTime {
                            state,
                            timestamp: SystemTime::now(),
                        };
                        match sender.unbounded_send(state) {
                            Ok(()) => {}
                            Err(err) => {
                                log(&format!("Error sending message: {err:?}"));
                            }
                        }
                    }
                    Err(err) => log(&format!("Error parsing state: {err:?}")),
                }
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
        let (sender, receiver) = mpsc::unbounded::<StateWithTime>();

        let ctx = cc.egui_ctx.clone();

        spawn_local(async move {
            let url = "ws://192.168.1.162:7878";

            reconnect(
                url.to_owned(),
                Arc::new(sender.clone()),
                Arc::new(ctx.clone()),
            );
        });

        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
            receiver,
            state_approximator: StateApproximator::default(),
            counter: 0,
            start: Instant::now(),
            updates_got: 0,
        }
    }
}

impl eframe::App for TemplateApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.counter += 1;
        ctx.request_repaint();
        // ctx.request_repaint_after(Duration::from_millis(1000 / 60));
        while let Ok(Some(state)) = self.receiver.try_next() {
            self.state_approximator.add_state(state);
            self.updates_got += 1;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut cur_turn = 0;
            let mut max_turns = 0;
            if let Some(game_state) = self.state_approximator.get_state() {
                draw_state(ui, &game_state);
                cur_turn = game_state.turn;
                max_turns = game_state.max_turns;
            }
            // TODO: show real fps
            let info = format!(
                "iter={iter}\nfps={fps:.3}\nturn={cur_turn}/{max_turns}\n",
                iter = self.counter,
                fps = self.counter as f64 / self.start.elapsed().as_secs_f64(),
            );

            ui.label(RichText::new(info).font(FontId::proportional(25.0)));

            egui::warn_if_debug_build(ui);
        });

        // egui::CentralPanel::default().show(&ctx, |ui| {
        //     ui.label(format!(
        //         "===================================\n{}, fps={fps:.3}, upds={updates}\n",
        //         self.counter,
        //         fps = self.counter as f64 / self.start.elapsed().as_secs_f64(),
        //         updates = self.updates_got,
        //         // last_msg = self.last_msg
        //     ));
        // });

        // egui::SidePanel::left("side_panel").show(ctx, |ui| {
        //     ui.horizontal(|ui| {

        //     });
        // });
    }
}

fn draw_state(ui: &mut egui::Ui, game_state: &GameState) {
    let available_rect = ui.available_rect_before_wrap();
    let zoom_x = available_rect.width() / game_state.width as f32;
    let zoom_y = available_rect.height() / game_state.height as f32;
    let zoom = if zoom_x < zoom_y { zoom_x } else { zoom_y };
    let conv = |p: Pos2| -> Pos2 { available_rect.min + p.to_vec2() * zoom };
    let conv_pt = |p: Point| -> Pos2 { conv(pos2(p.x as f32, p.y as f32)) };
    {
        // background
        let background_color = egui::Color32::from_rgb(240, 240, 240);
        ui.painter().rect_filled(
            egui::Rect::from_min_size(
                conv(pos2(0.0, 0.0)),
                egui::vec2(
                    game_state.width as f32 * zoom,
                    game_state.height as f32 * zoom,
                ),
            ),
            Rounding::default(),
            background_color,
        );
    }
    {
        // draw items
        for (id, item) in game_state.items.iter().enumerate() {
            let color = egui::Color32::LIGHT_BLUE;
            let center = conv_pt(item.pos);
            ui.painter()
                .circle_filled(center, item.radius as f32 * zoom, color);
            // draw item id
            // ui.painter().text(
            //     center,
            //     Align2::CENTER_CENTER,
            //     id.to_string(),
            //     FontId::monospace(15.0),
            //     egui::Color32::BLACK,
            // );
        }
    }
    {
        // draw players
        for player in game_state.players.iter() {
            let color = choose_player_color(player);
            let center = conv_pt(player.pos);
            ui.painter()
                .circle_filled(center, player.radius as f32 * zoom, color);
            draw_arrow(ui, center, conv_pt(player.target), color);
            // draw player id
            ui.painter().text(
                center,
                Align2::LEFT_CENTER,
                format!("{} [score={}]", player.name, player.score),
                FontId::monospace(10.0),
                egui::Color32::BLACK,
            );
        }
    }
}

fn choose_player_color(player: &Player) -> egui::Color32 {
    let hash = {
        let mut hasher = DefaultHasher::default();
        player.name.hash(&mut hasher);
        hasher.finish()
    };
    let r = (hash >> 16) as u8;
    let g = (hash >> 8) as u8;
    let b = hash as u8;
    egui::Color32::from_rgb(r, g, b)
}

fn draw_arrow(ui: &mut egui::Ui, from: Pos2, to: Pos2, color: egui::Color32) {
    let dir = to - from;
    let len = dir.length();
    let dir = dir / len;
    let arrow_len = 10.0;
    let arrow_width = 5.0;
    let arrow_start = to - dir * arrow_len;
    let arrow_dir = vec2(dir.y, -dir.x);
    let arrow_points = vec![
        arrow_start + arrow_dir * arrow_width,
        to,
        arrow_start - arrow_dir * arrow_width,
    ];
    ui.painter()
        .line_segment([from, to], Stroke::new(2.0, color));
    ui.painter()
        .add(Shape::line(arrow_points, Stroke::new(2.0, color)));
}
