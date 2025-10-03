use rkit::{
    draw::create_draw_2d,
    ecs::prelude::*,
    gfx::{self, Color},
    input::MouseButton,
    math::{Mat3, Vec2, vec2},
    save::{SaveFlags, clean_backups, clear_save_files, load_last_saved_file, save_data_to_file},
};
use serde::{Deserialize, Serialize};

const BACKUPS: usize = 1;
const BASE_DIR: &str = env!("CARGO_PKG_NAME");

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy)]
struct MyData {
    clicks: u64,
}

#[derive(Resource)]
struct Slot1 {
    name: &'static str,
    data: MyData,
}

#[derive(Resource)]
struct Slot2 {
    name: &'static str,
    data: MyData,
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(WindowConfigPlugin::default().title("Save Sys"))
        .insert_resource(Slot1 {
            name: "slot1",
            data: MyData::default(),
        })
        .insert_resource(Slot2 {
            name: "slot2",
            data: MyData::default(),
        })
        .on_setup(setup)
        .on_update(update)
        .on_render(draw)
        .run()
}

fn load_slot(name: &str) -> Option<MyData> {
    load_last_saved_file(BASE_DIR, name)
        .ok()
        .flatten()
        .map(|s| s.data)
}

fn save_slot(name: &str, data: &MyData) {
    let res = save_data_to_file(BASE_DIR, name, data, SaveFlags::default(), None)
        .and_then(|_| clean_backups(BASE_DIR, name, BACKUPS));
    if let Err(e) = res {
        log::error!("{e}");
    }
}

fn setup(mut slot1: ResMut<Slot1>, mut slot2: ResMut<Slot2>) {
    slot1.data = load_slot(slot1.name).unwrap_or_default();
    slot2.data = load_slot(slot2.name).unwrap_or_default();
}

fn update(
    mut slot1: ResMut<Slot1>,
    mut slot2: ResMut<Slot2>,
    mouse: Res<Mouse>,
    win: Res<Window>,
    keyboard: Res<Keyboard>,
) {
    let nuke = keyboard.just_pressed(KeyCode::Space);
    if nuke {
        match clear_save_files(BASE_DIR, None) {
            Ok(_) => {
                log::info!("Save data nuked!");
                slot1.data.clicks = 0;
                slot2.data.clicks = 0;
            }
            Err(e) => {
                log::error!("Error cleaning save files: {e}");
            }
        }
    }

    let needs_increment = mouse.just_pressed(MouseButton::Left);
    if !needs_increment {
        return;
    }

    let is_left = mouse.position().x < win.size().x * 0.5;
    if is_left {
        slot1.data.clicks = slot1.data.clicks.saturating_add(1);
        save_slot(slot1.name, &slot1.data);
    } else {
        slot2.data.clicks = slot2.data.clicks.saturating_add(1);
        save_slot(slot2.name, &slot2.data);
    }
}

fn draw(slot1: Res<Slot1>, slot2: Res<Slot2>, win: Res<Window>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.1, 0.2, 0.3));

    let win_size = win.size();
    let half_x = win_size.x * 0.5;
    draw.line(vec2(half_x, 0.0), vec2(half_x, win_size.y))
        .width(2.0);

    draw.push_matrix(Mat3::from_translation(Vec2::splat(10.0)));
    draw.text("Slot1").translate(Vec2::ZERO);
    draw.text(&format!("Clicks: {}", slot1.data.clicks))
        .size(12.0)
        .translate(vec2(0.0, 30.0));
    draw.pop_matrix();

    draw.push_matrix(Mat3::from_translation(vec2(half_x + 10.0, 10.0)));
    draw.text("Slot2").translate(Vec2::ZERO);
    draw.text(&format!("Clicks: {}", slot2.data.clicks))
        .size(12.0)
        .translate(vec2(0.0, 30.0));
    draw.pop_matrix();

    draw.text("Left click to increase value on a slot side.\nClose & Reopen and it must persist.\nSpace for nuke save data.")
        .h_align_center()
        .color(Color::rgb(0.9, 0.8, 0.7))
        .translate(win_size * 0.5)
        .origin(Vec2::splat(0.5));

    gfx::render_to_frame(&draw).unwrap();
}
