struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
};

struct Cell {
    density: f32,
    divergence: f32,
    pressure: f32,
    s: i32,
}

const WORKGROUP_SIZE: u32 = 16;

const SCREEN_SIZE: vec2<f32> = vec2<f32>(1200.0, 600.0); // Size of the screen
const SIM_SIZE: vec2<f32> = vec2<f32>(500.0, 250.0);
const GRID_SPACING: vec2<f32> = vec2<f32>(SCREEN_SIZE.x / SIM_SIZE.x, SCREEN_SIZE.y / SIM_SIZE.y);

const GRAVITY: f32 = 0.1;
const OVER_RELAXATION: f32 = 1.9;
const dt: f32 = 1.0 / 8.0; // Time step

// NOTE: For the staggered grid, consider cell (i, j). The horizontal velocity at (i, j) is the right edge of the cell.
@group(0) @binding(0) var<storage, read_write> cells: array<array<Cell, u32(SIM_SIZE.x)>, u32(SIM_SIZE.y)>;
@group(0) @binding(1) var<storage, read_write> horizontal_velocities: array<array<f32, u32(SIM_SIZE.x - 1)>, u32(SIM_SIZE.y)>;
@group(0) @binding(2) var<storage, read_write> vertical_velocities: array<array<f32, u32(SIM_SIZE.x)>, u32(SIM_SIZE.y - 1)>;
@group(0) @binding(3) var<storage, read_write> advected_horizontal_velocities: array<array<f32, u32(SIM_SIZE.x - 1)>, u32(SIM_SIZE.y)>;
@group(0) @binding(4) var<storage, read_write> advected_vertical_velocities: array<array<f32, u32(SIM_SIZE.x)>, u32(SIM_SIZE.y - 1)>;

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

