use crate::manager::MANAGER;
pub use crate::sound::{AsSoundInstance, Sound, SoundInstance};

mod manager;
mod sound;

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
    // TODO play_sound with volume
    let instance = sound.as_instance();
    MANAGER.borrow_mut().play_sound(instance);
}

#[inline]
pub fn pause_sound<S: AsSoundInstance>(sound: &S) {
    todo!()
}

#[inline]
pub fn resume_sound<S: AsSoundInstance>(sound: &S) {
    todo!()
}

#[inline]
pub fn stop_sound<S: AsSoundInstance>(sound: &S) {
    todo!()
}

#[inline]
pub fn set_sound_volume<S: AsSoundInstance>(sound: &S) {
    todo!()
}

#[inline]
pub fn sound_volume<S: AsSoundInstance>(sound: &S) -> f32 {
    todo!()
}

#[inline]
pub fn is_sound_playing<S: AsSoundInstance>(sound: &S) {
    todo!()
}

#[inline]
pub fn is_sound_paused<S: AsSoundInstance>(sound: &S) {
    todo!()
}

// TODO set_sound_pitch and set_sound_pan?

#[inline]
pub fn set_global_volume(v: f32) {
    todo!()
}

#[inline]
pub fn global_volume() -> f32 {
    todo!()
}
