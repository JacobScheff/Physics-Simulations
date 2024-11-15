struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
};

struct Grid {
    x: i32,
    y: i32,
}

struct Particle {
    position: vec2<f32>, // 16 bytes
    radius: f32, // 4 bytes
    velocity: vec2<f32>, // 8 bytes
    density: f32, // 4 bytes
    forces: vec4<f32>, // 16 bytes
}

const WORKGROUP_SIZE: u32 = 16;

const SCREEN_SIZE: vec2<f32> = vec2<f32>(1200.0, 600.0); // Size of the screen
const GRID_SIZE: vec2<f32> = vec2<f32>(80.0, 40.0);

const PARTICLE_AMOUNT_X: u32 = 96; // The number of particles in the x direction
const PARTICLE_AMOUNT_Y: u32 = 48; // The number of particles in the y direction
const TOTAL_PARTICLES: i32 = i32(PARTICLE_AMOUNT_X * PARTICLE_AMOUNT_Y); // The total number of particles
const RADIUS_OF_INFLUENCE: f32 = 75.0 / 4.0; // The radius of the sphere of influence. Also the radius to search for particles to calculate the density
const TARGET_DENSITY: f32 = 0.2; // The target density of the fluid
const PRESSURE_MULTIPLIER: f32 = 500.0; // The multiplier for the pressure force
const GRAVITY: f32 = 0.2; // The strength of gravity
const LOOK_AHEAD_TIME: f32 = 1.0 / 60.0; // The time to look ahead when calculating the predicted position
const VISCOSITY: f32 = 0.1; // The viscosity of the fluid
const DAMPENING: f32 = 0.95; // How much to slow down particles when they collide with the walls
const dt: f32 = 1.0 / 8.0; // The time step

const grids_to_check = vec2<i32>(i32(RADIUS_OF_INFLUENCE / SCREEN_SIZE.x * GRID_SIZE.x + 1.0), i32(RADIUS_OF_INFLUENCE / SCREEN_SIZE.y * GRID_SIZE.y + 1.0));
@group(0) @binding(0) var<storage, read_write> particles: array<Particle, u32(TOTAL_PARTICLES)>;
@group(0) @binding(1) var<storage, read> particle_lookup: array<i32, u32(GRID_SIZE.x * GRID_SIZE.y)>;
@group(0) @binding(2) var<storage, read> particle_counts: array<i32, u32(GRID_SIZE.x * GRID_SIZE.y)>;
@group(0) @binding(3) var<storage, read> mouse_info: array<f32, 4>; // 0-Up; 1-Down, x-pos, y-pos, 0-Repel; 1-Attract
const TEMP_PARTICLE_RADII: f32 = 2.5;

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), // Bottom Left
        vec2<f32>(1.0, -1.0),  // Bottom Right
        vec2<f32>(-1.0, 1.0),   // Top Left

        vec2<f32>(1.0, 1.0), // Top Right
        vec2<f32>(-1.0, 1.0), // Top Left
        vec2<f32>(1.0, -1.0) // Bottom Right
    );

    var out: VertexOutput;
    out.pos = vec4<f32>(positions[i], 0.0, 1.0);
    return out;
}


@compute @workgroup_size(WORKGROUP_SIZE, WORKGROUP_SIZE, 1)
fn main_density(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // let index = global_id.y * PARTICLE_AMOUNT_X + global_id.x;
    // if index < 0 || index >= u32(TOTAL_PARTICLES) {
    //     return;
    // }

    // // Update the density of the particle
    // let density = get_density(particle_positions[index]);
    // particle_densities[index] = density;
}

@compute @workgroup_size(WORKGROUP_SIZE, WORKGROUP_SIZE, 1)
fn main_forces(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // let index = global_id.y * PARTICLE_AMOUNT_X + global_id.x;
    // if index < 0 || index >= u32(TOTAL_PARTICLES) {
    //     return;
    // }
    
    // // Calculate the forces on the particle
    // particle_forces[index] = calculate_forces(index);
}

