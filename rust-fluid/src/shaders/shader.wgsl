struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
};

struct Grid {
    x: i32,
    y: i32,
}

const WORKGROUP_SIZE: u32 = 10;
const DISPATCH_SIZE: vec2<u32> = vec2<u32>(
    u32(PARTICLE_AMOUNT_X + WORKGROUP_SIZE - 1u) / u32(WORKGROUP_SIZE),
    u32(PARTICLE_AMOUNT_Y + WORKGROUP_SIZE - 1u) / u32(WORKGROUP_SIZE),
);

const SCREEN_SIZE: vec2<f32> = vec2<f32>(1200.0, 600.0); // Size of the screen
const GRID_SIZE: vec2<f32> = vec2<f32>(40.0, 20.0);

const PARTICLE_RADIUS: f32 = 1.25; // The radius of the particles
const PARTICLE_AMOUNT_X: u32 = 192; // The number of particles in the x direction
const PARTICLE_AMOUNT_Y: u32 = 96; // The number of particles in the y direction
const TOTAL_PARTICLES: i32 = i32(PARTICLE_AMOUNT_X * PARTICLE_AMOUNT_Y); // The total number of particles
const RADIUS_OF_INFLUENCE: f32 = 75.0; // MUST BE DIVISIBLE BY SCREEN_SIZE - The radius of the sphere of influence. Also the radius to search for particles to calculate the density
const TARGET_DENSITY: f32 = 0.6; // The target density of the fluid
const PRESURE_MULTIPLIER: f32 = 100.0; // The multiplier for the pressure force
const GRAVITY: f32 = 0.4; // The strength of gravity
const LOOK_AHEAD_TIME: f32 = 0.0; // 1.0 / 60.0; // The time to look ahead when calculating the predicted position
const VISCOSITY: f32 = 0.5; // The viscosity of the fluid
const DAMPENING: f32 = 0.95; // How much to slow down particles when they collide with the walls

const grids_to_check = vec2<i32>(i32(RADIUS_OF_INFLUENCE / SCREEN_SIZE.x * GRID_SIZE.x + 0.5), i32(RADIUS_OF_INFLUENCE / SCREEN_SIZE.y * GRID_SIZE.y + 0.5));
@group(0) @binding(0) var<storage, read_write> particle_positions: array<vec2<f32>, u32(TOTAL_PARTICLES)>;
@group(0) @binding(1) var<storage, read_write> particle_radii: array<f32, u32(TOTAL_PARTICLES)>;
@group(0) @binding(2) var<storage, read_write> particle_velocities: array<vec2<f32>, u32(TOTAL_PARTICLES)>;
@group(0) @binding(3) var<storage, read_write> particle_lookup: array<i32, u32(GRID_SIZE.x * GRID_SIZE.y)>;
@group(0) @binding(4) var<storage, read_write> particle_densities: array<f32, u32(TOTAL_PARTICLES)>;
@group(0) @binding(5) var<storage, read_write> particle_forces: array<vec4<f32>, u32(TOTAL_PARTICLES)>;
@group(0) @binding(6) var<storage, read_write> grid_index_map: array<array<i32, 2>, u32(TOTAL_PARTICLES)>;

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
    let index = global_id.y * PARTICLE_AMOUNT_X + global_id.x;
    if index < 0 || index >= u32(TOTAL_PARTICLES) {
        return;
    }

    // Update the density of the particle
    let density = get_density(particle_positions[index]);
    particle_densities[index] = density;
}

@compute @workgroup_size(WORKGROUP_SIZE, WORKGROUP_SIZE, 1)
fn main_forces(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.y * PARTICLE_AMOUNT_X + global_id.x;
    if index < 0 || index >= u32(TOTAL_PARTICLES) {
        return;
    }
    
    // Calculate the forces on the particle
    calculate_forces(index);
}

