use thiserror::Error;

pub struct Texture {
    pub image: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

#[derive(Debug, Error)]
pub enum TextureLoadError {}

impl Texture {
    pub fn new() -> Result<Self, TextureLoadError> {}
}
