use crate::{
    Draw2D, DrawPipelineId, DrawingInfo, Element2D, Transform2D,
    text::{AtlasType, ColorTextSpan, Font, HAlign, TextInfo, get_mut_text_system},
};
use corelib::{
    gfx::Color,
    math::{IntoVec2, Rect, Vec2, bvec2},
};
use macros::Drawable2D;
use smallvec::SmallVec;
use std::cell::RefCell;

type SpanVec<'a> = SmallVec<ColorTextSpan<'a>, 8>;
type ColorStack = SmallVec<Color, 4>;

thread_local! {
    static TEMP_VERTICES: RefCell<Vec<f32>> = const { RefCell::new(vec![]) };
    static TEMP_INDICES: RefCell<Vec<u32>> = const { RefCell::new(vec![]) };
}

#[derive(Drawable2D)]
pub struct ColorText2D<'a> {
    text: &'a str,
    font: Option<&'a Font>,
    position: Vec2,
    color: Color,
    alpha: f32,
    size: f32,
    line_height: Option<f32>,
    max_width: Option<f32>,
    h_align: HAlign,
    res: f32,
    shadow_color: Color,
    shadow_offset: Option<Vec2>,

    #[pipeline_id]
    pip: DrawPipelineId,

    #[transform_2d]
    transform: Option<Transform2D>,
}

impl<'a> ColorText2D<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            font: None,
            position: Vec2::ZERO,
            color: Color::WHITE,
            alpha: 1.0,
            size: 14.0,
            line_height: None,
            max_width: None,
            h_align: HAlign::default(),
            res: 1.0,
            shadow_color: Color::BLACK,
            shadow_offset: None,
            pip: DrawPipelineId::Text,
            transform: None,
        }
    }

    pub fn font(&mut self, font: &'a Font) -> &mut Self {
        self.font = Some(font);
        self
    }

    pub fn color(&mut self, color: Color) -> &mut Self {
        self.color = color;
        self
    }

    pub fn alpha(&mut self, alpha: f32) -> &mut Self {
        self.alpha = alpha;
        self
    }

    pub fn position(&mut self, pos: Vec2) -> &mut Self {
        self.position = pos;
        self
    }

    pub fn size(&mut self, size: f32) -> &mut Self {
        self.size = size;
        self
    }

    pub fn line_height(&mut self, height: f32) -> &mut Self {
        self.line_height = Some(height);
        self
    }

    pub fn max_width(&mut self, width: f32) -> &mut Self {
        self.max_width = Some(width);
        self
    }

    pub fn h_align_left(&mut self) -> &mut Self {
        self.h_align = HAlign::Left;
        self
    }

    pub fn h_align_center(&mut self) -> &mut Self {
        self.h_align = HAlign::Center;
        self
    }

    pub fn h_align_right(&mut self) -> &mut Self {
        self.h_align = HAlign::Right;
        self
    }

    pub fn resolution(&mut self, res: f32) -> &mut Self {
        self.res = res;
        self
    }

    pub fn shadow_color(&mut self, color: Color) -> &mut Self {
        self.shadow_color = color;
        self
    }

    pub fn shadow_offset(&mut self, offset: impl IntoVec2) -> &mut Self {
        self.shadow_offset = Some(offset.into_vec2());
        self
    }
}

impl Element2D for ColorText2D<'_> {
    fn process(&self, draw: &mut Draw2D) {
        let spans = parse_color_tags(self.text, self.color);

        if self.shadow_offset.is_some() {
            add_color_text_to_batch(self, &spans, true, draw);
        }

        add_color_text_to_batch(self, &spans, false, draw);
    }
}

fn add_color_text_to_batch(
    element: &ColorText2D,
    spans: &[ColorTextSpan],
    is_shadow: bool,
    draw: &mut Draw2D,
) {
    let (offset, base_color) = match element.shadow_offset {
        Some(offset) if is_shadow => (
            offset,
            element
                .shadow_color
                .with_alpha(element.shadow_color.a * element.alpha),
        ),
        _ => (
            Vec2::ZERO,
            element.color.with_alpha(element.color.a * element.alpha),
        ),
    };

    let info = TextInfo {
        pos: element.position + offset,
        font: element.font,
        text: element.text,
        spans: Some(spans),
        wrap_width: element.max_width,
        font_size: element.size,
        line_height: element.line_height,
        resolution: element.res,
        h_align: element.h_align,
    };

    TEMP_VERTICES.with_borrow_mut(|temp_vertices| {
        TEMP_INDICES.with_borrow_mut(|temp_indices| {
            temp_vertices.clear();
            temp_indices.clear();

            let block_size = {
                let mut sys = get_mut_text_system();
                let block = sys.prepare_text(&info, false).unwrap();
                if block.data.is_empty() {
                    return;
                }

                block.data.iter().enumerate().for_each(|(i, data)| {
                    let Vec2 { x: x1, y: y1 } = data.xy;
                    let Vec2 { x: x2, y: y2 } = data.xy + data.size;
                    let Vec2 { x: u1, y: v1 } = data.uvs1;
                    let Vec2 { x: u2, y: v2 } = data.uvs2;
                    let t = match data.typ {
                        AtlasType::Mask if data.pixelated => 1.0,
                        AtlasType::Mask => 0.0,
                        _ => 2.0,
                    };

                    // use per-glyph color if available if is not a shadow
                    let gc = if is_shadow {
                        base_color
                    } else {
                        data.color
                            .map(|col| col.with_alpha(col.a * element.alpha))
                            .unwrap_or(base_color)
                    };

                    #[rustfmt::skip]
                    let vertices = [
                        x1, y1, u1, v1, t, gc.r, gc.g, gc.b, gc.a,
                        x2, y1, u2, v1, t, gc.r, gc.g, gc.b, gc.a,
                        x1, y2, u1, v2, t, gc.r, gc.g, gc.b, gc.a,
                        x2, y2, u2, v2, t, gc.r, gc.g, gc.b, gc.a,
                    ];

                    let n = (i * 4) as u32;

                    #[rustfmt::skip]
                    let indices = [
                        n,     n + 1,   n + 2,
                        n + 2, n + 1,   n + 3
                    ];

                    temp_vertices.extend_from_slice(vertices.as_slice());
                    temp_indices.extend_from_slice(indices.as_slice());
                });

                block.size
            };

            let mut t = element.transform.unwrap_or_default();
            t.set_size(block_size);
            let pos = t.translation();
            let anchor = t.anchor();
            let scaled_size = t.size() * t.scale();
            let matrix = t.updated_mat3();

            let origin = element.position + pos - anchor * scaled_size;
            draw.last_text_bounds = Rect::new(origin, scaled_size);

            draw.add_to_batch(DrawingInfo {
                pipeline: element.pip,
                vertices: temp_vertices,
                indices: temp_indices,
                transform: matrix,
                sprite: None,
            });
        });
    });
}

