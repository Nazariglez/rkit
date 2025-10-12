use assets::FileLoader;
use bevy_ecs::{prelude::*, system::IntoSystem};
use futures::task::{Context, Poll};
use futures_util::future::BoxFuture;
use parking_lot::Mutex;
use rustc_hash::FxHashMap;
use std::{
    any::{Any, TypeId},
    path::{Path, PathBuf},
    sync::Arc,
};

mod events;
mod waker;

use crate::{
    assets2::{
        events::{AssetLoad, LoadState},
        waker::*,
    },
    prelude::{App, OnEnginePreFrame, PanicContext, Plugin},
};

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn apply(&self, app: &mut App) {
        app.insert_resource(AssetLoader::default()).on_schedule(
            OnEnginePreFrame,
            (load_assets_system, parse_assets_system).chain(),
        );
    }
}

struct ParsedAny {
    type_id: TypeId,
    value: Box<dyn Any + Send + Sync>,
}

type ParserFn = dyn Fn(&mut World, AssetData) -> Result<ParsedAny, String> + Send + Sync + 'static;

#[derive(Resource)]
pub struct AssetLoader {
    file_loader: FileLoader,
    loading: Vec<LoadWrapper>,
    loaded: FxHashMap<String, (TypeId, Box<dyn Any + Send + Sync>)>,
    states: FxHashMap<String, AssetLoad>,
    lists: FxHashMap<String, LoadList>,
    registers: FxHashMap<
        String,
        Arc<Mutex<Option<Box<dyn FnOnce(&mut World) -> Arc<ParserFn> + Send + Sync>>>>,
    >,
    parsers: FxHashMap<String, Arc<ParserFn>>,
}

impl Default for AssetLoader {
    fn default() -> Self {
        let mut loader = Self {
            lists: Default::default(),
            registers: Default::default(),
            parsers: Default::default(),
            loaded: Default::default(),
            file_loader: FileLoader::new().or_panic("Creating FileLoader"),
            loading: vec![],
            states: FxHashMap::default(),
        };

        println!("add parser for empty extension");
        loader.add_parser("", bytes_parser);
        loader
    }
}

impl AssetLoader {
    // Parsed and available to use
    fn is_asset_parsed(&self, id: &str) -> bool {
        self.loaded.contains_key(id)
    }

    // Consider an item "done" when it either parsed successfully OR ended in error.
    // (If you want errors NOT to count toward progress, change the second branch.)
    fn is_asset_done(&self, id: &str) -> bool {
        if self.is_asset_parsed(id) {
            return true;
        }
        self.states
            .get(id)
            .is_some_and(|s| matches!(s.state, LoadState::Err(_)))
    }

    pub fn get<T: Any + Send + Sync>(&self, id: &str) -> Option<&T> {
        let (tid, v) = self.loaded.get(id)?;
        if *tid == TypeId::of::<T>() {
            // &Box<dyn Any> -> &dyn Any -> &T
            v.downcast_ref::<T>()
        } else {
            None
        }
    }

    pub fn list_progress(&self, list_id: &str) -> f32 {
        let Some(list) = self.lists.get(list_id) else {
            return 0.0;
        };
        let total = list.items.len();
        if total == 0 {
            return 1.0;
        }
        let done = list
            .items
            .iter()
            .filter(|item| self.is_asset_done(&item.id))
            .count();
        (done as f32) / (total as f32)
    }

    /// True only after the asset has been parsed & stored
    pub fn is_loaded(&self, id: &str) -> bool {
        // If it's a plain asset id:
        if self.is_asset_parsed(id) {
            return true;
        }

        // If it's a list id, require ALL items done.
        if let Some(list) = self.lists.get(id) {
            let total = list.items.len();
            if total == 0 {
                return true;
            }
            return list.items.iter().all(|item| self.is_asset_done(&item.id));
        }

        false
    }

