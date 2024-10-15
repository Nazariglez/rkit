mod events;
mod list;
mod load_file;
mod loader;
mod waker;

pub use crate::list::{AssetList, AssetMap};
pub use crate::loader::AssetId;

use crate::loader::ASSET_LOADER;

#[inline]
pub fn load_asset(file_path: &str) -> AssetId {
    ASSET_LOADER.borrow_mut().load(file_path)
}

/// Used by system to pull the assets futures
#[inline]
pub(crate) fn update_assets() {
    ASSET_LOADER.borrow_mut().update();
}

#[inline]
pub fn is_loaded(id: &AssetId) -> bool {
    ASSET_LOADER.borrow().is_loaded(*id)
}

#[inline]
pub fn is_loading(id: &AssetId) -> bool {
    ASSET_LOADER.borrow().is_loading(*id)
}

#[inline]
pub fn parse_asset<T, F>(id: &AssetId, parser: F, keep: bool) -> Result<Option<T>, String>
where
    F: FnOnce(&str, &[u8]) -> Result<T, String>,
{
    ASSET_LOADER.borrow_mut().parse(*id, parser, keep)
}

#[inline]
pub fn clear_assets() {
    ASSET_LOADER.borrow_mut().clear();
}
