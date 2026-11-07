//! One-line text that ends in `...` when its cell is too narrow.
//!
//! A `NoWrap` label in a clipped cell loses its tail without a sign that
//! anything is missing. [`EllipsisText`] holds the whole text; the fitter
//! shortens what the label draws and restores it when the cell widens.

use bevy::{
    prelude::*,
    text::{ComputedTextBlock, TextLayoutInfo},
};

/// What a shortened text ends in.
const ELLIPSIS: &str = "...";

/// The whole text of a one-line label that shortens to fit its cell.
///
/// Put it on a `Text` node with `LineBreak::NoWrap` whose parent is the cell:
/// a node with `min_width: px(0)` and `Overflow::clip()`, so the cell can be
/// narrower than the text. Replace the component to change the text.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
#[require(EllipsisFit)]
pub struct EllipsisText(pub String);

/// What [`fit_ellipsis_texts`] remembers of one label.
///
/// The cut belongs to one text: `source` is the whole text it was made for,
/// so a replaced text shows whole and is fitted again, even when the old cut
/// happens to be a prefix of the new text. `overflowed_at` is the cell width,
/// in physical px, at which the text was last cut, zero while the whole text
/// shows.
///
/// A cut text goes back to the whole text only when its cell is wider than
/// `overflowed_at`. Asking whether the whole text would fit needs its width,
/// and the one way to learn that is to lay it out; at an unchanged cell width
/// that answer is already known, and asking again would flip the label
/// between the two texts every frame.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct EllipsisFit {
    source: String,
    overflowed_at: f32,
}

/// One place the shown text may be cut: a laid-out glyph starts at byte
/// `byte`, `x` physical px from the left edge of the text. The last edge of a
/// text is its end.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Edge {
    byte: usize,
    x: f32,
}

/// Fit each [`EllipsisText`] to its cell with [`fit`]. It reads this frame's
/// text layout, so a cut or restored text draws one frame after its label
/// spawns, its text changes or its cell resizes.
pub(crate) fn fit_ellipsis_texts(
    mut labels: Query<(
        &EllipsisText,
        &mut EllipsisFit,
        &mut Text,
        &TextLayoutInfo,
        &ComputedTextBlock,
        &ComputedNode,
        &ChildOf,
    )>,
    cells: Query<&ComputedNode>,
) {
    for (full, mut state, mut text, layout, block, node, child_of) in &mut labels {
        // A new text has no layout yet: draw it whole and fit it next frame.
        // The content decides, not the change tick: a surface that writes
        // the same text again every frame leaves the cut one alone.
        let replaced = state.source != full.0;
        if replaced || (text.0 != full.0 && !is_cut_of(&text.0, &full.0)) {
            if text.0 != full.0 {
                text.0.clone_from(&full.0);
            }
            if replaced {
                state.source.clone_from(&full.0);
            }
            if state.overflowed_at != 0.0 {
                state.overflowed_at = 0.0;
            }
            continue;
        }
        let Ok(cell) = cells.get(child_of.parent()) else {
            continue;
        };
        // Compare in physical px, where the layout rounds the width up
        // before it stores it in logical px; the round undoes the float
        // error of that conversion.
        let shown_width = (layout.size.x / node.inverse_scale_factor()).round();
        let width = cell.content_box().size().x;
        let Some((fitted, overflowed_at)) =
            fit(&full.0, &text.0, shown_width, width, &state, || {
                measured_edges(block)
            })
        else {
            continue;
        };
        if text.0 != fitted {
            text.0 = fitted;
        }
        if state.overflowed_at != overflowed_at {
            state.overflowed_at = overflowed_at;
        }
    }
}

/// Whether `shown` is `full` cut to a prefix with the ellipsis after it.
fn is_cut_of(shown: &str, full: &str) -> bool {
    shown
        .strip_suffix(ELLIPSIS)
        .is_some_and(|kept| full.starts_with(kept))
}

