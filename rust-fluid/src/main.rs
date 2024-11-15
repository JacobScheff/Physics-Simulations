use core::f32;
use bytemuck::{Pod, Zeroable};
use bytemuck::NoUninit;
use renderer_backend::{
    bind_group_layout_generator, compute_pipeline_builder::ComputePipelineBuilder,
    pipeline_builder::PipelineBuilder,
};
mod renderer_backend;
// use rand::Rng;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferUsages,
};
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::EventLoopBuilder,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

const SCREEN_SIZE: (u32, u32) = (1200, 600);
const TIME_BETWEEN_FRAMES: u64 = 2;
const GRID_SIZE: (i32, i32) = (80, 40); // How many grid cells to divide the screen into

const PARTICLE_RADIUS: f32 = 2.5; // The radius of the particles
const PARTICLE_AMOUNT_X: u32 = 96; // The number of particles in the x direction
const PARTICLE_AMOUNT_Y: u32 = 48; // The number of particles in the y direction
const PADDING: f32 = 50.0; // The padding around the screen

const WORKGROUP_SIZE: u32 = 16;
const DISPATCH_SIZE: (u32, u32) = (
    (PARTICLE_AMOUNT_X + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
    (PARTICLE_AMOUNT_Y + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
);

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Particle {
    position: [f32; 2],
    velocity: [f32; 2],
    radius: f32,
    density: f32,
    forces: [f32; 4],
}

impl Particle {
    fn new(position: [f32; 2], velocity: [f32; 2], radius: f32) -> Self {
        Self {
            position,
            velocity,
            radius,
            density: 0.0,
            forces: [0.0, 0.0, 0.0, 0.0],
        }
    }
}
struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    window: &'a Window,
    render_pipeline: wgpu::RenderPipeline,
    compute_density_pipeline: wgpu::ComputePipeline,
    compute_forces_pipeline: wgpu::ComputePipeline,
    compute_move_pipeline: wgpu::ComputePipeline,
    render_bind_group: wgpu::BindGroup,
    compute_densities_bind_group: wgpu::BindGroup,
    compute_forces_bind_group: wgpu::BindGroup,
    compute_move_bind_group: wgpu::BindGroup,
    frame_count: u32,
    particle_positions: Vec<Particle>,
    particle_positions_buffer: wgpu::Buffer,
    positions_reader_buffer: wgpu::Buffer,
    // particle_radii: Vec<f32>,
    // particle_radii_buffer: wgpu::Buffer,
    // particle_radii_reader_buffer: wgpu::Buffer,
    // particle_velocities: Vec<[f32; 2]>,
    // particle_velocities_buffer: wgpu::Buffer,
    // particle_velocities_reader_buffer: wgpu::Buffer,
    // particle_densities: Vec<f32>,
    // particle_densities_buffer: wgpu::Buffer,
    // particle_densities_reader_buffer: wgpu::Buffer,
    // particle_forces: Vec<[f32; 4]>,
    // particle_forces_buffer: wgpu::Buffer,
    // particle_forces_reader_buffer: wgpu::Buffer,
    particle_lookup: Vec<i32>,
    particle_lookup_buffer: wgpu::Buffer,
    particle_counts: Vec<i32>,
    particle_counts_buffer: wgpu::Buffer,
    mouse_info: [f32; 4], // 0-up; 1-down, x-pos, y-pos, 0-Atttract; 1-Repel
    mouse_info_buffer: wgpu::Buffer,
}

#[allow(unused)]
fn pos_to_grid_index(pos: (f32, f32)) -> i32 {
    let x = ((pos.0 / SCREEN_SIZE.0 as f32 * GRID_SIZE.0 as f32) as i32)
        .min(GRID_SIZE.0 - 1)
        .max(0) as i32;
    let y = ((pos.1 / SCREEN_SIZE.1 as f32 * GRID_SIZE.1 as f32) as i32)
        .min(GRID_SIZE.1 - 1)
        .max(0) as i32;

    x + y * GRID_SIZE.0
}

impl<'a> State<'a> {
    const GRID_DIV_SCREEN_SIZE: (f32, f32) = (
        GRID_SIZE.0 as f32 / SCREEN_SIZE.0 as f32,
        GRID_SIZE.1 as f32 / SCREEN_SIZE.1 as f32,
    );
    fn pos_to_grid(&self, pos: [f32; 2]) -> (i32, i32) {
        let x = (pos[0] * Self::GRID_DIV_SCREEN_SIZE.0)
            .min(GRID_SIZE.0 as f32 - 1.0)
            .max(0.0) as i32;
        let y = (pos[1] * Self::GRID_DIV_SCREEN_SIZE.1)
            .min(GRID_SIZE.1 as f32 - 1.0)
            .max(0.0) as i32;

        (x, y)
    }

    async fn new(window: &'a Window) -> Self {
        let size = window.inner_size();

        let instance_descriptor = wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        };
        let instance = wgpu::Instance::new(instance_descriptor);
        let surface = instance.create_surface(window).unwrap();

        // Pick the second adapter (NVIDIA 3060 RTX)
        let adapter = instance
            .enumerate_adapters(wgpu::Backends::all())
            .into_iter()
            .nth(0)
            .unwrap();
        println!("{:?}", adapter.get_info());

        let device_descriptor = wgpu::DeviceDescriptor {
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            label: Some("Device"),
        };
        let (device, queue) = adapter
            .request_device(&device_descriptor, None)
            .await
            .unwrap();

        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_capabilities.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_capabilities.present_modes[0],
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Pass bind group layout to render pipeline builder
        let mut render_pipeline_builder = PipelineBuilder::new();
        render_pipeline_builder.set_shader_module("shaders/shader.wgsl", "vs_main", "fs_main");
        render_pipeline_builder.set_pixel_format(config.format);
        render_pipeline_builder.set_bind_group_layout(
            bind_group_layout_generator::get_bind_group_layout(&device),
        );
        let render_pipeline = render_pipeline_builder.build_pipeline(&device);

        // Pass bind group layout to compute density pipeline builder
        let mut compute_density_pipeline_builder = ComputePipelineBuilder::new();
        compute_density_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_density");
        compute_density_pipeline_builder.set_bind_group_layout(
            bind_group_layout_generator::get_bind_group_layout(&device),
        );
        let compute_density_pipeline = compute_density_pipeline_builder.build_pipeline(&device);

        // Pass bind group layout to compute move pipeline builder
        let mut compute_move_pipeline_builder = ComputePipelineBuilder::new();
        compute_move_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_move");
        compute_move_pipeline_builder.set_bind_group_layout(
            bind_group_layout_generator::get_bind_group_layout(&device),
        );
        let compute_move_pipeline = compute_move_pipeline_builder.build_pipeline(&device);

        // Pass bind group layout to compute forces pipeline builder
        let mut compute_forces_pipeline_builder = ComputePipelineBuilder::new();
        compute_forces_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_forces");
        compute_forces_pipeline_builder.set_bind_group_layout(
            bind_group_layout_generator::get_bind_group_layout(&device),
        );
        let compute_forces_pipeline = compute_forces_pipeline_builder.build_pipeline(&device);

        // Create temporary bind groups
        let temp_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Temporary Render Bind Group"),
            layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[],
                label: Some("Temporary Render Bind Group Layout"),
            }),
            entries: &[],
        });

        let temp_compute_density_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Temporary Compute Density Bind Group"),
                layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[],
                    label: Some("Temporary Compute Density Bind Group Layout"),
                }),
                entries: &[],
            });

        let temp_compute_move_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Temporary Compute Move Bind Group"),
            layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[],
                label: Some("Temporary Compute Move Bind Group Layout"),
            }),
            entries: &[],
        });

        let temp_compute_forces_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Temporary Compute Forces Bind Group"),
            layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[],
                label: Some("Temporary Compute Forces Bind Group Layout"),
            }),
            entries: &[],
        });

        // Create particle data
        let mut particle_positions = vec![];
        let mut particle_velocities = vec![];
        let mut particle_radii = vec![];
        let mut particle_densities = vec![];
        let mut particle_forces = vec![];
        for i in 0..PARTICLE_AMOUNT_X {
            for j in 0..PARTICLE_AMOUNT_Y {
                // let x = SCREEN_SIZE.0 as f32 / (PARTICLE_AMOUNT_X + 1) as f32 * i as f32 + OFFSET.0;
                // let y = SCREEN_SIZE.1 as f32 / (PARTICLE_AMOUNT_Y + 1) as f32 * j as f32 + OFFSET.1;

                let x = (i as f32 + 0.5) * (SCREEN_SIZE.0 as f32 - 2.0 * PADDING)
                    / PARTICLE_AMOUNT_X as f32
                    + PADDING;
                let y = (j as f32 + 0.5) * (SCREEN_SIZE.1 as f32 - 2.0 * PADDING)
                    / PARTICLE_AMOUNT_Y as f32
                    + PADDING;

                // particle_positions.push([x, y]);
                particle_velocities.push([0.0, 0.0]);
                // particle_velocities.push([
                //     rand::thread_rng().gen_range(-1.0..1.0),
                //     rand::thread_rng().gen_range(-1.0..1.0),
                // ]);
                particle_radii.push(PARTICLE_RADIUS);
                particle_densities.push(0.0);
                particle_forces.push([0.0, 0.0, 0.0, 0.0]);

                particle_positions.push(Particle::new([x, y], [0.0, 0.0], PARTICLE_RADIUS));
            }
        }
        let particle_lookup: Vec<i32> = vec![0; GRID_SIZE.0 as usize * GRID_SIZE.1 as usize];
        let particle_counts: Vec<i32> = vec![0; GRID_SIZE.0 as usize * GRID_SIZE.1 as usize];

        // Buffer for particles
        let particle_positions_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Particle Positions Buffer Data"),
            contents: bytemuck::cast_slice(&particle_positions),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });
        let particle_velocities_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Particle Velocities Buffer Data"),
            contents: bytemuck::cast_slice(&particle_velocities),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });
        let particle_radii_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Particle Radii Buffer Data"),
            contents: bytemuck::cast_slice(&particle_radii),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });
        let particle_densities_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Particle Densities Buffer Data"),
            contents: bytemuck::cast_slice(&particle_densities),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });
        let particle_forces_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Particle Forces Buffer Data"),
            contents: bytemuck::cast_slice(&particle_forces),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });
        let particle_lookup_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Particle Lookup Buffer Data"),
            contents: bytemuck::cast_slice(&particle_lookup),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        let particle_counts_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Particle Counts Buffer Data"),
            contents: bytemuck::cast_slice(&particle_counts),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        // Write data to buffers
        queue.write_buffer(
            &particle_positions_buffer,
            0,
            bytemuck::cast_slice(&particle_positions),
        );
        queue.write_buffer(
            &particle_radii_buffer,
            0,
            bytemuck::cast_slice(&particle_radii),
        );
        queue.write_buffer(
            &particle_velocities_buffer,
            0,
            bytemuck::cast_slice(&particle_velocities),
        );
        queue.write_buffer(
            &particle_densities_buffer,
            0,
            bytemuck::cast_slice(&particle_densities),
        );
        queue.write_buffer(
            &particle_forces_buffer,
            0,
            bytemuck::cast_slice(&particle_forces),
        );
        queue.write_buffer(
            &particle_lookup_buffer,
            0,
            bytemuck::cast_slice(&particle_lookup),
        );
        queue.write_buffer(
            &particle_counts_buffer,
            0,
            bytemuck::cast_slice(&particle_counts),
        );

        // Reader buffers
        let positions_reader_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Positions Reader Buffer"),
            size: (std::mem::size_of::<[f32; 2]>() * particle_positions.len()) as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let particle_radii_reader_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Radii Reader Buffer"),
            size: (std::mem::size_of::<f32>() * particle_radii.len()) as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let particle_velocities_reader_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Velocities Reader Buffer"),
            size: (std::mem::size_of::<[f32; 2]>() * particle_velocities.len()) as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let particle_densities_reader_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Densities Reader Buffer"),
            size: (std::mem::size_of::<f32>() * particle_densities.len()) as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let particle_forces_reader_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Forces Reader Buffer"),
            size: (std::mem::size_of::<[f32; 4]>() * particle_forces.len()) as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Mouse info
        let mouse_info = [0.0, 0.0, 0.0, 0.0];
        let mouse_info_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Mouse Info Buffer Data"),
            contents: bytemuck::cast_slice(&mouse_info),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            compute_density_pipeline,
            compute_forces_pipeline,
            compute_move_pipeline,
            render_bind_group: temp_render_bind_group,
            compute_densities_bind_group: temp_compute_density_bind_group,
            compute_forces_bind_group: temp_compute_forces_bind_group,
            compute_move_bind_group: temp_compute_move_bind_group,
            frame_count: 0,
            particle_positions,
            particle_positions_buffer,
            positions_reader_buffer,
            // particle_radii,
            // particle_radii_buffer,
            // particle_radii_reader_buffer,
            // particle_velocities,
            // particle_velocities_buffer,
            // particle_velocities_reader_buffer,
            // particle_densities,
            // particle_densities_buffer,
            // particle_densities_reader_buffer,
            // particle_forces,
            // particle_forces_buffer,
            // particle_forces_reader_buffer,
            particle_lookup,
            particle_lookup_buffer,
            particle_counts,
            particle_counts_buffer,
            mouse_info,
            mouse_info_buffer,
        }
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    async fn update_position_from_buffer(&mut self) {
        // Copy particle positions to position_reading_buffer
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Copy Encoder"),
            });
        encoder.copy_buffer_to_buffer(
            &self.particle_positions_buffer,
            0,
            &self.positions_reader_buffer,
            0,
            self.particle_positions_buffer.size(),
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        // Map position_reading_buffer for reading asynchronously
        let buffer_slice = self.positions_reader_buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        // Wait for the mapping to complete
        self.device.poll(wgpu::Maintain::Wait);

        // Check if the mapping was successful
        if let Ok(()) = receiver.receive().await.unwrap() {
            let data = buffer_slice.get_mapped_range();
            let positions: &[f32] = bytemuck::cast_slice(&data);
            // Update the particle positions
            for i in 0..self.particle_positions.len() {
                self.particle_positions[i] = Particle::new(
                    [positions[i * 2], positions[i * 2 + 1]],
                    self.particle_positions[i].velocity,
                    self.particle_positions[i].radius,
                );
            }

            drop(data);
            self.positions_reader_buffer.unmap();
        } else {
            // Handle mapping error
            eprintln!("Error mapping buffer");
            // return Err(wgpu::SurfaceError::Lost); // Or handle the error appropriately
            return;
        }
    }

    // async fn update_velocities_from_buffer(&mut self) {
    //     // Copy particle velocities to velocity_reading_buffer
    //     let mut encoder = self
    //         .device
    //         .create_command_encoder(&wgpu::CommandEncoderDescriptor {
    //             label: Some("Copy Encoder"),
    //         });
    //     encoder.copy_buffer_to_buffer(
    //         &self.particle_velocities_buffer,
    //         0,
    //         &self.particle_velocities_reader_buffer,
    //         0,
    //         self.particle_velocities_buffer.size(),
    //     );
    //     self.queue.submit(std::iter::once(encoder.finish()));

    //     // Map velocity_reading_buffer for reading asynchronously
    //     let buffer_slice = self.particle_velocities_reader_buffer.slice(..);
    //     let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
    //     buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
    //         sender.send(result).unwrap();
    //     });

    //     // Wait for the mapping to complete
    //     self.device.poll(wgpu::Maintain::Wait);

    //     // Check if the mapping was successful
    //     if let Ok(()) = receiver.receive().await.unwrap() {
    //         let data = buffer_slice.get_mapped_range();
    //         let velocities: &[f32] = bytemuck::cast_slice(&data);
    //         // Update the particle velocities
    //         for i in 0..self.particle_velocities.len() {
    //             self.particle_velocities[i] = [velocities[i * 2], velocities[i * 2 + 1]];
    //         }

    //         drop(data);
    //         self.particle_velocities_reader_buffer.unmap();
    //     } else {
    //         // Handle mapping error
    //         eprintln!("Error mapping buffer");
    //         // return Err(wgpu::SurfaceError::Lost); // Or handle the error appropriately
    //         return;
    //     }
    // }

    // async fn update_forces_from_buffer(&mut self) {
    //     // Copy particle forces to force_reading_buffer
    //     let mut encoder = self
    //         .device
    //         .create_command_encoder(&wgpu::CommandEncoderDescriptor {
    //             label: Some("Copy Encoder"),
    //         });
    //     encoder.copy_buffer_to_buffer(
    //         &self.particle_forces_buffer,
    //         0,
    //         &self.particle_forces_reader_buffer,
    //         0,
    //         self.particle_forces_buffer.size(),
    //     );
    //     self.queue.submit(std::iter::once(encoder.finish()));

    //     // Map force_reading_buffer for reading asynchronously
    //     let buffer_slice = self.particle_forces_reader_buffer.slice(..);
    //     let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
    //     buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
    //         sender.send(result).unwrap();
    //     });

    //     // Wait for the mapping to complete
    //     self.device.poll(wgpu::Maintain::Wait);

    //     // Check if the mapping was successful
    //     if let Ok(()) = receiver.receive().await.unwrap() {
    //         let data = buffer_slice.get_mapped_range();
    //         let forces: &[f32] = bytemuck::cast_slice(&data);
    //         // Update the particle forces
    //         for i in 0..self.particle_forces.len() {
    //             self.particle_forces[i] = [
    //                 forces[i * 4],
    //                 forces[i * 4 + 1],
    //                 forces[i * 4 + 2],
    //                 forces[i * 4 + 3],
    //             ];
    //         }

    //         drop(data);
    //         self.particle_forces_reader_buffer.unmap();
    //     } else {
    //         // Handle mapping error
    //         eprintln!("Error mapping buffer");
    //         // return Err(wgpu::SurfaceError::Lost); // Or handle the error appropriately
    //         return;
    //     }
    // }

    // async fn update_density_from_buffer(&mut self) {
    //     // Copy particle densities to density_reading_buffer
    //     let mut encoder = self
    //         .device
    //         .create_command_encoder(&wgpu::CommandEncoderDescriptor {
    //             label: Some("Copy Encoder"),
    //         });
    //     encoder.copy_buffer_to_buffer(
    //         &self.particle_densities_buffer,
    //         0,
    //         &self.particle_densities_reader_buffer,
    //         0,
    //         self.particle_densities_buffer.size(),
    //     );
    //     self.queue.submit(std::iter::once(encoder.finish()));

    //     // Map density_reading_buffer for reading asynchronously
    //     let buffer_slice = self.particle_densities_reader_buffer.slice(..);
    //     let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
    //     buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
    //         sender.send(result).unwrap();
    //     });

    //     // Wait for the mapping to complete
    //     self.device.poll(wgpu::Maintain::Wait);

    //     // Check if the mapping was successful
    //     if let Ok(()) = receiver.receive().await.unwrap() {
    //         let data = buffer_slice.get_mapped_range();
    //         let densities: &[f32] = bytemuck::cast_slice(&data);
    //         // Update the particle densities
    //         for i in 0..self.particle_densities.len() {
    //             self.particle_densities[i] = densities[i];
    //         }

    //         drop(data);
    //         self.particle_densities_reader_buffer.unmap();
    //     } else {
    //         // Handle mapping error
    //         eprintln!("Error mapping buffer");
    //         // return Err(wgpu::SurfaceError::Lost); // Or handle the error appropriately
    //         return;
    //     }
    // }

    // async fn update_radii_from_buffer(&mut self) {
    //     // Copy particle radii to radii_reading_buffer
    //     let mut encoder = self
    //         .device
    //         .create_command_encoder(&wgpu::CommandEncoderDescriptor {
    //             label: Some("Copy Encoder"),
    //         });
    //     encoder.copy_buffer_to_buffer(
    //         &self.particle_radii_buffer,
    //         0,
    //         &self.particle_radii_reader_buffer,
    //         0,
    //         self.particle_radii_buffer.size(),
    //     );
    //     self.queue.submit(std::iter::once(encoder.finish()));

    //     // Map radii_reading_buffer for reading asynchronously
    //     let buffer_slice = self.particle_radii_reader_buffer.slice(..);
    //     let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
    //     buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
    //         sender.send(result).unwrap();
    //     });

    //     // Wait for the mapping to complete
    //     self.device.poll(wgpu::Maintain::Wait);

    //     // Check if the mapping was successful
    //     if let Ok(()) = receiver.receive().await.unwrap() {
    //         let data = buffer_slice.get_mapped_range();
    //         let radii: &[f32] = bytemuck::cast_slice(&data);
    //         // Update the particle radii
    //         for i in 0..self.particle_radii.len() {
    //             self.particle_radii[i] = radii[i];
    //         }

    //         drop(data);
    //         self.particle_radii_reader_buffer.unmap();
    //     } else {
    //         // Handle mapping error
    //         eprintln!("Error mapping buffer");
    //         // return Err(wgpu::SurfaceError::Lost); // Or handle the error appropriately
    //         return;
    //     }
    // }

    async fn sort_particles(&mut self) {
        if false {
            return;
        }

        // Update the particle positions and velocities from the buffers
        // self.update_position_from_buffer().await;

        // Map all particles to their grid cell
        let mut index_map: Vec<Vec<Vec<i32>>> =
            vec![vec![vec![]; GRID_SIZE.1 as usize]; GRID_SIZE.0 as usize];
        for i in 0..self.particle_positions.len() {
            let grid = self.pos_to_grid(self.particle_positions[i].position);
            index_map[grid.0 as usize][grid.1 as usize].push(i as i32);
        }

        // Create a new list of particles
        let mut new_positions: Vec<Particle> = vec![];
        // let mut new_velocities: Vec<[f32; 2]> = vec![];
        // let mut new_radii: Vec<f32> = vec![];
        // let mut new_densities: Vec<f32> = vec![];
        // let mut new_forces: Vec<[f32; 4]> = vec![];
        let mut lookup_table = vec![-1; GRID_SIZE.0 as usize * GRID_SIZE.1 as usize];
        let mut new_counts: Vec<i32> = vec![0; GRID_SIZE.0 as usize * GRID_SIZE.1 as usize];

        // Iterate over all grid cells
        for i in 0..GRID_SIZE.0 {
            for j in 0..GRID_SIZE.1 {
                let grid_index = i + j * GRID_SIZE.0;
                let mut index = -1;

                // Iterate over all particles in the grid cell
                for k in 0..index_map[i as usize][j as usize].len() {
                    let particle_index = index_map[i as usize][j as usize][k] as usize;
                    new_positions.push(self.particle_positions[particle_index]);
                    // new_velocities.push(self.particle_velocities[particle_index]);
                    // new_radii.push(self.particle_radii[particle_index]);
                    // new_densities.push(self.particle_densities[particle_index]);
                    // new_forces.push(self.particle_forces[particle_index]);
                    if index == -1 {
                        index = new_positions.len() as i32 - 1;
                    }
                    new_counts[grid_index as usize] += 1;
                }

                lookup_table[grid_index as usize] = index;
            }
        }

        self.particle_positions = new_positions;
        // self.particle_velocities = new_velocities;
        // self.particle_radii = new_radii;
        // self.particle_densities = new_densities;
        // self.particle_forces = new_forces;
        self.particle_lookup = lookup_table;
        self.particle_counts = new_counts;

        self.queue.write_buffer(
            &self.particle_positions_buffer,
            0,
            bytemuck::cast_slice(&self.particle_positions),
        );

        // self.queue.write_buffer(
        //     &self.particle_radii_buffer,
        //     0,
        //     bytemuck::cast_slice(&self.particle_radii),
        // );

        // self.queue.write_buffer(
        //     &self.particle_velocities_buffer,
        //     0,
        //     bytemuck::cast_slice(&self.particle_velocities),
        // );

        // self.queue.write_buffer(
        //     &self.particle_densities_buffer,
        //     0,
        //     bytemuck::cast_slice(&self.particle_densities),
        // );

        // self.queue.write_buffer(
        //     &self.particle_forces_buffer,
        //     0,
        //     bytemuck::cast_slice(&self.particle_forces),
        // );

        self.queue.write_buffer(
            &self.particle_lookup_buffer,
            0,
            bytemuck::cast_slice(&self.particle_lookup),
        );

        self.queue.write_buffer(
            &self.particle_counts_buffer,
            0,
            bytemuck::cast_slice(&self.particle_counts),
        );
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let start_time = std::time::Instant::now();

        // Send mouse info to the GPU
        self.queue.write_buffer(
            &self.mouse_info_buffer,
            0,
            bytemuck::cast_slice(&[self.mouse_info]),
        );

        // Dispatch the compute density shader
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Compute Encoder"),
            });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_density_pipeline);
            compute_pass.set_bind_group(0, &self.compute_densities_bind_group, &[]);
            compute_pass.dispatch_workgroups(DISPATCH_SIZE.0, DISPATCH_SIZE.1, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        // Dispatch the compute forces shader
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Compute Forces Encoder"),
            });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Forces Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_forces_pipeline);
            compute_pass.set_bind_group(0, &self.compute_forces_bind_group, &[]);
            compute_pass.dispatch_workgroups(DISPATCH_SIZE.0, DISPATCH_SIZE.1, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        // Dispatch the compute move shader
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Compute Move Encoder"),
            });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Move Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_move_pipeline);
            compute_pass.set_bind_group(0, &self.compute_move_bind_group, &[]);
            compute_pass.dispatch_workgroups(DISPATCH_SIZE.0, DISPATCH_SIZE.1, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        // Sort the particles
        pollster::block_on(self.sort_particles());

        // Render the particles
        let drawable = self.surface.get_current_texture()?;
        let image_view_descriptor = wgpu::TextureViewDescriptor::default();
        let image_view = drawable.texture.create_view(&image_view_descriptor);

        let command_encoder_descriptor = wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        };
        let mut command_encoder = self
            .device
            .create_command_encoder(&command_encoder_descriptor);
        let color_attachment = wgpu::RenderPassColorAttachment {
            view: &image_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.75,
                    g: 0.5,
                    b: 0.25,
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        };

        let render_pass_descriptor = wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        };

        {
            let mut render_pass = command_encoder.begin_render_pass(&render_pass_descriptor);
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.render_bind_group, &[]);
            render_pass.draw(0..3, 0..1); // Draw the first triangle
            render_pass.draw(3..6, 0..1); // Draw the second triangle
        }

        self.queue.submit(std::iter::once(command_encoder.finish()));

        drawable.present();

        if self.frame_count % 10 == 0 {
            let elapsed_time = start_time.elapsed();
            println!(
                "fps: {}",
                1.0 / elapsed_time.as_micros() as f32 * 1000.0 * 1000.0
            );
            // println!("Compute shaders and rendering time: {} ms", (density_elapsed_time.as_micros() as f32 + forces_elapsed_time.as_micros() as f32 + render_elapsed_time.as_micros() as f32) / 1000.0);
            self.frame_count = 0;
        }

        self.frame_count += 1;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum CustomEvent {
    Timer,
}

