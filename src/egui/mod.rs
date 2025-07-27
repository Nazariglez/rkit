use crate::{ecs::prelude::*, math::Vec2};
use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use corelib::{
    app::{window_dpi_scale, window_size},
    gfx::{
        self, BindGroup, BindGroupLayout, BindGroupLayoutRef, BindingType, BlendComponent,
        BlendFactor, BlendMode, BlendOperation, Buffer, Color, IndexFormat, RenderPipeline,
        RenderTexture, Renderer, Sampler, Texture, TextureFilter, TextureFormat, TextureWrap,
        VertexFormat,
    },
    math::{self, UVec2},
};
pub use egui::*;
use egui::{Event, epaint::ImageDelta};
use encase::{ShaderType, UniformBuffer};
use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;

static EGUI_PAINTER: Lazy<AtomicRefCell<EguiPainter>> =
    Lazy::new(|| AtomicRefCell::new(EguiPainter::default()));

fn get_egui_painter() -> AtomicRef<'static, EguiPainter> {
    EGUI_PAINTER.borrow()
}

fn get_mut_egui_painter() -> AtomicRefMut<'static, EguiPainter> {
    EGUI_PAINTER.borrow_mut()
}

struct CachedTexture {
    tex: Texture,
    sampler: Sampler,
    bind: BindGroup,
}

#[derive(Default, Copy, Clone, Debug, PartialEq, ShaderType)]
struct EguiLocals {
    screen_size_in_points: Vec2,
    dithering: u32,
    _pading: u32,
}

struct EguiPainter {
    pipeline: RenderPipeline,
    vbo: Buffer,
    ebo: Buffer,
    ubo: Buffer,
    ubs: UniformBuffer<[u8; 16]>,
    ubo_bind: BindGroup,
    textures: FxHashMap<TextureId, CachedTexture>,
}

impl Default for EguiPainter {
    fn default() -> Self {
        let mut ubs = UniformBuffer::new([0; 16]);
        ubs.write(&EguiLocals::default()).unwrap();

        let ubo = gfx::create_uniform_buffer(ubs.as_ref())
            .with_label("EguiPainter UBO Transform")
            .with_write_flag(true)
            .build()
            .unwrap();

        let vbo = gfx::create_vertex_buffer(&[] as &[f32])
            .with_label("EguiPainter VBO")
            .with_write_flag(true)
            .build()
            .unwrap();

        let ebo = gfx::create_index_buffer(&[] as &[u32])
            .with_label("EguiPainter EBO")
            .with_write_flag(true)
            .build()
            .unwrap();

        let surface_formats = gfx::limits().surface_formats;
        let target_format = surface_formats
            .iter()
            // .find(|t| matches!(t, TextureFormat::Rgba8UNorm | TextureFormat::Bgra8UNorm))
            .find(|t| t.is_srgb())
            .or_else(|| surface_formats.first())
            .cloned()
            .unwrap();

        let fs_entry = if target_format.is_srgb() {
            log::warn!(
                "Detected a linear (sRGBA aware) framebuffer {target_format:?}. egui prefers Rgba8Unorm or Bgra8Unorm"
            );
            "fs_main_linear_framebuffer"
        } else {
            "fs_main_gamma_framebuffer"
        };

        log::debug!("TODO: remove me -> selected format {target_format:?} {fs_entry:?}");

        let pipeline = gfx::create_render_pipeline(include_str!("./egui.wgsl"))
            .with_label("Egui RenderPipeline")
            .with_vertex_layout(
                gfx::VertexLayout::new()
                    .with_attr(0, VertexFormat::Float32x2)
                    .with_attr(1, VertexFormat::Float32x2)
                    .with_attr(2, VertexFormat::UInt32),
            )
            .with_bind_group_layout(
                BindGroupLayout::new().with_entry(
                    BindingType::uniform(0)
                        .with_vertex_visibility(true)
                        .with_fragment_visibility(true),
                ),
            )
            .with_bind_group_layout(
                BindGroupLayout::new()
                    .with_entry(BindingType::texture(0).with_fragment_visibility(true))
                    .with_entry(BindingType::sampler(1).with_fragment_visibility(true)),
            )
            .with_index_format(IndexFormat::UInt32)
            .with_blend_mode(BlendMode {
                color: BlendComponent {
                    src: BlendFactor::One,
                    dst: BlendFactor::InverseSourceAlpha,
                    op: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src: BlendFactor::InverseDestinationAlpha,
                    dst: BlendFactor::One,
                    op: BlendOperation::Add,
                },
            })
            // .with_compatible_texture(target_format)
            .with_fragment_entry(fs_entry)
            .with_primitive(gfx::Primitive::Triangles)
            .build()
            .unwrap();

        let ubo_bind = gfx::create_bind_group()
            .with_label("EguiPainter UBO BindGroup")
            .with_layout(pipeline.bind_group_layout_ref(0).unwrap())
            .with_uniform(0, &ubo)
            .build()
            .unwrap();

        Self {
            pipeline,
            vbo,
            ebo,
            ubo,
            ubs,
            ubo_bind,
            textures: FxHashMap::default(),
        }
    }
}

