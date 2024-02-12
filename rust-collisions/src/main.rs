// use macroquad::miniquad::window::set_window_size;
// use macroquad::prelude::*;
use std::thread::sleep;
use std::time::{Duration, Instant};
mod vector;
mod ball;

const SCREEN_SIZE: (i32, i32) = (1200, 600);
const FPS: i32 = 120;
const HORIZONTAL_CELLS: i32 = 48;
const VERTICAL_CELLS: i32 = 24;
const BALL_SIZE: i32 = 6;
const HORIZONTAL_AMOUNT: i32 = 16;
const VERTICAL_AMOUNT: i32 = 12;

// Get dot product of two vectors with magnitude and angle
fn dot_product(v1: f64, a1: f64, v2: f64, a2: f64) -> f64 {
    v1 * v2 * (a2 - a1).to_radians().cos()
}

// #[macroquad::main("BasicShapes")]
fn main() {
    let mut balls: Vec<ball::Ball> = Vec::new();
    
}