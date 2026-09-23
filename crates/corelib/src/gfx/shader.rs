use std::sync::Arc;

#[derive(Clone)]
pub struct Shader {
    pub(crate) raw: Arc<wgpu::ShaderModule>,
    pub(crate) module: Arc<wgpu::naga::Module>,
    pub(crate) info: Arc<wgpu::naga::valid::ModuleInfo>,
}

#[derive(Clone)]
pub enum ShaderInput<'a> {
    Source(&'a str),
    Shader(&'a Shader),
}

impl<'a> From<&'a str> for ShaderInput<'a> {
    fn from(source: &'a str) -> Self {
        Self::Source(source)
    }
}

impl<'a> From<&'a Shader> for ShaderInput<'a> {
    fn from(shader: &'a Shader) -> Self {
        Self::Shader(shader)
    }
}

pub struct ShaderBuilder<'a> {
    source: &'a str,
}

impl<'a> ShaderBuilder<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        Self { source }
    }

    pub fn build(self) -> Result<Shader, String> {
        get_mut_backend().gfx().create_shader(self.source)
    }
}

use crate::backend::{BackendImpl, GfxBackendImpl, get_mut_backend};
