use rkit::draw::{self, Sprite};
use rkit::gfx::{self, Color, Texture, TextureFormat};
use std::collections::HashMap;
use std::process::id;
use std::sync::Arc;

use cosmic_text::fontdb::Source;
use cosmic_text::{
    Align, Attrs, AttrsList, Buffer as TBuffer, BufferLine, CacheKey, Family, FontSystem,
    LineEnding, Metrics, Shaping, Stretch, Style, SwashCache, SwashContent, Weight, Wrap,
};
use draw::draw_2d;
use etagere::*;
use rkit::math::{uvec2, vec2, Rect, Vec2};

use rkit::draw::text;

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
    sys: text::TextSystem,
    tbuffer: TBuffer,
    cache: SwashCache,
    font_system: FontSystem,
    glyphs: HashMap<CacheKey, GlyphInfo>,
    atlas_allocator: AtlasAllocator,
    tex: Sprite,
    mask: Sprite,
    color: Sprite,
}

impl State {
    fn new() -> Result<Self, String> {
        const W: u32 = 350;
        const H: u32 = 350;
        let mut font_system = FontSystem::new(); // Manages fonts and caches
        let tbuffer = create_text_buffer(&mut font_system);
        let atlas = AtlasAllocator::new(size2(W as _, H as _));
        let tex = draw::create_sprite()
            .from_bytes(&[0; (W * H * 4) as _], W, H)
            // .with_format(TextureFormat::R8UNorm)
            .with_write_flag(true)
            .build()?;

        let sys = text::TextSystem::new()?;
        let mask = sys.mask_texture();
        let color = sys.color_texture();
        Ok(Self {
            sys,
            font_system,
            tbuffer,
            cache: SwashCache::new(),
            glyphs: HashMap::default(),
            atlas_allocator: atlas,
            tex,
            mask,
            color,
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
    s.sys.prepare_text(&text::TextInfo {
        font: None,
        text: r#"
        ベクトルテキスト🎉
àèìòùáéíóúäëïöüñ;:!"·$%&/()=
Lorem ipsum dolor sit amet, consectetur adipiscing elit. Integer at tellus libero. Vivamus convallis libero eget tincidunt volutpat. In nisl ex, pretium aliquet libero vitae, posuere aliquam mi. Duis tempus consectetur mauris, eget eleifend eros consequat in. Interdum et malesuada fames ac ante ipsum primis in faucibus. Nunc vitae elit ullamcorper, auctor tortor quis, faucibus risus. Maecenas risus elit, egestas et enim in, commodo pretium felis. Morbi ultrices justo vel lacinia pellentesque. Nulla elementum, neque id rhoncus tincidunt, mauris lorem vulputate justo, rutrum egestas risus leo a sem. Nam ipsum leo, cursus et viverra quis, tempor vitae velit. Suspendisse porta ipsum vel orci dictum, vel malesuada massa interdum. Ut posuere orci ac volutpat ultricies.
Nunc varius, diam ac cursus dapibus, tortor quam dapibus massa, ut sodales ex velit non libero. Pellentesque commodo eros tristique porttitor fermentum. Vivamus vel arcu elit. Morbi vitae erat enim. Mauris dui mi, faucibus vehicula placerat id, ullamcorper eget tellus. In laoreet lectus pulvinar, tempor turpis id, imperdiet est. Maecenas sed tincidunt quam. Pellentesque habitant morbi tristique senectus et netus et malesuada fames ac turpis egestas. Etiam vel euismod nisi. Praesent euismod nibh mauris, non tempor lectus ornare at. Quisque finibus id orci id fringilla. Curabitur nec urna vitae mi tincidunt auctor. Donec pulvinar ex a volutpat eleifend.
Praesent dapibus diam a lacinia fermentum. Sed venenatis tellus non tristique venenatis. Vestibulum id euismod nisi. Proin at enim quis orci malesuada varius vitae et massa. Nunc sit amet dolor erat. Donec vel ipsum ipsum. Maecenas at maximus ante, et sagittis risus. Sed convallis commodo ipsum, quis sodales nulla tincidunt at. Etiam condimentum elementum ipsum molestie cursus. Proin viverra nibh vitae dictum interdum. Nullam eleifend nisl a turpis finibus congue. Duis cursus turpis ex, eget feugiat lorem hendrerit nec. Praesent at suscipit dui, nec ultrices nunc. Orci varius natoque penatibus et magnis dis parturient montes, nascetur ridiculus mus. Ut risus est, ultricies non luctus sed, vehicula ac quam.
Fusce auctor sem purus. Morbi venenatis neque id tempor elementum. Quisque fringilla arcu vitae ante imperdiet viverra. Sed vel ante augue. Phasellus tempus diam et ligula tempor iaculis. Integer elementum maximus lorem sed dictum. Nunc leo odio, blandit eget vestibulum ut, venenatis non eros. Nullam volutpat bibendum tristique. Maecenas eu maximus mi. Pellentesque gravida eros eget mi lacinia, placerat dapibus nibh sagittis. Pellentesque interdum lobortis vulputate. Etiam eget metus eget velit ullamcorper porta at et velit. Nam sed congue sem. Nunc vestibulum augue ex, at gravida nulla gravida at. In eget placerat metus, vitae vestibulum elit. Suspendisse non arcu eget eros porttitor vehicula.
Sed sed accumsan augue. Nulla semper semper volutpat. Aliquam erat volutpat. Nullam bibendum ac nisi eu posuere. Quisque ac erat erat. Sed commodo massa nisi, in suscipit justo ultricies quis. Nulla vitae lectus non leo tristique egestas. Nulla est sapien, aliquam at sagittis nec, maximus non risus. Class aptent taciti sociosqu ad litora torquent per conubia nostra, per inceptos himenaeos. Cras placerat, leo sed laoreet egestas, erat quam ullamcorper nisl, sit amet ornare quam eros id sem. In in enim tempus, interdum mauris a, varius sem.
        "#,
        wrap_width: None,
        font_size: 14.0,
        line_height: None,
        scale: 1.0,
    });

    let mut draw = draw_2d();
    draw.clear(Color::WHITE);

    draw.image(&s.mask);
    draw.image(&s.color).position(vec2(400.0, 0.0));

    // let mut pos = vec2(10.0, 300.0);
    //
    // let block_size = measure(&s.tbuffer);
    //
    // for run in s.tbuffer.layout_runs() {
    //     for glyph in run.glyphs {
    //         let physical_glyph = glyph.physical((0.0, 0.0), 1.0);
    //         if !s.glyphs.contains_key(&physical_glyph.cache_key) {
    //             if let Some(image) = s
    //                 .cache
    //                 .get_image_uncached(&mut s.font_system, physical_glyph.cache_key)
    //             {
    //                 let width = image.placement.width;
    //                 let height = image.placement.height;
    //
    //                 if width == 0 || height == 0 {
    //                     continue;
    //                 }
    //
    //                 let alloc = s
    //                     .atlas_allocator
    //                     .allocate(size2((width + 1) as _, (height + 1) as _))
    //                     .unwrap();
    //
    //                 let offset = uvec2(alloc.rectangle.min.x as _, alloc.rectangle.min.y as _);
    //                 let size = uvec2(width, height);
    //
    //                 let mut store = |bytes: &[u8]| {
    //                     gfx::write_texture(&s.tex.texture())
    //                         .from_data(&bytes)
    //                         .with_offset(offset)
    //                         .with_size(size)
    //                         .build()
    //                         .unwrap();
    //
    //                     let info = GlyphInfo {
    //                         pos: Pos::new(image.placement.left as _, -image.placement.top as _),
    //                         size: Pos::new(image.placement.width as _, image.placement.height as _),
    //                         atlas_pos: vec2(alloc.rectangle.min.x as _, alloc.rectangle.min.y as _),
    //                     };
    //
    //                     s.glyphs.insert(physical_glyph.cache_key, info);
    //                 };
    //
    //                 match image.content {
    //                     SwashContent::Mask => {
    //                         let bytes = image
    //                             .data
    //                             .iter()
    //                             .flat_map(|v| Color::rgba_u8(255, 255, 255, *v).to_rgba_u8())
    //                             .collect::<Vec<_>>();
    //                         store(&bytes);
    //                     }
    //                     SwashContent::SubpixelMask => {
    //                         println!("|||||||||||||| HERE?");
    //                     }
    //                     SwashContent::Color => {
    //                         // println!("|||||||||||||||||||||here?");
    //                         store(&image.data);
    //                     }
    //                 }
    //             }
    //         }
    //
    //         if let Some(info) = s.glyphs.get(&physical_glyph.cache_key) {
    //             let p = info.atlas_pos;
    //             draw.image(&s.tex).position(vec2(400.0, 0.0));
    //
    //             let offset = vec2(block_size.x - run.line_w, 0.0) * 0.5;
    //
    //             let glyph_pos = vec2(physical_glyph.x as _, physical_glyph.y as _);
    //             let pp = pos + offset + glyph_pos + info.pos.as_vec2() + vec2(0.0, run.line_y);
    //             draw.image(&s.tex)
    //                 .crop(info.atlas_pos, info.size.as_vec2())
    //                 // .color(Color::RED)
    //                 .position(pp);
    //         }
    //     }
    // }
    //
    println!("--------------");

    s.mask = s.sys.mask_texture();
    s.color = s.sys.color_texture();

    gfx::render_to_frame(&draw).unwrap();
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
