use crate::{is_loaded, is_loading, AssetId};
use anymap3::AnyMap;
use rustc_hash::FxHashMap;

struct Data {
    id: AssetId,
    loaded: bool,
}

pub struct AssetList {
    inner: FxHashMap<String, Data>,
    len: usize,

    parsed: AnyMap,
    parsers: FxHashMap<String, Box<dyn Fn(&AssetId, &str, &mut AnyMap) -> Result<(), String>>>,
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
        let len = inner.len();
        Self {
            inner,
            len,
            parsed: AnyMap::default(),
            parsers: FxHashMap::default(),
        }
    }

    pub fn progress(&self) -> f32 {
        // let's do this just to avoid some float impression
        if self.is_loaded() {
            return 1.0;
        }

        // at this point we will iterate twice against the map but I think that the cost it's minimal
        let progress = self.load_len() as f32 / self.len as f32;
        progress
    }

    pub fn load_len(&self) -> usize {
        self.inner.iter().fold(0, |count, (_, data)| {
            if is_loaded(&data.id) {
                count + 1
            } else {
                count
            }
        })
    }

    pub fn is_loaded(&self) -> bool {
        self.load_len() >= self.len
    }

    pub fn with_extension_parser<T, F>(mut self, ext: &str, parser: F) -> Self
    where
        F: Fn(&str, &[u8]) -> Result<T, String> + 'static + Clone,
        T: 'static,
    {
        self.parsers.insert(
            ext.to_string(),
            Box::new(move |aid: &AssetId, id: &str, map: &mut AnyMap| {
                let parsed = super::parse_asset::<T, F>(aid, parser.clone(), false)?;
                if let Some(parsed_asset) = parsed {
                    map.insert(parsed_asset);
                }

                Ok(())
            }),
        );
        self
    }

    pub fn parse<T, F>(&mut self, parser: F) -> Result<Option<T>, String>
    where
        F: FnOnce(&AnyMap) -> Result<T, String>,
    {
        for (path, data) in &self.inner {
            if data.loaded || is_loading(&data.id) {
                continue;
            }

            let ext = path.split('.').last().and_then(|ext| self.parsers.get(ext));

            match ext {
                // use the parser provided to store the asset as the type needed
                Some(parser) => (*parser)(&data.id, path, &mut self.parsed)?,

                // parse as Vec<u8> if there is no parser added for this extension
                _ => {
                    let parsed = super::parse_asset(&data.id, parse_vec, false)?;
                    if let Some(parsed_asset) = parsed {
                        self.parsed.insert(parsed_asset);
                    }
                }
            }
        }

        if !self.is_loaded() {
            return Ok(None);
        }

        parser(&self.parsed).map(|res| Some(res))
    }
}

fn parse_vec(_id: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    Ok(data.to_vec())
}
