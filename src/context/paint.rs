use crate::context::{Context, EntityId};
use nalgebra_glm::{Vec2, Vec3, Vec4};

#[derive(Default, Debug, Clone)]
pub struct Lines(pub Vec<Line>);

#[derive(Debug, Clone)]
pub struct Line {
    pub start: nalgebra_glm::Vec3,
    pub end: nalgebra_glm::Vec3,
    pub color: nalgebra_glm::Vec4,
}

#[derive(Default, Debug, Clone)]
pub struct Quads(pub Vec<Quad>);

#[derive(Debug, Clone)]
pub struct Quad {
    pub size: nalgebra_glm::Vec2,
    pub offset: nalgebra_glm::Vec3,
    pub color: nalgebra_glm::Vec4,
}

#[derive(Default)]
pub struct Painting {
    pub lines: Vec<Line>,
    pub quads: Vec<Quad>,
}

pub fn paint_quad(painting: &mut Painting, offset: Vec3, size: Vec2, color: Vec4) {
    painting.quads.push(Quad {
        offset,
        size,
        color,
    });
}

pub fn paint_line(painting: &mut Painting, start: Vec3, end: Vec3, color: Vec4) {
    painting.lines.push(Line { start, end, color });
}

pub fn paint_box(
    painting: &mut Painting,
    center: nalgebra_glm::Vec3,
    size: nalgebra_glm::Vec3,
    color: nalgebra_glm::Vec4,
) {
    let half_size = size * 0.5;

    // Calculate box corners
    let p000 = center + nalgebra_glm::vec3(-half_size.x, -half_size.y, -half_size.z);
    let p001 = center + nalgebra_glm::vec3(-half_size.x, -half_size.y, half_size.z);
    let p010 = center + nalgebra_glm::vec3(-half_size.x, half_size.y, -half_size.z);
    let p011 = center + nalgebra_glm::vec3(-half_size.x, half_size.y, half_size.z);
    let p100 = center + nalgebra_glm::vec3(half_size.x, -half_size.y, -half_size.z);
    let p101 = center + nalgebra_glm::vec3(half_size.x, -half_size.y, half_size.z);
    let p110 = center + nalgebra_glm::vec3(half_size.x, half_size.y, -half_size.z);
    let p111 = center + nalgebra_glm::vec3(half_size.x, half_size.y, half_size.z);

    // Bottom face
    paint_line(painting, p000, p100, color);
    paint_line(painting, p100, p101, color);
    paint_line(painting, p101, p001, color);
    paint_line(painting, p001, p000, color);

    // Top face
    paint_line(painting, p010, p110, color);
    paint_line(painting, p110, p111, color);
    paint_line(painting, p111, p011, color);
    paint_line(painting, p011, p010, color);

    // Vertical edges
    paint_line(painting, p000, p010, color);
    paint_line(painting, p100, p110, color);
    paint_line(painting, p101, p111, color);
    paint_line(painting, p001, p011, color);
}

pub fn paint_sphere(
    painting: &mut Painting,
    center: nalgebra_glm::Vec3,
    radius: f32,
    segments: u32,
    color: nalgebra_glm::Vec4,
) {
    // Draw longitudinal lines (like Earth's meridians)
    for i in 0..segments {
        let phi = i as f32 * std::f32::consts::PI / segments as f32;
        let rotation_matrix = nalgebra_glm::rotate(
            &nalgebra_glm::identity(),
            phi,
            &nalgebra_glm::vec3(0.0, 1.0, 0.0),
        );

        // Draw a full circle at this rotation
        for j in 0..segments {
            let theta1 = j as f32 * 2.0 * std::f32::consts::PI / segments as f32;
            let theta2 = (j + 1) as f32 * 2.0 * std::f32::consts::PI / segments as f32;

            let p1 = nalgebra_glm::vec3(theta1.cos() * radius, theta1.sin() * radius, 0.0);
            let p2 = nalgebra_glm::vec3(theta2.cos() * radius, theta2.sin() * radius, 0.0);

            let p1_rotated = rotation_matrix * nalgebra_glm::Vec4::new(p1.x, p1.y, p1.z, 1.0);
            let p2_rotated = rotation_matrix * nalgebra_glm::Vec4::new(p2.x, p2.y, p2.z, 1.0);

            paint_line(
                painting,
                center + nalgebra_glm::vec3(p1_rotated.x, p1_rotated.y, p1_rotated.z),
                center + nalgebra_glm::vec3(p2_rotated.x, p2_rotated.y, p2_rotated.z),
                color,
            );
        }
    }

    // Draw latitudinal lines (like Earth's parallels)
    let lat_segments = segments / 2;
    for i in 1..lat_segments {
        let phi = i as f32 * std::f32::consts::PI / lat_segments as f32;
        let current_radius = radius * phi.sin();
        let y = radius * phi.cos();

        for j in 0..segments {
            let theta1 = j as f32 * 2.0 * std::f32::consts::PI / segments as f32;
            let theta2 = (j + 1) as f32 * 2.0 * std::f32::consts::PI / segments as f32;

            let start = center
                + nalgebra_glm::vec3(
                    theta1.cos() * current_radius,
                    y,
                    theta1.sin() * current_radius,
                );
            let end = center
                + nalgebra_glm::vec3(
                    theta2.cos() * current_radius,
                    y,
                    theta2.sin() * current_radius,
                );

            paint_line(painting, start, end, color);
        }
    }
}

#[allow(dead_code)]
pub fn paint_entity(context: &mut Context, entity: EntityId, painting: Painting) {
    use crate::context::*;
    if let Some(Lines(lines)) = get_component_mut::<Lines>(context, entity, LINES) {
        lines.clear();
        *lines = painting.lines;
    }
    if let Some(Quads(quads)) = get_component_mut::<Quads>(context, entity, QUADS) {
        quads.clear();
        *quads = painting.quads;
    }
}
