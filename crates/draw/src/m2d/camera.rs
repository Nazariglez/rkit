use crate::Transform2D;
use core::math::{vec2, vec3, vec4, Mat3, Mat4, Vec2};

#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub enum ScreenMode {
    #[default]
    Basic,
    Fill(Vec2),
    AspectFit(Vec2),
    AspectFill(Vec2),
}

#[derive(Copy, Clone, Debug)]
pub struct Camera2D {
    size: Vec2,

    projection: Mat4,
    inverse_projection: Mat4,
    dirty_projection: bool,

    ratio: Vec2,

    transform: Transform2D,
    inverse_matrix: Mat3,

    mode: ScreenMode,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            size: Vec2::ONE,

            projection: Mat4::IDENTITY,
            inverse_projection: Mat4::IDENTITY.inverse(),
            dirty_projection: true,

            ratio: Vec2::ONE,

            transform: Transform2D::default(),
            inverse_matrix: Mat3::IDENTITY.inverse(),

            mode: ScreenMode::Basic,
        }
    }
}

impl Camera2D {
    pub fn new(size: Vec2) -> Self {
        let mut t = Transform2D::default();
        t.set_size(size)
            .set_anchor(Vec2::splat(0.5))
            .set_pivot(Vec2::splat(0.5))
            .set_translation(size * 0.5);

        let mut cam = Self {
            size,
            transform: t,
            ..Default::default()
        };
        cam.update();
        cam
    }

    pub fn set_screen_mode(&mut self, mode: ScreenMode) {
        if self.mode != mode {
            self.mode = mode;
            self.dirty_projection = true;
        }
    }

    pub fn screen_mode(&self) -> ScreenMode {
        self.mode
    }

    pub fn set_size(&mut self, size: Vec2) {
        if self.size != size {
            self.size = size;
            self.transform.set_size(size);
            self.dirty_projection = true;
        }
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }

    pub fn set_position(&mut self, pos: Vec2) {
        if self.transform.position() != pos {
            self.transform.set_translation(pos);
        }
    }

    pub fn position(&self) -> Vec2 {
        self.transform.position()
    }

    pub fn set_rotation(&mut self, angle: f32) {
        if self.transform.rotation() != angle {
            self.transform.set_rotation(angle);
        }
    }

    pub fn rotation(&self) -> f32 {
        self.transform.rotation()
    }

    pub fn set_scale(&mut self, scale: Vec2) {
        if self.transform.scale() != scale {
            self.transform.set_scale(scale);
        }
    }

    pub fn transform(&self) -> Mat3 {
        debug_assert!(
            !self.transform.is_dirty(),
            "You must call camera.update first to get an updated transform"
        );
        self.transform.as_mat3()
    }

    pub fn projection(&self) -> Mat4 {
        debug_assert!(
            !self.dirty_projection,
            "You must call camera.update first to get an updated projection"
        );
        self.projection
    }

    pub fn update(&mut self) {
        if self.dirty_projection {
            println!("--> {:?}", self.size);
            self.calculate_projection();
            self.inverse_projection = self.projection.inverse();
            self.dirty_projection = false;
        }

        if self.transform.is_dirty() {
            self.transform.update();
            self.inverse_matrix = self.transform.updated_mat3().inverse();
        }
    }

    /// Translate a local point to screen coordinates
    pub fn local_to_screen(&mut self, point: Vec2) -> Vec2 {
        self.update();
        let half = self.size * 0.5;
        let transform = self.transform.updated_mat3();
        let pos = transform * vec3(point.x, point.y, 1.0);
        let pos = self.projection * vec4(pos.x, pos.y, pos.z, 1.0);
        vec2(half.x + (half.x * pos.x), half.y + (half.y * -pos.y))
    }

    /// Translates a screen point to local coordinates
    pub fn screen_to_local(&mut self, point: Vec2) -> Vec2 {
        self.update();

        // normalized coordinates
        let norm = point / self.size;
        let pos = norm * vec2(2.0, -2.0) + vec2(-1.0, 1.0);

        // projected position
        let pos = self
            .inverse_projection
            .project_point3(vec3(pos.x, pos.y, 1.0));

        // local position
        self.inverse_matrix.transform_point2(vec2(pos.x, pos.y))
    }

    fn calculate_projection(&mut self) {
        let (projection, ratio) = match self.mode {
            ScreenMode::Basic => calculate_ortho_projection(self.size),
            ScreenMode::Fill(work_size) => calculate_fill_projection(self.size, work_size),
            ScreenMode::AspectFit(work_size) => {
                calculate_aspect_fit_projection(self.size, work_size)
            }
            ScreenMode::AspectFill(work_size) => {
                calculate_aspect_fill_projection(self.size, work_size)
            }
        };

        self.projection = projection;
        self.inverse_projection = projection.inverse();
        self.ratio = ratio;
    }
}

fn calculate_ortho_projection(win_size: Vec2) -> (Mat4, Vec2) {
    // TODO position
    let projection = Mat4::orthographic_rh(0.0, win_size.x, win_size.y, 0.0, 0.0, 1.0);
    let pos = win_size * 0.5;
    let position = Mat4::from_translation(vec3(pos.x, pos.y, 1.0));
    let final_projection = projection; // * position;
    (final_projection, vec2(1.0, 1.0))
}

fn calculate_scaled_projection(win_size: Vec2, ratio: Vec2) -> Mat4 {
    let scale = Mat4::from_scale(vec3(ratio.x, ratio.y, 1.0));
    let pos = win_size * 0.5;
    let position = vec3(pos.x, pos.y, 1.0);
    let translation = Mat4::from_translation(position);
    let projection = Mat4::orthographic_rh(0.0, win_size.x, win_size.y, 0.0, -1.0, 1.0);

    projection * translation * scale
}

fn calculate_fill_projection(win_size: Vec2, work_size: Vec2) -> (Mat4, Vec2) {
    let ratio = vec2(win_size.x / work_size.x, win_size.y / work_size.y);
    let projection = calculate_scaled_projection(win_size, ratio);
    (projection, ratio)
}

fn calculate_aspect_fit_projection(win_size: Vec2, work_size: Vec2) -> (Mat4, Vec2) {
    let ratio = (win_size.x / work_size.x).min(win_size.y / work_size.y);
    let ratio = Vec2::splat(ratio);
    let projection = calculate_scaled_projection(win_size, ratio);
    (projection, ratio)
}

fn calculate_aspect_fill_projection(win_size: Vec2, work_size: Vec2) -> (Mat4, Vec2) {
    let ratio = (win_size.x / work_size.x).max(win_size.y / work_size.y);
    let ratio = Vec2::splat(ratio);
    let projection = calculate_scaled_projection(win_size, ratio);
    (projection, ratio)
}