async fn run() {
    env_logger::init();

    let event_loop = EventLoopBuilder::<CustomEvent>::with_user_event()
        .build()
        .unwrap();
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(SCREEN_SIZE.0, SCREEN_SIZE.1))
        .build(&event_loop)
        .unwrap();
    let event_loop_proxy = event_loop.create_proxy();

    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(TIME_BETWEEN_FRAMES));
        event_loop_proxy.send_event(CustomEvent::Timer).ok();
    });

    let mut state = State::new(&window).await;

    let render_bind_group_layout =
        bind_group_layout_generator::get_bind_group_layout(&state.device);
    state.render_bind_group = create_bind_group(&mut state, &render_bind_group_layout);

    let compute_density_bind_group_layout =
        bind_group_layout_generator::get_bind_group_layout(&state.device);
    state.compute_densities_bind_group = create_bind_group(&mut state, &compute_density_bind_group_layout);

    let compute_forces_bind_group_layout =
        bind_group_layout_generator::get_bind_group_layout(&state.device);
    state.compute_forces_bind_group = create_bind_group(&mut state, &compute_forces_bind_group_layout);

    let compute_move_bind_group_layout =
        bind_group_layout_generator::get_bind_group_layout(&state.device);
    state.compute_move_bind_group = create_bind_group(&mut state, &compute_move_bind_group_layout);

    // Pass bind group layout to pipeline builder
    let mut render_pipeline_builder = PipelineBuilder::new();
    render_pipeline_builder.set_shader_module("shaders/shader.wgsl", "vs_main", "fs_main");
    render_pipeline_builder.set_pixel_format(state.config.format);
    render_pipeline_builder.set_bind_group_layout(render_bind_group_layout);
    state.render_pipeline = render_pipeline_builder.build_pipeline(&state.device);

    // Pass bind group layout to compute pipeline builder
    let mut compute_density_pipeline_builder = ComputePipelineBuilder::new();
    compute_density_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_density");
    compute_density_pipeline_builder.set_bind_group_layout(compute_density_bind_group_layout);
    state.compute_density_pipeline = compute_density_pipeline_builder.build_pipeline(&state.device);

    // Pass bind group layout to compute forces pipeline builder
    let mut compute_forces_pipeline_builder = ComputePipelineBuilder::new();
    compute_forces_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_forces");
    compute_forces_pipeline_builder.set_bind_group_layout(compute_forces_bind_group_layout);
    state.compute_forces_pipeline = compute_forces_pipeline_builder.build_pipeline(&state.device);

    // Pass bind group layout to compute move pipeline builder
    let mut compute_move_pipeline_builder = ComputePipelineBuilder::new();
    compute_move_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_move");
    compute_move_pipeline_builder.set_bind_group_layout(compute_move_bind_group_layout);
    state.compute_move_pipeline = compute_move_pipeline_builder.build_pipeline(&state.device);

    // Sort the particles
    pollster::block_on(state.sort_particles());

    event_loop
        .run(move |event, elwt| match event {
            Event::UserEvent(..) => {
                state.window.request_redraw();
            }

            Event::WindowEvent {
                window_id,
                ref event,
            } if window_id == state.window.id() => match event {
                WindowEvent::Resized(physical_size) => state.resize(*physical_size),

                WindowEvent::CursorMoved { position, .. } => {
                    state.mouse_info[1] = position.x as f32;
                    state.mouse_info[2] = position.y as f32;
                    // println!("Mouse position: {:?}", state.mouse_info);
                }
                WindowEvent::MouseInput { state: element_state, button, .. } => {
                    if *button == MouseButton::Left {
                        state.mouse_info[0] = if *element_state == ElementState::Pressed {1.0} else {0.0};
                    }
                    if *button == MouseButton::Right {
                        state.mouse_info[3] = if *element_state == ElementState::Pressed {1.0} else {0.0};
                    }
                }

                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: PhysicalKey::Code(KeyCode::Escape),
                            state: ElementState::Pressed,
                            repeat: false,
                            ..
                        },
                    ..
                } => {
                    println!("Closing window");
                    elwt.exit();
                }

                WindowEvent::RedrawRequested => match {
                    state.render()
                } {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                    Err(e) => eprintln!("{:?}", e),
                },

                _ => (),
            },

            _ => {}
        })
        .expect("Error!");
}

fn create_bind_group(
    state: &mut State,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::BindGroup {
    let bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Particle Data Bind Group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: state.particle_positions_buffer.as_entire_binding(),
            },
            // wgpu::BindGroupEntry {
            //     binding: 1,
            //     resource: state.particle_radii_buffer.as_entire_binding(),
            // },
            // wgpu::BindGroupEntry {
            //     binding: 2,
            //     resource: state.particle_velocities_buffer.as_entire_binding(),
            // },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: state.particle_lookup_buffer.as_entire_binding(),
            },
            // wgpu::BindGroupEntry {
            //     binding: 4,
            //     resource: state.particle_densities_buffer.as_entire_binding(),
            // },
            // wgpu::BindGroupEntry {
            //     binding: 5,
            //     resource: state.particle_forces_buffer.as_entire_binding(),
            // },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: state.particle_counts_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: state.mouse_info_buffer.as_entire_binding(),
            },
        ],
    });

    bind_group
}

fn main() {
    pollster::block_on(run());
}