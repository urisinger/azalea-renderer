pub mod block_state;
pub mod model;

pub mod texture;

use texture::Texture;

use std::{
    collections::{hash_map, HashMap},
    fs,
    io::BufReader,
    path::PathBuf,
};

use log::*;

use self::model::{BlockModel, Cube};

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

    path: PathBuf,
}

impl LoadedAssets {
    pub fn from_path(device: &wgpu::Device, queue: &wgpu::Queue, path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut this = Self {
            texture_to_id: HashMap::new(),
            textures: Vec::new(),
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

            println!(
                "adding texture: {}, from path: {}",
                path.to_str().unwrap(),
                texture_path.to_string_lossy()
            );
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
        for path in fs::read_dir(&model_path).unwrap() {
            let path = path.unwrap().path();
            let mut name = "models/".to_string();
            name.push_str(path.strip_prefix(&model_path).unwrap().to_str().unwrap());
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
                None => Some(BlockModelRef {
                    ambient_occlusion: block_model.ambientocclusion,
                    parent: None,
                    textures: &block_model.textures,
                    elements: &block_model.elements,
                    assets: self,
                }),

                Some(None) => None,
            }
        } else {
            None
        }
    }
}
