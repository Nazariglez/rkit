use assets::FileLoader;
use bevy_ecs::{prelude::*, system::IntoSystem};
use futures::task::{Context, Poll};
use futures_util::future::BoxFuture;
use parking_lot::Mutex;
use rustc_hash::FxHashMap;
use std::{
    any::{Any, TypeId},
    borrow::Cow,
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

#[derive(Default)]
pub struct AssetsPlugin {
    loader: AssetLoader,
}

impl AssetsPlugin {
    pub fn add_parser<T, Sys, Marker>(mut self, id: &str, parser: Sys) -> Self
    where
        T: Any + Send + Sync + 'static,
        Sys: IntoSystem<In<AssetData>, Result<T, String>, Marker> + Send + Sync + 'static,
        Sys::System: Send + 'static,
    {
        self.loader.add_parser(id, parser);
        self
    }
}

impl Plugin for AssetsPlugin {
    fn apply(&self, app: &mut App) {
        let loader = AssetLoader {
            registers: self.loader.registers.clone(),
            ..Default::default()
        };

        app.insert_resource(loader).on_schedule(
            OnEnginePreFrame,
            (load_assets_system, parse_assets_system).chain(),
        );
    }
}

pub trait AutoLoad {
    fn load_list() -> LoadList;
    fn parse_list(loader: &mut AssetLoader, list: &mut LoadList) -> Result<Option<Self>, String>
    where
        Self: Sized;
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
    lists: FxHashMap<String, Vec<String>>,
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

        loader.add_parser("", bytes_parser);
        loader
    }
}

impl AssetLoader {
    pub fn auto_load<T: AutoLoad>(&mut self) {}

    /// Returns a reference to a loaded asset by its ID and type.
    pub fn get<T: Any + Send + Sync>(&self, id: &str) -> Option<&T> {
        let (tid, v) = self.loaded.get(id)?;
        if *tid == TypeId::of::<T>() {
            v.downcast_ref::<T>()
        } else {
            None
        }
    }

    /// Returns the loading progress of a list (0.0 to 1.0).
    pub fn list_progress(&self, list_id: &str) -> f32 {
        let Some(list) = self.lists.get(list_id) else {
            return 0.0;
        };

        let total = list.len();
        let done = list.iter().filter(|item| self.is_loaded(item)).count();
        (done as f32) / (total as f32)
    }

    /// Checks if an asset has been loaded and parsed.
    pub fn is_loaded(&self, id: &str) -> bool {
        self.loaded.contains_key(id)
    }

    /// Removes and returns a loaded asset, transferring ownership to the caller.
    pub fn take<T: Any + Send + Sync>(&mut self, id: &str) -> Option<T> {
        let (tid, v) = self.loaded.remove(id)?;
        let same_type = tid == TypeId::of::<T>();
        if !same_type {
            return None;
        }

        let val: T = v.downcast::<T>().ok().map(|b| *b)?;
        self.remove_from_lists(id);
        Some(val)
    }

    /// Registers a custom parser for a specific file extension.
    pub fn add_parser<T, Sys, Marker>(&mut self, id: &str, parser: Sys)
    where
        T: Any + Send + Sync + 'static,
        Sys: IntoSystem<In<AssetData>, Result<T, String>, Marker> + Send + Sync + 'static,
        Sys::System: Send + 'static,
    {
        let register = move |world: &mut World| -> Arc<ParserFn> {
            let sys_id = world.register_system(parser);

            Arc::new(move |world: &mut World, data: AssetData| {
                let res = world
                    .run_system_with(sys_id, data)
                    .map_err(|e| format!("parser system failed: {e}"))?;

                res.map(|t| ParsedAny {
                    type_id: TypeId::of::<T>(),
                    value: Box::new(t),
                })
            })
        };

        self.registers.insert(
            id.to_string(),
            Arc::new(Mutex::new(Some(Box::new(register)))),
        );
    }

