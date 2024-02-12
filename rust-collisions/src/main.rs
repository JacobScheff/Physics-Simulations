// use macroquad::miniquad::window::set_window_size;
// use macroquad::prelude::*;
use std::thread::sleep;
use std::time::{Duration, Instant};
mod vector;
mod ball;

const screen_size: (i32, i32) = (1200, 600);
const fps: i32 = 120;
const horizontal_cells: i32 = 48;
const vertical_cells: i32 = 24;
const ball_size: i32 = 6;
const horizontal_amount: i32 = 16;
const vertical_amount: i32 = 12;
const balls: Vec<ball::Ball> = Vec::new();

// Get dot product of two vectors with magnitude and angle
fn dot_product(v1: f64, a1: f64, v2: f64, a2: f64) -> f64 {
    v1 * v2 * (a2 - a1).to_radians().cos()
}

// #[macroquad::main("BasicShapes")]
fn main() {
    
    
}