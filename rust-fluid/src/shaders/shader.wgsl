struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
};

struct Grid {
    x: i32,
    y: i32,
}

struct Particle {
    position: vec2<f32>, // 8 bytes
    velocity: vec2<f32>, // 8 bytes
    radius: f32, // 4 bytes
    density: f32, // 4 bytes
    forces: vec4<f32>, // 16 bytes
}

const WORKGROUP_SIZE: u32 = 16;
const IPS_WORKGROUP_SIZE: u32 = 16;

const SCREEN_SIZE: vec2<f32> = vec2<f32>(1200.0, 600.0); // Size of the screen
const GRID_SIZE: vec2<f32> = vec2<f32>(8.0, 4.0);

const PARTICLE_AMOUNT_X: u32 = 48; // The number of particles in the x direction
const PARTICLE_AMOUNT_Y: u32 = 24; // The number of particles in the y direction
const TOTAL_PARTICLES: i32 = i32(PARTICLE_AMOUNT_X * PARTICLE_AMOUNT_Y); // The total number of particles
const RADIUS_OF_INFLUENCE: f32 = 75.0 / 4.0; // The radius of the sphere of influence. Also the radius to search for particles to calculate the density
const TARGET_DENSITY: f32 = 0.2; // The target density of the fluid
const PRESSURE_MULTIPLIER: f32 = 500.0; // The multiplier for the pressure force
const GRAVITY: f32 = 0.2; // The strength of gravity
const LOOK_AHEAD_TIME: f32 = 1.0 / 60.0; // The time to look ahead when calculating the predicted position
const VISCOSITY: f32 = 0.1; // The viscosity of the fluid
const DAMPENING: f32 = 0.95; // How much to slow down particles when they collide with the walls
const dt: f32 = 1.0 / 8.0; // The time step

const NUM_DIGITS: u32 = 4; // The number of digits in the grid index
const BASE: i32 = 10; // Base for the histogram
const BUCKET_SIZE: u32 = 32; // The amount of numbers in each bucket for the inclusive prefix sum
const NUM_BUCKETS: u32 = (u32(TOTAL_PARTICLES) + BUCKET_SIZE - 1) / BUCKET_SIZE;

const grids_to_check = vec2<i32>(i32(RADIUS_OF_INFLUENCE / SCREEN_SIZE.x * GRID_SIZE.x + 1.0), i32(RADIUS_OF_INFLUENCE / SCREEN_SIZE.y * GRID_SIZE.y + 1.0));
@group(0) @binding(0) var<storage, read_write> particles: array<Particle, u32(TOTAL_PARTICLES)>;
@group(0) @binding(1) var<storage, read_write> particle_lookup: array<i32, u32(GRID_SIZE.x * GRID_SIZE.y)>;
@group(0) @binding(2) var<storage, read_write> particle_counts: array<i32, u32(GRID_SIZE.x * GRID_SIZE.y)>;
@group(0) @binding(3) var<storage, read> mouse_info: array<f32, 4>; // 0-Up; 1-Down, x-pos, y-pos, 0-Repel; 1-Attract
@group(0) @binding(4) var<storage, read_write> histogram: array<array<atomic<u32>, u32(NUM_BUCKETS)>, u32(BASE)>;
@group(0) @binding(5) var<storage, read_write> inclusive_prefix_sum: array<array<atomic<u32>, u32(NUM_BUCKETS)>, u32(BASE)>;
@group(0) @binding(6) var<storage, read> current_digit_index: u32;
@group(0) @binding(7) var<storage, read_write> sorted_data: array<Particle, u32(TOTAL_PARTICLES)>;
@group(0) @binding(8) var<storage, read> scan_stage: u32;
@group(0) @binding(9) var<storage, read_write> scanned_inclusive_prefix_sum: array<array<u32, u32(NUM_BUCKETS)>, u32(BASE)>;
@group(0) @binding(10) var<storage, read_write> digit_histogram: array<atomic<u32>, u32(BASE)>;

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
    let density = get_density(particles[index].position);
    particles[index].density = density;
}

