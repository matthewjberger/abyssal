pub struct Lines {
    pub vertex_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub pipeline: wgpu::RenderPipeline,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineVertex {
    pub position: nalgebra_glm::Vec3,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineInstance {
    pub start: nalgebra_glm::Vec4,
    pub end: nalgebra_glm::Vec4,
    pub color: nalgebra_glm::Vec4,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineUniform {
    pub view_proj: nalgebra_glm::Mat4,
}

pub fn create_line_renderer(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    depth_format: wgpu::TextureFormat,
) -> Lines {
    let vertices = [
        LineVertex {
            position: nalgebra_glm::vec3(0.0, 0.0, 0.0),
        },
        LineVertex {
            position: nalgebra_glm::vec3(1.0, 0.0, 0.0),
        },
    ];

    let vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
        device,
        &wgpu::util::BufferInitDescriptor {
            label: Some("Line Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        },
    );

    let initial_instance_capacity = 1024;
    let instance_buffer_size = std::mem::size_of::<LineInstance>() * initial_instance_capacity;

    let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Line Instance Buffer"),
        size: instance_buffer_size as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Line Uniform Buffer"),
        size: std::mem::size_of::<LineUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
        label: Some("Line Bind Group Layout"),
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
        label: Some("Line Bind Group"),
    });

    let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/lines.wgsl"));

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Line Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Line Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<LineVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                },
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<LineInstance>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        1 => Float32x4,
                        2 => Float32x4,
                        3 => Float32x4
                    ],
                },
            ],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: depth_format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState {
                constant: -1, // Small negative bias to avoid z-fighting
                slope_scale: 0.0,
                clamp: 0.0,
            },
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    Lines {
        vertex_buffer,
        instance_buffer,
        uniform_buffer,
        bind_group,
        pipeline,
    }
}

pub fn update_lines_uniform(
    matrices: &crate::context::camera::CameraMatrices,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    lines: &mut Lines,
    instances: Vec<LineInstance>,
) {
    // Create the data that will be sent to the GPU
    let gpu_data = if instances.is_empty() {
        vec![LineInstance {
            start: nalgebra_glm::vec4(0.0, 0.0, 0.0, 0.0),
            end: nalgebra_glm::vec4(0.0, 0.0, 0.0, 0.0),
            color: nalgebra_glm::vec4(0.0, 0.0, 0.0, 0.0),
        }]
    } else {
        instances
    };

    let uniform = LineUniform {
        view_proj: matrices.projection * matrices.view,
    };

    queue.write_buffer(&lines.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));

    // Always recreate the buffer with the exact size needed
    lines.instance_buffer = wgpu::util::DeviceExt::create_buffer_init(
        device,
        &wgpu::util::BufferInitDescriptor {
            label: Some("Debug Line Instance Buffer"),
            contents: bytemuck::cast_slice(&gpu_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        },
    );
}

pub fn render_lines(render_pass: &mut wgpu::RenderPass<'_>, lines: &Lines) {
    let instance_size = std::mem::size_of::<LineInstance>();
    let debug_line_instance_count = (lines.instance_buffer.size() as usize / instance_size) as u32;
    if debug_line_instance_count > 0 {
        render_pass.set_pipeline(&lines.pipeline);
        render_pass.set_bind_group(0, &lines.bind_group, &[]);
        render_pass.set_vertex_buffer(0, lines.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, lines.instance_buffer.slice(..));
        render_pass.draw(0..2, 0..debug_line_instance_count);
    }
}
