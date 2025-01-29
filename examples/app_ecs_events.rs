use draw::create_draw_2d;
use rand::random;
use rkit::ecs::prelude::*;
use rkit::gfx::Color;
use rkit::math::{vec2, Vec2};
use rkit::{gfx, random, time};

#[derive(Event)]
struct MyCustomEvent {
    msg: String,
}

fn main() -> Result<(), String> {
    App::new()
        .add_event::<MyCustomEvent>()
        .add_systems(OnUpdate, (send_event_system, receive_event_system).chain())
        .run()
}

fn send_event_system(mut writer: EventWriter<MyCustomEvent>) {
    let rng = random::gen::<f32>();
    if rng <= 0.95 {
        return;
    }

    writer.send(MyCustomEvent {
        msg: format!("Random with value higher than 0.95: '{rng:.3}'"),
    });
}

fn receive_event_system(mut reader: EventReader<MyCustomEvent>) {
    for evt in reader.read() {
        println!("{}", evt.msg);
    }
}
