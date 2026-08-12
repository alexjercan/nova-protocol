//! Every tuned constant of the NOVA OS look: timings, alphas, font sizes,
//! paddings and insets.
//!
//! Named and centralised so a visual change is one edit and the screenshot
//! baselines have a single thing to diff against.
//!
//! Touch this module when retuning the monitor's spacing, timing or type.

use bevy::prelude::*;
use nova_ui::font::UiFont;

/// Seconds for the monitor to fade/activate fully open (or closed).
pub(crate) const DRAWER_SLIDE_SECS: f32 = 0.22;
/// Backdrop dim at full open. With the flight HUD hidden while the NOVA OS is open, the
/// backdrop is the ONLY thing separating the NOVA OS from the frozen scene, so
/// it doubles as the "you do not notice the old UI is gone" gray field. A deeper
/// gray rather than a real scene blur: bevy 0.19 has no UI backdrop-filter.
pub(crate) const DRAWER_BACKDROP_ALPHA: f32 = 0.94;
pub(crate) const DRAWER_SECTION_TITLE_FONT_PX: f32 = 14.0;
pub(crate) const DRAWER_LINE_FONT_PX: f32 = 16.0;
/// The staggered boot banner reveals one row this far apart, on real time so it
/// runs while virtual time is frozen (PoC `printBanner`'s ~130 ms cadence).
pub(crate) const NOVA_OS_BOOT_ROW_INTERVAL: f32 = 0.13;
/// The block caret's WIDTH as a fraction of the font size (PoC `.caret` is
/// 0.6em). This is the cursor block's drawn width only - NOT the glyph advance,
/// so it is not used to position the caret (that measures the real text width;
/// see [`super::shell::position_nova_os_block_caret`]).
pub(crate) const NOVA_OS_CARET_WIDTH_FRACTION: f32 = 0.6;
pub(crate) const DRAWER_SCROLL_LINE_HEIGHT_PX: f32 = 20.0;

/// Horizontal inset from the viewport edge to the physical monitor casing. Kept
/// small so the monitor sits almost at the screen edges (the top status-bar
/// chrome may overlap it - that is intentional).
pub(crate) const NOVA_OS_MONITOR_INSET_X_PX: f32 = 16.0;
/// Vertical inset from the viewport edge to the physical monitor casing.
pub(crate) const NOVA_OS_MONITOR_INSET_Y_PX: f32 = 14.0;
pub(crate) const NOVA_OS_BEZEL_PAD_PX: f32 = 26.0;
pub(crate) const NOVA_OS_SCREEN_PAD_PX: f32 = 18.0;
/// Safe-area inset for the actual SCREEN CONTENT (terminal + apps), as a
/// percentage of the content width per side, so it scales with resolution.
/// The CRT shader overscans the warped picture by `NOVA_OS_OVERSCAN` (~3.5% of
/// each edge is pushed under the bezel to hide the barrel-bowed corners), so a
/// flat 18px inset let edge text tuck under the bezel (owner playtest: "text is
/// not visible on the borders"). These clear the hidden band with a margin;
/// most of the padding lands under the bezel, leaving a small visible margin.
/// Horizontal and vertical differ because the hidden band is a fraction of each
/// dimension but percentage padding is width-relative on both axes.
pub(crate) const NOVA_OS_CONTENT_SAFE_X_PCT: f32 = 5.5;
pub(crate) const NOVA_OS_CONTENT_SAFE_Y_PCT: f32 = 3.6;
/// Fixed height of the persistent NOVA OS header bar (`<header>`), so it never
/// reflows when the middle `<main>` region swaps between the terminal and an
/// app. Matches the old in-terminal topbar box: 32px content + 10px bottom pad
/// + 1px bottom border.
pub(crate) const NOVA_OS_HEADER_HEIGHT_PX: f32 = 43.0;
/// Fixed height of the persistent NOVA OS footer bar (`<footer>`), so it stays
/// constant next to the flexing main region. Matches the old footer box: 18px
/// content + 6px top pad + 1px top border, rounded up for breathing room.
pub(crate) const NOVA_OS_FOOTER_HEIGHT_PX: f32 = 26.0;
/// Injection-moulded shell corners: a larger top radius and a tighter bottom,
/// like the PoC `.case` `border-radius: 22px 22px 14px 14px`, scaled up for the
/// full-viewport monitor.
pub(crate) const NOVA_OS_CASE_RADIUS_TOP_PX: f32 = 24.0;
pub(crate) const NOVA_OS_CASE_RADIUS_BOTTOM_PX: f32 = 15.0;
/// Recessed bezel + phosphor-screen corner radii (PoC `.bezel` 16px, screen 12).
pub(crate) const NOVA_OS_BEZEL_RADIUS_PX: f32 = 16.0;
pub(crate) const NOVA_OS_SCREEN_RADIUS_PX: f32 = 12.0;
/// Bottom casing strip under the bezel (PoC `.chin`, ~54px) that carries the
/// brand plate and the reserved controls row.
pub(crate) const NOVA_OS_CHIN_HEIGHT_PX: f32 = 54.0;

