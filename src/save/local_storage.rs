#![cfg(target_arch = "wasm32")]

use std::collections::HashMap;
use std::path::{Component, PathBuf};

use super::driver::SaveDriverImpl;
use web_sys::Storage;

pub struct LocalStorageSaveDriver;

impl SaveDriverImpl for LocalStorageSaveDriver {
    fn ensure_dir(_dir: &PathBuf) -> Result<(), String> {
        Ok(())
    }

    fn write_bytes(path: &PathBuf, bytes: &[u8]) -> Result<(), String> {
        let root = root_key_of(path)?;
        let key = key_of(path);
        let val = base64::encode(bytes);
        let s = local_storage()?;
        let mut map = storage_get_map(&s, &root)?;
        map.insert(key, val);
        storage_set_map(&s, &root, &map)
    }

    fn read_bytes(path: &PathBuf) -> Result<Vec<u8>, String> {
        let root = root_key_of(path)?;
        let key = key_of(path);
        let s = local_storage()?;
        let map = storage_get_map(&s, &root)?;
        let v = map.get(&key).ok_or_else(|| "Save not found".to_string())?;
        base64::decode(v).map_err(|e| format!("Decode error: {e}"))
    }

    fn exists(path: &PathBuf) -> Result<bool, String> {
        let root = root_key_of(path)?;
        let key = key_of(path);
        let s = local_storage()?;
        let map = storage_get_map(&s, &root)?;
        Ok(map.contains_key(&key))
    }

    fn rename(src: &PathBuf, dst: &PathBuf) -> Result<(), String> {
        let s = local_storage()?;
        let src_root = root_key_of(src)?;
        let dst_root = root_key_of(dst)?;
        let src_k = key_of(src);
        let dst_k = key_of(dst);

        let mut src_map = storage_get_map(&s, &src_root)?;
        let v = src_map
            .remove(&src_k)
            .ok_or_else(|| "rename: source missing".to_string())?;

        if src_root == dst_root {
            src_map.insert(dst_k, v);
            return storage_set_map(&s, &src_root, &src_map);
        }

        storage_set_map(&s, &src_root, &src_map)?;
        let mut dst_map = storage_get_map(&s, &dst_root)?;
        dst_map.insert(dst_k, v);
        storage_set_map(&s, &dst_root, &dst_map)
    }

    fn read_dir(dir: &PathBuf) -> Result<Vec<PathBuf>, String> {
        let s = local_storage()?;
        let root = root_key_of(dir)?;
        let map = storage_get_map(&s, &root)?;
        let prefix = prefix_of(dir);
        let mut out = Vec::new();
        for k in map.keys() {
            if k.starts_with(&prefix) {
                out.push(PathBuf::from(k));
            }
        }
        Ok(out)
    }

    fn remove_file(path: &PathBuf) -> Result<(), String> {
        let root = root_key_of(path)?;
        let key = key_of(path);
        let s = local_storage()?;
        let mut map = storage_get_map(&s, &root)?;
        map.remove(&key);
        storage_set_map(&s, &root, &map)
    }
}

fn local_storage() -> Result<Storage, String> {
    web_sys::window()
        .ok_or_else(|| "No windows".to_string())?
        .local_storage()
        .map_err(|e| format!("Cannot access LocalStorage: {e}"))?
        .ok_or_else(|| "LocalStorage unavailable".to_string())
}

fn storage_get_map(s: &Storage, root: &str) -> Result<HashMap<String, String>, String> {
    match s
        .get_item(root)
        .map_err(|_| "LocalStorage get_item failed".to_string())?
    {
        Some(text) => serde_json::from_str(text).map_err(|_| "Invalid namespace data".to_string()),
        None => Ok(HashMap::new()),
    }
}

fn storage_set_map(s: &Storage, root: &str, map: &HashMap<String, String>) -> Result<(), String> {
    let text = serde_json::to_string(map).map_err(|_| "Invalid namespace data".to_string())?;
    s.set_item(root, &text)
        .map_err(|_| "LocalStorage set_item failed".to_string())
}

fn root_key_of(path: &PathBuf) -> Result<String, String> {
    match path.components().next() {
        Some(Component::Normal(os)) => Ok(os.to_string_lossy().to_string()),
        _ => Err("Invalid save namespace".to_string()),
    }
}

fn key_of(path: &PathBuf) -> String {
    path.to_string_lossy().to_string()
}

fn prefix_of(dir: &PathBuf) -> String {
    let mut p = dir.to_string_lossy().to_string();
    if !p.ends_with('/') {
        p.push('/');
    }
    p
}