/// Where a laid-out text starts each glyph, from the shaper's own clusters.
///
/// The shaper reports a base plus its combining mark, or an emoji ZWJ
/// sequence, as several clusters of which only the first holds the glyphs;
/// the rest are ligature continuations, and a cut before one would draw half
/// of the glyph. `x` adds the cluster advances in logical order, so the text
/// must be left to right and on one line, as an `EllipsisText` is.
fn measured_edges(block: &ComputedTextBlock) -> Vec<Edge> {
    let mut edges = Vec::new();
    let mut x = 0.0;
    let mut end = 0;
    if let Some(line) = block.buffer().lines().next() {
        for run in line.runs() {
            for cluster in run.clusters() {
                let range = cluster.text_range();
                if !cluster.is_ligature_continuation() {
                    edges.push(Edge {
                        byte: range.start,
                        x,
                    });
                }
                x += cluster.advance();
                end = range.end;
            }
        }
    }
    edges.push(Edge { byte: end, x });
    edges
}

/// A cell must be this much wider than at the last cut, in physical px,
/// before the whole text is tried again: under it the difference is the
/// layout's own rounding.
const REGROW_PX: f32 = 0.5;

/// The text a `width` cell shows for `full`, and the cell width it was cut
/// at (zero for the whole text), given the `shown` text laid out
/// `shown_width` wide, or `None` to keep both. The widths are physical px.
/// `edges` measures the shown text and is asked for only when it overflows.
///
/// An overflowing text is cut to the longest prefix, ending where a glyph
/// starts, whose measured width plus the ellipsis fits; see [`cut`]. A cut
/// text goes back to the whole text when its cell is wider than at the cut,
/// and the next frame cuts it for the new width if it still overflows.
fn fit(
    full: &str,
    shown: &str,
    shown_width: f32,
    width: f32,
    state: &EllipsisFit,
    edges: impl FnOnce() -> Vec<Edge>,
) -> Option<(String, f32)> {
    // Not laid out yet: nothing is measured.
    if shown.is_empty() || shown_width <= 0.0 {
        return None;
    }
    if shown_width > width {
        // The bare ellipsis is the shortest cut. A cell narrower than it has
        // nothing left to drop, so it is not measured again each frame.
        if shown == ELLIPSIS {
            return None;
        }
        return Some((cut(full, shown, &edges(), width), width));
    }
    (shown != full && width > state.overflowed_at + REGROW_PX).then(|| (full.to_string(), 0.0))
}

/// `full` cut at a glyph start, with the ellipsis after it, for a `width`
/// cell, from the `edges` of the overflowing `shown` text.
///
/// The cut is the longest that fits and always shorter than `shown`, so at
/// one cell width the text only gets shorter and settles. The ellipsis width
/// is read from `shown` when it ends in one. A whole text has none to
/// measure: it is three of the narrowest glyph advance in the text, exact for
/// the monospace UI font and never too wide, and the next frame measures the
/// real one and cuts again if it overflows.
fn cut(full: &str, shown: &str, edges: &[Edge], width: f32) -> String {
    let kept_len = if shown == full {
        None
    } else {
        shown
            .strip_suffix(ELLIPSIS)
            .filter(|kept| full.starts_with(kept))
            .map(str::len)
    };
    let end = edges.last().map_or(0.0, |edge| edge.x);
    let measured = kept_len.and_then(|len| {
        edges
            .iter()
            .find(|edge| edge.byte == len)
            .map(|edge| end - edge.x)
    });
    let ellipsis = measured.unwrap_or_else(|| {
        let narrowest = edges
            .windows(2)
            .map(|pair| pair[1].x - pair[0].x)
            .filter(|advance| *advance > 0.0)
            .fold(f32::INFINITY, f32::min);
        if narrowest.is_finite() {
            narrowest * ELLIPSIS.len() as f32
        } else {
            0.0
        }
    });
    let limit = kept_len.unwrap_or(full.len());
    let kept = edges
        .iter()
        .filter(|edge| edge.byte < limit && full.is_char_boundary(edge.byte))
        .take_while(|edge| (edge.x + ellipsis).ceil() <= width)
        .last()
        .map_or("", |edge| &full[..edge.byte]);
    format!("{}{ELLIPSIS}", kept.trim_end())
}

