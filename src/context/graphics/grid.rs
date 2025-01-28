pub struct Grid {
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub pipeline: wgpu::RenderPipeline,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GridUniform {
    pub view_proj: nalgebra_glm::Mat4,
    pub camera_world_pos: nalgebra_glm::Vec3,
    pub grid_size: f32,
    pub grid_min_pixels: f32,
    pub grid_cell_size: f32,
    pub _padding: [f32; 2],
}

pub fn create_grid(
    device: &wgpu::Device,
    color_format: wgpu::TextureFormat,
    depth_format: wgpu::TextureFormat,
) -> Grid {
    use wgpu::util::DeviceExt;

    let grid_uniform = GridUniform {
        view_proj: nalgebra_glm::Mat4::identity(),
        camera_world_pos: nalgebra_glm::Vec3::zeros(),
        grid_size: 100.0,
        grid_min_pixels: 2.0,
        grid_cell_size: 0.025,
        _padding: [0.0; 2],
    };

    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Grid Uniform Buffer"),
        contents: bytemuck::cast_slice(&[grid_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
        label: Some("Grid Layout"),
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
        label: Some("Grid Bind Group"),
    });

    let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/grid.wgsl"));

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Grid Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Grid Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
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

    Grid {
        uniform_buffer,
        bind_group,
        pipeline,
    }
}

pub fn render_grid(render_pass: &mut wgpu::RenderPass<'_>, grid: &Grid) {
    render_pass.set_pipeline(&grid.pipeline);
    render_pass.set_bind_group(0, &grid.bind_group, &[]);
    render_pass.draw(0..6, 0..1);
}

pub fn update_grid(
    matrices: &crate::context::camera::CameraMatrices,
    queue: &wgpu::Queue,
    grid: &Grid,
) {
    let uniform = GridUniform {
        view_proj: matrices.projection * matrices.view,
        camera_world_pos: matrices.camera_position.xyz(),
        grid_size: 100.0,
        grid_min_pixels: 2.0,
        grid_cell_size: 0.025,
        _padding: [0.0; 2],
    };
    queue.write_buffer(&grid.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));
}
