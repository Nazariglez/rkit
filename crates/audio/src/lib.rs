use crate::manager::MANAGER;
pub use crate::manager::{Manager, PlayOptions};
pub use crate::sound::{AsSoundInstance, Sound, SoundId, SoundInstance};

mod manager;
mod sound;

// TODO return PlayBuilder and StopBuilder to have options like delay, volume, etc...

#[inline]
pub fn create_sound(bytes: &[u8]) -> Result<Sound, String> {
    MANAGER.borrow_mut().create_sound(bytes)
}

#[inline]
pub fn create_sound_instance(sound: &Sound) -> SoundInstance {
    MANAGER.borrow_mut().create_sound_instance(sound)
}

#[inline]
pub fn play_sound<S: AsSoundInstance>(sound: &S) -> AudioPlay {
    AudioPlay::new(sound.as_instance())
}

#[inline]
pub fn pause_sound<S: AsSoundInstance>(sound: &S) {
    MANAGER.borrow_mut().pause_sound(sound.as_instance());
}

#[inline]
pub fn resume_sound<S: AsSoundInstance>(sound: &S) {
    MANAGER.borrow_mut().resume_sound(sound.as_instance());
}

#[inline]
pub fn stop_sound<S: AsSoundInstance>(sound: &S) {
    MANAGER.borrow_mut().stop_sound(sound.as_instance());
}

#[inline]
pub fn set_sound_volume<S: AsSoundInstance>(sound: &S, vol: f32) {
    MANAGER
        .borrow_mut()
        .set_sound_volume(sound.as_instance(), vol);
}

#[inline]
pub fn sound_volume<S: AsSoundInstance>(sound: &S) -> f32 {
    MANAGER
        .borrow()
        .sound_volume(sound.as_instance())
        .unwrap_or_default()
}

#[inline]
pub fn set_sound_pitch<S: AsSoundInstance>(sound: &S, pitch: f32) {
    MANAGER
        .borrow_mut()
        .set_sound_pitch(sound.as_instance(), pitch);
}

#[inline]
pub fn sound_pitch<S: AsSoundInstance>(sound: &S) -> f32 {
    MANAGER
        .borrow()
        .sound_pitch(sound.as_instance())
        .unwrap_or_default()
}

#[inline]
pub fn set_sound_panning<S: AsSoundInstance>(sound: &S, panning: f32) {
    MANAGER
        .borrow_mut()
        .set_sound_panning(sound.as_instance(), panning);
}

#[inline]
pub fn sound_panning<S: AsSoundInstance>(sound: &S) -> f32 {
    MANAGER
        .borrow()
        .sound_panning(sound.as_instance())
        .unwrap_or_default()
}

#[inline]
pub fn is_sound_playing<S: AsSoundInstance>(sound: &S) -> bool {
    MANAGER
        .borrow()
        .is_playing(sound.as_instance())
        .unwrap_or_default()
}

#[inline]
pub fn is_sound_paused<S: AsSoundInstance>(sound: &S) -> bool {
    MANAGER
        .borrow()
        .is_paused(sound.as_instance())
        .unwrap_or_default()
}

#[inline]
pub fn sound_progress<S: AsSoundInstance>(sound: &S) -> f32 {
    MANAGER
        .borrow()
        .sound_progress(sound.as_instance())
        .unwrap_or_default()
}

// TODO set_sound_pitch and set_sound_pan?

#[inline]
pub fn set_global_volume(v: f32) {
    MANAGER.borrow_mut().set_volume(v);
}

#[inline]
pub fn global_volume() -> f32 {
    MANAGER.borrow().volume
}

/// Used by the system to clean after the frame ends
#[inline]
pub(crate) fn clean_audio_manager() {
    MANAGER.borrow_mut().clean();
}

pub struct AudioPlay {
    instance: Option<SoundInstance>,
    opts: PlayOptions,
}

impl AudioPlay {
    pub fn new(instance: SoundInstance) -> Self {
        Self {
            instance: Some(instance),
            opts: Default::default(),
        }
    }

    pub fn volume(mut self, vol: f32) -> Self {
        self.opts.volume = vol.clamp(0.0, 1.0);
        self
    }

    pub fn repeat(mut self, val: bool) -> Self {
        self.opts.repeat = val;
        self
    }

    pub fn pitch(mut self, speed: f32) -> Self {
        self.opts.pitch = speed;
        self
    }

    pub fn panning(mut self, panning: f32) -> Self {
        self.opts.panning = panning.clamp(0.0, 1.0);
        self
    }
}

impl Drop for AudioPlay {
    fn drop(&mut self) {
        debug_assert!(
            self.instance.is_some(),
            "Instance must exists always on drop. This should be unreachable."
        );
        let instance = self.instance.take().unwrap();
        let opts = self.opts;
        MANAGER.borrow_mut().play_sound(instance, opts);
    }
}