/// Wire the fitter.
pub(crate) fn build(app: &mut App) {
    // Reads the frame's text layout, which the UI writes in PostLayout.
    app.add_systems(
        PostUpdate,
        fit_ellipsis_texts.after(bevy::ui::UiSystems::PostLayout),
    );
}

#[cfg(test)]
mod tests {
    use bevy::{
        asset::AssetPlugin,
        camera::{Camera, ComputedCameraValues, RenderTargetInfo},
        image::TextureAtlasLayout,
        math::UVec2,
        text::Font,
    };

    use super::*;

    const NARROW: f32 = 6.5;
    const WIDE: f32 = 13.0;

    fn cut_at(width: f32) -> EllipsisFit {
        EllipsisFit {
            source: String::new(),
            overflowed_at: width,
        }
    }

    /// The measured layout of `text` at 13 px Iosevka Term: 6.5 px per char,
    /// 13 px for a CJK char or an emoji, and a ZWJ, a combining mark or a
    /// char after a ZWJ joins the glyph before it. Returns the cut edges and
    /// the width the layout reports (rounded up).
    fn lay_out(text: &str) -> (Vec<Edge>, f32) {
        let mut edges = Vec::new();
        let mut x = 0.0;
        let mut joined = false;
        for (byte, ch) in text.char_indices() {
            let joins = joined || ch == '\u{200d}' || ch == '\u{301}';
            joined = ch == '\u{200d}';
            if !joins {
                edges.push(Edge { byte, x });
                x += if ch > '\u{2e80}' { WIDE } else { NARROW };
            }
        }
        edges.push(Edge {
            byte: text.len(),
            x,
        });
        (edges, x.ceil())
    }

    /// One frame of the system on the table layout: what `shown` becomes.
    fn frame(full: &str, shown: &str, cell: f32, state: &mut EllipsisFit) -> String {
        let (edges, width) = lay_out(shown);
        match fit(full, shown, width, cell, state, || edges) {
            Some((next, overflowed_at)) => {
                state.overflowed_at = overflowed_at;
                next
            }
            None => shown.to_string(),
        }
    }

    /// Iosevka Term at 13 px advances 6.5 px, and the layout rounds a width up.
    #[test]
    fn a_text_too_wide_for_its_cell_ends_in_an_ellipsis_and_restores_once_the_cell_widens() {
        let full = "Capacitor bank";
        let laid = |text: &str| lay_out(text).0;
        // 14 chars lay out 91 px wide; an 84 px cell holds 12.
        assert_eq!(
            fit(full, full, 91.0, 84.0, &EllipsisFit::default(), || laid(
                full
            )),
            Some(("Capacitor...".to_string(), 84.0))
        );
        // The cell has not widened: keep the cut text, whatever its width
        // says would fit.
        assert_eq!(
            fit(full, "Capacitor...", 78.0, 84.0, &cut_at(84.0), || laid(
                "Capacitor..."
            )),
            None
        );
        // A wider cell tries the whole text again.
        assert_eq!(
            fit(full, "Capacitor...", 78.0, 91.0, &cut_at(84.0), || laid(
                "Capacitor..."
            )),
            Some((full.to_string(), 0.0))
        );
        // 13 chars round 84.5 px up to 85 and overflow an 84.6 px cell. The
        // 12-char text that replaces it lays out 78 px, which would hold 13
        // chars again: it must stay.
        let full = "Flight computer core";
        assert_eq!(
            fit(full, "Flight com...", 85.0, 84.6, &cut_at(84.6), || laid(
                "Flight com..."
            )),
            Some(("Flight co...".to_string(), 84.6))
        );
        assert_eq!(
            fit(full, "Flight co...", 78.0, 84.6, &cut_at(84.6), || laid(
                "Flight co..."
            )),
            None
        );
    }