@compute @workgroup_size(WORKGROUP_SIZE, WORKGROUP_SIZE, 1)
fn main_move(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // let index = global_id.y * PARTICLE_AMOUNT_X + global_id.x;
    // if index < 0 || index >= u32(TOTAL_PARTICLES) {
    //     return;
    // }

    // // Move the particle
    // let force = particle_forces[index];
    // let radius = particle_radii[index];
    // let density = particle_densities[index];

    // var acceleration = vec2<f32>(force.xy / max(density, 0.0001));
    // acceleration += force.zw;
    // acceleration.y += GRAVITY;
    // // if density == 0.0 {
    // //     acceleration = vec2<f32>(0.0, GRAVITY);
    // // }

    // particle_velocities[index] += acceleration;
    // particle_positions[index] += particle_velocities[index] * dt;

    // // Collide with the walls
    // if particle_positions[index].x - radius < 0.0 {
    //     particle_positions[index].x = radius;
    //     particle_velocities[index].x = -particle_velocities[index].x * DAMPENING;
    // }
    // if particle_positions[index].x + radius > SCREEN_SIZE.x {
    //     particle_positions[index].x = SCREEN_SIZE.x - radius;
    //     particle_velocities[index].x = -particle_velocities[index].x * DAMPENING;
    // }
    // if particle_positions[index].y - radius < 0.0 {
    //     particle_positions[index].y = radius;
    //     particle_velocities[index].y = -particle_velocities[index].y * DAMPENING;
    // }
    // if particle_positions[index].y + radius > SCREEN_SIZE.y {
    //     particle_positions[index].y = SCREEN_SIZE.y - radius;
    //     particle_velocities[index].y = -particle_velocities[index].y * DAMPENING;
    // }
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    // 55.729168, 55.208332
    if(particles[1].position.x == 55.729168) {
        return vec4<f32>(0.0, 1.0, 0.0, 1.0);
    }
    else {
        return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }

    // let x: f32 = in.pos.x;
    // let y: f32 = in.pos.y;

    // var final_color: vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 1.0);

    // let grid = pos_to_grid(vec2<f32>(x, y));
    // for (var g: i32 = -2; g <= 2; g=g+1){
    //         var gx: i32 = g / 2;
    //         var gy: i32 = g % 2;
    //         if grid.x + gx < 0 || grid.x + gx >= i32(GRID_SIZE.x) || grid.y + gy < 0 || grid.y + gy >= i32(GRID_SIZE.y) {
    //             continue;
    //         }
    //         let first_grid_index = grid_to_index(grid_add(grid, Grid(gx, gy)));
    //         if first_grid_index < 0 || first_grid_index >= i32(GRID_SIZE.x * GRID_SIZE.y) {
    //             continue;
    //         }
            
    //         let starting_index = particle_lookup[first_grid_index];
    //         if starting_index == -1 {
    //             continue;
    //         }
            
    //         var ending_index = starting_index + particle_counts[first_grid_index];

    //         for (var i = starting_index; i <= ending_index; i=i+1){
    //             let d = (x - particles[i].position.x) * (x - particles[i].position.x) + (y - particles[i].position.y) * (y - particles[i].position.y);
    //             if d < TEMP_PARTICLE_RADII * TEMP_PARTICLE_RADII {
    //                 // // let speed = length(particle_velocities[i]);
    //                 // let speed: f32 = 10.0;
    //                 // // let density = particle_densities[i];
    //                 // let density: f32 = 10.0;

    //                 // // Create a gradient color
    //                 // let min_speed: f32 = 0.0;
    //                 // let max_speed: f32 = 12.0;
    //                 // var speed_t: f32 = (speed - min_speed) / (max_speed - min_speed);
    //                 // speed_t = min(max(speed_t, 0.0), 1.0);
    //                 // let min_density: f32 = 0.0;
    //                 // let max_density: f32 = 0.4;
    //                 // var density_t: f32 = (density - min_density) / (max_density - min_density);
    //                 // density_t = min(max(density_t, 0.0), 1.0);
    //                 // let color: vec3<f32> = vec3<f32>(speed_t, density_t, 1.0 - speed_t);

    //                 let color = vec3<f32>(0.0, 0.0, 1.0);
                    
    //                 final_color = vec4<f32>(color, 1.0);
    //                 break;
    //             }
    //         }
    // }

    // return final_color;
}

