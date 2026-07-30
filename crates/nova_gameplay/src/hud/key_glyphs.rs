//! Key-glyph lookup for the HUD: which keycap picture stands for which binding.
//!
//! The HUD shows keys as PICTURES, not `[BRACKETED]` text - the icon dock, the
//! anchored verb cues and the objective stack's NOVA OS affordance all draw a
//! keycap from `assets/input-prompts/keyboard/Alt/` (the dark keycaps with white
//! glyphs, relocated into `assets/` by task 20260728-233707).
//!
//! This module owns the mapping, and with it the explicit asset path list that
//! [`nova_assets`]'s `GameAssets::key_glyphs` collection preloads: the glyphs
//! load and load-gate like every other static asset (the pattern established by
//! task 20260729-000956 for the UI SFX), never lazily per chip. A FOLDER
//! collection cannot be used - folder collections do not work on wasm - so the
//! path list is explicit and `key_glyph_collection_matches_mapping_table`
//! (in `nova_assets`) pins the two together.
//!
//! Keys are addressed by their DISPLAY LABEL - the string
//! `nova_gameplay::input::player`'s `keyboard_label` produces for a `KeyCode`
//! (`"X"`, `"Space"`, `"ControlLeft"`), plus the handful of fixed pseudo-labels
//! the flight rig uses for gestures that have no single key (`"CTRL"`,
//! `"SHIFT"`, `"SCROLL"`). That is exactly what `FlightVerbHints` carries, so
//! the HUD looks a glyph up with the string it already has.
//!
//! Unmapped labels resolve to `None`, and every consumer falls back to a TEXT
//! chip - a rebind to an unmapped key degrades to the old look instead of
//! rendering an empty box. The remapping/gamepad follow-up (20260710-231927)
//! may `server.load` a glyph for a runtime-rebound key: that is dynamic content
//! and cannot sit behind a one-shot preload collection.

use bevy::{platform::collections::HashMap, prelude::*};

/// Glob-import surface: `use nova_gameplay::hud::key_glyphs::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::{
        key_glyph_asset_paths, key_glyph_stem, trimmed_cap, KeyCap, KeyGlyphs, KEY_GLYPH_DIR,
    };
}

/// Where the keycap art lives, relative to `assets/`.
pub const KEY_GLYPH_DIR: &str = "input-prompts/keyboard/Alt";

/// The mapping: display label -> keycap file stem under [`KEY_GLYPH_DIR`].
///
/// Covers every key the flight rig binds (`flight_rig_reserved_sources`) plus
/// the HUD's own chrome keys (Tab for the NOVA OS, the backquote HUD-level
/// cycle) and the fixed gesture pseudo-labels. Two upstream filenames are
/// misspelled/abbreviated and are pinned here so a rename is caught by
/// `every_bound_key_maps_to_an_existing_glyph_asset`: `T_Crtl_Key_Alt` (the
/// upstream typo for Ctrl) and `T_Brackets_L/R_Key_Alt`.
pub const KEY_GLYPH_FILES: &[(&str, &str)] = &[
    // Flight verbs.
    ("X", "T_X_Key_Alt"),
    ("G", "T_G_Key_Alt"),
    ("O", "T_O_Key_Alt"),
    ("Z", "T_Z_Key_Alt"),
    ("W", "T_W_Key_Alt"),
    ("Space", "T_Space_Key_Alt"),
    // Modifier gestures: both physical sides share one keycap, and the flight
    // rig's fixed pseudo-labels ("CTRL"/"SHIFT") land on the same art.
    ("CTRL", "T_Crtl_Key_Alt"),
    ("ControlLeft", "T_Crtl_Key_Alt"),
    ("ControlRight", "T_Crtl_Key_Alt"),
    ("SHIFT", "T_Shift_Key_Alt"),
    ("ShiftLeft", "T_Shift_Key_Alt"),
    ("ShiftRight", "T_Shift_Key_Alt"),
    // Component fine-lock cycle: the wheel gesture (the hint's label) and the
    // two bracket keys that step it discretely.
    ("SCROLL", "T_Mouse_Scroll_Key_Dark_Key_Alt"),
    ("BracketLeft", "T_Brackets_L_Key_Alt"),
    ("BracketRight", "T_Brackets_R_Key_Alt"),
    // HUD chrome.
    ("Tab", "T_Tab_Key_Alt"),
    ("TAB", "T_Tab_Key_Alt"),
    ("Backquote", "T_Tilde_Key_Alt"),
];

/// The keycap file stem for `label`, or `None` when the key has no art (the
/// caller then falls back to a text chip).
pub fn key_glyph_stem(label: &str) -> Option<&'static str> {
    KEY_GLYPH_FILES
        .iter()
        .find(|(key, _)| *key == label)
        .map(|(_, stem)| *stem)
}

