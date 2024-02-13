// use macroquad::miniquad::window::set_window_size;
// use macroquad::prelude::*;
use std::thread::sleep;
use std::time::{Duration, Instant};
mod ball;
mod vector;

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

// Binary search functions
fn binary_search_ball_index_first(
    arr: &Vec<ball::Ball>,
    target_cell_id: i32,
    mut start: i32,
    mut end: i32,
) -> i32 {
    let mut loops = 0;

    while (start <= end) {
        loops += 1;
        let mid = ((start + end) / 2) as i32;
        if arr[mid as usize].get_cell_id() == target_cell_id {
            // Get the first index with the same cell id
            for i in (0..=mid).rev() {
                if arr[i as usize].get_cell_id() != target_cell_id {
                    return i + 1;
                }
            }
            return 0;
        }
        else if arr[mid as usize].get_cell_id() < target_cell_id {
            start = mid + 1;
        } else {
            end = mid - 1;
        }
    }

    return -1;
}

fn binary_search_ball_index_last(
    arr: &Vec<ball::Ball>,
    target_cell_id: i32,
    mut start: i32,
    mut end: i32,
) -> i32 {
    let mut loops = 0;

    while (start <= end) {
        loops += 1;
        let mid = ((start + end) / 2) as i32;
        if arr[mid as usize].get_cell_id() == target_cell_id {
            // Get the first index with the same cell id
            for i in (mid..arr.len() as i32) {
                if arr[i as usize].get_cell_id() != target_cell_id {
                    return i - 1;
                }
            }
            return arr.len() as i32 - 1;
        }
        else if arr[mid as usize].get_cell_id() < target_cell_id {
            start = mid + 1;
        } else {
            end = mid - 1;
        }
    }

    return -1;
}

// Insertion sort the balls (possibly can use binary sort to make this even faster)
fn sort_balls(mut arr: Vec<ball::Ball>, mut keys: Vec<Vec<i32>>) -> (Vec<ball::Ball>, Vec<Vec<i32>>) {
    for i in 1..arr.len() {
        let key = arr[i].clone();
        let key_id = key.get_cell_id();
        let mut j = i - 1;
        while j >= 0 && arr[j].get_cell_id() > key_id {
            arr[j + 1] = arr[j].clone();
            j -= 1;
        }
        arr[j + 1] = key.clone();
    }

    // Update the ball index key
    let mut start_index = 0;
    for i in 0..HORIZONTAL_CELLS*VERTICAL_CELLS {
        let found_index_start = binary_search_ball_index_first(&arr, i, start_index, arr.len() as i32 - 1);
        let found_index_end = binary_search_ball_index_last(&arr, i, start_index, arr.len() as i32 - 1);
        if found_index_start != -1 {
            start_index = found_index_start;
        }
        keys[i as usize][0] = found_index_start;
        keys[i as usize][1] = found_index_end;
    }

    return (arr, keys);
}

// #[macroquad::main("BasicShapes")]
fn main() {
    // Create the balls list
    let mut balls: Vec<ball::Ball> = Vec::new();
    let mut ball_index_key: Vec<Vec<i32>> = vec![vec![-1, -1]; HORIZONTAL_CELLS as usize * VERTICAL_CELLS as usize];

    for i in 0..HORIZONTAL_AMOUNT {
        for j in 0..VERTICAL_AMOUNT {
            let x = (SCREEN_SIZE.0 - BALL_SIZE * 2) * i / HORIZONTAL_AMOUNT + BALL_SIZE;
            let y = (SCREEN_SIZE.1 - BALL_SIZE * 2) * j / VERTICAL_AMOUNT + BALL_SIZE;
            let velocity = vector::Vector::new(0.0, 0.0);
            balls.push(ball::Ball::new(
                x as f64,
                y as f64,
                velocity,
                BALL_SIZE as f64,
                (i * VERTICAL_AMOUNT + j) as i32,
            ));
        }
    }

    balls.push(ball::Ball::new(
        1120.0,
        500.0,
        vector::Vector::new(-800.0, 400.0),
        40.0,
        HORIZONTAL_AMOUNT * VERTICAL_AMOUNT as i32,
    ));

    
}