/// BRIGHT knob detents: the extra brightness multiply fed
/// to the CRT `brightness` uniform, mirroring the PoC `BRIGHT` array. Index
/// [`NOVA_OS_BRIGHT_DEFAULT_DETENT`] (= 1.0) is the shipped neutral default.
pub(crate) const NOVA_OS_BRIGHT_DETENTS: [f32; 4] = [0.8, 1.0, 1.15, 1.3];
/// SCAN knob detents: the scanline-strength uniform. Index
/// [`NOVA_OS_SCAN_DEFAULT_DETENT`] is [`NOVA_OS_CRT_SCANLINE_STRENGTH`], the
/// shipped default look; 0 turns scanlines off, index 3 is a heavy, obviously
/// aggressive raster (owner call 2026-07-27). (The PoC's [0, 0.18, 0.34, 0.52]
/// were CSS-overlay opacities; the in-game shader's `scanline_strength` darkens
/// far harder, so the range is scaled to it.)
pub(crate) const NOVA_OS_SCAN_DETENTS: [f32; 4] = [0.0, 0.03, NOVA_OS_CRT_SCANLINE_STRENGTH, 0.20];
/// Dial-pointer angle (degrees) per detent index, mirroring the PoC `ANGLES`.
pub(crate) const NOVA_OS_KNOB_ANGLES: [f32; 4] = [-115.0, -38.0, 38.0, 115.0];
/// Default BRIGHT detent index (PoC `brightIndex = 1`, = neutral 1.0).
pub(crate) const NOVA_OS_BRIGHT_DEFAULT_DETENT: usize = 1;
/// Default SCAN detent index (PoC `scanIndex = 2`, = the shipped scanline look).
pub(crate) const NOVA_OS_SCAN_DEFAULT_DETENT: usize = 2;
pub(crate) const NOVA_OS_TERMINAL_PAD_X_PX: f32 = 16.0;
pub(crate) const NOVA_OS_TERMINAL_PAD_Y_PX: f32 = 14.0;
pub(crate) const NOVA_OS_PROMPT_ROW_HEIGHT_PX: f32 = 58.0;
pub(crate) const NOVA_OS_BACKDROP: Color = Color::srgb_u8(0, 3, 6);
// Dark-GRAY moulded plastic, matching the PoC `:root` `--case-*` (neutral, not
// blue). `--case-0`, a mid raised body, and `--case-edge`.
pub(crate) const NOVA_OS_CASE: Color = Color::srgb_u8(10, 13, 16);
pub(crate) const NOVA_OS_CASE_RAISED: Color = Color::srgb_u8(16, 22, 27);
pub(crate) const NOVA_OS_CASE_EDGE: Color = Color::srgb_u8(5, 7, 10);
pub(crate) const NOVA_OS_SCREEN: Color = Color::srgb_u8(0, 4, 1);
// Palette lifted from `nova_os_terminal_poc.html`: a hot neon phosphor for the
// prompt, borders and headers; a pale mint for ordinary body text (the HTML
// `--text`), which reads brighter and higher-contrast on the near-black screen
// than the old all-one-green treatment.
pub(crate) const NOVA_OS_PHOSPHOR: Color = Color::srgb_u8(54, 255, 121);
pub(crate) const NOVA_OS_TEXT: Color = Color::srgb_u8(185, 255, 201);
pub(crate) const NOVA_OS_PHOSPHOR_DIM: Color = Color::srgb_u8(95, 238, 137);
pub(crate) const NOVA_OS_PHOSPHOR_MUTED: Color = Color::srgb_u8(70, 207, 118);
pub(crate) const NOVA_OS_INFO: Color = Color::srgb_u8(54, 163, 255);
pub(crate) const NOVA_OS_AMBER: Color = Color::srgb_u8(255, 184, 74);
// Moulded-plastic depth palette (casing gradient stops, screws, seam catch).
// The PoC `.case` body runs a 168deg gradient from a lit top (`--case-3`) down
// through the mid body to an almost-black undercut; these are those `--case-*`
// stops (dark GRAY, not blue).
pub(crate) const NOVA_OS_CASE_LIT: Color = Color::srgb_u8(47, 56, 63);
pub(crate) const NOVA_OS_CASE_MID: Color = Color::srgb_u8(22, 27, 32);
pub(crate) const NOVA_OS_CASE_DEEP: Color = Color::srgb_u8(10, 13, 16);
/// The 1px top light line that catches the moulding lip (PoC `inset 0 1px 0`).
pub(crate) const NOVA_OS_CASE_HIGHLIGHT: Color = Color::srgba(1.0, 1.0, 1.0, 0.12);
/// Screw head shading (PoC `.screw` radial gradient light -> dark).
pub(crate) const NOVA_OS_SCREW_LIT: Color = Color::srgb_u8(89, 101, 110);
pub(crate) const NOVA_OS_SCREW_DARK: Color = Color::srgb_u8(10, 13, 16);
/// Chin-button moulding (PoC `.power-btn` `linear-gradient(180deg,#333c44,#1a2026)`
/// with a 1px inner top-highlight and a near-black outer border): a small raised
/// pill of plastic, lighter than the surrounding case so it reads as a pressable
/// key rather than a painted rectangle.
pub(crate) const NOVA_OS_BUTTON_LIT: Color = Color::srgb_u8(51, 60, 68);
pub(crate) const NOVA_OS_BUTTON_DEEP: Color = Color::srgb_u8(26, 32, 38);
pub(crate) const NOVA_OS_BUTTON_BORDER: Color = Color::srgba(0.0, 0.0, 0.0, 0.75);
/// Knob dial dome (PoC `.dial` `radial-gradient(circle at 34% 28%,#4a555d,#232a30
/// 58%,#0d1114)`): an off-centre highlight over a dark disc gives the moulded
/// rotary its rounded 3D body.
pub(crate) const NOVA_OS_DIAL_LIT: Color = Color::srgb_u8(74, 85, 93);
pub(crate) const NOVA_OS_DIAL_MID: Color = Color::srgb_u8(35, 42, 48);
pub(crate) const NOVA_OS_DIAL_DARK: Color = Color::srgb_u8(13, 17, 20);
/// The PWR button/LED flashes this warm orange while the monitor is powering
/// down (owner playtest: "turn orange and then close"), before the raster
/// collapse finishes the close.
pub(crate) const NOVA_OS_ORANGE: Color = Color::srgb_u8(255, 120, 40);
/// An unlit green bulb: the SND indicator dims to this dark phosphor when muted,
/// so the bulb (not a text swap) carries the on/off state.
pub(crate) const NOVA_OS_BULB_OFF: Color = Color::srgb_u8(18, 34, 22);
pub(crate) const NOVA_OS_CONTENT_Z: i32 = 0;
pub(crate) const NOVA_OS_OVERLAY_Z: i32 = 1;
/// Phosphor rim traces the screen edge above the CRT overlay; the glass sheen is
/// the frontmost surface layer over it.
pub(crate) const NOVA_OS_RIM_Z: i32 = 2;
pub(crate) const NOVA_OS_GLASS_Z: i32 = 3;
/// Blink rate of the terminal caret, in full on/off cycles per second.
pub(crate) const NOVA_OS_CARET_BLINK_HZ: f32 = 1.25;

