pub fn to_radians(degrees: i32) -> f64 {
    (degrees as f64) * std::f64::consts::PI / 180.0
}

pub struct Ball {
    x: f64,
    y: f64,
    v: f64,
    a: f64,
    vx: f64,
    vy: f64,
    radius: f64,
    mass: f64,
}

impl Ball {
    pub fn new(x: f64, y: f64, v: f64, a: f64, radius: f64) -> Ball {
        Ball {
            x,
            y,
            v,
            a,
            vx: v * a.to_radians().cos(),
            vy: v * a.to_radians().sin(),
            radius,
            mass: radius * radius * std::f64::consts::PI,
        }
    }

    pub fn get_cell(&self, screenSizeX: i32, screenSizeY: i32, horizontalCells: i32, verticalCells: i32) -> (i32, i32) {
        let x = (self.x / (screenSizeX as f64 / horizontalCells as f64)).min(0.0).max(horizontalCells as f64 - 1.0) as i32;
        let y = (self.y / (screenSizeY as f64 / verticalCells as f64)).min(0.0).max(verticalCells as f64 - 1.0) as i32;
        (x, y)
    }

    pub fn move_ball(&mut self, screen_size_x: i32, screen_size_y: i32, horizontal_cells: i32, vertical_cells: i32, gravity: f64, dt: f64) -> ((i32, i32), (i32, i32)) {
        // Get the current cell
        let current_cell = self.get_cell(screen_size_x, screen_size_y, horizontal_cells, vertical_cells);

        // // Apply gravity
        // self.vy += gravity * dt;
        // self.v = (self.vx * self.vx + self.vy * self.vy).sqrt();

        // Move the ball
        self.x += self.vx * dt;
        self.y += self.vy * dt;

        // Apply border collision
        // NOTE: -self.vx.abs() may return positive value
        let mut velocityChanged = false;
        if self.x - self.radius < 0.0 {
            self.x = self.radius;
            self.vx = self.vx.abs();
            velocityChanged = true;
        } else if self.x + self.radius > screen_size_x as f64 {
            self.x = screen_size_x as f64 - self.radius;
            self.vx = -self.vx.abs();
            velocityChanged = true;
        }
        if self.y - self.radius < 0.0 {
            self.y = self.radius;
            self.vy = self.vy.abs();
            velocityChanged = true;
        } else if self.y + self.radius > screen_size_y as f64 {
            self.y = screen_size_y as f64 - self.radius;
            self.vy = -self.vy;
            velocityChanged = true;
        }

        // Calculate the new angle if the velocity changed from a border collision
        if velocityChanged {
            self.a = self.vy.atan2(self.vx).to_degrees();
        }

        // Get the new cell
        let new_cell = self.get_cell(screen_size_x, screen_size_y, horizontal_cells, vertical_cells);
        
        // Return the current and new cell so that it can be used to update the grid
        return (current_cell, new_cell);
    }

    pub fn collide(&mut self, other: &mut Ball, screen_size_x: i32, screen_size_y: i32, horizontal_cells: i32, vertical_cells: i32) {
        let distance = ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt();
        if distance <= self.radius + other.radius {
            let originalVx = self.vx;
            let originalVy = self.vy;
            let contactAngle = (other.y - self.y).atan2(other.x - self.x).to_degrees();
            let contactAngleCos = contactAngle.to_radians().cos();
            let contactAngleSin = contactAngle.to_radians().sin();
            let contactAngle90Cos = -contactAngleSin;
            let contactAngle90Sin = contactAngleCos;

            // self.vx = (self.v * math.cos(math.radians(self.a - contactAngle)) * (self.mass - other.mass) + 2 * other.mass * other.v * math.cos(math.radians(other.a - contactAngle))) / (self.mass + other.mass) * contactAngleCos + self.v * math.sin(math.radians(self.a - contactAngle)) * contactAngle90Cos
            // self.vy = (self.v * math.cos(math.radians(self.a - contactAngle)) * (self.mass - other.mass) + 2 * other.mass * other.v * math.cos(math.radians(other.a - contactAngle))) / (self.mass + other.mass) * contactAngleSin + self.v * math.sin(math.radians(self.a - contactAngle)) * contactAngle90Sin
            // other.vx = (other.v * math.cos(math.radians(other.a - contactAngle)) * (other.mass - self.mass) + 2 * self.mass * originalVx * math.cos(math.radians(self.a - contactAngle))) / (self.mass + other.mass) * contactAngleCos + other.v * math.sin(math.radians(other.a - contactAngle)) * contactAngle90Cos
            // other.vy = (other.v * math.cos(math.radians(other.a - contactAngle)) * (other.mass - self.mass) + 2 * self.mass * originalVy * math.cos(math.radians(self.a - contactAngle))) / (self.mass + other.mass) * contactAngleSin + other.v * math.sin(math.radians(other.a - contactAngle)) * contactAngle90Sin
            // self.a = math.degrees(math.atan2(self.vy, self.vx))
            // other.a = math.degrees(math.atan2(other.vy, other.vx))
            // self.v = (self.vx ** 2 + self.vy ** 2) ** 0.5
            // other.v = (other.vx ** 2 + other.vy ** 2)  ** 0.5

            // Apply direct collision
            self.vx = (self.v * (self.a - contactAngle).to_radians().cos() * (self.mass - other.mass) + 2.0 * other.mass * other.v * (other.a - contactAngle).to_radians().cos()) / (self.mass + other.mass) * contactAngleCos + self.v * (self.a - contactAngle).to_radians().sin() * contactAngle90Cos;
            self.vy = (self.v * (self.a - contactAngle).to_radians().cos() * (self.mass - other.mass) + 2.0 * other.mass * other.v * (other.a - contactAngle).to_radians().cos()) / (self.mass + other.mass) * contactAngleSin + self.v * (self.a - contactAngle).to_radians().sin() * contactAngle90Sin;
            other.vx = (other.v * (other.a - contactAngle).to_radians().cos() * (other.mass - self.mass) + 2.0 * self.mass * originalVx * (self.a - contactAngle).to_radians().cos()) / (self.mass + other.mass) * contactAngleCos + other.v * (other.a - contactAngle).to_radians().sin() * contactAngle90Cos;
            other.vy = (other.v * (other.a - contactAngle).to_radians().cos() * (other.mass - self.mass) + 2.0 * self.mass * originalVy * (self.a - contactAngle).to_radians().cos()) / (self.mass + other.mass) * contactAngleSin + other.v * (other.a - contactAngle).to_radians().sin() * contactAngle90Sin;
            self.a = self.vy.atan2(self.vx).to_degrees();
            other.a = other.vy.atan2(other.vx).to_degrees();
            self.v = (self.vx.powi(2) + self.vy.powi(2)).sqrt();
            other.v = (other.vx.powi(2) + other.vy.powi(2)).sqrt();

            // If the balls are overlapping, move them apart
            let selfCurrentCell = self.get_cell(screen_size_x, screen_size_y, horizontal_cells, vertical_cells);
            let otherCurrentCell = other.get_cell(screen_size_x, screen_size_y, horizontal_cells, vertical_cells);
            
        }
    }
}