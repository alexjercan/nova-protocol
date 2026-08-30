//! The soft round mask every additive billboard in the game is drawn through.
//!
//! A hanabi particle is a quad. Untextured, a quad with colour in it is a
//! SQUARE, and at gameplay distance that is invisible only because the quad is
//! a few pixels across. Close to the camera - which is where every effect worth
//! looking at happens - a fireball reads as a glowing box, and no amount of
//! tuning the colour gradient fixes a hard corner.
//!
//! So the mask lives here, once, and every effect that draws a glow samples it
//! through `ParticleTextureModifier` with `ModulateOpacityFromR`. That mapping
//! is why the mask is a mask and not a sprite: it multiplies the particle's
//! ALPHA and leaves its colour alone, so an effect keeps the HDR gradient it
//! authored and only gains a round, feathered edge. One texture can therefore
//! serve a white-hot detonation core and a dim muzzle flash without either
//! having to agree on a colour.
//!
//! # Why a lazy resource
//!
//! Built on the first effect that asks, not at startup, on the same terms as
//! the effect assets it is bound to: an app running with particles off, and a
//! headless test app with no `Assets<Image>` at all, must not pay for - or
//! panic on - a texture nothing will sample.

use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_hanabi::{ExprWriter, ImageSampleMapping, Module, ParticleTextureModifier};

/// `SoftDot`, `soft_dot_image` and the two calls that bind the mask into an
/// effect graph.
pub mod prelude {
    pub use super::{declare_soft_dot_slot, soft_dot_image, soft_dot_modifier, SoftDot};
}

/// Name of the mask's texture slot, for anything reading a built graph back.
pub const SOFT_DOT_SLOT: &str = "soft_dot";

/// Width and height of the mask in texels.
///
/// Generous for a blurred circle, because the largest consumer is a detonation
/// core several world units across seen from a few units away: at that size a
/// smaller mask shows its own texels as rings in the falloff, which is a worse
/// artefact than the square it was added to fix.
const SOFT_DOT_TEXELS: u32 = 128;

/// Opacity of the mask at normalised radius `d` (0 at the centre, 1 at the
/// inscribed edge).
///
/// `(1 - d^2)^2` and not a linear ramp: it leaves a broad flat core and spends
/// its whole falloff in the outer third, which is how a hot gas ball actually
/// reads - opaque in the middle, then gone. A linear ramp instead looks like a
/// cone lit from behind. Pure, so the curve can be read without an app.
fn soft_dot_falloff(d: f32) -> f32 {
    let inside = 1.0 - d.clamp(0.0, 1.0).powi(2);
    inside * inside
}

/// A square, linear, single-channel-meaningful mask: a feathered circle,
/// transparent at the corners.
///
/// Linear (`Rgba8Unorm`) rather than sRGB because this is a mask and not a
/// colour - a gamma curve applied to an opacity ramp bends the falloff for no
/// reason. All four channels carry the same value so the same asset works
/// under any of hanabi's sample mappings, not only the opacity one.
#[must_use]
pub fn soft_dot_image() -> Image {
    let texels = SOFT_DOT_TEXELS as usize;
    let half = SOFT_DOT_TEXELS as f32 / 2.0;
    let mut data = Vec::with_capacity(texels * texels * 4);
    for y in 0..texels {
        for x in 0..texels {
            // Texel CENTRES, so the circle is symmetric about the quad rather
            // than half a texel off toward the origin.
            let dx = (x as f32 + 0.5 - half) / half;
            let dy = (y as f32 + 0.5 - half) / half;
            let value = soft_dot_falloff(dx.hypot(dy));
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "the falloff is clamped to 0..=1, so the scaled value is in 0..=255"
            )]
            let byte = (value * 255.0).round() as u8;
            data.extend_from_slice(&[byte, byte, byte, byte]);
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: SOFT_DOT_TEXELS,
            height: SOFT_DOT_TEXELS,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    // Named explicitly rather than left to the app's default sampler: a
    // project that defaults to nearest filtering for its pixel art would get a
    // mask with visible stair-stepping on every glow in the game.
    image.sampler = ImageSampler::linear();
    image
}

/// The shared soft-dot mask, built on the first effect that binds it.
///
/// Held as a resource and cloned into each effect instance's `EffectMaterial`,
/// the same shape as the shared `EffectAsset` handles beside it: one texture
/// for the whole app, not one per detonation.
#[derive(Resource, Default, Debug)]
pub struct SoftDot(Option<Handle<Image>>);

impl SoftDot {
    /// The mask handle, generating the texture on the first call.
    pub fn handle(&mut self, images: &mut Assets<Image>) -> Handle<Image> {
        self.0
            .get_or_insert_with(|| images.add(soft_dot_image()))
            .clone()
    }
}

/// The render modifier that draws a particle through the mask.
///
/// Pairs with [`declare_soft_dot_slot`] on the same graph: this asks for slot
/// 0 and that call defines slot 0, and the two are separate only because
/// hanabi takes the modifier off the [`ExprWriter`] and the slot off the
/// [`Module`] the writer becomes. Every effect nova builds binds exactly one
/// texture, so slot 0 is the mask by construction.
///
/// `ModulateOpacityFromR` is the whole point - see the module docs. The
/// particle keeps the colour its own gradient gave it and gains only a round
/// edge.
#[must_use]
pub fn soft_dot_modifier(writer: &ExprWriter) -> ParticleTextureModifier {
    ParticleTextureModifier {
        texture_slot: writer.lit(0u32).expr(),
        sample_mapping: ImageSampleMapping::ModulateOpacityFromR,
    }
}

/// Define the slot [`soft_dot_modifier`] samples, on the finished module.
pub fn declare_soft_dot_slot(module: &mut Module) {
    module.add_texture_slot(SOFT_DOT_SLOT);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_falloff_is_opaque_at_the_centre_and_gone_at_the_edge() {
        assert_eq!(soft_dot_falloff(0.0), 1.0);
        assert_eq!(soft_dot_falloff(1.0), 0.0);
        assert_eq!(soft_dot_falloff(2.0), 0.0, "the corners clamp, not wrap");
    }

    #[test]
    fn the_falloff_keeps_a_broad_core_and_spends_itself_late() {
        // Half way out the dot is still more than half opaque: that flat
        // middle is what stops a glow reading as a cone.
        assert!(soft_dot_falloff(0.5) > 0.5);
        assert!(soft_dot_falloff(0.9) < 0.05);
        assert!(soft_dot_falloff(0.75) < soft_dot_falloff(0.25), "monotonic");
    }

    #[test]
    fn the_mask_is_round_not_square() {
        let image = soft_dot_image();
        let data = image.data.as_ref().expect("the mask carries its texels");
        let texels = SOFT_DOT_TEXELS as usize;
        let at = |x: usize, y: usize| data[(y * texels + x) * 4];

        let centre = at(texels / 2, texels / 2);
        let edge = at(texels - 1, texels / 2);
        let corner = at(texels - 1, texels - 1);
        assert_eq!(centre, 255, "the middle is fully opaque");
        assert_eq!(edge, 0, "the inscribed circle reaches zero at the edge");
        assert_eq!(
            corner, 0,
            "the corners are empty - this is what kills the square"
        );
    }

    #[test]
    fn the_handle_is_built_once_and_shared() {
        let mut images = Assets::<Image>::default();
        let mut dot = SoftDot::default();
        let first = dot.handle(&mut images);
        let second = dot.handle(&mut images);
        assert_eq!(first, second);
        assert_eq!(images.len(), 1, "a second ask must not mint a second mask");
    }
}
