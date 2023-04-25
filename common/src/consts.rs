use std::time::Duration;

pub const MAX_ACC: f64 = 20.0;
pub const MAX_SPEED: f64 = 100.0;
pub const MAX_ITEMS: usize = 10;
pub const MIN_ITEM_R: i32 = 20;
pub const MAX_ITEM_R: i32 = 100;
pub const PLAYER_RADIUS: i32 = 20;

pub const WIDTH: i32 = 2000;
pub const HEIGHT: i32 = 1200;
pub const MAX_TURNS: usize = 600;
pub const TURN_WAIT_TIME: Duration = Duration::from_millis(500);
