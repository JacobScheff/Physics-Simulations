struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
};

struct Grid {
    x: i32,
    y: i32,
}

const WORKGROUP_SIZE: u32 = 10;

const SCREEN_SIZE: vec2<f32> = vec2<f32>(1200.0, 600.0); // Size of the screen
const FOV: f32 = 60.0 * 3.14159 / 180.0; // Field of view in radians
const ASPECT_RATIO: f32 = SCREEN_SIZE.x / SCREEN_SIZE.y; // Aspect ratio of the screen
const PARTICLE_COUNT_X: u32 = 50;
const PARTICLE_COUNT_Y: u32 = 50;
const GRID_SIZE: vec2<f32> = vec2<f32>(20.0, 10.0);

@group(0) @binding(0) var<storage, read> frame_count: u32;
@group(0) @binding(1) var<storage, read_write> particle_positions: array<vec2<f32>, u32(PARTICLE_COUNT_X * PARTICLE_COUNT_Y)>;
@group(0) @binding(2) var<storage, read> particle_radii: array<f32>;
@group(0) @binding(3) var<storage, read_write> particle_velocities: array<vec2<f32>, u32(PARTICLE_COUNT_X * PARTICLE_COUNT_Y)>;
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
    let index = global_id.y * PARTICLE_COUNT_X + global_id.x;
    if index < 0 || index >= u32(PARTICLE_COUNT_X * PARTICLE_COUNT_Y) {
        return;
    }

    let pos: vec2<f32> = particle_positions[index];
    let vel = particle_velocities[index];
    let mass = 3.14159265359 * particle_radii[index] * particle_radii[index];
    let radius = particle_radii[index];

    let grid = pos_to_grid(pos);
    for (var gx: i32 = -1; gx <= 1; gx=gx+1){
        for(var gy: i32 = -1; gy <=1; gy=gy+1){
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
                if particle_lookup[i] != -1 && particle_lookup[i] > starting_index {
                    ending_index = particle_lookup[i];
                    break;
                }
            }
            if ending_index == -1 {
                ending_index = i32(PARTICLE_COUNT_X * PARTICLE_COUNT_Y);
            }

            for (var i = starting_index; i < ending_index; i=i+1){
                let other_pos = particle_positions[i];
                let d = (pos.x - particle_positions[i].x) * (pos.x - particle_positions[i].x) + (pos.y - particle_positions[i].y) * (pos.y - particle_positions[i].y);
                if d <= (radius + particle_radii[i]) * (radius + particle_radii[i]) && u32(i) != u32(index) {
                    let other_vel = particle_velocities[i];
                    let other_mass = 3.14159265359 * particle_radii[i] * particle_radii[i];
                    let other_radius = particle_radii[i];

                    // particle_velocities[index] -= 2.0 * other_mass / (mass + other_mass) * dot(vel - other_vel, pos - other_pos) / length(pos - other_pos) / length(pos - other_pos) * (pos - other_pos);
                    // particle_velocities[i] -= 2.0 * mass / (mass + other_mass) * dot(other_vel - vel, other_pos - pos) / length(other_pos - pos) / length(other_pos - pos) * (other_pos - pos);

                    particle_velocities[index] -= 2.0 * other_mass / (mass + other_mass) * dot(vel - other_vel, normalize(pos - other_pos) * (radius + other_radius)) / (radius + other_radius) / (radius + other_radius) * normalize(pos - other_pos) * (radius + other_radius);
                    particle_velocities[i] -= 2.0 * mass / (mass + other_mass) * dot(other_vel - vel, normalize(other_pos - pos) * (radius + other_radius)) / (radius + other_radius) / (radius + other_radius) * normalize(other_pos - pos) * (radius + other_radius);

                    // If the particles are overlapping, move them apart
                    if d < (radius + particle_radii[i]) * (radius + particle_radii[i]) {
                        var overlap = (radius + particle_radii[i]) - sqrt(d);
                        overlap /= 2.0; // If both move 0.5x, then they move a total amount of 1x
                        particle_positions[index] += overlap * normalize(pos - other_pos);
                        particle_positions[i] -= overlap * normalize(pos - other_pos);
                    }
                    storageBarrier();
                }
            }

        }
    }
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
            if starting_index == -1 {
                continue;
            }

            var ending_index = -1;

            // let next_grid_index = first_grid_index + 1;
            for (var i = first_grid_index + 1; i < i32(GRID_SIZE.x * GRID_SIZE.y); i=i+1){
                if particle_lookup[i] != -1 && particle_lookup[i] > starting_index {
                    ending_index = particle_lookup[i];
                    break;
                }
            }
            if ending_index == -1 {
                ending_index = i32(PARTICLE_COUNT_X * PARTICLE_COUNT_Y);
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