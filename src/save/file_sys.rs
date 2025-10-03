#![cfg(not(target_arch = "wasm32"))]

use super::driver::SaveDriverImpl;
use std::{fs, path::PathBuf};

pub struct FileSysSaveDriver;

impl SaveDriverImpl for FileSysSaveDriver {
    fn ensure_dir(dir: &PathBuf) -> Result<(), String> {
        fs::create_dir_all(dir).map_err(|e| format!("Cannot create save directory: {e}"))
    }
    fn write_bytes(path: &PathBuf, bytes: &[u8]) -> Result<(), String> {
        fs::write(path, bytes).map_err(|e| format!("Write error: {e}"))
    }
    fn read_bytes(path: &PathBuf) -> Result<Vec<u8>, String> {
        fs::read(path).map_err(|e| format!("File read error: {e}"))
    }
    fn exists(path: &PathBuf) -> Result<bool, String> {
        fs::exists(path).map_err(|e| e.to_string())
    }
    fn rename(src: &PathBuf, dst: &PathBuf) -> Result<(), String> {
        fs::rename(src, dst).map_err(|e| format!("Rename file error: {e}"))
    }
    fn read_dir(dir: &PathBuf) -> Result<Vec<PathBuf>, String> {
        let mut out = Vec::new();
        for e in fs::read_dir(dir).map_err(|e| e.to_string())? {
            out.push(e.map_err(|e| e.to_string())?.path());
        }
        Ok(out)
    }
    fn remove_file(path: &PathBuf) -> Result<(), String> {
        fs::remove_file(path).map_err(|e| format!("Failed to delete '{}': {e}", path.display()))
    }
}
