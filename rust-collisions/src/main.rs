use std::thread::sleep;
use std::time::{Duration, Instant};
mod ball;

fn main() {
    let screen_size: (i32, i32) = (1200, 600);
    let ball_size = 6;
    let horizontal_amount: i32 = 20;
    let vertical_amount: i32 = 15;
    let fps: i32 = 65;
    let horizontal_cells: i32 = 48;
    let vertical_cells: i32 = 24;
    // let gravity: i32 = 200;
    
    // Create the balls array
    let mut balls: Vec<Vec<Vec<ball::Ball>>> = Vec::new();
    for i in 0..horizontal_cells {
        let mut row: Vec<Vec<ball::Ball>> = Vec::new();
        for j in 0..vertical_cells {
            row.push(Vec::new());
        }
        balls.push(row);
    }

    // Initialize the balls
    for i in 0..horizontal_amount {
        for j in 0..vertical_amount {
            // Create the ball
            let ball = ball::Ball::new(
                ((screen_size.0 - ball_size * 2) * i / horizontal_amount + ball_size) as f64,
                ((screen_size.1 - ball_size * 2) * j / vertical_amount + ball_size) as f64,
                0.0,
                0.0,
                ball_size as f64,
                (i * vertical_amount + j) as i32,
            );
            let cell = ball.get_cell(screen_size.0, screen_size.1, horizontal_cells, vertical_cells);
            balls[cell.0 as usize][cell.1 as usize].push(ball);
        }
    }

    // Never ending loop that runs at fps
    let mut last_time = Instant::now();
    loop {
        // Calculate the delta time
        let dt = last_time.elapsed().as_secs_f64();
        last_time = Instant::now();

        // Move the balls
        for x in 0..horizontal_cells {
            for y in 0..vertical_cells {
                for i in 0..balls[x as usize][y as usize].len() {
                    let mut ball = balls[x as usize][y as usize][i].clone();
                    let (current_cell, new_cell) = ball.move_ball(screen_size.0, screen_size.1, horizontal_cells, vertical_cells, 0.0, dt);
                    // Update the cells if the ball moved to a different cell
                    if current_cell != new_cell {
                        balls[current_cell.0 as usize][current_cell.1 as usize].retain(|b| b.get_id() != ball.get_id());
                        balls[new_cell.0 as usize][new_cell.1 as usize].push(ball);
                    }
                    else {
                        balls[current_cell.0 as usize][current_cell.1 as usize][i] = ball;
                    }
                }
            }
        }

        // Collision check the balls
        for x in 0..horizontal_cells {
            for y in 0..vertical_cells {
                // Iterate over the balls in the cell
                let mut i = 0;
                loop {
                    if(balls[x as usize][y as usize].len() == 0){
                        break;
                    }
                    // Iterate over the balls in the same cell or the adjacent cells
                    for j in -1..2 {
                        for k in -1..2 {
                            if x + j >= 0 && x + j < horizontal_cells && y + k >= 0 && y + k < vertical_cells {
                                let mut l = 0;
                                loop{
                                    if(balls[(x + j) as usize][(y + k) as usize].len() == 0){
                                        break;
                                    }
                                    // Create a copy of the balls to avoid borrowing issues
                                    let mut ball = balls[x as usize][y as usize][i].clone();
                                    let mut other_ball = balls[(x + j) as usize][(y + k) as usize][l].clone();
                                    // Check ifthe balls are different
                                    if ball.get_id() == other_ball.get_id() {
                                        continue;
                                    }
                                    // Check for collisions
                                    let (current_cell, new_cell, other_current_cell, other_new_cell) = ball.collide(&mut other_ball, screen_size.0, screen_size.1, horizontal_cells, vertical_cells);
                                    // Update the cells if the ball moved to a different cell or update the ball's state
                                    if current_cell != new_cell {
                                        balls[current_cell.0 as usize][current_cell.1 as usize].retain(|b| b.get_id() != ball.get_id());
                                        balls[new_cell.0 as usize][new_cell.1 as usize].push(ball);
                                    }
                                    else {
                                        balls[current_cell.0 as usize][current_cell.1 as usize][i] = ball;
                                    }
                                    if other_current_cell != other_new_cell {
                                        balls[other_current_cell.0 as usize][other_current_cell.1 as usize].retain(|b| b.get_id() != other_ball.get_id());
                                        balls[other_new_cell.0 as usize][other_new_cell.1 as usize].push(other_ball);
                                    }
                                    else {
                                        balls[other_current_cell.0 as usize][other_current_cell.1 as usize][l] = other_ball;
                                    }
                                    l += 1;
                                    if l >= balls[(x + j) as usize][(y + k) as usize].len() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    i += 1;
                    if i >= balls[x as usize][y as usize].len() {
                        break;
                    }
                }
            }
        }

        // for x in 0..horizontal_cells {
        //     for y in 0..vertical_cells {
        //         for &mut ball in balls[x as usize][y as usize] {
        //             // Check for collisions with the balls in the same cell or the adjacent cells
        //             for i in -1..2 {
        //                 for j in -1..2 {
        //                     if x + i >= 0 && x + i < horizontal_cells && y + j >= 0 && y + j < vertical_cells {
        //                         for &mut other_ball in balls[(x + i) as usize][(y + j) as usize] {
        //                             if ball.getId() != other_ball.getId() {
        //                                 let (current_cell, new_cell, other_current_cell, other_new_cell) = ball.collide(other_ball, screen_size.0, screen_size.1, horizontal_cells, vertical_cells);
        //                                 if current_cell != new_cell {
        //                                     // balls[current_cell.0 as usize][current_cell.1 as usize].retain(|b| b != ball);
        //                                     // balls[new_cell.0 as usize][new_cell.1 as usize].push(ball.clone());
        //                                 }
        //                                 if other_current_cell != other_new_cell {
        //                                     // balls[other_current_cell.0 as usize][other_current_cell.1 as usize].retain(|b| b != other_ball);
        //                                     // balls[other_new_cell.0 as usize][other_new_cell.1 as usize].push(other_ball.clone());
        //                                 }
        //                             }
        //                         }
        //                     }
        //                 }
        //             }
        //         }
        //     }
        // }

        sleep(Duration::from_millis(1000 / fps as u64));
    }
}