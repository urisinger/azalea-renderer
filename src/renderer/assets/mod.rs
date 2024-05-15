pub mod block_state;
pub mod model;

pub mod texture;

use std::collections::HashMap;

use self::model::Cube;

pub struct BlockModelRef<'a> {
    pub ambientocclusion: bool,
    pub parent: &'a Option<BlockModelRef<'a>>,
    pub textures: &'a HashMap<String, String>,
    pub elements: &'a Option<Vec<Cube>>,
}

pub struct LoadedAssets {}
