//! The HUD readout strip: a generic surface for showing a SCENARIO VARIABLE on
//! the HUD (task 20260716-174729) - the modding-surface piece the gauntlet
//! time-trial needs and any mod can reuse. It is the display half of the
//! scenario-variable vocabulary: `scenario_elapsed` (and any authored variable)
//! already exists on the event world; nothing put one on screen until now.
//!
//! Data path mirrors the comms panel exactly. A scenario's `HudReadout` action
//! upserts a named readout on the event world (nova_scenario), whose sync copies
//! the active readouts - each with its CURRENT variable value read that frame -
//! into [`HudReadouts`] here (nova_gameplay). The strip reconciles one row per
//! active readout and formats the value ([`HudReadoutFormat`]): `Time` renders
//! `mm:ss.s`, `Number` one decimal, `Integer` none.
//!
//! The strip is an [`HudTier::Instrument`] widget (a flight readout, shown with
//! the velocity/speed chips even at the Minimal HUD level), positioned
//! top-center. It freezes automatically on pause and behind the outcome overlay:
//! `scenario_elapsed` freezes there (the clock stops ticking), so the last
//! synced value simply holds - a time-trial's FINAL time stays on screen,
//! frozen, through the Victory banner with no extra machinery.
//!
//! Scenario teardown clears the event world; the sync writes an empty
//! [`HudReadouts`]; the strip despawns every row instantly - the same reset
//! class as the comms panel and objectives, so a readout cannot leak into the
//! next scenario or the menu.

use bevy::prelude::*;
use nova_ui::theme;

use super::HudTier;

/// Glob-import surface: `use nova_gameplay::hud::readout::prelude::*`
/// re-exports the public API of this module. `HudReadoutFormat` is deliberately
/// NOT re-exported here: nova_scenario's prelude exports a same-named authoring
/// enum, and nova_core globs both preludes, so re-exporting it from both is an
/// ambiguous glob re-export (task 20260721-151934). It stays `pub` and is
/// reached by its full path from the sync in nova_scenario's world.rs.
pub mod prelude {
    pub use super::{HudReadoutEntry, HudReadouts};
}

/// How a [`HudReadoutEntry`] value renders as text. The scenario-side
/// `HudReadoutFormat` maps onto this, the same nova_scenario -> nova_gameplay
/// split as `StoryMessageActionConfig` -> `StoryLine` (the HUD cannot depend on
/// nova_scenario).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudReadoutFormat {
    /// One decimal place, e.g. `12.3`.
    Number,
    /// No decimals (rounded), e.g. `12`.
    Integer,
    /// Minutes and seconds, `mm:ss.s`, e.g. `01:23.4` - the time-trial clock.
    Time,
}

impl HudReadoutFormat {
    /// Render `value` in this format.
    pub fn render(self, value: f64) -> String {
        match self {
            HudReadoutFormat::Number => format!("{value:.1}"),
            HudReadoutFormat::Integer => format!("{:.0}", value.round()),
            HudReadoutFormat::Time => {
                // mm:ss.s. Clamp negatives to zero so a stray negative never
                // prints a minus that reads as garbage on a clock.
                let value = value.max(0.0);
                let minutes = (value / 60.0).floor() as u64;
                let seconds = value - (minutes as f64) * 60.0;
                format!("{minutes:02}:{seconds:04.1}")
            }
        }
    }
}

/// One active HUD readout: a named slot showing a variable's current value in a
/// chosen format, with an optional label. Delivered to the HUD by
/// nova_scenario's event-world sync (rebuilt every frame so the value tracks the
/// live variable); the strip renders one row per entry.
#[derive(Clone, Debug, PartialEq)]
pub struct HudReadoutEntry {
    /// The readout's stable id, so a scenario can update or clear one slot
    /// without disturbing the others; also the row's reconciliation key.
    pub slot: String,
    /// Optional caption shown before the value, e.g. `TIME`.
    pub label: Option<String>,
    /// How [`value`](Self::value) renders.
    pub format: HudReadoutFormat,
    /// The variable's value this frame (already read off the event world).
    pub value: f64,
}

/// The scenario's active HUD readouts, in authored order. Written by
/// nova_scenario's event-world sync (rebuilt each frame with fresh variable
/// values, emptied on teardown); the readout strip renders them. Lives in
/// nova_gameplay because the HUD cannot depend on nova_scenario (the dependency
/// points the other way) - the same split as `StoryFeed`/`GameObjectives`.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct HudReadouts(pub Vec<HudReadoutEntry>);

/// Readout font size (px), a touch larger than comms body text so the clock
/// reads at a glance.
const READOUT_FONT_SIZE_PX: f32 = 18.0;

/// The strip container (top-center).
#[derive(Component)]
struct HudReadoutStripMarker;

/// One readout row, keyed by its [`HudReadoutEntry::slot`] for reconciliation.
#[derive(Component)]
struct HudReadoutRow {
    slot: String,
}

/// Drives the HUD readout strip: inits [`HudReadouts`], spawns the (empty)
/// top-center strip in Startup, and reconciles one row per active readout each
/// frame within [`super::NovaHudSystems`].
pub struct HudReadoutPlugin;

impl Plugin for HudReadoutPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudReadouts>();
        app.add_systems(Startup, spawn_readout_strip);
        app.add_systems(
            Update,
            sync_readout_rows.in_set(super::NovaHudSystems),
        );
    }
}

