use crate::consts::{
    HEIGHT, MAX_ACC, MAX_ITEMS, MAX_ITEM_R, MAX_SPEED, MAX_TURNS, MIN_ITEM_R, WIDTH,
};
use crate::point::Point;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};

pub struct Player {
    name: String,
    pos: Point,
    speed: Point,
    target: Point,
    score: i64,
    radius: i32,
    // TODO: contact info?
}

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

pub struct GameState {
    width: i32,
    height: i32,
    turn: usize,
    max_turns: usize,
    players: Vec<Player>,
    items: Vec<Item>,
}

pub struct GameResults {
    players: Vec<Player>,
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
            let x = rng.gen_range(r..self.width - r);
            let y = rng.gen_range(r..self.height - r);
            self.items.push(Item {
                pos: Point { x, y },
                radius: r,
            })
        }
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
}
