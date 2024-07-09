pub mod block_state;
pub mod model;

pub mod texture;

use bevy_ecs::system::{ResMut, Resource};

use std::{collections::HashMap, fs, io::BufReader, path::PathBuf};

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
}

impl<'a> BlockModelRef<'a> {
    pub fn elements(&self) -> Option<&'a Vec<Cube>> {
        if let Some(elements) = self.elements {
            Some(elements)
        } else {
            if let Some(parent) = &self.parent {
                parent.elements()
            } else {
                None
            }
        }
    }

    pub fn get_texture(&self, name: &str) -> Option<String> {
        self.get_texture_helper(self, name)
    }

    fn get_texture_helper(&self, top: &Self, name: &str) -> Option<String> {
        let name = name.strip_prefix("minecraft:").unwrap_or(name);
        let texture = if let Some(strip_name) = name.strip_prefix('#') {
            self.textures
                .get(strip_name)
                .map(|n| top.get_texture_helper(top, n))
                .unwrap_or_else(|| {
                    self.parent
                        .as_ref()
                        .map(|parent| parent.get_texture_helper(top, name))
                        .flatten()
                })
        } else {
            Some(name.to_owned())
        };

        if texture.is_none() {
            error!(
                "could not load texture {}, from textures: {:?}",
                name, self.textures
            );
        }
        texture
    }
}

#[derive(Debug, Resource)]
pub struct TextureIdMap(HashMap<String, usize>);

#[derive(Debug, Resource)]
pub struct LoadedAssets {
    block_models: HashMap<String, BlockModel>,

    block_states: HashMap<String, BlockRenderState>,
}

impl LoadedAssets {
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut block_states = HashMap::new();
        let mut block_models = HashMap::new();

        let block_model_path = path.join("models/block");
        for path in walkdir::WalkDir::new(&block_model_path) {
            let path = path.unwrap().path().to_owned();

            if !path.is_file() || !path.extension().map_or(false, |e| e == "json") {
                continue;
            }

            let mut name = "block/".to_string();

            name.push_str(
                path.strip_prefix(&block_model_path)
                    .unwrap()
                    .with_extension("")
                    .to_str()
                    .unwrap(),
            );

            let s = fs::read_to_string(path).unwrap();

            block_models.insert(name, BlockModel::from_str(&s).unwrap());
        }

        let block_state_path = path.join("blockstates");

        for path in walkdir::WalkDir::new(&block_state_path) {
            let path = path.unwrap().path().to_owned();

            if !path.is_file() || !path.extension().map_or(false, |e| e == "json") {
                continue;
            }

            let mut name = "block/".to_string();
            name.push_str(
                path.strip_prefix(&block_state_path)
                    .unwrap()
                    .with_extension("")
                    .to_str()
                    .unwrap(),
            );

            let s = fs::read_to_string(path).unwrap();

            block_states.insert(name, BlockRenderState::from_str(&s).unwrap());
        }

        Self {
            block_models,
            block_states,
        }
    }

    pub fn get_block_state(&self, name: &str) -> Option<&BlockRenderState> {
        self.block_states.get(name)
    }

    pub fn get_block_model<'a>(&'a self, name: &str) -> Option<BlockModelRef<'a>> {
        let name = name.strip_prefix("minecraft:").unwrap_or(name);
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
                }),

                Some(None) => None,
                _ => Some(BlockModelRef {
                    ambient_occlusion: block_model.ambientocclusion,
                    parent: None,
                    textures: &block_model.textures,
                    elements: &block_model.elements,
                }),
            }
        } else {
            None
        }
    }
}
