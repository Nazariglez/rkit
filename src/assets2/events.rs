use std::marker::PhantomData;

use bevy_ecs::event::Event;

use crate::assets2::AutoLoad;

#[derive(Clone, Debug)]
pub(crate) struct AssetLoad {
    pub(crate) id: String,
    pub(crate) state: LoadState,
}

#[allow(unused)]
#[derive(Clone, Debug)]
pub(crate) enum LoadState {
    Loading,
    Loaded(Vec<u8>),
    Err(String),
}

#[derive(Event)]
pub struct AutoLoadEvt<T>
where
    T: AutoLoad,
{
    _marker: PhantomData<T>,
}

impl<T> Default for AutoLoadEvt<T>
where
    T: AutoLoad,
{
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

// #[derive(Event)]
// pub struct LoadListEvt {
//     pub id: String,
// }
