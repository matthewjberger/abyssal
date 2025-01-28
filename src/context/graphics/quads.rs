use wgpu::util::DeviceExt as _;

pub struct Quads {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub pipeline: wgpu::RenderPipeline,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadVertex {
    pub position: nalgebra_glm::Vec3,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadInstance {
    pub model_matrix_0: nalgebra_glm::Vec4,
    pub model_matrix_1: nalgebra_glm::Vec4,
    pub model_matrix_2: nalgebra_glm::Vec4,
    pub model_matrix_3: nalgebra_glm::Vec4,
    pub color: nalgebra_glm::Vec4,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadUniform {
    pub view_proj: nalgebra_glm::Mat4,
}

pub fn create_quad_renderer(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
    depth_format: wgpu::TextureFormat,
) -> Quads {
    // Create a unit quad centered at origin in XY plane
    let vertices = [
        QuadVertex {
            position: nalgebra_glm::vec3(-0.5, -0.5, 0.0),
        },
        QuadVertex {
            position: nalgebra_glm::vec3(0.5, -0.5, 0.0),
        },
        QuadVertex {
            position: nalgebra_glm::vec3(0.5, 0.5, 0.0),
        },
        QuadVertex {
            position: nalgebra_glm::vec3(-0.5, 0.5, 0.0),
        },
    ];

    let indices: &[u16] = &[0, 1, 2, 2, 3, 0];

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Quad Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Quad Index Buffer"),
        contents: bytemuck::cast_slice(indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    let initial_instance_capacity = 1024;
    let instance_buffer_size = std::mem::size_of::<QuadInstance>() * initial_instance_capacity;

    let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Quad Instance Buffer"),
        size: instance_buffer_size as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Quad Uniform Buffer"),
        size: std::mem::size_of::<QuadUniform>() as u64,
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
        label: Some("Quad Bind Group Layout"),
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
        label: Some("Quad Bind Group"),
    });

    let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/quads.wgsl"));

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Quad Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Quad Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[
                // Vertex buffer
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                },
                // Instance buffer
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<QuadInstance>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        1 => Float32x4,
                        2 => Float32x4,
                        3 => Float32x4,
                        4 => Float32x4,
                        5 => Float32x4
                    ],
                },
            ],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent::OVER,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
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
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    Quads {
        vertex_buffer,
        index_buffer,
        instance_buffer,
        uniform_buffer,
        bind_group,
        pipeline,
    }
}

pub fn update_quads_uniform(
    matrices: &crate::context::camera::CameraMatrices,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    quads: &mut Quads,
    instances: Vec<QuadInstance>,
) {
    let uniform = QuadUniform {
        view_proj: matrices.projection * matrices.view,
    };

    queue.write_buffer(&quads.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));

    // Create the data that will be sent to the GPU
    let gpu_data = if instances.is_empty() {
        vec![QuadInstance {
            model_matrix_0: nalgebra_glm::vec4(0.0, 0.0, 0.0, 0.0),
            model_matrix_1: nalgebra_glm::vec4(0.0, 0.0, 0.0, 0.0),
            model_matrix_2: nalgebra_glm::vec4(0.0, 0.0, 0.0, 0.0),
            model_matrix_3: nalgebra_glm::vec4(0.0, 0.0, 0.0, 0.0),
            color: nalgebra_glm::vec4(0.0, 0.0, 0.0, 0.0),
        }]
    } else {
        instances
    };

    // Always recreate the buffer with the exact size needed
    quads.instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Quad Instance Buffer"),
        contents: bytemuck::cast_slice(&gpu_data),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });
}

pub fn render_quads(render_pass: &mut wgpu::RenderPass<'_>, quads: &Quads) {
    let instance_size = std::mem::size_of::<QuadInstance>();
    let instance_count = (quads.instance_buffer.size() as usize / instance_size) as u32;
    if instance_count > 0 {
        render_pass.set_pipeline(&quads.pipeline);
        render_pass.set_bind_group(0, &quads.bind_group, &[]);
        render_pass.set_vertex_buffer(0, quads.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, quads.instance_buffer.slice(..));
        render_pass.set_index_buffer(quads.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..6, 0, 0..instance_count);
    }
}
