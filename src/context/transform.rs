use crate::context::{
    get_component, get_component_mut, query_entities, tree::Parent, Context, EntityId,
    GLOBAL_TRANSFORM, LOCAL_TRANSFORM, PARENT,
};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LocalTransform {
    pub translation: nalgebra_glm::Vec3,
    pub rotation: nalgebra_glm::Quat,
    pub scale: nalgebra_glm::Vec3,
}

impl Default for LocalTransform {
    fn default() -> Self {
        Self {
            translation: nalgebra_glm::Vec3::new(0.0, 0.0, 0.0),
            rotation: nalgebra_glm::Quat::identity(),
            scale: nalgebra_glm::Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

impl LocalTransform {
    pub fn as_matrix(&self) -> nalgebra_glm::Mat4 {
        nalgebra_glm::translation(&self.translation)
            * nalgebra_glm::quat_to_mat4(&self.rotation.normalize())
            * nalgebra_glm::scaling(&self.scale)
    }

    pub fn right_vector(&self) -> nalgebra_glm::Vec3 {
        extract_right_vector(&self.as_matrix())
    }

    pub fn up_vector(&self) -> nalgebra_glm::Vec3 {
        extract_up_vector(&self.as_matrix())
    }

    pub fn forward_vector(&self) -> nalgebra_glm::Vec3 {
        extract_forward_vector(&self.as_matrix())
    }
}

#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct GlobalTransform(pub nalgebra_glm::Mat4);

impl GlobalTransform {
    pub fn right_vector(&self) -> nalgebra_glm::Vec3 {
        extract_right_vector(&self.0)
    }

    pub fn up_vector(&self) -> nalgebra_glm::Vec3 {
        extract_up_vector(&self.0)
    }

    pub fn forward_vector(&self) -> nalgebra_glm::Vec3 {
        extract_forward_vector(&self.0)
    }
}

fn extract_right_vector(transform: &nalgebra_glm::Mat4) -> nalgebra_glm::Vec3 {
    nalgebra_glm::vec3(transform[(0, 0)], transform[(1, 0)], transform[(2, 0)])
}

fn extract_up_vector(transform: &nalgebra_glm::Mat4) -> nalgebra_glm::Vec3 {
    nalgebra_glm::vec3(transform[(0, 1)], transform[(1, 1)], transform[(2, 1)])
}

fn extract_forward_vector(transform: &nalgebra_glm::Mat4) -> nalgebra_glm::Vec3 {
    nalgebra_glm::vec3(-transform[(0, 2)], -transform[(1, 2)], -transform[(2, 2)])
}

pub fn update_global_transforms_system(context: &mut Context) {
    query_entities(context, LOCAL_TRANSFORM | GLOBAL_TRANSFORM)
        .into_iter()
        .for_each(|entity| {
            let new_global_transform = query_global_transform(context, entity);
            let global_transform =
                get_component_mut::<GlobalTransform>(context, entity, GLOBAL_TRANSFORM).unwrap();
            *global_transform = GlobalTransform(new_global_transform);
        });
}

pub fn query_global_transform(context: &Context, entity: EntityId) -> nalgebra_glm::Mat4 {
    let Some(local_transform) = get_component::<LocalTransform>(context, entity, LOCAL_TRANSFORM)
    else {
        return nalgebra_glm::Mat4::identity();
    };
    if let Some(Parent(parent)) = get_component::<Parent>(context, entity, PARENT) {
        query_global_transform(context, *parent) * local_transform.as_matrix()
    } else {
        local_transform.as_matrix()
    }
}