fn bind_group_from(tex: &Texture, sampler: &Sampler, layout: &BindGroupLayoutRef) -> BindGroup {
    gfx::create_bind_group()
        .with_label("EguiPainter Texture BindGroup")
        .with_layout(layout)
        .with_texture(0, tex)
        .with_sampler(1, sampler)
        .build()
        .unwrap()
}

fn create_texture(data: &[u8], width: u32, height: u32) -> Texture {
    gfx::create_texture()
        .with_label("EguiPainter Texture")
        .from_bytes(data, width, height)
        // .with_format(TextureFormat::Rgba8UNorm)
        .with_write_flag(true)
        .build()
        .unwrap()
}

fn create_sampler_from(opts: &TextureOptions) -> Sampler {
    let filter = |tf| match tf {
        egui::TextureFilter::Nearest => TextureFilter::Nearest,
        egui::TextureFilter::Linear => TextureFilter::Linear,
    };
    let wrap = match opts.wrap_mode {
        egui::TextureWrapMode::ClampToEdge => TextureWrap::Clamp,
        egui::TextureWrapMode::Repeat => TextureWrap::Repeat,
        egui::TextureWrapMode::MirroredRepeat => TextureWrap::MirrorRepeat,
    };

    gfx::create_sampler()
        .with_mag_filter(filter(opts.magnification))
        .with_min_filter(filter(opts.minification))
        .with_wrap_x(wrap)
        .with_wrap_y(wrap)
        .build()
        .unwrap()
}

fn empty_texture(width: u32, height: u32) -> Texture {
    gfx::create_texture()
        .with_label("EguiPainter Texture")
        .with_empty_size(width, height)
        .with_write_flag(true)
        // .with_format(TextureFormat::Rgba8UNorm)
        .build()
        .unwrap()
}

fn update_texture(tex: &mut Texture, x: u32, y: u32, width: u32, height: u32, data: &[u8]) {
    gfx::write_texture(tex)
        .with_offset(UVec2::new(x, y))
        .with_size(UVec2::new(width, height))
        .from_data(data)
        .build()
        .unwrap();
}

impl EguiPainter {
    pub fn paint(
        &mut self,
        clear_color: Option<Color>,
        primitives: &[ClippedPrimitive],
        target: Option<&gfx::RenderTexture>,
    ) -> Result<(), String> {
        // FIXME: everthing should be done in one renderer with multuple passes

        if let Some(color) = clear_color {
            let mut renderer = Renderer::new();
            renderer.begin_pass().clear_color(color);

            gfx::render_to_frame(&renderer)?;
        }

        for ClippedPrimitive {
            clip_rect,
            primitive,
        } in primitives.iter()
        {
            //
            match primitive {
                epaint::Primitive::Mesh(mesh) => self.paint_mesh(mesh, clip_rect, target)?,
                epaint::Primitive::Callback(paint_callback) => {
                    log::warn!("Egui CALLBACK unimplemented.");
                }
            }
        }

        Ok(())
    }