@compute @workgroup_size(WORKGROUP_SIZE, WORKGROUP_SIZE, 1)
fn main_forces(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.y * PARTICLE_AMOUNT_X + global_id.x;
    if index < 0 || index >= u32(TOTAL_PARTICLES) {
        return;
    }
    
    // Calculate the forces on the particle
    particles[index].forces = calculate_forces(index);
}

@compute @workgroup_size(WORKGROUP_SIZE, WORKGROUP_SIZE, 1)
fn main_move(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.y * PARTICLE_AMOUNT_X + global_id.x;
    if index < 0 || index >= u32(TOTAL_PARTICLES) {
        return;
    }

    // Move the particle
    let force = particles[index].forces;
    let radius = particles[index].radius;
    let density = particles[index].density;

    var acceleration = vec2<f32>(force.xy / max(density, 0.0001));
    acceleration += force.zw;
    acceleration.y += GRAVITY;
    // if density == 0.0 {
    //     acceleration = vec2<f32>(0.0, GRAVITY);
    // }

    particles[index].velocity += acceleration;
    particles[index].position += particles[index].velocity * dt;

    // Collide with the walls
    if particles[index].position.x - radius < 0.0 {
        particles[index].position.x = radius;
        particles[index].velocity.x = -particles[index].velocity.x * DAMPENING;
    }
    if particles[index].position.x + radius > SCREEN_SIZE.x {
        particles[index].position.x = SCREEN_SIZE.x - radius;
        particles[index].velocity.x = -particles[index].velocity.x * DAMPENING;
    }
    if particles[index].position.y - radius < 0.0 {
        particles[index].position.y = radius;
        particles[index].velocity.y = -particles[index].velocity.y * DAMPENING;
    }
    if particles[index].position.y + radius > SCREEN_SIZE.y {
        particles[index].position.y = SCREEN_SIZE.y - radius;
        particles[index].velocity.y = -particles[index].velocity.y * DAMPENING;
    }
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let x: f32 = in.pos.x;
    let y: f32 = in.pos.y;

    var final_color: vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 1.0);

    let grid = pos_to_grid(vec2<f32>(x, y));
    for (var g: i32 = -2; g <= 2; g=g+1){
            var gx: i32 = g / 2;
            var gy: i32 = g % 2;
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
            
            var ending_index = starting_index + particle_counts[first_grid_index];

            for (var i = starting_index; i <= ending_index; i=i+1){
                let d = (x - particles[i].position.x) * (x - particles[i].position.x) + (y - particles[i].position.y) * (y - particles[i].position.y);
                if d < particles[i].radius * particles[i].radius {
                let speed = length(particles[i].velocity);
                let density = particles[i].density;

                // Create a gradient color
                let min_speed: f32 = 0.0;
                let max_speed: f32 = 12.0;
                var speed_t: f32 = (speed - min_speed) / (max_speed - min_speed);
                speed_t = min(max(speed_t, 0.0), 1.0);
                let min_density: f32 = 0.0;
                let max_density: f32 = 0.4;
                var density_t: f32 = (density - min_density) / (max_density - min_density);
                density_t = min(max(density_t, 0.0), 1.0);
                let color: vec3<f32> = vec3<f32>(speed_t, density_t, 1.0 - speed_t);
                    
                final_color = vec4<f32>(color, 1.0);
                break;
            }
        }
    }

    return final_color;
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

