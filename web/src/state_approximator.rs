use std::collections::VecDeque;

use game_common::{
    game_state::{GameState, Item, Player},
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
    states: VecDeque<StateWithTime>,
}

impl StateApproximator {
    pub fn add_state(&mut self, mut state: StateWithTime) {
        const OFFSET: Duration = Duration::from_secs(2);
        state.timestamp = state
            .timestamp
            .checked_add(OFFSET)
            .unwrap_or(state.timestamp);
        self.states.push_back(state);
        self.make_timestamps_equaly_distributed();
    }

    fn make_timestamps_equaly_distributed(&mut self) {
        if self.states.len() >= 2 {
            let start = self.states[0].timestamp;
            let end = self.states.back().unwrap().timestamp;
            let delta = end
                .duration_since(start)
                .unwrap_or(Duration::ZERO)
                .as_secs_f64()
                / (self.states.len() - 1) as f64;
            for i in 1..self.states.len() {
                self.states[i].timestamp = start
                    .checked_add(Duration::from_secs_f64(delta * i as f64))
                    .unwrap_or(self.states[i].timestamp);
            }
        }
    }

    pub fn more_buffer(&self) -> Duration {
        let cur_time = SystemTime::now();
        if let Some(last) = self.states.back() {
            last.timestamp
                .duration_since(cur_time)
                .unwrap_or(Duration::ZERO)
        } else {
            Duration::ZERO
        }
    }

    pub fn get_state(&mut self) -> Option<GameState> {
        if self.states.is_empty() {
            return None;
        }
        let cur_time = SystemTime::now();
        while self.states.len() > 1 && self.states[1].timestamp < cur_time {
            self.states.pop_front();
        }
        if self.states.len() == 1 || self.states[1].state.turn < self.states[0].state.turn {
            return Some(self.states[0].state.clone());
        }

        if self.states[0].timestamp > cur_time {
            // hacky thing to make it start showing motion right away
            self.states[0].timestamp = cur_time;
        }

        let prev = &self.states[0];
        let next = &self.states[1];
        let delta = next
            .timestamp
            .duration_since(prev.timestamp)
            .unwrap_or(Duration::ZERO)
            .as_secs_f64();
        let cur_delta = cur_time
            .duration_since(prev.timestamp)
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
        let items: Vec<Item> = prev
            .state
            .items
            .iter()
            .filter_map(|item| {
                if next.state.items.contains(item) {
                    return Some(item.clone());
                }
                for p in players.iter() {
                    if item.intersects(p) {
                        return None;
                    }
                }
                Some(item.clone())
            })
            .collect();
        let fake_state = GameState {
            width: prev.state.width,
            height: prev.state.height,
            turn: prev.state.turn,
            max_turns: prev.state.max_turns,
            players,
            items,
            game_id: prev.state.game_id.clone(),
        };
        return Some(fake_state);
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}