    fn paint_mesh(
        &mut self,
        mesh: &Mesh,
        clip_rect: &Rect,
        target: Option<&RenderTexture>,
    ) -> Result<(), String> {
        //
        let tex_bind = self
            .textures
            .get(&mesh.texture_id)
            .ok_or_else(|| format!("Invalid Egui texture id '{:?}'", mesh.texture_id))?;

        gfx::write_buffer(&self.vbo)
            .with_data(&mesh.vertices)
            .build()?;

        gfx::write_buffer(&self.ebo)
            .with_data(&mesh.indices)
            .build()?;

        let target_size = target.map_or_else(window_size, |t| t.size());
        let [sx, sy, sw, sh] = scissor_rect(clip_rect, window_dpi_scale(), target_size);
        if sw == 0 || sh == 0 {
            return Ok(());
        }

        let mut renderer = Renderer::new();
        renderer
            .begin_pass()
            // .scissors(sx, sy, sw, sh)
            .pipeline(&self.pipeline)
            .bindings(&[&self.ubo_bind, &tex_bind.bind])
            .buffers(&[&self.vbo, &self.ebo])
            .draw(0..mesh.indices.len() as u32);

        match target {
            Some(rt) => gfx::render_to_texture(rt, &renderer),
            None => gfx::render_to_frame(&renderer),
        }
    }

    // pub fn add_texture(&mut self, tex: &Texture) -> SizedTexture {
    // // TODO: instead of texture we may need a sprite to know which sampler to use?
    // // similar to draw2d?
    //
    //     let id = TextureId::User(tex.id().into());
    //     let size = tex.size();
    //     self.textures.insert(id, tex.clone());
    //     SizedTexture {
    //         id,
    //         size: egui::Vec2::new(size.x, size.y),
    //     }
    // }

    pub fn set_texture(&mut self, id: TextureId, delta: &ImageDelta) {
        let [width, height] = delta.image.size();

        // update texture
        if let Some([x, y]) = delta.pos {
            let cached = self.textures.entry(id).or_insert_with(|| {
                let sampler = create_sampler_from(&delta.options);
                let tex = empty_texture(width as _, height as _);
                let bind = bind_group_from(
                    &tex,
                    &sampler,
                    self.pipeline.bind_group_layout_ref(1).unwrap(),
                );
                CachedTexture { tex, sampler, bind }
            });

            match &delta.image {
                ImageData::Color(image) => {
                    debug_assert_eq!(
                        image.width() * image.height(),
                        image.pixels.len(),
                        "Mismatch between texture size and texel count"
                    );

                    let data = bytemuck::cast_slice(&image.pixels);
                    update_texture(
                        &mut cached.tex,
                        x as _,
                        y as _,
                        width as _,
                        height as _,
                        data,
                    );
                }
            }

            return;
        }

        // create a new texture
        let tex = match &delta.image {
            egui::ImageData::Color(image) => {
                debug_assert_eq!(
                    image.width() * image.height(),
                    image.pixels.len(),
                    "Mismatch between texture size and texel count"
                );

                let data = bytemuck::cast_slice(&image.pixels);
                create_texture(data, width as _, height as _)
            }
        };

        let sampler = create_sampler_from(&delta.options);
        let bind = bind_group_from(
            &tex,
            &sampler,
            self.pipeline.bind_group_layout_ref(1).unwrap(),
        );
        self.textures
            .insert(id, CachedTexture { tex, sampler, bind });
    }

    pub fn remove_texture(&mut self, id: impl Into<TextureId>) {
        self.textures.remove(&id.into());
    }
}

