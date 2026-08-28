//! Key-glyph lookup: which keycap picture stands for which binding.
//!
//! The game shows keys as PICTURES, not `[BRACKETED]` text - the HUD icon dock,
//! the anchored verb cues, the objective stack's NOVA OS affordance and the
//! settings Controls rows all draw a keycap from
//! `assets/input-prompts/keyboard/Alt/` (the dark keycaps with white glyphs).
//!
//! This module owns the mapping, and with it the explicit asset path list that
//! `nova_assets`'s `GameAssets::key_glyphs` collection preloads: the glyphs
//! load and load-gate like every other static asset, the same as the UI SFX,
//! never lazily per chip. A FOLDER
//! collection cannot be used - folder collections do not work on wasm - so the
//! path list is explicit and `key_glyph_collection_matches_mapping_table`
//! (in `nova_assets`) pins the two together.
//!
//! The table covers the WHOLE pack, not just what the flight rig ships bound.
//! A settings rebind can put any key on any row, and a lazily loaded glyph
//! would arrive after the row that wanted it was drawn - and would have to be
//! alpha-scanned on some later frame to be sized. Preloading the 101 tiny
//! keycaps and pad glyphs (226K on disk) buys a rebind to any key its picture,
//! on the first frame, on wasm too.
//!
//! Keys are addressed by their DISPLAY LABEL - the string
//! [`nova_input::prelude::InputSource::label`] produces (`"X"`, `"Space"`,
//! `"ControlLeft"`, `"LMB"`), plus the fixed pseudo-labels the flight rig uses
//! for gestures that have no single key (`"CTRL"`, `"SHIFT"`, `"SCROLL"`) and
//! the axis notes a binding readout adds (`"Mouse"`, `"Scroll Up"`). That is
//! exactly what `FlightVerbHints` and `BindingChip` carry, so a consumer looks
//! a glyph up with the string it already has.
//!
//! Unmapped labels resolve to `None`, and every consumer falls back to a TEXT
//! chip - a key with no art degrades to the old look instead of rendering an
//! empty box.

use bevy::{platform::collections::HashMap, prelude::*};

/// The `KeyGlyphs` lookup, `KeyCap`, the glyph asset-path helpers and `KEY_GLYPH_DIR`.
pub mod prelude {
    pub use super::{
        key_glyph_asset_paths, key_glyph_stem, trimmed_cap, KeyCap, KeyGlyphs, KEY_GLYPH_DIR,
        KEY_GLYPH_FILES, PAD_GLYPH_DIR, PAD_GLYPH_FILES,
    };
}

/// Where the keycap art lives, relative to `assets/`.
pub const KEY_GLYPH_DIR: &str = "input-prompts/keyboard/Alt";