    /// If you want type-specific readiness
    pub fn is_parsed<T: Any + Send + Sync>(&self, id: &str) -> bool {
        self.loaded
            .get(id)
            .is_some_and(|(tid, _)| *tid == TypeId::of::<T>())
    }

    /// Optional: take ownership and remove from the store
    pub fn take<T: Any + Send + Sync>(&mut self, id: &str) -> Option<T> {
        let (tid, v) = self.loaded.remove(id)?;
        if tid == TypeId::of::<T>() {
            v.downcast::<T>().ok().map(|b| *b)
        } else {
            None
        }
    }

    pub fn add_parser<T, Sys, Marker>(&mut self, id: &str, parser: Sys)
    where
        T: Any + Send + Sync + 'static,
        Sys: IntoSystem<In<AssetData>, Result<T, String>, Marker> + Send + Sync + 'static,
        Sys::System: Send + 'static,
    {
        let register = move |world: &mut World| -> Arc<ParserFn> {
            let sys_id = world.register_system(parser);

            Arc::new(move |world: &mut World, data: AssetData| {
                // Run the system and box the output
                let res: Result<T, String> = world
                    .run_system_with(sys_id, data)
                    .map_err(|e| format!("parser system failed: {e}"))?;

                res.map(|t| ParsedAny {
                    type_id: TypeId::of::<T>(),
                    value: Box::new(t) as Box<dyn Any + Send + Sync>,
                })
            })
        };

        self.registers.insert(
            id.to_string(),
            Arc::new(Mutex::new(Some(Box::new(register)))),
        );
    }

    pub fn load_list(&mut self, id: &str, list: impl Into<LoadList>) {
        self.lists.insert(id.to_string(), list.into());
    }

    /// Materialize all pending parser factories into `parsers`.
    /// Safe to call every frame; it only acts on entries that still exist in `registers`.
    fn prepare_all_parsers(&mut self, world: &mut World) {
        if self.registers.is_empty() {
            return;
        }

        // collect keys first to avoid borrowing issues
        let keys: Vec<String> = self.registers.keys().cloned().collect();
        for key in keys {
            if let Some(factory_cell) = self.registers.remove(&key) {
                if let Some(factory) = factory_cell.lock().take() {
                    let parser = factory(world);
                    self.parsers.insert(key.clone(), parser);
                    log::debug!(
                        "Prepared parser for extension: {}",
                        if key.is_empty() { "<default>" } else { &key }
                    );
                }
            }
        }
    }

    pub fn is_loading(&self, id: &str) -> bool {
        self.states
            .get(id)
            .is_some_and(|s| matches!(s.state, LoadState::Loading))
    }

    pub(crate) fn update(&mut self) {
        let mut needs_clean = false;
        self.loading.iter_mut().for_each(|loader| {
            let asset_state = self.states.get_mut(&loader.id).unwrap();
            if let Some(state) = loader.try_load(&asset_state.id) {
                asset_state.state = state;
                needs_clean = true;
            }
        });

        if needs_clean {
            self.loading.retain(|loader| !loader.is_loaded());
        }
    }

    pub fn load(&mut self, file_path: &str) {
        // ⬇️ don't enqueue twice
        if self.states.contains_key(file_path) || self.is_loaded(file_path) {
            log::debug!("Skipping load '{}': already pending or loaded", file_path);
            return;
        }

        log::debug!("Loading file '{file_path}'");
        let fut = Box::pin(self.file_loader.load_file(file_path));
        self.states.insert(
            file_path.to_string(),
            AssetLoad {
                id: file_path.to_string(),
                state: LoadState::Loading,
            },
        );
        let wrapper = LoadWrapper::new(file_path, fut);
        self.loading.push(wrapper);
    }

    pub fn load_bytes<S, B>(&mut self, id: S, bytes: B)
    where
        S: Into<String>,
        B: Into<Vec<u8>>,
    {
        let id = id.into();

        // don't re-enqueue if it's already pending or parsed
        if self.states.contains_key(&id) || self.is_loaded(&id) {
            return;
        }

        self.states.insert(
            id.clone(),
            AssetLoad {
                id,
                state: LoadState::Loaded(bytes.into()),
            },
        );
    }

