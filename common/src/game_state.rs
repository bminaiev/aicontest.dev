use std::collections::VecDeque;
use std::str::FromStr;

use crate::consts::{
    MAX_ACC, MAX_ITEMS, MAX_ITEM_R, MAX_SPEED, MAX_TURNS, MIN_ITEM_R, PLAYER_RADIUS, START_HEIGHT,
    START_MAX_PLAYERS, START_WIDTH,
};
use crate::player_move::PlayerMove;
use crate::point::Point;
use anyhow::{anyhow, bail};
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};

#[derive(Clone, Debug)]
pub struct Player {
    pub name: String,
    pub pos: Point,
    pub speed: Point,
    pub target: Point,
    pub score: i64,
    pub radius: i32,
    // TODO: contact info?
}

#[derive(Clone, PartialEq, Eq)]
pub struct Item {
    pub pos: Point,
    pub radius: i32,
}

impl Item {
    pub fn intersects(&self, player: &Player) -> bool {
        let dist = self.pos - player.pos;
        let max_ok_dist = self.radius + player.radius;
        dist.len2() <= (max_ok_dist * max_ok_dist) as f64
    }

    pub fn intersects_item(&self, another: &Self) -> bool {
        let dist = self.pos - another.pos;
        let max_ok_dist = self.radius + another.radius;
        dist.len2() <= (max_ok_dist * max_ok_dist) as f64
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
    pub game_id: String,
}

pub struct GameResults {
    pub players: Vec<Player>,
    pub game_id: String,
}

impl GameResults {
    pub fn new(state: GameState) -> Self {
        let mut players = state.players;
        players.sort_by_key(|player| -player.score);
        Self {
            players,
            game_id: state.game_id,
        }
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

pub fn next_turn_player_state(player: &mut Player, width: i32, height: i32) {
    let mut acc = player.target - player.pos;
    if acc.len() > MAX_ACC {
        acc = acc.scale(MAX_ACC);
    }
    player.speed += acc;
    if player.speed.len() > MAX_SPEED {
        player.speed = player.speed.scale(MAX_SPEED);
    }
    player.pos += player.speed;
    clamp(
        &mut player.pos.x,
        &mut player.speed.x,
        player.radius,
        width - player.radius,
    );
    clamp(
        &mut player.pos.y,
        &mut player.speed.y,
        player.radius,
        height - player.radius,
    );
}

impl GameState {
    pub fn next_turn(mut self) -> NextTurn {
        for player in self.players.iter_mut() {
            next_turn_player_state(player, self.width, self.height);
        }
        let mut ids: Vec<_> = (0..self.players.len()).collect();
        let mut rng = thread_rng();
        ids.shuffle(&mut rng);
        for &id in ids.iter() {
            for i in (0..self.items.len()).rev() {
                if self.items[i].intersects(&self.players[id]) {
                    self.players[id].score += 1;
                    self.items.remove(i);
                }
            }
        }
        self.turn += 1;
        if self.turn == self.max_turns {
            return NextTurn::FinalResults(GameResults::new(self));
        }
        self.update_size();
        self.add_more_items();
        NextTurn::GameState(self)
    }

    fn update_size(&mut self) {
        let scaling = self.scaling_coef().sqrt();
        self.width = ((START_WIDTH as f64) * scaling).round() as i32;
        self.height = ((START_HEIGHT as f64) * scaling).round() as i32;
    }

    fn calc_max_items(&self) -> usize {
        ((MAX_ITEMS as f64) * self.scaling_coef()).round() as usize
    }

    fn scaling_coef(&self) -> f64 {
        if self.players.len() < START_MAX_PLAYERS {
            return 1.0;
        }
        (self.players.len() as f64) / (START_MAX_PLAYERS as f64)
    }

    fn add_more_items(&mut self) {
        let mut rng = thread_rng();

        let max_items = self.calc_max_items();
        while self.items.len() < max_items {
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

    pub fn new(game_id: &str) -> Self {
        let mut res = Self {
            width: START_WIDTH,
            height: START_HEIGHT,
            turn: 0,
            max_turns: MAX_TURNS,
            players: vec![],
            items: vec![],
            game_id: game_id.to_owned(),
        };
        res.add_more_items();
        res
    }

    pub fn to_string(&self) -> String {
        let mut res = String::new();
        res += &format!(
            "TURN {turn} {max_turns} {width} {height} {game_id}\n",
            turn = self.turn,
            max_turns = self.max_turns,
            width = self.width,
            height = self.height,
            game_id = self.game_id,
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
        let game_id = tokens.next("game_id")?;
        let mut res = Self {
            width,
            height,
            turn,
            max_turns,
            players: vec![],
            items: vec![],
            game_id,
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

    pub fn apply_move(&mut self, mut player_move: PlayerMove) {
        // TODO: validate move
        const MAX_C: u32 = u32::MAX / 10;
        if player_move.target.x.unsigned_abs() > MAX_C
            || player_move.target.y.unsigned_abs() > MAX_C
        {
            player_move.target.x /= 10;
            player_move.target.y /= 10;
        }
        if let Some(idx) = self.find_player_idx(&player_move.name) {
            self.players[idx].target = player_move.target;
        } else {
            let radius = PLAYER_RADIUS;
            let pos = self.gen_rand_position(radius);
            self.players.push(Player {
                name: player_move.name,
                pos,
                speed: Point::ZERO,
                target: player_move.target,
                score: 0,
                radius,
            });
        }
    }
}

#[test]
fn next_turn_state() {
    let mut player = Player {
        name: "player".to_owned(),
        pos: Point { x: 100, y: 100 },
        speed: Point { x: 10, y: 0 },
        target: Point { x: 150, y: 200 }, // sent by `GO 150 200` command
        score: 0,
        radius: 1,
    };
    next_turn_player_state(&mut player, 1000, 1000);
    // acceleration direction is (150, 200) - (100, 100) = (50, 100)
    // the length of vector (50, 100) is sqrt(50^2 + 100^2) = 111.8, which is bigger than MAX_ACC=20.0, so real acceleration is:
    // (50, 100) * 20.0 / 111.8 = (8.9, 17.8)
    // after that acceleration is rounded to integers: (9, 18)
    // new speed is (10, 0) + (9, 18) = (19, 18)
    assert_eq!(player.speed, Point { x: 19, y: 18 });
    // new position is (100, 100) + (19, 18) = (119, 118)
    assert_eq!(player.pos, Point { x: 119, y: 118 });
}