/// The mapping: display label -> keycap file stem under [`KEY_GLYPH_DIR`].
///
/// Several upstream filenames are misspelled or abbreviated and are pinned
/// here so a rename is caught by
/// `every_bound_key_maps_to_an_existing_glyph_asset`: `T_Crtl_Key_Alt` (the
/// upstream typo for Ctrl), `T_Brackets_L/R_Key_Alt`, and `T_3_Key_Alt-1`,
/// which draws a `4`.
///
/// The pack's spare alternates (`T_Enter_Tall`, `T_Shift_Super_Wide`,
/// `T_Backspace_Alt`, the keyboard/mouse sprite sheet) are absent: nothing
/// names them, and preloading art no label resolves is dead weight.
pub const KEY_GLYPH_FILES: &[(&str, &str)] = &[
    // Letters.
    ("A", "T_A_Key_Alt"),
    ("B", "T_B_Key_Alt"),
    ("C", "T_C_Key_Alt"),
    ("D", "T_D_Key_Alt"),
    ("E", "T_E_Key_Alt"),
    ("F", "T_F_Key_Alt"),
    ("G", "T_G_Key_Alt"),
    ("H", "T_H_Key_Alt"),
    ("I", "T_I_Key_Alt"),
    ("J", "T_J_Key_Alt"),
    ("K", "T_K_Key_Alt"),
    ("L", "T_L_Key_Alt"),
    ("M", "T_M_Key_Alt"),
    ("N", "T_N_Key_Alt"),
    ("O", "T_O_Key_Alt"),
    ("P", "T_P_Key_Alt"),
    ("Q", "T_Q_Key_Alt"),
    ("R", "T_R_Key_Alt"),
    ("S", "T_S_Key_Alt"),
    ("T", "T_T_Key_Alt"),
    ("U", "T_U_Key_Alt"),
    ("V", "T_V_Key_Alt"),
    ("W", "T_W_Key_Alt"),
    ("X", "T_X_Key_Alt"),
    ("Y", "T_Y_Key_Alt"),
    ("Z", "T_Z_Key_Alt"),
    // Digits. The `4` cap ships misnamed as a second `3`.
    ("0", "T_0_Key_Alt"),
    ("1", "T_1_Key_Alt"),
    ("2", "T_2_Key_Alt"),
    ("3", "T_3_Key_Alt"),
    ("4", "T_3_Key_Alt-1"),
    ("5", "T_5_Key_Alt"),
    ("6", "T_6_Key_Alt"),
    ("7", "T_7_Key_Alt"),
    ("8", "T_8_Key_Alt"),
    ("9", "T_9_Key_Alt"),
    // Function row.
    ("F1", "T_F1_Key_Alt"),
    ("F2", "T_F2_Key_Alt"),
    ("F3", "T_F3_Key_Alt"),
    ("F4", "T_F4_Key_Alt"),
    ("F5", "T_F5_Key_Alt"),
    ("F6", "T_F6_Key_Alt"),
    ("F7", "T_F7_Key_Alt"),
    ("F8", "T_F8_Key_Alt"),
    ("F9", "T_F9_Key_Alt"),
    ("F10", "T_F10_Key_Alt"),
    ("F11", "T_F11_Key_Alt"),
    ("F12", "T_F12_Key_Alt"),
    // Arrows.
    ("ArrowUp", "T_Up_Key_Alt"),
    ("ArrowDown", "T_Down_Key_Alt"),
    ("ArrowLeft", "T_Left_Key_Alt"),
    ("ArrowRight", "T_Right_Key_Alt"),
    // The wide caps and the editing keys.
    ("Space", "T_Space_Key_Alt"),
    ("Tab", "T_Tab_Key_Alt"),
    ("TAB", "T_Tab_Key_Alt"),
    ("Enter", "T_Enter_Key_Alt"),
    ("Escape", "T_Esc_Key_Alt"),
    ("Esc", "T_Esc_Key_Alt"),
    ("Backspace", "T_BackSpace_Key_Alt"),
    ("CapsLock", "T_CapsLock_Key_Alt"),
    ("NumLock", "T_NumLock_Key_Alt"),
    ("PrintScreen", "T_PrtScrn_Key_Alt"),
    // Navigation.
    ("Insert", "T_Ins_Key_Alt"),
    ("Delete", "T_Del_Key_Alt"),
    ("Home", "T_Home_Key_Alt"),
    ("End", "T_End_Key_Alt"),
    ("PageUp", "T_PageUp_Key_Alt"),
    ("PageDown", "T_PageDown_Key_Alt"),
    // Modifiers: both physical sides share one keycap, and the flight rig's
    // fixed pseudo-labels ("CTRL"/"SHIFT") land on the same art.
    ("CTRL", "T_Crtl_Key_Alt"),
    ("ControlLeft", "T_Crtl_Key_Alt"),
    ("ControlRight", "T_Crtl_Key_Alt"),
    ("SHIFT", "T_Shift_Key_Alt"),
    ("ShiftLeft", "T_Shift_Key_Alt"),
    ("ShiftRight", "T_Shift_Key_Alt"),
    ("AltLeft", "T_Alt_Key_Alt"),
    ("AltRight", "T_Alt_Key_Alt"),
    // Punctuation the game can reach. `Equal` is deliberately absent: the pack
    // draws a `+`, and a row bound to `=` would read as the wrong key.
    ("BracketLeft", "T_Brackets_L_Key_Alt"),
    ("BracketRight", "T_Brackets_R_Key_Alt"),
    ("Backquote", "T_Tilde_Key_Alt"),
    ("Minus", "T_Minus_Key_Alt"),
    ("Slash", "T_Slash_Key_Alt"),
    ("Semicolon", "T_Semicolon_Key_Alt"),
    ("Quote", "T_Quotation_Key_Alt"),
    // The mouse: the three buttons under `InputSource::label`'s short names,
    // then the axis notes a readout adds for motion and the wheel.
    ("LMB", "T_Mouse_Left_Key_Alt"),
    ("RMB", "T_Mouse_Right_Key_Alt"),
    ("MMB", "T_Mouse_Middle_Key_Alt"),
    ("Mouse", "T_Mouse_Simple_Key_Alt"),
    ("SCROLL", "T_Mouse_Scroll_Key_Dark_Key_Alt"),
    ("Scroll Up", "T_Mouse_Scroll_Up_Key_Dark_Key_Alt"),
    ("Scroll Down", "T_Mouse_Scroll_Down_Key_Dark_Key_Alt"),
];

