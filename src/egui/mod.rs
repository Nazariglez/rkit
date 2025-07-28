mod painter;

use crate::{
    app::{window_dpi_scale, window_size},
    ecs::{
        app::App,
        bevy_ecs::prelude::*,
        input::{KeyCode, Keyboard, Mouse, MouseButton},
        plugin::Plugin,
        schedules::OnPreUpdate,
        time::Time,
        window::{Window as RWindow, WindowResizeEvent},
    },
    gfx::{self, Color},
};
use egui::Event;
pub use egui::*;

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
            .add_systems(OnPreUpdate, read_input_system);
    }
}

#[derive(Resource)]
pub struct EguiContext {
    ctx: Context,
    raw_input: RawInput,
    clear_color: Option<Color>,
}

impl EguiContext {
    pub fn wants_pointer(&self) -> bool {
        self.ctx.wants_pointer_input()
    }

    pub fn wants_keyboard(&self) -> bool {
        self.ctx.wants_keyboard_input()
    }

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
        let win_size = window_size();
        let win_dpi = window_dpi_scale();
        let mut painter = painter::get_mut_egui_painter();

        self.textures_delta.set.iter().for_each(|(id, delta)| {
            painter.set_texture(*id, delta);
        });

        painter.paint(self.clear, &self.primitives, target, win_size, win_dpi)?;

        self.textures_delta.free.iter().for_each(|id| {
            painter.remove_texture(*id);
        });

        Ok(())
    }
}

fn read_input_system(
    mut ectx: ResMut<EguiContext>,
    mouse: Res<Mouse>,
    keyboard: Res<Keyboard>,
    time: Res<Time>,
    window: Res<RWindow>,
    resize_evt: EventReader<WindowResizeEvent>,
) {
    // strore zoom factor so we can calculate things later on
    let zoom_factor = ectx.ctx.zoom_factor();
    let viewport_id = ectx.raw_input.viewport_id;
    if let Some(viewport) = ectx.raw_input.viewports.get_mut(&viewport_id) {
        viewport.native_pixels_per_point = Some(window.dpi_scale());
    }

    // if the windows is resized we need to force a repaint
    if !resize_evt.is_empty() {
        ectx.ctx.request_repaint();
    }

    // increment delta time for egui
    ectx.raw_input.time = Some(time.elapsed_f32() as f64);
    ectx.raw_input.predicted_dt = time.delta_f32();

    // define the screen rect bounds
    let win_size = window.size();
    ectx.raw_input.screen_rect = Some(Rect {
        min: pos2(0.0, 0.0),
        max: egui_pos(zoom_factor, win_size.x, win_size.y),
    });

    let input = &mut ectx.raw_input;

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
        .pressed_keys()
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

    keyboard.pressed_text().iter().for_each(|text| {
        let printable = !text.is_empty() && text.chars().all(is_printable);
        if printable {
            let is_cmd = modifiers.ctrl || modifiers.mac_cmd || modifiers.command;
            if is_cmd {
                return;
            }

            input.events.push(Event::Text(text.to_string()));
        }
    });

    // mouse inputs
    let raw_pos = mouse.position();
    let mouse_pos = egui_pos(zoom_factor, raw_pos.x, raw_pos.y);

    if mouse.is_moving() {
        input.events.push(Event::PointerMoved(mouse_pos));
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

    if !input.events.is_empty() {
        ectx.ctx.request_repaint();
    }
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

// impl code from here https://github.com/hasenbanck/egui_winit_platform/blob/master/src/lib.rs#L397
#[allow(clippy::manual_range_contains)]
fn is_printable(chr: char) -> bool {
    let is_in_private_use_area = '\u{e000}' <= chr && chr <= '\u{f8ff}'
        || '\u{f0000}' <= chr && chr <= '\u{ffffd}'
        || '\u{100000}' <= chr && chr <= '\u{10fffd}';

    !is_in_private_use_area && !chr.is_ascii_control()
}
