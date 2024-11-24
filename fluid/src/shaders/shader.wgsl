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
const SIM_SIZE: vec2<f32> = vec2<f32>(1200.0, 600.0);

const GRAVITY: f32 = 0.1;
const OVER_RELAXATION: f32 = 1.0;
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
    
    return vec4<f32>(0.0, 0.0, pressure, 1.0);

    // return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}

@compute @workgroup_size(WORKGROUP_SIZE, WORKGROUP_SIZE, 1)
fn main_gravity(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.xy;

    // Make sure velocity index is within bounds
    if (index.x >= u32(SIM_SIZE.x - 1) || index.y >= u32(SIM_SIZE.y - 1)) {
        return;
    }

    vertical_velocities[index.y][index.x] -= GRAVITY * dt;
}

@compute @workgroup_size(WORKGROUP_SIZE, WORKGROUP_SIZE, 1)
fn main_divergence(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.xy;

    // Make sure cell index is within bounds
    if (index.x >= u32(SIM_SIZE.x) || index.y >= u32(SIM_SIZE.y)) {
        return;
    }
    
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

    // Make sure cell index is within bounds
    if (index.x >= u32(SIM_SIZE.x) || index.y >= u32(SIM_SIZE.y)) {
        return;
    }

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

    if (s == 0) {
        return; // Avoid division by zero
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
        // Make sure velocity index is within bounds
        if (index.x >= u32(SIM_SIZE.x - 1) || index.y >= u32(SIM_SIZE.y)) {
            return;
        }

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
        let pos = vec2<f32>(f32(index.x) + 0.5, f32(index.y));

        let prev_pos: vec2<f32> = pos - dt * vel;
        var prev_index: vec2<i32> = vec2<i32>(floor(prev_pos));
        
        // If the previous index is in the top half of the cell, add 1 to the y index
        if (prev_pos.y - f32(prev_index.y) > 0.5) {
            prev_index.y += 1;
            if (prev_index.y >= i32(SIM_SIZE.y)) {
                prev_index.y = i32(SIM_SIZE.y) - 1;
            }
        }

        let diff: vec2<f32> = prev_pos - vec2<f32>(prev_index);

        // Clamp indices to avoid out-of-bounds access
        let x0 = clamp(prev_index.x - 1, 0, i32(SIM_SIZE.x - 2));
        let x1 = clamp(prev_index.x, 0, i32(SIM_SIZE.x - 2));
        let y0 = clamp(prev_index.y - 1, 0, i32(SIM_SIZE.y - 1));
        let y1 = clamp(prev_index.y, 0, i32(SIM_SIZE.y - 1));
        
        let u00 = horizontal_velocities[y0][x0];
        let u01 = horizontal_velocities[y0][x1];
        let u10 = horizontal_velocities[y1][x0];
        let u11 = horizontal_velocities[y1][x1];
    

        advected_horizontal_velocities[index.y][index.x] = mix(mix(u00, u01, diff.x), mix(u10, u11, diff.x), diff.y);
    }

    // Vertical velocities
    {
        // Make sure velocity index is within bounds
        if (index.x >= u32(SIM_SIZE.x) || index.y >= u32(SIM_SIZE.y - 1)) {
            return;
        }

        let v = vertical_velocities[index.y][index.x];
        var u_avg = 0.0;
        var u_div = 0;
        if(index.x > 0) {
            u_avg += horizontal_velocities[index.y][index.x - 1];
            u_div += 1;
        }
        if(index.x < u32(SIM_SIZE.x) - 1) {
            u_avg += horizontal_velocities[index.y][index.x];
            u_div += 1;
        }
        if (index.x > 0) {
            u_avg += horizontal_velocities[index.y][index.x - 1];
            u_div += 1;
        }
        if (index.x < u32(SIM_SIZE.x) - 1) {
            u_avg += horizontal_velocities[index.y][index.x];
            u_div += 1;
        }
        u_avg /= f32(u_div);

        let vel = vec2<f32>(u_avg, v);
        let pos = vec2<f32>(f32(index.x), f32(index.y) + 0.5);

        let prev_pos: vec2<f32> = pos - dt * vel;
        var prev_index: vec2<i32> = vec2<i32>(floor(prev_pos));

        // If the previous index is in the right half of the cell, add 1 to the x index
        if (prev_pos.x - f32(prev_index.x) > 0.5) {
            prev_index.x += 1;
            if (prev_index.x >= i32(SIM_SIZE.x)) {
                prev_index.x = i32(SIM_SIZE.x) - 1;
            }
        }

        let diff: vec2<f32> = prev_pos - vec2<f32>(prev_index);

        // Clamp indices to avoid out-of-bounds access
        let x0 = clamp(prev_index.x - 1, 0, i32(SIM_SIZE.x - 1));
        let x1 = clamp(prev_index.x, 0, i32(SIM_SIZE.x - 1));
        let y0 = clamp(prev_index.y - 1, 0, i32(SIM_SIZE.y - 2));
        let y1 = clamp(prev_index.y, 0, i32(SIM_SIZE.y - 2));
        
        let v00 = vertical_velocities[y0][x0];
        let v01 = vertical_velocities[y0][x1];
        let v10 = vertical_velocities[y1][x0];
        let v11 = vertical_velocities[y1][x1];
    

        advected_vertical_velocities[index.y][index.x] = mix(mix(v00, v01, diff.x), mix(v10, v11, diff.x), diff.y);
    }
}