@compute @workgroup_size(1, 1, 1)
fn main_sort(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // Create a new list of particles
    var lookup_table: array<i32, i32(GRID_SIZE.x * GRID_SIZE.y)>;

    // Create a map of the particles' indices and grid's indices
    for (var i: i32 = 0; i < TOTAL_PARTICLES; i=i+1){
        let grid = pos_to_grid(particle_positions[i]);
        let grid_index = grid_to_index(grid);
        grid_index_map[i] = array<i32, 2>(grid_index, i);

        // Reset the forces
        particle_forces[i] = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    // Bubble sort the particles
    // for (var i: i32 = 0; i < TOTAL_PARTICLES - 1; i=i+1){
    //     for (var j: i32 = 0; j < TOTAL_PARTICLES - i - 1; j=j+1){
    //         if grid_index_map[j][0] > grid_index_map[j + 1][0] {
    //             let temp = grid_index_map[j];
    //     //         let temp_pos = particle_positions[j];
    //     //         let temp_vel = particle_velocities[j];
    //             let temp_rad = particle_radii[j];
    //     //         let temp_den = particle_densities[j];

    //             // grid_index_map[j] = grid_index_map[j + 1];
    //     //         particle_positions[j] = particle_positions[j + 1];
    //     //         particle_velocities[j] = particle_velocities[j + 1];
    //     //         particle_radii[j] = particle_radii[j + 1];
    //     //         particle_densities[j] = particle_densities[j + 1];

    //             // grid_index_map[j + 1] = temp;
    //     //         particle_positions[j + 1] = temp_pos;
    //     //         particle_velocities[j + 1] = temp_vel;
    //             // particle_radii[j + 1] = temp_rad;
    //     //         particle_densities[j + 1] = temp_den;
    //         }
    //     }
    // }

    // // Initialize the new lookup table
    // for (var i: i32 = 0; i < i32(GRID_SIZE.x * GRID_SIZE.y); i=i+1){
    //     lookup_table[i] = -1;
    // }

    // Create the new lookup table
    // var last_grid_index = -1;
    // for (var i: i32 = 0; i < TOTAL_PARTICLES; i=i+1){
    //     let grid_index = i32(grid_index_map[i]);
    //     if grid_index != last_grid_index {
    //         lookup_table[grid_index] = i;
    //         last_grid_index = grid_index;
    //     }
    // }
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let x: f32 = in.pos.x;
    let y: f32 = in.pos.y;

    let grid = pos_to_grid(vec2<f32>(x, y));
    for (var gx: i32 = -1; gx <= 1; gx=gx+1){
        for(var gy: i32 = -1; gy <=1; gy=gy+1){
            if grid.x + gx < 0 || grid.x + gx >= i32(GRID_SIZE.x) || grid.y + gy < 0 || grid.y + gy >= i32(GRID_SIZE.y) {
                continue;
            }
            let first_grid_index = grid_to_index(grid_add(grid, Grid(gx, gy)));
            if first_grid_index < 0 || first_grid_index >= i32(GRID_SIZE.x * GRID_SIZE.y) {
                continue;
            }
            
            let starting_index = particle_lookup[first_grid_index];
            if starting_index == -1 {
                continue;
            }
            
            var ending_index = -1;

            // let next_grid_index = first_grid_index + 1;
            for (var i = first_grid_index + 1; i < i32(GRID_SIZE.x * GRID_SIZE.y); i=i+1){
                if particle_lookup[i] != -1 {
                    ending_index = particle_lookup[i];
                    break;
                }
            }
            if ending_index == -1 {
                ending_index = i32(TOTAL_PARTICLES);
            }

            for (var i = starting_index; i < ending_index; i=i+1){
                let d = (x - particle_positions[i].x) * (x - particle_positions[i].x) + (y - particle_positions[i].y) * (y - particle_positions[i].y);
                if d < particle_radii[i] * particle_radii[i] {
                    let speed = length(particle_velocities[i]);
                    let density = particle_densities[i];

                    // Create a gradient color
                    let min_speed: f32 = 0.0;
                    let max_speed: f32 = 20.0;
                    var speed_t: f32 = (speed - min_speed) / (max_speed - min_speed);
                    speed_t = min(max(speed_t, 0.0), 1.0);
                    let min_density: f32 = 0.0;
                    let max_density: f32 = 1.6;
                    var density_t: f32 = (density - min_density) / (max_density - min_density);
                    density_t = min(max(density_t, 0.0), 1.0);
                    let color: vec3<f32> = vec3<f32>(speed_t, density_t, 1.0 - speed_t);
                    // let color: vec3<f32> = vec3<f32>(0.2, density_t, 0.2);
                    // let density_error = density - TARGET_DENSITY;
                    // var color: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
                    // if density_error > 0.0 {
                    //     color = vec3<f32>(density_error, 0.0, 0.0);
                    // } else {
                    //     color = vec3<f32>(0.0, 0.0, -density_error);
                    // }
                    
                    return vec4<f32>(color, 1.0);
                }
            }

        }
    }

    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}

fn density_to_pressure(density: f32) -> f32 {
    let density_error = density - TARGET_DENSITY;
    return density_error * PRESURE_MULTIPLIER;
}

fn smoothing_kernel(distance: f32) -> f32 {
    if distance >= RADIUS_OF_INFLUENCE {
        return 0.0;
    }

    let volume = 3.141592653589 * pow(RADIUS_OF_INFLUENCE, 4.0) / 6.0;
    return (RADIUS_OF_INFLUENCE - distance) * (RADIUS_OF_INFLUENCE - distance) / volume;
}

fn get_density(pos: vec2<f32>) -> f32 {
    let grid = pos_to_grid(pos);
    var density = 0.0;

    for (var gx: i32 = -grids_to_check.x; gx <= grids_to_check.x; gx=gx+1){
        for(var gy: i32 = -grids_to_check.y; gy <= grids_to_check.y; gy=gy+1){
            if grid.x + gx < 0 || grid.x + gx >= i32(GRID_SIZE.x) || grid.y + gy < 0 || grid.y + gy >= i32(GRID_SIZE.y) {
                continue;
            }
            let first_grid_index = grid_to_index(grid_add(grid, Grid(gx, gy)));
            if first_grid_index < 0 || first_grid_index >= i32(GRID_SIZE.x * GRID_SIZE.y) {
                continue;
            }
            
            let starting_index = particle_lookup[first_grid_index];
            if starting_index == -1 {
                continue;
            }
            
            var ending_index = -1;

            // let next_grid_index = first_grid_index + 1;
            for (var i = first_grid_index + 1; i < i32(GRID_SIZE.x * GRID_SIZE.y); i=i+1){
                if particle_lookup[i] != -1 {
                    ending_index = particle_lookup[i];
                    break;
                }
            }
            if ending_index == -1 {
                ending_index = i32(TOTAL_PARTICLES);
            }

            for (var i = starting_index; i < ending_index; i=i+1){
                let distance = length(pos - (particle_positions[i] + particle_velocities[i] * LOOK_AHEAD_TIME));
                if distance <= RADIUS_OF_INFLUENCE {
                    if distance == 0.0 {
                        continue;
                    }
                    let influence = smoothing_kernel(distance);
                    density += influence * 3.141592653589 * particle_radii[i] * particle_radii[i];
                }
            }

        }
    }

    return density;
}

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

fn calculate_forces(index: u32) {
        let position: vec2<f32> = particle_positions[index] + particle_velocities[index] * LOOK_AHEAD_TIME;

        let grid = pos_to_grid(position);

        let density: f32 = particle_densities[index];

        for (var gx: i32 = -grids_to_check.x; gx <= grids_to_check.x; gx=gx+1){
            for (var gy: i32 = -grids_to_check.y; gy <= grids_to_check.y; gy=gy+1){
                let first_grid_index: i32 = grid_to_index(grid_add(grid, Grid(gx, gy)));
                if (first_grid_index < 0 || first_grid_index >= i32(GRID_SIZE.x * GRID_SIZE.y)) {
                    continue;
                }

                let starting_index = particle_lookup[first_grid_index];
                var ending_index: i32 = -1;

                let next_grid_index: i32 = first_grid_index + 1;
                if next_grid_index >= i32(GRID_SIZE.x * GRID_SIZE.y) {
                    ending_index = i32(TOTAL_PARTICLES);
                } else {
                    ending_index = particle_lookup[next_grid_index];
                }

                for(var i: i32 = starting_index; i < ending_index; i=i+1){
                    if i == -1 || i == i32(index) || i >= i32(TOTAL_PARTICLES) {
                        continue;
                    }
                    let offset: vec2<f32> = position - (particle_positions[i] + particle_velocities[i] * LOOK_AHEAD_TIME);
                    let distance = sqrt(offset.x * offset.x + offset.y * offset.y);
                    if distance == 0.0 {
                        continue;
                    }
                    let dir = vec2<f32>(offset.x / distance, offset.y / distance);

                    let slope = smoothing_kernel_derivative(distance);
                    let other_density = particle_densities[i];
                    let shared_pressure = calculate_shared_pressure(density, other_density);

                    // Pressure force
                    let pressure_force = dir * shared_pressure * slope * 3.141592653589 * PARTICLE_RADIUS * PARTICLE_RADIUS / max(density, 0.000001);
                    if density == 0.0 {
                        continue;
                    }

                    // Viscosity force
                    let viscosity_influence = viscosity_kernel(distance);
                    let viscosity_force = (particle_velocities[i] - particle_velocities[index]) * viscosity_influence;

                    // Apply the forces
                    particle_forces[index] += vec4<f32>(pressure_force.x, pressure_force.y, viscosity_force.x, viscosity_force.y);
                    particle_forces[i] -= vec4<f32>(pressure_force.x, pressure_force.y, viscosity_force.x, viscosity_force.y);
                }
            }
        }

    }

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