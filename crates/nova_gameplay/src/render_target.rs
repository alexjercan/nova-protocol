//! Offscreen render targets: the one WebGL2-safe way to make a
//! render-to-texture image, and the in-place resize a target that tracks a UI
//! node needs.
//!
//! Not gameplay logic - this crate owns the recipe because it is the lowest
//! crate every consumer already shares: the HUD target inset (`nova_hud`), the
//! NOVA OS terminal/map/ship viewers (`nova_os_ui`), the render-scale lever
//! (`nova_scenario`) and the channel's frame recorder (`nova_channel`). Both
//! rules below are constraints that were each discovered once and are easy to
//! "clean up" back out of a single call site, so every offscreen target is born
//! and resized through this module.

use bevy::{
    prelude::*,
    render::render_resource::{Extent3d, TextureFormat},
};

/// The offscreen render-target recipe and the resize that keeps one sized to
/// its node.
pub mod prelude {
    pub use super::{new_render_target_image, resize_render_target};
}

/// Create an offscreen render target of `size` (clamped to at least 1x1, the
/// smallest texture wgpu will create). Rgba8UnormSrgb storage with no
/// view-format override; `new_target_texture` sets the RENDER_ATTACHMENT |
/// TEXTURE_BINDING | COPY_DST usages.
///
/// The `None` view format is a CONSTRAINT, not a default spelled out. Bevy's
/// 3d/render_to_texture example uses Rgba8Unorm storage with an Rgba8UnormSrgb
/// view instead, but a `Some` view format fills the texture's `view_formats`,
/// and creating such a texture needs `DownlevelFlags::VIEW_FORMATS` - absent on
/// WebGL2, where it is a fatal render validation error the moment the surface
/// spawns. An sRGB-format target with the default view goes through the same
/// Rgba8UnormSrgb view end to end, so native rendering is unchanged.
pub fn new_render_target_image(size: UVec2) -> Image {
    Image::new_target_texture(
        size.x.max(1),
        size.y.max(1),
        TextureFormat::Rgba8UnormSrgb,
        None,
    )
}

/// Resize an offscreen render target in place to `desired`, and tell the camera
/// that renders into it to re-derive its target info. A no-op while the image
/// already measures `desired`, so a steady frame never marks the asset changed.
///
/// `projection` is the projection of that camera; pass `None` when it is not
/// spawned yet, and the resize still lands.
///
/// Touching the projection is a workaround for the engine bug
/// `bevy-camera-ignores-runtime-rendertarget-swap`: bevy's `camera_system`
/// re-derives a camera's target size and scale only when the target CONTENT
/// changes, the camera is added, or its `Projection` changed. An in-place
/// `Image::resize` is none of those, so without the touch the camera keeps
/// drawing at the old size and the surface shows a stretched frame. When bevy
/// fixes the bug, dropping the touch here drops it from every viewer at once.
pub fn resize_render_target(
    images: &mut Assets<Image>,
    image: &Handle<Image>,
    desired: UVec2,
    projection: Option<Mut<'_, Projection>>,
) {
    let needs_resize = images
        .get(image)
        .map(|img| img.size() != desired)
        .unwrap_or(true);
    if !needs_resize {
        return;
    }
    if let Some(mut img) = images.get_mut(image) {
        img.resize(Extent3d {
            width: desired.x,
            height: desired.y,
            depth_or_array_layers: 1,
        });
    }
    if let Some(mut projection) = projection {
        projection.set_changed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WebGL2-safe invariant of every offscreen render target (v0.5.0 web crash
    /// regression): a non-empty `view_formats` needs
    /// `DownlevelFlags::VIEW_FORMATS`, which WebGL2 lacks, so `create_texture`
    /// fails validation and Bevy quits the app on game start. A target must be
    /// born plain sRGB with the default view.
    #[test]
    fn a_render_target_is_born_webgl2_safe() {
        let image = new_render_target_image(UVec2::new(256, 128));
        assert_eq!(
            image.texture_descriptor.format,
            TextureFormat::Rgba8UnormSrgb,
            "target renders and samples as sRGB"
        );
        assert!(
            image.texture_descriptor.view_formats.is_empty(),
            "non-empty view_formats is a fatal validation error on WebGL2"
        );
        assert!(
            image.texture_view_descriptor.is_none(),
            "no view override; the default view already has the sRGB format"
        );
    }

    /// wgpu refuses a zero-sized texture, and a viewport node measures zero on
    /// the frames before its first layout pass.
    #[test]
    fn a_render_target_is_never_born_zero_sized() {
        let image = new_render_target_image(UVec2::ZERO);
        assert_eq!(image.texture_descriptor.size.width, 1);
        assert_eq!(image.texture_descriptor.size.height, 1);
    }

    /// A target follows the node it fills: a new size resizes the SAME image,
    /// so every camera and material already holding the handle keeps working.
    #[test]
    fn a_render_target_resizes_in_place_to_the_size_asked_for() {
        let mut images = Assets::<Image>::default();
        let handle = images.add(new_render_target_image(UVec2::new(320, 200)));

        resize_render_target(&mut images, &handle, UVec2::new(640, 400), None);

        assert_eq!(
            images.get(&handle).expect("target").size(),
            UVec2::new(640, 400)
        );
    }
}
