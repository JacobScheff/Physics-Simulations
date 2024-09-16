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
const FOV: f32 = 60.0 * 3.14159 / 180.0; // Field of view in radians
const ASPECT_RATIO: f32 = SCREEN_SIZE.x / SCREEN_SIZE.y; // Aspect ratio of the screen
const GRID_SIZE: vec2<f32> = vec2<f32>(4.0, 2.0);

const PARTICLE_RADIUS: f32 = 10.0; // The radius of the particles
const PARTICLE_AMOUNT_X: u32 = 4; // The number of particles in the x direction
const PARTICLE_AMOUNT_Y: u32 = 2; // The number of particles in the y direction
const RADIUS_OF_INFLUENCE: f32 = 75.0; // MUST BE DIVISIBLE BY SCREEN_SIZE - The radius of the sphere of influence. Also the radius to search for particles to calculate the density
const TARGET_DENSITY: f32 = 0.2; // The target density of the fluid
const PRESURE_MULTIPLIER: f32 = 1.0; //500.0; // The multiplier for the pressure force
const GRAVITY: f32 = 0.02; // The strength of gravity
const LOOK_AHEAD_TIME: f32 = 1.0 / 60.0; // The time to look ahead when calculating the predicted position
const VISCOSITY: f32 = 0.5; // The viscosity of the fluid
const DAMPENING: f32 = 0.95; // How much to slow down particles when they collide with the walls

