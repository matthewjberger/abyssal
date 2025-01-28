mod gpu;
mod grid;
mod lines;
mod quads;
mod sky;

use crate::context::{
    camera::{Camera, CameraMatrices},
    graphics::{lines::LineInstance, quads::QuadInstance},
    paint::{Lines, Quads},
    transform::GlobalTransform,
    tree::{is_descendant_of, Parent},
    ui::PaneKind,
    Context,
};

/// A resource for graphics state
#[derive(Default)]
pub struct Graphics {
    /// The renderer context
    pub renderer: Option<Renderer>,

    /// The size of the display viewport
    pub viewport_size: (u32, u32),
}

/// Contains all resources required for rendering
pub struct Renderer {
    pub gpu: gpu::Gpu,
    pub ui_depth_texture_view: wgpu::TextureView,
    pub ui: egui_wgpu::Renderer,
    pub targets: Vec<RenderTarget>,
}

pub struct RenderTarget {
    pub color_texture: wgpu::Texture,
    pub color_texture_view: wgpu::TextureView,
    pub depth_texture: wgpu::Texture,
    pub depth_texture_view: wgpu::TextureView,
    pub grid: grid::Grid,
    pub sky: sky::Sky,
    pub lines: lines::Lines,
    pub quads: quads::Quads,
}

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub fn initialize_graphics_system(context: &mut crate::context::Context) {
    let window_handle = {
        let Some(window_handle) = context.resources.window.handle.as_mut() else {
            return;
        };
        window_handle.clone()
    };

    let winit::dpi::PhysicalSize { width, height } = window_handle.inner_size();
    context.resources.graphics.viewport_size = (width, height);

    let renderer = pollster::block_on(async move {
        create_renderer_async(window_handle.clone(), width, height, DEPTH_FORMAT).await
    });
    context.resources.graphics.renderer = Some(renderer);
}

pub fn render_frame_system(context: &mut crate::context::Context) {
    if context.resources.graphics.viewport_size == (0, 0) {
        return;
    }

    update_pane_uniforms_system(context);

    let mut viewports = context
        .resources
        .user_interface
        .tile_tree_context
        .viewport_tiles
        .values()
        .copied()
        .collect::<Vec<_>>();

    for (_, viewport) in viewports.iter_mut() {
        let scale_factor = context.resources.window.scale_factor;
        *viewport = egui::Rect {
            min: egui::pos2(viewport.min.x as f32, viewport.min.y as f32) * scale_factor as f32,
            max: egui::pos2(viewport.max.x as f32, viewport.max.y as f32) * scale_factor as f32,
        };
    }

    let Some((egui::FullOutput { textures_delta, .. }, paint_jobs)) =
        context.resources.user_interface.frame_output.take()
    else {
        return;
    };

    let Some(window_handle) = context.resources.window.handle.as_ref() else {
        return;
    };

    let screen_descriptor = {
        let (width, height) = context.resources.graphics.viewport_size;
        egui_wgpu::ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: window_handle.scale_factor() as f32,
        }
    };

    ensure_viewports(context, viewports.len());

    let Some(renderer) = context.resources.graphics.renderer.as_mut() else {
        return;
    };

    for (id, image_delta) in &textures_delta.set {
        renderer
            .ui
            .update_texture(&renderer.gpu.device, &renderer.gpu.queue, *id, image_delta);
    }

    for id in &textures_delta.free {
        renderer.ui.free_texture(id);
    }

    let mut encoder = renderer
        .gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

    renderer.ui.update_buffers(
        &renderer.gpu.device,
        &renderer.gpu.queue,
        &mut encoder,
        &paint_jobs,
        &screen_descriptor,
    );

    let Ok(surface_texture) = renderer.gpu.surface.get_current_texture() else {
        return;
    };

    let surface_texture_view = surface_texture
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Clear Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &surface_texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.19,
                        g: 0.24,
                        b: 0.42,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &renderer.ui_depth_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
    }

    viewports
        .iter()
        .zip(renderer.targets.iter())
        .for_each(|((kind, viewport), target)| {
            let viewport_size = (viewport.width() as u32, viewport.height() as u32);
            render_pane(&mut encoder, kind, target, viewport_size);

            let source_origin = wgpu::Origin3d { x: 0, y: 0, z: 0 };
            let destination_origin = wgpu::Origin3d {
                x: viewport.min.x as u32,
                y: viewport.min.y as u32,
                z: 0,
            };

            encoder.copy_texture_to_texture(
                wgpu::ImageCopyTexture {
                    texture: &target.color_texture,
                    mip_level: 0,
                    origin: source_origin,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyTexture {
                    texture: &surface_texture.texture,
                    mip_level: 0,
                    origin: destination_origin,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: viewport_size.0,
                    height: viewport_size.1,
                    depth_or_array_layers: 1,
                },
            );
        });

    {
        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("GUI Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &surface_texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &renderer.ui_depth_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        renderer.ui.render(
            &mut render_pass.forget_lifetime(),
            &paint_jobs,
            &screen_descriptor,
        );
    }

    renderer.gpu.queue.submit(std::iter::once(encoder.finish()));
    surface_texture.present();
}

