use encase::ShaderType;
use std::ops::{Add, Div, Mul, Sub};

/// Represents a color in the sRGB space (alpha is linear)
#[derive(Default, Debug, Clone, Copy, PartialEq, ShaderType)]
#[align(16)]
pub struct Color {
    /// Red value
    pub r: f32,
    /// Green value
    pub g: f32,
    /// Blue value
    pub b: f32,
    /// Alpha value
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Color = Color::new(0.0, 0.0, 0.0, 0.0);
    pub const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);
    pub const BLACK: Color = Color::new(0.0, 0.0, 0.0, 1.0);
    pub const RED: Color = Color::new(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Color = Color::new(0.0, 1.0, 0.0, 1.0);
    pub const BLUE: Color = Color::new(0.0, 0.0, 1.0, 1.0);
    pub const YELLOW: Color = Color::new(1.0, 1.0, 0.0, 1.0);
    pub const MAGENTA: Color = Color::new(1.0, 0.0, 1.0, 1.0);
    pub const SILVER: Color = Color::new(0.753, 0.753, 0.753, 1.0);
    pub const GRAY: Color = Color::new(0.5, 0.5, 0.5, 1.0);
    pub const OLIVE: Color = Color::new(0.5, 0.5, 0.0, 1.0);
    pub const PURPLE: Color = Color::new(0.5, 0.0, 0.5, 1.0);
    pub const MAROON: Color = Color::new(0.5, 0.0, 0.0, 1.0);
    pub const BROWN: Color = Color::new(0.647, 0.164, 0.164, 1.0);
    pub const SADDLE_BROWN: Color = Color::new(0.545, 0.27, 0.074, 1.0);
    pub const AQUA: Color = Color::new(0.0, 1.0, 1.0, 1.0);
    pub const TEAL: Color = Color::new(0.0, 0.5, 0.5, 1.0);
    pub const NAVY: Color = Color::new(0.0, 0.0, 0.5, 1.0);
    pub const ORANGE: Color = Color::new(1.0, 0.647, 0.0, 1.0);
    pub const PINK: Color = Color::new(1.0, 0.753, 0.796, 1.0);

    #[inline(always)]
    /// Create a new color from red, green, blue and alpha values
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    #[inline(always)]
    /// Create a new color from red, green, blue and alpha values
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self::new(r, g, b, a)
    }

    #[inline(always)]
    /// Create a new color from red, green and blue values
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    #[inline(always)]
    /// Create a new color from hexadecimal number like 0x000000ff (0xRRGGBBAA)
    pub const fn hex(hex: u32) -> Self {
        let [r, g, b, a] = hex_to_rgba(hex);
        Self { r, g, b, a }
    }

    #[inline(always)]
    /// Create a new color from rgba bytes
    pub const fn rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    #[inline(always)]
    /// Returns the same color with the red passed
    pub const fn with_red(&self, red: f32) -> Color {
        Self::new(red, self.g, self.b, self.a)
    }

    #[inline(always)]
    /// Returns the same color with the green passed
    pub const fn with_green(&self, green: f32) -> Color {
        Self::new(self.r, green, self.b, self.a)
    }

    #[inline(always)]
    /// Returns the same color with the blue passed
    pub const fn with_blue(&self, blue: f32) -> Color {
        Self::new(self.r, self.g, blue, self.a)
    }

    #[inline(always)]
    /// Returns the same color with the alpha passed
    pub const fn with_alpha(&self, alpha: f32) -> Color {
        Self::new(self.r, self.g, self.b, alpha)
    }

    #[inline(always)]
    /// Returns an array with the r, g, b, a values
    pub const fn to_rgba(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    #[inline(always)]
    /// Returns an array with the r, g, b values
    pub const fn to_rgb(&self) -> [f32; 3] {
        [self.r, self.g, self.b]
    }

    #[inline(always)]
    /// Returns the hexadecimal representation of the color like 0xRRGGBBAA
    pub const fn to_hex(&self) -> u32 {
        rgba_to_hex(self.r, self.g, self.b, self.a)
    }

    #[inline(always)]
    /// Returns the hexadecimal representation of the colors as string like #RRGGBBAA
    pub fn to_hex_string(&self) -> String {
        hex_to_string(self.to_hex())
    }

    #[inline(always)]
    /// Returns byte representation of the color
    pub const fn to_rgba_u8(&self) -> [u8; 4] {
        let r = (self.r * 255.0) as _;
        let g = (self.g * 255.0) as _;
        let b = (self.b * 255.0) as _;
        let a = (self.a * 255.0) as _;
        [r, g, b, a]
    }

    #[inline(always)]
    /// Returns the same color as premultiplied alpha
    pub const fn to_premultiplied_alpha(&self) -> Color {
        if self.a == 0.0 {
            Self::TRANSPARENT
        } else if self.a == 1.0 {
            *self
        } else {
            Self {
                r: self.r * self.a,
                g: self.g * self.a,
                b: self.b * self.a,
                a: self.a,
            }
        }
    }

    /// Converts the sRGB color to linear RGB color
    pub fn to_linear_rgba(&self) -> LinearColor {
        (*self).into()
    }

    /// Converts a linear RGB color to sRGB and returns a Color struct
    pub fn from_linear_rgb(c: LinearColor) -> Color {
        c.into()
    }

    /// Returns the color as linear
    pub fn as_linear(self) -> Self {
        Self {
            r: gamma_to_linear(self.r),
            g: gamma_to_linear(self.g),
            b: gamma_to_linear(self.b),
            a: self.a,
        }
    }
}

