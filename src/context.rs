pub mod camera;
pub mod graphics;
pub mod input;
pub mod paint;
pub mod transform;
pub mod tree;
pub mod ui;
pub mod window;

crate::ecs! {
    Context {
        camera: camera::Camera => CAMERA,
        global_transform: transform::GlobalTransform => GLOBAL_TRANSFORM,
        lines: paint::Lines => LINES,
        local_transform: transform::LocalTransform => LOCAL_TRANSFORM,
        name: tree::Name => NAME,
        parent: tree::Parent => PARENT,
        quads: paint::Quads => QUADS,
    }
    Resources {
        window: window::Window,
        graphics: graphics::Graphics,
        user_interface: ui::UserInterface,
        input: input::Input,
        active_camera_entity: Option<EntityId>,
    }
}