fn pos_to_grid(pos: vec2<f32>) -> vec2<i32> {
    return vec2<i32>(i32(pos.x * f32(SIM_SIZE.x) / SCREEN_SIZE.x), i32(pos.y * f32(SIM_SIZE.y) / SCREEN_SIZE.y));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Get the cell at the current position
    let pos = in.pos.xy;
    let gridPos = pos_to_grid(pos);

    let cell = cells[gridPos.y][gridPos.x];
    let pressure = cell.pressure;
    
    return vec4<f32>(0.0, 0.0, pressure * 999999, 1.0);

    // return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}

@compute @workgroup_size(WORKGROUP_SIZE, WORKGROUP_SIZE, 1)
fn main_gravity(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.xy;

    vertical_velocities[index.y][index.x] -= GRAVITY * dt;
}

@compute @workgroup_size(WORKGROUP_SIZE, WORKGROUP_SIZE, 1)
fn main_divergence(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.xy;
    
    var divergence = 0.0;
    // Left neighbor
    if (index.x > 0) {
        divergence -= horizontal_velocities[index.y][index.x  - 1];
    }
    // Right neighbor
    if (index.x < u32(SIM_SIZE.x) - 1) {
        divergence += horizontal_velocities[index.y][index.x];
    }
    // Bottom neighbor
    if (index.y > 0) {
        divergence -= vertical_velocities[index.y - 1][index.x];
    }
    // Top neighbor
    if (index.y < u32(SIM_SIZE.y) - 1) {
        divergence += vertical_velocities[index.y][index.x];
    }
    cells[index.y][index.x].divergence = divergence * OVER_RELAXATION;
}

@compute @workgroup_size(WORKGROUP_SIZE, WORKGROUP_SIZE, 1)
fn main_velocity(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index: vec2<u32> = global_id.xy;

    let me = cells[index.y][index.x];
    let divergence = me.divergence;

    // Calculate s
    var s = 0;
    if (index.x > 0) { // Left neighbor
        s += cells[index.y][index.x - 1].s;
    }
    if (index.x < u32(SIM_SIZE.x) - 1) { // Right neighbor
        s += cells[index.y][index.x + 1].s;
    }
    if (index.y > 0) { // Bottom neighbor
        s += cells[index.y - 1][index.x].s;
    }
    if (index.y < u32(SIM_SIZE.y) - 1) { // Top neighbor
        s += cells[index.y + 1][index.x].s;
    }

    // Update the neighbor velocities
    if (index.x > 0) { // Left neighbor
        horizontal_velocities[index.y][index.x - 1] += divergence * f32(cells[index.y][index.x - 1].s) / f32(s);
    }
    if (index.x < u32(SIM_SIZE.x) - 1) { // Right neighbor
        horizontal_velocities[index.y][index.x] -= divergence * f32(cells[index.y][index.x + 1].s) / f32(s);
    }
    if (index.y > 0) { // Bottom neighbor
        vertical_velocities[index.y - 1][index.x] += divergence * f32(cells[index.y - 1][index.x].s) / f32(s);
    }
    if (index.y < u32(SIM_SIZE.y) - 1) { // Top neighbor
        vertical_velocities[index.y][index.x] -= divergence * f32(cells[index.y + 1][index.x].s) / f32(s);
    }

    // Update pressure
    cells[index.y][index.x].pressure += divergence *  me.density * (f32(SCREEN_SIZE.x) / f32(SIM_SIZE.x) * f32(SCREEN_SIZE.y) / f32(SIM_SIZE.y)) / f32(s) * dt;
}

@compute @workgroup_size(WORKGROUP_SIZE, WORKGROUP_SIZE, 1)
fn main_advection(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.xy;

    // Horizontal velocities
    {
        let u = horizontal_velocities[index.y][index.x];
        var v_avg = 0.0;
        var v_div = 0;
        if(index.y > 0) {
            v_avg += vertical_velocities[index.y - 1][index.x];
            v_div += 1;
        }
        if(index.y < u32(SIM_SIZE.y) - 1) {
            v_avg += vertical_velocities[index.y][index.x];
            v_div += 1;
        }
        if (index.y > 0) {
            v_avg += vertical_velocities[index.y - 1][index.x];
            v_div += 1;
        }
        if (index.y < u32(SIM_SIZE.y) - 1) {
            v_avg += vertical_velocities[index.y][index.x];
            v_div += 1;
        }
        v_avg /= f32(v_div);

        let vel = vec2<f32>(u, v_avg);
        let pos = vec2<f32>(f32(index.x) + 0.5 * GRID_SPACING.x, f32(index.y) + 0.5 * GRID_SPACING.y);

        let prev_pos: vec2<f32> = pos - dt * vel;
        var prev_index: vec2<i32> = pos_to_grid(prev_pos);

        // If the position is in the top half of the cell, add 1 to the y index to uuse the correct 4 horizontal velocities
        if (prev_pos.y > f32(prev_index.y) + 0.5 * GRID_SPACING.y) {
            prev_index.y += 1;
        }

        // Calculate the old horizontal velocity using weighted average
        let w00: f32 = 1.0 - prev_pos.x / GRID_SPACING.x;
        let w01: f32 = prev_pos.x / GRID_SPACING.x;
        let w10: f32 = 1.0 - prev_pos.y / GRID_SPACING.y;
        let w11: f32 = prev_pos.y / GRID_SPACING.y;
        var prev_horizontal_vel: f32 = 0.0;
        if (prev_index.x > 0 && prev_index.y > 0) { // Bottom left
            prev_horizontal_vel += w00 * w10 * horizontal_velocities[prev_index.y - 1][prev_index.x - 1];
        }
        if (prev_index.x < i32(SIM_SIZE.x) - 1 && prev_index.y > 0) { // Bottom right
            prev_horizontal_vel += w01 * w10 * horizontal_velocities[prev_index.y - 1][prev_index.x];
        }
        if (prev_index.x > 0 && prev_index.y < i32(SIM_SIZE.y) - 1) { // Top left
            prev_horizontal_vel += w01 * w11 * horizontal_velocities[prev_index.y][prev_index.x - 1];
        }
        if (prev_index.x < i32(SIM_SIZE.x) - 1 && prev_index.y < i32(SIM_SIZE.y) - 1) { // Top right
            prev_horizontal_vel += w00 * w11 * horizontal_velocities[prev_index.y][prev_index.x];
        }

        // Update the horizontal velocity
        advected_horizontal_velocities[index.y][index.x] = prev_horizontal_vel;
    }

    
}