    /// Wide glyphs past the cut make the whole text wider than its cut
    /// prefix predicts. The label must not swap between the two each frame.
    #[test]
    fn a_text_with_wide_glyphs_stays_cut_at_an_unchanged_cell_width() {
        let full = "Outpost \u{6771}\u{4eac}\u{6771}\u{4eac}\u{6771}\u{4eac}";
        let mut state = EllipsisFit::default();
        let cut = frame(full, full, 84.0, &mut state);
        // The cut prefix is narrow glyphs only, with room to spare.
        assert_eq!(frame(full, &cut, 84.0, &mut state), cut);
    }

    #[test]
    fn a_replaced_text_is_not_mistaken_for_a_cut_of_the_old_one() {
        assert!(is_cut_of("Capacitor...", "Capacitor bank"));
        assert!(!is_cut_of("Capacitor...", "Flight computer core"));
        assert!(!is_cut_of("Capacitor bank", "Capacitor bank"));
    }

    /// Drive `full` in a `cell` px cell for `count` frames, then widen the
    /// cell and narrow it again. Wide glyphs sit before, across and after the
    /// cut, and a ZWJ sequence lies across it.
    #[test]
    fn a_text_with_wide_glyphs_never_returns_to_a_text_it_left_at_one_cell_width() {
        // One wide glyph over the cell costs one glyph, not the average
        // advance's worth: 8 narrow and 3 wide glyphs, 91 px, in 78 px with
        // the 19.5 px ellipsis keep nine narrow glyphs.
        let full = "Outpost A\u{6771}\u{4eac}\u{6771}";
        assert_eq!(
            frame(full, full, 78.0, &mut EllipsisFit::default()),
            "Outpost A..."
        );
        let texts = [
            "Capacitor assembly \u{6771}\u{4eac}\u{6771}\u{4eac}",
            "\u{6771}\u{4eac}\u{6771}\u{4eac} Capacitor assembly",
            "Cap \u{6771}\u{4eac} bank \u{6771}\u{4eac} assembly",
            "Pod bank \u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467} e\u{301}e\u{301}x tail",
            "\u{6771}\u{4eac}\u{6771}\u{4eac}\u{6771}\u{4eac}\u{6771}\u{4eac}\u{6771}\u{4eac}",
        ];
        for full in texts {
            for cell in [40.0, 59.0, 84.0, 84.6, 97.5] {
                let mut state = EllipsisFit::default();
                let mut shown = full.to_string();
                let mut seen: Vec<String> = Vec::new();
                for step in 0..12 {
                    let next = frame(full, &shown, cell, &mut state);
                    if next != shown {
                        assert!(
                            !seen.contains(&next),
                            "{full:?} in {cell} px, step {step}: {shown:?} went back to {next:?}"
                        );
                        seen.push(shown.clone());
                        shown = next;
                    }
                }
                assert_eq!(frame(full, &shown, cell, &mut state), shown);
                assert!(
                    lay_out(&shown).1 <= cell || shown == "...",
                    "{full:?} in {cell} px settled on {shown:?}"
                );
                // A cell that widens shows the whole text, then cuts it for
                // the new width and settles again.
                let wider = cell + 40.0;
                for _ in 0..12 {
                    shown = frame(full, &shown, wider, &mut state);
                }
                assert!(
                    lay_out(&shown).1 <= wider,
                    "{full:?} in {wider} px settled on {shown:?}"
                );
                assert_eq!(frame(full, &shown, wider, &mut state), shown);
            }
        }
    }

