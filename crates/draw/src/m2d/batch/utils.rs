use corelib::math::{Mat3, Mat4, Vec4};

pub(super) fn mat4_from_affine2d(m: Mat3) -> Mat4 {
    let c0 = m.x_axis;
    let c1 = m.y_axis;
    let c2 = m.z_axis;
    Mat4::from_cols(
        Vec4::new(c0.x, c0.y, 0.0, 0.0),
        Vec4::new(c1.x, c1.y, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 0.0),
        Vec4::new(c2.x, c2.y, 0.0, 1.0),
    )
}
