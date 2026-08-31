//! Decoder for the repo's OWN generated `.glb` files (`scripts/nova_glb.py`:
//! one node, float32 POSITION + NORMAL, u32 indices, flat `baseColorFactor`
//! materials), for galleries that show `art/part-candidates` models.
//!
//! `art/` is deliberately not an asset source (it ships in no build), and
//! bevy's default `UnapprovedPathMode::Forbid` refuses a `../` escape from
//! `assets/` - rightly, for the game. Registering a source is not an option
//! either once `AppBuilder::new()` has added `DefaultPlugins`, so a candidate
//! gallery decodes the files itself: a page of code against a format this
//! repo controls. Shipped Blender exports carry node transforms and u16
//! indices this decoder rejects on purpose - load those through the asset
//! server instead.
//!
//! Included with `#[path = "shared/glb.rs"] mod glb;`, kit-style.

// Each producer includes the whole kit and uses the part its scene needs; the
// unused half is not dead code, it is another gallery's tool.
#![allow(
    dead_code,
    reason = "one source, many example targets: what one producer leaves unused another needs, so no single build can fulfil an expectation"
)]

use std::path::Path;

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

/// One flat-colour primitive out of a generated glb.
pub struct GlbPrimitive {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub colour: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
}

impl GlbPrimitive {
    /// The primitive as a bevy [`Mesh`].
    pub fn mesh(&self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals.clone())
        .with_inserted_indices(Indices::U32(self.indices.clone()))
    }

    /// The primitive's flat material, read the way bevy's own loader would.
    pub fn material(&self) -> StandardMaterial {
        StandardMaterial {
            // glTF factors are linear, as bevy's own loader reads them;
            // double-sided without culling matches it too.
            base_color: Color::linear_rgba(
                self.colour[0],
                self.colour[1],
                self.colour[2],
                self.colour[3],
            ),
            metallic: self.metallic,
            perceptual_roughness: self.roughness,
            double_sided: true,
            cull_mode: None,
            ..default()
        }
    }
}

/// The axis-aligned bounds over every primitive, as `(centre, size)`.
pub fn bounds(primitives: &[GlbPrimitive]) -> (Vec3, Vec3) {
    let (low, high) = primitives
        .iter()
        .flat_map(|primitive| &primitive.positions)
        .fold((Vec3::MAX, Vec3::MIN), |(low, high), position| {
            let position = Vec3::from_array(*position);
            (low.min(position), high.max(position))
        });
    ((low + high) * 0.5, high - low)
}

/// Decode a `scripts/nova_glb.py` glb. Panics on a missing file or a shape
/// the writer never produces - these are our own generated files, and a
/// gallery silently skipping a candidate would defeat it.
pub fn read_glb(path: &Path) -> Vec<GlbPrimitive> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("glb: read {path:?}: {e}"));
    let word =
        |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().expect("glb header word"));
    assert_eq!(word(0), 0x4654_6C67, "{path:?} is not a glb (magic)");

    // The two chunks: JSON first, then the one binary buffer.
    let mut json: Option<&[u8]> = None;
    let mut bin: &[u8] = &[];
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let (length, kind) = (word(at) as usize, word(at + 4));
        let data = &bytes[at + 8..at + 8 + length];
        match kind {
            0x4E4F_534A => json = Some(data),
            0x004E_4942 => bin = data,
            _ => {}
        }
        at += 8 + length;
    }
    let doc: serde_json::Value = serde_json::from_slice(json.expect("glb JSON chunk"))
        .unwrap_or_else(|e| panic!("glb: parse {path:?}: {e}"));

    // An accessor's raw bytes: the writer packs one accessor per buffer view,
    // tightly, at the view's offset.
    let accessor_bytes = |index: u64| -> &[u8] {
        let accessor = &doc["accessors"][index as usize];
        let view = &doc["bufferViews"][accessor["bufferView"].as_u64().expect("view") as usize];
        let offset = view["byteOffset"].as_u64().unwrap_or(0) as usize;
        let length = view["byteLength"].as_u64().expect("view length") as usize;
        &bin[offset..offset + length]
    };
    let floats3 = |index: u64| -> Vec<[f32; 3]> {
        accessor_bytes(index)
            .as_chunks::<12>()
            .0
            .iter()
            .map(|chunk| {
                [
                    f32::from_le_bytes(chunk[0..4].try_into().expect("f32")),
                    f32::from_le_bytes(chunk[4..8].try_into().expect("f32")),
                    f32::from_le_bytes(chunk[8..12].try_into().expect("f32")),
                ]
            })
            .collect()
    };

    let factor = |material: &serde_json::Value, name: &str, default: f64| -> f32 {
        material["pbrMetallicRoughness"][name]
            .as_f64()
            .unwrap_or(default) as f32
    };
    doc["meshes"][0]["primitives"]
        .as_array()
        .expect("glb primitives")
        .iter()
        .map(|primitive| {
            let material =
                &doc["materials"][primitive["material"].as_u64().expect("material") as usize];
            let colour = material["pbrMetallicRoughness"]["baseColorFactor"]
                .as_array()
                .map(|values| std::array::from_fn(|i| values[i].as_f64().unwrap_or(1.0) as f32))
                .unwrap_or([1.0; 4]);
            GlbPrimitive {
                positions: floats3(primitive["attributes"]["POSITION"].as_u64().expect("pos")),
                normals: floats3(primitive["attributes"]["NORMAL"].as_u64().expect("nrm")),
                indices: accessor_bytes(primitive["indices"].as_u64().expect("idx"))
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|chunk| u32::from_le_bytes(*chunk))
                    .collect(),
                colour,
                metallic: factor(material, "metallicFactor", 1.0),
                roughness: factor(material, "roughnessFactor", 1.0),
            }
        })
        .collect()
}
