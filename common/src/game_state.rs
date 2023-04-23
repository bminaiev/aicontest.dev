use std::collections::VecDeque;
use std::str::FromStr;

use crate::consts::{
    HEIGHT, MAX_ACC, MAX_ITEMS, MAX_ITEM_R, MAX_SPEED, MAX_TURNS, MIN_ITEM_R, PLAYER_RADIUS, WIDTH,
};
use crate::player_move::PlayerMove;
use crate::point::Point;
use anyhow::{anyhow, bail};
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};

#[derive(Clone)]
pub struct Player {
    pub name: String,
    pub pos: Point,
    pub speed: Point,
    pub target: Point,
    pub score: i64,
    pub radius: i32,
    // TODO: contact info?
}

#[derive(Clone)]
pub struct Item {
    pub pos: Point,
    pub radius: i32,
}

impl Item {
    pub fn intersects(&self, player: &Player) -> bool {
        let dist = self.pos - player.pos;
        let max_ok_dist = self.radius + player.radius;
        dist.len2() <= max_ok_dist * max_ok_dist
    }

    pub fn intersects_item(&self, another: &Self) -> bool {
        let dist = self.pos - another.pos;
        let max_ok_dist = self.radius + another.radius;
        dist.len2() <= max_ok_dist * max_ok_dist
    }
}

#[derive(Clone)]
pub struct GameState {
    pub width: i32,
    pub height: i32,
    pub turn: usize,
    pub max_turns: usize,
    pub players: Vec<Player>,
    pub items: Vec<Item>,
}

pub struct GameResults {
    pub players: Vec<Player>,
}

impl GameResults {
    pub fn new(state: GameState) -> Self {
        let mut players = state.players;
        players.sort_by_key(|player| -player.score);
        Self { players }
    }
}

pub enum NextTurn {
    GameState(GameState),
    FinalResults(GameResults),
}

fn clamp(pos: &mut i32, speed: &mut i32, min_pos: i32, max_pos: i32) {
    if *pos < min_pos {
        *pos = 2 * min_pos - *pos;
        *speed *= -1;
    } else if *pos >= max_pos {
        *pos = 2 * max_pos - *pos;
        *speed *= -1;
    }
}

struct TokenReader {
    tokens: VecDeque<String>,
}

impl TokenReader {
    pub fn new(s: &str) -> Self {
        Self {
            tokens: s.split_ascii_whitespace().map(|s| s.to_string()).collect(),
        }
    }

    pub fn next<T>(&mut self, err_msg: &str) -> anyhow::Result<T>
    where
        T: FromStr,
        <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
    {
        Ok(self
            .tokens
            .pop_front()
            .ok_or_else(|| anyhow!(err_msg.to_owned()))?
            .parse()
            .map_err(|err| anyhow!("Failed to parse '{err_msg}': {err:?}"))?)
    }
}

impl GameState {
    pub fn next_turn(mut self) -> NextTurn {
        for player in self.players.iter_mut() {
            let acc = player.target - player.pos;
            let acc = acc.scale(MAX_ACC);
            player.speed += acc;
            if player.speed.len() > MAX_SPEED {
                player.speed = player.speed.scale(MAX_SPEED);
            }
            player.pos += player.speed;
            clamp(
                &mut player.pos.x,
                &mut player.speed.x,
                player.radius,
                self.width - player.radius,
            );
            clamp(
                &mut player.pos.y,
                &mut player.speed.y,
                player.radius,
                self.height - player.radius,
            );
        }
        let mut ids: Vec<_> = (0..self.players.len()).collect();
        let mut rng = thread_rng();
        ids.shuffle(&mut rng);
        for &id in ids.iter() {
            for i in (0..self.items.len()).rev() {
                if self.items[i].intersects(&self.players[id]) {
                    self.players[id].score += 1;
                    self.items.remove(i);
                    // TODO: create new objects
                }
            }
        }
        self.turn += 1;
        if self.turn == self.max_turns {
            return NextTurn::FinalResults(GameResults::new(self));
        }
        self.add_more_items();
        NextTurn::GameState(self)
    }

    fn add_more_items(&mut self) {
        let mut rng = thread_rng();

        while self.items.len() < MAX_ITEMS {
            // TODO: make logic more interesting
            let r = rng.gen_range(MIN_ITEM_R..MAX_ITEM_R);
            let new_item = Item {
                pos: self.gen_rand_position(r),
                radius: r,
            };
            let mut ok = true;
            for existing in self.items.iter() {
                if existing.intersects_item(&new_item) {
                    ok = false;
                    break;
                }
            }
            if ok {
                self.items.push(new_item)
            }
        }
    }

