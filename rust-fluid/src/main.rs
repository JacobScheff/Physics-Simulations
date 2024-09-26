use core::f32;
use std::collections::HashMap;

use renderer_backend::{
    bind_group_layout_generator, compute_pipeline_builder::ComputePipelineBuilder,
    pipeline_builder::PipelineBuilder,
};
mod renderer_backend;
use cgmath::prelude::*;
use rand::*;
use wgpu::{
    core::device::global,
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
const TIME_BETWEEN_FRAMES: u64 = 10;
const OFFSET: (f32, f32) = (10.0, 8.0); // How much to offset all the particle's starting positions
const GRID_SIZE: (i32, i32) = (80, 40); // How many grid cells to divide the screen into

const PARTICLE_RADIUS: f32 = 1.25 * 2.0; // The radius of the particles
const PARTICLE_AMOUNT_X: u32 = 192 / 2; // The number of particles in the x direction
const PARTICLE_AMOUNT_Y: u32 = 96 / 2; // The number of particles in the y direction
const PADDING: f32 = 100.0; // The padding around the screen
                            // const RADIUS_OF_INFLUENCE: f32 = 75.0; // The radius of the sphere of influence. Also the radius to search for particles to calculate the density
                            // const TARGET_DENSITY: f32 = 0.2; // The target density of the fluid
                            // const PRESURE_MULTIPLIER: f32 = 100.0; // The multiplier for the pressure force
const GRAVITY: f32 = 1.0; // The strength of gravity
                          // const LOOK_AHEAD_TIME: f32 = 1.0 / 60.0; // The time to look ahead when calculating the predicted position
                          // const VISCOSITY: f32 = 0.5; // The viscosity of the fluid
const DAMPENING: f32 = 0.95; // How much to slow down particles when they collide with the walls

// const grids_to_check: (i32, i32) = (
//     (RADIUS_OF_INFLUENCE / SCREEN_SIZE.0 as f32 * GRID_SIZE.0 as f32 + 0.5) as i32,
//     (RADIUS_OF_INFLUENCE / SCREEN_SIZE.1 as f32 * GRID_SIZE.1 as f32 + 0.5) as i32,
// );

const WORKGROUP_SIZE: u32 = 10;
const DISPATCH_SIZE: (u32, u32) = (
    (PARTICLE_AMOUNT_X + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
    (PARTICLE_AMOUNT_Y + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
);

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
    compute_sort_pipeline: wgpu::ComputePipeline,
    render_bind_group: wgpu::BindGroup,
    compute_densities_bind_group: wgpu::BindGroup,
    compute_forces_bind_group: wgpu::BindGroup,
    compute_sort_bind_group: wgpu::BindGroup,
    frame_count: u32,
    particle_positions: Vec<[f32; 2]>,
    particle_positions_buffer: wgpu::Buffer,
    particle_radii: Vec<f32>,
    particle_radii_buffer: wgpu::Buffer,
    particle_velocities: Vec<[f32; 2]>,
    particle_velocities_buffer: wgpu::Buffer,
    particle_densities: Vec<f32>,
    particle_densities_buffer: wgpu::Buffer,
    particle_forces: Vec<[f32; 4]>,
    particle_forces_buffer: wgpu::Buffer,
    particle_lookup: Vec<i32>,
    particle_lookup_buffer: wgpu::Buffer,
    grid_index_map: Vec<[i32; 2]>,
    grid_index_map_buffer: wgpu::Buffer,
    position_reading_buffer: wgpu::Buffer,
    velocity_reading_buffer: wgpu::Buffer,
    density_reading_buffer: wgpu::Buffer,
    force_reading_buffer: wgpu::Buffer,
}

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

    fn grid_to_index(&self, grid: (i32, i32)) -> i32 {
        grid.0 + grid.1 * GRID_SIZE.0
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
            &self.position_reading_buffer,
            0,
            self.particle_positions_buffer.size(),
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        // Map position_reading_buffer for reading asynchronously
        let buffer_slice = self.position_reading_buffer.slice(..);
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
            // for i in 0..self.particle_positions.len() {
            //     self.particle_positions[i] = [positions[i * 2], positions[i * 2 + 1]];
            // }
            self.particle_positions = positions
                .chunks(2)
                .map(|chunk| [chunk[0], chunk[1]])
                .collect();

            drop(data);
            self.position_reading_buffer.unmap();
        } else {
            // Handle mapping error
            eprintln!("Error mapping buffer");
            // return Err(wgpu::SurfaceError::Lost); // Or handle the error appropriately
            return;
        }
    }

    async fn update_velocities_from_buffer(&mut self) {
        // Copy particle velocities to velocity_reading_buffer
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Copy Encoder"),
            });
        encoder.copy_buffer_to_buffer(
            &self.particle_velocities_buffer,
            0,
            &self.velocity_reading_buffer,
            0,
            self.particle_velocities_buffer.size(),
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        // Map velocity_reading_buffer for reading asynchronously
        let buffer_slice = self.velocity_reading_buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        // Wait for the mapping to complete
        self.device.poll(wgpu::Maintain::Wait);

        // Check if the mapping was successful
        if let Ok(()) = receiver.receive().await.unwrap() {
            let data = buffer_slice.get_mapped_range();
            let velocities: &[f32] = bytemuck::cast_slice(&data);
            // Update the particle velocities
            // for i in 0..self.particle_velocities.len() {
            //     self.particle_velocities[i] = [velocities[i * 2], velocities[i * 2 + 1]];
            // }
            self.particle_velocities = velocities
                .chunks(2)
                .map(|chunk| [chunk[0], chunk[1]])
                .collect();

            drop(data);
            self.velocity_reading_buffer.unmap();
        } else {
            // Handle mapping error
            eprintln!("Error mapping buffer");
            // return Err(wgpu::SurfaceError::Lost); // Or handle the error appropriately
            return;
        }
    }

    async fn update_densities_from_buffer(&mut self) {
        // Copy particle densities to density_reading_buffer
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Copy Encoder"),
            });
        encoder.copy_buffer_to_buffer(
            &self.particle_densities_buffer,
            0,
            &self.density_reading_buffer,
            0,
            self.particle_densities_buffer.size(),
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        // Map density_reading_buffer for reading asynchronously
        let buffer_slice = self.density_reading_buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        // Wait for the mapping to complete
        self.device.poll(wgpu::Maintain::Wait);

        // Check if the mapping was successful
        if let Ok(()) = receiver.receive().await.unwrap() {
            let data = buffer_slice.get_mapped_range();
            let densities: &[f32] = bytemuck::cast_slice(&data);
            // // Update the particle densities
            // for i in 0..self.particle_densities.len() {
            //     self.particle_densities[i] = densities[i];
            // }
            self.particle_densities = densities.to_vec();

            drop(data);
            self.density_reading_buffer.unmap();
        } else {
            // Handle mapping error
            eprintln!("Error mapping buffer");
            // return Err(wgpu::SurfaceError::Lost); // Or handle the error appropriately
            return;
        }
    }

    async fn update_forces_from_buffer(&mut self) {
        // Copy particle forces to force_reading_buffer
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Copy Encoder"),
            });
        encoder.copy_buffer_to_buffer(
            &self.particle_forces_buffer,
            0,
            &self.force_reading_buffer,
            0,
            self.particle_forces_buffer.size(),
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        // Map force_reading_buffer for reading asynchronously
        let buffer_slice = self.force_reading_buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        // Wait for the mapping to complete
        self.device.poll(wgpu::Maintain::Wait);

        // Check if the mapping was successful
        if let Ok(()) = receiver.receive().await.unwrap() {
            let data = buffer_slice.get_mapped_range();
            let forces: &[f32] = bytemuck::cast_slice(&data);
            // Update the particle forces
            // for i in 0..self.particle_forces.len() {
            //     self.particle_forces[i] = [forces[i * 4], forces[i * 4 + 1], forces[i * 4 + 2], forces[i * 4 + 3]];
            // }
            self.particle_forces = forces
                .chunks(4)
                .map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3]])
                .collect();

            drop(data);
            self.force_reading_buffer.unmap();
        } else {
            // Handle mapping error
            eprintln!("Error mapping buffer");
            // return Err(wgpu::SurfaceError::Lost); // Or handle the error appropriately
            return;
        }
    }

    async fn sort_particles(&mut self) {
        // Create a new list of particles
        let mut new_positions: Vec<[f32; 2]> = Vec::with_capacity(self.particle_positions.len());
        let mut new_velocities: Vec<[f32; 2]> = Vec::with_capacity(self.particle_velocities.len());
        let mut new_radii: Vec<f32> = Vec::with_capacity(self.particle_radii.len());
        let mut new_densities: Vec<f32> = Vec::with_capacity(self.particle_densities.len());
        let mut new_forces: Vec<[f32; 4]> = vec![[0.0, 0.0, 0.0, 0.0]; self.particle_forces.len()];
        let mut lookup_table = vec![-1; GRID_SIZE.0 as usize * GRID_SIZE.1 as usize];

        // Create a HashMap to store the indices of the particles in each grid cell
        let mut index_map: HashMap<(i32, i32), Vec<i32>> = HashMap::new();
        for i in 0..self.particle_positions.len() {
            let grid = self.pos_to_grid(self.particle_positions[i]);
            index_map.entry(grid).or_insert(vec![]).push(i as i32);
        }

        for i in 0..GRID_SIZE.0 {
            for j in 0..GRID_SIZE.1 {
                let grid_index = i + j * GRID_SIZE.0;
                let mut index = -1;

                // Check if the grid cell exists in the HashMap:
                if let Some(particle_indices) = index_map.get(&(i, j)) {
                    for &particle_index in particle_indices {
                        let particle_index = particle_index as usize;
                        new_positions.push(self.particle_positions[particle_index]);
                        new_velocities.push(self.particle_velocities[particle_index]);
                        new_radii.push(self.particle_radii[particle_index]);
                        new_densities.push(self.particle_densities[particle_index]);
                        if index == -1 {
                            index = new_positions.len() as i32 - 1;
                        }
                    }
                }

                lookup_table[grid_index as usize] = index;
            }
        }

        self.particle_positions = new_positions;
        self.particle_velocities = new_velocities;
        self.particle_radii = new_radii;
        self.particle_densities = new_densities;
        self.particle_forces = new_forces;
        self.particle_lookup = lookup_table;

        self.queue.write_buffer(
            &self.particle_positions_buffer,
            0,
            bytemuck::cast_slice(&self.particle_positions),
        );

        self.queue.write_buffer(
            &self.particle_radii_buffer,
            0,
            bytemuck::cast_slice(&self.particle_radii),
        );

        self.queue.write_buffer(
            &self.particle_velocities_buffer,
            0,
            bytemuck::cast_slice(&self.particle_velocities),
        );

        self.queue.write_buffer(
            &self.particle_densities_buffer,
            0,
            bytemuck::cast_slice(&self.particle_densities),
        );

        self.queue.write_buffer(
            &self.particle_forces_buffer,
            0,
            bytemuck::cast_slice(&self.particle_forces),
        );

        self.queue.write_buffer(
            &self.particle_lookup_buffer,
            0,
            bytemuck::cast_slice(&self.particle_lookup),
        );
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
            .nth(1)
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
            bind_group_layout_generator::get_bind_group_layout(&device, false),
        );
        let render_pipeline = render_pipeline_builder.build_pipeline(&device);

        // Pass bind group layout to compute pipeline builder
        let mut compute_density_pipeline_builder = ComputePipelineBuilder::new();
        compute_density_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_density");
        compute_density_pipeline_builder.set_bind_group_layout(
            bind_group_layout_generator::get_bind_group_layout(&device, true),
        );
        let compute_density_pipeline = compute_density_pipeline_builder.build_pipeline(&device);

        // Pass bind group layout to compute forces pipeline builder
        let mut compute_forces_pipeline_builder = ComputePipelineBuilder::new();
        compute_forces_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_forces");
        compute_forces_pipeline_builder.set_bind_group_layout(
            bind_group_layout_generator::get_bind_group_layout(&device, true),
        );
        let compute_forces_pipeline = compute_forces_pipeline_builder.build_pipeline(&device);

        // Pass bind group layout to compute sort pipeline builder
        let mut compute_sort_pipeline_builder = ComputePipelineBuilder::new();
        compute_sort_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_sort");
        compute_sort_pipeline_builder.set_bind_group_layout(
            bind_group_layout_generator::get_bind_group_layout(&device, true),
        );
        let compute_sort_pipeline = compute_sort_pipeline_builder.build_pipeline(&device);

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

        let temp_compute_forces_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Temporary Compute Forces Bind Group"),
            layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[],
                label: Some("Temporary Compute Forces Bind Group Layout"),
            }),
            entries: &[],
        });

        let temp_compute_sort_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Temporary Compute Sort Bind Group"),
            layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[],
                label: Some("Temporary Compute Sort Bind Group Layout"),
            }),
            entries: &[],
        });

        // Create particle data
        let mut particle_positions = vec![];
        let mut particle_velocities = vec![];
        let mut particle_radii = vec![];
        let mut particle_densities = vec![];
        let mut particle_forces = vec![];
        let mut grid_index_map = vec![];
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

                particle_positions.push([x, y]);
                particle_velocities.push([0.0, 0.0]);
                // particle_velocities.push([
                //     rand::thread_rng().gen_range(-1.0..1.0),
                //     rand::thread_rng().gen_range(-1.0..1.0),
                // ]);
                particle_radii.push(PARTICLE_RADIUS);
                particle_densities.push(0.0);
                particle_forces.push([0.0, 0.0, 0.0, 0.0]);
                grid_index_map.push([pos_to_grid_index((x, y)), (j + i * PARTICLE_AMOUNT_Y) as i32]);
            }
        }
        let particle_lookup: Vec<i32> = vec![0; GRID_SIZE.0 as usize * GRID_SIZE.1 as usize];

        // Buffer for particles
        let particle_positions_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Particle Positions Buffer Data"),
            contents: bytemuck::cast_slice(&particle_positions),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });
        let position_reading_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Position Reading Buffer Data"),
            contents: bytemuck::cast_slice(&particle_positions),
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        });
        let particle_velocities_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Particle Velocities Buffer Data"),
            contents: bytemuck::cast_slice(&particle_velocities),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });
        let velocity_reading_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Velocity Reading Buffer Data"),
            contents: bytemuck::cast_slice(&particle_velocities),
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        });
        let particle_radii_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Particle Radii Buffer Data"),
            contents: bytemuck::cast_slice(&particle_radii),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        let particle_densities_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Particle Densities Buffer Data"),
            contents: bytemuck::cast_slice(&particle_densities),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });
        let density_reading_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Density Reading Buffer Data"),
            contents: bytemuck::cast_slice(&particle_densities),
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        });
        let particle_forces_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Particle Forces Buffer Data"),
            contents: bytemuck::cast_slice(&particle_forces),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });
        let force_reading_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Force Reading Buffer Data"),
            contents: bytemuck::cast_slice(&particle_forces),
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        });
        let particle_lookup_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Particle Lookup Buffer Data"),
            contents: bytemuck::cast_slice(&particle_lookup),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        let grid_index_map_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Grid Index Map Buffer Data"),
            contents: bytemuck::cast_slice(&grid_index_map),
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
            &grid_index_map_buffer,
            0,
            bytemuck::cast_slice(&grid_index_map),
        );

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
            compute_sort_pipeline,
            render_bind_group: temp_render_bind_group,
            compute_densities_bind_group: temp_compute_density_bind_group,
            compute_forces_bind_group: temp_compute_forces_bind_group,
            compute_sort_bind_group: temp_compute_sort_bind_group,
            frame_count: 0,
            particle_positions,
            particle_positions_buffer,
            particle_radii,
            particle_radii_buffer,
            particle_velocities,
            particle_velocities_buffer,
            particle_densities,
            particle_densities_buffer,
            particle_forces,
            particle_forces_buffer,
            particle_lookup,
            particle_lookup_buffer,
            grid_index_map,
            grid_index_map_buffer,
            position_reading_buffer,
            velocity_reading_buffer,
            density_reading_buffer,
            force_reading_buffer,
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

    // fn density_to_pressure(&self, density: f32) -> f32 {
    //     let density_error = density - TARGET_DENSITY;
    //     return density_error * PRESURE_MULTIPLIER;
    // }

    // fn smoothing_kernel(&self, distance: f32) -> f32 {
    //     if distance >= RADIUS_OF_INFLUENCE {
    //         return 0.0;
    //     }

    //     let volume = f32::consts::PI * RADIUS_OF_INFLUENCE.powi(4) / 6.0;
    //     return (RADIUS_OF_INFLUENCE - distance) * (RADIUS_OF_INFLUENCE - distance) / volume;
    // }

    // fn smoothing_kernel_derivative(&self, distance: f32) -> f32 {
    //     if distance >= RADIUS_OF_INFLUENCE {
    //         return 0.0;
    //     }

    //     let scale = 12.0 / RADIUS_OF_INFLUENCE.powi(4) * f32::consts::PI;
    //     return (RADIUS_OF_INFLUENCE - distance) * scale;
    // }

    // fn viscosity_kernel(&self, distance: f32) -> f32 {
    //     if distance >= RADIUS_OF_INFLUENCE {
    //         return 0.0;
    //     }

    //     let volume = f32::consts::PI * RADIUS_OF_INFLUENCE.powi(8) / 4.0;
    //     let value = RADIUS_OF_INFLUENCE * RADIUS_OF_INFLUENCE - distance * distance;
    //     return value * value * value / volume;
    // }

    // fn calculate_shared_pressure(&self, density_a: f32, density_b: f32) -> f32 {
    //     let pressure_a = self.density_to_pressure(density_a);
    //     let pressure_b = self.density_to_pressure(density_b);
    //     return (pressure_a + pressure_b) / 2.0;
    // }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let start_time = std::time::Instant::now();

        // Update the particles from the buffers
        // let update_start_time = std::time::Instant::now();
        // pollster::block_on(self.update_position_from_buffer());
        // pollster::block_on(self.update_velocities_from_buffer());
        // pollster::block_on(self.update_densities_from_buffer());
        // pollster::block_on(self.update_forces_from_buffer());
        // let update_elapsed_time = update_start_time.elapsed();
        // println!(
        //     "Update time: {} ms",
        //     update_elapsed_time.as_micros() as f32 / 1000.0
        // );

        // // Apply forces and move the particles
        // let move_start_time = std::time::Instant::now();
        // for i in 0..self.particle_positions.len() {
        //     let mut acceleration = [
        //         self.particle_forces[i][0] / self.particle_densities[i].max(0.0001),
        //         self.particle_forces[i][1] / self.particle_densities[i].max(0.0001) + GRAVITY,
        //     ];
        //     if self.particle_densities[i] == 0.0 {
        //         acceleration = [0.0, 0.0];
        //     }
        //     self.particle_velocities[i][0] += acceleration[0];
        //     self.particle_velocities[i][1] += acceleration[1];

        //     self.particle_positions[i][0] += self.particle_velocities[i][0];
        //     self.particle_positions[i][1] += self.particle_velocities[i][1];

        //     if self.particle_positions[i][0] < 0.0 {
        //         self.particle_positions[i][0] = 0.0;
        //         self.particle_velocities[i][0] = -self.particle_velocities[i][0] * DAMPENING;
        //     }

        //     if self.particle_positions[i][0] > SCREEN_SIZE.0 as f32 {
        //         self.particle_positions[i][0] = SCREEN_SIZE.0 as f32;
        //         self.particle_velocities[i][0] = -self.particle_velocities[i][0] * DAMPENING;
        //     }

        //     if self.particle_positions[i][1] < 0.0 {
        //         self.particle_positions[i][1] = 0.0;
        //         self.particle_velocities[i][1] = -self.particle_velocities[i][1] * DAMPENING;
        //     }

        //     if self.particle_positions[i][1] > SCREEN_SIZE.1 as f32 {
        //         self.particle_positions[i][1] = SCREEN_SIZE.1 as f32;
        //         self.particle_velocities[i][1] = -self.particle_velocities[i][1] * DAMPENING;
        //     }
        // }
        // let move_elapsed_time = move_start_time.elapsed();
        // println!(
        //     "Move time: {} ms",
        //     move_elapsed_time.as_micros() as f32 / 1000.0
        // );

        // Sort the particles into their grid cells
        // let sort_start_time = std::time::Instant::now();
        // pollster::block_on(self.sort_particles());
        // let sort_elapsed_time = sort_start_time.elapsed();
        // println!(
        //     "Sort time: {} ms",
        //     sort_elapsed_time.as_micros() as f32 / 1000.0
        // );

        let density_start_time = std::time::Instant::now();
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

        let density_elapsed_time = density_start_time.elapsed();
        // println!(
        //     "Density calculation time: {} ms",
        //     density_elapsed_time.as_micros() as f32 / 1000.0
        // );

        // Dispatch the compute forces shader
        let forces_start_time = std::time::Instant::now();
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
            compute_pass.set_pipeline(&self.compute_forces_pipeline); // Assuming you have a compute pipeline
            compute_pass.set_bind_group(0, &self.compute_forces_bind_group, &[]);
            compute_pass.dispatch_workgroups(DISPATCH_SIZE.0, DISPATCH_SIZE.1, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        let forces_elapsed_time = forces_start_time.elapsed();
        // println!(
        //     "Forces calculation time: {} ms",
        //     forces_elapsed_time.as_micros() as f32 / 1000.0
        // );

        // Dispatch the compute sort shader
        let sort_start_time = std::time::Instant::now();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Compute Sort Encoder"),
            });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Sort Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_sort_pipeline); // Assuming you have a compute pipeline
            compute_pass.set_bind_group(0, &self.compute_sort_bind_group, &[]);
            compute_pass.dispatch_workgroups(1, 1, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        let sort_elapsed_time = sort_start_time.elapsed();
        // println!(
        //     "Sort calculation time: {} ms",
        //     sort_elapsed_time.as_micros() as f32 / 1000.0
        // );

        // // Render the particles
        // let render_start_time = std::time::Instant::now();
        // let image_view_descriptor = wgpu::TextureViewDescriptor::default();

        // let command_encoder_descriptor = wgpu::CommandEncoderDescriptor {
        //     label: Some("Render Encoder"),
        // };
        // let mut command_encoder = self
        //     .device
        //     .create_command_encoder(&command_encoder_descriptor);

        // let render_pass_descriptor = wgpu::RenderPassDescriptor {
        //     label: Some("Render Pass"),
        //     color_attachments: &[],
        //     depth_stencil_attachment: None,
        //     occlusion_query_set: None,
        //     timestamp_writes: None,
        // };

        // {
        //     let mut render_pass = command_encoder.begin_render_pass(&render_pass_descriptor);
        //     render_pass.set_pipeline(&self.render_pipeline);
        //     render_pass.set_bind_group(0, &self.render_bind_group, &[]); // Access using self
        //     render_pass.draw(0..3, 0..1); // Draw the first triangle
        //     render_pass.draw(3..6, 0..1); // Draw the second triangle
        // }

        // self.queue.submit(std::iter::once(command_encoder.finish()));

        // let render_elapsed_time = render_start_time.elapsed();
        // println!(
        //     "Render time: {} ms",
        //     render_elapsed_time.as_micros() as f32 / 1000.0
        // );

        // println!("Problem is probably with main_sort insertion sort, not updating positions");

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
    state.sort_particles().await;

    let render_bind_group_layout =
        bind_group_layout_generator::get_bind_group_layout(&state.device, false);
    state.render_bind_group = create_bind_group(&mut state, &render_bind_group_layout);

    let compute_bind_group_layout =
        bind_group_layout_generator::get_bind_group_layout(&state.device, true);
    state.compute_densities_bind_group = create_bind_group(&mut state, &compute_bind_group_layout);

    let compute_forces_bind_group_layout =
        bind_group_layout_generator::get_bind_group_layout(&state.device, true);
    state.compute_forces_bind_group = create_bind_group(&mut state, &compute_forces_bind_group_layout);

    let compute_sort_bind_group_layout =
        bind_group_layout_generator::get_bind_group_layout(&state.device, true);
    state.compute_sort_bind_group = create_bind_group(&mut state, &compute_sort_bind_group_layout);

    // Pass bind group layout to pipeline builder
    let mut render_pipeline_builder = PipelineBuilder::new();
    render_pipeline_builder.set_shader_module("shaders/shader.wgsl", "vs_main", "fs_main");
    render_pipeline_builder.set_pixel_format(state.config.format);
    render_pipeline_builder.set_bind_group_layout(render_bind_group_layout);
    state.render_pipeline = render_pipeline_builder.build_pipeline(&state.device);

    // Pass bind group layout to compute pipeline builder
    let mut compute_density_pipeline_builder = ComputePipelineBuilder::new();
    compute_density_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_density");
    compute_density_pipeline_builder.set_bind_group_layout(compute_bind_group_layout);
    state.compute_density_pipeline = compute_density_pipeline_builder.build_pipeline(&state.device);

    // Pass bind group layout to compute forces pipeline builder
    let mut compute_forces_pipeline_builder = ComputePipelineBuilder::new();
    compute_forces_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_forces");
    compute_forces_pipeline_builder.set_bind_group_layout(compute_forces_bind_group_layout);
    state.compute_forces_pipeline = compute_forces_pipeline_builder.build_pipeline(&state.device);

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
        label: Some("Sphere Bind Group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: state.particle_positions_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: state.particle_radii_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: state.particle_velocities_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: state.particle_lookup_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: state.particle_densities_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: state.particle_forces_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: state.grid_index_map_buffer.as_entire_binding(),
            },
        ],
    });

    bind_group
}

fn main() {
    pollster::block_on(run());
}
