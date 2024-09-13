use renderer_backend::pipeline_builder::PipelineBuilder;
mod renderer_backend;
use wgpu::{
    core::device::global, util::{BufferInitDescriptor, DeviceExt}, BufferUsages
};
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::EventLoopBuilder,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};
use cgmath::prelude::*;

const SCREEN_SIZE: (u32, u32) = (1200, 600);
const TIME_BETWEEN_FRAMES: u64 = 10;
const PARTICLE_COUNT_X: u32 = 10;
const PARTICLE_COUNT_Y: u32 = 10;

struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    window: &'a Window,
    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    frame_count: u32,
    frame_count_buffer: wgpu::Buffer,
    particle_positions_buffer: wgpu::Buffer,
    particle_radii_buffer: wgpu::Buffer,
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

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("Particle Bind Group Layout"),
        });

        // Pass bind group layout to pipeline builder
        let mut pipeline_builder = PipelineBuilder::new();
        pipeline_builder.set_shader_module("shaders/shader.wgsl", "vs_main", "fs_main");
        pipeline_builder.set_pixel_format(config.format);
        pipeline_builder.set_bind_group_layout(bind_group_layout);
        let render_pipeline = pipeline_builder.build_pipeline(&device);

        // Create a temporary bind group
        let temp_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Temporary Bind Group"),
            layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[],
                label: Some("Temporary Bind Group Layout"),
            }),
            entries: &[],
        });

        // Create particle data
        let mut particle_positions = vec![];
        let mut particle_radii = vec![];
        for i in 0..PARTICLE_COUNT_X {
            for j in 0..PARTICLE_COUNT_Y {
                let x = SCREEN_SIZE.0 as f32 / (PARTICLE_COUNT_X + 1) as f32 * i as f32;
                let y = SCREEN_SIZE.1 as f32 / (PARTICLE_COUNT_Y + 1) as f32 * j as f32;
                particle_positions.push([x, y]);
                particle_radii.push(10.0);
            }
        }

        // Buffer for particles
        let particle_positions_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Particle Positions Buffer Data"),
            contents: bytemuck::cast_slice(&particle_positions),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        let particle_radii_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Particle Radii Buffer Data"),
            contents: bytemuck::cast_slice(&particle_radii),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        // Write data to buffers
        queue.write_buffer(&particle_positions_buffer, 0, bytemuck::cast_slice(&particle_positions));
        queue.write_buffer(&particle_radii_buffer, 0, bytemuck::cast_slice(&particle_radii));        

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
            bind_group: temp_bind_group,
            frame_count: 0,
            frame_count_buffer,
            particle_positions_buffer,
            particle_radii_buffer,
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

        // Update the frame count buffer before rendering
        self.queue.write_buffer(
            &self.frame_count_buffer,
            0,
            bytemuck::cast_slice(&[self.frame_count]),
        );

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
            render_pass.set_bind_group(0, &self.bind_group, &[]); // Access using self
            render_pass.draw(0..3, 0..1); // Draw the first triangle
            render_pass.draw(3..6, 0..1); // Draw the second triangle
        }

        self.queue.submit(std::iter::once(command_encoder.finish()));

        drawable.present();

        self.frame_count += 1;

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

    // Create bind group layout
    let bind_group_layout =
        state
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("Particle Bind Group Layout"),
            });
    state.bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Particle Bind Group"),
        layout: &bind_group_layout,
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
        ],
    });

    // Pass bind group layout to pipeline builder
    let mut pipeline_builder = PipelineBuilder::new();
    pipeline_builder.set_shader_module("shaders/shader.wgsl", "vs_main", "fs_main");
    pipeline_builder.set_pixel_format(state.config.format);
    pipeline_builder.set_bind_group_layout(bind_group_layout);
    state.render_pipeline = pipeline_builder.build_pipeline(&state.device);

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
