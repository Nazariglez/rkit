use rkit::draw::{self, Sprite};
use rkit::gfx::{self, Color, Texture, TextureFormat};
use rkit::{input, time};
use std::sync::Arc;

use cosmic_text::fontdb::Source;
use cosmic_text::{
    Align, Attrs, AttrsList, Buffer as TBuffer, BufferLine, CacheKey, Family, FontSystem,
    LineEnding, Metrics, Shaping, Stretch, Style, SwashCache, SwashContent, Weight, Wrap,
};
use draw::draw_2d;
use draw::text::get_text_system;
use rkit::app::{set_window_title, window_width};
use rkit::math::{uvec2, vec2, Rect, Vec2};

#[derive(Copy, Clone, Debug)]
struct Pos<N> {
    x: N,
    y: N,
}

impl<N> Pos<N> {
    pub fn new(x: N, y: N) -> Self {
        Self { x, y }
    }
}

impl Pos<i16> {
    pub fn as_vec2(self) -> Vec2 {
        vec2(self.x as _, self.y as _)
    }
}

impl Pos<u16> {
    pub fn as_vec2(self) -> Vec2 {
        vec2(self.x as _, self.y as _)
    }
}

#[derive(Debug)]
struct GlyphInfo {
    pos: Pos<i16>,
    size: Pos<u16>,
    atlas_pos: Vec2,
}

struct State {
    mask: Sprite,
    color: Sprite,
    font_size: f32,
    text: String,
}

impl State {
    fn new() -> Result<Self, String> {
        let sys = get_text_system();
        Ok(Self {
            mask: sys.mask_texture(),
            color: sys.color_texture(),
            font_size: 14.0,
            text: "".to_string(),
        })
    }
}

fn main() {
    rkit::init_with(|| State::new().unwrap())
        .on_update(update)
        .run()
        .unwrap()
}

fn measure(buffer: &TBuffer) -> Vec2 {
    let (width, total_lines) = buffer
        .layout_runs()
        .fold((0.0, 0usize), |(width, total_lines), run| {
            (run.line_w.max(width), total_lines + 1)
        });

    vec2(width, total_lines as f32 * buffer.metrics().line_height)
}

fn update(s: &mut State) {
    println!("frame init");
    // if is_key_down(KeyCode::ShiftLeft)
    //     && is_key_pressed(KeyCode::Space)
    //     && is_key_down(KeyCode::SuperLeft)
    // {
    //     s.font_size -= 1.0;
    // } else if is_key_down(KeyCode::ShiftLeft) && is_key_pressed(KeyCode::Space) {
    //     s.font_size += 1.0;
    // }
    //
    let mut draw = draw_2d();
    draw.clear(Color::ORANGE);

    // draw.image(&s.mask).position(vec2(200.0, 10.0));
    // draw.image(&s.color).position(vec2(400.0, 0.0));
    // //
    // draw.text("🤪ベクトルテキスト🎉")
    //     // draw.text("🤪🎉")
    //     .size(48.0)
    //     .color(Color::BLUE);

    draw.text(&format!("FPS: {:.2}", time::fps()))
        .position(vec2(10.0, 10.0));

    draw.text(&s.text)
        .position(vec2(200.0, 200.0))
        .size(12.0)
        .max_width(window_width())
        .h_align_center()
        // .position(vec2(400.0, 300.0))
        .color(Color::BLACK);

    let text_list = input::text_pressed();
    text_list.iter().for_each(|t| {
        s.text.push_str(t);
        // draw.text(t).size(s.font_size);
    });
    println!("--------------");
    //
    let sys = get_text_system();
    s.mask = sys.mask_texture();
    s.color = sys.color_texture();

    gfx::render_to_frame(&draw).unwrap();

    // set_window_title(&format!("{:.0}", time::fps()));
}

#[derive(Clone)]
struct Font {
    family: Arc<String>,
    weight: Weight,
    style: Style,
    stretch: Stretch,
}

fn create_font(font_system: &mut FontSystem, data: &'static [u8]) -> Result<Font, String> {
    let ids = font_system
        .db_mut()
        .load_font_source(Source::Binary(Arc::new(data)));
    let id = ids
        .get(0)
        .ok_or_else(|| "Cannot create the font".to_string())?
        .clone();
    let face = font_system
        .db()
        .face(id)
        .ok_or_else(|| "Invalid font type".to_string())?;
    Ok(Font {
        family: Arc::new(face.families[0].0.to_string()),
        weight: face.weight,
        style: face.style,
        stretch: face.stretch,
    })
}

fn create_text_buffer(font_system: &mut FontSystem) -> TBuffer {
    // font_system
    //     .db_mut()
    //     .load_font_data(include_bytes!("assets/Ubuntu-B.ttf").to_vec());

    let font = create_font(
        font_system,
        include_bytes!("assets/arcade-legacy/arcade-legacy.ttf"),
    )
    .unwrap();
    let attrs = Attrs::new()
        .family(Family::Name(&font.family))
        .weight(font.weight)
        .style(font.style)
        .stretch(font.stretch);

    let metrics = Metrics::new(16.0, 32.0 * 1.2); // Font size and line height
    let mut buffer = TBuffer::new(font_system, metrics); // Create the buffer for text

    buffer.set_text(font_system, "ベクトルテキスト🎉", attrs, Shaping::Advanced); // Set the text
    buffer.lines.push(BufferLine::new(
        "Super Text --- Super Text\nSuuuuuup",
        LineEnding::default(),
        AttrsList::new(attrs),
        Shaping::Advanced,
    ));
    // buffer.set_rich_text()
    buffer.set_wrap(font_system, Wrap::Word);
    // buffer.set_size(font_system, Some(250.0), None);
    buffer.shape_until_scroll(font_system, false);

    list_fonts(font_system);

    buffer
}

fn list_fonts(font_system: &FontSystem) {
    let db = font_system.db(); // Get the font database
    for (index, font) in db.faces().enumerate() {
        let family = &font.families;
        let weight = font.weight;
        let style = font.style;

        println!(
            "Font {}: Family: {:?}, Weight: {:?}, Style: {:?}",
            index, family, weight, style
        );
    }
}
