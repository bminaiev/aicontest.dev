#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const ZERO: Self = Point { x: 0, y: 0 };

    pub fn len2(&self) -> f64 {
        let x = self.x as f64;
        let y = self.y as f64;
        x * x + y * y
    }
    pub fn len(&self) -> f64 {
        self.len2().sqrt()
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

    pub fn dist2(&self, other: &Self) -> i32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
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
