pub struct Ball {
    x: i32,
    y: i32,
    vx: i32,
    vy: i32,
    radius: i32
}

impl Ball {
    pub fn new(x: i32, y: i32, vx: i32, vy: i32, radius: i32) -> Ball {
        Ball { x, y, vx, vy, radius }
    }
}