fn scissor_rect(clip_rect: &egui::Rect, pixels_per_point: f32, target_size: Vec2) -> [u32; 4] {
    let t_size = (target_size * pixels_per_point).as_uvec2();

    let clip_min_x = pixels_per_point * clip_rect.min.x;
    let clip_min_y = pixels_per_point * clip_rect.min.y;
    let clip_max_x = pixels_per_point * clip_rect.max.x;
    let clip_max_y = pixels_per_point * clip_rect.max.y;

    let clip_min_x = clip_min_x.round() as u32;
    let clip_min_y = clip_min_y.round() as u32;
    let clip_max_x = clip_max_x.round() as u32;
    let clip_max_y = clip_max_y.round() as u32;

    let clip_min_x = clip_min_x.clamp(0, t_size.x);
    let clip_min_y = clip_min_y.clamp(0, t_size.y);
    let clip_max_x = clip_max_x.clamp(clip_min_x, t_size.x);
    let clip_max_y = clip_max_y.clamp(clip_min_y, t_size.y);

    let x = clip_min_x;
    let y = clip_min_y;
    let width = clip_max_x - clip_min_x;
    let height = clip_max_y - clip_min_y;

    [x, y, width, height]
}

#[derive(Debug, Default)]
pub struct EguiPlugin {}

impl Plugin for EguiPlugin {
    fn apply(&self, app: &mut App) {
        let ctx = EguiContext {
            ctx: Context::default(),
            raw_input: RawInput::default(),
            clear_color: None,
        };

        app.add_resource(ctx)
            .add_systems(OnPreRender, read_input_system);
    }
}

#[derive(Resource)]
pub struct EguiContext {
    ctx: Context,
    raw_input: RawInput,
    clear_color: Option<Color>,
}

impl EguiContext {
    pub fn clear(&mut self, color: Color) -> &mut Self {
        self.clear_color = Some(color);
        self
    }

    pub fn run<F>(&mut self, cb: F) -> EguiDraw
    where
        F: FnMut(&Context),
    {
        let FullOutput {
            platform_output: _,
            textures_delta,
            shapes,
            pixels_per_point,
            viewport_output,
        } = self.ctx.run(self.raw_input.take(), cb);
        let needs_update_textures = !textures_delta.is_empty();
        let needs_repaint = needs_update_textures
            || viewport_output
                .values()
                .any(|out| out.repaint_delay.is_zero());

        let primitives = self.ctx.tessellate(shapes, pixels_per_point);

        // TODO: process platform_output

        EguiDraw {
            clear: self.clear_color,
            ctx: self.ctx.clone(),
            textures_delta,
            primitives,
            needs_repaint,
            pixels_per_point,
        }
    }
}

pub struct EguiDraw {
    clear: Option<Color>,
    ctx: Context,
    textures_delta: TexturesDelta,
    primitives: Vec<ClippedPrimitive>,
    needs_repaint: bool,
    pixels_per_point: f32,
}

impl gfx::AsRenderer for EguiDraw {
    fn render(&self, target: Option<&gfx::RenderTexture>) -> Result<(), String> {
        let mut painter = get_mut_egui_painter();

        let screen_size = target.map(|t| t.size()).unwrap_or_else(window_size);

        // let screen_size = screen_size / window_dpi_scale();

        painter
            .ubs
            .write(&EguiLocals {
                screen_size_in_points: screen_size,
                dithering: u32::from(false),
                _pading: 0,
            })
            .map_err(|e| e.to_string())?;

        gfx::write_buffer(&painter.ubo)
            .with_data(painter.ubs.as_ref())
            .build()?;

        self.textures_delta.set.iter().for_each(|(id, delta)| {
            painter.set_texture(*id, delta);
        });

        painter.paint(self.clear, &self.primitives, target)?;

        self.textures_delta.free.iter().for_each(|id| {
            painter.remove_texture(*id);
        });

        Ok(())
    }
}

