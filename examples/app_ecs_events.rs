use rkit::{prelude::*, random};

#[derive(Message)]
struct MyCustomEvent {
    msg: String,
}

fn main() -> Result<(), String> {
    App::new()
        .add_message::<MyCustomEvent>()
        .add_plugin(MainPlugins::default())
        .on_update((send_event_system, receive_event_system).chain())
        .run()
}

fn send_event_system(mut cmds: Commands) {
    let rng = random::r#gen::<f32>();
    if rng <= 0.95 {
        return;
    }

    cmds.queue(move |world: &mut World| {
        world.write_message(MyCustomEvent {
            msg: format!("Random with value higher than 0.95: '{rng:.3}'"),
        });
    });
}

fn receive_event_system(mut reader: MessageReader<MyCustomEvent>) {
    for evt in reader.read() {
        println!("{}", evt.msg);
    }
}