fn get_density(pos: vec2<f32>) -> f32 {
    let grid = pos_to_grid(pos);
    var density = 0.0;

    for (var g: i32 = 0; g < (grids_to_check.x * 2 + 1) * (grids_to_check.y * 2 + 1); g=g+1){
        let gx: i32 = g / (grids_to_check.y * 2 + 1) - grids_to_check.x;
        let gy: i32 = g % (grids_to_check.y * 2 + 1) - grids_to_check.y;

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
            
        var ending_index = starting_index + particle_counts[first_grid_index];

        for (var i: u32 = u32(starting_index); i <= u32(ending_index); i=i+1){
            let distance = length(pos - (particles[i].position + particles[i].velocity * LOOK_AHEAD_TIME));
            if distance <= RADIUS_OF_INFLUENCE {
                let influence = smoothing_kernel(distance);
                density += influence * 3.141592653589 * particles[i].radius * particles[i].radius;
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

fn calculate_forces(index: u32) -> vec4<f32> {
    var forces = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    // NOTE: index is already the index of the particle in the grid_index_map
    let position: vec2<f32> = particles[index].position + particles[index].velocity * LOOK_AHEAD_TIME;

    let grid = pos_to_grid(position);

    let density: f32 = particles[index].density;

    for (var g: i32 = 0; g < (grids_to_check.x * 2 + 1) * (grids_to_check.y * 2 + 1); g=g+1){
        let gx: i32 = g / (grids_to_check.y * 2 + 1) - grids_to_check.x;
        let gy: i32 = g % (grids_to_check.y * 2 + 1) - grids_to_check.y;
            
        if grid.x + gx < 0 || grid.x + gx >= i32(GRID_SIZE.x) || grid.y + gy < 0 || grid.y + gy >= i32(GRID_SIZE.y) {
            continue;
        }
            
        let first_grid_index: i32 = grid_to_index(grid_add(grid, Grid(gx, gy)));
        if (first_grid_index < 0 || first_grid_index >= i32(GRID_SIZE.x * GRID_SIZE.y)) {
            continue;
        }

        let starting_index = particle_lookup[first_grid_index];
        if starting_index == -1 {
            continue;
        }

        var ending_index: i32 = starting_index + particle_counts[first_grid_index];

        for(var i: i32 = starting_index; i <= ending_index; i=i+1){
            if i == -1 || i == i32(index) || i >= i32(TOTAL_PARTICLES) {
                continue;
            }
            let offset: vec2<f32> = position - (particles[i].position + particles[i].velocity * LOOK_AHEAD_TIME);
            let distance: f32 = sqrt(offset.x * offset.x + offset.y * offset.y);
            if distance == 0.0 || distance > RADIUS_OF_INFLUENCE {
                continue;
            }
            let dir = vec2<f32>(offset.x / distance, offset.y / distance);

            let slope = smoothing_kernel_derivative(distance);
            let other_density = particles[i].density;
            let shared_pressure = calculate_shared_pressure(density, other_density);

            // Pressure force
            let pressure_force = dir * shared_pressure * slope * 3.141592653589 * particles[i].radius * particles[i].radius / max(density, 0.000001);
            // if density == 0.0 {
            //     continue;
            // }
                    
            // Viscosity force
            let viscosity_influence = viscosity_kernel(distance);
            var viscosity_force = (particles[i].velocity - particles[index].velocity) * viscosity_influence;
            viscosity_force *= VISCOSITY;

            // Apply the forces
            forces += vec4<f32>(pressure_force.x, pressure_force.y, viscosity_force.x, viscosity_force.y);
        }
    }

    // Check for mouse interaction
    if mouse_info[0] == 1.0 || mouse_info[3] == 1.0 {
        let mouse_pos = vec2<f32>(mouse_info[1], mouse_info[2]);
        let offset = position - mouse_pos;
        let distance = sqrt(offset.x * offset.x + offset.y * offset.y);
        if distance < RADIUS_OF_INFLUENCE {
            let dir = vec2<f32>(offset.x / distance, offset.y / distance);
            var mouse_force = dir * smoothing_kernel(distance) * 100000.0;
            if mouse_info[3] == 1.0 {
                mouse_force *= -0.005; // Attract
            }
            else {
                mouse_force *= 0.5; // Repel
            }
            forces += vec4<f32>(mouse_force.x, mouse_force.y, 0.0, 0.0);
        }
    }

    return forces;

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

// --- Sort --- //
fn val_to_digit(val: i32, digit_index: u32) -> i32 {
    let valf32 = f32(val);
    let divisor = pow(f32(BASE), f32(digit_index));
    let digit = i32(floor(valf32 / divisor)) % BASE;

    return digit;
}

@compute @workgroup_size(WORKGROUP_SIZE, 1)
fn update_histogram(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index: u32 = global_id.x;
    if (i32(index) >= TOTAL_PARTICLES) {
        return;
    }
    
    let bucket_index: u32 = index / BUCKET_SIZE;
    let grid_index = grid_to_index(pos_to_grid(particles[index].position));
    let digit = val_to_digit(grid_index, current_digit_index);

    // Update the inclusive prefix sum
    atomicAdd(&histogram[digit][bucket_index], 1u);

    // Update the digit histogram
    atomicAdd(&digit_histogram[digit], 1u);

    // Update particle counts if is the last digit being sorted
    if (current_digit_index == NUM_DIGITS - 1) {
        particle_counts[grid_index] += 1;
    } else {
        particle_counts[grid_index] = 0;
    }
}

@compute @workgroup_size(IPS_WORKGROUP_SIZE, 1)
// https://www.youtube.com/watch?v=RdfmxfZBHpo, Hillis Steele Scan
fn update_inclusive_prefix_sum(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index: u32 = global_id.x;
    let digit: u32 = global_id.y;
    if (index >= NUM_BUCKETS || digit >= u32(BASE)) {
        return;
    }
    
    let lookup_distance: u32 = u32(pow(2.0, f32(scan_stage)));

    if (index >= lookup_distance) {
        scanned_inclusive_prefix_sum[digit][index] = inclusive_prefix_sum[digit][index] + inclusive_prefix_sum[digit][index - lookup_distance];
    } else {
        scanned_inclusive_prefix_sum[digit][index] = inclusive_prefix_sum[digit][index];
    }
}

@compute @workgroup_size(WORKGROUP_SIZE, 1)
fn update_indices(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index: u32 = global_id.x;
    if (i32(index) >= TOTAL_PARTICLES) {
        return;
    }

    let grid_index = grid_to_index(pos_to_grid(particles[index].position));
    let digit: i32 = val_to_digit(grid_index, current_digit_index);

    // Calculate the number of elements before it
    var global_offset: i32 = 0;
    for (var i: i32 = 0; i < digit; i = i + 1) {
        global_offset += i32(digit_histogram[i]);
    }

    // Calculate the local offset
    let bucket_index: u32 = index / BUCKET_SIZE;
    let bucket_start: u32 = bucket_index * BUCKET_SIZE;
    let bucket_end: u32 = min(bucket_start + BUCKET_SIZE, u32(TOTAL_PARTICLES));

    var local_offset: u32 = inclusive_prefix_sum[digit][bucket_index] - 1u;
    for (var i: u32 = bucket_end - 1; i > index; i = i - 1u) {
        let other_grid_index = grid_to_index(pos_to_grid(particles[i].position));
        if (val_to_digit(other_grid_index, current_digit_index) == digit) {
            local_offset -= 1u;
        }
    }

    let new_index: i32 = i32(local_offset) + global_offset;
    sorted_data[new_index] = particles[index];
}

@compute @workgroup_size(WORKGROUP_SIZE, 1)
fn update_lookup(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index: u32 = global_id.x;
    if (i32(index) >= TOTAL_PARTICLES) {
        return;
    }

    let grid_index = grid_to_index(pos_to_grid(particles[index].position));

    if (index == 0) {
        particle_lookup[grid_index] = i32(index);
    } else {
        let prev_grid_index = grid_to_index(pos_to_grid(particles[index - 1].position));
        if (grid_index != prev_grid_index) {
            particle_lookup[grid_index] = i32(index);
        }
    }   
}