fn read_input_system(
    mouse: Res<Mouse>,
    keyboard: Res<Keyboard>,
    mut ctx: ResMut<EguiContext>,
    time: Res<Time>,
    resize_evt: EventReader<WindowResizeEvent>,
) {
    // strore zoom factor so we can calculate things later on
    let zoom_factor = ctx.ctx.zoom_factor();

    // if the windows is resized we need to force a repaint
    if !resize_evt.is_empty() {
        ctx.ctx.request_repaint();
    }

    // increment delta time for egui
    ctx.raw_input.time = Some(match ctx.raw_input.time {
        Some(t) => t + time.delta().as_secs_f64(),
        None => 0.0,
    });

    let input = &mut ctx.raw_input;

    // keybaord inputs
    let mac_cmd =
        cfg!(any(target_os = "macos", target_arch = "wasm32")) && keyboard.is_super_down();
    let modifiers = Modifiers {
        alt: keyboard.is_alt_down(),
        ctrl: keyboard.is_crtl_down(),
        shift: keyboard.is_shift_down(),
        mac_cmd,
        command: mac_cmd || keyboard.is_crtl_down(),
    };

    keyboard
        .down_keys()
        .iter()
        .filter_map(kc_to_egui_key)
        .for_each(|key| {
            input.events.push(Event::Key {
                key,
                // TODO: I this we should make this right, physical key vs logical
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            });
        });

    keyboard
        .released_keys()
        .iter()
        .filter_map(kc_to_egui_key)
        .for_each(|key| {
            input.events.push(Event::Key {
                key,
                // TODO: I this we should make this right, physical key vs logical
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers,
            });
        });

    // TODO: char/text input

    // mouse inputs
    let raw_pos = mouse.position();
    let mouse_pos = egui_pos(zoom_factor, raw_pos.x, raw_pos.y);

    if mouse.is_moving() {
        input.events.push(Event::MouseMoved(mouse_pos.to_vec2()));
    }

    if mouse.is_scrolling() {
        let wd = mouse.wheel_delta();
        if modifiers.ctrl || modifiers.command {
            let factor = (wd.y / 200.0).exp();
            input.events.push(Event::Zoom(factor));
        } else {
            input.events.push(Event::MouseWheel {
                unit: MouseWheelUnit::Point,
                delta: egui_pos(zoom_factor, wd.x, wd.y).to_vec2(),
                modifiers,
            });
        }
    }

    if mouse.just_left() {
        input.events.push(Event::PointerGone);
    }

    mouse
        .down_buttons()
        .iter()
        .filter_map(mb_to_egui_pointer)
        .for_each(|button| {
            input.events.push(Event::PointerButton {
                pos: mouse_pos,
                button,
                pressed: true,
                modifiers,
            });
        });

    mouse
        .released_buttons()
        .iter()
        .filter_map(mb_to_egui_pointer)
        .for_each(|button| {
            input.events.push(Event::PointerButton {
                pos: mouse_pos,
                button,
                pressed: false,
                modifiers,
            });
        });
}