async fn create_renderer_async(
    window: impl Into<wgpu::SurfaceTarget<'static>>,
    width: u32,
    height: u32,
    depth_format: wgpu::TextureFormat,
) -> Renderer {
    let gpu = gpu::create_gpu_async(window, width, height).await;
    let ui_depth_texture_view = {
        let device: &wgpu::Device = &gpu.device;
        let texture = device.create_texture(
            &(wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            }),
        );
        texture.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Depth32Float),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            base_array_layer: 0,
            array_layer_count: None,
            mip_level_count: None,
        })
    };
    let egui_renderer = egui_wgpu::Renderer::new(
        &gpu.device,
        gpu.surface_config.format,
        Some(depth_format),
        1,
        false,
    );

    Renderer {
        gpu,
        ui_depth_texture_view,
        ui: egui_renderer,
        targets: Vec::new(),
    }
}

pub fn resize_renderer_system(context: &mut crate::context::Context, width: u32, height: u32) {
    let Some(renderer) = context.resources.graphics.renderer.as_mut() else {
        return;
    };

    if width == 0 || height == 0 {
        return;
    }

    // Update surface config
    renderer.gpu.surface_config.width = width;
    renderer.gpu.surface_config.height = height;
    renderer
        .gpu
        .surface
        .configure(&renderer.gpu.device, &renderer.gpu.surface_config);

    let ui_depth_view = {
        let device: &wgpu::Device = &renderer.gpu.device;
        let texture = device.create_texture(
            &(wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            }),
        );
        texture.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Depth32Float),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            base_array_layer: 0,
            array_layer_count: None,
            mip_level_count: None,
        })
    };
    renderer.ui_depth_texture_view = ui_depth_view;

    renderer.targets = (0..renderer.targets.len())
        .map(|_| create_render_target(renderer))
        .collect();

    context.resources.graphics.viewport_size = (width, height);
}

fn create_render_target(renderer: &mut Renderer) -> RenderTarget {
    let color_texture = renderer
        .gpu
        .device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("Viewport Texture"),
            size: wgpu::Extent3d {
                width: renderer.gpu.surface_config.width,
                height: renderer.gpu.surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: renderer.gpu.surface_config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[renderer.gpu.surface_config.format],
        });
    let color_texture_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let device: &wgpu::Device = &renderer.gpu.device;
    let width = renderer.gpu.surface_config.width;
    let height = renderer.gpu.surface_config.height;
    let depth_texture = device.create_texture(
        &(wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        }),
    );
    let depth_texture_view = depth_texture.create_view(&wgpu::TextureViewDescriptor {
        label: None,
        format: Some(wgpu::TextureFormat::Depth32Float),
        dimension: Some(wgpu::TextureViewDimension::D2),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        base_array_layer: 0,
        array_layer_count: None,
        mip_level_count: None,
    });
    let grid = grid::create_grid(
        &renderer.gpu.device,
        renderer.gpu.surface_config.format,
        DEPTH_FORMAT,
    );
    let sky = sky::create_sky(
        &renderer.gpu.device,
        &renderer.gpu.queue,
        renderer.gpu.surface_config.format,
        DEPTH_FORMAT,
    );
    let lines = lines::create_line_renderer(
        &renderer.gpu.device,
        renderer.gpu.surface_config.format,
        DEPTH_FORMAT,
    );
    let quads = quads::create_quad_renderer(
        &renderer.gpu.device,
        renderer.gpu.surface_config.format,
        DEPTH_FORMAT,
    );
    RenderTarget {
        color_texture,
        color_texture_view,
        depth_texture,
        depth_texture_view,
        grid,
        sky,
        lines,
        quads,
    }
}

