use renderer_backend::{
    bind_group_layout_generator, compute_pipeline_builder::ComputePipelineBuilder,
    pipeline_builder::PipelineBuilder,
};
mod renderer_backend;
use cgmath::prelude::*;
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
const PARTICLE_COUNT_X: u32 = 100;
const PARTICLE_COUNT_Y: u32 = 100;
const OFFSET: (f32, f32) = (10.0, 8.0); // How much to offset all the particle's starting positions
const GRID_SIZE: (i32, i32) = (80, 40); // How many grid cells to divide the screen into

const WORKGROUP_SIZE: u32 = 10;
const DISPATCH_SIZE: (u32, u32) = (
    (PARTICLE_COUNT_X + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
    (PARTICLE_COUNT_Y + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
);

struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    window: &'a Window,
    render_pipeline: wgpu::RenderPipeline,
    compute_pipeline: wgpu::ComputePipeline,
    render_bind_group: wgpu::BindGroup,
    compute_bind_group: wgpu::BindGroup,
    frame_count: u32,
    frame_count_buffer: wgpu::Buffer,
    particle_positions: Vec<[f32; 2]>,
    particle_positions_buffer: wgpu::Buffer,
    particle_radii: Vec<f32>,
    particle_radii_buffer: wgpu::Buffer,
    particle_velocities: Vec<[f32; 2]>,
    particle_velocities_buffer: wgpu::Buffer,
    particle_lookup: Vec<i32>,
    particle_lookup_buffer: wgpu::Buffer,
    position_reading_buffer: wgpu::Buffer,
    velocity_reading_buffer: wgpu::Buffer,
}

impl<'a> State<'a> {
    fn pos_to_grid_index(&self, pos: [f32; 2]) -> i32 {
        let x = (pos[0] / SCREEN_SIZE.0 as f32 * GRID_SIZE.0 as f32).min(GRID_SIZE.0 as f32 - 1.0).max(0.0)
            as i32;
        let y = (pos[1] / SCREEN_SIZE.1 as f32 * GRID_SIZE.1 as f32).min(GRID_SIZE.1 as f32 - 1.0).max(0.0)
            as i32;

        x + y * GRID_SIZE.0
    }

    fn pos_to_grid(&self, pos: [f32; 2]) -> (i32, i32) {
        let x = (pos[0] / SCREEN_SIZE.0 as f32 * GRID_SIZE.0 as f32).min(GRID_SIZE.0 as f32 - 1.0).max(0.0)
            as i32;
        let y = (pos[1] / SCREEN_SIZE.1 as f32 * GRID_SIZE.1 as f32).min(GRID_SIZE.1 as f32 - 1.0).max(0.0)
            as i32;

        (x, y)
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
            for i in 0..self.particle_positions.len() {
                self.particle_positions[i] = [positions[i * 2], positions[i * 2 + 1]];
            }

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
            for i in 0..self.particle_velocities.len() {
                self.particle_velocities[i] = [velocities[i * 2], velocities[i * 2 + 1]];
            }

            drop(data);
            self.velocity_reading_buffer.unmap();
        } else {
            // Handle mapping error
            eprintln!("Error mapping buffer");
            // return Err(wgpu::SurfaceError::Lost); // Or handle the error appropriately
            return;
        }
    }

    async fn sort_particles(&mut self) {
        // Update the particle positions and velocities from the buffers
        self.update_position_from_buffer().await;
        self.update_velocities_from_buffer().await;

        // Move the particles
        self.move_particles();

        // Map all particles to their grid cell
        let mut index_map: Vec<Vec<Vec<i32>>> =
            vec![vec![vec![]; GRID_SIZE.1 as usize]; GRID_SIZE.0 as usize];
        for i in 0..self.particle_positions.len() {
            let grid = self.pos_to_grid(self.particle_positions[i]);
            index_map[grid.0 as usize][grid.1 as usize].push(i as i32);
        }

        // Create a new list of particles
        let mut new_positions: Vec<[f32; 2]> = vec![];
        let mut new_velocities: Vec<[f32; 2]> = vec![];
        let mut new_radii: Vec<f32> = vec![];
        let mut lookup_table = vec![-1; GRID_SIZE.0 as usize * GRID_SIZE.1 as usize];

        // Iterate over all grid cells
        for i in 0..GRID_SIZE.0 {
            for j in 0..GRID_SIZE.1 {
                let grid_index = i + j * GRID_SIZE.0;
                let mut index = -1;

                // Iterate over all particles in the grid cell
                for k in 0..index_map[i as usize][j as usize].len() {
                    let particle_index = index_map[i as usize][j as usize][k] as usize;
                    new_positions.push(self.particle_positions[particle_index]);
                    new_velocities.push(self.particle_velocities[particle_index]);
                    new_radii.push(self.particle_radii[particle_index]);
                    if index == -1 {
                        index = new_positions.len() as i32 - 1;
                    }
                }

                lookup_table[grid_index as usize] = index;
            }
        }

        self.particle_positions = new_positions;
        self.particle_velocities = new_velocities;
        self.particle_radii = new_radii;
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
        let mut compute_pipeline_builder = ComputePipelineBuilder::new();
        compute_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main");
        compute_pipeline_builder.set_bind_group_layout(
            bind_group_layout_generator::get_bind_group_layout(&device, true),
        );
        let compute_pipeline = compute_pipeline_builder.build_pipeline(&device);

        // Create temporary bind groups
        let temp_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Temporary Render Bind Group"),
            layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[],
                label: Some("Temporary Render Bind Group Layout"),
            }),
            entries: &[],
        });

        let temp_compute_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Temporary Compute Bind Group"),
            layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[],
                label: Some("Temporary Compute Bind Group Layout"),
            }),
            entries: &[],
        });

        // Create particle data
        let mut particle_positions = vec![];
        let mut particle_velocities = vec![];
        let mut particle_radii = vec![];
        for i in 0..PARTICLE_COUNT_X {
            for j in 0..PARTICLE_COUNT_Y {
                let x = SCREEN_SIZE.0 as f32 / (PARTICLE_COUNT_X + 1) as f32 * i as f32 + OFFSET.0;
                let y = SCREEN_SIZE.1 as f32 / (PARTICLE_COUNT_Y + 1) as f32 * j as f32 + OFFSET.1;

                particle_positions.push([x, y]);
                particle_velocities.push([x / SCREEN_SIZE.0 as f32 * 2.0 - 1.0, y / SCREEN_SIZE.1 as f32 * 2.0 - 1.0]);
                particle_radii.push(1.0);
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
        let particle_lookup_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Particle Lookup Buffer Data"),
            contents: bytemuck::cast_slice(&particle_lookup),
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
            &particle_lookup_buffer,
            0,
            bytemuck::cast_slice(&particle_lookup),
        );

        // Buffer for the frame count
        let frame_count_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Frame Count Buffer"),
            contents: bytemuck::cast_slice(&[0]),
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
            compute_pipeline,
            render_bind_group: temp_render_bind_group,
            compute_bind_group: temp_compute_render_bind_group,
            frame_count: 0,
            frame_count_buffer,
            particle_positions,
            particle_positions_buffer,
            particle_radii,
            particle_radii_buffer,
            particle_velocities,
            particle_velocities_buffer,
            particle_lookup,
            particle_lookup_buffer,
            position_reading_buffer,
            velocity_reading_buffer,
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
    
    fn move_particles(&mut self) {
        for index in 0..self.particle_positions.len() {
            // println!("{:?}", self.particle_velocities[index]);
            if self.particle_positions[index][0] < 0.0 {
                self.particle_positions[index][0] = 0.0;
                self.particle_velocities[index][0] = -self.particle_velocities[index][0];
            }
            if self.particle_positions[index][0] > SCREEN_SIZE.0 as f32 {
                self.particle_positions[index][0] = SCREEN_SIZE.0 as f32;
                self.particle_velocities[index][0] = -self.particle_velocities[index][0];
            }
        
            if self.particle_positions[index][1] < 0.0 {
                self.particle_positions[index][1] = 0.0;
                self.particle_velocities[index][1] = -self.particle_velocities[index][1];
            }
            if self.particle_positions[index][1] > SCREEN_SIZE.1 as f32 {
                self.particle_positions[index][1] = SCREEN_SIZE.1 as f32;
                self.particle_velocities[index][1] = -self.particle_velocities[index][1];
            }
            self.particle_positions[index][0] += self.particle_velocities[index][0];   
            self.particle_positions[index][1] += self.particle_velocities[index][1]; 
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let start_time = std::time::Instant::now();

        pollster::block_on(self.sort_particles());

        // Update the frame count buffer before rendering
        self.queue.write_buffer(
            &self.frame_count_buffer,
            0,
            bytemuck::cast_slice(&[self.frame_count]),
        );

        // Dispatch the compute shader
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
            compute_pass.set_pipeline(&self.compute_pipeline); // Assuming you have a compute pipeline
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.dispatch_workgroups(DISPATCH_SIZE.0, DISPATCH_SIZE.1, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

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
            render_pass.set_bind_group(0, &self.render_bind_group, &[]); // Access using self
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
        bind_group_layout_generator::get_bind_group_layout(&state.device, false);
    state.render_bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Sphere Bind Group"),
        layout: &render_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: state.frame_count_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: state.particle_positions_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: state.particle_radii_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: state.particle_velocities_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: state.particle_lookup_buffer.as_entire_binding(),
            },
        ],
    });

    let compute_bind_group_layout =
        bind_group_layout_generator::get_bind_group_layout(&state.device, true);
    state.compute_bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Sphere Bind Group"),
        layout: &compute_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: state.frame_count_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: state.particle_positions_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: state.particle_radii_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: state.particle_velocities_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: state.particle_lookup_buffer.as_entire_binding(),
            },
        ],
    });

    // Pass bind group layout to pipeline builder
    let mut render_pipeline_builder = PipelineBuilder::new();
    render_pipeline_builder.set_shader_module("shaders/shader.wgsl", "vs_main", "fs_main");
    render_pipeline_builder.set_pixel_format(state.config.format);
    render_pipeline_builder.set_bind_group_layout(render_bind_group_layout);
    state.render_pipeline = render_pipeline_builder.build_pipeline(&state.device);

    // Pass bind group layout to compute pipeline builder
    let mut compute_pipeline_builder = ComputePipelineBuilder::new();
    compute_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main");
    compute_pipeline_builder.set_bind_group_layout(compute_bind_group_layout);
    state.compute_pipeline = compute_pipeline_builder.build_pipeline(&state.device);

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

                WindowEvent::RedrawRequested => match state.render() {
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

fn main() {
    pollster::block_on(run());
}
