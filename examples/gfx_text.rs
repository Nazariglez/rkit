use rkit::draw::{self, Sprite};
use rkit::gfx::{self, Color, Texture, TextureFormat};
use std::collections::HashMap;

use cosmic_text::{
    Attrs, Buffer as TBuffer, CacheKey, Family, FontSystem, Metrics, Shaping, Stretch, SwashCache,
    SwashContent, Weight,
};
use draw::draw_2d;
use etagere::*;
use rkit::math::{uvec2, vec2, Rect};

#[derive(Debug)]
struct GlyphInfo {
    pos: Rect, // TODO use i16 and u16 to reduce memory consumition to half
    atlas: Rect,
}

struct State {
    tbuffer: TBuffer,
    cache: SwashCache,
    font_system: FontSystem,
    images: HashMap<CacheKey, (Sprite, GlyphInfo)>,
    atlas_allocator: AtlasAllocator,
    tex: Sprite,
}

impl State {
    fn new() -> Result<Self, String> {
        const W: u32 = 250;
        const H: u32 = 250;
        let mut font_system = FontSystem::new(); // Manages fonts and caches
        let tbuffer = create_text_buffer(&mut font_system);
        let atlas = AtlasAllocator::new(size2(W as _, H as _));
        let tex = draw::create_sprite()
            .from_bytes(&[0; (W * H) as _], W, H)
            .with_format(TextureFormat::R8UNorm)
            .with_write_flag(true)
            .build()?;

        Ok(Self {
            font_system,
            tbuffer,
            cache: SwashCache::new(),
            images: HashMap::default(),
            atlas_allocator: atlas,
            tex,
        })
    }
}

fn main() {
    rkit::init_with(|| State::new().unwrap())
        .on_update(update)
        .run()
        .unwrap()
}

fn update(s: &mut State) {
    let mut draw = draw_2d();
    draw.clear(Color::WHITE);
    let mut pos = vec2(10.0, 300.0);

    for run in s.tbuffer.layout_runs() {
        println!("GLYPHS -> {} {} {}", run.glyphs.len(), run.text, run.line_i);
        for glyph in run.glyphs {
            let mut n = 0;

            let physical_glyph = glyph.physical((0.0, 0.0), 2.0);
            println!(
                "{:?} {:?}",
                physical_glyph.cache_key.glyph_id,
                s.images.contains_key(&physical_glyph.cache_key)
            );
            if !s.images.contains_key(&physical_glyph.cache_key) {
                if let Some(image) = s
                    .cache
                    .get_image_uncached(&mut s.font_system, physical_glyph.cache_key)
                {
                    match image.content {
                        SwashContent::Mask => {
                            let width = image.placement.width;
                            let height = image.placement.height;

                            if width == 0 || height == 0 {
                                continue;
                            }
                            let tex = draw::create_sprite()
                                .from_bytes(&image.data, width, height)
                                .with_format(TextureFormat::R8UNorm)
                                .build()
                                .unwrap();

                            let alloc = s
                                .atlas_allocator
                                .allocate(size2(width as _, height as _))
                                .unwrap();

                            let offset =
                                uvec2(alloc.rectangle.min.x as _, alloc.rectangle.min.y as _);
                            let size = uvec2(width, height);
                            gfx::write_texture(&s.tex.texture())
                                .from_data(&image.data)
                                .with_offset(offset)
                                .with_size(size)
                                .build()
                                .unwrap();

                            let info = GlyphInfo {
                                pos: Rect::new(
                                    vec2(image.placement.left as _, image.placement.top as _),
                                    vec2(image.placement.width as _, image.placement.height as _),
                                ),
                                atlas: Rect::new(
                                    vec2(alloc.rectangle.min.x as _, alloc.rectangle.min.y as _),
                                    vec2(
                                        alloc.rectangle.width() as _,
                                        alloc.rectangle.height() as _,
                                    ),
                                ),
                            };

                            println!("info: {:?}", info);

                            s.images.insert(physical_glyph.cache_key, (tex, info));
                        }
                        SwashContent::SubpixelMask => {
                            println!("|||||||||||||| HERE?");
                        }
                        SwashContent::Color => {
                            println!("|||||||||||||||||||||here?");
                        }
                    }
                }
            }

            if let Some((sprite, info)) = s.images.get(&physical_glyph.cache_key) {
                println!("N: {}", n);
                n += 1;
                let p = info.atlas.min();
                draw.image(sprite).position(p);
                draw.image(&s.tex).position(vec2(400.0, 0.0));

                let glyph_pos = vec2(physical_glyph.x as _, physical_glyph.y as _);
                let pp = pos
                    + glyph_pos
                    + info.pos.origin
                    + vec2(0.0, run.line_i as f32 * run.line_height);
                draw.image(&s.tex)
                    .crop(info.atlas.min(), info.pos.size)
                    .position(pp);
                // .crop(crop_origin, crop_size);

                // pos.x += info.atlas.size.x;
            }

            // println!("{:?}", glyph);
        }
    }

    println!("--------------");

    gfx::render_to_frame(&draw).unwrap();

    // panic!()
}

fn create_text_buffer(font_system: &mut FontSystem) -> TBuffer {
    font_system
        .db_mut()
        .load_font_data(include_bytes!("assets/Ubuntu-B.ttf").to_vec());

    let metrics = Metrics::new(16.0, 20.0); // Font size and line height
    let mut buffer = TBuffer::new(font_system, metrics); // Create the buffer for text

    let attrs = Attrs::new(); //.family(Family::Name("Utu")); //.family(Family::SansSerif).weight(Weight::BOLD);
    buffer.set_text(
        font_system,
        "Kanji (漢字, Japanese pronunciation: [kaɲdʑi])",
        attrs,
        Shaping::Advanced,
    ); // Set the text
    buffer.shape_until_scroll(font_system, false);

    buffer
}
