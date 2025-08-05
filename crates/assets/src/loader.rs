use super::waker::*;
use crate::events::{AssetLoad, AssetState};
use crate::load_file::FileLoader;
use crate::update_assets;
use atomic_refcell::AtomicRefCell;
use futures::task::{Context, Poll};
use futures_util::future::BoxFuture;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::Arc;
use thunderdome::{Arena, Index};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AssetId(Index);

// TODO url loader
pub(crate) static ASSET_LOADER: Lazy<AtomicRefCell<AssetLoader>> = Lazy::new(|| {
    corelib::app::on_sys_pre_update(update_assets);
    AtomicRefCell::new(AssetLoader::new())
});

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
            .is_some_and(|s| matches!(s.state, AssetState::Loaded(_)))
    }

    pub fn is_loading(&self, id: AssetId) -> bool {
        self.states
            .get(id.0)
            .is_some_and(|s| matches!(s.state, AssetState::Loading))
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

        let (parsed, remove, res) = match &loaded.state {
            AssetState::Loading => (false, false, Ok(None)),
            AssetState::Loaded(d) => (true, !keep, Ok(Some(parser(&loaded.id, d.as_slice())?))),
            AssetState::Err(err) => (false, !keep, Err(err.to_string())),
        };

        if parsed {
            log::info!("Asset '{}' parsed.", &loaded.id);
        }

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
        log::debug!("Loading file '{file_path}'");
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

    pub(crate) fn clear(&mut self) {
        self.states.clear();
    }
}

type InnerBoxFuture = BoxFuture<'static, Result<Vec<u8>, String>>;

struct LoadWrapper {
    id: AssetId,
    fut: Arc<Mutex<InnerBoxFuture>>,
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
        if self.loaded {
            return None;
        }

        let waker = DummyWaker.into_task_waker();
        let mut ctx = Context::from_waker(&waker);
        match self.fut.lock().as_mut().poll(&mut ctx) {
            Poll::Ready(r_buff) => {
                self.loaded = true;
                match r_buff {
                    Ok(buff) => {
                        log::debug!("File loaded: '{id}'");
                        Some(AssetState::Loaded(buff))
                    }
                    Err(err) => {
                        let err = format!("Cannot load file: {id}: {err}");
                        log::warn!("{err}");
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