fn density_to_pressure(density: f32) -> f32 {
    let density_error = density - TARGET_DENSITY;
    return density_error * PRESSURE_MULTIPLIER;
}

fn smoothing_kernel(distance: f32) -> f32 {
    if distance >= RADIUS_OF_INFLUENCE {
        return 0.0;
    }

    let volume = 3.141592653589 * pow(RADIUS_OF_INFLUENCE, 4.0) / 6.0;
    return (RADIUS_OF_INFLUENCE - distance) * (RADIUS_OF_INFLUENCE - distance) / volume;
}

// fn get_density(pos: vec2<f32>) -> f32 {
//     let grid = pos_to_grid(pos);
//     var density = 0.0;

//     // for (var gx: i32 = -grids_to_check.x; gx <= grids_to_check.x; gx=gx+1){
//     //     for(var gy: i32 = -grids_to_check.y; gy <= grids_to_check.y; gy=gy+1){
//         for (var g: i32 = 0; g < (grids_to_check.x * 2 + 1) * (grids_to_check.y * 2 + 1); g=g+1){
//             let gx: i32 = g / (grids_to_check.y * 2 + 1) - grids_to_check.x;
//             let gy: i32 = g % (grids_to_check.y * 2 + 1) - grids_to_check.y;

//             if grid.x + gx < 0 || grid.x + gx >= i32(GRID_SIZE.x) || grid.y + gy < 0 || grid.y + gy >= i32(GRID_SIZE.y) {
//                 continue;
//             }
//             let first_grid_index = grid_to_index(grid_add(grid, Grid(gx, gy)));
//             if first_grid_index < 0 || first_grid_index >= i32(GRID_SIZE.x * GRID_SIZE.y) {
//                 continue;
//             }
            
//             let starting_index = particle_lookup[first_grid_index];
//             if starting_index == -1 {
//                 continue;
//             }
            
//             var ending_index = starting_index + particle_counts[first_grid_index];

//             for (var i: u32 = u32(starting_index); i <= u32(ending_index); i=i+1){
//                 let distance = length(pos - (particle_positions[i] + particle_velocities[i] * LOOK_AHEAD_TIME));
//                 if distance <= RADIUS_OF_INFLUENCE {
//                     let influence = smoothing_kernel(distance);
//                     density += influence * 3.141592653589 * TEMP_PARTICLE_RADII * TEMP_PARTICLE_RADII;
//                 }
//             }

//         }

//     //     }
//     // }

//     return density;
// }

fn pos_to_grid(pos: vec2<f32>) -> Grid {
    return Grid(
        max(min(i32(pos.x / SCREEN_SIZE.x * GRID_SIZE.x), i32(GRID_SIZE.x - 1)), 0),
        max(min(i32(pos.y / SCREEN_SIZE.y * GRID_SIZE.y), i32(GRID_SIZE.y - 1)), 0)
    );
}

fn grid_to_index(grid: Grid) -> i32 {
    return grid.y * i32(GRID_SIZE.x) + grid.x;
}

fn grid_add(grid: Grid, offset: Grid) -> Grid {
    return Grid(grid.x + offset.x, grid.y + offset.y);
}

// fn calculate_forces(index: u32) -> vec4<f32> {
//     var forces = vec4<f32>(0.0, 0.0, 0.0, 0.0);

//     // NOTE: index is already the index of the particle in the grid_index_map
//     let position: vec2<f32> = particle_positions[index] + particle_velocities[index] * LOOK_AHEAD_TIME;

//     let grid = pos_to_grid(position);

//     let density: f32 = particle_densities[index];

//     // for (var gx: i32 = -grids_to_check.x; gx <= grids_to_check.x; gx=gx+1){
//     //     for (var gy: i32 = -grids_to_check.y; gy <= grids_to_check.y; gy=gy+1){
//         for (var g: i32 = 0; g < (grids_to_check.x * 2 + 1) * (grids_to_check.y * 2 + 1); g=g+1){
//             let gx: i32 = g / (grids_to_check.y * 2 + 1) - grids_to_check.x;
//             let gy: i32 = g % (grids_to_check.y * 2 + 1) - grids_to_check.y;
            