const OPEN_TAG: &str = "[color:#";
const CLOSE_TAG: &str = "[/color]";

// pushes a span if the text slice is non empty
#[inline]
fn push_span<'a>(spans: &mut SpanVec<'a>, text: &'a str, color_stack: &ColorStack) {
    if !text.is_empty() {
        spans.push(ColorTextSpan {
            text,
            color: Some(*color_stack.last().unwrap()),
        });
    }
}

// tries to parse an opening color tag
#[inline]
fn try_parse_open_tag(s: &str) -> Option<(Color, usize)> {
    // s should start right after "[color:#"
    let close_bracket = s.find(']')?;
    let hex_str = &s[..close_bracket];
    let color = parse_hex_color(hex_str);
    Some((color, close_bracket + 1))
}

/// Parses color tags [color:#RRGGBB]...[/color] or [color:#RRGGBBAA]...[/color]
fn parse_color_tags<'a>(input: &'a str, default_color: Color) -> SpanVec<'a> {
    let mut spans = SpanVec::new();
    let mut color_stack: ColorStack = SmallVec::new();
    color_stack.push(default_color);
    let mut cursor = 0;

    while cursor < input.len() {
        let remaining = &input[cursor..];

        let open_pos = remaining.find(OPEN_TAG);
        let close_pos = remaining.find(CLOSE_TAG);

        match (open_pos, close_pos) {
            // opening tag comes first
            (Some(op), close) if close.is_none() || op < close.unwrap() => {
                push_span(&mut spans, &remaining[..op], &color_stack);

                let after_prefix = &remaining[op + OPEN_TAG.len()..];
                if let Some((color, consumed)) = try_parse_open_tag(after_prefix) {
                    color_stack.push(color);
                    cursor += op + OPEN_TAG.len() + consumed;
                } else {
                    // malformed tagd
                    cursor += op + 1;
                }
            }
            // closing tag comes first
            (open, Some(cp)) if open.is_none() || cp < open.unwrap() => {
                push_span(&mut spans, &remaining[..cp], &color_stack);

                if color_stack.len() > 1 {
                    color_stack.pop();
                }
                cursor += cp + CLOSE_TAG.len();
            }
            // no more tags
            (None, None) => {
                push_span(&mut spans, remaining, &color_stack);
                break;
            }
            _ => break,
        }
    }

    spans
}

// parses hex color
// differs from Color::hex_string becsue I need to support with alpha and without
#[inline]
fn parse_hex_color(hex: &str) -> Color {
    let bytes = hex.as_bytes();
    let mut value: u32 = 0;

    for &b in bytes.iter().take(8) {
        let nibble = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => 0,
        };
        value = (value << 4) | nibble as u32;
    }

    if bytes.len() <= 6 {
        value = (value << 8) | 0xFF;
    }

    Color::hex(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_color_tag() {
        let default = Color::WHITE;
        let spans = parse_color_tags("Hello [color:#FF0000]world[/color]!", default);

        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].text, "Hello ");
        assert_eq!(spans[0].color, Some(Color::WHITE));
        assert_eq!(spans[1].text, "world");
        assert_eq!(spans[2].text, "!");
        assert_eq!(spans[2].color, Some(Color::WHITE));
    }

    #[test]
    fn test_parse_nested_color_tags() {
        let default = Color::WHITE;
        let spans = parse_color_tags(
            "A [color:#FF0000]red [color:#00FF00]green[/color] back[/color] end",
            default,
        );

        assert_eq!(spans.len(), 5);
        assert_eq!(spans[0].text, "A ");
        assert_eq!(spans[1].text, "red ");
        assert_eq!(spans[2].text, "green");
        assert_eq!(spans[3].text, " back");
        assert_eq!(spans[4].text, " end");
    }

    #[test]
    fn test_parse_no_tags() {
        let default = Color::WHITE;
        let spans = parse_color_tags("Hello world!", default);

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "Hello world!");
        assert_eq!(spans[0].color, Some(Color::WHITE));
    }

    #[test]
    fn test_parse_rgba_color() {
        let default = Color::WHITE;
        let spans = parse_color_tags("[color:#FF000080]semi-transparent[/color]", default);

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "semi-transparent");
    }
}
