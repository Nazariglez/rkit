use bevy_ecs::{prelude::*, system::IntoSystem};
use rustc_hash::FxHashMap;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::prelude::{App, OnEnginePreFrame, PanicContext, Plugin};

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn apply(&self, app: &mut App) {
        app.insert_resource(AssetLoader::default())
            .on_schedule(OnEnginePreFrame, (load_assets_system, parse_assets_system));
    }
}

type ParserFn = dyn Fn(&mut World, AssetData) + Send + Sync + 'static;

#[derive(Resource)]
pub struct AssetLoader {
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
        };

        loader.add_parser("", bytes_parser);
        loader
    }
}

impl AssetLoader {
    pub fn add_parser<T, Sys, Marker>(&mut self, id: &str, parser: Sys)
    where
        T: Send + 'static,
        Sys: IntoSystem<In<AssetData>, Result<T, String>, Marker> + Send + Sync + 'static,
        Sys::System: Send + 'static,
    {
        let register = move |world: &mut World| -> Arc<ParserFn> {
            let sys_id = world.register_system(parser);
            Arc::new(move |world: &mut World, data: AssetData| {
                let _ = world.run_system_with(sys_id, data);
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

    fn try_parse_item(&mut self, world: &mut World, item: &LoadItem) {
        let ext = item.parser_ext.as_deref().unwrap_or_default();

        if !self.parsers.contains_key(ext) {
            let register = self
                .registers
                .remove(ext)
                .ok_or_else(|| format!("Parser not found for extension: {ext}"))
                .or_panic_with(|| format!("Parser not found for extension: {ext}"))
                .lock()
                .or_panic_with(|| format!("Parser not found for extension: {ext}"))
                .take()
                .or_panic_with(|| format!("Parser not found for extension: {ext}"));

            // store parser
            let parser = register(world);
            self.parsers.insert(ext.to_string(), parser);
            log::debug!("Added parser for extension: {ext}");
        }

        let parser = self
            .parsers
            .get(ext)
            .or_panic_with(|| format!("Parser not found for extension: {ext}"));

        // (*parser)(
        //     world,
        //     AssetData {
        //         path: todo!(),
        //         data: todo!(),
        //     },
        // );
        // parser must be present, just use it
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
    id: String,
    data: Vec<u8>,
}

impl AssetData {}

fn bytes_parser(data: In<AssetData>) -> Result<Vec<u8>, String> {
    Ok(data.data.to_vec())
}

fn load_assets_system(world: &mut World) {
    let _ = world.resource_scope(|world: &mut World, asset_loader: Mut<AssetLoader>| {
        asset_loader.lists.iter().for_each(|(_, list)| {
            list.items.iter().for_each(|item| {});
        });
    });
}

fn parse_assets_system(world: &mut World) {}

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
