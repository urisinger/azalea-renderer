#[derive(serde::Deserialize, Debug)]
pub enum BlockRenderState {
    #[serde(rename = "variants")]
    Variants(Variants),

    #[serde(rename = "multi_part")]
    MultiPart(()),
}

impl BlockRenderState {
    fn from_str(s: &str) -> serde_json::Result<Self> {
        serde_json::from_str(s)
    }
}

#[derive(serde::Deserialize, Debug)]
#[serde(untagged)]
pub enum Variants {
    Array(Vec<Variant>),
    #[serde(with = "tuple_vec_map")]
    Map(Vec<(String, Variant)>),
}

#[derive(serde::Deserialize, Debug)]
#[serde(untagged)]
pub enum Variant {
    Single(VariantDesc),
    Array(Vec<VariantDesc>),
}

#[derive(serde::Deserialize, Debug)]
pub struct VariantDesc {
    pub model: String,
    #[serde(default)]
    pub y: i32,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub ublock: bool,
}

#[cfg(test)]
mod tests {
    use super::BlockRenderState;

    #[test]
    fn deserialize_test() {
        {
            _ = BlockRenderState::from_str(
                r#"
{
    "variants": {
        "snowy=false": [
            { "model": "block/grass_block" },
            { "model": "block/grass_block", "y": 90 },
            { "model": "block/grass_block", "y": 180 },
            { "model": "block/grass_block", "y": 270 }
        ],
        "snowy=true":  { "model": "block/grass_block_snow" }
    }
}
                "#,
            )
            .unwrap();
        }
    }
}
