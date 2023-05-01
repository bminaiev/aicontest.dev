use std::time::Duration;

pub const MAX_ACC: f64 = 20.0;
pub const MAX_SPEED: f64 = 100.0;
pub const MAX_ITEMS: usize = 10;
pub const MIN_ITEM_R: i32 = 20;
pub const MAX_ITEM_R: i32 = 100;
pub const PLAYER_RADIUS: i32 = 20;

pub const START_WIDTH: i32 = 2000;
pub const START_HEIGHT: i32 = 1500;
// if more players play, field becomes bigger
pub const START_MAX_PLAYERS: usize = 5;

pub const MAX_TURNS: usize = 600;
pub const TURN_WAIT_TIME: Duration = Duration::from_millis(500);

pub const MAX_LOGIN_LEN: usize = 20;
pub const MAX_PASSWORD_LEN: usize = 100;