/// Straight-alpha CRT overlay tint + scanline controls, passed to WGSL. Kept
/// deliberately faint so the overlay never films the text underneath: the tint
/// is a whisper of green, the vignette darkens only the outer edges, and the
/// centre glow is a low bulge that reads as volume rather than a wash (see
/// `assets/shaders/nova_os_crt.wgsl`).
pub(crate) const NOVA_OS_CRT_TINT: LinearRgba = LinearRgba::new(0.212, 1.0, 0.475, 0.03);
pub(crate) const NOVA_OS_CRT_SCANLINE_STRENGTH: f32 = 0.06;
pub(crate) const NOVA_OS_CRT_VIGNETTE_STRENGTH: f32 = 0.55;
/// Centre-peaked phosphor bulge that gives the flat panel its CRT volume and a
/// clearly brighter middle (the HTML radial-gradient centre).
pub(crate) const NOVA_OS_CRT_GLOW_STRENGTH: f32 = 0.07;
pub(crate) const NOVA_OS_CRT_GRAIN_STRENGTH: f32 = 0.03;
/// Barrel-warp amount for the sampling shader: a gentle bow that reads as a tube
/// without pushing corner text past readability (curvature-vs-readability, tuned
/// by playtest). Bloom is the soft green glyph halo.
pub(crate) const NOVA_OS_CRT_WARP: f32 = 0.12;
pub(crate) const NOVA_OS_CRT_BLOOM: f32 = 0.85;

