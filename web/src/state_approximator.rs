use game_common::{
    game_state::{GameState, Player},
    point::Point,
};
use instant::{Duration, SystemTime};
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone)]
pub struct StateWithTime {
    pub state: GameState,
    pub timestamp: SystemTime,
}

#[derive(Default)]
pub struct StateApproximator {
    prev: Option<StateWithTime>,
    next: Option<StateWithTime>,
}

impl StateApproximator {
    pub fn add_state(&mut self, state: StateWithTime) {
        if self.next.is_some() && self.next.as_ref().unwrap().state.turn < state.state.turn {
            self.prev = self.next.clone();
            self.next = Some(state)
        } else {
            if self.prev.is_some() && self.prev.as_ref().unwrap().state.turn < state.state.turn {
                self.next = Some(state);
            } else {
                self.prev = Some(state);
                self.next = None;
            }
        }
    }

    pub fn get_state(&self) -> Option<GameState> {
        if self.prev.is_none() {
            return None;
        }
        let prev = self.prev.as_ref().unwrap();
        if self.next.is_none() {
            return Some(prev.state.clone());
        }
        let next = self.next.as_ref().unwrap();
        let delta = next
            .timestamp
            .duration_since(prev.timestamp)
            .unwrap_or(Duration::ZERO)
            .as_secs_f64();
        let cur_delta = SystemTime::now()
            .duration_since(next.timestamp)
            .unwrap_or(Duration::ZERO)
            .as_secs_f64();
        let pos = if delta == 0.0 { 0.0 } else { cur_delta / delta };
        let players: Vec<Player> = prev
            .state
            .players
            .iter()
            .map(|player| {
                let next_player = next.state.players.iter().find(|p| p.name == player.name);
                if let Some(next_player) = next_player {
                    let mut new_player = player.clone();
                    new_player.pos = Point {
                        x: (pos * next_player.pos.x as f64 + (1.0 - pos) * player.pos.x as f64)
                            as i32,
                        y: (pos * next_player.pos.y as f64 + (1.0 - pos) * player.pos.y as f64)
                            as i32,
                    };
                    new_player
                } else {
                    player.clone()
                }
            })
            .collect();
        let fake_state = GameState {
            width: prev.state.width,
            height: prev.state.height,
            turn: prev.state.turn,
            max_turns: prev.state.max_turns,
            players,
            items: prev.state.items.clone(),
        };
        return Some(fake_state);
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}
