pub mod block_state;
pub mod model;

pub mod texture;

use texture::Texture;

use std::{collections::HashMap, error::Error, fs, io::BufReader, path::PathBuf};

use log::*;

use self::{
    block_state::BlockRenderState,
    model::{BlockModel, Cube},
};

pub struct BlockModelRef<'a> {
    pub ambient_occlusion: bool,
    pub parent: Option<Box<BlockModelRef<'a>>>,
    pub textures: &'a HashMap<String, String>,
    pub elements: &'a Option<Vec<Cube>>,

    pub assets: &'a LoadedAssets,
}

pub struct LoadedAssets {
    texture_to_id: HashMap<String, usize>,

    textures: Vec<Texture>,

    block_models: HashMap<String, BlockModel>,

    block_states: HashMap<String, BlockRenderState>,

    path: PathBuf,
}

impl LoadedAssets {
    pub fn from_path(device: &wgpu::Device, queue: &wgpu::Queue, path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut this = Self {
            texture_to_id: HashMap::new(),
            textures: Vec::new(),
            block_states: HashMap::new(),
            block_models: HashMap::new(),
            path: path.clone(),
        };

        let texture_path = path.join("textures");

        info!("loading textures from {}", texture_path.to_str().unwrap());

        for path in walkdir::WalkDir::new(&texture_path) {
            let path = path.unwrap().path().to_owned();

            if !path.is_file() || !path.extension().map_or(false, |e| e == "png") {
                continue;
            }

            let mut name = "textures/".to_string();

            name.push_str(
                path.strip_prefix(&texture_path)
                    .unwrap()
                    .with_extension("")
                    .to_str()
                    .unwrap(),
            );
            this.add_texture(
                Texture::new(device, queue, BufReader::new(fs::File::open(path).unwrap())).unwrap(),
                name,
            );
        }

        let model_path = path.join("models");
        for path in walkdir::WalkDir::new(&model_path) {
            let path = path.unwrap().path().to_owned();

            if !path.is_file() || !path.extension().map_or(false, |e| e == "json") {
                continue;
            }

            let mut name = "models/".to_string();
            info!(
                "adding model: {}, from path: {}",
                path.to_str().unwrap(),
                model_path.to_string_lossy()
            );
            name.push_str(path.strip_prefix(&model_path).unwrap().to_str().unwrap());

            let s = fs::read_to_string(path).unwrap();
            this.add_block_model(BlockModel::from_str(&s).unwrap(), name);
        }

        let block_state_path = path.join("blockstates");
        for path in walkdir::WalkDir::new(&block_state_path) {
            let path = path.unwrap().path().to_owned();

            if !path.is_file() || !path.extension().map_or(false, |e| e == "json") {
                continue;
            }

            let mut name = "blockstates/".to_string();
            info!(
                "adding blockstate: {}, from path: {}\n",
                path.to_str().unwrap(),
                model_path.to_string_lossy()
            );
            name.push_str(
                path.strip_prefix(&block_state_path)
                    .unwrap()
                    .to_str()
                    .unwrap(),
            );

            let s = fs::read_to_string(path).unwrap();
            this.add_block_state(BlockRenderState::from_str(&s).unwrap(), name);
        }

        this
    }

    pub fn add_texture(&mut self, texture: Texture, name: String) {
        self.texture_to_id.insert(name, self.textures.len());
        self.textures.push(texture);
    }

    pub fn get_texture_id(&self, name: &str) -> Option<usize> {
        self.texture_to_id.get(name).copied()
    }

    pub fn add_block_state(&mut self, block_state: BlockRenderState, name: String) {
        self.block_states.insert(name, block_state);
    }

    pub fn get_block_state(&self, name: &str) -> Option<&BlockRenderState> {
        self.block_states.get(name)
    }

    pub fn add_block_model(&mut self, model: BlockModel, name: String) {
        self.block_models.insert(name, model);
    }

    pub fn get_block_model<'a>(&'a self, name: &str) -> Option<BlockModelRef<'a>> {
        if let Some(block_model) = self.block_models.get(name) {
            let parent = block_model
                .parent
                .clone()
                .map(|parent| self.get_block_model(&parent));
            match parent {
                Some(Some(parent)) => Some(BlockModelRef {
                    ambient_occlusion: block_model.ambientocclusion,
                    parent: Some(Box::new(parent)),
                    textures: &block_model.textures,
                    elements: &block_model.elements,
                    assets: self,
                }),

                Some(None) => None,
                _ => Some(BlockModelRef {
                    ambient_occlusion: block_model.ambientocclusion,
                    parent: None,
                    textures: &block_model.textures,
                    elements: &block_model.elements,
                    assets: self,
                }),
            }
        } else {
            None
        }
    }
}
