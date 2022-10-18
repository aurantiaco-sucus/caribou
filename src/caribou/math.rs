use std::ops::{Add, Sub};

#[derive(Default, Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ScalarPair {
    pub x: f32,
    pub y: f32,
}

impl ScalarPair {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn to_int(&self) -> IntPair {
        IntPair {
            x: self.x as i32,
            y: self.y as i32,
        }
    }
}

impl From<(f32, f32)> for ScalarPair {
    fn from((x, y): (f32, f32)) -> Self {
        Self { x, y }
    }
}

impl Add for ScalarPair {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for ScalarPair {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl ScalarPair {
    pub fn times(&self, rhs: f32) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IntPair {
    pub x: i32,
    pub y: i32,
}

impl IntPair {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn to_scalar(&self) -> ScalarPair {
        ScalarPair {
            x: self.x as f32,
            y: self.y as f32,
        }
    }
}

impl From<(i32, i32)> for IntPair {
    fn from((x, y): (i32, i32)) -> Self {
        Self { x, y }
    }
}

impl Add for IntPair {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for IntPair {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl IntPair {
    pub fn times(&self, rhs: i32) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

pub struct Region {
    pub origin: ScalarPair,
    pub size: ScalarPair,
}

impl Region {
    pub fn new(origin: ScalarPair, size: ScalarPair) -> Self {
        Self { origin, size }
    }

    pub fn origin_size(origin: ScalarPair, size: ScalarPair) -> Self {
        Self { origin, size }
    }

    pub fn begin_end(begin: ScalarPair, end: ScalarPair) -> Self {
        Self { origin: begin, size: end - begin }
    }

    pub fn contains(&self, point: ScalarPair) -> bool {
        point.x >= self.origin.x && point.x < self.origin.x + self.size.x &&
        point.y >= self.origin.y && point.y < self.origin.y + self.size.y
    }

    pub fn contains_region(&self, region: &Region) -> bool {
        self.contains(region.origin) && self.contains(region.origin + region.size)
    }

    pub fn intersects(&self, region: &Region) -> bool {
        self.contains(region.origin) || self.contains(region.origin + region.size) ||
        region.contains(self.origin) || region.contains(self.origin + self.size)
    }
}