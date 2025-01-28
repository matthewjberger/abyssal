use crate::context::Context;

#[derive(Default)]
pub struct Window {
    /// The raw window handle
    pub handle: Option<std::sync::Arc<winit::window::Window>>,

    /// The scale factor of the window
    pub scale_factor: f64,

    /// The physical size of the window
    pub physical_size: winit::dpi::PhysicalSize<u32>,

    /// Should the program exit next frame
    pub should_exit: bool,

    /// The number of frames rendered per second
    pub frames_per_second: f32,

    /// The time between the last frame and the current frame
    pub delta_time: f32,

    /// The time the current frame was started
    pub last_frame_start_instant: Option<std::time::Instant>,

    /// When the current frame started
    pub current_frame_start_instant: Option<std::time::Instant>,

    /// When the initial frame started, when the application starts up
    pub initial_frame_start_instant: Option<std::time::Instant>,

    /// A monotonically increasing counter incremented each frame
    pub frame_counter: u32,

    /// Milliseconds that the process has been running continuously
    pub uptime_milliseconds: u64,
}

pub fn scale_factor_changed_system(
    context: &mut Context,
    scale_factor: f64,
    inner_size_writer: &winit::event::InnerSizeWriter,
) {
    context.resources.window.scale_factor = scale_factor;
    let winit::dpi::PhysicalSize { width, height } = context.resources.window.physical_size;
    let mut inner_size_writer = inner_size_writer.clone();
    if let Err(error) =
        inner_size_writer.request_inner_size(winit::dpi::PhysicalSize { width, height })
    {
        log::error!("Failed to request inner size. {error}");
    }
}

/// Calculates and refreshes frame timing values such as delta time
pub fn update_frame_timing_system(context: &mut Context) {
    let now = std::time::Instant::now();

    let crate::context::Context {
        resources:
            crate::context::Resources {
                window:
                    Window {
                        delta_time,
                        last_frame_start_instant,
                        current_frame_start_instant,
                        initial_frame_start_instant,
                        frame_counter,
                        uptime_milliseconds,
                        frames_per_second,
                        ..
                    },
                ..
            },
        ..
    } = context;

    // Capture first instant
    if initial_frame_start_instant.is_none() {
        *initial_frame_start_instant = Some(now);
    }

    // Delta time
    *delta_time =
        last_frame_start_instant.map_or(0.0, |last_frame| (now - last_frame).as_secs_f32());

    // Last frame start
    *last_frame_start_instant = Some(now);

    // Current frame start
    if current_frame_start_instant.is_none() {
        *current_frame_start_instant = Some(now);
    }

    // Calculate uptime
    if let Some(app_start) = *initial_frame_start_instant {
        *uptime_milliseconds = (now - app_start).as_millis() as u64;
    }

    // Calculate frames per second
    *frame_counter += 1;
    match current_frame_start_instant.as_ref() {
        Some(start) => {
            if (now - *start).as_secs_f32() >= 1.0 {
                *frames_per_second = *frame_counter as f32;
                *frame_counter = 0;
                *current_frame_start_instant = Some(now);
            }
        }
        None => {
            *current_frame_start_instant = Some(now);
        }
    }
}

pub fn initialize_window_system(
    context: &mut Context,
    event_loop: &winit::event_loop::ActiveEventLoop,
) {
    let Some(window_handle) = create_window(event_loop) else {
        return;
    };
    context.resources.window.handle = Some(window_handle.clone());
    context.resources.window.last_frame_start_instant = Some(std::time::Instant::now());
    context.resources.window.scale_factor = 1.0;
}

fn create_window(
    event_loop: &winit::event_loop::ActiveEventLoop,
) -> Option<std::sync::Arc<winit::window::Window>> {
    let mut attributes = winit::window::Window::default_attributes().with_title("Abyssal");
    if let Some(icon) = load_icon(include_bytes!("../icon/icon.png")) {
        attributes.window_icon = Some(icon);
    }
    let Ok(window) = event_loop.create_window(attributes) else {
        return None;
    };
    let window_handle = std::sync::Arc::new(window);
    Some(window_handle)
}

fn load_icon(bytes: &[u8]) -> Option<winit::window::Icon> {
    match image::load_from_memory(bytes) {
        Ok(image) => {
            let image = image.to_rgba8();
            let (width, height) = image.dimensions();
            if let Ok(icon) = winit::window::Icon::from_rgba(image.to_vec(), width, height) {
                Some(icon)
            } else {
                None
            }
        }
        Err(e) => {
            eprintln!("Failed to load icon: {e}");
            None
        }
    }
}
