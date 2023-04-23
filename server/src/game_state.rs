use crate::connction::Connection;
use crate::consts::{
    HEIGHT, MAX_ACC, MAX_ITEMS, MAX_ITEM_R, MAX_SPEED, MAX_TURNS, MIN_ITEM_R, PLAYER_RADIUS, WIDTH,
};
use crate::player_move::PlayerMove;
use crate::point::Point;
use anyhow::Result;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};

#[derive(Clone)]
pub struct Player {
    pub name: String,
    pos: Point,
    speed: Point,
    target: Point,
    pub score: i64,
    radius: i32,
    // TODO: contact info?
}

#[derive(Clone)]
pub struct Item {
    pos: Point,
    radius: i32,
}

impl Item {
    pub fn intersects(&self, player: &Player) -> bool {
        let dist = self.pos - player.pos;
        let max_ok_dist = self.radius + player.radius;
        dist.len2() <= max_ok_dist * max_ok_dist
    }
}

#[derive(Clone)]
pub struct GameState {
    width: i32,
    height: i32,
    pub turn: usize,
    max_turns: usize,
    pub players: Vec<Player>,
    items: Vec<Item>,
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
                self.width - player.radius,
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
            self.items.push(Item {
                pos: self.gen_rand_position(r),
                radius: r,
            })
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
                "{name} {score} {x} {y} {r} {vx} {vy}\n",
                name = player.name,
                score = player.score,
                x = player.pos.x,
                y = player.pos.y,
                r = player.radius,
                vx = player.speed.x,
                vy = player.speed.y
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
        res
    }

    pub async fn send_to_conn(&self, conn: &mut Connection) -> Result<()> {
        conn.write(self.to_string()).await?;
        Ok(())
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
