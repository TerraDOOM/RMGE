use nalgebra::base::{Matrix4, Vector2, Vector3};

pub type Vec2 = Vector2<f32>;
pub type Vec3 = Vector3<f32>;
pub type Mat4 = Matrix4<f32>;

#[derive(Debug, Copy, Clone)]
pub struct Quad {
    pub points: [Vec3; 4],
}

#[derive(Debug, Copy, Clone)]
pub struct Quad2d {
    pub points: [Vec2; 4],
}

impl From<Quad2d> for Quad {
    fn from(quad: Quad2d) -> Quad {
        let a = quad.points;
        Quad {
            points: [
                Vec3::new(a[0][0], a[0][1], 0.0),
                Vec3::new(a[1][0], a[1][1], 0.0),
                Vec3::new(a[2][0], a[2][1], 0.0),
                Vec3::new(a[3][0], a[3][1], 0.0),
            ],
        }
    }
}
