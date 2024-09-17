use rkit::draw::{self, Sprite};
use rkit::gfx::{self, Color, Texture, TextureFormat};
use std::collections::HashMap;

use cosmic_text::{
    Attrs, AttrsList, Buffer as TBuffer, CacheKey, Family, FontSystem, Metrics, Shaping,
    SwashCache, SwashContent, Weight,
};
use draw::draw_2d;
use etagere::*;
use rkit::math::{uvec2, vec2};

struct State {
    tbuffer: TBuffer,
    cache: SwashCache,
    font_system: FontSystem,
    images: HashMap<CacheKey, (Sprite, Allocation)>,
    atlas_allocator: AtlasAllocator,
    tex: Sprite,
}

impl State {
    fn new() -> Result<Self, String> {
        let a: [f32; 10] = [10.0; 10];
        let mut font_system = FontSystem::new(); // Manages fonts and caches
        let tbuffer = create_text_buffer(&mut font_system);
        let atlas = AtlasAllocator::new(size2(250, 250));
        let tex = draw::create_sprite()
            .from_bytes(&[0; 250 * 250], 250, 250)
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
    draw.clear(Color::BLACK);
    let mut pos = vec2(0.0, 0.0);

    for run in s.tbuffer.layout_runs() {
        for glyph in run.glyphs {
            let physical_glyph = glyph.physical((0.0, 0.0), 1.0);
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
                            println!("w: {width}, h: {height}, l:{}", image.data.len());
                            let tex = draw::create_sprite()
                                .from_bytes(&image.data, width, height)
                                .with_format(TextureFormat::R8UNorm)
                                .build()
                                .unwrap();

                            let alloc = s
                                .atlas_allocator
                                .allocate(size2(width as _, height as _))
                                .unwrap();

                            println!("{:?}", alloc.rectangle);

                            let offset =
                                uvec2(alloc.rectangle.min.x as _, alloc.rectangle.min.y as _);
                            let size = uvec2(width, height);
                            gfx::write_texture(&s.tex.texture())
                                .from_data(&image.data)
                                .with_offset(offset)
                                .with_size(size)
                                .build()
                                .unwrap();

                            s.images.insert(physical_glyph.cache_key, (tex, alloc));
                        }
                        SwashContent::SubpixelMask => {}
                        SwashContent::Color => {}
                    }
                }
            }

            if let Some((sprite, alloc)) = s.images.get(&physical_glyph.cache_key) {
                let p = vec2(alloc.rectangle.min.x as _, alloc.rectangle.min.y as _);
                draw.image(sprite).position(p);
                draw.image(&s.tex).position(vec2(400.0, 0.0));

                pos += vec2(sprite.size().x, 0.0);
            }

            println!("{:?}", glyph);
        }
    }

    println!("--------------");

    gfx::render_to_frame(&draw).unwrap();
}

fn create_text_buffer(font_system: &mut FontSystem) -> TBuffer {
    let metrics = Metrics::new(32.0, 20.0); // Font size and line height
    let mut buffer = TBuffer::new(font_system, metrics); // Create the buffer for text

    let attrs = Attrs::new()
        .family(Family::SansSerif)
        .weight(Weight::EXTRA_BOLD); // Default text attributes (you can customize this)
    buffer.set_text(
        font_system,
        "Hello, Cosmic Text! ",
        attrs,
        Shaping::Advanced,
    ); // Set the text
    buffer.shape_until_scroll(font_system, false);

    buffer
}
