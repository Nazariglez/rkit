use std::path::PathBuf;

pub(super) trait SaveDriverImpl {
    fn ensure_dir(dir: &PathBuf) -> Result<(), String>;
    fn write_bytes(path: &PathBuf, bytes: &[u8]) -> Result<(), String>;
    fn read_bytes(path: &PathBuf) -> Result<Vec<u8>, String>;
    fn exists(path: &PathBuf) -> Result<bool, String>;
    fn rename(src: &PathBuf, dst: &PathBuf) -> Result<(), String>;
    fn read_dir(dir: &PathBuf) -> Result<Vec<PathBuf>, String>;
    fn remove_file(path: &PathBuf) -> Result<(), String>;
}

#[cfg(target_arch = "wasm32")]
pub type SaveDriver = super::local_storage::LocalStorageSaveDriver;

#[cfg(not(target_arch = "wasm32"))]
pub type SaveDriver = super::file_sys::FileSysSaveDriver;