    pub(crate) fn clear(&mut self) {
        self.loaded.clear();
        self.lists.clear();
    }
}

struct LoadItem {
    id: String,
    typ: LoadType,
    parser_ext: Option<String>,
}

enum LoadType {
    Path(PathBuf),
    Bytes(Vec<u8>),
}

pub struct LoadList {
    items: Vec<LoadItem>,
}

pub struct AssetData {
    pub id: String,
    pub data: Vec<u8>,
}

fn bytes_parser(data: In<AssetData>) -> Result<Vec<u8>, String> {
    Ok(data.data.to_vec())
}

fn load_assets_system(world: &mut World) {
    let _ = world.resource_scope(|_world: &mut World, mut loader: Mut<AssetLoader>| {
        // ----- Phase 1: scan lists (immutable), collect what to start -----
        enum ToStart {
            Path(String), // own the String to avoid lifetimes
            Bytes { id: String, bytes: Vec<u8> },
        }

        let mut to_start: Vec<ToStart> = Vec::new();

        for list in loader.lists.values() {
            for item in &list.items {
                // ⬇️ add this second condition
                if loader.states.contains_key(&item.id) || loader.is_loaded(&item.id) {
                    continue;
                }
                match &item.typ {
                    LoadType::Path(p) => {
                        to_start.push(ToStart::Path(p.to_string_lossy().into_owned()))
                    }
                    LoadType::Bytes(b) => to_start.push(ToStart::Bytes {
                        id: item.id.clone(),
                        bytes: b.clone(),
                    }),
                }
            }
        }

        // ----- Phase 2: mutate loader (kick off loads / insert ready bytes) -----
        for action in to_start {
            match action {
                ToStart::Path(path) => {
                    loader.load(&path);
                }
                ToStart::Bytes { id, bytes } => {
                    loader.states.insert(
                        id.clone(),
                        AssetLoad {
                            id,
                            state: LoadState::Loaded(bytes),
                        },
                    );
                }
            }
        }

        // Poll I/O futures to move Loading -> Loaded/Err
        loader.update();
    });
}

fn parse_assets_system(world: &mut World) {
    world.resource_scope(|world: &mut World, mut loader: Mut<AssetLoader>| {
        loader.prepare_all_parsers(world);
        // -------- phase 1: collect parse jobs from states --------
        struct Job {
            id: String,
            ext: String,
            bytes: Vec<u8>,
        }
        let mut jobs: Vec<Job> = Vec::new();

        for (id, st) in loader.states.iter() {
            if let LoadState::Loaded(bytes) = &st.state {
                // derive extension from the id/path
                let ext = std::path::Path::new(id)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or_default()
                    .to_string();

                jobs.push(Job {
                    id: id.clone(),
                    ext,
                    bytes: bytes.clone(),
                });
            }
        }

        // -------- phase 2: ensure parser, parse, store, clean --------
        let mut to_remove: Vec<String> = Vec::new();

        for Job { id, ext, bytes } in jobs {
            let parsers = loader.parsers.keys().collect::<Vec<_>>();
            println!("parsers {parsers:?}");

            // fall back to "" parser if needed
            let parser = loader
                .parsers
                .get(&ext)
                .or_else(|| loader.parsers.get(""))
                .expect("parser must exist (ext or default)");

            match (parser)(
                world,
                AssetData {
                    id: id.clone(),
                    data: bytes,
                },
            ) {
                Ok(parsed) => {
                    loader
                        .loaded
                        .insert(id.clone(), (parsed.type_id, parsed.value));

                    log::info!("Parsed and stored '{id}'");
                    to_remove.push(id);
                }
                Err(e) => {
                    if let Some(s) = loader.states.get_mut(&id) {
                        s.state = LoadState::Err(e.clone());
                    }
                    log::warn!("Parse failed for '{id}': {e}");
                }
            }
        }

        for id in to_remove {
            loader.states.remove(&id);
        }
    });
}

