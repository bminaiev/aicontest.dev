use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
};

use eframe::epaint::ahash::HashMap;
use egui::{pos2, vec2, Align2, Color32, Context, FontId, Pos2, RichText, Rounding, Shape, Stroke};
use egui_extras::{Column, TableBuilder};
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};

use game_common::{
    game_state::{GameState, Player},
    point::Point,
};
use instant::SystemTime;
use poll_promise::Promise;
use wasm_bindgen::{
    prelude::{wasm_bindgen, Closure},
    JsCast,
};
use wasm_bindgen_futures::spawn_local;

#[derive(PartialEq, Eq)]
enum SortBy {
    Score,
    Name,
}

pub struct App {
    receiver: UnboundedReceiver<Option<StateWithTime>>,
    state_approximator: StateApproximator,
    fps_counter: FpsCounter,
    show_users: HashMap<String, bool>,
    show_top5: bool,
    sort_players_by: SortBy,
    server_url: String,
    connected: bool,
    top_results_promise: Promise<Vec<Player>>,
}

use web_sys::{CloseEvent, MessageEvent, WebSocket};

use crate::{
    fps_counter::FpsCounter,
    state_approximator::{StateApproximator, StateWithTime},
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

fn reconnect(url: String, sender: Arc<UnboundedSender<Option<StateWithTime>>>, ctx: Arc<Context>) {
    log("Connection closed, reconnecting...");

    let ws = WebSocket::new(&url).unwrap();

    let onmessage_callback = Closure::wrap(Box::new({
        let sender = sender.clone();
        move |e: MessageEvent| match e.data().dyn_into::<js_sys::JsString>() {
            Ok(data) => {
                let message: String = data.to_string().into();
                match GameState::from_string(&message) {
                    Ok(state) => {
                        let state = StateWithTime {
                            state,
                            timestamp: SystemTime::now(),
                        };
                        match sender.unbounded_send(Some(state)) {
                            Ok(()) => {}
                            Err(err) => {
                                log(&format!("Error sending message: {err:?}"));
                            }
                        }
                    }
                    Err(err) => log(&format!("Error parsing state: {err:?}")),
                }
            }
            Err(_err) => {
                log("Received non-string message");
            }
        }
    }) as Box<dyn FnMut(MessageEvent)>);

    let url = Arc::new(url);

    let onclose_callback = Closure::wrap(Box::new(move |_: CloseEvent| {
        // TODO: wait a bit before reconnecting
        if let Err(err) = sender.unbounded_send(None) {
            log(&format!("Error: {err:?}"))
        }
        reconnect((*url).clone(), sender.clone(), ctx.clone());
    }) as Box<dyn FnMut(CloseEvent)>);

    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));

    onmessage_callback.forget();
    onclose_callback.forget();
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (sender, receiver) = mpsc::unbounded::<Option<StateWithTime>>();

        let server_url = std::option_env!("SERVER_URL").unwrap_or("ws://127.0.0.1:7878");

        let ctx = cc.egui_ctx.clone();

        spawn_local(async move {
            reconnect(
                server_url.to_owned(),
                Arc::new(sender.clone()),
                Arc::new(ctx.clone()),
            );
        });

        let (sender, promise) = Promise::new();
        {
            let request = ehttp::Request::get("https://aicontest.dev/top_results.txt");
            ehttp::fetch(request, move |response| {
                let mut res = vec![];
                if let Ok(response) = response {
                    if let Some(text) = response.text() {
                        for line in text.lines() {
                            let words: Vec<_> = line.split_ascii_whitespace().collect();
                            res.push(Player {
                                name: words[0].to_owned(),
                                pos: Point::ZERO,
                                speed: Point::ZERO,
                                target: Point::ZERO,
                                score: words[2].parse().unwrap(),
                                radius: 0,
                            })
                        }
                    }
                }
                sender.send(res);
            });
        }

        Self {
            receiver,
            state_approximator: StateApproximator::default(),
            fps_counter: FpsCounter::new(),
            show_users: HashMap::default(),
            show_top5: true,
            sort_players_by: SortBy::Score,
            server_url: server_url.to_owned(),
            connected: false,
            top_results_promise: promise,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        while let Ok(Some(state)) = self.receiver.try_next() {
            if let Some(state) = state {
                self.connected = true;
                self.state_approximator.add_state(state);
            } else {
                self.connected = false;
            }
        }

        let game_state = self.state_approximator.get_state();

        let full_width = ctx.available_rect().width();
        let side_width = full_width * 0.15;

        egui::SidePanel::left("side_panel")
            .exact_width(side_width)
            .show_separator_line(false)
            .show(ctx, |ui| {
                let fps = self.fps_counter.add_frame();

                ui.hyperlink_to(
                    "README",
                    "https://github.com/bminaiev/aicontest.dev/blob/master/README.md",
                );

                if let Some(game_state) = &game_state {
                    ui.label(format!(
                        "turn: {}/{}",
                        game_state.turn, game_state.max_turns
                    ));
                    ui.label(format!("#players: {}", game_state.players.len()));
                }
                if !self.connected {
                    ui.label(
                        RichText::new(format!("Trying to connect to {}...", self.server_url))
                            .color(Color32::RED),
                    );
                }
                ui.label(format!("fps: {:.1}", fps));
                ui.checkbox(&mut self.show_top5, "Show top-5 players");
                ui.separator();

                if let Some(top_results) = self.top_results_promise.ready() {
                    let top_results = top_results.clone();
                    if !top_results.is_empty() {
                        ui.collapsing("Highest Scores", |ui| {
                            show_ratings(self, ui, top_results, false);
                        });
                        ui.separator();
                    }
                }

                ui.vertical(|ui| {
                    if let Some(game_state) = &game_state {
                        show_ratings(self, ui, game_state.players.clone(), true);
                    }
                });

                egui::warn_if_debug_build(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(game_state) = &game_state {
                draw_state(self, ui, game_state);
            }
        });
    }
}

