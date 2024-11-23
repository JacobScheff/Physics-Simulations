use bytemuck::{Pod, Zeroable};
use core::f32;
use renderer_backend::{
    bind_group_layout_generator, compute_pipeline_builder::ComputePipelineBuilder,
    pipeline_builder::PipelineBuilder,
};
mod renderer_backend;
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
const SIM_SIZE: (i32, i32) = (500, 250); // How many grid cells to divide the screen into
const PADDING: f32 = 50.0; // The padding around the screen

const WORKGROUP_SIZE: u32 = 16;
const DISPATCH_SIZE: (u32, u32) = (
    ((SIM_SIZE.0 as u32) + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
    ((SIM_SIZE.1 as u32) + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
);

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Particle {
    velocity: [f32; 2], // 8 bytes
    density: f32, // 4 bytes
    _padding: f32, // 4 bytes
}

impl Particle {    
    fn new(velocity: [f32; 2], density: f32) -> Self {
        Self {
            velocity: velocity,
            density: density,
            _padding: 0.0,
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
    particle_buffer_read: wgpu::Buffer,
    particle_buffer_write: wgpu::Buffer,
    frame_count: u32,
}

impl<'a> State<'a> {
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
        render_pipeline_builder
            .set_bind_group_layout(bind_group_layout_generator::get_bind_group_layout(&device));
        let render_pipeline = render_pipeline_builder.build_pipeline(&device);

        // Pass bind group layout to compute density pipeline builder
        let mut compute_density_pipeline_builder = ComputePipelineBuilder::new();
        compute_density_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_density");
        compute_density_pipeline_builder
            .set_bind_group_layout(bind_group_layout_generator::get_bind_group_layout(&device));
        let compute_density_pipeline = compute_density_pipeline_builder.build_pipeline(&device);

        // Pass bind group layout to compute move pipeline builder
        let mut compute_move_pipeline_builder = ComputePipelineBuilder::new();
        compute_move_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_move");
        compute_move_pipeline_builder
            .set_bind_group_layout(bind_group_layout_generator::get_bind_group_layout(&device));
        let compute_move_pipeline = compute_move_pipeline_builder.build_pipeline(&device);

        // Pass bind group layout to compute forces pipeline builder
        let mut compute_forces_pipeline_builder = ComputePipelineBuilder::new();
        compute_forces_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_forces");
        compute_forces_pipeline_builder
            .set_bind_group_layout(bind_group_layout_generator::get_bind_group_layout(&device));
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

        let mut particle_data = vec![
            vec![
                Particle::new([0.0, 0.0], 0.0);
                SIM_SIZE.0 as usize
            ];
            SIM_SIZE.1 as usize
        ];

        particle_data[3][2] = Particle::new([5.0, 9.0], 22.0);
        
        let particle_data_flat: Vec<Particle> = particle_data.into_iter().flatten().collect();
        let particle_data_u8: Vec<u8> = bytemuck::cast_slice(&particle_data_flat).to_vec();

        // Store particle data in a texture
        let particle_buffer_read = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Particle Data Buffer Read"),
            contents: &particle_data_u8,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let particle_buffer_write = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Particle Data Buffer Write"),
            contents: &particle_data_u8,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
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
            particle_buffer_read,
            particle_buffer_write,
            frame_count: 0,
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

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let start_time = std::time::Instant::now();

        // Compute the density of the particles
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Desisty Compute Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Density Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_density_pipeline);
            compute_pass.set_bind_group(0, &self.compute_densities_bind_group, &[]);
            compute_pass.dispatch_workgroups(DISPATCH_SIZE.0, DISPATCH_SIZE.1, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        // Copy particle data from the write buffer to the read buffer
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Copy Encoder"),
            });

        {
            encoder.copy_buffer_to_buffer(
                &self.particle_buffer_write,
                0,
                &self.particle_buffer_read,
                0,
                (SIM_SIZE.0 * SIM_SIZE.1) as u64 * std::mem::size_of::<Particle>() as u64,
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));

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
    state.compute_densities_bind_group =
        create_bind_group(&mut state, &compute_density_bind_group_layout);

    let compute_forces_bind_group_layout =
        bind_group_layout_generator::get_bind_group_layout(&state.device);
    state.compute_forces_bind_group =
        create_bind_group(&mut state, &compute_forces_bind_group_layout);

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

                WindowEvent::RedrawRequested => match { state.render() } {
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
                resource: state.particle_buffer_read.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: state.particle_buffer_write.as_entire_binding(),
            },
        ],
    });

    bind_group
}

fn main() {
    pollster::block_on(run());
}
