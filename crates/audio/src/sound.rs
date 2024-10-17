use kira::sound::static_sound::StaticSoundData;

#[derive(Clone)]
pub struct SoundInstance {
    pub(crate) id: InstanceId,
    pub(crate) snd: Sound,
}

#[derive(Copy, Clone, Default, Eq, PartialEq, Hash)]
pub(crate) enum InstanceId {
    #[default]
    Global,
    Local(u64),
}

pub trait AsSoundInstance {
    fn as_instance(&self) -> SoundInstance;
}

impl AsSoundInstance for SoundInstance {
    fn as_instance(&self) -> SoundInstance {
        self.clone()
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct SoundId(pub(crate) u64);

#[derive(Clone)]
pub struct Sound {
    pub(crate) id: SoundId,
    pub(crate) raw: StaticSoundData,
}

impl Sound {
    pub fn id(&self) -> SoundId {
        self.id
    }
}

impl PartialEq<Self> for Sound {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl AsSoundInstance for Sound {
    fn as_instance(&self) -> SoundInstance {
        SoundInstance {
            id: InstanceId::Global,
            snd: self.clone(),
        }
    }
}
