//! What a rock is MADE of, and what that makes it look like.
//!
//! Every asteroid in the game wears one texture
//! (`assets/base/textures/asteroid.png`) at one tiling, through one white
//! `StandardMaterial`, so a field of rocks is a field of the same rock. The
//! texture's own linear albedo spans about 0.04 to 0.17 - a narrow dark band -
//! and nothing varies the roughness, so the surface has neither colour nor
//! specular variety to read depth from.
//!
//! A KIND is the fix and the seam. One open id says what a body is made of;
//! [`AsteroidKindLook`] is the shading that id resolves to, and
//! `asteroid_surface.wgsl` spends it. The id is not new: it is
//! [`AsteroidConfig::material`](super::asteroid::AsteroidConfig::material),
//! which already exists, is already authored per rock, and already says "ice or
//! metal body" in its own docs. It reached the impact table and nothing else.
//! Now it reaches the surface too, and it is where an ore yield attaches next -
//! one id, one authored field, three consumers.
//!
//! Ids are OPEN STRINGS, not an enum, for the reason
//! [`SurfaceMaterial`](nova_gameplay::prelude::SurfaceMaterial) is: a mod fields
//! a new rock by naming one, and the table it names is data rather than a Rust
//! variant list.
//!
//! Open does NOT mean forgiving. An id this table does not know is an ERROR,
//! not a grey rock: the scenario lint refuses it before the file ships, and the
//! render path refuses it again and says which body and which id. A mod author
//! who typed `granit` has to hear about it, and a rock that silently became
//! stone would be the one way they never would. There is no default kind and
//! nothing resolves an absent one - [`AsteroidConfig::material`] is required,
//! so a rock that does not say what it is made of does not load at all.

use bevy::prelude::*;
use nova_gameplay::prelude::MATERIAL_ROCK;

/// The kind tag, the shading a kind resolves to, and the ids the base game
/// ships.
pub mod prelude {
    pub use super::{
        asteroid_kind_at, asteroid_kind_from_mix, asteroid_kind_look, is_asteroid_kind,
        AsteroidKind, AsteroidKindLook, ASTEROID_KINDS, ASTEROID_KIND_SUMMARIES, KIND_CARBON,
        KIND_ICE, KIND_METAL, KIND_PLAIN, KIND_ROCK,
    };
}

/// Ordinary stone: the default, and what an unknown id falls back to.
///
/// The same string as [`MATERIAL_ROCK`], deliberately - a rock that authored
/// nothing keeps the material id it has always had, and gains a look.
pub const KIND_ROCK: &str = MATERIAL_ROCK;

/// Nickel-iron: dark, cold-toned, metallic, with a bright seam network.
pub const KIND_METAL: &str = "metal";

/// Water ice: pale, blue-shifted, glossy in patches, crackled through.
pub const KIND_ICE: &str = "ice";

/// Carbonaceous: nearly black, matte, almost featureless.
pub const KIND_CARBON: &str = "carbon";

/// The CONTROL, not a rock: the texture alone, exactly as it was drawn before
/// kinds existed.
///
/// Every kind knob is off, so a `plain` rock is the before picture in the same
/// frame as the after. It is also the escape hatch for a mod that ships a
/// finished texture and wants nothing done to it.
pub const KIND_PLAIN: &str = "plain";

/// The resolved kind id this rock was built with, carried on the asteroid root.
///
/// A separate component from
/// [`SurfaceMaterial`](nova_gameplay::prelude::SurfaceMaterial) even though both
/// hold the same string: that one is the audio table's key and lives in
/// nova_gameplay, and a render observer has no business reading it. This is what
/// a future mining or ore system reads too.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct AsteroidKind(pub String);

impl AsteroidKind {
    /// The tag for a named kind.
    pub fn new(kind: impl Into<String>) -> Self {
        Self(kind.into())
    }
}

