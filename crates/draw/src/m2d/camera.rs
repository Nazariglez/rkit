use corelib::math::{Mat3, Mat4, Rect, Vec2, vec2, vec3, vec4};

pub trait BaseCam2D {
    fn projection(&self) -> Mat4;
    fn inverse_projection(&self) -> Mat4;
    fn transform(&self) -> Mat3;
    fn inverse_transform(&self) -> Mat3;
    fn size(&self) -> Vec2;
    fn local_to_screen(&self, point: Vec2) -> Vec2;
    fn screen_to_local(&self, point: Vec2) -> Vec2;
    fn bounds(&self) -> Rect;
    fn is_pixel_perfect(&self) -> bool;

    fn is_point_visible(&self, pos: Vec2) -> bool {
        self.bounds().contains(pos)
    }

    fn is_rect_visible(&self, rect: Rect) -> bool {
        self.bounds().intersects(&rect)
    }
}

/// Defines how the camera view adapts to the screen resolution.
#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub enum ScreenMode {
    /// No scaling is applied. The camera's view size is directly mapped to the screen.
    ///
    /// This mode does not preserve aspect ratio, does not scale based on screen size,
    /// and is resolution-independent.
    #[default]
    Normal,

    /// Scales the camera view to exactly fill the screen ignoring the aspect ratio.
    ///
    /// The content is stretched to match the screen size, which may distort it.
    /// The `Vec2` parameter represents the working size.
    Fill(Vec2),

    /// Scales the camera view to fit entirely within the screen preserving aspect ratio.
    ///
    /// This may result in letterboxing.
    /// The `Vec2` parameter represents the intended working size.
    AspectFit(Vec2),

    /// Scales the camera view to fill the entire screen preserving aspect ratio.
    ///
    /// Parts of the content may be cropped if the screen and camera aspect ratios differ.
    /// The `Vec2` parameter represents the intended working size.
    AspectFill(Vec2),

    /// Scales the camera view so its width matches the screen's width preserving aspect ratio.
    ///
    /// This may result in vertical cropping or padding depending on the screen height.
    /// The `Vec2` parameter represents the intended working size.
    FitWidth(Vec2),

    /// Scales the camera view so its height matches the screen's height preserving aspect ratio.
    ///
    /// This may result in horizontal cropping or padding depending on the screen width.
    /// The `Vec2` parameter represents the intended working size.
    FitHeight(Vec2),
}

#[derive(Copy, Clone, Debug)]
pub struct Camera2D {
    pixel_perfect: bool,

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
            pixel_perfect: false,

            position: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
            size: Vec2::ONE,

            projection: Mat4::IDENTITY,
            inverse_projection: Mat4::IDENTITY,
            dirty_projection: true,

            ratio: Vec2::ONE,

            transform: Mat3::IDENTITY,
            inverse_transform: Mat3::IDENTITY,

            mode: ScreenMode::Normal,
            dirty_transform: true,
        }
    }
}

impl BaseCam2D for Camera2D {
    #[inline]
    fn projection(&self) -> Mat4 {
        debug_assert!(
            !self.dirty_projection,
            "You must call camera.update first to get an updated projection"
        );
        self.projection
    }

    #[inline]
    fn inverse_projection(&self) -> Mat4 {
        debug_assert!(
            !self.dirty_projection,
            "You must call camera.update first to get an updated inverse_projection"
        );
        self.inverse_projection
    }

    #[inline]
    fn transform(&self) -> Mat3 {
        debug_assert!(
            !self.dirty_transform,
            "You must call camera.update first to get an updated transform"
        );
        self.transform
    }

    #[inline]
    fn inverse_transform(&self) -> Mat3 {
        debug_assert!(
            !self.dirty_transform,
            "You must call camera.update first to get an updated inverse_transform"
        );
        self.inverse_transform
    }

    #[inline]
    fn is_pixel_perfect(&self) -> bool {
        self.pixel_perfect
    }

    #[inline]
    fn size(&self) -> Vec2 {
        self.size
    }

    #[inline]
    fn local_to_screen(&self, point: Vec2) -> Vec2 {
        self.transform_to_screen(point, self.transform())
    }

    #[inline]
    fn screen_to_local(&self, point: Vec2) -> Vec2 {
        self.screen_to_transform(point, self.inverse_transform())
    }

