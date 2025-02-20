use crate::sound::{InstanceId, SoundId};
use crate::{clean_audio_manager, Sound, SoundInstance};
use atomic_refcell::AtomicRefCell;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings};
use kira::sound::PlaybackState;
use kira::{
    AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Panning, PlaybackRate, Tween,
};
use num::Zero;
use once_cell::sync::Lazy;
use rustc_hash::{FxBuildHasher, FxHashMap};
use smallvec::SmallVec;

pub(crate) static MANAGER: Lazy<AtomicRefCell<Manager>> = Lazy::new(|| {
    corelib::app::on_sys_post_update(clean_audio_manager);
    AtomicRefCell::new(Manager::default())
});

struct InstanceData {
    id: u64,
    raw: StaticSoundData,
    handle: StaticSoundHandle,
    volume: f32,
    pitch: f32,
    panning: f32,
}

impl InstanceData {
    fn is_stopped(&self) -> bool {
        matches!(self.handle.state(), PlaybackState::Stopped)
    }
}

pub struct Manager {
    count_ids: u64,
    manager: AudioManager,
    instances: FxHashMap<SoundId, SmallVec<InstanceData, 10>>,
    pub(crate) volume: f32,
}

impl Default for Manager {
    fn default() -> Self {
        let manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map_err(|e| format!("Cannot initialize audio backend: {:?}", e.to_string()))
            .unwrap();

        Self {
            count_ids: 0,
            manager,
            instances: FxHashMap::with_capacity_and_hasher(10, FxBuildHasher),
            volume: 1.0,
        }
    }
}

impl Manager {
    pub fn create_sound(&mut self, bytes: &[u8]) -> Result<Sound, String> {
        create_sound_from_bytes(self.next_id(), bytes)
    }

    pub fn create_sound_instance(&mut self, snd: &Sound) -> SoundInstance {
        let id = InstanceId::Local(self.next_id());
        SoundInstance {
            id,
            snd: snd.clone(),
        }
    }

    pub fn play_sound(&mut self, instance: SoundInstance, opts: PlayOptions) {
        // if the instance is global then we assign a new id for the current instance
        let id = match instance.id {
            InstanceId::Global => self.next_id(),
            InstanceId::Local(id) => id,
        };

        // If the sound is in progress get the list if not create the list
        let list = self.instances.entry(instance.snd.id).or_default();

        // Check if an instance with the same id already exists in the list
        let data_opt = list.iter_mut().find(|data| data.id == id);
        if let Some(data) = data_opt {
            if matches!(data.handle.state(), PlaybackState::Playing) {
                return;
            }

            match self.manager.play(data.raw.with_settings(opts.into())) {
                Ok(handle) => {
                    data.handle = handle;
                    data.volume = opts.volume;
                    data.pitch = opts.volume;
                    data.panning = opts.volume;
                }
                Err(e) => {
                    log::error!("Error playing sound: {}", e.to_string());
                }
            }
            return;
        }

        // No instance with this id found, so create and insert a new one
        match self
            .manager
            .play(instance.snd.raw.with_settings(opts.into()))
        {
            Ok(handle) => {
                let data = InstanceData {
                    id,
                    raw: instance.snd.raw,
                    handle,
                    volume: opts.volume,
                    pitch: opts.pitch,
                    panning: opts.panning,
                };
                list.push(data);
            }
            Err(e) => {
                log::error!("Error playing sound: {}", e.to_string());
            }
        }
    }

    pub fn stop_sound(&mut self, instance: SoundInstance) {
        let Some(list) = self.instances.get_mut(&instance.snd.id) else {
            return;
        };

        match instance.id {
            InstanceId::Global => {
                list.iter_mut().for_each(|d| {
                    d.handle.stop(Tween::default());
                });
                list.clear();
            }
            InstanceId::Local(id) => {
                let Some(idx) = list.iter().position(|d| d.id == id) else {
                    return;
                };

                let mut data = list.remove(idx);
                data.handle.stop(Tween::default());
            }
        }
    }

    pub fn pause_sound(&mut self, instance: SoundInstance) {
        let Some(list) = self.instances.get_mut(&instance.snd.id) else {
            return;
        };

        match instance.id {
            InstanceId::Global => {
                list.iter_mut().for_each(|d| {
                    d.handle.pause(Tween::default());
                });
            }
            InstanceId::Local(id) => {
                if let Some(data) = list.iter_mut().find(|d| d.id == id) {
                    data.handle.pause(Tween::default());
                }
            }
        }
    }

