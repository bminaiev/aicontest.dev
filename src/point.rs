#[derive(Copy, Clone)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn len2(&self) -> i32 {
        self.x * self.x + self.y * self.y
    }
    pub fn len(&self) -> f64 {
        (self.len2() as f64).sqrt()
    }

    pub fn scale(&self, target_len: f64) -> Self {
        if self.x == 0 && self.y == 0 {
            return *self;
        }
        let mult = target_len / (self.len());
        Point {
            x: ((self.x as f64) * mult).round() as i32,
            y: ((self.y as f64) * mult).round() as i32,
        }
    }
}

impl std::ops::Sub for Point {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Point {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl std::ops::Add for Point {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Point {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::AddAssign for Point {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}