    fn bounds(&self) -> Rect {
        let size = self.size_visible();
        let origin = self.position - (size * 0.5);
        Rect::new(origin, size)
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

    #[inline]
    pub fn screen_to_transform(&self, point: Vec2, inverse: Mat3) -> Vec2 {
        // normalized coordinates
        let norm = point / self.size();
        let pos = norm * vec2(2.0, -2.0) + vec2(-1.0, 1.0);

        // projected position
        let pos = self
            .inverse_projection()
            .project_point3(vec3(pos.x, pos.y, 1.0));

        // local position
        inverse.transform_point2(vec2(pos.x, pos.y))
    }

    #[inline]
    pub fn transform_to_screen(&self, point: Vec2, transform: Mat3) -> Vec2 {
        let half = self.size() * 0.5;
        let pos = transform * vec3(point.x, point.y, 1.0);
        let pos = self.projection() * vec4(pos.x, pos.y, pos.z, 1.0);
        half + (half * vec2(pos.x, -pos.y))
    }

    #[inline]
    pub fn set_screen_mode(&mut self, mode: ScreenMode) {
        if self.mode != mode {
            self.mode = mode;
            self.dirty_projection = true;
        }
    }

    #[inline]
    pub fn screen_mode(&self) -> ScreenMode {
        self.mode
    }

    #[inline]
    pub fn set_pixel_perfect(&mut self, value: bool) {
        if self.pixel_perfect != value {
            self.pixel_perfect = value;
            self.dirty_projection = true;
            self.dirty_transform = true;
        }
    }

    #[inline]
    pub fn set_size(&mut self, size: Vec2) {
        if self.size != size {
            self.size = size;
            self.dirty_projection = true;
        }
    }

    #[inline]
    pub fn set_position(&mut self, pos: Vec2) {
        if self.position != pos {
            self.position = pos;
            self.dirty_transform = true;
        }
    }

    #[inline]
    pub fn position(&self) -> Vec2 {
        self.position
    }

    #[inline]
    pub fn set_rotation(&mut self, angle: f32) {
        if self.rotation != angle {
            self.rotation = angle;
            self.dirty_transform = true;
        }
    }

    #[inline]
    pub fn rotation(&self) -> f32 {
        self.rotation
    }

    #[inline]
    pub fn set_scale(&mut self, scale: Vec2) {
        if self.scale != scale {
            self.scale = scale;
            self.dirty_transform = true;
        }
    }

    #[inline]
    pub fn scale(&self) -> Vec2 {
        self.scale
    }

    #[inline]
    pub fn set_zoom(&mut self, scale: f32) {
        self.set_scale(Vec2::splat(scale));
    }

    #[inline]
    pub fn zoom(&self) -> f32 {
        self.scale.x
    }

    #[inline]
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

    #[inline]
    pub fn resolution(&self) -> Vec2 {
        match self.mode {
            ScreenMode::Normal => self.size,
            ScreenMode::Fill(r) => r,
            ScreenMode::AspectFit(r) => r,
            ScreenMode::AspectFill(r) => r,
            ScreenMode::FitWidth(r) => r,
            ScreenMode::FitHeight(r) => r,
        }
    }

    #[inline]
    pub fn ratio(&self) -> Vec2 {
        self.ratio
    }

    #[inline]
    pub fn size_visible(&self) -> Vec2 {
        self.size / (self.ratio * self.scale)
    }

    /// Translate a local point to screen coordinates
    #[inline]
    pub fn local_to_screen(&self, point: Vec2) -> Vec2 {
        debug_assert!(!self.dirty_projection);
        debug_assert!(!self.dirty_transform);
        BaseCam2D::local_to_screen(self, point)
    }

    /// Translates a screen point to local coordinates
    #[inline]
    pub fn screen_to_local(&self, point: Vec2) -> Vec2 {
        debug_assert!(!self.dirty_projection);
        debug_assert!(!self.dirty_transform);
        BaseCam2D::screen_to_local(self, point)
    }

    #[inline]
    fn calculate_projection(&mut self) {
        let (projection, ratio) = match self.mode {
            ScreenMode::Normal => calculate_ortho_projection(self.size, self.pixel_perfect),
            ScreenMode::Fill(work_size) => {
                calculate_fill_projection(self.size, work_size, self.pixel_perfect)
            }
            ScreenMode::AspectFit(work_size) => {
                calculate_aspect_fit_projection(self.size, work_size, self.pixel_perfect)
            }
            ScreenMode::AspectFill(work_size) => {
                calculate_aspect_fill_projection(self.size, work_size, self.pixel_perfect)
            }
            ScreenMode::FitWidth(work_size) => {
                calculate_fit_width_projection(self.size, work_size, self.pixel_perfect)
            }
            ScreenMode::FitHeight(work_size) => {
                calculate_fit_height_projection(self.size, work_size, self.pixel_perfect)
            }
        };

        self.projection = projection;
        self.inverse_projection = projection.inverse();
        self.ratio = ratio;
    }

    #[inline]
    fn calculate_transform(&mut self) {
        let translation = Mat3::from_translation(-self.position);
        let rotation = Mat3::from_angle(self.rotation);
        let scale = Mat3::from_scale(self.scale);
        let transform = rotation * scale * translation;
        self.transform = transform;
        self.inverse_transform = transform.inverse();
    }
}

fn calculate_ortho_projection(win_size: Vec2, pixel_perfect: bool) -> (Mat4, Vec2) {
    let win_size = if pixel_perfect {
        win_size.floor()
    } else {
        win_size
    };
    let pos = if pixel_perfect {
        (win_size * 0.5).floor()
    } else {
        win_size * 0.5
    };
    let projection = Mat4::orthographic_rh(0.0, win_size.x, win_size.y, 0.0, 0.0, 1.0);
    let position = Mat4::from_translation(vec3(pos.x, pos.y, 0.0));
    let final_projection = projection * position;
    (final_projection, vec2(1.0, 1.0))
}

fn calculate_scaled_projection(win_size: Vec2, ratio: Vec2, pixel_perfect: bool) -> Mat4 {
    let scale = Mat4::from_scale(vec3(ratio.x, ratio.y, 1.0));
    let pos = if pixel_perfect {
        (win_size * 0.5).floor()
    } else {
        win_size * 0.5
    };
    let position = vec3(pos.x, pos.y, 0.0);
    let translation = Mat4::from_translation(position);
    let projection = Mat4::orthographic_rh(0.0, win_size.x, win_size.y, 0.0, 0.0, 1.0);

    projection * translation * scale
}

fn calculate_fill_projection(win_size: Vec2, work_size: Vec2, pixel_perfect: bool) -> (Mat4, Vec2) {
    let (win_size, work_size) = if pixel_perfect {
        (win_size.floor(), work_size.floor())
    } else {
        (win_size, work_size)
    };

    let ratio = win_size / work_size;
    let projection = calculate_scaled_projection(win_size, ratio, pixel_perfect);
    (projection, ratio)
}

fn calculate_aspect_fit_projection(
    win_size: Vec2,
    work_size: Vec2,
    pixel_perfect: bool,
) -> (Mat4, Vec2) {
    let (win_size, work_size) = if pixel_perfect {
        (win_size.floor(), work_size.floor())
    } else {
        (win_size, work_size)
    };

    let ratio = (win_size / work_size).min_element();
    let ratio = Vec2::splat(ratio);
    let projection = calculate_scaled_projection(win_size, ratio, pixel_perfect);
    (projection, ratio)
}

fn calculate_aspect_fill_projection(
    win_size: Vec2,
    work_size: Vec2,
    pixel_perfect: bool,
) -> (Mat4, Vec2) {
    let (win_size, work_size) = if pixel_perfect {
        (win_size.floor(), work_size.floor())
    } else {
        (win_size, work_size)
    };

    let ratio = (win_size / work_size).max_element();
    let ratio = Vec2::splat(ratio);
    let projection = calculate_scaled_projection(win_size, ratio, pixel_perfect);
    (projection, ratio)
}

fn calculate_fit_width_projection(
    win_size: Vec2,
    work_size: Vec2,
    pixel_perfect: bool,
) -> (Mat4, Vec2) {
    let (win_size, work_size) = if pixel_perfect {
        (win_size.floor(), work_size.floor())
    } else {
        (win_size, work_size)
    };

    let ratio = win_size.x / work_size.x;
    let ratio = Vec2::splat(ratio);
    let projection = calculate_scaled_projection(win_size, ratio, pixel_perfect);
    (projection, ratio)
}

fn calculate_fit_height_projection(
    win_size: Vec2,
    work_size: Vec2,
    pixel_perfect: bool,
) -> (Mat4, Vec2) {
    let (win_size, work_size) = if pixel_perfect {
        (win_size.floor(), work_size.floor())
    } else {
        (win_size, work_size)
    };

    let ratio = win_size.y / work_size.y;
    let ratio = Vec2::splat(ratio);
    let projection = calculate_scaled_projection(win_size, ratio, pixel_perfect);
    (projection, ratio)
}