/// The kind a `draw` in `0.0..1.0` picks out of a WEIGHTED mix.
///
/// A mix is `[(id, weight)]` and the weights are relative counts, not
/// percentages: `[("rock", 24), ("carbon", 5), ("metal", 1)]` is "mostly rock,
/// some carbon, rare metal" and stays that when a fourth entry is added. That
/// is the whole reason weights beat a plain repeated list - an author says the
/// proportion once instead of writing `rock` twenty-four times - and it is the
/// only complexity a mix carries. There is no normalisation step and no
/// percentage that has to add to a hundred.
///
/// `None` when the mix is empty or every weight is zero, which is the caller's
/// signal to keep whatever the template already authored.
pub fn asteroid_kind_from_mix<S: AsRef<str>>(mix: &[(S, u32)], draw: f32) -> Option<&str> {
    let total: u32 = mix.iter().map(|(_, weight)| *weight).sum();
    if total == 0 {
        return None;
    }
    // Clamped rather than trusted: a draw of exactly 1.0 would land one past
    // the last bucket, and a caller hashing its own numbers is easy to get
    // slightly wrong.
    let mut ticket = (draw.clamp(0.0, 1.0) * total as f32) as u32;
    ticket = ticket.min(total - 1);
    let mut run = 0;
    for (id, weight) in mix {
        run += *weight;
        if ticket < run {
            return Some(id.as_ref());
        }
    }
    // Unreachable: `run` finishes at `total` and `ticket` is below it.
    None
}

/// The kind the body at `index` in an AUTHORED list gets from `mix`.
///
/// For fixed content - a hand-placed belt - where there is no scatter RNG to
/// draw from. The index is hashed rather than cycled: walking the buckets in
/// order would give a strictly repeating pattern, and an authored list is often
/// written in rough spatial order, so the pattern would land as stripes across
/// the map. The same index always gives the same kind, so the belt is the same
/// belt on every load and in every chapter that spawns it.
pub fn asteroid_kind_at<S: AsRef<str>>(mix: &[(S, u32)], index: usize) -> Option<&str> {
    let mut hash = 0x811c_9dc5u32 ^ (index as u32).wrapping_mul(0x9e37_79b1);
    for _ in 0..3 {
        hash ^= hash >> 15;
        hash = hash.wrapping_mul(0x2545_f491);
    }
    asteroid_kind_from_mix(mix, (hash >> 8) as f32 / 16_777_216.0)
}

/// How a kind is SHADED: the palette it is drawn from and the noise knobs the
/// surface shader spends.
///
/// Colours are LINEAR (the shader's own space), authored here through
/// [`Srgba`] so the numbers in this file read the way a colour picker shows
/// them. Every scalar is a plain ratio in 0..1 unless its own doc says
/// otherwise; none of them is a length, so none of them is in meters.
///
/// Rust-side for now. The RON surface this round is the kind ID
/// ([`AsteroidConfig::material`](super::asteroid::AsteroidConfig::material))
/// plus the silhouette seed - a kind is picked, not authored. Making the LOOK
/// authorable is one content kind away: give this struct `serde` and an `id`,
/// register it the way `ImpactSoundConfig` is registered, and resolve through
/// the merged registry instead of [`asteroid_kind_look`]. That is the whole
/// remaining step, and nothing else has to move.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AsteroidKindLook {
    /// The dark end of the palette: what the low ground of the macro noise is
    /// coloured.
    pub shade: LinearRgba,
    /// The light end of the palette: what the high ground is coloured.
    pub tint: LinearRgba,
    /// The colour of the cell walls the Worley layer draws - metal seams, ice
    /// crackle.
    pub vein: LinearRgba,
    /// Macro-noise cycles per unit of the body's own local space. A rock's
    /// surface sits 3.5 to 6 units out, so 0.5 is a handful of masses across
    /// one body and 1.5 is a mottle.
    pub macro_scale: f32,
    /// How far the macro noise's own domain is warped before it is read, in the
    /// same local units. Warping is what turns fBm's soapy blobs into
    /// stretched, tangled strata.
    pub warp: f32,
    /// How hard the macro noise is pushed away from its midpoint before it
    /// picks a palette colour. 1 is the raw noise; above 1 widens the range the
    /// palette covers and gives the surface visible regions.
    pub contrast: f32,
    /// How much of the kind palette replaces the texture's own colour. 0 is the
    /// texture untouched (see [`KIND_PLAIN`]), 1 is the palette wearing the
    /// texture only as grain.
    pub kind_mix: f32,
    /// How strongly the texture's brightness modulates the palette. The texture
    /// stops being the colour and becomes the crevices.
    pub grain: f32,
    /// The texture's own mean linear luminance, which [`Self::grain`] is
    /// measured against so a bright texture does not simply wash the palette
    /// out. See [`ROCK_TEXTURE_LINEAR_MID`].
    pub grain_mid: f32,
    /// How much of the second, incommensurate texture scale is blended in to
    /// break the tile repeat. 0 samples one scale (and repeats); 1 is a full
    /// noise-masked blend of two.
    pub break_up: f32,
    /// Worley cell cycles per unit of local space: how fine the seam or crackle
    /// network is.
    pub vein_scale: f32,
    /// How strongly the cell walls are painted in [`Self::vein`]. 0 turns the
    /// whole Worley layer off, and the shader then does not evaluate it.
    pub vein_strength: f32,
    /// Perceptual roughness where the surface is smoothest.
    pub roughness_low: f32,
    /// Perceptual roughness where the surface is roughest. The shader runs
    /// between the two off the same noise the palette uses, so colour and
    /// specular agree about where the rock is worn.
    pub roughness_high: f32,
    /// Metallic response. Nonzero only for [`KIND_METAL`]: a metallic
    /// dielectric is what makes rock read as plastic.
    pub metallic: f32,
}