/// This synchronizes the viewport uniforms with the world
fn update_pane_uniforms_system(context: &mut crate::context::Context) {
    use crate::context::*;

    // First collect all viewport and camera data
    let viewports = context
        .resources
        .user_interface
        .tile_tree_context
        .viewport_tiles
        .values()
        .copied()
        .collect::<Vec<_>>();

    // Collect camera matrices
    let mut camera_matrices = Vec::new();
    for (kind, viewport) in &viewports {
        let matrices = if let PaneKind::Scene {
            camera_entity: Some(camera_entity),
            ..
        } = kind
        {
            if let (Some(camera), Some(transform)) = (
                get_component::<Camera>(context, *camera_entity, CAMERA),
                get_component::<GlobalTransform>(context, *camera_entity, GLOBAL_TRANSFORM),
            ) {
                let view = nalgebra_glm::inverse(&transform.0);
                let projection = camera.projection_matrix(viewport.width() / viewport.height());

                Some(CameraMatrices {
                    view,
                    projection,
                    camera_position: transform.0.column(3).xyz(),
                })
            } else {
                None
            }
        } else {
            None
        };
        camera_matrices.push(matrices);
    }

    // Collect scene data for each viewport
    let scene_data: Vec<_> = viewports
        .iter()
        .map(|(kind, _)| {
            if let PaneKind::Scene {
                scene_entity: _,
                camera_entity,
            } = kind
            {
                // Find the scene this camera belongs to by traversing up
                let actual_scene = if let Some(camera) = camera_entity {
                    let mut current = *camera;
                    let mut found_scene = None;
                    // Keep traversing up until we find a root node
                    while let Some(Parent(parent)) =
                        get_component::<Parent>(context, current, PARENT)
                    {
                        current = *parent;
                        // If current is a root node (no parent), this is our scene
                        if get_component::<Parent>(context, current, PARENT).is_none() {
                            found_scene = Some(current);
                            break;
                        }
                    }
                    found_scene
                } else {
                    None
                };

                // Use the actual scene entity for rendering
                if let Some(actual_scene) = actual_scene {
                    // Get all entities in this scene's hierarchy
                    let scene_entities = query_entities(context, LOCAL_TRANSFORM)
                        .into_iter()
                        .filter(|entity| is_descendant_of(context, *entity, actual_scene))
                        .collect::<Vec<_>>();

                    // Process lines for this scene's entities only
                    let scene_lines: Vec<_> = scene_entities
                        .iter()
                        .filter_map(|entity| {
                            let Lines(lines) = get_component::<Lines>(context, *entity, LINES)?;
                            let global_transform = get_component::<GlobalTransform>(
                                context,
                                *entity,
                                GLOBAL_TRANSFORM,
                            )?;

                            Some(
                                lines
                                    .iter()
                                    .map(|line| {
                                        // Transform line to world space
                                        let start_world = (global_transform.0
                                            * nalgebra_glm::vec4(
                                                line.start.x,
                                                line.start.y,
                                                line.start.z,
                                                1.0,
                                            ))
                                        .xyz();
                                        let end_world = (global_transform.0
                                            * nalgebra_glm::vec4(
                                                line.end.x, line.end.y, line.end.z, 1.0,
                                            ))
                                        .xyz();

                                        LineInstance {
                                            start: nalgebra_glm::vec4(
                                                start_world.x,
                                                start_world.y,
                                                start_world.z,
                                                1.0,
                                            ),
                                            end: nalgebra_glm::vec4(
                                                end_world.x,
                                                end_world.y,
                                                end_world.z,
                                                1.0,
                                            ),
                                            color: line.color,
                                        }
                                    })
                                    .collect::<Vec<_>>(),
                            )
                        })
                        .flatten()
                        .collect();

                    // Process quads for this scene's entities only
                    let scene_quads: Vec<_> = scene_entities
                        .iter()
                        .filter_map(|entity| {
                            let Quads(quads) = get_component::<Quads>(context, *entity, QUADS)?;
                            let global_transform = get_component::<GlobalTransform>(
                                context,
                                *entity,
                                GLOBAL_TRANSFORM,
                            )?;
                            Some(
                                quads
                                    .iter()
                                    .map(|quad| {
                                        let scale = nalgebra_glm::scaling(&nalgebra_glm::vec3(
                                            quad.size.x,
                                            quad.size.y,
                                            1.0,
                                        ));
                                        let offset =
                                            nalgebra_glm::translation(&nalgebra_glm::vec3(
                                                quad.offset.x,
                                                quad.offset.y,
                                                quad.offset.z,
                                            ));
                                        let final_transform = global_transform.0 * offset * scale;
                                        QuadInstance {
                                            model_matrix_0: final_transform.column(0).into(),
                                            model_matrix_1: final_transform.column(1).into(),
                                            model_matrix_2: final_transform.column(2).into(),
                                            model_matrix_3: final_transform.column(3).into(),
                                            color: quad.color,
                                        }
                                    })
                                    .collect::<Vec<_>>(),
                            )
                        })
                        .flatten()
                        .collect();

                    Some((scene_lines, scene_quads))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    // Now update renderer with collected data
    let Some(renderer) = context.resources.graphics.renderer.as_mut() else {
        return;
    };

    for (((target, (kind, _)), matrices), scene_data) in renderer
        .targets
        .iter_mut()
        .zip(viewports.iter())
        .zip(camera_matrices.iter())
        .zip(scene_data.iter())
    {
        match kind {
            PaneKind::Scene { .. } => {
                if let Some(matrices) = matrices {
                    grid::update_grid(matrices, &renderer.gpu.queue, &target.grid);
                    sky::update_sky(matrices, &renderer.gpu.queue, &target.sky);

                    if let Some((scene_lines, scene_quads)) = scene_data {
                        lines::update_lines_uniform(
                            matrices,
                            &renderer.gpu.device,
                            &renderer.gpu.queue,
                            &mut target.lines,
                            scene_lines.clone(),
                        );
                        quads::update_quads_uniform(
                            matrices,
                            &renderer.gpu.device,
                            &renderer.gpu.queue,
                            &mut target.quads,
                            scene_quads.clone(),
                        );
                    }
                }
            }
            PaneKind::Color(_color) => {}
            PaneKind::Empty => {}
        }
    }
}

fn render_pane(
    encoder: &mut wgpu::CommandEncoder,
    pane_kind: &PaneKind,
    target: &RenderTarget,
    viewport_size: (u32, u32),
) {
    let clear_color = match pane_kind {
        PaneKind::Scene { .. } => wgpu::Color::BLACK,
        PaneKind::Color(color) => wgpu::Color {
            r: (color.r() as f64 / 255.0),
            g: (color.g() as f64 / 255.0),
            b: (color.b() as f64 / 255.0),
            a: 1.0,
        },
        PaneKind::Empty => wgpu::Color {
            r: 32.0 / 255.0,
            g: 32.0 / 255.0,
            b: 32.0 / 255.0,
            a: 1.0,
        },
    };

    // Create viewport-sized render pass
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Viewport Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &target.color_texture_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(clear_color),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &target.depth_texture_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
    });

    // Set viewport to match pane size
    let viewport = egui::Rect::from_min_size(
        egui::pos2(0.0, 0.0),
        egui::vec2(viewport_size.0 as f32, viewport_size.1 as f32),
    );

    if viewport.width() <= 0.0 || viewport.height() <= 0.0 {
        return;
    }

    render_pass.set_viewport(
        viewport.min.x,
        viewport.min.y,
        viewport.width().max(1.0),
        viewport.height().max(1.0),
        0.0,
        1.0,
    );

    if matches!(pane_kind, PaneKind::Scene { .. }) {
        sky::render_sky(&mut render_pass, &target.sky);
        lines::render_lines(&mut render_pass, &target.lines);
        quads::render_quads(&mut render_pass, &target.quads);
        grid::render_grid(&mut render_pass, &target.grid);
    }
}

fn ensure_viewports(context: &mut Context, viewport_count: usize) {
    let Some(renderer) = context.resources.graphics.renderer.as_mut() else {
        return;
    };
    if renderer.targets.len() >= viewport_count {
        return;
    }
    let new_render_targets = viewport_count - renderer.targets.len();
    (0..new_render_targets).for_each(|_| {
        let target = create_render_target(renderer);
        renderer.targets.push(target);
    });
}

pub fn query_viewport_aspect_ratio(context: &crate::context::Context) -> Option<f32> {
    let Some(renderer) = &context.resources.graphics.renderer else {
        return None;
    };
    let surface_config = &renderer.gpu.surface_config;
    let aspect_ratio = surface_config.width as f32 / surface_config.height.max(1) as f32;
    Some(aspect_ratio)
}