/// The DISTINCT asset paths the mapping references, sorted - the list
/// `GameAssets::key_glyphs` must preload. Several labels share one keycap, so
/// this is shorter than [`KEY_GLYPH_FILES`].
pub fn key_glyph_asset_paths() -> Vec<String> {
    let mut paths: Vec<String> = KEY_GLYPH_FILES
        .iter()
        .map(|(_, stem)| format!("{KEY_GLYPH_DIR}/{stem}.png"))
        .collect();
    paths.sort();
    paths.dedup();
    paths
}

/// One preloaded keycap: the picture, plus the sub-rect the drawn cap actually
/// occupies inside it.
///
/// Every file under [`KEY_GLYPH_DIR`] is a 128x128 canvas, but the caps are NOT
/// square: a letter cap is drawn 96x104, the wide modifiers (Tab, Shift, Ctrl)
/// 112x74, Space 128x68, and the mouse 76x128 - each centred in the canvas with
/// transparent bands around it. Rendering that canvas into a square box throws
/// the difference away, which cost the wide caps ~40% of their height and made
/// their legends unreadable (owner playtest 2026-07-30, task 20260730-122940).
///
/// So a cap carries its own bounds, measured from the alpha channel at load
/// ([`KeyGlyphs::measure_caps`]), and every HUD site sizes from them: HEIGHT is
/// pinned to the site's constant, width follows the aspect. `cap` is `None`
/// until the scan runs (and on bare rigs that never load pixels), where the
/// whole canvas at a 1:1 box is the old, harmless behaviour.
#[derive(Clone, Debug)]
pub struct KeyCap {
    image: Handle<Image>,
    cap: Option<Rect>,
}

impl KeyCap {
    /// The keycap picture.
    pub fn image(&self) -> &Handle<Image> {
        &self.image
    }

    /// The drawn cap's bounds inside the canvas, in texture pixels - `None`
    /// while unmeasured.
    pub fn cap(&self) -> Option<Rect> {
        self.cap
    }

    /// The cap's width:height. 1.0 while unmeasured, which reproduces the
    /// square box this type replaced.
    pub fn aspect(&self) -> f32 {
        self.cap
            .filter(|cap| cap.height() > 0.0)
            .map_or(1.0, |cap| cap.width() / cap.height())
    }

    /// The on-screen node box for this cap at `height_px` - THE sizing rule:
    /// height is the site's constant, width follows the art.
    pub fn node_size(&self, height_px: f32) -> Vec2 {
        Vec2::new(height_px * self.aspect(), height_px)
    }

    /// Point an existing image node at this cap, sized for `height_px`. The one
    /// path every keycap site goes through, whether it spawns the node
    /// ([`KeyCap::node`]) or repaints it every frame.
    pub fn apply(&self, height_px: f32, image: &mut ImageNode, node: &mut Node) {
        if image.image != self.image {
            image.image = self.image.clone();
        }
        if image.rect != self.cap {
            image.rect = self.cap;
        }
        let size = self.node_size(height_px);
        node.width = Val::Px(size.x);
        node.height = Val::Px(size.y);
    }

    /// Whether `image`/`node` already draw this cap at `height_px` - the guard
    /// a per-frame repainter needs so it does not dirty a `Node` (and with it a
    /// UI relayout) on every quiet pass.
    pub fn is_applied(&self, image: &ImageNode, node: &Node, height_px: f32) -> bool {
        let size = self.node_size(height_px);
        image.image == self.image
            && image.rect == self.cap
            && node.width == Val::Px(size.x)
            && node.height == Val::Px(size.y)
    }

    /// The image node and `Node` for a cap drawn at `height_px`, for sites that
    /// spawn their keycap rather than repainting it.
    pub fn node(&self, height_px: f32) -> (ImageNode, Node) {
        let mut image = ImageNode::default();
        let mut node = Node::default();
        self.apply(height_px, &mut image, &mut node);
        (image, node)
    }
}

/// The opaque bounds of `image`, in texture pixels, or `None` when the image
/// carries no readable pixels (not loaded yet, or uploaded render-world-only)
/// or is fully transparent.
///
/// One alpha scan per glyph over a 128x128 canvas, run once at load: cheap
/// enough not to need caching beyond the [`KeyGlyphs`] map, and free of any
/// filesystem or platform assumption, so it works identically on wasm.
pub fn trimmed_cap(image: &Image) -> Option<Rect> {
    let (width, height) = (image.width(), image.height());
    let mut min = UVec2::new(width, height);
    let mut max = UVec2::ZERO;
    for y in 0..height {
        for x in 0..width {
            let opaque = image
                .get_color_at(x, y)
                .is_ok_and(|color| color.alpha() > 0.0);
            if opaque {
                min = min.min(UVec2::new(x, y));
                max = max.max(UVec2::new(x + 1, y + 1));
            }
        }
    }
    (min.x < max.x && min.y < max.y)
        .then(|| Rect::new(min.x as f32, min.y as f32, max.x as f32, max.y as f32))
}