fn calc_places(players: &[Player]) -> Vec<(Player, String)> {
    let mut res = vec![];
    let mut i = 0;
    while i != players.len() {
        let mut j = i;
        while j != players.len() && players[j].score == players[i].score {
            j += 1;
        }
        let place = if i + 1 == j {
            j.to_string()
        } else {
            format!("{}-{}", i + 1, j)
        };
        while i != j {
            res.push((players[i].clone(), place.clone()));
            i += 1;
        }
    }
    res
}

fn show_ratings(
    app: &mut App,
    ui: &mut egui::Ui,
    mut players: Vec<Player>,
    show_first_column: bool,
) {
    players.sort_by_key(|player| -player.score);
    let mut players = calc_places(&players);

    ui.horizontal(|ui| {
        ui.label("Sort by:");
        ui.radio_value(&mut app.sort_players_by, SortBy::Score, "Score");
        ui.radio_value(&mut app.sort_players_by, SortBy::Name, "Name");
    });

    match app.sort_players_by {
        SortBy::Score => {}
        SortBy::Name => players.sort_by_key(|(player, _)| player.name.clone()),
    }

    let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
    let table = TableBuilder::new(ui)
        .striped(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::auto())
        .column(Column::auto())
        .column(Column::auto());
    let table = if show_first_column {
        table.column(Column::auto())
    } else {
        table
    };

    table
        .header(20.0, |mut header| {
            if show_first_column {
                header.col(|ui| {
                    ui.strong("");
                });
            }
            header.col(|ui| {
                ui.strong("#");
            });
            header.col(|ui| {
                ui.strong("Name");
            });
            header.col(|ui| {
                ui.strong("Score");
            });
        })
        .body(|body| {
            body.rows(text_height, players.len(), |row_index, mut row| {
                let (player, place) = &players[row_index];
                let color = choose_player_color(player);
                if show_first_column {
                    row.col(|ui| {
                        let mut value = *app.show_users.get(&player.name).unwrap_or(&false);
                        if ui.checkbox(&mut value, "").clicked() {
                            app.show_users.insert(player.name.clone(), value);
                        }
                    });
                }
                row.col(|ui| {
                    ui.label(place);
                });
                row.col(|ui| {
                    ui.label(RichText::new(&player.name).color(color));
                });
                row.col(|ui| {
                    ui.label(player.score.to_string());
                });
            });
        });
}

fn draw_state(app: &App, ui: &mut egui::Ui, game_state: &GameState) {
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
        for item in game_state.items.iter() {
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
        draw_players(app, game_state, ui, zoom, conv_pt);
    }
}

fn draw_players(
    app: &App,
    game_state: &GameState,
    ui: &mut egui::Ui,
    zoom: f32,
    conv_pt: impl Fn(Point) -> Pos2,
) {
    let mut scores = game_state
        .players
        .iter()
        .map(|p| p.score)
        .collect::<Vec<_>>();
    scores.sort();
    scores.reverse();

    let top5_score = std::cmp::max(1, *scores.get(4).unwrap_or(&0));

    for player in game_state.players.iter() {
        let color = choose_player_color(player);
        let center = conv_pt(player.pos);
        ui.painter()
            .circle_filled(center, player.radius as f32 * zoom, color);
        // draw_arrow(ui, center, conv_pt(player.target), color);

        let mut show_it = *app.show_users.get(&player.name).unwrap_or(&false);
        if app.show_top5 && player.score >= top5_score {
            show_it = true;
        }

        if show_it {
            ui.painter().text(
                conv_pt(
                    player.pos
                        + Point {
                            x: player.radius,
                            y: 0,
                        },
                ),
                Align2::LEFT_CENTER,
                format!("{} [score={}]", player.name, player.score),
                FontId::monospace(13.0),
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
    let arrow_len = 5.0;
    let arrow_width = 2.0;
    let arrow_start = to - dir * arrow_len;
    let arrow_dir = vec2(dir.y, -dir.x);
    let arrow_points = vec![
        arrow_start + arrow_dir * arrow_width,
        to,
        arrow_start - arrow_dir * arrow_width,
    ];
    ui.painter().add(Shape::dashed_line(
        &[from, to],
        Stroke::new(1.0, color),
        10.0,
        5.0,
    ));
    ui.painter()
        .add(Shape::line(arrow_points, Stroke::new(1.0, color)));
}
