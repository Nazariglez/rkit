use crate::manager::MANAGER;
pub use crate::sound::{AsSoundInstance, Sound, SoundInstance};

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
pub fn play_sound<S: AsSoundInstance>(sound: &S) {
    MANAGER.borrow_mut().play_sound(sound.as_instance());
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

// TODO set_sound_pitch and set_sound_pan?

#[inline]
pub fn set_global_volume(v: f32) {
    MANAGER.borrow_mut().set_volume(v);
}

#[inline]
pub fn global_volume() -> f32 {
    MANAGER.borrow().volume
}