    pub fn resume_sound(&mut self, instance: SoundInstance) {
        let Some(list) = self.instances.get_mut(&instance.snd.id) else {
            return;
        };

        match instance.id {
            InstanceId::Global => {
                list.iter_mut().for_each(|d| {
                    d.handle.resume(Tween::default());
                });
            }
            InstanceId::Local(id) => {
                if let Some(data) = list.iter_mut().find(|d| d.id == id) {
                    data.handle.resume(Tween::default());
                }
            }
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.manager
            .main_track()
            .set_volume(decibels_from_volume(self.volume), Tween::default());
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn is_playing(&self, instance: SoundInstance) -> Option<bool> {
        self.instances.get(&instance.snd.id)?.iter().find_map(|d| {
            let check = match instance.id {
                InstanceId::Global => true,
                InstanceId::Local(id) => id == d.id,
            };

            check.then(|| matches!(d.handle.state(), PlaybackState::Playing))
        })
    }

    pub fn is_paused(&self, instance: SoundInstance) -> Option<bool> {
        self.instances.get(&instance.snd.id)?.iter().find_map(|d| {
            let check = match instance.id {
                InstanceId::Global => true,
                InstanceId::Local(id) => id == d.id,
            };

            check.then(|| matches!(d.handle.state(), PlaybackState::Paused))
        })
    }

    pub fn set_sound_volume(&mut self, instance: SoundInstance, vol: f32) {
        let Some(list) = self.instances.get_mut(&instance.snd.id) else {
            return;
        };

        let vol = vol.clamp(0.0, 1.0);
        match instance.id {
            InstanceId::Global => {
                list.iter_mut().for_each(|d| {
                    d.handle
                        .set_volume(decibels_from_volume(vol), Tween::default());
                    d.volume = vol;
                });
            }
            InstanceId::Local(id) => {
                if let Some(data) = list.iter_mut().find(|d| d.id == id) {
                    data.handle
                        .set_volume(decibels_from_volume(vol), Tween::default());
                    data.volume = vol;
                }
            }
        }
    }

    pub fn sound_volume(&self, instance: SoundInstance) -> Option<f32> {
        self.instances.get(&instance.snd.id)?.iter().find_map(|d| {
            let check = match instance.id {
                InstanceId::Global => true,
                InstanceId::Local(id) => id == d.id,
            };

            check.then_some(d.volume)
        })
    }

    pub fn set_sound_pitch(&mut self, instance: SoundInstance, pitch: f32) {
        let Some(list) = self.instances.get_mut(&instance.snd.id) else {
            return;
        };

        match instance.id {
            InstanceId::Global => {
                list.iter_mut().for_each(|d| {
                    d.handle
                        .set_playback_rate(PlaybackRate(pitch as _), Tween::default());
                    d.pitch = pitch;
                });
            }
            InstanceId::Local(id) => {
                if let Some(data) = list.iter_mut().find(|d| d.id == id) {
                    data.handle
                        .set_playback_rate(PlaybackRate(pitch as _), Tween::default());
                    data.pitch = pitch;
                }
            }
        }
    }

    pub fn sound_pitch(&self, instance: SoundInstance) -> Option<f32> {
        self.instances.get(&instance.snd.id)?.iter().find_map(|d| {
            let check = match instance.id {
                InstanceId::Global => true,
                InstanceId::Local(id) => id == d.id,
            };

            check.then_some(d.pitch)
        })
    }

    pub fn set_sound_panning(&mut self, instance: SoundInstance, panning: f32) {
        let Some(list) = self.instances.get_mut(&instance.snd.id) else {
            return;
        };

        let panning = panning.clamp(0.0, 1.0);
        match instance.id {
            InstanceId::Global => {
                list.iter_mut().for_each(|d| {
                    d.handle.set_panning(Panning(panning), Tween::default());
                    d.panning = panning;
                });
            }
            InstanceId::Local(id) => {
                if let Some(data) = list.iter_mut().find(|d| d.id == id) {
                    data.handle.set_panning(Panning(panning), Tween::default());
                    data.panning = panning;
                }
            }
        }
    }

    pub fn sound_panning(&self, instance: SoundInstance) -> Option<f32> {
        self.instances.get(&instance.snd.id)?.iter().find_map(|d| {
            let check = match instance.id {
                InstanceId::Global => true,
                InstanceId::Local(id) => id == d.id,
            };

            check.then_some(d.panning)
        })
    }

    pub fn sound_progress(&self, instance: SoundInstance) -> Option<f32> {
        self.instances.get(&instance.snd.id)?.iter().find_map(|d| {
            let check = match instance.id {
                InstanceId::Global => true,
                InstanceId::Local(id) => id == d.id,
            };

            check.then(|| {
                let duration = d.raw.duration().as_secs_f64();
                let position = d.handle.position();
                if duration.is_zero() {
                    0.0
                } else {
                    (position / duration) as f32
                }
            })
        })
    }

    fn next_id(&mut self) -> u64 {
        let id = self.count_ids;
        self.count_ids += 1;
        id
    }

    pub fn clean(&mut self) {
        self.instances.retain(|_, v| {
            v.retain(|d| !d.is_stopped());
            !v.is_empty()
        });
    }
}

fn create_sound_from_bytes(id: u64, bytes: &[u8]) -> Result<Sound, String> {
    let raw = StaticSoundData::from_cursor(std::io::Cursor::new(bytes.to_vec()))
        .map_err(|e| e.to_string())?;

    Ok(Sound {
        id: SoundId(id),
        raw,
    })
}

#[derive(Copy, Clone)]
pub struct PlayOptions {
    pub volume: f32,
    pub repeat: bool,
    pub pitch: f32,
    pub panning: f32,
}

impl Default for PlayOptions {
    fn default() -> Self {
        Self {
            volume: 1.0,
            repeat: false,
            pitch: 1.0,
            panning: 0.5,
        }
    }
}

impl From<PlayOptions> for StaticSoundSettings {
    fn from(value: PlayOptions) -> Self {
        Self {
            start_time: Default::default(),
            start_position: Default::default(),
            loop_region: value.repeat.then_some((..).into()),
            reverse: false,
            volume: decibels_from_volume(value.volume).into(),
            playback_rate: PlaybackRate(value.pitch as _).into(),
            panning: Panning(value.panning).into(),
            fade_in_tween: None,
        }
    }
}

pub(crate) fn decibels_from_volume(amplitude: f32) -> Decibels {
    const MIN_DECIBELS: f32 = -60.0;
    if amplitude <= 0.0 {
        return Decibels(MIN_DECIBELS);
    }
    Decibels(20.0 * amplitude.log10())
}