/// The mean linear luminance of `assets/base/textures/asteroid.png`, measured
/// over all 736x736 texels: 0.095, with 5th and 95th percentiles at 0.038 and
/// 0.171.
///
/// That narrow dark band is half of why a rock reads flat, and it is why
/// [`AsteroidKindLook::grain`] is a RATIO against this mid rather than an
/// offset from 0.5 - the texture's brightness becomes relief on the palette
/// instead of dragging the palette down to its own level.
pub const ROCK_TEXTURE_LINEAR_MID: f32 = 0.095;

/// Every kind id the base game ships, in the order a pick list should offer
/// them: the ordinary one first, the control last.
///
/// This is what the scenario lint checks an authored `material` against and
/// what the editor's kind picker cycles through. A mod adding a kind adds it
/// here today; when the kind table becomes loaded data, this becomes the base
/// bundle's rows and the lint reads the catalog instead.
pub const ASTEROID_KINDS: [&str; 5] = [KIND_ROCK, KIND_METAL, KIND_ICE, KIND_CARBON, KIND_PLAIN];

/// The same ids with one line each on what they are, for a picker that has to
/// teach the vocabulary as it offers it.
///
/// A creator meeting an empty box cannot guess `carbon`. Kept beside
/// [`ASTEROID_KINDS`] and pinned to it by a test, so a kind cannot be added to
/// one and not the other.
pub const ASTEROID_KIND_SUMMARIES: [(&str, &str); 5] = [
    (KIND_ROCK, "Ordinary stone: warm tan banded with cool slate"),
    (KIND_METAL, "Nickel-iron: cold, metallic, bright seams"),
    (KIND_ICE, "Water ice: pale blue, glossy, crackled through"),
    (KIND_CARBON, "Carbonaceous: near-black and matte"),
    (
        KIND_PLAIN,
        "The control: the texture with nothing done to it",
    ),
];

/// Whether `kind` names a kind that exists. The lint's question.
pub fn is_asteroid_kind(kind: &str) -> bool {
    ASTEROID_KINDS.contains(&kind)
}

/// The shading a kind id resolves to, or `None` when no kind answers to that
/// id.
///
/// `None` is a REFUSAL, not a default: there is deliberately no house look for
/// an unrecognised id, because a rock that quietly became stone is how a typo
/// ships. Callers say which body and which id and then stop.
pub fn asteroid_kind_look(kind: &str) -> Option<AsteroidKindLook> {
    match kind {
        KIND_ROCK => Some(rock()),
        KIND_METAL => Some(metal()),
        KIND_ICE => Some(ice()),
        KIND_CARBON => Some(carbon()),
        KIND_PLAIN => Some(plain()),
        _ => None,
    }
}

/// Linear colour from the sRGB numbers a picker shows.
fn srgb(red: f32, green: f32, blue: f32) -> LinearRgba {
    Srgba::rgb(red, green, blue).into()
}

