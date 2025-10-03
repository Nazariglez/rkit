use std::path::{Path, PathBuf};

pub(super) trait SaveDriverImpl {
    fn ensure_dir(dir: &Path) -> Result<(), String>;
    fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), String>;
    fn read_bytes(path: &Path) -> Result<Vec<u8>, String>;
    fn exists(path: &Path) -> Result<bool, String>;
    fn rename(src: &Path, dst: &Path) -> Result<(), String>;
    fn read_dir(dir: &Path) -> Result<Vec<PathBuf>, String>;
    fn remove_file(path: &Path) -> Result<(), String>;
}

#[cfg(target_arch = "wasm32")]
pub type SaveDriver = super::local_storage::LocalStorageSaveDriver;

#[cfg(not(target_arch = "wasm32"))]
pub type SaveDriver = super::file_sys::FileSysSaveDriver;
