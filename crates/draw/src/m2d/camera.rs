use core::math::{vec2, vec3, vec4, Mat3, Mat4, Rect, Vec2};

pub trait BaseCam2D {
    fn projection(&self) -> Mat4;
    fn inverse_projection(&self) -> Mat4;
    fn transform(&self) -> Mat3;
    fn inverse_transform(&self) -> Mat3;
    fn size(&self) -> Vec2;
}

#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub enum ScreenMode {
    #[default]
    Normal,
    Fill(Vec2),
    AspectFit(Vec2),
    AspectFill(Vec2),
}

#[derive(Copy, Clone, Debug)]
pub struct Camera2D {
    position: Vec2,
    rotation: f32,
    scale: Vec2,
    size: Vec2,

    projection: Mat4,
    pub(crate) inverse_projection: Mat4,
    dirty_projection: bool,

    transform: Mat3,
    inverse_transform: Mat3,
    dirty_transform: bool,

    ratio: Vec2,
    mode: ScreenMode,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
            size: Vec2::ONE,

            projection: Mat4::IDENTITY,
            inverse_projection: Mat4::IDENTITY.inverse(),
            dirty_projection: true,

            ratio: Vec2::ONE,

            transform: Mat3::IDENTITY,
            inverse_transform: Mat3::IDENTITY.inverse(),

            mode: ScreenMode::Normal,
            dirty_transform: true,
        }
    }
}

impl BaseCam2D for Camera2D {
    fn projection(&self) -> Mat4 {
        debug_assert!(
            !self.dirty_projection,
            "You must call camera.update first to get an updated projection"
        );
        self.projection
    }

    fn inverse_projection(&self) -> Mat4 {
        debug_assert!(
            !self.dirty_projection,
            "You must call camera.update first to get an updated inverse_projection"
        );
        self.inverse_projection
    }

    fn transform(&self) -> Mat3 {
        debug_assert!(
            !self.dirty_transform,
            "You must call camera.update first to get an updated transform"
        );
        self.transform
    }

    fn inverse_transform(&self) -> Mat3 {
        debug_assert!(
            !self.dirty_transform,
            "You must call camera.update first to get an updated inverse_transform"
        );
        self.inverse_transform
    }

    fn size(&self) -> Vec2 {
        self.size
    }
}

impl Camera2D {
    pub fn new(size: Vec2, mode: ScreenMode) -> Self {
        let mut cam = Self {
            size,
            mode,
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
            self.dirty_projection = true;
        }
    }

    pub fn set_position(&mut self, pos: Vec2) {
        if self.position != pos {
            self.position = pos;
            self.dirty_transform = true;
        }
    }

    pub fn position(&self) -> Vec2 {
        self.position
    }

    pub fn set_rotation(&mut self, angle: f32) {
        if self.rotation != angle {
            self.rotation = angle;
            self.dirty_transform = true;
        }
    }

    pub fn rotation(&self) -> f32 {
        self.rotation
    }

    pub fn set_scale(&mut self, scale: Vec2) {
        if self.scale != scale {
            self.scale = scale;
            self.dirty_transform = true;
        }
    }

    pub fn scale(&self) -> Vec2 {
        self.scale
    }

    pub fn set_zoom(&mut self, scale: f32) {
        self.set_scale(Vec2::splat(scale));
    }

    pub fn zoom(&self) -> f32 {
        self.scale.x
    }

    pub fn update(&mut self) {
        if self.dirty_projection {
            self.calculate_projection();
            self.dirty_projection = false;
        }

        if self.dirty_transform {
            self.calculate_transform();
            self.dirty_transform = false;
        }
    }

    pub fn resolution(&self) -> Vec2 {
        match self.mode {
            ScreenMode::Normal => self.size,
            ScreenMode::Fill(r) => r,
            ScreenMode::AspectFit(r) => r,
            ScreenMode::AspectFill(r) => r,
        }
    }

    pub fn ratio(&self) -> Vec2 {
        self.ratio
    }

    /// Translate a local point to screen coordinates
    pub fn local_to_screen(&self, point: Vec2) -> Vec2 {
        debug_assert!(!self.dirty_projection);
        debug_assert!(!self.dirty_transform);
        let half = self.size * 0.5;
        let pos = self.transform * vec3(point.x, point.y, 1.0);
        let pos = self.projection * vec4(pos.x, pos.y, pos.z, 1.0);
        vec2(half.x + (half.x * pos.x), half.y + (half.y * -pos.y))
    }

    /// Translates a screen point to local coordinates
    pub fn screen_to_local(&self, point: Vec2) -> Vec2 {
        debug_assert!(!self.dirty_projection);
        debug_assert!(!self.dirty_transform);

        // normalized coordinates
        let norm = point / self.size;
        let pos = norm * vec2(2.0, -2.0) + vec2(-1.0, 1.0);

        // projected position
        let pos = self
            .inverse_projection
            .project_point3(vec3(pos.x, pos.y, 1.0));

        // local position
        self.inverse_transform.transform_point2(vec2(pos.x, pos.y))
    }

    pub fn bounds(&self) -> Rect {
        let size = self.size / (self.ratio * self.scale);
        let origin = self.position - (size * 0.5);
        Rect::new(origin, size)
    }

    fn calculate_projection(&mut self) {
        let (projection, ratio) = match self.mode {
            ScreenMode::Normal => calculate_ortho_projection(self.size),
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

    fn calculate_transform(&mut self) {
        let translation = Mat3::from_translation(-self.position);
        let rotation = Mat3::from_angle(self.rotation);
        let scale = Mat3::from_scale(self.scale);
        let transform = rotation * scale * translation;
        self.transform = transform;
        self.inverse_transform = transform.inverse();
    }
}

fn calculate_ortho_projection(win_size: Vec2) -> (Mat4, Vec2) {
    let projection = Mat4::orthographic_rh(0.0, win_size.x, win_size.y, 0.0, 0.0, 1.0);
    let pos = win_size * 0.5;
    let position = Mat4::from_translation(vec3(pos.x, pos.y, 0.0));
    let final_projection = projection * position;
    (final_projection, vec2(1.0, 1.0))
}

fn calculate_scaled_projection(win_size: Vec2, ratio: Vec2) -> Mat4 {
    let scale = Mat4::from_scale(vec3(ratio.x, ratio.y, 1.0));
    let pos = win_size * 0.5;
    let position = vec3(pos.x, pos.y, 0.0);
    let translation = Mat4::from_translation(position);
    let projection = Mat4::orthographic_rh(0.0, win_size.x, win_size.y, 0.0, 0.0, 1.0);

    projection * translation * scale
}

fn calculate_fill_projection(win_size: Vec2, work_size: Vec2) -> (Mat4, Vec2) {
    let ratio = win_size / work_size;
    let projection = calculate_scaled_projection(win_size, ratio);
    (projection, ratio)
}

fn calculate_aspect_fit_projection(win_size: Vec2, work_size: Vec2) -> (Mat4, Vec2) {
    let ratio = (win_size / work_size).min_element();
    let ratio = Vec2::splat(ratio);
    let projection = calculate_scaled_projection(win_size, ratio);
    (projection, ratio)
}

fn calculate_aspect_fill_projection(win_size: Vec2, work_size: Vec2) -> (Mat4, Vec2) {
    let ratio = (win_size / work_size).max_element();
    let ratio = Vec2::splat(ratio);
    let projection = calculate_scaled_projection(win_size, ratio);
    (projection, ratio)
}
