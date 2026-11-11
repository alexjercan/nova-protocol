//! The HUD line for a sector crossing: `SECTOR (x, y, z)` on the notice slot
//! the frame the open world's observer leaves one cell for another, with
//! `- FIRST VISIT` for a cell the player's chart has not recorded. The cell
//! is written as the chart view labels it. No sound of its own: the notice
//! slot plays the interface cue on a new line.
//!
//! Here, and not in `nova_hud` or `nova_world_base`: the crossing is a
//! `nova_world` message and the line is a `nova_hud` message, and this is the
//! lowest crate that names both.

use bevy::prelude::*;
use nova_gameplay::prelude::Chart;
use nova_hud::prelude::HudNotice;
use nova_world::prelude::{NovaWorldSystems, SectorCrossed};
use nova_world_base::prelude::NovaWorldBaseSystems;

/// Registers [`SectorCrossed`] and [`HudNotice`] (both no-ops beside their
/// owners, so the poster stands alone) and runs
/// [`post_sector_crossing_notice`] in `Update` after
/// [`NovaWorldSystems::Observe`], which posts the crossing, and before
/// [`NovaWorldBaseSystems::Chart`], which records the entered cell: the
/// `FIRST VISIT` read must see the chart as it stood before this frame's
/// crossing.
pub struct SectorNoticePlugin;

impl Plugin for SectorNoticePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SectorCrossed>();
        app.add_message::<HudNotice>();
        app.init_resource::<Chart>();
        app.add_systems(
            Update,
            post_sector_crossing_notice
                .after(NovaWorldSystems::Observe)
                .before(NovaWorldBaseSystems::Chart),
        );
    }
}

/// The notice line for a crossing into `to`, `first_visit` when the chart
/// had not recorded that cell.
pub fn sector_notice_line(to: nova_events::prelude::SectorCoord, first_visit: bool) -> String {
    if first_visit {
        format!("SECTOR {to} - FIRST VISIT")
    } else {
        format!("SECTOR {to}")
    }
}

/// Post one notice per crossing this frame.
pub fn post_sector_crossing_notice(
    mut crossings: MessageReader<SectorCrossed>,
    chart: Res<Chart>,
    mut notices: MessageWriter<HudNotice>,
) {
    for crossing in crossings.read() {
        let line = sector_notice_line(crossing.to, !chart.has_visited(crossing.to));
        debug!("hud_notice: {line} (from {})", crossing.from);
        notices.write(HudNotice(line));
    }
}

#[cfg(test)]
mod tests {
    use nova_events::prelude::SectorCoord;

    use super::*;

    /// The crossing poster alone, with the chart empty.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(SectorNoticePlugin);
        app.update();
        app
    }

    fn cross(app: &mut App, from: SectorCoord, to: SectorCoord) {
        app.world_mut().write_message(SectorCrossed { from, to });
    }

    /// The notices posted this update.
    fn notices(app: &App) -> Vec<String> {
        app.world()
            .resource::<Messages<HudNotice>>()
            .iter_current_update_messages()
            .map(|notice| notice.0.clone())
            .collect()
    }

    #[test]
    fn a_crossing_into_an_uncharted_cell_posts_first_visit() {
        let mut app = app();
        cross(&mut app, SectorCoord::ORIGIN, SectorCoord::new(1, 0, 0));
        app.update();
        assert_eq!(notices(&app), ["SECTOR (1, 0, 0) - FIRST VISIT"]);
    }

    #[test]
    fn a_crossing_into_a_charted_cell_posts_the_cell_alone() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<Chart>()
            .visit(SectorCoord::new(1, 0, 0), Vec::new(), 0.0);
        cross(&mut app, SectorCoord::ORIGIN, SectorCoord::new(1, 0, 0));
        app.update();
        assert_eq!(notices(&app), ["SECTOR (1, 0, 0)"]);
    }

    #[test]
    fn a_quiet_frame_posts_nothing() {
        let mut app = app();
        app.update();
        assert!(notices(&app).is_empty());
    }

    /// The chart records the entered cell in [`NovaWorldBaseSystems::Chart`]
    /// on the crossing frame when its root is live; the poster reads the
    /// chart before that.
    #[test]
    fn the_poster_reads_the_chart_before_the_crossing_is_recorded() {
        let mut app = app();
        app.add_systems(
            Update,
            (|mut chart: ResMut<Chart>| {
                chart.visit(SectorCoord::new(1, 0, 0), Vec::new(), 0.0);
            })
            .in_set(NovaWorldBaseSystems::Chart),
        );
        cross(&mut app, SectorCoord::ORIGIN, SectorCoord::new(1, 0, 0));
        app.update();
        assert_eq!(notices(&app), ["SECTOR (1, 0, 0) - FIRST VISIT"]);
        assert!(app
            .world()
            .resource::<Chart>()
            .has_visited(SectorCoord::new(1, 0, 0)));
    }
}
