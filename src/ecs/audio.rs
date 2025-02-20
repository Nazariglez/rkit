use crate::audio::Manager;
use crate::ecs::app::App;
use bevy_ecs::prelude::*;

use super::{plugin::Plugin, prelude::OnEnginePostFrame};
pub use crate::audio::{AsSoundInstance, Sound, SoundInstance};

#[derive(Default)]
pub struct AudioPlugin;
impl Plugin for AudioPlugin {
    fn apply(self, app: App) -> App {
        app.add_resource(Audio::default())
            .add_systems(OnEnginePostFrame, clean_system)
    }
}

#[derive(Resource, Default)]
pub struct Audio {
    manager: Manager,
}

impl Audio {
    pub fn create_sound(&mut self, bytes: &[u8]) -> Result<Sound, String> {
        self.manager.create_sound(bytes)
    }

    pub fn create_instance(&mut self, snd: &Sound) -> SoundInstance {
        self.manager.create_sound_instance(snd)
    }

    pub fn play<S: AsSoundInstance>(&mut self, snd: &S) {
        self.manager
            .play_sound(snd.as_instance(), Default::default());
    }

    pub fn stop<S: AsSoundInstance>(&mut self, snd: &S) {
        self.manager.stop_sound(snd.as_instance());
    }

    pub fn pause<S: AsSoundInstance>(&mut self, snd: &S) {
        self.manager.pause_sound(snd.as_instance());
    }

    pub fn resume<S: AsSoundInstance>(&mut self, snd: &S) {
        self.manager.resume_sound(snd.as_instance());
    }

    pub fn set_volume<S: AsSoundInstance>(&mut self, snd: &S, vol: f32) {
        self.manager.set_sound_volume(snd.as_instance(), vol);
    }

    pub fn volume<S: AsSoundInstance>(&self, snd: &S) -> f32 {
        self.manager
            .sound_volume(snd.as_instance())
            .unwrap_or_default()
    }

    pub fn set_pitch<S: AsSoundInstance>(&mut self, snd: &S, pitch: f32) {
        self.manager.set_sound_pitch(snd.as_instance(), pitch);
    }

    pub fn pitch<S: AsSoundInstance>(&self, snd: &S) -> f32 {
        self.manager
            .sound_pitch(snd.as_instance())
            .unwrap_or_default()
    }

    pub fn set_panning<S: AsSoundInstance>(&mut self, snd: &S, panning: f32) {
        self.manager.set_sound_panning(snd.as_instance(), panning);
    }

    pub fn panning<S: AsSoundInstance>(&self, snd: &S) -> f32 {
        self.manager
            .sound_panning(snd.as_instance())
            .unwrap_or_default()
    }

    pub fn is_playing<S: AsSoundInstance>(&self, snd: &S) -> bool {
        self.manager
            .is_playing(snd.as_instance())
            .unwrap_or_default()
    }

    pub fn is_paused<S: AsSoundInstance>(&self, snd: &S) -> bool {
        self.manager
            .is_paused(snd.as_instance())
            .unwrap_or_default()
    }

    pub fn progress<S: AsSoundInstance>(&self, snd: &S) -> f32 {
        self.manager
            .sound_progress(snd.as_instance())
            .unwrap_or_default()
    }

    pub fn set_global_volume(&mut self, vol: f32) {
        self.manager.set_volume(vol);
    }

    pub fn global_volume(&mut self) -> f32 {
        self.manager.volume()
    }
}

fn clean_system(audio: Option<ResMut<Audio>>) {
    let Some(mut audio) = audio else {
        return;
    };

    audio.manager.clean();
}