const grids_to_check = vec2<i32>(i32(SCREEN_SIZE.x / RADIUS_OF_INFLUENCE + 0.5), i32(SCREEN_SIZE.y / RADIUS_OF_INFLUENCE + 0.5));
// TODO: Cache density for particles
@group(0) @binding(0) var<storage, read> frame_count: u32;
@group(0) @binding(1) var<storage, read_write> particle_positions: array<vec2<f32>, u32(PARTICLE_AMOUNT_X * PARTICLE_AMOUNT_Y)>;
@group(0) @binding(2) var<storage, read> particle_radii: array<f32>;
@group(0) @binding(3) var<storage, read_write> particle_velocities: array<vec2<f32>, u32(PARTICLE_AMOUNT_X * PARTICLE_AMOUNT_Y)>;
@group(0) @binding(4) var<storage, read> particle_lookup: array<i32, u32(GRID_SIZE.x * GRID_SIZE.y)>;

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
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.y * PARTICLE_AMOUNT_X + global_id.x;
    if index < 0 || index >= u32(PARTICLE_AMOUNT_X * PARTICLE_AMOUNT_Y) {
        return;
    }

    // let pos: vec2<f32> = particle_positions[index];
    // let vel = particle_velocities[index];
    // let mass = 3.14159265359 * particle_radii[index] * particle_radii[index];
    // let radius = particle_radii[index];

    let forces: vec4<f32> = calculate_forces(index);
    let pressure_force: vec2<f32> = forces.xy;
    let viscosity_force: vec2<f32> = forces.zw;
    let gravity_force: vec2<f32> = vec2<f32>(0.0, GRAVITY);

    var particle_acceleration: vec2<f32> = (pressure_force) / max(get_density(particle_positions[index]), 0.000001);
    particle_acceleration = particle_acceleration + viscosity_force;
    particle_acceleration = particle_acceleration + gravity_force;
    particle_velocities[index] = particle_velocities[index] + particle_acceleration;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let x: f32 = in.pos.x;
    let y: f32 = in.pos.y;

    let grid = pos_to_grid(vec2<f32>(x, y));
    for (var gx: i32 = -1; gx <= 1; gx=gx+1){
        for(var gy: i32 = -1; gy <=1; gy=gy+1){
            let first_grid_index = grid_to_index(grid_add(grid, Grid(gx, gy)));
            if first_grid_index < 0 || first_grid_index >= i32(GRID_SIZE.x * GRID_SIZE.y) {
                continue;
            }
            
            let starting_index = particle_lookup[first_grid_index];
            var ending_index = -1;

            let next_grid_index = first_grid_index + 1;
            if next_grid_index >= i32(GRID_SIZE.x * GRID_SIZE.y) {
                ending_index = i32(PARTICLE_AMOUNT_X * PARTICLE_AMOUNT_Y);
            }
            else {
                ending_index = particle_lookup[next_grid_index];
            }

            for (var i = starting_index; i < ending_index; i=i+1){
                let d = (x - particle_positions[i].x) * (x - particle_positions[i].x) + (y - particle_positions[i].y) * (y - particle_positions[i].y);
                if d < particle_radii[i] * particle_radii[i] {
                    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
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

fn get_density(pos: vec2<f32>) -> f32 {
    let grid = pos_to_grid(pos);

    var density = 0.0;

    for (var gx: i32 = -grids_to_check.x; gx <= grids_to_check.x; gx=gx+1){
        for(var gy: i32 = -grids_to_check.y; gy <=grids_to_check.y; gy=gy+1){
            let first_grid_index = grid_to_index(grid_add(grid, Grid(gx, gy)));
            if first_grid_index < 0 || first_grid_index >= i32(GRID_SIZE.x * GRID_SIZE.y) {
                continue;
            }
            
            let starting_index = particle_lookup[first_grid_index];
            var ending_index = -1;

            let next_grid_index = first_grid_index + 1;
            if next_grid_index >= i32(GRID_SIZE.x * GRID_SIZE.y) {
                ending_index = i32(PARTICLE_AMOUNT_X * PARTICLE_AMOUNT_Y);
            }
            else {
                ending_index = particle_lookup[next_grid_index];
            }

            for (var i = starting_index; i < ending_index; i=i+1){
                let distance = length(pos - (particle_positions[i] + particle_velocities[i] * LOOK_AHEAD_TIME));
                let influence = smoothing_kernel(distance);
                density += influence * 3.141592653589 * particle_radii[i] * particle_radii[i];
            }

        }
    }

    return density;
}

fn calculate_shared_pressure(density_a: f32, density_b: f32) -> f32 {
    let pressure_a = density_to_pressure(density_a);
    let pressure_b = density_to_pressure(density_b);
    return (pressure_a + pressure_b) / 2.0;
}

fn calculate_forces(index: u32) -> vec4<f32> {
    var pressure_force = vec2<f32>(0.0, 0.0);
    var viscosity_force = vec2<f32>(0.0, 0.0);
    let position = particle_positions[index] + particle_velocities[index] * LOOK_AHEAD_TIME;

    let grid = pos_to_grid(position);

    let density = get_density(position);

    for (var gx: i32 = -grids_to_check.x; gx <= grids_to_check.x; gx=gx+1){
        for(var gy: i32 = -grids_to_check.y; gy <=grids_to_check.y; gy=gy+1){
            let first_grid_index = grid_to_index(grid_add(grid, Grid(gx, gy)));
            if first_grid_index < 0 || first_grid_index >= i32(GRID_SIZE.x * GRID_SIZE.y) {
                continue;
            }
            
            let starting_index = particle_lookup[first_grid_index];
            var ending_index = -1;

            let next_grid_index = first_grid_index + 1;
            if next_grid_index >= i32(GRID_SIZE.x * GRID_SIZE.y) {
                ending_index = i32(PARTICLE_AMOUNT_X * PARTICLE_AMOUNT_Y);
            }
            else {
                ending_index = particle_lookup[next_grid_index];
            }

            for (var i = starting_index; i < ending_index; i=i+1){
                let offset = position - (particle_positions[i] + particle_velocities[i] * LOOK_AHEAD_TIME);
                let distance = length(offset);
                if distance == 0.0 {
                    continue;
                }
                let dir = normalize(offset);

                let slope = smoothing_kernel_derivative(distance);
                let other_density = get_density(particle_positions[i]);
                let shared_pressure = calculate_shared_pressure(density, other_density);

                // Pressure force
                pressure_force = pressure_force + dir * shared_pressure * slope * 3.141592653589 * particle_radii[i] * particle_radii[i] / max(density, 0.000001);

                // Viscosity force
                let viscosity_influence = viscosity_kernel(distance);
                viscosity_force = viscosity_force + (particle_velocities[i] - particle_velocities[index]) * viscosity_influence;
            }

        }
    }

    return vec4<f32>(pressure_force, viscosity_force * VISCOSITY);
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