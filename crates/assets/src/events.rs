#[derive(Clone, Debug)]
pub(crate) struct AssetLoad {
    pub(crate) id: String,
    pub(crate) state: AssetState,
}

impl AssetLoad {
    /// Id used to load the asset
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Blob buffer
    pub fn data(&self) -> Result<Option<&[u8]>, String> {
        match &self.state {
            AssetState::Loading => Ok(None),
            AssetState::Loaded(buff) => Ok(Some(buff.as_ref())),
            AssetState::Err(err) => Err(err.clone()),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum AssetState {
    Loading,
    Loaded(Vec<u8>),
    Err(String),
}
