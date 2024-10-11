use bytes::Bytes;

#[derive(Clone, Debug)]
pub struct AssetLoad {
    pub(crate) id: String,
    pub(crate) state: AssetState,
}

impl AssetLoad {
    /// Id used to load the asset
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Blob buffer
    pub fn data(&self) -> Result<&[u8], String> {
        match &self.state {
            AssetState::Loaded(buff) => Ok(buff.as_ref()),
            AssetState::Err(err) => Err(err.clone()),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum AssetState {
    Loaded(Bytes),
    Err(String),
}
