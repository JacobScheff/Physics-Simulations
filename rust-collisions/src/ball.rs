pub struct Ball {
    x: f64,
    y: f64,
    v: f64,
    a: f64,
    vx: f64,
    vy: f64,
    radius: f64,
    mass: f64,
    id: i32,
}

impl Ball {
    pub fn new(x: f64, y: f64, v: f64, a: f64, radius: f64, id: i32) -> Ball {
        Ball {
            x,
            y,
            v,
            a,
            vx: v * a.to_radians().cos(),
            vy: v * a.to_radians().sin(),
            radius,
            mass: radius * radius * std::f64::consts::PI,
            id: id,
        }
    }

    pub fn get_cell(&self, screen_size_x: i32, screen_size_y: i32, horizontal_cells: i32, vertical_cells: i32) -> (i32, i32) {
        let x = (self.x / (screen_size_x as f64 / horizontal_cells as f64)).min(0.0).max(horizontal_cells as f64 - 1.0) as i32;
        let y = (self.y / (screen_size_y as f64 / vertical_cells as f64)).min(0.0).max(vertical_cells as f64 - 1.0) as i32;
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
        let mut velocity_changed = false;
        if self.x - self.radius < 0.0 {
            self.x = self.radius;
            self.vx = self.vx.abs();
            velocity_changed = true;
        } else if self.x + self.radius > screen_size_x as f64 {
            self.x = screen_size_x as f64 - self.radius;
            self.vx = -self.vx.abs();
            velocity_changed = true;
        }
        if self.y - self.radius < 0.0 {
            self.y = self.radius;
            self.vy = self.vy.abs();
            velocity_changed = true;
        } else if self.y + self.radius > screen_size_y as f64 {
            self.y = screen_size_y as f64 - self.radius;
            self.vy = -self.vy;
            velocity_changed = true;
        }

        // Calculate the new angle if the velocity changed from a border collision
        if velocity_changed {
            self.a = self.vy.atan2(self.vx).to_degrees();
        }

        // Get the new cell
        let new_cell = self.get_cell(screen_size_x, screen_size_y, horizontal_cells, vertical_cells);
        
        // Return the current and new cell so that it can be used to update the grid
        return (current_cell, new_cell);
    }

    pub fn collide(&mut self, other: &mut Ball, screen_size_x: i32, screen_size_y: i32, horizontal_cells: i32, vertical_cells: i32) -> ((i32, i32), (i32, i32), (i32, i32), (i32, i32)) {
        // Calculate the cells before the collision
        let self_current_cell = self.get_cell(screen_size_x, screen_size_y, horizontal_cells, vertical_cells);
        let other_current_cell = other.get_cell(screen_size_x, screen_size_y, horizontal_cells, vertical_cells);
        
        // Calculate the distance between the balls
        let distance = ((self.x - other.x).powf(2.0) + (self.y - other.y).powf(2.0)).sqrt();
        // Apply collision if the balls are touching
        if distance <= self.radius + other.radius {
            let original_vx = self.vx;
            let original_vy = self.vy;
            let contact_angle = (other.y - self.y).atan2(other.x - self.x).to_degrees();
            let contact_angle_cos = contact_angle.to_radians().cos();
            let contact_angle_sin = contact_angle.to_radians().sin();
            let contact_angle90_cos = -contact_angle_sin;
            let contact_angle90_sin = contact_angle_sin;

            // Apply direct collision
            self.vx = (self.v * (self.a - contact_angle).to_radians().cos() * (self.mass - other.mass) + 2.0 * other.mass * other.v * (other.a - contact_angle).to_radians().cos()) / (self.mass + other.mass) * contact_angle_cos + self.v * (self.a - contact_angle).to_radians().sin() * contact_angle90_cos;
            self.vy = (self.v * (self.a - contact_angle).to_radians().cos() * (self.mass - other.mass) + 2.0 * other.mass * other.v * (other.a - contact_angle).to_radians().cos()) / (self.mass + other.mass) * contact_angle_sin + self.v * (self.a - contact_angle).to_radians().sin() * contact_angle90_sin;
            other.vx = (other.v * (other.a - contact_angle).to_radians().cos() * (other.mass - self.mass) + 2.0 * self.mass * original_vx * (self.a - contact_angle).to_radians().cos()) / (self.mass + other.mass) * contact_angle_cos + other.v * (other.a - contact_angle).to_radians().sin() * contact_angle90_cos;
            other.vy = (other.v * (other.a - contact_angle).to_radians().cos() * (other.mass - self.mass) + 2.0 * self.mass * original_vy * (self.a - contact_angle).to_radians().cos()) / (self.mass + other.mass) * contact_angle_sin + other.v * (other.a - contact_angle).to_radians().sin() * contact_angle90_sin;
            self.a = self.vy.atan2(self.vx).to_degrees();
            other.a = other.vy.atan2(other.vx).to_degrees();
            self.v = (self.vx.powf(2.0) + self.vy.powf(2.0)).sqrt();
            other.v = (other.vx.powf(2.0) + other.vy.powf(2.0)).sqrt();

            // If the balls are overlapping, move them apart
            if distance < self.radius + other.radius {
                let distance_to_move = self.radius + other.radius - distance;
                self.x -= distance_to_move * contact_angle.to_radians().cos() * other.mass / (self.mass + other.mass);
                self.y -= distance_to_move * contact_angle.to_radians().sin() * other.mass / (self.mass + other.mass);
                other.x += distance_to_move * contact_angle.to_radians().cos() * self.mass / (self.mass + other.mass);
                other.y += distance_to_move * contact_angle.to_radians().sin() * self.mass / (self.mass + other.mass);
            }
        }
        let self_new_cell = self.get_cell(screen_size_x, screen_size_y, horizontal_cells, vertical_cells);
        let other_new_cell = other.get_cell(screen_size_x, screen_size_y, horizontal_cells, vertical_cells);
        return (self_current_cell, self_new_cell, other_current_cell, other_new_cell);
    }

    pub fn get_id(&self) -> i32 {
        self.id
    }

    pub fn get_x(&self) -> f64 {
        self.x
    }

    pub fn get_y(&self) -> f64 {
        self.y
    }

    pub fn get_radius(&self) -> f64 {
        self.radius
    }

    pub fn clone(&self) -> Ball {
        Ball {
            x: self.x,
            y: self.y,
            v: self.v,
            a: self.a,
            vx: self.vx,
            vy: self.vy,
            radius: self.radius,
            mass: self.mass,
            id: self.id,
        }
    }
}