    /// Loads multiple assets as a named list for tracking progress.
    pub fn load_list(&mut self, id: &str, list: impl Into<LoadList>) {
        let list_id = id.to_string();
        let LoadList { items } = list.into();

        for item in &items {
            match &item.typ {
                LoadType::Path(p) => {
                    self.load(&p.to_string_lossy().to_string());
                }
                LoadType::Bytes(b) => {
                    self.load_bytes(&item.id, b.clone());
                }
            }
        }

        self.lists
            .insert(list_id, items.into_iter().map(|item| item.id).collect());
    }

    /// Checks if an asset is currently being loaded.
    pub fn is_loading(&self, id: &str) -> bool {
        self.states
            .get(id)
            .is_some_and(|s| matches!(s.state, LoadState::Loading))
    }

    /// Loads an asset from a file path.
    pub fn load(&mut self, file_path: &str) {
        self.load_with_id(file_path, file_path);
    }

    /// Load an asset from a file path with a custom ID.
    pub fn load_with_id(&mut self, id: &str, file_path: &str) {
        if self.states.contains_key(id) || self.is_loaded(id) {
            log::debug!("Skipping load '{}': already pending or loaded", id);
            return;
        }

        log::debug!("Loading asset file '{file_path}'");
        let fut = Box::pin(self.file_loader.load_file(file_path));
        self.states.insert(
            id.to_string(),
            AssetLoad {
                id: id.to_string(),
                state: LoadState::Loading,
            },
        );
        let wrapper = LoadWrapper::new(file_path, fut);
        self.loading.push(wrapper);
    }

    /// Loads an asset from raw bytes with a custom ID.
    pub fn load_bytes<S, B>(&mut self, id: S, bytes: B)
    where
        S: Into<String>,
        B: Into<Vec<u8>>,
    {
        let id = id.into();

        if self.states.contains_key(&id) || self.is_loaded(&id) {
            log::debug!("Skipping load '{}': already pending or loaded", id);
            return;
        }

        log::debug!("Loading asset bytes '{id}'");
        self.states.insert(
            id.clone(),
            AssetLoad {
                id,
                state: LoadState::Loaded(bytes.into()),
            },
        );
    }

    /// Clears all loaded assets.
    pub fn clear(&mut self) {
        self.loaded.clear();
        self.lists.clear();
    }

    fn update(&mut self) {
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

    fn process_parsers(&mut self, world: &mut World) {
        if self.registers.is_empty() {
            return;
        }

        let keys = self.registers.keys().cloned().collect::<Vec<_>>();
        for key in keys {
            if let Some(factory_cell) = self.registers.remove(&key) {
                if let Some(factory) = factory_cell.lock().take() {
                    let parser = factory(world);
                    self.parsers.insert(key.clone(), parser);
                    log::debug!(
                        "Parser for extension '{}' ready",
                        if key.is_empty() { "<default>" } else { &key }
                    );
                }
            }
        }
    }

    fn remove_from_lists(&mut self, id: &str) {
        self.lists.iter_mut().for_each(|(_, list)| {
            list.retain(|item| item != id);
        });

        self.lists.retain(|_, list| !list.is_empty());
    }

    #[cfg(test)]
    pub(crate) fn is_parsed<T: Any + Send + Sync>(&self, id: &str) -> bool {
        self.loaded
            .get(id)
            .is_some_and(|(tid, _)| *tid == TypeId::of::<T>())
    }
}

struct LoadItem {
    id: String,
    typ: LoadType,
}

enum LoadType {
    Path(PathBuf),
    Bytes(Vec<u8>),
}

#[derive(Default)]
pub struct LoadList {
    items: Vec<LoadItem>,
}

impl LoadList {
    #[inline]
    pub fn add_from_path(&mut self, path: &str) -> &mut Self {
        self.items.push(LoadItem {
            id: path.to_string(),
            typ: LoadType::Path(PathBuf::from(path)),
        });
        self
    }

    #[inline]
    pub fn add_from_path_with_id(&mut self, id: &str, path: &str) -> &mut Self {
        self.items.push(LoadItem {
            id: id.to_string(),
            typ: LoadType::Path(PathBuf::from(path)),
        });
        self
    }

