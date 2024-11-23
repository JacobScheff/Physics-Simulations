struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
};

struct Particle {
    velocity: vec2<f32>, // 8 bytes
    density: f32, // 4 bytes
}

const WORKGROUP_SIZE: u32 = 16;

const SCREEN_SIZE: vec2<f32> = vec2<f32>(1200.0, 600.0); // Size of the screen
const SIM_SIZE: vec2<f32> = vec2<f32>(500.0, 250.0);

@group(0) @binding(0) var<storage, read_write> particles_read: array<array<Particle, u32(SIM_SIZE.x)>, u32(SIM_SIZE.y)>;
@group(0) @binding(1) var<storage, read_write> particles_write: array<array<Particle, u32(SIM_SIZE.x)>, u32(SIM_SIZE.y)>;

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
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}

@compute @workgroup_size(WORKGROUP_SIZE, WORKGROUP_SIZE, 1)
fn main_density(@builtin(global_invocation_id) global_id: vec3<u32>) {

}

@compute @workgroup_size(WORKGROUP_SIZE, WORKGROUP_SIZE, 1)
fn main_forces(@builtin(global_invocation_id) global_id: vec3<u32>) {
    
}

@compute @workgroup_size(WORKGROUP_SIZE, WORKGROUP_SIZE, 1)
fn main_move(@builtin(global_invocation_id) global_id: vec3<u32>) {
    
}