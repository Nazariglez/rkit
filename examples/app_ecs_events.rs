use rkit::{prelude::*, random};

#[derive(Message)]
struct MyCustomEvent {
    msg: String,
}

fn main() -> Result<(), String> {
    App::new()
        .add_event::<MyCustomEvent>()
        .add_plugin(MainPlugins::default())
        .add_systems(OnUpdate, (send_event_system, receive_event_system).chain())
        .run()
}

fn send_event_system(mut writer: MessageWriter<MyCustomEvent>) {
    let rng = random::r#gen::<f32>();
    if rng <= 0.95 {
        return;
    }

    writer.send(MyCustomEvent {
        msg: format!("Random with value higher than 0.95: '{rng:.3}'"),
    });
}

fn receive_event_system(mut reader: MessageReader<MyCustomEvent>) {
    for evt in reader.read() {
        println!("{}", evt.msg);
    }
}