    #[inline]
    pub fn add_from_bytes(&mut self, id: &str, bytes: &[u8]) -> &mut Self {
        self.items.push(LoadItem {
            id: id.to_string(),
            typ: LoadType::Bytes(bytes.to_vec()),
        });
        self
    }
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
        loader.update();
    });
}

fn parse_assets_system(world: &mut World) {
    world.resource_scope(|world: &mut World, mut loader: Mut<AssetLoader>| {
        loader.process_parsers(world);

        // gather all the assets to parse
        struct ToParse {
            id: String,
            ext: String,
            bytes: Vec<u8>,
        }

        let mut to_parse = vec![];
        for (id, st) in loader.states.iter() {
            if let LoadState::Loaded(bytes) = &st.state {
                let ext = std::path::Path::new(id)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or_default()
                    .to_string();

                to_parse.push(ToParse {
                    id: id.clone(),
                    ext,
                    bytes: bytes.clone(),
                });
            }
        }

        // parse the assets
        let mut to_remove: Vec<String> = Vec::new();
        for ToParse { id, ext, bytes } in to_parse {
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

                    log::info!("Asset parsed '{id}'");
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

        // remove the assets that have been parsed
        for id in to_remove {
            loader.states.remove(&id);
        }
    });
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

trait ToLoadItem {
    fn to_load_item(&self) -> LoadItem;
}

impl ToLoadItem for String {
    fn to_load_item(&self) -> LoadItem {
        LoadItem {
            id: self.clone(),
            typ: LoadType::Path(PathBuf::from(self)),
        }
    }
}

impl ToLoadItem for PathBuf {
    fn to_load_item(&self) -> LoadItem {
        LoadItem {
            id: self.to_string_lossy().to_string(),
            typ: LoadType::Path(self.clone()),
        }
    }
}

impl<T> ToLoadItem for (T, Vec<u8>)
where
    T: AsRef<str>,
{
    fn to_load_item(&self) -> LoadItem {
        LoadItem {
            id: self.0.as_ref().to_string(),
            typ: LoadType::Bytes(self.1.clone()),
        }
    }
}

impl<T> ToLoadItem for (T, &[u8])
where
    T: AsRef<str>,
{
    fn to_load_item(&self) -> LoadItem {
        LoadItem {
            id: self.0.as_ref().to_string(),
            typ: LoadType::Bytes(self.1.to_vec()),
        }
    }
}

impl<T> ToLoadItem for (T, Cow<'_, [u8]>)
where
    T: AsRef<str>,
{
    fn to_load_item(&self) -> LoadItem {
        LoadItem {
            id: self.0.as_ref().to_string(),
            typ: LoadType::Bytes(self.1.to_vec()),
        }
    }
}

impl<T> ToLoadItem for (T, PathBuf)
where
    T: AsRef<str>,
{
    fn to_load_item(&self) -> LoadItem {
        LoadItem {
            id: self.0.as_ref().to_string(),
            typ: LoadType::Path(self.1.clone()),
        }
    }
}

impl<T> ToLoadItem for (T, &Path)
where
    T: AsRef<str>,
{
    fn to_load_item(&self) -> LoadItem {
        LoadItem {
            id: self.0.as_ref().to_string(),
            typ: LoadType::Path(self.1.to_path_buf()),
        }
    }
}

impl<T> ToLoadItem for (T, &str)
where
    T: AsRef<str>,
{
    fn to_load_item(&self) -> LoadItem {
        LoadItem {
            id: self.0.as_ref().to_string(),
            typ: LoadType::Path(PathBuf::from(self.1)),
        }
    }
}

impl<T> ToLoadItem for (T, String)
where
    T: AsRef<str>,
{
    fn to_load_item(&self) -> LoadItem {
        LoadItem {
            id: self.0.as_ref().to_string(),
            typ: LoadType::Path(PathBuf::from(&self.1)),
        }
    }
}

impl<T> ToLoadItem for (T, &PathBuf)
where
    T: AsRef<str>,
{
    fn to_load_item(&self) -> LoadItem {
        LoadItem {
            id: self.0.as_ref().to_string(),
            typ: LoadType::Path(self.1.clone()),
        }
    }
}

impl<'a> ToLoadItem for &'a str {
    fn to_load_item(&self) -> LoadItem {
        LoadItem {
            id: (*self).to_string(),
            typ: LoadType::Path(PathBuf::from(*self)),
        }
    }
}

impl<'a> ToLoadItem for &'a String {
    fn to_load_item(&self) -> LoadItem {
        LoadItem {
            id: (*self).clone(),
            typ: LoadType::Path(PathBuf::from(&**self)),
        }
    }
}

impl<'a> ToLoadItem for &'a Path {
    fn to_load_item(&self) -> LoadItem {
        LoadItem {
            id: self.to_string_lossy().to_string(),
            typ: LoadType::Path(self.to_path_buf()),
        }
    }
}

impl<'a> ToLoadItem for &'a PathBuf {
    fn to_load_item(&self) -> LoadItem {
        LoadItem {
            id: self.to_string_lossy().to_string(),
            typ: LoadType::Path((*self).clone()),
        }
    }
}

impl<'a, 'b> ToLoadItem for &'a &'b str {
    fn to_load_item(&self) -> LoadItem {
        (*self).to_load_item()
    }
}

impl<'a, 'b> ToLoadItem for &'a &'b String {
    fn to_load_item(&self) -> LoadItem {
        (*self).to_load_item()
    }
}

impl<'a, 'b> ToLoadItem for &'a &'b Path {
    fn to_load_item(&self) -> LoadItem {
        (*self).to_load_item()
    }
}

impl<'a, 'b> ToLoadItem for &'a &'b PathBuf {
    fn to_load_item(&self) -> LoadItem {
        (*self).to_load_item()
    }
}

impl<I, N> From<I> for LoadList
where
    I: IntoIterator<Item = N>,
    N: ToLoadItem,
{
    fn from(value: I) -> Self {
        Self {
            items: value.into_iter().map(|item| item.to_load_item()).collect(),
        }
    }
}

impl<N> FromIterator<N> for LoadList
where
    N: ToLoadItem,
{
    fn from_iter<I: IntoIterator<Item = N>>(iter: I) -> Self {
        Self {
            items: iter.into_iter().map(|n| n.to_load_item()).collect(),
        }
    }
}

pub trait AssetCmdExt {
    fn load(&mut self, file_path: &str);
    fn load_with_id(&mut self, id: &str, file_path: &str);
    fn load_bytes<S, B>(&mut self, id: S, bytes: B)
    where
        S: Into<String>,
        B: Into<Vec<u8>>;
    fn load_list(&mut self, id: &str, list: impl Into<LoadList>);
}

impl AssetCmdExt for Commands<'_, '_> {
    fn load(&mut self, file_path: &str) {
        self.queue(AssetLoadCmd {
            id: file_path.to_string(),
            file_path: file_path.to_string(),
        });
    }

    fn load_with_id(&mut self, id: &str, file_path: &str) {
        self.queue(AssetLoadCmd {
            id: id.to_string(),
            file_path: file_path.to_string(),
        });
    }

    fn load_bytes<S, B>(&mut self, id: S, bytes: B)
    where
        S: Into<String>,
        B: Into<Vec<u8>>,
    {
        self.queue(AssetLoadBytesCmd {
            id: id.into(),
            bytes: bytes.into(),
        });
    }

    fn load_list(&mut self, id: &str, list: impl Into<LoadList>) {
        self.queue(AssetLoadListCmd {
            id: id.to_string(),
            list: list.into(),
        });
    }
}

struct AssetLoadCmd {
    id: String,
    file_path: String,
}

impl Command for AssetLoadCmd {
    fn apply(self, world: &mut World) {
        debug_assert!(
            world.contains_resource::<AssetLoader>(),
            "AssetLoader must be present"
        );

        world
            .resource_mut::<AssetLoader>()
            .load_with_id(&self.id, &self.file_path);
    }
}

struct AssetLoadBytesCmd {
    id: String,
    bytes: Vec<u8>,
}

impl Command for AssetLoadBytesCmd {
    fn apply(self, world: &mut World) {
        debug_assert!(
            world.contains_resource::<AssetLoader>(),
            "AssetLoader must be present"
        );
        world
            .resource_mut::<AssetLoader>()
            .load_bytes(self.id, self.bytes);
    }
}

struct AssetLoadListCmd {
    id: String,
    list: LoadList,
}

impl Command for AssetLoadListCmd {
    fn apply(self, world: &mut World) {
        debug_assert!(
            world.contains_resource::<AssetLoader>(),
            "AssetLoader must be present"
        );
        world
            .resource_mut::<AssetLoader>()
            .load_list(&self.id, self.list);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::world::World;

    fn parser_string_from_utf8(asset_input: In<AssetData>) -> Result<String, String> {
        String::from_utf8(asset_input.data.clone()).map_err(|utf8_error| utf8_error.to_string())
    }

    fn parser_always_error(_asset_input: In<AssetData>) -> Result<String, String> {
        Err("error".to_string())
    }

    #[test]
    fn test_load_bytes_defualt_parser() {
        let mut world = World::new();
        world.insert_resource(AssetLoader::default());

        {
            let mut asset_loader = world.resource_mut::<AssetLoader>();
            asset_loader.load_bytes("no_id", b"hello world");
        }

        parse_assets_system(&mut world);

        let asset_loader = world.resource::<AssetLoader>();
        assert!(asset_loader.is_parsed::<Vec<u8>>("no_id"));
        let stored_bytes = asset_loader.get::<Vec<u8>>("no_id").unwrap();
        assert_eq!(stored_bytes, &b"hello world".to_vec());
    }

    #[test]
    fn test_load_bytes_ext_parser() {
        let mut world = World::new();
        world.insert_resource(AssetLoader::default());

        {
            let mut asset_loader = world.resource_mut::<AssetLoader>();
            asset_loader.add_parser("txt", parser_string_from_utf8);
            asset_loader.load_bytes("text_file.txt", b"sample text");
        }

        parse_assets_system(&mut world);

        let asset_loader = world.resource::<AssetLoader>();
        assert!(asset_loader.is_parsed::<String>("text_file.txt"));
        let stored_text = asset_loader.get::<String>("text_file.txt").unwrap();
        assert_eq!(stored_text, "sample text");
    }

    #[test]
    fn test_load_list_take() {
        let mut world = World::new();
        world.insert_resource(AssetLoader::default());

        let list_identifier_string = "example_list_id".to_string();
        let asset_identifier_string = "inline_data.txt".to_string();

        {
            let mut asset_loader = world.resource_mut::<AssetLoader>();
            asset_loader.add_parser("txt", parser_string_from_utf8);

            let load_list = LoadList {
                items: vec![LoadItem {
                    id: asset_identifier_string.clone(),
                    typ: LoadType::Bytes(b"bytes content".to_vec()),
                }],
            };

            asset_loader.load_list(&list_identifier_string, load_list);
        }

        parse_assets_system(&mut world);

        {
            let mut asset_loader = world.resource_mut::<AssetLoader>();
            assert!(asset_loader.is_parsed::<String>(&asset_identifier_string));
            let taken_value: String = asset_loader
                .take::<String>(&asset_identifier_string)
                .unwrap();
            assert_eq!(taken_value, "bytes content");
            assert!(!asset_loader.is_loaded(&asset_identifier_string));
            assert!(asset_loader.lists.get(&list_identifier_string).is_none());
        }
    }

    #[test]
    fn test_load_list_progress() {
        let mut world = World::new();
        world.insert_resource(AssetLoader::default());

        let list_identifier_string = "dual_list_progress".to_string();

        {
            let mut asset_loader = world.resource_mut::<AssetLoader>();
            asset_loader.add_parser("txt", parser_string_from_utf8);

            let load_list = LoadList {
                items: vec![
                    LoadItem {
                        id: "first_loaded.txt".to_string(),
                        typ: LoadType::Bytes(b"alpha".to_vec()),
                    },
                    LoadItem {
                        id: "path_not_parsed.yet".to_string(),
                        typ: LoadType::Path(PathBuf::from("some/missing.file")),
                    },
                ],
            };

            asset_loader.load_list(&list_identifier_string, load_list);
        }

        parse_assets_system(&mut world);

        let asset_loader = world.resource::<AssetLoader>();
        let progress_value = asset_loader.list_progress(&list_identifier_string);
        assert!((progress_value - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_load_bytes_error_state() {
        let mut world = World::new();
        world.insert_resource(AssetLoader::default());

        let asset_identifier_string = "bad_asset.err".to_string();

        {
            let mut asset_loader = world.resource_mut::<AssetLoader>();
            asset_loader.add_parser("err", parser_always_error);
            asset_loader.load_bytes(&asset_identifier_string, b"does not matter");
        }

        parse_assets_system(&mut world);

        let asset_loader = world.resource::<AssetLoader>();
        assert!(
            asset_loader
                .get::<String>(&asset_identifier_string)
                .is_none()
        );
        let state_entry = asset_loader.states.get(&asset_identifier_string).unwrap();
        match &state_entry.state {
            LoadState::Err(error_message) => {
                assert!(error_message.contains("error"));
            }
            _ => panic!("expected error state"),
        }
    }

    #[test]
    fn test_is_loading() {
        let mut world = World::new();
        world.insert_resource(AssetLoader::default());

        let asset_identifier_string = "unavailable_path.asset".to_string();

        {
            let mut asset_loader = world.resource_mut::<AssetLoader>();
            asset_loader.load(&asset_identifier_string);
            assert!(asset_loader.is_loading(&asset_identifier_string));
        }
    }

    #[test]
    fn test_load_bytes_clear() {
        let mut world = World::new();
        world.insert_resource(AssetLoader::default());

        {
            let mut asset_loader = world.resource_mut::<AssetLoader>();
            asset_loader.load_bytes("first_blob", b"one");
            asset_loader.load_bytes("second_blob", b"two");
        }

        parse_assets_system(&mut world);

        {
            let mut asset_loader = world.resource_mut::<AssetLoader>();
            asset_loader.lists.insert(
                "temporary_list".to_string(),
                vec!["first_blob".to_string(), "second_blob".to_string()],
            );
            assert!(asset_loader.is_loaded("first_blob"));
            assert!(asset_loader.is_loaded("second_blob"));
            assert!(!asset_loader.lists.is_empty());
            asset_loader.clear();
            assert!(asset_loader.loaded.is_empty());
            assert!(asset_loader.lists.is_empty());
        }
    }

    #[test]
    fn test_load_bytes_type_specific_parsed() {
        let mut world = World::new();
        world.insert_resource(AssetLoader::default());

        {
            let mut asset_loader = world.resource_mut::<AssetLoader>();
            asset_loader.add_parser("txt", parser_string_from_utf8);
            asset_loader.load_bytes("typ.txt", b"abc");
        }

        parse_assets_system(&mut world);

        let asset_loader = world.resource::<AssetLoader>();
        assert!(asset_loader.is_parsed::<String>("typ.txt"));
        assert!(!asset_loader.is_parsed::<Vec<u8>>("typ.txt"));
        assert!(asset_loader.get::<Vec<u8>>("typ.txt").is_none());
    }

    #[test]
    fn test_load_bytes_skips_duplicates() {
        let mut world = World::new();
        world.insert_resource(AssetLoader::default());

        {
            let mut asset_loader = world.resource_mut::<AssetLoader>();
            asset_loader.load_bytes("dup_id.txt", b"first");
            let states_count_before = asset_loader.states.len();
            asset_loader.load_bytes("dup_id.txt", b"second");
            let states_count_after = asset_loader.states.len();
            assert_eq!(states_count_before, states_count_after);
        }

        {
            parse_assets_system(&mut world);
            let asset_loader = world.resource::<AssetLoader>();
            assert!(asset_loader.is_loaded("dup_id.txt"));
        }
    }
}