//             if grid.x + gx < 0 || grid.x + gx >= i32(GRID_SIZE.x) || grid.y + gy < 0 || grid.y + gy >= i32(GRID_SIZE.y) {
//                 continue;
//             }
            
//             let first_grid_index: i32 = grid_to_index(grid_add(grid, Grid(gx, gy)));
//             if (first_grid_index < 0 || first_grid_index >= i32(GRID_SIZE.x * GRID_SIZE.y)) {
//                 continue;
//             }

//             let starting_index = particle_lookup[first_grid_index];
//             if starting_index == -1 {
//                 continue;
//             }

//             var ending_index: i32 = starting_index + particle_counts[first_grid_index];

//             for(var i: i32 = starting_index; i <= ending_index; i=i+1){
//                 if i == -1 || i == i32(index) || i >= i32(TOTAL_PARTICLES) {
//                     continue;
//                 }
//                 let offset: vec2<f32> = position - (particle_positions[i] + particle_velocities[i] * LOOK_AHEAD_TIME);
//                 let distance: f32 = sqrt(offset.x * offset.x + offset.y * offset.y);
//                 if distance == 0.0 || distance > RADIUS_OF_INFLUENCE {
//                     continue;
//                 }
//                 let dir = vec2<f32>(offset.x / distance, offset.y / distance);

//                 let slope = smoothing_kernel_derivative(distance);
//                 let other_density = particle_densities[i];
//                 let shared_pressure = calculate_shared_pressure(density, other_density);

//                 // Pressure force
//                 let pressure_force = dir * shared_pressure * slope * 3.141592653589 * particle_radii[i] * particle_radii[i] / max(density, 0.000001);
//                 // if density == 0.0 {
//                 //     continue;
//                 // }
                    
//                 // Viscosity force
//                 let viscosity_influence = viscosity_kernel(distance);
//                 var viscosity_force = (particle_velocities[i] - particle_velocities[index]) * viscosity_influence;
//                 viscosity_force *= VISCOSITY;

//                 // Apply the forces
//                 forces += vec4<f32>(pressure_force.x, pressure_force.y, viscosity_force.x, viscosity_force.y);
//             }
//     //     }
//     // }
//         }

//     // Check for mouse interaction
//     if mouse_info[0] == 1.0 || mouse_info[3] == 1.0 {
//         let mouse_pos = vec2<f32>(mouse_info[1], mouse_info[2]);
//         let offset = position - mouse_pos;
//         let distance = sqrt(offset.x * offset.x + offset.y * offset.y);
//         if distance < RADIUS_OF_INFLUENCE {
//             let dir = vec2<f32>(offset.x / distance, offset.y / distance);
//             var mouse_force = dir * smoothing_kernel(distance) * 100000.0;
//             if mouse_info[3] == 1.0 {
//                 mouse_force *= -0.005; // Attract
//             }
//             else {
//                 mouse_force *= 0.5; // Repel
//             }
//             forces += vec4<f32>(mouse_force.x, mouse_force.y, 0.0, 0.0);
//         }
//     }

//     return forces;

// }

fn smoothing_kernel_derivative(distance: f32) -> f32 {
    if distance >= RADIUS_OF_INFLUENCE {
        return 0.0;
    }

    let scale = 12.0 / (pow(RADIUS_OF_INFLUENCE, 4.0) * 3.141592653589);
    return (RADIUS_OF_INFLUENCE - distance) * scale;
}

fn viscosity_kernel(distance: f32) -> f32 {
    if distance >= RADIUS_OF_INFLUENCE {
        return 0.0;
    }

    let volume = 3.141592653589 * pow(RADIUS_OF_INFLUENCE, 8.0) / 4.0;
    let value = RADIUS_OF_INFLUENCE * RADIUS_OF_INFLUENCE - distance * distance;
    return value * value * value / volume;
}

fn calculate_shared_pressure(density_a: f32, density_b: f32) -> f32 {
    let pressure_a = density_to_pressure(density_a);
    let pressure_b = density_to_pressure(density_b);
    return (pressure_a + pressure_b) / 2.0;
}