/// How much taller than the site's constant a portrait cap may be drawn.
///
/// [`KeyCap::node_size`] sizes a tall-narrow cap by its width so it stays
/// legible; without a bound, one very tall glyph would set the height of every
/// row it appeared in.
const MAX_PORTRAIT_GROWTH: f32 = 1.6;

/// Narrower than this and a glyph is drawn as PORTRAIT art rather than a
/// keycap.
///
/// The pack's letter caps measure 0.92 - a hair taller than wide, and every
/// one of them the same - while the mouse glyphs measure 0.59. Sizing by the
/// short axis with no threshold caught the letters too and grew every chip in
/// the dock by two pixels, so the rule starts BELOW the keycaps.
const PORTRAIT_ASPECT: f32 = 0.8;

/// Where the gamepad prompt art lives, relative to `assets/`. The SAME pack as
/// the keycaps ([`KEY_GLYPH_DIR`]) and the same `Alt` style, so a pad glyph and
/// a keycap sitting in one row are drawn by one hand.
pub const PAD_GLYPH_DIR: &str = "input-prompts/gamepad/Alt";

/// The mapping for the PAD: glyph label -> file stem under [`PAD_GLYPH_DIR`].
///
/// A separate table from [`KEY_GLYPH_FILES`] because it is a separate
/// directory, and because the two vocabularies overlap: a pad's face buttons
/// read `A`/`B`/`X`/`Y` and so do four keyboard keys. The pad keys are `Pad
/// `-prefixed, which is exactly what
/// [`nova_input::prelude::InputSource::glyph_label`] produces.
///
/// Bevy's names are not the shell's. `LeftTrigger` is the BUMPER and
/// `LeftTrigger2` is the trigger, so the art is paired to the button, not to
/// the name. `Mode` (the guide button) has no entry: the pack draws no guide
/// glyph and nothing in the game binds that button.
///
/// The pack's own Xbox filenames are kept verbatim, as the keycaps are. Two
/// of them are not what they look like: `T_X_X_Alt` is the MENU button (three
/// bars), and `T_X_Share_Alt` is the VIEW button - the face-button X is
/// `T_X_X_White_Alt`.
pub const PAD_GLYPH_FILES: &[(&str, &str)] = &[
    // Face buttons, under the names `gamepad_label` prints. The white variants,
    // not the coloured ones, so a pad glyph reads like the keycaps beside it.
    ("Pad A", "T_X_A_White_Alt"),
    ("Pad B", "T_X_B_White_Alt"),
    ("Pad X", "T_X_X_White_Alt"),
    ("Pad Y", "T_X_Y_White_Alt"),
    // Bumpers and triggers. Bevy calls the bumper `LeftTrigger` and the
    // trigger `LeftTrigger2`, which is why the art does not read that way.
    ("Pad Left Trigger", "T_X_LB_Alt"),
    ("Pad Right Trigger", "T_X_RB_Alt"),
    ("Pad Left Trigger 2", "T_X_LT_Alt"),
    ("Pad Right Trigger 2", "T_X_RT_Alt"),
    // Stick presses, then the two chords a pad has instead of Escape.
    ("Pad Left Thumb", "T_X_Left_Stick_Click_Alt"),
    ("Pad Right Thumb", "T_X_Right_Stick_Click_Alt"),
    ("Pad Select", "T_X_Share_Alt"),
    ("Pad Start", "T_X_X_Alt"),
    // The D-pad.
    ("Pad D-Pad Up", "T_X_Dpad_Up_Alt"),
    ("Pad D-Pad Down", "T_X_Dpad_Down_Alt"),
    ("Pad D-Pad Left", "T_X_Dpad_Left_Alt"),
    ("Pad D-Pad Right", "T_X_Dpad_Right_Alt"),
    // The stick AXIS notes a readout adds - not buttons, so not `Pad`-keyed.
    ("Left Stick", "T_X_L_2D_Alt"),
    ("Right Stick", "T_X_R_2D_Alt"),
];

