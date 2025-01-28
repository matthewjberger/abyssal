pub struct Sky {
    pub uniform_buffer: wgpu::Buffer,
    pub texture: wgpu::Texture,
    pub texture_view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
    pub pipeline: wgpu::RenderPipeline,
}

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SkyUniform {
    proj: nalgebra_glm::Mat4,
    proj_inv: nalgebra_glm::Mat4,
    view: nalgebra_glm::Mat4,
    cam_pos: nalgebra_glm::Vec4,
}

pub fn create_sky(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    surface_format: wgpu::TextureFormat,
    depth_format: wgpu::TextureFormat,
) -> Sky {
    let sky_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Sky Uniform Buffer"),
        size: std::mem::size_of::<SkyUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let sky_texture = load_sky_texture(device, queue);
    let sky_texture_view = sky_texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::Cube),
        ..Default::default()
    });

    let sky_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let sky_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Sky Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::Cube,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let sky_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &sky_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: sky_uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&sky_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sky_sampler),
            },
        ],
        label: Some("Sky Bind Group"),
    });

    let sky_shader = device.create_shader_module(wgpu::include_wgsl!("shaders/sky.wgsl"));

    let sky_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Sky Pipeline Layout"),
        bind_group_layouts: &[&sky_bind_group_layout],
        push_constant_ranges: &[],
    });

    let sky_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Sky Pipeline"),
        layout: Some(&sky_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &sky_shader,
            entry_point: Some("vs_sky"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &sky_shader,
            entry_point: Some("fs_sky"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: depth_format,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });
    Sky {
        uniform_buffer: sky_uniform_buffer,
        texture: sky_texture,
        texture_view: sky_texture_view,
        sampler: sky_sampler,
        bind_group: sky_bind_group,
        pipeline: sky_pipeline,
    }
}

fn load_sky_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
    let hdr_data = include_bytes!("hdr/sky.hdr");
    let cursor = std::io::Cursor::new(hdr_data);
    let decoder =
        image::codecs::hdr::HdrDecoder::new(cursor).expect("Failed to create HDR decoder");
    let metadata = decoder.metadata();
    let decoded = decoder
        .read_image_hdr()
        .expect("Failed to decode HDR image");

    // Create source texture for equirectangular image
    let equirect_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Equirectangular Source Texture"),
        size: wgpu::Extent3d {
            width: metadata.width,
            height: metadata.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    // Upload HDR data
    let data: Vec<f32> = decoded
        .into_iter()
        .flat_map(|pixel| [pixel.0[0], pixel.0[1], pixel.0[2], 1.0])
        .collect();

    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &equirect_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(&data),
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(metadata.width * 16), // 4 x f32
            rows_per_image: Some(metadata.height),
        },
        wgpu::Extent3d {
            width: metadata.width,
            height: metadata.height,
            depth_or_array_layers: 1,
        },
    );

    // Create destination cubemap texture
    let cubemap = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Sky Cubemap Texture"),
        size: wgpu::Extent3d {
            width: 1024,
            height: 1024,
            depth_or_array_layers: 6,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    });

    // Create compute pipeline for cubemap generation
    let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/equirect_to_cube.wgsl"));

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Cubemap Generation Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rgba32Float,
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                },
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Cubemap Generation Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Cubemap Generation Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Cubemap Generation Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(
                    &equirect_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&device.create_sampler(
                    &wgpu::SamplerDescriptor {
                        label: Some("Equirect Sampler"),
                        address_mode_u: wgpu::AddressMode::ClampToEdge,
                        address_mode_v: wgpu::AddressMode::ClampToEdge,
                        address_mode_w: wgpu::AddressMode::ClampToEdge,
                        mag_filter: wgpu::FilterMode::Linear,
                        min_filter: wgpu::FilterMode::Linear,
                        mipmap_filter: wgpu::FilterMode::Linear,
                        ..Default::default()
                    },
                )),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(
                    &cubemap.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            },
        ],
    });

    // Execute compute shader
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Cubemap Generation Encoder"),
    });

    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Cubemap Generation Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        // Dispatch compute shader (64x64 workgroups for 1024x1024 faces, 6 faces)
        compute_pass.dispatch_workgroups(64, 64, 6);
    }

    queue.submit(Some(encoder.finish()));

    cubemap
}

pub fn update_sky(
    matrices: &crate::context::camera::CameraMatrices,
    queue: &wgpu::Queue,
    sky: &Sky,
) {
    let uniform = SkyUniform {
        proj: matrices.projection,
        proj_inv: nalgebra_glm::inverse(&matrices.projection),
        view: matrices.view,
        cam_pos: nalgebra_glm::vec4(
            matrices.camera_position.x,
            matrices.camera_position.y,
            matrices.camera_position.z,
            1.0,
        ),
    };
    queue.write_buffer(&sky.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));
}

pub fn render_sky(render_pass: &mut wgpu::RenderPass<'_>, sky: &Sky) {
    render_pass.set_pipeline(&sky.pipeline);
    render_pass.set_bind_group(0, &sky.bind_group, &[]);
    render_pass.draw(0..3, 0..1);
}
