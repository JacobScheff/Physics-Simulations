use std::env::current_dir;
use std::fs;

use super::bind_group_layout_generator;

pub struct ComputePipelineBuilder {
    shader_filename: String,
    entry_point: String,
    bind_group_layout: Option<wgpu::BindGroupLayout>
}

impl ComputePipelineBuilder {

    pub fn new() -> Self {
        ComputePipelineBuilder {
            shader_filename: "dummy".to_string(),
            entry_point: "dummy".to_string(),
            bind_group_layout: None
        }
    }

    pub fn set_bind_group_layout(&mut self, bind_group_layout: wgpu::BindGroupLayout) {
        self.bind_group_layout = Some(bind_group_layout);
    }

    pub fn set_shader_module(&mut self, shader_filename: &str, entry_point: &str) {
        self.shader_filename = shader_filename.to_string();
        self.entry_point = entry_point.to_string();
    }

    pub fn build_pipeline(&self, device: &wgpu::Device) -> wgpu::ComputePipeline {
        
        let mut filepath = current_dir().unwrap();
        filepath.push("src/");
        filepath.push(self.shader_filename.as_str());
        let filepath = filepath.into_os_string().into_string().unwrap();
        let source_code = fs::read_to_string(filepath).expect("Can't read the shader source file.");

        let shader_module_descriptor = wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(source_code.into()),
        };
        let shader_module = device.create_shader_module(shader_module_descriptor);

        // Create the bind group layout *before* building the pipeline
        let bind_group_layout = bind_group_layout_generator::get_bind_group_layout(device);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline_descriptor = wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: self.entry_point.as_str(),
        };
        device.create_compute_pipeline(&compute_pipeline_descriptor)
    }
}