    fn gen_rand_position(&self, radius: i32) -> Point {
        let mut rng = thread_rng();
        let x = rng.gen_range(radius..self.width - radius);
        let y = rng.gen_range(radius..self.height - radius);
        Point { x, y }
    }

    pub fn new() -> Self {
        let mut res = Self {
            width: WIDTH,
            height: HEIGHT,
            turn: 0,
            max_turns: MAX_TURNS,
            players: vec![],
            items: vec![],
        };
        res.add_more_items();
        res
    }

    pub fn to_string(&self) -> String {
        let mut res = String::new();
        res += &format!(
            "TURN {turn} {max_turns} {width} {height}\n",
            turn = self.turn,
            max_turns = self.max_turns,
            width = self.width,
            height = self.height,
        );
        res += &format!("{}\n", self.players.len());
        for player in self.players.iter() {
            res += &format!(
                "{name} {score} {x} {y} {r} {vx} {vy} {target_x} {target_y}\n",
                name = player.name,
                score = player.score,
                x = player.pos.x,
                y = player.pos.y,
                r = player.radius,
                vx = player.speed.x,
                vy = player.speed.y,
                target_x = player.target.x,
                target_y = player.target.y,
            );
        }
        res += &format!("{}\n", self.items.len());
        for item in self.items.iter() {
            res += &format!(
                "{x} {y} {r}\n",
                x = item.pos.x,
                y = item.pos.y,
                r = item.radius
            );
        }
        res += "END_STATE\n";
        res
    }

    pub fn from_string(s: &str) -> anyhow::Result<Self> {
        let mut tokens = TokenReader::new(s);
        let cmd_word: String = tokens.next("TURN")?;
        if cmd_word != "TURN" {
            bail!("Expected TURN, got {}", cmd_word);
        }
        let turn = tokens.next("turn")?;
        let max_turns = tokens.next("max_turn")?;
        let width = tokens.next("width")?;
        let height = tokens.next("height")?;
        let mut res = Self {
            width,
            height,
            turn,
            max_turns,
            players: vec![],
            items: vec![],
        };
        let num_players = tokens.next("num_players")?;
        for _ in 0..num_players {
            let name = tokens.next("player name")?;
            let score = tokens.next("player score")?;
            let x = tokens.next("player x")?;
            let y = tokens.next("player y")?;
            let r = tokens.next("player r")?;
            let vx = tokens.next("player vx")?;
            let vy = tokens.next("player vy")?;
            let target_x = tokens.next("player target_x")?;
            let target_y = tokens.next("player target_y")?;
            res.players.push(Player {
                name,
                score,
                pos: Point { x, y },
                radius: r,
                speed: Point { x: vx, y: vy },
                target: Point {
                    x: target_x,
                    y: target_y,
                },
            });
        }
        let num_items = tokens.next("num items")?;
        for _ in 0..num_items {
            let x = tokens.next("item x")?;
            let y = tokens.next("item y")?;
            let r = tokens.next("item r")?;
            res.items.push(Item {
                pos: Point { x, y },
                radius: r,
            });
        }
        let end_state: String = tokens.next("END_STATE")?;
        if end_state != "END_STATE" {
            bail!("Expected END_STATE, got {}", end_state);
        }
        Ok(res)
    }

    fn find_player_idx(&self, player_name: &str) -> Option<usize> {
        for i in 0..self.players.len() {
            if self.players[i].name == player_name {
                return Some(i);
            }
        }
        None
    }

    pub fn make_player_first(&mut self, player_name: &str) -> bool {
        if let Some(idx) = self.find_player_idx(player_name) {
            self.players.swap(0, idx);
            true
        } else {
            false
        }
    }

    pub fn apply_move(&mut self, player_move: PlayerMove) {
        // TODO: validate move
        if let Some(idx) = self.find_player_idx(&player_move.name) {
            self.players[idx].target = player_move.target;
        } else {
            let radius = PLAYER_RADIUS;
            let pos = self.gen_rand_position(radius);
            self.players.push(Player {
                name: player_move.name,
                pos,
                speed: Point::ZERO,
                target: pos,
                score: 0,
                radius,
            });
        }
    }
}