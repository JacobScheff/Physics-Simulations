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
    
    // Initialize the balls
// balls = [[[] for j in range(verticalCells)] for i in range(horizontalCells)]
    let mut balls: Vec<Vec<ball::Ball>> = Vec::new();
    for i in 0..horizontal_cells {
        let mut row: Vec<ball::Ball> = Vec::new();
        for j in 0..vertical_cells {
            let x = i * horizontal_amount;
            let y = j * vertical_amount;
            let ball = ball::Ball::new(x, y, ball_size, screen_size);
            row.push(ball);
        }
        balls.push(row);
    }

    println!("Hello, world!");
}