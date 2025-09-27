use crate::audio::{Manager, PlayOptions};
use crate::ecs::app::App;
use bevy_ecs::prelude::*;

use super::{plugin::Plugin, prelude::OnEnginePostFrame};
pub use crate::audio::{AsSoundInstance, Sound, SoundId, SoundInstance};

#[derive(Default)]
pub struct AudioPlugin;
impl Plugin for AudioPlugin {
    fn apply(&self, app: &mut App) {
        app.insert_resource(Audio::default())
            .on_schedule(OnEnginePostFrame, clean_system);
    }
}

#[derive(Resource, Default)]
pub struct Audio {
    manager: Manager,
}

impl Audio {
    #[inline]
    pub fn create_sound(&mut self, bytes: &[u8]) -> Result<Sound, String> {
        self.manager.create_sound(bytes)
    }

    #[inline]
    pub fn create_instance(&mut self, snd: &Sound) -> SoundInstance {
        self.manager.create_sound_instance(snd)
    }

    #[inline]
    pub fn play<'manager, S: AsSoundInstance>(&'manager mut self, snd: &S) -> SoundPlay<'manager> {
        SoundPlay::new(&mut self.manager, snd.as_instance())
    }

    #[inline]
    pub fn stop<S: AsSoundInstance>(&mut self, snd: &S) {
        self.manager.stop_sound(snd.as_instance());
    }

    #[inline]
    pub fn pause<S: AsSoundInstance>(&mut self, snd: &S) {
        self.manager.pause_sound(snd.as_instance());
    }

    #[inline]
    pub fn resume<S: AsSoundInstance>(&mut self, snd: &S) {
        self.manager.resume_sound(snd.as_instance());
    }

    #[inline]
    pub fn set_volume<S: AsSoundInstance>(&mut self, snd: &S, vol: f32) {
        self.manager.set_sound_volume(snd.as_instance(), vol);
    }

    #[inline]
    pub fn volume<S: AsSoundInstance>(&self, snd: &S) -> f32 {
        self.manager
            .sound_volume(snd.as_instance())
            .unwrap_or_default()
    }

    #[inline]
    pub fn set_pitch<S: AsSoundInstance>(&mut self, snd: &S, pitch: f32) {
        self.manager.set_sound_pitch(snd.as_instance(), pitch);
    }

    #[inline]
    pub fn pitch<S: AsSoundInstance>(&self, snd: &S) -> f32 {
        self.manager
            .sound_pitch(snd.as_instance())
            .unwrap_or_default()
    }

    #[inline]
    pub fn set_panning<S: AsSoundInstance>(&mut self, snd: &S, panning: f32) {
        self.manager.set_sound_panning(snd.as_instance(), panning);
    }

    #[inline]
    pub fn panning<S: AsSoundInstance>(&self, snd: &S) -> f32 {
        self.manager
            .sound_panning(snd.as_instance())
            .unwrap_or_default()
    }

    #[inline]
    pub fn is_playing<S: AsSoundInstance>(&self, snd: &S) -> bool {
        self.manager
            .is_playing(snd.as_instance())
            .unwrap_or_default()
    }

    #[inline]
    pub fn is_paused<S: AsSoundInstance>(&self, snd: &S) -> bool {
        self.manager
            .is_paused(snd.as_instance())
            .unwrap_or_default()
    }

    #[inline]
    pub fn progress<S: AsSoundInstance>(&self, snd: &S) -> f32 {
        self.manager
            .sound_progress(snd.as_instance())
            .unwrap_or_default()
    }

    #[inline]
    pub fn set_global_volume(&mut self, vol: f32) {
        self.manager.set_volume(vol);
    }

    #[inline]
    pub fn global_volume(&mut self) -> f32 {
        self.manager.volume()
    }

    #[inline]
    pub fn mute(&mut self, mute: bool) {
        self.manager.mute(mute);
    }

    #[inline]
    pub fn is_muted(&self) -> bool {
        self.manager.is_muted()
    }
}

fn clean_system(audio: Option<ResMut<Audio>>) {
    let Some(mut audio) = audio else {
        return;
    };

    audio.manager.clean();
}

pub struct SoundPlay<'manager> {
    instance: Option<SoundInstance>,
    manager: &'manager mut Manager,
    opts: PlayOptions,
}

impl<'manager> SoundPlay<'manager> {
    fn new(manager: &'manager mut Manager, instance: SoundInstance) -> Self {
        Self {
            instance: Some(instance),
            manager,
            opts: Default::default(),
        }
    }

    #[inline]
    pub fn start_at(mut self, secs: f32) -> Self {
        self.opts.start_at = Some(secs);
        self
    }

    #[inline]
    pub fn volume(mut self, vol: f32) -> Self {
        self.opts.volume = vol.clamp(0.0, 1.0);
        self
    }

    #[inline]
    pub fn repeat(mut self, val: bool) -> Self {
        self.opts.repeat = val;
        self
    }

    #[inline]
    pub fn pitch(mut self, speed: f32) -> Self {
        self.opts.pitch = speed;
        self
    }

    #[inline]
    pub fn panning(mut self, panning: f32) -> Self {
        self.opts.panning = panning.clamp(0.0, 1.0);
        self
    }
}

impl Drop for SoundPlay<'_> {
    fn drop(&mut self) {
        debug_assert!(
            self.instance.is_some(),
            "Instance must exists always on drop. This should be unreachable."
        );
        let instance = self.instance.take().unwrap();
        let opts = self.opts;
        self.manager.play_sound(instance, opts);
    }
}
