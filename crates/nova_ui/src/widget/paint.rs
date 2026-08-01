//! Shared gradient and shadow builders for the themed widgets.

use bevy::prelude::*;

pub(super) fn grad3(top: Color, mid: Color, bottom: Color) -> BackgroundGradient {
    BackgroundGradient(vec![LinearGradient::degrees(
        180.0,
        vec![
            ColorStop::percent(top, 0.0),
            ColorStop::percent(mid, 55.0),
            ColorStop::percent(bottom, 100.0),
        ],
    )
    .into()])
}

pub(super) fn grad2(top: Color, bottom: Color) -> BackgroundGradient {
    BackgroundGradient(vec![LinearGradient::degrees(
        180.0,
        vec![
            ColorStop::percent(top, 0.0),
            ColorStop::percent(bottom, 100.0),
        ],
    )
    .into()])
}

/// The moulded-face drop shadow (`--drop`): outset only, since Bevy 0.19
/// `BoxShadow` has no inset shadows (the demo's inner rim/undercut are
/// approximated by the face gradient, matching the NOVA OS casing).
pub(super) fn drop_shadow() -> BoxShadow {
    BoxShadow::new(
        Color::srgba(0.0, 0.0, 0.0, 0.55),
        Val::ZERO,
        Val::Px(2.0),
        Val::ZERO,
        Val::Px(8.0),
    )
}

/// A coloured glow drop shadow (selected/primary faces). Kept subtle so it
/// reads as a lit element, not a blurry halo that fights the text.
pub(super) fn glow_shadow(color: Color) -> BoxShadow {
    BoxShadow::new(
        color.with_alpha(0.22),
        Val::ZERO,
        Val::ZERO,
        Val::ZERO,
        Val::Px(7.0),
    )
}