impl From<Color> for [u8; 4] {
    fn from(c: Color) -> Self {
        c.to_rgba_u8()
    }
}

impl From<Color> for [f32; 4] {
    fn from(c: Color) -> Self {
        c.to_rgba()
    }
}

impl From<u32> for Color {
    fn from(color: u32) -> Self {
        Color::hex(color)
    }
}

impl From<[f32; 4]> for Color {
    fn from(color: [f32; 4]) -> Self {
        Color::new(color[0], color[1], color[2], color[3])
    }
}

impl From<[f32; 3]> for Color {
    fn from(color: [f32; 3]) -> Self {
        Color::rgb(color[0], color[1], color[2])
    }
}

impl From<[u8; 4]> for Color {
    fn from(color: [u8; 4]) -> Self {
        Color::rgba_u8(color[0], color[1], color[2], color[3])
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Color {{ r: {}, g: {}, b: {}, a: {}}}",
            self.r, self.g, self.b, self.a
        )
    }
}

#[inline(always)]
/// Converts a rgba color values to a hexadecimal values
pub const fn rgba_to_hex(r: f32, g: f32, b: f32, a: f32) -> u32 {
    (((r * 255.0) as u32) << 24)
        + (((g * 255.0) as u32) << 16)
        + (((b * 255.0) as u32) << 8)
        + ((a * 255.0) as u32)
}

#[inline(always)]
/// Converts an hexadecimal value to a rgba values
pub const fn hex_to_rgba(hex: u32) -> [f32; 4] {
    [
        ((hex >> 24) & 0xFF) as f32 / 255.0,
        ((hex >> 16) & 0xFF) as f32 / 255.0,
        ((hex >> 8) & 0xFF) as f32 / 255.0,
        (hex & 0xFF) as f32 / 255.0,
    ]
}

#[inline(always)]
/// Converts a hexadecimal value to string
pub fn hex_to_string(hex: u32) -> String {
    format!("{:#X}", hex)
}

/// Color on the linear space
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct LinearColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl From<Color> for LinearColor {
    fn from(c: Color) -> Self {
        Self {
            r: gamma_to_linear(c.r),
            g: gamma_to_linear(c.g),
            b: gamma_to_linear(c.b),
            a: c.a,
        }
    }
}

impl From<LinearColor> for Color {
    fn from(c: LinearColor) -> Self {
        Self {
            r: linear_to_gamma(c.r),
            g: linear_to_gamma(c.g),
            b: linear_to_gamma(c.b),
            a: c.a,
        }
    }
}

/// Converts the value from srgb space to linear
pub fn gamma_to_linear(x: f32) -> f32 {
    if x > 0.04045 {
        ((x + 0.055) / 1.055).powf(2.4)
    } else {
        x / 12.92
    }
}

/// Converts the value from linear space to srg
pub fn linear_to_gamma(x: f32) -> f32 {
    if x > 0.0031308 {
        let a = 0.055;
        (1.0 + a) * x.powf(1.0 / 2.4) - a
    } else {
        12.92 * x
    }
}

pub enum ColorType {
    Gamma(Color),
    Linear(LinearColor),
}

impl From<Color> for ColorType {
    fn from(c: Color) -> Self {
        Self::Gamma(c)
    }
}

impl From<LinearColor> for ColorType {
    fn from(c: LinearColor) -> Self {
        Self::Linear(c)
    }
}

impl From<ColorType> for Color {
    fn from(c: ColorType) -> Self {
        match c {
            ColorType::Gamma(c) => c,
            ColorType::Linear(c) => c.into(),
        }
    }
}

