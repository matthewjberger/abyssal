pub struct Gpu {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
}

pub async fn create_gpu_async(
    window: impl Into<wgpu::SurfaceTarget<'static>>,
    width: u32,
    height: u32,
) -> Gpu {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all),
        ..Default::default()
    });

    let surface = instance.create_surface(window).unwrap();

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to request adapter!");
    let (device, queue) = {
        log::info!("WGPU Adapter Features: {:#?}", adapter.features());
        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("WGPU Device"),
                    memory_hints: wgpu::MemoryHints::default(),
                    required_features: adapter.features(),
                    required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .expect("Failed to request a device!")
    };

    let surface_capabilities = surface.get_capabilities(&adapter);

    log::info!("Surface capabilities for format {surface_capabilities:#?}:");

    let surface_format = surface_capabilities
        .formats
        .iter()
        .copied()
        .find(|f| !f.is_srgb()) // egui wants a non-srgb surface texture
        .unwrap_or(surface_capabilities.formats[0]);

    let surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
        format: surface_format,
        width,
        height,
        present_mode: surface_capabilities.present_modes[0],
        alpha_mode: surface_capabilities.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 1,
    };

    surface.configure(&device, &surface_config);

    Gpu {
        surface,
        device,
        queue,
        surface_config,
    }
}