    #[test]
    fn a_cut_never_splits_a_glyph_cluster() {
        // Cell widths from nothing to the whole text, in steps that land the
        // cut inside every sequence in turn.
        let full = "Pod bank \u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467} e\u{301}e\u{301}x tail";
        let starts: Vec<usize> = lay_out(full).0.iter().map(|edge| edge.byte).collect();
        for tenth in 0..1500 {
            let cell = tenth as f32 / 10.0;
            let mut state = EllipsisFit::default();
            let mut shown = full.to_string();
            for _ in 0..12 {
                shown = frame(full, &shown, cell, &mut state);
            }
            let kept = shown.strip_suffix(ELLIPSIS).unwrap_or(&shown);
            assert!(
                shown == full || (full.starts_with(kept) && starts.contains(&kept.len())),
                "{cell} px cut {shown:?}, inside a cluster"
            );
        }
    }

    /// The cell and the label under test, in a real taffy and text layout.
    ///
    /// The font is the game's UI typeface; it holds no CJK or emoji glyphs,
    /// so those lay out as one fallback glyph each, which is all the cut
    /// reads.
    const IOSEVKA: &[u8] = include_bytes!("../../../../assets/fonts/SGr-IosevkaTerm-Medium.ttf");

    fn rig(full: &str, cell_px: f32) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            TransformPlugin,
            bevy::a11y::AccessibilityPlugin,
            bevy::input::InputPlugin,
            bevy::picking::PickingPlugin,
            bevy::picking::InteractionPlugin,
            bevy::text::TextPlugin,
            bevy::ui::UiPlugin,
        ));
        app.init_asset::<Image>().init_asset::<TextureAtlasLayout>();
        build(&mut app);
        app.world_mut().spawn((
            Camera2d,
            Camera {
                computed: ComputedCameraValues {
                    target_info: Some(RenderTargetInfo {
                        physical_size: UVec2::new(1280, 720),
                        scale_factor: 1.0,
                    }),
                    ..default()
                },
                ..default()
            },
        ));
        let font = app
            .world_mut()
            .resource_mut::<Assets<Font>>()
            .add(Font::from_bytes(IOSEVKA.to_vec()));
        let cell = app
            .world_mut()
            .spawn(Node {
                width: px(cell_px),
                min_width: px(0),
                overflow: Overflow::clip(),
                ..default()
            })
            .id();
        let label = app
            .world_mut()
            .spawn((
                Text::new(full),
                TextFont {
                    font: font.into(),
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextLayout::new(Justify::Left, LineBreak::NoWrap),
                EllipsisText(full.to_string()),
                ChildOf(cell),
            ))
            .id();
        (app, cell, label)
    }

    fn shown(app: &App, label: Entity) -> String {
        app.world().get::<Text>(label).expect("a label").0.clone()
    }

    /// Run `count` frames and return the shown text after each.
    fn frames(app: &mut App, label: Entity, count: usize) -> Vec<String> {
        (0..count)
            .map(|_| {
                app.update();
                shown(app, label)
            })
            .collect()
    }

    /// The laid-out width of the label, in px at scale 1.
    fn laid_out_width(app: &App, label: Entity) -> f32 {
        app.world()
            .get::<TextLayoutInfo>(label)
            .expect("a label")
            .size
            .x
    }

    fn set_cell_width(app: &mut App, cell: Entity, width: f32) {
        app.world_mut().get_mut::<Node>(cell).expect("a cell").width = px(width);
    }

    #[test]
    fn a_replaced_text_that_fits_shows_whole_even_when_the_old_cut_prefixes_it() {
        let (mut app, _cell, label) = rig("Raider 12", 52.0);
        let cut = frames(&mut app, label, 6);
        assert!(
            cut[5].ends_with(ELLIPSIS) && "Raider 12".starts_with(cut[5].trim_end_matches('.')),
            "the long name is cut: {cut:?}"
        );
        // The cut is a prefix of the new text, which fits the cell whole.
        app.world_mut()
            .entity_mut(label)
            .insert(EllipsisText("Raider".to_string()));
        let after = frames(&mut app, label, 6);
        assert!(
            after.iter().all(|text| text == "Raider"),
            "the new text shows whole: {after:?}"
        );
        // A surface that writes the same text each frame keeps its cut.
        app.world_mut()
            .entity_mut(label)
            .insert(EllipsisText("Raider 12".to_string()));
        let settled = frames(&mut app, label, 4)[3].clone();
        assert_ne!(settled, "Raider 12");
        for _ in 0..6 {
            app.world_mut()
                .entity_mut(label)
                .insert(EllipsisText("Raider 12".to_string()));
            app.update();
            assert_eq!(shown(&app, label), settled);
        }
    }

    /// The system on a real layout: every text settles within four frames on
    /// one text that fits its cell and ends at a glyph start, and a widened
    /// cell shows it whole.
    #[test]
    fn a_laid_out_label_settles_on_one_text_and_fits_its_cell() {
        let texts = [
            ("ascii", "Long-range capacitor bank assembly"),
            (
                "cjk past the cut",
                "Capacitor assembly \u{6771}\u{4eac}\u{6771}\u{4eac}",
            ),
            (
                "cjk before the cut",
                "\u{6771}\u{4eac}\u{6771}\u{4eac} Capacitor assembly",
            ),
            (
                "zwj and mark",
                "Pod bank \u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467} e\u{301}e\u{301}x tail",
            ),
            (
                "flags",
                "Pod ban\u{1F1E9}\u{1F1EA}\u{1F1EB}\u{1F1F7} tail text",
            ),
        ];
        for (name, full) in texts {
            // Where a glyph starts in the whole text.
            let (mut whole, _cell, label) = rig(full, 2000.0);
            frames(&mut whole, label, 3);
            let starts: Vec<usize> = measured_edges(
                whole
                    .world()
                    .get::<ComputedTextBlock>(label)
                    .expect("a block"),
            )
            .iter()
            .map(|edge| edge.byte)
            .collect();
            assert_eq!(shown(&whole, label), full, "{name}: fits whole in 2000 px");

            for cell_px in [30.0, 52.0, 71.5, 84.0, 97.0, 110.0] {
                let (mut app, cell, label) = rig(full, cell_px);
                let log = frames(&mut app, label, 12);
                let settled = log[11].clone();
                assert!(
                    log[4..].iter().all(|text| *text == settled),
                    "{name} in {cell_px} px: {log:?}"
                );
                // Taffy rounds the cell to whole px.
                let cell_width = app
                    .world()
                    .get::<ComputedNode>(cell)
                    .expect("a cell")
                    .content_box()
                    .size()
                    .x;
                assert!(
                    laid_out_width(&app, label) <= cell_width || settled == ELLIPSIS,
                    "{name} in {cell_px} px ({cell_width} laid out): {settled:?} lays out {} px",
                    laid_out_width(&app, label)
                );
                let mut left: Vec<&String> = Vec::new();
                for text in &log {
                    if left.last() != Some(&text) {
                        assert!(!left.contains(&text), "{name} in {cell_px} px: {log:?}");
                        left.push(text);
                    }
                }
                if settled != full {
                    let kept = settled.strip_suffix(ELLIPSIS).expect("a cut ends in ...");
                    assert!(
                        full.starts_with(kept) && starts.contains(&kept.len()),
                        "{name} in {cell_px} px: {settled:?} ends inside a glyph"
                    );
                }
                // Widening shows the whole text; narrowing again cuts the
                // same way.
                set_cell_width(&mut app, cell, 2000.0);
                assert_eq!(frames(&mut app, label, 6)[5], full, "{name}");
                set_cell_width(&mut app, cell, cell_px);
                assert_eq!(
                    frames(&mut app, label, 8)[7],
                    settled,
                    "{name} in {cell_px} px"
                );
            }
        }
    }
}
