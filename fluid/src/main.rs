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
// const PADDING: f32 = 50.0; // The padding around the screen

const WORKGROUP_SIZE: u32 = 16;
const DISPATCH_SIZE: (u32, u32) = (
    ((SIM_SIZE.0 as u32) + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
    ((SIM_SIZE.1 as u32) + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
);

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Cell {
    density: f32,
    divergence: f32,
    pressure: f32,
    s: i32,
}

impl Cell {    
    fn new(density: f32) -> Self {
        Self {
            density: density,
            divergence: 0.0,
            pressure: 0.0,
            s: 1,
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
    compute_gravity_pipeline: wgpu::ComputePipeline,
    compute_divergence_pipeline: wgpu::ComputePipeline,
    compute_velocity_pipeline: wgpu::ComputePipeline,
    compute_advection_pipeline: wgpu::ComputePipeline,
    render_bind_group: wgpu::BindGroup,
    compute_gravity_bind_group: wgpu::BindGroup,
    compute_divergence_bind_group: wgpu::BindGroup,
    compute_velocity_bind_group: wgpu::BindGroup,
    compute_advection_bind_group: wgpu::BindGroup,
    cell_buffer: wgpu::Buffer,
    horizontal_velocity_buffer: wgpu::Buffer,
    vertical_velocity_buffer: wgpu::Buffer,
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

        // Pass bind group layout to compute gravity pipeline builder
        let mut compute_gravity_pipeline_builder = ComputePipelineBuilder::new();
        compute_gravity_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_gravity");
        compute_gravity_pipeline_builder
            .set_bind_group_layout(bind_group_layout_generator::get_bind_group_layout(&device));
        let compute_gravity_pipeline = compute_gravity_pipeline_builder.build_pipeline(&device);

        // Pass bind group layout to compute velocity pipeline builder
        let mut compute_velocity_pipeline_builder = ComputePipelineBuilder::new();
        compute_velocity_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_velocity");
        compute_velocity_pipeline_builder
            .set_bind_group_layout(bind_group_layout_generator::get_bind_group_layout(&device));
        let compute_velocity_pipeline = compute_velocity_pipeline_builder.build_pipeline(&device);

        // Pass bind group layout to compute divergence pipeline builder
        let mut compute_divergence_pipeline_builder = ComputePipelineBuilder::new();
        compute_divergence_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_divergence");
        compute_divergence_pipeline_builder
            .set_bind_group_layout(bind_group_layout_generator::get_bind_group_layout(&device));
        let compute_divergence_pipeline = compute_divergence_pipeline_builder.build_pipeline(&device);

        // Pass bind group layout to compute advection pipeline builder
        let mut compute_advection_pipeline_builder = ComputePipelineBuilder::new();
        compute_advection_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_advection");
        compute_advection_pipeline_builder
            .set_bind_group_layout(bind_group_layout_generator::get_bind_group_layout(&device));
        let compute_advection_pipeline = compute_advection_pipeline_builder.build_pipeline(&device);

        // Create temporary bind groups
        let temp_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Temporary Render Bind Group"),
            layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[],
                label: Some("Temporary Render Bind Group Layout"),
            }),
            entries: &[],
        });

        let temp_compute_gravity_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Temporary Compute gravity Bind Group"),
                layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[],
                    label: Some("Temporary Compute gravity Bind Group Layout"),
                }),
                entries: &[],
            });

        let temp_compute_velocity_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Temporary Compute velocity Bind Group"),
            layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[],
                label: Some("Temporary Compute velocity Bind Group Layout"),
            }),
            entries: &[],
        });

        let temp_compute_divergence_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Temporary Compute divergence Bind Group"),
            layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[],
                label: Some("Temporary Compute divergence Bind Group Layout"),
            }),
            entries: &[],
        });

        let temp_compute_advection_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Temporary Compute advection Bind Group"),
            layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[],
                label: Some("Temporary Compute advection Bind Group Layout"),
            }),
            entries: &[],
        });

        let mut cell_data = vec![
            vec![
                Cell::new(0.5);
                SIM_SIZE.0 as usize
            ];
            SIM_SIZE.1 as usize
        ];

        // Set s to 0 for border cells
        for i in 0..SIM_SIZE.0 as usize {
            cell_data[0][i].s = 0;
            cell_data[SIM_SIZE.1 as usize - 1][i].s = 0;
        }
        for i in 0..SIM_SIZE.1 as usize {
            cell_data[i][0].s = 0;
            cell_data[i][SIM_SIZE.0 as usize - 1].s = 0;
        }
        
        let cell_data_flat: Vec<Cell> = cell_data.into_iter().flatten().collect();
        let cell_data_u8: Vec<u8> = bytemuck::cast_slice(&cell_data_flat).to_vec();

        let horizontal_velocity_data = vec![
            vec![0.0; SIM_SIZE.0 as usize - 1];
            SIM_SIZE.1 as usize
        ];
        let horizontal_velocity_data_flat: Vec<f32> = horizontal_velocity_data.into_iter().flatten().collect();
        let horizontal_velocity_data_u8: Vec<u8> = bytemuck::cast_slice(&horizontal_velocity_data_flat).to_vec();

        let vertical_velocity_data = vec![
            vec![0.0; SIM_SIZE.0 as usize];
            SIM_SIZE.1 as usize - 1
        ];

        let vertical_velocity_data_flat: Vec<f32> = vertical_velocity_data.into_iter().flatten().collect();
        let vertical_velocity_data_u8: Vec<u8> = bytemuck::cast_slice(&vertical_velocity_data_flat).to_vec();

        // Buffers
        let cell_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("cell Data Buffer"),
            contents: &cell_data_u8,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        
        let horizontal_velocity_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Horizontal Velocity Buffer"),
            contents: bytemuck::cast_slice(&horizontal_velocity_data_u8),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let vertical_velocity_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertical Velocity Buffer"),
            contents: bytemuck::cast_slice(&vertical_velocity_data_u8),
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
            compute_gravity_pipeline,
            compute_divergence_pipeline,
            compute_velocity_pipeline,
            compute_advection_pipeline,
            render_bind_group: temp_render_bind_group,
            compute_gravity_bind_group: temp_compute_gravity_bind_group,
            compute_divergence_bind_group: temp_compute_divergence_bind_group,
            compute_velocity_bind_group: temp_compute_velocity_bind_group,
            compute_advection_bind_group: temp_compute_advection_bind_group,
            cell_buffer,
            horizontal_velocity_buffer,
            vertical_velocity_buffer,
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

        // Compute the gravity of the cells
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Desisty Compute Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("gravity Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_gravity_pipeline);
            compute_pass.set_bind_group(0, &self.compute_gravity_bind_group, &[]);
            compute_pass.dispatch_workgroups(DISPATCH_SIZE.0, DISPATCH_SIZE.1, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        // Compute the divergence of the cells
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Desisty Compute Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("divergence Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_divergence_pipeline);
            compute_pass.set_bind_group(0, &self.compute_divergence_bind_group, &[]);
            compute_pass.dispatch_workgroups(DISPATCH_SIZE.0, DISPATCH_SIZE.1, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        // Compute the velocity of the cells
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Desisty Compute Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("velocity Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_velocity_pipeline);
            compute_pass.set_bind_group(0, &self.compute_velocity_bind_group, &[]);
            compute_pass.dispatch_workgroups(DISPATCH_SIZE.0, DISPATCH_SIZE.1, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        // Dispatch the advection compute shader
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Desisty Compute Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("advection Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_advection_pipeline);
            compute_pass.set_bind_group(0, &self.compute_advection_bind_group, &[]);
            compute_pass.dispatch_workgroups(DISPATCH_SIZE.0, DISPATCH_SIZE.1, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        // Render the cells
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

    let compute_gravity_bind_group_layout =
        bind_group_layout_generator::get_bind_group_layout(&state.device);
    state.compute_gravity_bind_group =
        create_bind_group(&mut state, &compute_gravity_bind_group_layout);

    let compute_divergence_bind_group_layout =
        bind_group_layout_generator::get_bind_group_layout(&state.device);
    state.compute_divergence_bind_group =
        create_bind_group(&mut state, &compute_divergence_bind_group_layout);

    let compute_velocity_bind_group_layout =
        bind_group_layout_generator::get_bind_group_layout(&state.device);
    state.compute_velocity_bind_group = create_bind_group(&mut state, &compute_velocity_bind_group_layout);

    let compute_advection_bind_group_layout =
        bind_group_layout_generator::get_bind_group_layout(&state.device);
    state.compute_advection_bind_group = create_bind_group(&mut state, &compute_advection_bind_group_layout);

    // Pass bind group layout to pipeline builder
    let mut render_pipeline_builder = PipelineBuilder::new();
    render_pipeline_builder.set_shader_module("shaders/shader.wgsl", "vs_main", "fs_main");
    render_pipeline_builder.set_pixel_format(state.config.format);
    render_pipeline_builder.set_bind_group_layout(render_bind_group_layout);
    state.render_pipeline = render_pipeline_builder.build_pipeline(&state.device);

    // Pass bind group layout to compute pipeline builder
    let mut compute_gravity_pipeline_builder = ComputePipelineBuilder::new();
    compute_gravity_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_gravity");
    compute_gravity_pipeline_builder.set_bind_group_layout(compute_gravity_bind_group_layout);
    state.compute_gravity_pipeline = compute_gravity_pipeline_builder.build_pipeline(&state.device);

    // Pass bind group layout to compute divergence pipeline builder
    let mut compute_divergence_pipeline_builder = ComputePipelineBuilder::new();
    compute_divergence_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_divergence");
    compute_divergence_pipeline_builder.set_bind_group_layout(compute_divergence_bind_group_layout);
    state.compute_divergence_pipeline = compute_divergence_pipeline_builder.build_pipeline(&state.device);

    // Pass bind group layout to compute velocity pipeline builder
    let mut compute_velocity_pipeline_builder = ComputePipelineBuilder::new();
    compute_velocity_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_velocity");
    compute_velocity_pipeline_builder.set_bind_group_layout(compute_velocity_bind_group_layout);
    state.compute_velocity_pipeline = compute_velocity_pipeline_builder.build_pipeline(&state.device);

    // Pass bind group layout to compute advection pipeline builder
    let mut compute_advection_pipeline_builder = ComputePipelineBuilder::new();
    compute_advection_pipeline_builder.set_shader_module("shaders/shader.wgsl", "main_advection");
    compute_advection_pipeline_builder.set_bind_group_layout(compute_advection_bind_group_layout);
    state.compute_advection_pipeline = compute_advection_pipeline_builder.build_pipeline(&state.device);

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
        label: Some("cell Data Bind Group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: state.cell_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: state.horizontal_velocity_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: state.vertical_velocity_buffer.as_entire_binding(),
            },
        ],
    });

    bind_group
}

fn main() {
    pollster::block_on(run());
}