/// Every mapped label, with the directory and file stem its picture lives at.
/// The two packs read as one table to a consumer; only the paths differ.
fn glyph_files() -> impl Iterator<Item = (&'static str, &'static str, &'static str)> {
    KEY_GLYPH_FILES
        .iter()
        .map(|(label, stem)| (*label, KEY_GLYPH_DIR, *stem))
        .chain(
            PAD_GLYPH_FILES
                .iter()
                .map(|(label, stem)| (*label, PAD_GLYPH_DIR, *stem)),
        )
}

/// The keycap file stem for `label`, or `None` when the button has no art (the
/// caller then falls back to a text chip). Covers both packs.
pub fn key_glyph_stem(label: &str) -> Option<&'static str> {
    glyph_files()
        .find(|(key, ..)| *key == label)
        .map(|(_, _, stem)| stem)
}

/// The DISTINCT asset paths the mapping references, sorted - the list
/// `GameAssets::key_glyphs` must preload. Several labels share one picture, so
/// this is shorter than the two tables together.
pub fn key_glyph_asset_paths() -> Vec<String> {
    let mut paths: Vec<String> = glyph_files()
        .map(|(_, dir, stem)| format!("{dir}/{stem}.png"))
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
/// their legends unreadable.
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

    /// The on-screen node box for this cap at `height_px`: the HEIGHT is the
    /// site's constant, and the width follows the art - except for genuinely
    /// portrait art ([`PORTRAIT_ASPECT`]), which is sized by its width instead
    /// so it stays legible.
    ///
    /// Height alone starves a portrait glyph: against a 20px `W` the mouse
    /// caps came out about 12px wide, and the scroll-up and scroll-down chips
    /// could not be told apart on the Controls rows. A grown cap is bounded in
    /// turn by [`MAX_PORTRAIT_GROWTH`], so one odd piece of art cannot stretch
    /// the row it sits in.
    pub fn node_size(&self, height_px: f32) -> Vec2 {
        let aspect = self.aspect();
        if aspect >= PORTRAIT_ASPECT {
            return Vec2::new(height_px * aspect, height_px);
        }
        let height = (height_px / aspect).min(height_px * MAX_PORTRAIT_GROWTH);
        Vec2::new(height * aspect, height)
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
            glyph_files()
                .filter_map(|(label, _, stem)| {
                    resolve(stem).map(|image| (label, KeyCap { image, cap: None }))
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
    use nova_input::prelude::InputSource;
    use nova_ship::prelude::flight_rig_reserved_sources;

    use super::*;

    /// DoD 2, half one: every source the flight rig and the HUD actually bind
    /// resolves to a glyph file that EXISTS on disk (this pins the upstream
    /// `Crtl` typo and the `Brackets_L/R` names), and so do the fixed gesture
    /// pseudo-labels. The other half - that every mapped path is in the preload
    /// collection - lives in `nova_assets` where the collection is.
    ///
    /// EVERY source, not the keyboard ones: a pad default with no art draws
    /// text in a picture column, and the pad half of the table is where new
    /// bindings are landing. The other three owners' lists are unreachable
    /// from here (they depend on this crate), so `nova_menu` - which sees all
    /// of them, as it does for the conflict check - walks the whole table.
    #[test]
    fn every_bound_source_maps_to_an_existing_glyph_asset() {
        // Labelled by the PRODUCTION labeller the chips use - a local
        // reimplementation would keep this green while a change to the real
        // labels broke the runtime lookup.
        let bound: Vec<String> = flight_rig_reserved_sources()
            .into_iter()
            .map(|(source, _)| source.glyph_label())
            .collect();
        assert!(!bound.is_empty(), "delivery guard: the rig binds sources");

        // The HUD's own chrome keys, taken from the table rather than named
        // here, so a rebound chrome action is caught by this test too.
        let chrome: Vec<String> = crate::hud_bindings()
            .iter()
            .flat_map(|action| action.sources())
            .map(|source| source.glyph_label())
            .collect();
        // Plus the labels that are NOT a registry binding: the three gesture
        // pseudo-labels, which stand for a modifier or a wheel rather than a
        // key, and the NOVA OS affordance's fallback (the toggle lives in
        // `nova_os_ui`, which depends on this crate and so cannot be named
        // from here - `novaos_key_label` falls back to Tab without it).
        let extra = ["CTRL", "SHIFT", "SCROLL", "Tab"];
        for key in bound
            .iter()
            .chain(chrome.iter())
            .map(String::as_str)
            .chain(extra)
        {
            let stem = key_glyph_stem(key)
                .unwrap_or_else(|| panic!("no glyph mapped for the bound source '{key}'"));
            let path = key_glyph_asset_paths()
                .into_iter()
                .find(|path| path.ends_with(&format!("/{stem}.png")))
                .unwrap_or_else(|| panic!("'{key}' maps to {stem}, which is not preloaded"));
            assert!(
                std::path::Path::new("../../assets").join(&path).exists(),
                "glyph missing on disk: {path}"
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
            paths.len() < KEY_GLYPH_FILES.len() + PAD_GLYPH_FILES.len(),
            "shared keycaps collapse to one path"
        );
        assert!(
            paths.iter().any(|path| path.starts_with(PAD_GLYPH_DIR)),
            "the pad pack preloads beside the keyboard one"
        );
    }

    /// The table grew to the whole pack for the settings rebind rows, so the
    /// disk check cannot ride on what the flight rig happens to bind any
    /// more: EVERY mapped stem must be a file.
    #[test]
    fn every_mapped_stem_is_a_file_on_disk() {
        for path in key_glyph_asset_paths() {
            let full = std::path::Path::new("../../assets").join(&path);
            assert!(full.exists(), "glyph missing on disk: {path}");
        }
    }

    /// The pad has pictures too, and its face buttons must not be answered
    /// with the KEYBOARD's `A`. The `Pad ` prefix is what keeps them apart, so
    /// it is asserted through the production labeller.
    #[test]
    fn a_pad_button_resolves_its_own_art_and_not_the_keyboards() {
        let pad_a = InputSource::Gamepad(GamepadButton::South);
        assert_eq!(
            key_glyph_stem(&pad_a.glyph_label()),
            Some("T_X_A_White_Alt"),
            "the pad face button draws the pad glyph"
        );
        assert_eq!(
            key_glyph_stem(&InputSource::Keyboard(KeyCode::KeyA).glyph_label()),
            Some("T_A_Key_Alt"),
            "and the keyboard key still draws its keycap"
        );
        assert_eq!(
            key_glyph_stem(&InputSource::Gamepad(GamepadButton::Mode).glyph_label()),
            None,
            "the pack draws no guide glyph; it falls back to text"
        );
    }

    /// The labels a settings row asks for come from `InputSource::label`, not
    /// from a spelling this crate invents - a friendlier one silently loses
    /// the picture.
    #[test]
    fn a_rebindable_key_resolves_by_its_source_label() {
        for source in [
            InputSource::Keyboard(KeyCode::KeyQ),
            InputSource::Keyboard(KeyCode::Digit4),
            InputSource::Keyboard(KeyCode::F7),
            InputSource::Keyboard(KeyCode::ArrowLeft),
            InputSource::Keyboard(KeyCode::Delete),
            InputSource::Mouse(MouseButton::Middle),
        ] {
            let label = source.label();
            assert!(
                key_glyph_stem(&label).is_some(),
                "no keycap glyph for '{label}'"
            );
        }
    }

    /// A letter keycap keeps the site's height; only genuinely portrait art is
    /// sized by its width instead.
    ///
    /// The regression this pins: sizing every cap by its SHORT axis, which is
    /// what a mouse glyph needs, caught the 26 letter caps too - they measure
    /// 0.92, a hair taller than wide - and grew every chip in the dock by two
    /// pixels. `keybind_dock`'s own sizing tests caught it; the rule lives
    /// here, so its guard belongs here.
    #[test]
    fn only_portrait_art_is_sized_by_its_width() {
        let cap = |width: f32, height: f32| KeyCap {
            image: Handle::default(),
            cap: Some(Rect::new(0.0, 0.0, width, height)),
        };

        assert_eq!(
            cap(120.0, 130.0).node_size(20.0),
            Vec2::new(120.0 / 130.0 * 20.0, 20.0),
            "a letter cap (0.92) is a keycap: the height is the site's constant"
        );
        assert_eq!(
            cap(200.0, 130.0).node_size(20.0),
            Vec2::new(200.0 / 130.0 * 20.0, 20.0),
            "and so is a wide one"
        );

        let mouse = cap(76.0, 128.0).node_size(20.0);
        assert!(
            mouse.x > 18.0 && mouse.y > 20.0,
            "a mouse glyph (0.59) is sized by its WIDTH instead, got {mouse:?}"
        );
        assert!(
            mouse.y <= 20.0 * MAX_PORTRAIT_GROWTH,
            "bounded, so one tall glyph cannot set the height of its row"
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