fn kc_to_egui_key(key: KeyCode) -> Option<egui::Key> {
    use egui::Key::*;
    Some(match key {
        // Punctuation
        KeyCode::Backquote => Backtick,
        KeyCode::Backslash => Backslash,
        KeyCode::BracketLeft => OpenBracket,
        KeyCode::BracketRight => CloseBracket,
        KeyCode::Comma => Comma,
        KeyCode::Minus => Minus,
        KeyCode::Period => Period,
        KeyCode::Quote => Quote,
        KeyCode::Semicolon => Semicolon,
        KeyCode::Slash => Slash,
        KeyCode::Equal => Equals,

        // Numbers
        KeyCode::Digit0 | KeyCode::Numpad0 => Num0,
        KeyCode::Digit1 | KeyCode::Numpad1 => Num1,
        KeyCode::Digit2 | KeyCode::Numpad2 => Num2,
        KeyCode::Digit3 | KeyCode::Numpad3 => Num3,
        KeyCode::Digit4 | KeyCode::Numpad4 => Num4,
        KeyCode::Digit5 | KeyCode::Numpad5 => Num5,
        KeyCode::Digit6 | KeyCode::Numpad6 => Num6,
        KeyCode::Digit7 | KeyCode::Numpad7 => Num7,
        KeyCode::Digit8 | KeyCode::Numpad8 => Num8,
        KeyCode::Digit9 | KeyCode::Numpad9 => Num9,

        // Letters
        KeyCode::KeyA => A,
        KeyCode::KeyB => B,
        KeyCode::KeyC => C,
        KeyCode::KeyD => D,
        KeyCode::KeyE => E,
        KeyCode::KeyF => F,
        KeyCode::KeyG => G,
        KeyCode::KeyH => H,
        KeyCode::KeyI => I,
        KeyCode::KeyJ => J,
        KeyCode::KeyK => K,
        KeyCode::KeyL => L,
        KeyCode::KeyM => M,
        KeyCode::KeyN => N,
        KeyCode::KeyO => O,
        KeyCode::KeyP => P,
        KeyCode::KeyQ => Q,
        KeyCode::KeyR => R,
        KeyCode::KeyS => S,
        KeyCode::KeyT => T,
        KeyCode::KeyU => U,
        KeyCode::KeyV => V,
        KeyCode::KeyW => W,
        KeyCode::KeyX => X,
        KeyCode::KeyY => Y,
        KeyCode::KeyZ => Z,

        // Function keys
        KeyCode::F1 => F1,
        KeyCode::F2 => F2,
        KeyCode::F3 => F3,
        KeyCode::F4 => F4,
        KeyCode::F5 => F5,
        KeyCode::F6 => F6,
        KeyCode::F7 => F7,
        KeyCode::F8 => F8,
        KeyCode::F9 => F9,
        KeyCode::F10 => F10,
        KeyCode::F11 => F11,
        KeyCode::F12 => F12,
        KeyCode::F13 => F13,
        KeyCode::F14 => F14,
        KeyCode::F15 => F15,
        KeyCode::F16 => F16,
        KeyCode::F17 => F17,
        KeyCode::F18 => F18,
        KeyCode::F19 => F19,
        KeyCode::F20 => F20,
        KeyCode::F21 => F21,
        KeyCode::F22 => F22,
        KeyCode::F23 => F23,
        KeyCode::F24 => F24,
        KeyCode::F25 => F25,
        KeyCode::F26 => F26,
        KeyCode::F27 => F27,
        KeyCode::F28 => F28,
        KeyCode::F29 => F29,
        KeyCode::F30 => F30,
        KeyCode::F31 => F31,
        KeyCode::F32 => F32,
        KeyCode::F33 => F33,
        KeyCode::F34 => F34,
        KeyCode::F35 => F35,

        // Commands
        KeyCode::ArrowDown => ArrowDown,
        KeyCode::ArrowLeft => ArrowLeft,
        KeyCode::ArrowRight => ArrowRight,
        KeyCode::ArrowUp => ArrowUp,
        KeyCode::Backspace => Backspace,
        KeyCode::Enter | KeyCode::NumpadEnter => Enter,
        KeyCode::Space => Space,
        KeyCode::Tab => Tab,
        KeyCode::Escape => Escape,
        KeyCode::Insert => Insert,
        KeyCode::Delete => Delete,
        KeyCode::Home => Home,
        KeyCode::End => End,
        KeyCode::PageUp => PageUp,
        KeyCode::PageDown => PageDown,

        // Clipboard
        KeyCode::Copy => Copy,
        KeyCode::Cut => Cut,
        KeyCode::Paste => Paste,

        // Browser
        KeyCode::BrowserBack => BrowserBack,

        _ => return None,
    })
}

fn mb_to_egui_pointer(btn: MouseButton) -> Option<egui::PointerButton> {
    Some(match btn {
        MouseButton::Left => egui::PointerButton::Primary,
        MouseButton::Middle => egui::PointerButton::Middle,
        MouseButton::Right => egui::PointerButton::Secondary,
        MouseButton::Back => egui::PointerButton::Extra1,
        MouseButton::Forward => egui::PointerButton::Extra2,
        MouseButton::Unknown => return None,
    })
}

fn egui_pos(zoom_factor: f32, x: f32, y: f32) -> Pos2 {
    pos2(x, y) / zoom_factor
}