impl From<Vec<String>> for LoadList {
    fn from(value: Vec<String>) -> Self {
        Self {
            items: value
                .into_iter()
                .map(|item| {
                    let file_path = Path::new(&item);
                    let ext = file_path
                        .extension()
                        .map(|ext| ext.to_string_lossy().to_string());
                    LoadItem {
                        id: item.clone(),
                        typ: LoadType::Path(file_path.to_path_buf()),
                        parser_ext: ext,
                    }
                })
                .collect(),
        }
    }
}

impl<const N: usize> From<[&str; N]> for LoadList {
    fn from(value: [&str; N]) -> Self {
        Self {
            items: value
                .iter()
                .map(|&item| {
                    let file_path = Path::new(item);
                    let ext = file_path
                        .extension()
                        .map(|ext| ext.to_string_lossy().to_string());
                    LoadItem {
                        id: item.to_string(),
                        typ: LoadType::Path(file_path.to_path_buf()),
                        parser_ext: ext,
                    }
                })
                .collect(),
        }
    }
}

impl<const N: usize> From<[String; N]> for LoadList {
    fn from(value: [String; N]) -> Self {
        Self {
            items: value
                .iter()
                .map(|item| {
                    let file_path = Path::new(&item);
                    let ext = file_path
                        .extension()
                        .map(|ext| ext.to_string_lossy().to_string());
                    LoadItem {
                        id: item.clone(),
                        typ: LoadType::Path(file_path.to_path_buf()),
                        parser_ext: ext,
                    }
                })
                .collect(),
        }
    }
}

impl From<Vec<&str>> for LoadList {
    fn from(value: Vec<&str>) -> Self {
        Self {
            items: value
                .into_iter()
                .map(|item| {
                    let file_path = Path::new(item);
                    let ext = file_path
                        .extension()
                        .map(|ext| ext.to_string_lossy().to_string());
                    LoadItem {
                        id: item.to_string(),
                        typ: LoadType::Path(file_path.to_path_buf()),
                        parser_ext: ext,
                    }
                })
                .collect(),
        }
    }
}

impl<const N: usize> From<&[&str; N]> for LoadList {
    fn from(value: &[&str; N]) -> Self {
        Self {
            items: value
                .iter()
                .map(|&item| {
                    let file_path = std::path::Path::new(item);
                    let ext = file_path
                        .extension()
                        .map(|ext| ext.to_string_lossy().to_string());
                    LoadItem {
                        id: item.to_string(),
                        typ: LoadType::Path(file_path.to_path_buf()),
                        parser_ext: ext,
                    }
                })
                .collect(),
        }
    }
}

impl<const N: usize> From<&[String; N]> for LoadList {
    fn from(value: &[String; N]) -> Self {
        Self {
            items: value
                .iter()
                .map(|item| {
                    let file_path = Path::new(&item);
                    let ext = file_path
                        .extension()
                        .map(|ext| ext.to_string_lossy().to_string());
                    LoadItem {
                        id: item.clone(),
                        typ: LoadType::Path(file_path.to_path_buf()),
                        parser_ext: ext,
                    }
                })
                .collect(),
        }
    }
}

type InnerBoxFuture = BoxFuture<'static, Result<Vec<u8>, String>>;

struct LoadWrapper {
    id: String,
    fut: Arc<Mutex<InnerBoxFuture>>,
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

    pub fn try_load(&mut self, id: &str) -> Option<LoadState> {
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
                        Some(LoadState::Loaded(buff))
                    }
                    Err(err) => {
                        let err = format!("Cannot load file: {id}: {err}");
                        log::warn!("{err}");
                        Some(LoadState::Err(err))
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
