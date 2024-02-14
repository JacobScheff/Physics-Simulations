use crate::vector::Vector;
use crate::SCREEN_SIZE;
use crate::HORIZONTAL_CELLS;
use crate::VERTICAL_CELLS;

pub struct Ball {
    x: f64,
    y: f64,
    velocity: Vector,
    radius: f64,
    mass: f64,
    id: i32,
}

impl Ball {
    pub fn new(x: f64, y: f64, velocity: Vector, radius: f64, id: i32) -> Ball {
        Ball {
            x,
            y,
            velocity,
            radius,
            mass: radius * radius * std::f64::consts::PI,
            id: id,
        }
    }

    pub fn move_ball(&mut self, dt: f64) {
        // Move the ball
        self.x += self.velocity.x * dt;
        self.y += self.velocity.y * dt;

        // Apply border collision
        if self.x < self.radius {
            self.x = self.radius;
            self.velocity.x = self.velocity.x.abs();
        } else if self.x > SCREEN_SIZE.0 as f64 - self.radius {
            self.x = SCREEN_SIZE.0 as f64 - self.radius;
            self.velocity.x = -self.velocity.x.abs();
        }
        if self.y < self.radius {
            self.y = self.radius;
            self.velocity.y = self.velocity.y.abs();
        } else if self.y > SCREEN_SIZE.1 as f64 - self.radius {
            self.y = SCREEN_SIZE.1 as f64 - self.radius;
            self.velocity.y = -self.velocity.y.abs();
        }
    }

    pub fn collide(mut self, other: &mut Ball) -> (Ball, Ball) {
        let distance: f64 = ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt();
        if distance == 0.0 {
            return (self, other.clone());
        }

        if distance <= self.radius + other.radius {
            let original_velocity_self: Vector = self.velocity.clone();
            let original_velocity_other: Vector = other.velocity.clone();

            let self_position: Vector = Vector::new(self.x, self.y);
            let other_position: Vector = Vector::new(other.x, other.y);

            let total_mass: f64 = self.mass + other.mass;

            self.velocity = original_velocity_self.subtract(&self_position.subtract(&other_position).normalize().multiply(2.0 * other.mass / total_mass).multiply(original_velocity_self.subtract(&original_velocity_other).dot_product(&self_position.subtract(&other_position))).divide(distance.powi(2)));
            other.velocity = original_velocity_other.subtract(&other_position.subtract(&self_position).normalize().multiply(2.0 * self.mass / total_mass).multiply(original_velocity_other.subtract(&original_velocity_self).dot_product(&other_position.subtract(&self_position))).divide(distance.powi(2)));

            if distance < self.radius + other.radius {
                let constact_angle: f64 = (self.y - other.y).atan2(self.x - other.x);
                let distance_to_move: f64 = (self.radius + other.radius - distance);
                self.x += distance_to_move * constact_angle.cos() * other.mass / total_mass;
                self.y += distance_to_move * constact_angle.sin() * other.mass / total_mass;
                other.x -= distance_to_move * constact_angle.cos() * self.mass / total_mass;
                other.y -= distance_to_move * constact_angle.sin() * self.mass / total_mass;
            }
        }

        return (self.clone(), other.clone());
    }

    pub fn get_cell(&self) -> (i32, i32) {
        let mut cell_x: i32 = (self.x / (SCREEN_SIZE.0 as f64 / HORIZONTAL_CELLS as f64)) as i32;
        cell_x = cell_x.clamp(0, HORIZONTAL_CELLS - 1);
        let mut cell_y: i32 = (self.y / (SCREEN_SIZE.1 as f64 / VERTICAL_CELLS as f64)) as i32;
        cell_y = cell_y.clamp(0, VERTICAL_CELLS - 1);
        (cell_x, cell_y)
    }

    pub fn get_cell_id(&self) -> i32 {
        let cell: (i32, i32) = self.get_cell();
        cell.0 + cell.1 * HORIZONTAL_CELLS
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

    pub fn get_mass(&self) -> f64 {
        self.mass
    }

    pub fn get_velocity(&self) -> Vector {
        self.velocity.clone()
    }

    pub fn set_x(&mut self, x: f64) {
        self.x = x;
    }

    pub fn set_y(&mut self, y: f64) {
        self.y = y;
    }

    pub fn set_velocity(&mut self, velocity: Vector) {
        self.velocity = velocity;
    }

    pub fn clone(&self) -> Ball {
        Ball {
            x: self.x,
            y: self.y,
            velocity: self.velocity.clone(),
            radius: self.radius,
            mass: self.mass,
            id: self.id,
        }
    }
    
}