/// CRT overscan: after the barrel warp bows the sampled UV outward, the shader
/// pulls it back toward centre by this factor so the bowed corners land under the
/// bezel instead of sampling past the picture and reading as a tube-black margin
/// (a real CRT's overscan). `< 0.943` clears the corner at [`NOVA_OS_CRT_WARP`].
///
/// This lives HERE, in Rust, and is fed to the shader as a uniform: it is half of
/// the screen->image mapping [`super::crt::nova_os_crt_screen_to_image_uv`] performs for the
/// forwarded pointer, and a WGSL-local copy is a second definition the pointer
/// cannot see - which is exactly how the mis-click bug happened.
pub(crate) const NOVA_OS_CRT_OVERSCAN: f32 = 0.93;

/// Degauss pulse duration in seconds: how long the coil
/// wobble+flash rings out after an app launch/exit/switch. Short enough to feel
/// like a physical coil settle, long enough to read. The envelope is
/// `remaining / NOVA_OS_DEGAUSS_DURATION`, fed to the shader's `degauss` uniform.
pub(crate) const NOVA_OS_DEGAUSS_DURATION: f32 = 0.45;

/// Global stacking-context z for the OPEN NOVA OS: it is a modal, so backdrop and
/// panel rise above the flight HUD chrome (which carries no `GlobalZIndex` = 0).
/// Same modal tier the pause overlay uses (`nova_menu`); the NOVA OS and the
/// pause menu are mutually exclusive `PauseStates` variants, so sharing the tier
/// is fine. The tab handle stays at the HUD z (it is chrome).
pub(crate) const DRAWER_BACKDROP_Z: i32 = 10;
pub(crate) const DRAWER_PANEL_Z: i32 = 11;
/// z for NOVA OS-exempt diagnostic/status chrome that stays visible while the
/// NOVA OS is open: it must sit above the deepened backdrop so the gray field
/// cannot dim it. Read by status widgets that tag themselves
/// [`nova_hud::prelude::HudNovaOsExempt`].
pub(crate) const DRAWER_EXEMPT_Z: i32 = 12;

pub(crate) fn nova_os_font(ui_font: Option<&UiFont>) -> Handle<Font> {
    ui_font.map(UiFont::handle).unwrap_or_default()
}

pub(crate) fn nova_os_text_font(font_size: f32, font: Handle<Font>) -> TextFont {
    TextFont {
        font: FontSource::Handle(font),
        font_size: FontSize::Px(font_size),
        ..default()
    }
}

/// The prompt/ghost pieces must never wrap: a wrapped ghost is exactly the
/// "completion appears below the line" bug. `NoWrap` keeps every piece on the
/// single input line and lets the wrap node clip horizontally instead.
pub(crate) fn nova_os_prompt_text_layout() -> TextLayout {
    TextLayout {
        justify: Justify::Left,
        linebreak: LineBreak::NoWrap,
    }
}
