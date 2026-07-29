//! Key-glyph lookup for the HUD: which keycap picture stands for which binding.
//!
//! The HUD shows keys as PICTURES, not `[BRACKETED]` text - the icon dock, the
//! anchored verb cues and the objective hint's NOVA OS affordance all draw a
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

use bevy::platform::collections::HashMap;

/// Glob-import surface: `use nova_gameplay::hud::key_glyphs::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::{key_glyph_asset_paths, key_glyph_stem, KeyGlyphs, KEY_GLYPH_DIR};
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

/// The preloaded keycap handles, keyed by display label - built by asset
/// loading from the `GameAssets::key_glyphs` mapped collection and published on
/// [`super::NovaHudAssets`]. Empty on bare-app rigs that never ran asset
/// loading, which is exactly the text-chip fallback path.
#[derive(Clone, Default, Debug)]
pub struct KeyGlyphs(HashMap<&'static str, bevy::prelude::Handle<bevy::image::Image>>);

impl KeyGlyphs {
    /// Build the label->handle map from a stem-keyed collection (the caller
    /// resolves each stem in [`KEY_GLYPH_FILES`]); a stem the collection does
    /// not carry is skipped, so a partial load degrades to text chips.
    pub fn from_stems(
        mut resolve: impl FnMut(&str) -> Option<bevy::prelude::Handle<bevy::image::Image>>,
    ) -> Self {
        Self(
            KEY_GLYPH_FILES
                .iter()
                .filter_map(|(label, stem)| resolve(stem).map(|handle| (*label, handle)))
                .collect(),
        )
    }

    /// The keycap for `label`, or `None` (unmapped key, or assets not loaded).
    pub fn get(&self, label: &str) -> Option<bevy::prelude::Handle<bevy::image::Image>> {
        self.0.get(label).cloned()
    }

    /// Whether any glyph is loaded (bare-app rigs carry none).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
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