/// Ordinary stone. Deliberately the least changed of the kinds: it is what
/// almost every authored rock already is, so it has to read as the shipped rock
/// stopping being repetitive rather than as a new material.
fn rock() -> AsteroidKindLook {
    AsteroidKindLook {
        shade: srgb(0.28, 0.30, 0.34),
        tint: srgb(0.52, 0.45, 0.34),
        vein: srgb(0.60, 0.56, 0.50),
        macro_scale: 0.9,
        warp: 0.5,
        contrast: 2.2,
        kind_mix: 0.8,
        grain: 1.0,
        grain_mid: ROCK_TEXTURE_LINEAR_MID,
        break_up: 1.0,
        vein_scale: 1.1,
        vein_strength: 0.06,
        roughness_low: 0.70,
        roughness_high: 0.96,
        metallic: 0.0,
    }
}

/// Nickel-iron. The metallic response and the bright Worley seams do the work;
/// the palette is barely coloured at all, because metal takes its colour from
/// what it reflects.
fn metal() -> AsteroidKindLook {
    AsteroidKindLook {
        shade: srgb(0.21, 0.24, 0.30),
        tint: srgb(0.50, 0.48, 0.44),
        vein: srgb(0.76, 0.72, 0.64),
        macro_scale: 1.0,
        warp: 0.45,
        contrast: 2.0,
        kind_mix: 0.85,
        grain: 0.9,
        grain_mid: ROCK_TEXTURE_LINEAR_MID,
        break_up: 1.0,
        vein_scale: 1.0,
        // Low, and it took a capture to learn how low. At 0.45 the cell walls
        // drew a honeycomb over the whole body - a new repeating pattern in
        // place of the old one. Metal reads as metal from its METALLIC
        // response; the seams only have to be findable.
        vein_strength: 0.16,
        roughness_low: 0.28,
        roughness_high: 0.70,
        metallic: 0.85,
    }
}

/// Water ice. The only kind with a glossy end to its roughness range, which is
/// what makes a rotating ice body flash at the light rig instead of sitting
/// there.
fn ice() -> AsteroidKindLook {
    AsteroidKindLook {
        shade: srgb(0.42, 0.56, 0.72),
        tint: srgb(0.84, 0.88, 0.91),
        vein: srgb(0.93, 0.97, 1.0),
        macro_scale: 0.6,
        warp: 0.70,
        contrast: 1.8,
        kind_mix: 0.95,
        grain: 0.6,
        grain_mid: ROCK_TEXTURE_LINEAR_MID,
        break_up: 1.0,
        vein_scale: 1.8,
        vein_strength: 0.24,
        roughness_low: 0.08,
        roughness_high: 0.55,
        metallic: 0.0,
    }
}

/// Carbonaceous. Nearly black and nearly featureless by design: the point of it
/// is the silhouette a C-type cuts against a lit field, so the grain is turned
/// UP and the palette is turned down.
fn carbon() -> AsteroidKindLook {
    AsteroidKindLook {
        shade: srgb(0.055, 0.062, 0.075),
        tint: srgb(0.14, 0.13, 0.115),
        vein: srgb(0.19, 0.18, 0.16),
        macro_scale: 0.85,
        warp: 0.55,
        contrast: 2.0,
        kind_mix: 0.95,
        grain: 0.7,
        grain_mid: ROCK_TEXTURE_LINEAR_MID,
        break_up: 1.0,
        vein_scale: 0.9,
        vein_strength: 0.06,
        roughness_low: 0.88,
        roughness_high: 1.0,
        metallic: 0.0,
    }
}