/// The strip: a top-center column, empty until a scenario shows a readout. It is
/// always spawned (like the comms panel) so the reconcile has a stable parent to
/// grow rows under; the container itself has no visible chrome, so an empty
/// strip draws nothing.
fn spawn_readout_strip(mut commands: Commands) {
    commands.spawn((
        Name::new("HudReadoutStrip"),
        HudReadoutStripMarker,
        HudTier::Instrument,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(16.0),
            left: Val::Percent(50.0),
            // Center the column on the screen's horizontal midpoint.
            margin: UiRect {
                left: Val::Px(-80.0),
                ..default()
            },
            width: Val::Px(160.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(4.0),
            ..default()
        },
    ));
}

/// Reconcile the strip's rows against [`HudReadouts`]: add a row for a new
/// slot, drop a row whose slot is gone (teardown empties the whole set), and
/// update the text of a surviving row in place - no per-frame despawn/respawn
/// even though the value changes every frame (the row entity persists; only its
/// `Text` is rewritten).
fn sync_readout_rows(
    readouts: Res<HudReadouts>,
    mut commands: Commands,
    strip: Query<Entity, With<HudReadoutStripMarker>>,
    mut rows: Query<(Entity, &HudReadoutRow, &mut Text)>,
) {
    let Ok(strip) = strip.single() else {
        return;
    };

    // Drop rows whose slot is no longer active (covers teardown wholesale).
    for (entity, row, _) in &rows {
        if !readouts.0.iter().any(|r| r.slot == row.slot) {
            commands.entity(entity).despawn();
        }
    }

    for entry in &readouts.0 {
        let text = format_readout(entry);
        // Update in place if the row already exists.
        if let Some((_, _, mut existing)) = rows
            .iter_mut()
            .find(|(_, row, _)| row.slot == entry.slot)
        {
            if existing.0 != text {
                existing.0 = text;
            }
            continue;
        }
        // Otherwise grow a new row under the strip.
        commands.entity(strip).with_children(|parent| {
            parent.spawn((
                Name::new(format!("HudReadoutRow({})", entry.slot)),
                HudReadoutRow {
                    slot: entry.slot.clone(),
                },
                Text::new(text),
                TextFont::from_font_size(READOUT_FONT_SIZE_PX),
                TextColor(theme::TEXT),
            ));
        });
    }
}

/// Render a readout row's text: `LABEL value` (label upper-cased, matching the
/// comms speaker styling) or just the value when no label is authored.
fn format_readout(entry: &HudReadoutEntry) -> String {
    let value = entry.format.render(entry.value);
    match &entry.label {
        Some(label) => format!("{} {}", label.to_uppercase(), value),
        None => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three formats render as documented: one-decimal Number, rounded
    /// Integer, and mm:ss.s Time (the time-trial clock).
    #[test]
    fn formats_render_as_documented() {
        assert_eq!(HudReadoutFormat::Number.render(12.34), "12.3");
        assert_eq!(HudReadoutFormat::Integer.render(12.6), "13");
        assert_eq!(HudReadoutFormat::Time.render(83.44), "01:23.4");
        assert_eq!(HudReadoutFormat::Time.render(0.0), "00:00.0");
        // A minute boundary and a sub-minute value pad correctly.
        assert_eq!(HudReadoutFormat::Time.render(60.0), "01:00.0");
        assert_eq!(HudReadoutFormat::Time.render(5.0), "00:05.0");
        // A stray negative clamps to zero rather than printing a minus.
        assert_eq!(HudReadoutFormat::Time.render(-3.0), "00:00.0");
    }

    /// A labelled row upper-cases the label; an unlabelled row is bare value.
    #[test]
    fn label_is_upper_cased_and_optional() {
        let labelled = HudReadoutEntry {
            slot: "timer".to_string(),
            label: Some("time".to_string()),
            format: HudReadoutFormat::Time,
            value: 83.44,
        };
        assert_eq!(format_readout(&labelled), "TIME 01:23.4");

        let bare = HudReadoutEntry {
            label: None,
            ..labelled
        };
        assert_eq!(format_readout(&bare), "01:23.4");
    }

    fn readout_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<HudReadouts>();
        app.add_systems(Startup, spawn_readout_strip);
        app.add_systems(Update, sync_readout_rows);
        app
    }

    fn row_texts(app: &mut App) -> Vec<String> {
        app.world_mut()
            .query_filtered::<&Text, With<HudReadoutRow>>()
            .iter(app.world())
            .map(|t| t.0.clone())
            .collect()
    }

    /// A shown readout grows a row with the formatted value; updating the
    /// value rewrites the SAME row (no churn); clearing the set (teardown)
    /// drops every row.
    #[test]
    fn rows_reconcile_and_clear_on_empty() {
        let mut app = readout_app();
        app.update();
        assert!(row_texts(&mut app).is_empty(), "no readouts, no rows");

        app.world_mut().resource_mut::<HudReadouts>().0 = vec![HudReadoutEntry {
            slot: "timer".to_string(),
            label: Some("TIME".to_string()),
            format: HudReadoutFormat::Time,
            value: 5.0,
        }];
        app.update();
        assert_eq!(row_texts(&mut app), vec!["TIME 00:05.0"]);

        // Capture the row entity to prove the update is in-place.
        let row_entity = app
            .world_mut()
            .query_filtered::<Entity, With<HudReadoutRow>>()
            .single(app.world())
            .expect("one row");

        app.world_mut().resource_mut::<HudReadouts>().0[0].value = 12.3;
        app.update();
        assert_eq!(row_texts(&mut app), vec!["TIME 00:12.3"]);
        let still = app
            .world_mut()
            .query_filtered::<Entity, With<HudReadoutRow>>()
            .single(app.world())
            .expect("still one row");
        assert_eq!(row_entity, still, "the value update reused the same row");

        // Teardown syncs an empty set: every row drops.
        app.world_mut().resource_mut::<HudReadouts>().0.clear();
        app.update();
        assert!(
            row_texts(&mut app).is_empty(),
            "an emptied readout set drops every row"
        );
    }
}
