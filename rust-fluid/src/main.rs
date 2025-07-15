use core::f32;
use bytemuck::{Pod, Zeroable};
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

const PARTICLE_RADIUS: f32 = 1.25 / 4.0; // The radius of the particles
const PARTICLE_AMOUNT_X: u32 = 192 * 4; // The number of particles in the x direction
const PARTICLE_AMOUNT_Y: u32 = 96 * 4; // The number of particles in the y direction
const TOTAL_PARTICLES: u32 = PARTICLE_AMOUNT_X * PARTICLE_AMOUNT_Y; // The total number of particles
const PADDING: f32 = 50.0; // The padding around the screen

const BASE: u32 = 10;
const NUM_DIGITS: u32 = 5;
const BUCKET_SIZE: u32 = 32; // The amount of numbers in each bucket for the inclusive prefix sum
const NUM_BUCKETS: u32 = TOTAL_PARTICLES.div_ceil(BUCKET_SIZE); // The number of buckets

const WORKGROUP_SIZE: u32 = 16;
const DISPATCH_SIZE: (u32, u32) = (
    (PARTICLE_AMOUNT_X + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
    (PARTICLE_AMOUNT_Y + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
);
const SORT_DISPATCH_SIZE: u32 = ((TOTAL_PARTICLES + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE) as u32;

const IPS_WORKGROUP_SIZE: u32 = 16;
const IPS_DISPATCH_SIZE: u32 = ((NUM_BUCKETS + IPS_WORKGROUP_SIZE - 1) / IPS_WORKGROUP_SIZE) as u32;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Particle {
    position: [f32; 2], // 8 bytes
    velocity: [f32; 2], // 8 bytes
    radius: f32, // 4 bytes
    density: f32, // 4 bytes
    _padding: [f32; 2], // Padding, 8 bytes
    forces: [f32; 4], // 16 bytes
}

impl Particle {
    fn new(position: [f32; 2], velocity: [f32; 2], radius: f32) -> Self {
        Self {
            position,
            velocity,
            radius,
            density: 0.0,
            _padding: [0.0, 0.0],
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
    particles: Vec<Particle>,
    particle_buffer: wgpu::Buffer,
    particle_reader_buffer: wgpu::Buffer,
    particle_lookup: Vec<i32>,
    particle_lookup_buffer: wgpu::Buffer,
    particle_counts: Vec<i32>,
    particle_counts_buffer: wgpu::Buffer,
    mouse_info: [f32; 4], // 0-up; 1-down, x-pos, y-pos, 0-Atttract; 1-Repel
    mouse_info_buffer: wgpu::Buffer,
    histogram: Vec<Vec<u32>>,
    histogram_buffer: wgpu::Buffer,
    histogram_read_buffer: wgpu::Buffer,
    digit_histogram_buffer: wgpu::Buffer,
    scanned_inclusive_prefix_sum_buffer: wgpu::Buffer,
    inclusive_prefix_sum: Vec<Vec<u32>>,
    inclusive_prefix_sum_buffer: wgpu::Buffer,
    inclusive_prefix_sum_read_buffer: wgpu::Buffer,
    scan_stage_buffer: wgpu::Buffer,
    current_digit_index: u32,
    current_digit_index_buffer: wgpu::Buffer,
    sorted_data_buffer: wgpu::Buffer,
    update_histogram_pipeline: wgpu::ComputePipeline,
    update_histogram_bind_group: wgpu::BindGroup,
    update_inclusive_prefix_sum_pipeline: wgpu::ComputePipeline,
    update_inclusive_prefix_sum_bind_group: wgpu::BindGroup,
    update_indices_pipeline: wgpu::ComputePipeline,
    update_indices_bind_group: wgpu::BindGroup,
    update_lookup_pipeline: wgpu::ComputePipeline,
    update_lookup_bind_group: wgpu::BindGroup,
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
            .nth(1)
            .unwrap();
        println!("{:?}", adapter.get_info());

        let device_descriptor = wgpu::DeviceDescriptor {
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits {
                max_storage_buffers_per_shader_stage: 11,
                max_compute_invocations_per_workgroup: 1024,
                ..wgpu::Limits::default()
            },
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

        // --- Sort Pipelines --- //
        let mut update_histogram_pipeline_builder = ComputePipelineBuilder::new();
        update_histogram_pipeline_builder.set_shader_module("shaders/shader.wgsl", "update_histogram");
        update_histogram_pipeline_builder.set_bind_group_layout(
            bind_group_layout_generator::get_bind_group_layout(&device),
        );
        let update_histogram_pipeline = update_histogram_pipeline_builder.build_pipeline(&device);

        let mut update_inclusive_prefix_sum_pipeline_builder = ComputePipelineBuilder::new();
        update_inclusive_prefix_sum_pipeline_builder.set_shader_module("shaders/shader.wgsl", "update_inclusive_prefix_sum");
        update_inclusive_prefix_sum_pipeline_builder.set_bind_group_layout(
            bind_group_layout_generator::get_bind_group_layout(&device),
        );
        let update_inclusive_prefix_sum_pipeline = update_inclusive_prefix_sum_pipeline_builder.build_pipeline(&device);

        let mut update_indices_pipeline_builder = ComputePipelineBuilder::new();
        update_indices_pipeline_builder.set_shader_module("shaders/shader.wgsl", "update_indices");
        update_indices_pipeline_builder.set_bind_group_layout(
            bind_group_layout_generator::get_bind_group_layout(&device),
        );
        let update_indices_pipeline = update_indices_pipeline_builder.build_pipeline(&device);

        let mut update_lookup_pipeline_builder = ComputePipelineBuilder::new();
        update_lookup_pipeline_builder.set_shader_module("shaders/shader.wgsl", "update_lookup");
        update_lookup_pipeline_builder.set_bind_group_layout(
            bind_group_layout_generator::get_bind_group_layout(&device),
        );
        let update_lookup_pipeline = update_lookup_pipeline_builder.build_pipeline(&device);

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

        // --- Sort Bind Groups --- //
        let temp_update_histogram_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Temporary Update Histogram Bind Group"),
            layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[],
                label: Some("Temporary Update Histogram Bind Group Layout"),
            }),
            entries: &[],
        });

        let temp_update_inclusive_prefix_sum_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Temporary Update Inclusive Prefix Sum Bind Group"),
                layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[],
                    label: Some("Temporary Update Inclusive Prefix Sum Bind Group Layout"),
                }),
                entries: &[],
            });

        let temp_update_indices_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Temporary Update Indices Bind Group"),
            layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[],
                label: Some("Temporary Update Indices Bind Group Layout"),
            }),
            entries: &[],
        });

        let temp_update_lookup_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Temporary Update Lookup Bind Group"),
            layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[],
                label: Some("Temporary Update Lookup Bind Group Layout"),
            }),
            entries: &[],
        });

        // Create particle data
        let mut particles = vec![];
        for i in 0..PARTICLE_AMOUNT_X {
            for j in 0..PARTICLE_AMOUNT_Y {
                let x = (i as f32 + 0.5) * (SCREEN_SIZE.0 as f32 - 2.0 * PADDING)
                    / PARTICLE_AMOUNT_X as f32
                    + PADDING;
                let y = (j as f32 + 0.5) * (SCREEN_SIZE.1 as f32 - 2.0 * PADDING)
                    / PARTICLE_AMOUNT_Y as f32
                    + PADDING;


                particles.push(Particle::new([x, y], [0.0, 0.0], PARTICLE_RADIUS));
            }
        }
        // println!("{:?}", particles[1]);
        let particle_lookup: Vec<i32> = vec![0; GRID_SIZE.0 as usize * GRID_SIZE.1 as usize];
        let particle_counts: Vec<i32> = vec![0; GRID_SIZE.0 as usize * GRID_SIZE.1 as usize];

        // Buffer for particles
        let particle_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Particle Buffer Data"),
            contents: bytemuck::cast_slice(&particles),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });
        let particle_reader_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Reader Buffer"),
            size: (std::mem::size_of::<Particle>() * particles.len()) as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
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
            &particle_buffer,
            0,
            bytemuck::cast_slice(&particles),
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

        // Mouse info
        let mouse_info = [0.0, 0.0, 0.0, 0.0];
        let mouse_info_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Mouse Info Buffer Data"),
            contents: bytemuck::cast_slice(&mouse_info),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        // --- Sort Buffers --- //
        let histogram = vec![vec![0u32; NUM_BUCKETS as usize]; BASE as usize];
        let inclusive_prefix_sum = vec![vec![0u32; NUM_BUCKETS as usize]; BASE as usize];

        let current_digit_index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Current Digit Index Buffer"),
            contents: bytemuck::cast_slice(&[0u32]),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        });

        let histogram_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Histogram Buffer"),
            contents: bytemuck::cast_slice(&histogram.concat()),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        });

        let histogram_read_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Histogram Read Buffer"),
            size: (histogram.len() * histogram[0].len() * std::mem::size_of::<u32>()) as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let digit_histogram_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Digit Histogram Buffer"),
            contents: bytemuck::cast_slice(&[0u32; BASE as usize]),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let scanned_inclusive_prefix_sum_buffer =
            device.create_buffer_init(&BufferInitDescriptor {
                label: Some("Sorted Data Buffer"),
                contents: bytemuck::cast_slice(&inclusive_prefix_sum.concat()),
                usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            });

        let inclusive_prefix_sum_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("inclusive Prefix Sum Buffer"),
            contents: bytemuck::cast_slice(&inclusive_prefix_sum.concat()),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        });

        let inclusive_prefix_sum_read_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("inclusive Prefix Sum Read Buffer"),
            size: (inclusive_prefix_sum.len()
                * inclusive_prefix_sum[0].len()
                * std::mem::size_of::<u32>()) as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sorted_data_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Sorted Data Buffer"),
            contents: bytemuck::cast_slice(&particles),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        });

        let scan_stage_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Scan Stage Buffer"),
            contents: bytemuck::cast_slice(&[0u32]),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
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
            particles,
            particle_buffer,
            particle_reader_buffer,
            particle_lookup,
            particle_lookup_buffer,
            particle_counts,
            particle_counts_buffer,
            mouse_info,
            mouse_info_buffer,
            histogram,
            histogram_buffer,
            histogram_read_buffer,
            digit_histogram_buffer,
            scanned_inclusive_prefix_sum_buffer,
            inclusive_prefix_sum,
            inclusive_prefix_sum_buffer,
            inclusive_prefix_sum_read_buffer,
            scan_stage_buffer,
            current_digit_index: 0,
            current_digit_index_buffer,
            sorted_data_buffer,
            update_histogram_pipeline,
            update_histogram_bind_group: temp_update_histogram_bind_group,
            update_inclusive_prefix_sum_pipeline,
            update_inclusive_prefix_sum_bind_group: temp_update_inclusive_prefix_sum_bind_group,
            update_indices_pipeline,
            update_indices_bind_group: temp_update_indices_bind_group,
            update_lookup_pipeline,
            update_lookup_bind_group: temp_update_lookup_bind_group,
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

    async fn update_particles_from_buffer(&mut self) {
        // Copy particles to particle_reading_buffer
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Copy Encoder"),
            });
        encoder.copy_buffer_to_buffer(
            &self.particle_buffer,
            0,
            &self.particle_reader_buffer,
            0,
            self.particle_buffer.size(),
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        // Map particle_reading_buffer for reading asynchronously
        let buffer_slice = self.particle_reader_buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        // Wait for the mapping to complete
        self.device.poll(wgpu::Maintain::Wait);

        // Check if the mapping was successful
        if let Ok(()) = receiver.receive().await.unwrap() {
            let data = buffer_slice.get_mapped_range();
            self.particles = bytemuck::cast_slice(&data).to_vec();
            
            drop(data);
            self.particle_reader_buffer.unmap();
        } else {
            // Handle mapping error
            eprintln!("Error mapping buffer");
            // return Err(wgpu::SurfaceError::Lost); // Or handle the error appropriately
            return;
        }
    }
    
    fn sort_particles(&mut self) {
        for i in 0..NUM_DIGITS {
            self.sort_particles_by_digit(i);
        }
    }

    fn sort_particles_by_digit(&mut self, digit: u32) {
        // Reset the histogram buffer
        self.queue.write_buffer(
            &self.histogram_buffer,
            0,
            bytemuck::cast_slice(&[0u32; NUM_BUCKETS as usize * BASE as usize]),
        );

        // Reset the digit histogram buffer
        self.queue.write_buffer(
            &self.digit_histogram_buffer,
            0,
            bytemuck::cast_slice(&[0u32; BASE as usize]),
        );

        // Reset particle counts if it is the last digit
        if digit == NUM_DIGITS - 1 {
            self.queue.write_buffer(
                &self.particle_counts_buffer,
                0,
                bytemuck::cast_slice(&vec![0; GRID_SIZE.0 as usize * GRID_SIZE.1 as usize]),
            );
        }

        // Set the current digit index
        self.queue.write_buffer(
            &self.current_digit_index_buffer,
            0,
            bytemuck::cast_slice(&[digit]),
        );

        // Dispatch the histogram compute shader
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Compute Histogram Encoder"),
            });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.update_histogram_pipeline);
            compute_pass.set_bind_group(0, &self.update_histogram_bind_group, &[]);
            compute_pass.dispatch_workgroups(SORT_DISPATCH_SIZE, 1, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        // Set the inclusive prefix sum buffer to the histogram buffer
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Copy Encoder"),
            });
        encoder.copy_buffer_to_buffer(
            &self.histogram_buffer,
            0,
            &self.inclusive_prefix_sum_buffer,
            0,
            (self.histogram.len() * self.histogram[0].len() * std::mem::size_of::<u32>()) as u64,
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Dispatch the inclusive prefix sum compute shader
        let loops_needed = (NUM_BUCKETS as f32).log2().ceil() as u32;
        for i in 0..loops_needed {
            // Update the scan stage buffer
            self.queue
                .write_buffer(&self.scan_stage_buffer, 0, bytemuck::cast_slice(&[i]));

            // Dispatch the inclusive prefix sum compute shader
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Compute Inclusive Prefix Sum Encoder"),
                });
            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Compute Pass"),
                    timestamp_writes: None,
                });
                compute_pass.set_pipeline(&self.update_inclusive_prefix_sum_pipeline);
                compute_pass.set_bind_group(0, &self.update_inclusive_prefix_sum_bind_group, &[]);
                compute_pass.dispatch_workgroups(IPS_DISPATCH_SIZE, BASE, 1);
            }

            self.queue.submit(std::iter::once(encoder.finish()));

            // Copy the scanned inclusive prefix sum buffer to the inclusive prefix sum buffer
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Copy Encoder"),
                });
            encoder.copy_buffer_to_buffer(
                &self.scanned_inclusive_prefix_sum_buffer,
                0,
                &self.inclusive_prefix_sum_buffer,
                0,
                (self.inclusive_prefix_sum.len()
                    * self.inclusive_prefix_sum[0].len()
                    * std::mem::size_of::<u32>()) as u64,
            );

            self.queue.submit(std::iter::once(encoder.finish()));
        }

        // Dispatch the indices compute shader
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Compute Indices Encoder"),
            });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.update_indices_pipeline);
            compute_pass.set_bind_group(0, &self.update_indices_bind_group, &[]);
            compute_pass.dispatch_workgroups(SORT_DISPATCH_SIZE, 1, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        // Copy the sorted data to the data buffer
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Copy Encoder"),
            });
        encoder.copy_buffer_to_buffer(
            &self.sorted_data_buffer,
            0,
            &self.particle_buffer,
            0,
            (self.particles.len() * std::mem::size_of::<Particle>())
                as u64,
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Update particle lookup if it is the last digit
        if digit == NUM_DIGITS - 1 {
            // Dispatch the lookup compute shader
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Compute Lookup Encoder"),
                });
            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Compute Pass"),
                    timestamp_writes: None,
                });
                compute_pass.set_pipeline(&self.update_lookup_pipeline);
                compute_pass.set_bind_group(0, &self.update_lookup_bind_group, &[]);
                compute_pass.dispatch_workgroups(SORT_DISPATCH_SIZE, 1, 1);
            }

            self.queue.submit(std::iter::once(encoder.finish()));
        }
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

        // Reset particle lookup
        self.queue.write_buffer(
            &self.particle_lookup_buffer,
            0,
            bytemuck::cast_slice(&vec![-1; GRID_SIZE.0 as usize * GRID_SIZE.1 as usize]),
        );
        
        // Sort the particles
        self.sort_particles();

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

    // --- Sort Bind Groups --- //
    let update_histogram_bind_group_layout =
        bind_group_layout_generator::get_bind_group_layout(&state.device);
    state.update_histogram_bind_group = create_bind_group(&mut state, &update_histogram_bind_group_layout);

    let update_inclusive_prefix_sum_bind_group_layout =
        bind_group_layout_generator::get_bind_group_layout(&state.device);
    state.update_inclusive_prefix_sum_bind_group = create_bind_group(&mut state, &update_inclusive_prefix_sum_bind_group_layout);

    let update_indices_bind_group_layout =
        bind_group_layout_generator::get_bind_group_layout(&state.device);
    state.update_indices_bind_group = create_bind_group(&mut state, &update_indices_bind_group_layout);

    let update_lookup_bind_group_layout =
        bind_group_layout_generator::get_bind_group_layout(&state.device);
    state.update_lookup_bind_group = create_bind_group(&mut state, &update_lookup_bind_group_layout);

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

    // --- Sort Pipelines --- //
    let mut update_histogram_pipeline_builder = ComputePipelineBuilder::new();
    update_histogram_pipeline_builder.set_shader_module("shaders/shader.wgsl", "update_histogram");
    update_histogram_pipeline_builder.set_bind_group_layout(update_histogram_bind_group_layout);
    state.update_histogram_pipeline = update_histogram_pipeline_builder.build_pipeline(&state.device);

    let mut update_inclusive_prefix_sum_pipeline_builder = ComputePipelineBuilder::new();
    update_inclusive_prefix_sum_pipeline_builder.set_shader_module("shaders/shader.wgsl", "update_inclusive_prefix_sum");
    update_inclusive_prefix_sum_pipeline_builder.set_bind_group_layout(update_inclusive_prefix_sum_bind_group_layout);
    state.update_inclusive_prefix_sum_pipeline = update_inclusive_prefix_sum_pipeline_builder.build_pipeline(&state.device);

    let mut update_indices_pipeline_builder = ComputePipelineBuilder::new();
    update_indices_pipeline_builder.set_shader_module("shaders/shader.wgsl", "update_indices");
    update_indices_pipeline_builder.set_bind_group_layout(update_indices_bind_group_layout);
    state.update_indices_pipeline = update_indices_pipeline_builder.build_pipeline(&state.device);

    let mut update_lookup_pipeline_builder = ComputePipelineBuilder::new();
    update_lookup_pipeline_builder.set_shader_module("shaders/shader.wgsl", "update_lookup");
    update_lookup_pipeline_builder.set_bind_group_layout(update_lookup_bind_group_layout);
    state.update_lookup_pipeline = update_lookup_pipeline_builder.build_pipeline(&state.device);

    // Sort the particles
    state.sort_particles();

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
                resource: state.particle_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: state.particle_lookup_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: state.particle_counts_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: state.mouse_info_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: state.histogram_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: state.inclusive_prefix_sum_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: state.current_digit_index_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: state.sorted_data_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 8,
                resource: state.scan_stage_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 9,
                resource: state
                    .scanned_inclusive_prefix_sum_buffer
                    .as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 10,
                resource: state.digit_histogram_buffer.as_entire_binding(),
            },
        ],
    });

    bind_group
}

fn main() {

    pollster::block_on(run());
}