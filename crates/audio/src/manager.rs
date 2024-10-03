use crate::sound::{InstanceId, SoundId};
use crate::{Sound, SoundInstance};
use atomic_refcell::AtomicRefCell;
use kira::manager::error::PlaySoundError;
use kira::manager::{AudioManager, AudioManagerSettings, DefaultBackend};
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings};
use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

pub(crate) static MANAGER: Lazy<AtomicRefCell<Manager>> =
    Lazy::new(|| AtomicRefCell::new(Manager::default()));

struct InstanceData {
    id: u64,
    raw: StaticSoundData,
}

pub(crate) struct Manager {
    count_ids: u64,
    manager: AudioManager,
    instances: FxHashMap<SoundId, SmallVec<InstanceData, 5>>, // 5 seems a good number for concurrent sounds of the same type
}

impl Default for Manager {
    fn default() -> Self {
        let manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map_err(|e| format!("Cannot initialize audio backend: {:?}", e.to_string()))
            .unwrap();

        Self {
            count_ids: 0,
            manager,
            instances: FxHashMap::default(),
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

    pub fn play_sound(&mut self, instance: SoundInstance) {
        // if the instance is global then we assign a new id for the current instance
        let id = match instance.id {
            InstanceId::Global => self.next_id(),
            InstanceId::Local(id) => id,
        };

        // If the sound is in progress get the list if not create the list
        let list = self
            .instances
            .entry(instance.snd.id)
            .or_insert_with(SmallVec::new);

        // Check if an instance with the same id already exists in the list
        let data_opt = list.iter_mut().find(|data| data.id == id);
        if let Some(data) = data_opt {
            if let Err(e) = self.manager.play(data.raw.clone()) {
                log::error!("Error playing sound: {}", e.to_string());
            }
            return;
        }

        // No instance with this id found, so create and insert a new one
        let data = InstanceData {
            id,
            raw: instance.snd.raw.clone(),
        };

        list.push(data);

        // Play the new sound instance
        if let Err(e) = self.manager.play(instance.snd.raw.clone()) {
            log::error!("Error playing sound: {}", e.to_string());
        }
    }

    fn next_id(&mut self) -> u64 {
        let id = self.count_ids;
        self.count_ids += 1;
        id
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