/// The control: every knob off, so the shader's output is the texture times the
/// standard material's own base colour - byte for byte what a rock was drawn as
/// before kinds existed.
fn plain() -> AsteroidKindLook {
    AsteroidKindLook {
        shade: LinearRgba::WHITE,
        tint: LinearRgba::WHITE,
        vein: LinearRgba::WHITE,
        macro_scale: 0.0,
        warp: 0.0,
        contrast: 1.0,
        kind_mix: 0.0,
        grain: 0.0,
        grain_mid: ROCK_TEXTURE_LINEAR_MID,
        break_up: 0.0,
        vein_scale: 1.0,
        vein_strength: 0.0,
        // StandardMaterial's own defaults, so `plain` is not a look but the
        // absence of one.
        roughness_low: 0.5,
        roughness_high: 0.5,
        metallic: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An id nobody ships is an ERROR, not a grey rock. This is the line the
    /// whole no-fallback rule stands on: a typo in a mod has to be findable,
    /// and a kind that quietly resolved to stone is the one way it would not
    /// be.
    #[test]
    fn an_unknown_kind_is_not_a_kind() {
        assert_eq!(asteroid_kind_look("obsidian-from-a-mod"), None);
        assert!(!is_asteroid_kind("obsidian-from-a-mod"));
        assert!(!is_asteroid_kind(""));
    }

    /// Every id the pick list offers resolves, and every id that resolves is on
    /// the pick list. A kind reachable from one and not the other is either
    /// unpickable or unrenderable.
    #[test]
    fn the_shipped_ids_and_the_shipped_looks_are_the_same_set() {
        for kind in ASTEROID_KINDS {
            assert!(
                asteroid_kind_look(kind).is_some(),
                "'{kind}' is offered and does not resolve"
            );
            assert!(is_asteroid_kind(kind));
        }
        assert_eq!(ASTEROID_KINDS.len(), 5);
    }

    /// The picker's list is the shipped list, in the same order, with a line
    /// of its own for every id. A summary table that drifted would offer a
    /// creator a kind the lint refuses.
    #[test]
    fn every_kind_is_summarised_once_and_in_order() {
        let ids: Vec<&str> = ASTEROID_KIND_SUMMARIES.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, ASTEROID_KINDS.to_vec());
        for (id, summary) in ASTEROID_KIND_SUMMARIES {
            assert!(!summary.is_empty(), "'{id}' offers no explanation");
        }
    }

    /// Stone's id is the impact table's rock id, so one authored string keeps
    /// answering the sound question and the look question.
    #[test]
    fn the_rock_kind_id_is_the_rock_material_id() {
        assert_eq!(KIND_ROCK, MATERIAL_ROCK);
    }

    /// The control has to BE the control: any knob left on would put the before
    /// picture and the after picture in the same place.
    #[test]
    fn the_plain_kind_turns_every_knob_off() {
        let control = asteroid_kind_look(KIND_PLAIN).expect("the control is a shipped kind");

        assert_eq!(control.kind_mix, 0.0);
        assert_eq!(control.macro_scale, 0.0);
        assert_eq!(control.warp, 0.0);
        assert_eq!(control.grain, 0.0);
        assert_eq!(control.break_up, 0.0);
        assert_eq!(control.vein_strength, 0.0);
        assert_eq!(control.metallic, 0.0);
        assert_eq!(control.roughness_low, control.roughness_high);
    }

    /// Every shipped kind has to be a kind: a palette with two ends, a macro
    /// noise big enough to see, and a roughness range that runs the right way.
    /// A knob typed backwards is invisible in code review and obvious in a
    /// capture, which is the wrong order to find it in.
    #[test]
    fn every_shipped_kind_is_shaded_within_range() {
        for kind in [KIND_ROCK, KIND_METAL, KIND_ICE, KIND_CARBON] {
            let look = asteroid_kind_look(kind).expect("a shipped kind resolves");

            assert!(
                look.roughness_low < look.roughness_high,
                "{kind}: roughness runs {} to {}",
                look.roughness_low,
                look.roughness_high
            );
            assert!(
                (0.0..=1.0).contains(&look.roughness_low)
                    && (0.0..=1.0).contains(&look.roughness_high)
                    && (0.0..=1.0).contains(&look.metallic)
                    && (0.0..=1.0).contains(&look.kind_mix)
                    && (0.0..=1.0).contains(&look.grain)
                    && (0.0..=1.0).contains(&look.break_up)
                    && (0.0..=1.0).contains(&look.vein_strength),
                "{kind}: a ratio knob is outside 0..1"
            );
            assert!(look.macro_scale > 0.0, "{kind}: no macro noise");
            assert!(look.grain_mid > 0.0, "{kind}: grain would divide by zero");
            assert!(
                look.tint.red + look.tint.green + look.tint.blue
                    > look.shade.red + look.shade.green + look.shade.blue,
                "{kind}: the palette's light end is darker than its dark end"
            );
        }
    }

    /// Kinds have to be TOLD APART at a glance, which is the only reason they
    /// exist. Metal is the metallic one, ice is the glossy one, carbon is the
    /// dark one.
    #[test]
    fn the_kinds_read_differently_from_each_other() {
        let look = |kind| asteroid_kind_look(kind).expect("a shipped kind resolves");
        let stone = look(KIND_ROCK);
        let metal = look(KIND_METAL);
        let ice = look(KIND_ICE);
        let carbon = look(KIND_CARBON);

        assert!(metal.metallic > 0.5 && stone.metallic == 0.0);
        assert!(ice.roughness_low < stone.roughness_low);
        assert!(ice.tint.blue > ice.tint.red);
        assert!(carbon.tint.red < stone.tint.red);
        assert!(carbon.shade.red < stone.shade.red);
    }

    /// A weight is a share of the draw, and the shares are laid out in the
    /// order they are authored. A mix of 6:3:1 hands the first six tenths of
    /// the range to the first id.
    #[test]
    fn a_weight_is_a_share_of_the_draw() {
        let mix = [(KIND_ROCK, 6), (KIND_ICE, 3), (KIND_METAL, 1)];
        assert_eq!(asteroid_kind_from_mix(&mix, 0.0), Some(KIND_ROCK));
        assert_eq!(asteroid_kind_from_mix(&mix, 0.59), Some(KIND_ROCK));
        assert_eq!(asteroid_kind_from_mix(&mix, 0.61), Some(KIND_ICE));
        assert_eq!(asteroid_kind_from_mix(&mix, 0.89), Some(KIND_ICE));
        assert_eq!(asteroid_kind_from_mix(&mix, 0.91), Some(KIND_METAL));
    }

    /// The ends of the range are the ends of the mix, including a draw of
    /// exactly 1.0 - which would otherwise index one bucket past the last.
    #[test]
    fn the_ends_of_the_draw_stay_inside_the_mix() {
        let mix = [(KIND_ROCK, 6), (KIND_METAL, 1)];
        assert_eq!(asteroid_kind_from_mix(&mix, 1.0), Some(KIND_METAL));
        assert_eq!(asteroid_kind_from_mix(&mix, 2.0), Some(KIND_METAL));
        assert_eq!(asteroid_kind_from_mix(&mix, -1.0), Some(KIND_ROCK));
    }

    /// A mix that says nothing picks nothing, so the caller keeps whatever the
    /// template authored rather than being handed a default it did not ask for.
    #[test]
    fn a_mix_with_no_weight_picks_nothing() {
        let empty: [(&str, u32); 0] = [];
        assert_eq!(asteroid_kind_from_mix(&empty, 0.5), None);
        assert_eq!(asteroid_kind_from_mix(&[(KIND_ROCK, 0)], 0.5), None);
        assert_eq!(asteroid_kind_at(&empty, 3), None);
    }

    /// An authored list gets the mix without getting a pattern: the same index
    /// always resolves the same way, consecutive indices do not march through
    /// the buckets in order, and a long enough run still finds the rare kind.
    #[test]
    fn an_authored_index_is_hashed_not_cycled() {
        let mix = [(KIND_ROCK, 6), (KIND_CARBON, 3), (KIND_METAL, 1)];
        assert_eq!(asteroid_kind_at(&mix, 17), asteroid_kind_at(&mix, 17));

        let walk: Vec<&str> = (0..10)
            .map(|index| asteroid_kind_at(&mix, index).expect("the mix has weight"))
            .collect();
        assert_ne!(
            walk,
            vec![
                KIND_ROCK,
                KIND_ROCK,
                KIND_ROCK,
                KIND_ROCK,
                KIND_ROCK,
                KIND_ROCK,
                KIND_CARBON,
                KIND_CARBON,
                KIND_CARBON,
                KIND_METAL,
            ],
            "walking the buckets in index order is the pattern the hash exists to break"
        );

        let long: Vec<&str> = (0..60)
            .map(|index| asteroid_kind_at(&mix, index).expect("the mix has weight"))
            .collect();
        for wanted in [KIND_ROCK, KIND_CARBON, KIND_METAL] {
            assert!(
                long.contains(&wanted),
                "60 bodies of a 6:3:1 mix show every kind in it: {long:?}"
            );
        }
    }
}
