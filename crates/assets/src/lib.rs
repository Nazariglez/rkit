mod events;
mod load_file;
mod loader;
mod waker;

use crate::loader::ASSET_LOADER;
pub use events::AssetLoad;

#[inline]
pub fn load_asset(file_path: &str) {
    ASSET_LOADER.borrow_mut().load(file_path);
}

#[inline]
pub fn update_assets() {
    ASSET_LOADER.borrow_mut().update();
}

#[inline]
pub fn get_asset(id: &str) -> Option<&AssetLoad> {
    // ASSET_LOADER.borrow().get(id)
    todo!()
}

// TODO this should not be exposed and used automatically on frame end event
#[inline]
pub fn clean() {
    ASSET_LOADER.borrow_mut().clean();
}
