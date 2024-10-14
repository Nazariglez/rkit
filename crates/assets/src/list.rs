use crate::{is_loaded, is_loading, AssetId};
use rustc_hash::FxHashMap;
use std::any::{Any, TypeId};
use std::rc::Rc;

#[derive(Default)]
pub struct AssetMap {
    inner: FxHashMap<TypeId, FxHashMap<String, Rc<dyn Any>>>,
    len: usize,
}

impl AssetMap {
    pub(crate) fn insert<T: 'static>(&mut self, id: String, asset: T) {
        let type_id = TypeId::of::<T>();

        self.inner
            .entry(type_id)
            .or_insert_with(FxHashMap::default)
            .insert(id, Rc::new(asset));

        self.len += 1;
    }

    pub fn get<T: Clone + 'static>(&self, id: &str) -> Result<T, String> {
        self.inner
            .get(&TypeId::of::<T>())
            .ok_or_else(|| format!("Invalid type for asset: {id}"))
            .and_then(|map| {
                map.get(id)
                    .ok_or_else(|| format!("Cannot find asset: {id}"))
            })
            .and_then(|asset| {
                asset
                    .clone()
                    .downcast::<T>()
                    .map_err(|_| format!("Failed to downcast asset with id '{id}' to correct type"))
            })
            .and_then(|asset| Ok((*asset).clone()))
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

struct Data {
    id: AssetId,
    loaded: bool,
}

pub struct AssetList {
    inner: FxHashMap<String, Data>,
    total: usize,

    assets: AssetMap,
    parsers: FxHashMap<String, Box<dyn Fn(&AssetId, &str, &mut AssetMap) -> Result<(), String>>>,
}

impl AssetList {
    pub fn new(paths: &[&str]) -> Self {
        let inner = paths
            .iter()
            .map(|path| {
                let id = super::load_asset(path);
                (path.to_string(), Data { id, loaded: false })
            })
            .collect::<FxHashMap<String, Data>>();
        let count = inner.len();
        Self {
            inner,
            total: count,
            assets: AssetMap::default(),
            parsers: FxHashMap::default(),
        }
    }

    pub fn progress(&self) -> f32 {
        // let's do this just to avoid some float impression
        if self.is_loaded() {
            return 1.0;
        }

        // at this point we will iterate twice against the map but I think that the cost it's minimal
        let progress = self.load_len() as f32 / self.total as f32;
        progress
    }

    pub fn load_len(&self) -> usize {
        self.assets.len()
            + self.inner.iter().fold(0, |count, (_, data)| {
                if is_loaded(&data.id) {
                    count + 1
                } else {
                    count
                }
            })
    }

    pub fn is_loaded(&self) -> bool {
        self.load_len() >= self.total
    }

    pub fn with_extension_parser<T, F>(mut self, ext: &str, parser: F) -> Self
    where
        F: Fn(&str, &[u8]) -> Result<T, String> + 'static + Clone,
        T: 'static,
    {
        self.parsers.insert(
            ext.to_string(),
            Box::new(move |aid: &AssetId, id: &str, map: &mut AssetMap| {
                let parsed = super::parse_asset::<T, F>(aid, parser.clone(), false)?;
                if let Some(parsed_asset) = parsed {
                    map.insert(id.to_string(), parsed_asset);
                }

                Ok(())
            }),
        );
        self
    }

    pub fn parse<T, F>(&mut self, parser: F) -> Result<Option<T>, String>
    where
        F: FnOnce(&AssetMap) -> Result<T, String>,
    {
        for (path, data) in &mut self.inner {
            if data.loaded || is_loading(&data.id) {
                continue;
            }

            let ext = path.split('.').last().and_then(|ext| self.parsers.get(ext));
            match ext {
                // use the parser provided to store the asset as the type needed
                Some(parser) => (*parser)(&data.id, path, &mut self.assets)?,

                // parse as Vec<u8> if there is no parser added for this extension
                _ => {
                    let parsed = super::parse_asset(&data.id, parse_vec, false)?;
                    if let Some(parsed_asset) = parsed {
                        self.assets.insert(path.clone(), parsed_asset);
                    }
                }
            }

            data.loaded = true;
        }

        println!("{} {}", self.total, self.load_len());
        if !self.is_loaded() {
            return Ok(None);
        }

        parser(&self.assets).map(|res| Some(res))
    }
}

fn parse_vec(_id: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    Ok(data.to_vec())
}
