use crate::context::{camera, graphics, input, transform, ui, window, Context};

pub fn run(context: &mut Context) {
    let event_loop = match winit::event_loop::EventLoop::builder().build() {
        Ok(event_loop) => event_loop,
        Err(error) => {
            log::error!("Failed to create event loop: {error}");
            return;
        }
    };
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    if let Err(error) = event_loop.run_app(context) {
        log::error!("Failed to run app: {error}");
    }
}

impl winit::application::ApplicationHandler for Context {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.resources.window.handle.is_some() {
            return;
        }
        run_initialization_systems(self, event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if self.resources.window.should_exit
            || matches!(event, winit::event::WindowEvent::CloseRequested)
        {
            event_loop.exit();
            return;
        }
        run_systems(self, &event);
        if let Some(window_handle) = self.resources.window.handle.as_mut() {
            window_handle.request_redraw();
        }
    }
}

fn run_systems(context: &mut Context, event: &winit::event::WindowEvent) {
    match event {
        // Systems that run every frame
        winit::event::WindowEvent::RedrawRequested => {
            run_main_systems(context);
        }
        // Systems than run when the window is resized
        winit::event::WindowEvent::Resized(physical_size) => {
            context.resources.window.physical_size = *physical_size;
            let winit::dpi::PhysicalSize { width, height } = physical_size;
            run_resize_systems(context, *width, *height);
        }
        winit::event::WindowEvent::ScaleFactorChanged {
            scale_factor,
            inner_size_writer,
        } => {
            run_scale_factor_changed_systems(context, *scale_factor, inner_size_writer);
        }
        event => {
            ui::receive_window_event(context, event);
            input::receive_window_event(context, event);
        }
    }
}

/// Systems that run when the window is first created
fn run_initialization_systems(
    context: &mut Context,
    event_loop: &winit::event_loop::ActiveEventLoop,
) {
    window::initialize_window_system(context, event_loop);
    graphics::initialize_graphics_system(context);
    ui::initialize_ui_system(context);
}

// Systems that run every frame
fn run_main_systems(context: &mut Context) {
    // Wait for the renderer to be initialized before running systems
    if context.resources.graphics.renderer.is_none() {
        return;
    }
    window::update_frame_timing_system(context);
    ui::ensure_tile_tree_system(context);
    input::escape_key_exit_system(context);
    camera::look_camera_system(context);
    camera::wasd_keyboard_controls_system(context);
    transform::update_global_transforms_system(context);
    ui::create_ui_system(context);
    graphics::render_frame_system(context);
    input::reset_input_system(context);
}

// Systems that run when the window is resized
fn run_resize_systems(context: &mut Context, width: u32, height: u32) {
    graphics::resize_renderer_system(context, width, height);
}

// Systems that run when the window's scale factor changes
fn run_scale_factor_changed_systems(
    context: &mut Context,
    scale_factor: f64,
    inner_size_writer: &winit::event::InnerSizeWriter,
) {
    ui::scale_factor_changed_system(context, scale_factor);
    window::scale_factor_changed_system(context, scale_factor, inner_size_writer);
}
