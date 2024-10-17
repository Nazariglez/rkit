#[derive(Clone, Debug)]
pub(crate) struct AssetLoad {
    pub(crate) id: String,
    pub(crate) state: AssetState,
}

#[derive(Clone, Debug)]
pub(crate) enum AssetState {
    Loading,
    Loaded(Vec<u8>),
    Err(String),
}
