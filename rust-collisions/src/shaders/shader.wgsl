struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
};

const SCREEN_SIZE: vec2<f32> = vec2<f32>(1200.0, 600.0); // Size of the screen
const FOV: f32 = 60.0 * 3.14159 / 180.0; // Field of view in radians
const ASPECT_RATIO: f32 = SCREEN_SIZE.x / SCREEN_SIZE.y; // Aspect ratio of the screen
const PARTICLE_COUNT_X: u32 = 10;
const PARTICLE_COUNT_Y: u32 = 10;

@group(0) @binding(0) var<storage, read> frame_count: u32;
@group(0) @binding(1) var<storage, read_write> particle_positions: array<vec2<f32>, u32(PARTICLE_COUNT_X * PARTICLE_COUNT_Y)>;
@group(0) @binding(2) var<storage, read> particle_radii: array<f32>;

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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let x: f32 = in.pos.x;
    let y: f32 = in.pos.y;
    
    for (var i = 0; i < i32(PARTICLE_COUNT_X * PARTICLE_COUNT_Y); i=i+1){
        let d = (x - particle_positions[i].x) * (x - particle_positions[i].x) + (y - particle_positions[i].y) * (y - particle_positions[i].y);
        if d < particle_radii[i] * particle_radii[i] {
            return vec4<f32>(0.0, 1.0, 0.0, 1.0);
        }
    }

    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}