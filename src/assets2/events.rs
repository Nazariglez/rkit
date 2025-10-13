#[derive(Clone, Debug)]
pub(crate) struct AssetLoad {
    pub(crate) id: String,
    pub(crate) state: LoadState,
}

#[allow(unused)]
#[derive(Clone, Debug)]
pub(crate) enum LoadState {
    Loading,
    Loaded(Vec<u8>),
    Err(String),
}
