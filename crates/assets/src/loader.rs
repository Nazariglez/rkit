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

// TODO url loader
pub(crate) static ASSET_LOADER: Lazy<AtomicRefCell<AssetLoader>> =
    Lazy::new(|| AtomicRefCell::new(AssetLoader::new()));

pub(crate) struct AssetLoader {
    loading: Vec<LoadWrapper>,
    file_loader: FileLoader,
    loaded: FxHashMap<String, AssetLoad>,
}

impl AssetLoader {
    fn new() -> Self {
        Self {
            loading: vec![],
            file_loader: FileLoader::new().unwrap(),
            loaded: FxHashMap::default(),
        }
    }

    pub(crate) fn get(&self, id: &str) -> Option<&AssetLoad> {
        // self.loaded.get(id)
        todo!()
    }

    pub(crate) fn update(&mut self) {
        let mut needs_clean = true;
        let mut loading = &mut self.loading;
        loading.iter_mut().for_each(|loader| {
            if let Some(loaded) = loader.try_load() {
                self.loaded.insert(loaded.id.clone(), loaded);
                needs_clean = true;
            }
        });

        if needs_clean {
            loading.retain(|loader| !loader.is_loaded());
        }
    }

    pub(crate) fn load(&mut self, file_path: &str) {
        log::info!("Loading file '{}'", file_path);
        let fut = Box::pin(self.file_loader.load_file(file_path));
        self.loading.push(LoadWrapper::new(file_path, fut));
    }

    pub(crate) fn clean(&mut self) {
        self.loaded.clear();
    }
}

struct LoadWrapper {
    id: String,
    fut: Arc<Mutex<BoxFuture<'static, Result<Vec<u8>, String>>>>,
    loaded: bool,
}

impl LoadWrapper {
    pub fn new(id: &str, fut: BoxFuture<'static, Result<Vec<u8>, String>>) -> Self {
        Self {
            id: id.to_string(),
            fut: Arc::new(Mutex::new(fut)),
            loaded: false,
        }
    }

    pub fn try_load(&mut self) -> Option<AssetLoad> {
        let waker = DummyWaker.into_task_waker();
        let mut ctx = Context::from_waker(&waker);
        match self.fut.lock().as_mut().poll(&mut ctx) {
            Poll::Ready(r_buff) => {
                self.loaded = true;
                match r_buff {
                    Ok(buff) => Some(AssetLoad {
                        id: self.id.clone(),
                        state: AssetState::Loaded(buff),
                    }),
                    Err(err) => {
                        let err = format!("Cannot load file: {}: {}", self.id, err);
                        log::warn!("{}", err);
                        Some(AssetLoad {
                            id: self.id.clone(),
                            state: AssetState::Err(err),
                        })
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
