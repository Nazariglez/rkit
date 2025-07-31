use corelib::math::{Rect, ivec2};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::Sprite;

/// Returns a HashMap containing a list of textures created from the Atlas data
pub fn create_sprites_from_spritesheet(
    data: &[u8],
    spritesheet: &Sprite,
) -> Result<FxHashMap<String, Sprite>, String> {
    let data = atlas_from_bytes(data)?;
    let mut textures = FxHashMap::default();
    data.frames.iter().for_each(|af| {
        textures.insert(
            af.filename.clone(),
            spritesheet.clone_with_frame(af.frame.into()),
        );
    });
    Ok(textures)
}

#[inline]
fn atlas_from_bytes(data: &[u8]) -> Result<AtlasRoot, String> {
    serde_json::from_slice(data).map_err(|e| e.to_string())
}

#[derive(Serialize, Deserialize, Debug)]
struct AtlasRoot {
    frames: Vec<AtlasFrame>,
    meta: AtlasMeta,
}

#[derive(Serialize, Deserialize, Debug)]
struct AtlasFrame {
    filename: String,
    frame: AtlasRect,
    rotated: bool,
    trimmed: bool,
    #[serde(alias = "spriteSourceSize")]
    sprite_source_size: AtlasRect,
    #[serde(alias = "sourceSize")]
    source_size: AtlasSize,
    #[serde(default)]
    pivot: AtlasPoint,
}

#[derive(Serialize, Deserialize, Debug)]
struct AtlasMeta {
    app: String,
    version: String,
    image: String,
    format: String,
    size: AtlasSize,
    scale: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
struct AtlasPoint {
    x: f32,
    y: f32,
}

impl Default for AtlasPoint {
    fn default() -> Self {
        Self { x: 0.5, y: 0.5 }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
struct AtlasSize {
    w: i32,
    h: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
struct AtlasRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl From<AtlasRect> for Rect {
    fn from(val: AtlasRect) -> Self {
        Rect::new(ivec2(val.x, val.y).as_vec2(), ivec2(val.w, val.h).as_vec2())
    }
}
