use super::waker::*;
use crate::events::{AssetLoad, AssetState};
use crate::load_file::FileLoader;
use atomic_refcell::AtomicRefCell;
use futures::future::LocalBoxFuture;
use futures::task::{Context, Poll};
use futures_util::future::BoxFuture;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use thunderdome::{Arena, Index};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AssetId(Index);

// TODO url loader
pub(crate) static ASSET_LOADER: Lazy<AtomicRefCell<AssetLoader>> =
    Lazy::new(|| AtomicRefCell::new(AssetLoader::new()));

pub(crate) struct AssetLoader {
    loading: Vec<LoadWrapper>,
    file_loader: FileLoader,
    states: Arena<AssetLoad>,
}

impl AssetLoader {
    fn new() -> Self {
        Self {
            loading: vec![],
            file_loader: FileLoader::new().unwrap(),
            states: Arena::default(),
        }
    }

    pub fn is_loaded(&self, id: AssetId) -> bool {
        self.states
            .get(id.0)
            .map_or(false, |s| matches!(s.state, AssetState::Loaded(_)))
    }

    pub fn is_loading(&self, id: AssetId) -> bool {
        self.states
            .get(id.0)
            .map_or(false, |s| matches!(s.state, AssetState::Loading))
    }

    // pub(crate) fn get(&self, id: AssetId) -> Result<Option<Vec<u8>>, String> {
    //     let loaded = self.states.get(id.0)
    //         .ok_or_else(|| "Invalid AssetID".to_string())?;
    //
    //     match &loaded.state {
    //         AssetState::Loading => Ok(None),
    //         AssetState::Loaded(d) => Ok(Some(d.clone())),
    //         AssetState::Err(err) => Err(err.to_string())
    //     }
    // }

    pub(crate) fn take(&mut self, id: AssetId) -> Result<Option<Vec<u8>>, String> {
        let loaded = self
            .states
            .remove(id.0)
            .ok_or_else(|| "Invalid AssetID".to_string())?;

        match loaded.state {
            AssetState::Loading => Ok(None),
            AssetState::Loaded(d) => Ok(Some(d)),
            AssetState::Err(err) => Err(err),
        }
    }

    pub(crate) fn parse<T, F>(
        &mut self,
        id: AssetId,
        parser: F,
        keep: bool,
    ) -> Result<Option<T>, String>
    where
        F: FnOnce(&str, &[u8]) -> Result<T, String>,
    {
        let loaded = self
            .states
            .get(id.0)
            .ok_or_else(|| "Invalid AssetID".to_string())?;

        let (remove, res) = match &loaded.state {
            AssetState::Loading => (false, Ok(None)),
            AssetState::Loaded(d) => (!keep, Ok(Some(parser(&loaded.id, d.as_slice())?))),
            AssetState::Err(err) => (!keep, Err(err.to_string())),
        };

        if remove {
            let _ = self.states.remove(id.0);
        }

        res
    }

    pub(crate) fn update(&mut self) {
        let mut needs_clean = true;
        self.loading.iter_mut().for_each(|loader| {
            let asset_state = self.states.get_mut(loader.id.0).unwrap();
            if let Some(state) = loader.try_load(&asset_state.id) {
                asset_state.state = state;
                needs_clean = true;
            }
        });

        if needs_clean {
            self.loading.retain(|loader| !loader.is_loaded());
        }
    }

    pub(crate) fn load(&mut self, file_path: &str) -> AssetId {
        log::info!("Loading file '{}'", file_path);
        let fut = Box::pin(self.file_loader.load_file(file_path));
        let idx = self.states.insert(AssetLoad {
            id: file_path.to_string(),
            state: AssetState::Loading,
        });
        let id = AssetId(idx);
        let wrapper = LoadWrapper::new(id, fut);
        self.loading.push(wrapper);
        id
    }

    pub(crate) fn clean(&mut self) {
        self.states.clear();
    }
}

struct LoadWrapper {
    id: AssetId,
    fut: Arc<Mutex<BoxFuture<'static, Result<Vec<u8>, String>>>>,
    loaded: bool,
}

impl LoadWrapper {
    pub fn new(id: AssetId, fut: BoxFuture<'static, Result<Vec<u8>, String>>) -> Self {
        Self {
            id,
            fut: Arc::new(Mutex::new(fut)),
            loaded: false,
        }
    }

    pub fn try_load(&mut self, id: &str) -> Option<AssetState> {
        let waker = DummyWaker.into_task_waker();
        let mut ctx = Context::from_waker(&waker);
        match self.fut.lock().as_mut().poll(&mut ctx) {
            Poll::Ready(r_buff) => {
                self.loaded = true;
                match r_buff {
                    Ok(buff) => Some(AssetState::Loaded(buff)),
                    Err(err) => {
                        let err = format!("Cannot load file: {}: {}", id, err);
                        log::warn!("{}", err);
                        Some(AssetState::Err(err))
                    }
                }
            }
            _ => None,
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }
}
