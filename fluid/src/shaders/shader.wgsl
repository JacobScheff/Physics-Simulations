struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
};

struct Cell {
    density: f32,
    divergence: f32,
    pressure: f32,
    s: i32,
    new_s: i32,
    wall: i32,
}

const WORKGROUP_SIZE: u32 = 16;

const SCREEN_SIZE: vec2<f32> = vec2<f32>(1200.0, 600.0); // Size of the screen
const SIM_SIZE: vec2<f32> = vec2<f32>(500.0, 250.0);

const GRAVITY: f32 = 0.1;
const OVER_RELAXATION: f32 = 1.9;
const dt: f32 = 1.0 / 8.0; // Time step

// NOTE: For the staggered grid, consider cell (i, j). The horizontal velocity at (i, j) is the right edge of the cell.
@group(0) @binding(0) var<storage, read_write> cells: array<array<Cell, u32(SIM_SIZE.x)>, u32(SIM_SIZE.y)>;
@group(0) @binding(1) var<storage, read_write> horizontal_velocities: array<array<f32, u32(SIM_SIZE.x - 1)>, u32(SIM_SIZE.y)>;
@group(0) @binding(2) var<storage, read_write> vertical_velocities: array<array<f32, u32(SIM_SIZE.x)>, u32(SIM_SIZE.y - 1)>;

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
    var new_s = 0;
    if (index.x > 0) { // Left neighbor
        new_s += cells[index.y][index.x - 1].s;
    }
    if (index.x < u32(SIM_SIZE.x) - 1) { // Right neighbor
        new_s += cells[index.y][index.x + 1].s;
    }
    if (index.y > 0) { // Bottom neighbor
        new_s += cells[index.y - 1][index.x].s;
    }
    if (index.y < u32(SIM_SIZE.y) - 1) { // Top neighbor
        new_s += cells[index.y + 1][index.x].s;
    }

    if (me.wall == 1) {
        new_s = 0;
    }

    cells[index.y][index.x].new_s = new_s;

    // Update the neighbor velocities
    if (index.x > 0) { // Left neighbor
        horizontal_velocities[index.y][index.x - 1] += divergence * f32(cells[index.y][index.x - 1].s) / f32(new_s);
    }
    if (index.x < u32(SIM_SIZE.x) - 1) { // Right neighbor
        horizontal_velocities[index.y][index.x] -= divergence * f32(cells[index.y][index.x + 1].s) / f32(new_s);
    }
    if (index.y > 0) { // Bottom neighbor
        vertical_velocities[index.y - 1][index.x] += divergence * f32(cells[index.y - 1][index.x].s) / f32(new_s);
    }
    if (index.y < u32(SIM_SIZE.y) - 1) { // Top neighbor
        vertical_velocities[index.y][index.x] -= divergence * f32(cells[index.y + 1][index.x].s) / f32(new_s);
    }

    // Update pressure
    cells[index.y][index.x].pressure += divergence *  me.density * (f32(SCREEN_SIZE.x) / f32(SIM_SIZE.x) * f32(SCREEN_SIZE.y) / f32(SIM_SIZE.y)) / f32(new_s) * dt;
}

@compute @workgroup_size(WORKGROUP_SIZE, WORKGROUP_SIZE, 1)
fn main_advection(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.xy;

    // Set s to new_s
    cells[index.y][index.x].s = cells[index.y][index.x].new_s;
}