impl From<ColorType> for LinearColor {
    fn from(c: ColorType) -> Self {
        match c {
            ColorType::Gamma(c) => c.into(),
            ColorType::Linear(c) => c,
        }
    }
}

impl Add for Color {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            r: (self.r + other.r).min(1.0),
            g: (self.g + other.g).min(1.0),
            b: (self.b + other.b).min(1.0),
            a: (self.a + other.a).min(1.0),
        }
    }
}

impl Sub for Color {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            r: (self.r - other.r).max(0.0),
            g: (self.g - other.g).max(0.0),
            b: (self.b - other.b).max(0.0),
            a: (self.a - other.a).max(0.0),
        }
    }
}

impl Mul<f32> for Color {
    type Output = Self;

    fn mul(self, factor: f32) -> Self {
        Self {
            r: (self.r * factor).min(1.0),
            g: (self.g * factor).min(1.0),
            b: (self.b * factor).min(1.0),
            a: (self.a * factor).min(1.0),
        }
    }
}

impl Div<f32> for Color {
    type Output = Self;

    fn div(self, divisor: f32) -> Self {
        let factor = if divisor == 0.0 { 1.0 } else { 1.0 / divisor };
        self * factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < f32::EPSILON
    }

    #[test]
    fn test_color_addition() {
        let color1 = Color::rgb(0.5, 0.4, 0.3);
        let color2 = Color::rgb(0.2, 0.3, 0.4);
        let result = color1 + color2;

        assert!(approx_eq(result.r, 0.7));
        assert!(approx_eq(result.g, 0.7));
        assert!(approx_eq(result.b, 0.7));
        assert_eq!(result.a, 1.0);
    }

    #[test]
    fn test_color_subtraction() {
        let color1 = Color::rgb(0.5, 0.4, 0.3);
        let color2 = Color::rgb(0.2, 0.3, 0.4);
        let result = color1 - color2;

        assert!(approx_eq(result.r, 0.3));
        assert!(approx_eq(result.g, 0.1));
        assert!(approx_eq(result.b, 0.0)); // Clamped to 0
        assert_eq!(result.a, 0.0);
    }

    #[test]
    fn test_color_multiplication() {
        let color = Color::rgb(0.5, 0.4, 0.3);
        let result = color * 2.0;

        assert!(approx_eq(result.r, 1.0)); // Clamped to 1.0
        assert!(approx_eq(result.g, 0.8));
        assert!(approx_eq(result.b, 0.6));
        assert_eq!(result.a, 1.0);
    }

    #[test]
    fn test_color_division() {
        let color = Color::rgb(0.5, 0.4, 0.3);
        let result = color / 2.0;

        assert!(approx_eq(result.r, 0.25));
        assert!(approx_eq(result.g, 0.2));
        assert!(approx_eq(result.b, 0.15));
        assert_eq!(result.a, 0.5);
    }

    #[test]
    fn test_color_premultiplied_alpha() {
        let color = Color::rgba(0.5, 0.4, 0.3, 0.5);
        let result = color.to_premultiplied_alpha();

        assert!(approx_eq(result.r, 0.25));
        assert!(approx_eq(result.g, 0.2));
        assert!(approx_eq(result.b, 0.15));
        assert_eq!(result.a, 0.5);
    }

    // FIXME there is an issue with the precision converting to and from hex
    // #[test]
    // fn test_color_to_hex_and_back() {
    //     let color = Color::rgb(0.5, 0.4, 0.3);
    //     let hex = color.to_hex();
    //     let result = Color::hex(hex);
    //
    //     println!("COLOR: {:?} {:?}", color, result);
    //     assert!(approx_eq(result.r, color.r));
    //     assert!(approx_eq(result.g, color.g));
    //     assert!(approx_eq(result.b, color.b));
    //     assert_eq!(result.a, color.a);
    // }

    #[test]
    fn test_color_to_rgba_u8() {
        let color = Color::rgb(0.5, 0.4, 0.3);
        let rgba_u8 = color.to_rgba_u8();

        assert_eq!(rgba_u8, [127, 102, 76, 255]); // Converted to u8
    }

    #[test]
    fn test_color_conversion_linear_to_srgb() {
        let srgb_color = Color::rgb(0.5, 0.4, 0.3);
        let linear_color: LinearColor = srgb_color.to_linear_rgba();
        let result = Color::from_linear_rgb(linear_color);

        assert!(approx_eq(result.r, srgb_color.r));
        assert!(approx_eq(result.g, srgb_color.g));
        assert!(approx_eq(result.b, srgb_color.b));
        assert_eq!(result.a, srgb_color.a);
    }
}
