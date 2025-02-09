use crate::prelude::{App, OnEnginePreFrame, OnEngineSetup, OnPreFrame, Plugin};
use bevy_ecs::prelude::Resource;
use bevy_ecs::system::{Commands, ResMut};
use corelib::time;
use web_time::{Duration, Instant};

pub struct TimePlugin;
impl Plugin for TimePlugin {
    fn apply(self, mut app: App) -> App {
        app.add_systems(OnEngineSetup, init_time_system)
            .add_systems(OnEnginePreFrame, update_time_system)
    }
}

fn init_time_system(mut cmds: Commands) {
    cmds.insert_resource(Time {
        fps: time::fps(),
        delta: time::delta(),
        delta_f32: time::delta_f32(),
        elapsed: time::elapsed(),
        elapsed_f32: time::elapsed_f32(),
        init_time: time::init_time(),
        last_time: time::last_time(),
    });
}

fn update_time_system(mut t_res: ResMut<Time>) {
    t_res.fps = time::fps();
    t_res.delta = time::delta();
    t_res.delta_f32 = time::delta_f32();
    t_res.elapsed = time::elapsed();
    t_res.elapsed_f32 = time::elapsed_f32();
    t_res.last_time = time::last_time();
}

#[derive(Resource)]
pub struct Time {
    fps: f32,
    delta: Duration,
    delta_f32: f32,
    elapsed: Duration,
    elapsed_f32: f32,
    init_time: Instant,
    last_time: Option<Instant>,
}

impl Time {
    pub fn fps(&self) -> f32 {
        self.fps
    }

    pub fn now(&self) -> Instant {
        Instant::now()
    }

    pub fn delta(&self) -> Duration {
        self.delta
    }

    pub fn delta_f32(&self) -> f32 {
        self.delta_f32
    }

    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    pub fn elapsed_f32(&self) -> f32 {
        self.elapsed_f32
    }

    pub fn init_time(&self) -> Instant {
        self.init_time
    }

    pub fn last_time(&self) -> Option<Instant> {
        self.last_time
    }
}
