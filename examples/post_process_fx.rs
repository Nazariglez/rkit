use corelib::app::window_height;
use corelib::input::{KeyCode, is_key_pressed};
use rkit::app::window_size;
use rkit::draw::{Sprite, create_draw_2d};
use rkit::gfx::{self, Color};
use rkit::math::{Vec2, vec2};
use rkit::postfx::{
    BlurFx, ColorReplaceFx, GrayScaleFx, PixelateFx, PostFx, PostProcess, RgbSplitFx,
};
use rkit::time;

struct MyFilters {
    pixelate: PixelateFx,
    color_replace: ColorReplaceFx,
    gray_scale: GrayScaleFx,
    blur: BlurFx,
    rgb_split: RgbSplitFx,
}

impl MyFilters {
    fn new() -> Result<Self, String> {
        let mut pixelate = PixelateFx::new(Default::default())?;
        pixelate.enabled = false;
        let mut color_replace = ColorReplaceFx::new(Default::default())?;
        color_replace.enabled = false;
        let mut gray_scale = GrayScaleFx::new(Default::default())?;
        gray_scale.enabled = false;
        let mut blur = BlurFx::new(Default::default())?;
        blur.enabled = false;
        let mut rgb_split = RgbSplitFx::new(Default::default())?;
        rgb_split.enabled = false;

        Ok(Self {
            pixelate,
            color_replace,
            gray_scale,
            blur,
            rgb_split,
        })
    }

    fn update(&mut self) -> Result<(), String> {
        let elapsed = time::elapsed_f32();

        // Update pixelate's pixel size
        self.pixelate.params.size = Vec2::splat(10.0 + elapsed.sin());

        // Update color_replace out color
        let r = elapsed.sin() * 0.5 + 0.5;
        let g = elapsed.cos() * 0.5 + 0.5;
        self.color_replace.params.in_color = Color::rgba_u8(100, 126, 191, 255);
        self.color_replace.params.out_color = Color::rgb(r, g, 0.0);
        self.color_replace.params.tolerance = 0.5;

        // Update grayscale factor
        self.gray_scale.params.factor = elapsed.sin() * 0.5 + 0.5;

        // Blur strength
        self.blur.params.strength = (elapsed.cos() * 0.5 + 0.5) * 8.0;

        // RGBSplit values
        self.rgb_split.params.red = vec2(-10.0 + elapsed.sin() * 20.0, 0.0);
        self.rgb_split.params.green = vec2(0.0, -10.0 + elapsed.cos() * 20.0);
        self.rgb_split.params.green =
            vec2(-10.0 + elapsed.cos() * 20.0, -10.0 + elapsed.sin() * 20.0);

        // Now we need to upload to the gpu the changes made in the params
        self.pixelate.update()?;
        self.color_replace.update()?;
        self.gray_scale.update()?;
        self.blur.update()?;
        self.rgb_split.update()?;

        Ok(())
    }

    fn filters(&self) -> [&dyn PostFx; 5] {
        [
            &self.gray_scale,
            &self.color_replace,
            &self.pixelate,
            &self.blur,
            &self.rgb_split,
        ]
    }
}

struct State {
    sprite: Sprite,
    filters: MyFilters,
}

impl State {
    fn new() -> Result<Self, String> {
        let sprite = draw::create_sprite()
            .from_image(include_bytes!("assets/ferris.png"))
            .build()?;

        let filters = MyFilters::new()?;

        Ok(Self { sprite, filters })
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(|| State::new().unwrap())
        .update(update)
        .run()
}

fn update(s: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.1, 0.2, 0.3));

    let elapsed = time::elapsed_f32() * 0.5;
    let offset = vec2(elapsed.sin(), elapsed.cos()) * 150.0;
    draw.image(&s.sprite)
        .position(window_size() * 0.5 - s.sprite.size() * 0.5 + offset);

    // Update the filters if needed
    s.filters.update().unwrap();

    // Render the PostProcess
    gfx::render_to_frame(&PostProcess::new(&draw, &s.filters.filters())).unwrap();

    draw_ui(s);
}

fn draw_ui(s: &mut State) {
    let mut draw = create_draw_2d();
    draw.text(&format!("1: Pixelate: {:?}", s.filters.pixelate.enabled))
        .position(vec2(10.0, 10.0))
        .size(12.0);

    draw.text(&format!(
        "2: ColorReplace: {:?}",
        s.filters.color_replace.enabled
    ))
    .position(vec2(10.0, 30.0))
    .size(12.0);

    draw.text(&format!("3: GrayScale: {:?}", s.filters.gray_scale.enabled))
        .position(vec2(10.0, 50.0))
        .size(12.0);

    draw.text(&format!("4: Blur: {:?}", s.filters.blur.enabled))
        .position(vec2(10.0, 70.0))
        .size(12.0);

    draw.text(&format!("5: RgbSplit: {:?}", s.filters.rgb_split.enabled))
        .position(vec2(10.0, 90.0))
        .size(12.0);

    draw.text(&format!("MS: {:.4}", time::delta_f32()))
        .anchor(vec2(0.0, 1.0))
        .translate(vec2(10.0, window_height() - 10.0))
        .size(10.0);

    gfx::render_to_frame(&draw).unwrap();

    if is_key_pressed(KeyCode::Digit1) {
        s.filters.pixelate.enabled = !s.filters.pixelate.enabled;
    }

    if is_key_pressed(KeyCode::Digit2) {
        s.filters.color_replace.enabled = !s.filters.color_replace.enabled;
    }

    if is_key_pressed(KeyCode::Digit3) {
        s.filters.gray_scale.enabled = !s.filters.gray_scale.enabled;
    }

    if is_key_pressed(KeyCode::Digit4) {
        s.filters.blur.enabled = !s.filters.blur.enabled;
    }

    if is_key_pressed(KeyCode::Digit5) {
        s.filters.rgb_split.enabled = !s.filters.rgb_split.enabled;
    }
}
