use crate::vector::Vector;
use crate::screen_size;
use crate::fps;
use crate::horizontal_cells;
use crate::vertical_cells;
use crate::ball_size;
use crate::horizontal_amount;
use crate::vertical_amount;
use crate::balls;

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
        } else if self.x > screen_size.0 as f64 - self.radius {
            self.x = screen_size.0 as f64 - self.radius;
            self.velocity.x = -self.velocity.x.abs();
        }
        if self.y < self.radius {
            self.y = self.radius;
            self.velocity.y = self.velocity.y.abs();
        } else if self.y > screen_size.1 as f64 - self.radius {
            self.y = screen_size.1 as f64 - self.radius;
            self.velocity.y = -self.velocity.y.abs();
        }
    }

    pub fn collide(&mut self, other: &mut Ball) {
        let distance: f64 = ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt();
        if distance == 0.0 {
            return;
        }

        if distance <= self.radius + other.radius {
            let original_velocity_self: Vector = self.velocity.clone();
            let original_velocity_other: Vector = other.velocity.clone();

            let self_position: Vector = Vector::new(self.x, self.y);
            let other_position: Vector = Vector::new(other.x, other.y);

            let total_mass: f64 = self.mass + other.mass;

            self.velocity = original_velocity_self.subtract(&self_position.subtract(&other_position).normalize().multiply(2.0 * other.mass / total_mass).multiply(original_velocity_other.subtract(&original_velocity_other).dot_product(&self_position.subtract(&other_position))).divide(distance.powi(2)));
            other.velocity = original_velocity_other.subtract(&other_position.subtract(&self_position).normalize().multiply(2.0 * self.mass / total_mass).multiply(original_velocity_self.subtract(&original_velocity_self).dot_product(&other_position.subtract(&self_position))).divide(distance.powi(2)));

            if distance < self.radius + other.radius {
                let constactAngle: f64 = (self.y - other.y).atan2(self.x - other.x);
                let distanceToMove: f64 = (self.radius + other.radius - distance);
                self.x += distanceToMove * constactAngle.cos() * other.mass / total_mass;
                self.y += distanceToMove * constactAngle.sin() * other.mass / total_mass;
                other.x -= distanceToMove * constactAngle.cos() * self.mass / total_mass;
                other.y -= distanceToMove * constactAngle.sin() * self.mass / total_mass;
            }
        }
    }

    pub fn getCell(&self) -> (i32, i32) {
        let mut cell_x: i32 = (self.x / (screen_size.0 as f64 / horizontal_cells as f64)) as i32;
        cell_x = cell_x.clamp(0, horizontal_cells - 1);
        let mut cell_y: i32 = (self.y / (screen_size.1 as f64 / vertical_cells as f64)) as i32;
        cell_y = cell_y.clamp(0, vertical_cells - 1);
        (cell_x, cell_y)
    }

    pub fn getCellId(&self) -> i32 {
        let cell: (i32, i32) = self.getCell();
        cell.0 + cell.1 * horizontal_cells
    }
    
}