/// The preloaded keycaps, keyed by display label - built by asset loading from
/// the `GameAssets::key_glyphs` mapped collection and published on
/// [`super::NovaHudAssets`]. Empty on bare-app rigs that never ran asset
/// loading, which is exactly the text-chip fallback path.
#[derive(Clone, Default, Debug)]
pub struct KeyGlyphs(HashMap<&'static str, KeyCap>);

impl KeyGlyphs {
    /// Build the label->keycap map from a stem-keyed collection (the caller
    /// resolves each stem in [`KEY_GLYPH_FILES`]); a stem the collection does
    /// not carry is skipped, so a partial load degrades to text chips. The caps
    /// come out UNMEASURED - [`KeyGlyphs::measure_caps`] fills them in once the
    /// pixels are there.
    pub fn from_stems(mut resolve: impl FnMut(&str) -> Option<Handle<Image>>) -> Self {
        Self(
            KEY_GLYPH_FILES
                .iter()
                .filter_map(|(label, stem)| {
                    resolve(stem).map(|image| (*label, KeyCap { image, cap: None }))
                })
                .collect(),
        )
    }

    /// Scan every held glyph's alpha channel for the cap it draws, so the HUD
    /// can size from the ART instead of from the canvas. Returns how many caps
    /// resolved; a glyph whose pixels are not readable keeps its square
    /// fallback rather than rendering a sliver.
    pub fn measure_caps(&mut self, images: &Assets<Image>) -> usize {
        // Several labels share one keycap (both Control keys, both Shifts), so
        // scan each distinct IMAGE once and fan the answer out.
        let mut scanned: Vec<(AssetId<Image>, Option<Rect>)> = Vec::new();
        for cap in self.0.values_mut() {
            let id = cap.image.id();
            let measured = match scanned.iter().find(|(known, _)| *known == id) {
                Some((_, rect)) => *rect,
                None => {
                    let rect = images.get(&cap.image).and_then(trimmed_cap);
                    scanned.push((id, rect));
                    rect
                }
            };
            cap.cap = measured;
        }
        self.0.values().filter(|cap| cap.cap.is_some()).count()
    }

    /// The keycap for `label`, or `None` (unmapped key, or assets not loaded).
    pub fn get(&self, label: &str) -> Option<KeyCap> {
        self.0.get(label).cloned()
    }

    /// Whether any glyph is loaded (bare-app rigs carry none).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// How many keycap LABELS are held - the denominator for "did every
    /// preloaded glyph resolve a cap". Labels, not images: several labels share
    /// one keycap, which [`KeyGlyphs::measure_caps`] scans once and fans out.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::player::flight_rig_reserved_sources;

    /// DoD 2, half one: every key the flight rig actually binds resolves to a
    /// keycap file that EXISTS on disk (this pins the upstream `Crtl` typo and
    /// the `Brackets_L/R` names), and so do the fixed gesture pseudo-labels and
    /// the HUD chrome keys. The other half - that every mapped path is in the
    /// preload collection - lives in `nova_assets` where the collection is.
    #[test]
    fn every_bound_key_maps_to_an_existing_glyph_asset() {
        // The rig's real keyboard bindings, labelled by the PRODUCTION labeller
        // the hints use - a local reimplementation would keep this green while
        // a change to the real labels broke the runtime lookup.
        let label = crate::input::player::keyboard_label;
        let bound: Vec<String> = flight_rig_reserved_sources()
            .into_iter()
            .filter_map(|(source, _)| match source {
                crate::input::player::InputSource::Keyboard(key) => Some(label(key)),
                _ => None,
            })
            .collect();
        assert!(!bound.is_empty(), "delivery guard: the rig binds keys");

        // Plus the labels the HUD uses that are not a single rig binding.
        let extra = ["CTRL", "SHIFT", "SCROLL", "Tab", "Backquote"];
        for key in bound.iter().map(String::as_str).chain(extra) {
            let stem = key_glyph_stem(key)
                .unwrap_or_else(|| panic!("no keycap glyph mapped for the bound key '{key}'"));
            let path = std::path::Path::new("../../assets")
                .join(KEY_GLYPH_DIR)
                .join(format!("{stem}.png"));
            assert!(
                path.exists(),
                "keycap glyph missing on disk: {}",
                path.display()
            );
        }
    }

    /// The path list the collection preloads is the DISTINCT set of mapped
    /// files - shared keycaps (both Control keys, both Shift keys) must not
    /// load twice.
    #[test]
    fn asset_paths_are_the_distinct_mapped_files() {
        let paths = key_glyph_asset_paths();
        let mut sorted = paths.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(paths, sorted, "the list is sorted and deduplicated");
        assert!(
            paths.contains(&format!("{KEY_GLYPH_DIR}/T_Crtl_Key_Alt.png")),
            "the (upstream-misspelled) Ctrl keycap is preloaded"
        );
        assert!(
            paths.len() < KEY_GLYPH_FILES.len(),
            "shared keycaps collapse to one path"
        );
    }

    /// An unmapped key resolves to nothing, which is the text-chip fallback -
    /// a rebind to an exotic key must degrade, never render an empty box.
    #[test]
    fn unmapped_keys_fall_back_instead_of_resolving() {
        assert_eq!(key_glyph_stem("F13"), None);
        assert_eq!(key_glyph_stem(""), None);
    }
}
