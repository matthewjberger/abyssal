use crate::context::{
    add_components, get_component, get_component_mut,
    graphics::query_viewport_aspect_ratio,
    input, query_entities,
    transform::{GlobalTransform, LocalTransform},
    Context, EntityId, CAMERA, GLOBAL_TRANSFORM, LOCAL_TRANSFORM,
};

#[derive(Debug, Clone)]
pub struct Camera {
    pub projection: Projection,
    pub fov: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            projection: Projection::Perspective(PerspectiveCamera::default()),
            fov: 45.0,
        }
    }
}

impl Camera {
    pub fn projection_matrix(&self, aspect_ratio: f32) -> nalgebra_glm::Mat4 {
        match &self.projection {
            Projection::Perspective(camera) => {
                let mut camera = camera.clone();
                camera.y_fov_rad = self.fov.to_radians();
                camera.matrix(aspect_ratio)
            }
            Projection::Orthographic(camera) => camera.matrix(),
        }
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub struct CameraMatrices {
    pub camera_position: nalgebra_glm::Vec3,
    pub projection: nalgebra_glm::Mat4,
    pub view: nalgebra_glm::Mat4,
}

#[derive(Debug, Clone)]
pub enum Projection {
    Perspective(PerspectiveCamera),
    Orthographic(OrthographicCamera),
}

impl Default for Projection {
    fn default() -> Self {
        Self::Perspective(PerspectiveCamera::default())
    }
}

#[derive(Debug, Clone)]
pub struct PerspectiveCamera {
    pub aspect_ratio: Option<f32>,
    pub y_fov_rad: f32,
    pub z_far: Option<f32>,
    pub z_near: f32,
}

impl Default for PerspectiveCamera {
    fn default() -> Self {
        Self {
            aspect_ratio: None,
            y_fov_rad: 90_f32.to_radians(),
            z_far: None,
            z_near: 0.01,
        }
    }
}

impl PerspectiveCamera {
    pub fn matrix(&self, viewport_aspect_ratio: f32) -> nalgebra_glm::Mat4 {
        let aspect_ratio = if let Some(aspect_ratio) = self.aspect_ratio {
            aspect_ratio
        } else {
            viewport_aspect_ratio
        };

        if let Some(z_far) = self.z_far {
            nalgebra_glm::perspective_zo(aspect_ratio, self.y_fov_rad, self.z_near, z_far)
        } else {
            nalgebra_glm::infinite_perspective_rh_zo(aspect_ratio, self.y_fov_rad, self.z_near)
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct OrthographicCamera {
    pub x_mag: f32,
    pub y_mag: f32,
    pub z_far: f32,
    pub z_near: f32,
}

impl OrthographicCamera {
    pub fn matrix(&self) -> nalgebra_glm::Mat4 {
        let z_sum = self.z_near + self.z_far;
        let z_diff = self.z_near - self.z_far;
        nalgebra_glm::Mat4::new(
            1.0 / self.x_mag,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0 / self.y_mag,
            0.0,
            0.0,
            0.0,
            0.0,
            2.0 / z_diff,
            0.0,
            0.0,
            0.0,
            z_sum / z_diff,
            1.0,
        )
    }
}

pub fn query_active_camera_matrices(context: &Context) -> Option<CameraMatrices> {
    let active_camera = context.resources.active_camera_entity?;
    query_camera_matrices(context, active_camera)
}

pub fn query_camera_matrices(context: &Context, camera_entity: EntityId) -> Option<CameraMatrices> {
    let (Some(camera), Some(local_transform), Some(global_transform)) = (
        get_component::<Camera>(context, camera_entity, CAMERA),
        get_component::<LocalTransform>(context, camera_entity, LOCAL_TRANSFORM),
        get_component::<GlobalTransform>(context, camera_entity, GLOBAL_TRANSFORM),
    ) else {
        return None;
    };

    let normalized_rotation = local_transform.rotation.normalize();
    let camera_translation = global_transform.0.column(3).xyz();
    let target = camera_translation
        + nalgebra_glm::quat_rotate_vec3(&normalized_rotation, &(-nalgebra_glm::Vec3::z()));
    let up = nalgebra_glm::quat_rotate_vec3(&normalized_rotation, &nalgebra_glm::Vec3::y());

    let aspect_ratio = query_viewport_aspect_ratio(context).unwrap_or(4.0 / 3.0);

    Some(CameraMatrices {
        camera_position: camera_translation,
        projection: camera.projection_matrix(aspect_ratio),
        view: nalgebra_glm::look_at(&camera_translation, &target, &up),
    })
}

/// Pure query function - only returns the nth camera entity
pub fn query_nth_camera(context: &Context, index: usize) -> Option<EntityId> {
    query_entities(context, CAMERA).get(index).copied()
}

/// Initializes a camera with proper transform settings
pub fn initialize_camera_transform(context: &mut Context, camera_entity: EntityId) {
    if let Some(local_transform) =
        get_component_mut::<LocalTransform>(context, camera_entity, LOCAL_TRANSFORM)
    {
        // Set a default position offset from origin
        local_transform.translation = nalgebra_glm::vec3(0.0, 4.0, 5.0);

        // Ensure rotation is looking at origin with proper up vector
        let camera_pos = local_transform.translation;
        let target = nalgebra_glm::Vec3::zeros();
        let up = nalgebra_glm::Vec3::y();

        // Calculate rotation to look at target
        let forward = nalgebra_glm::normalize(&(target - camera_pos));
        let right = nalgebra_glm::normalize(&nalgebra_glm::cross(&up, &forward));
        let new_up = nalgebra_glm::cross(&forward, &right);

        // Convert to quaternion
        let rotation_mat = nalgebra_glm::mat3(
            right.x, new_up.x, -forward.x, right.y, new_up.y, -forward.y, right.z, new_up.z,
            -forward.z,
        );
        local_transform.rotation = nalgebra_glm::mat3_to_quat(&rotation_mat);
    }
}

/// System that ensures all cameras have proper initialization
pub fn ensure_camera_transform_system(context: &mut Context) {
    let camera_entities: Vec<_> = query_entities(context, CAMERA)
        .into_iter()
        .filter(|entity| {
            get_component::<LocalTransform>(context, *entity, LOCAL_TRANSFORM).is_none()
        })
        .collect();

    for entity in camera_entities {
        add_components(context, entity, LOCAL_TRANSFORM);
        initialize_camera_transform(context, entity);
    }
}

pub fn query_nth_camera_matrices(context: &mut Context, index: usize) -> Option<CameraMatrices> {
    let camera_entity = query_nth_camera(context, index)?;
    let matrices = query_camera_matrices(context, camera_entity)?;
    Some(matrices)
}

pub fn wasd_keyboard_controls_system(context: &mut Context) {
    let Some(camera_entity) = context.resources.active_camera_entity else {
        return;
    };
    let delta_time = context.resources.window.delta_time;
    let speed = 10.0 * delta_time;

    let (
        left_key_pressed,
        right_key_pressed,
        forward_key_pressed,
        backward_key_pressed,
        up_key_pressed,
    ) = {
        let keyboard = &context.resources.input.keyboard;
        (
            keyboard.is_key_pressed(winit::keyboard::KeyCode::KeyA),
            keyboard.is_key_pressed(winit::keyboard::KeyCode::KeyD),
            keyboard.is_key_pressed(winit::keyboard::KeyCode::KeyW),
            keyboard.is_key_pressed(winit::keyboard::KeyCode::KeyS),
            keyboard.is_key_pressed(winit::keyboard::KeyCode::Space),
        )
    };

    let Some(local_transform) =
        get_component_mut::<LocalTransform>(context, camera_entity, LOCAL_TRANSFORM)
    else {
        return;
    };

    let forward = local_transform.forward_vector();
    let right = local_transform.right_vector();
    let up = local_transform.up_vector();

    if forward_key_pressed {
        local_transform.translation += forward * speed;
    }
    if backward_key_pressed {
        local_transform.translation -= forward * speed;
    }

    if left_key_pressed {
        local_transform.translation -= right * speed;
    }
    if right_key_pressed {
        local_transform.translation += right * speed;
    }
    if up_key_pressed {
        local_transform.translation += up * speed;
    }
}

/// Updates the active camera's orientation using
/// mouse controls for orbiting and panning
pub fn look_camera_system(context: &mut Context) {
    let Some(camera_entity) = context.resources.active_camera_entity else {
        return;
    };
    let (_local_transform_matrix, _, right, up) = {
        let Some(local_transform) =
            get_component_mut::<LocalTransform>(context, camera_entity, LOCAL_TRANSFORM)
        else {
            return;
        };
        let local_transform_matrix = local_transform.as_matrix();
        let forward = local_transform.forward_vector();
        let right = local_transform.right_vector();
        let up = local_transform.up_vector();
        (local_transform_matrix, forward, right, up)
    };

    if context
        .resources
        .input
        .mouse
        .state
        .contains(input::MouseState::RIGHT_CLICKED)
    {
        let mut delta =
            context.resources.input.mouse.position_delta * context.resources.window.delta_time;
        delta.x *= -1.0;
        delta.y *= -1.0;

        let Some(local_transform) =
            get_component_mut::<LocalTransform>(context, camera_entity, LOCAL_TRANSFORM)
        else {
            return;
        };

        let yaw = nalgebra_glm::quat_angle_axis(delta.x, &nalgebra_glm::Vec3::y());
        local_transform.rotation = yaw * local_transform.rotation;

        let forward = local_transform.forward_vector();
        let current_pitch = forward.y.asin();

        let new_pitch = current_pitch + delta.y;
        if new_pitch.abs() <= 89_f32.to_radians() {
            let pitch = nalgebra_glm::quat_angle_axis(delta.y, &nalgebra_glm::Vec3::x());
            local_transform.rotation *= pitch;
        }
    }

    if context
        .resources
        .input
        .mouse
        .state
        .contains(input::MouseState::MIDDLE_CLICKED)
    {
        let mut delta =
            context.resources.input.mouse.position_delta * context.resources.window.delta_time;
        delta.x *= -1.0;
        delta.y *= -1.0;

        let Some(local_transform) =
            get_component_mut::<LocalTransform>(context, camera_entity, LOCAL_TRANSFORM)
        else {
            return;
        };
        local_transform.translation += right * delta.x;
        local_transform.translation += up * delta.y;
    }
}
