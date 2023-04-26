use std::collections::VecDeque;

use instant::{Duration, SystemTime};

pub struct FpsCounter {
    frames: VecDeque<f64>,
}

const SECS: f64 = 2.0;

impl FpsCounter {
    pub fn new() -> Self {
        Self {
            frames: VecDeque::new(),
        }
    }

    // returns fps
    pub fn add_frame(&mut self) -> f64 {
        let cur_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs_f64();
        self.frames.push_back(cur_time);
        let remove_till = cur_time - SECS;
        while *self.frames.front().unwrap() < remove_till {
            self.frames.pop_front();
        }
        (self.frames.len() as f64) / SECS
    }
}
