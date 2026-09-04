use rkit::app::WindowConfig;
use rkit::draw::{self, Sprite, create_draw_2d};
use rkit::gfx::{self, Color, TextureMipLevel};
use rkit::math::vec2;

const MIP_IMAGES: [&[u8]; 9] = [
    include_bytes!("assets/ferris-mipmaps.png"),
    include_bytes!("assets/ferris-mipmaps-1.png"),
    include_bytes!("assets/ferris-mipmaps-2.png"),
    include_bytes!("assets/ferris-mipmaps-3.png"),
    include_bytes!("assets/ferris-mipmaps-4.png"),
    include_bytes!("assets/ferris-mipmaps-5.png"),
    include_bytes!("assets/ferris-mipmaps-6.png"),
    include_bytes!("assets/ferris-mipmaps-7.png"),
    include_bytes!("assets/ferris-mipmaps-8.png"),
];
const WIDTHS: [f32; 5] = [180.0, 110.0, 56.0, 28.0, 14.0];
const WIDTH_LABELS: [&str; 5] = ["180px", "110px", "56px", "28px", "14px"];
const CENTERS: [f32; 5] = [280.0, 500.0, 670.0, 800.0, 900.0];

struct State {
    plain: Sprite,
    generated: Sprite,
    manual: Sprite,
}

impl State {
    fn new() -> Result<Self, String> {
        let plain = draw::create_sprite().from_image(MIP_IMAGES[0]).build()?;
        let generated = draw::create_sprite()
            .from_image(MIP_IMAGES[0])
            .with_mipmaps()
            .build()?;

        let mipmaps = MIP_IMAGES
            .iter()
            .map(|bytes| decode_image(bytes))
            .collect::<Result<Vec<_>, _>>()?;
        let levels = mipmaps
            .iter()
            .map(|(pixels, width, height)| TextureMipLevel::new(pixels, *width, *height))
            .collect::<Vec<_>>();
        let manual = draw::create_sprite().from_mipmaps(&levels).build()?;

        Ok(Self {
            plain,
            generated,
            manual,
        })
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(|| State::new().unwrap())
        .with_window(
            WindowConfig::default()
                .title("Sprite mipmaps at progressively smaller sizes")
                .size(1050, 650)
                .resizable(false),
        )
        .update(update)
        .run()
}

fn update(state: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.035, 0.04, 0.055));
    draw.text("Mipmapped sprites at progressively smaller sizes")
        .translate(vec2(28.0, 20.0))
        .size(18.0)
        .color(Color::WHITE);

    let rows = [
        ("No mipmaps", &state.plain, 205.0),
        ("Generated", &state.generated, 390.0),
        ("Manual", &state.manual, 575.0),
    ];
    for (label, sprite, baseline) in rows {
        draw.text(label)
            .translate(vec2(28.0, baseline - 70.0))
            .size(14.0)
            .color(Color::WHITE);

        for (index, ((width, label), center)) in WIDTHS
            .into_iter()
            .zip(WIDTH_LABELS)
            .zip(CENTERS)
            .enumerate()
        {
            let height = width * 219.0 / 350.0;
            let background = if index % 2 == 0 {
                Color::rgb(0.82, 0.84, 0.88)
            } else {
                Color::rgb(0.025, 0.03, 0.045)
            };
            draw.rect(
                vec2(center - width * 0.5 - 8.0, baseline - height - 8.0),
                vec2(width + 16.0, height + 16.0),
            )
            .fill_color(background)
            .fill();
            draw.image(sprite)
                .position(vec2(center - width * 0.5, baseline - height))
                .size(vec2(width, height));
            draw.text(label)
                .translate(vec2(center - 18.0, baseline + 10.0))
                .size(9.0)
                .color(Color::GRAY);
        }
    }

    draw.text("Individual sprite image; atlas frames need extruded borders or authored mip levels")
        .translate(vec2(28.0, 620.0))
        .size(9.0)
        .color(Color::GRAY);
    gfx::render_to_frame(&draw).unwrap();
}

fn decode_image(bytes: &[u8]) -> Result<(Vec<u8>, u32, u32), String> {
    let image = image::load_from_memory(bytes)
        .map_err(|err| format!("Could not decode manual mip image: {err}"))?
        .into_rgba8();
    let (width, height) = image.dimensions();
    Ok((image.into_raw(), width, height))
}
