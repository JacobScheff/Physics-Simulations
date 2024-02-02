mod ball;
extern crate sdl2;

fn main() {
    let screen_size: (i32, i32) = (1200, 600);
    let ball_size = 6;
    let horizontal_amount: i32 = 20;
    let vertical_amount: i32 = 15;
    let fps: i32 = 65;
    let horizontal_cells: i32 = 48;
    let vertical_cells: i32 = 24;
    // let gravity: i32 = 200;
    
    // Initialize a window to draw on
    // https://nercury.github.io/rust/opengl/tutorial/2018/02/08/opengl-in-rust-from-scratch-01-window.html
    let _sdl = sdl2::init().unwrap();

    println!("Hello, world!");
}