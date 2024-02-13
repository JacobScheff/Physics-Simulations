pub struct Vector {
    pub x: f64,
    pub y: f64,
}

impl Vector {
    pub fn new(x: f64, y: f64) -> Vector {
        Vector { x, y }
    }

    pub fn add(&self, other: &Vector) -> Vector {
        Vector {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }

    pub fn subtract(&self, other: &Vector) -> Vector {
        Vector {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }

    pub fn multiply(&self, scalar: f64) -> Vector {
        Vector {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }

    pub fn divide(&self, scalar: f64) -> Vector {
        Vector {
            x: self.x / scalar,
            y: self.y / scalar,
        }
    }

    pub fn dot_product(&self, other: &Vector) -> f64 {
        self.x * other.x + self.y * other.y
    }

    pub fn cross_product(&self, other: &Vector) -> f64 {
        self.x * other.y - self.y * other.x
    }

    pub fn get_magnitude(&self) -> f64 {
        (self.x.powf(2.0) + self.y.powf(2.0)).sqrt()
    }

    pub fn get_angle(&self) -> f64 {
        self.y.atan2(self.x).to_degrees()
    }

    pub fn normalize(&self) -> Vector {
        let magnitude = self.get_magnitude();
        if magnitude != 0.0 {
            return Vector::new(self.x / magnitude, self.y / magnitude);
        } else {
            self.clone()
        }
    }

    pub fn set_magnitude(&self, magnitude: f64) -> Vector {
        return self.normalize().multiply(magnitude).clone();
    }

    pub fn clone(&self) -> Vector {
        Vector {
            x: self.x,
            y: self.y,
        }
    }
}