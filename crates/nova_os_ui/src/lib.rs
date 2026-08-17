//! The NOVA OS cockpit monitor: the in-world CRT terminal the player opens with
//! Tab, and the apps that run on it.
//!
//! This is a terminal runtime with a screen, not a HUD widget. The flight HUD
//! (`nova_gameplay::hud`) draws instruments over the world and hides while this
//! monitor is open; this crate draws a physical monitor into an offscreen image,
//! composites it through a CRT shader, and forwards pointer input back into it.
//! The two share only the `PauseStates::NovaOs` axis and the
//! `HudNovaOsExempt` tag, so they sit in separate crates: nothing in the HUD
//! reaches in here.
//!
//! The pure terminal model this renders - the shell grammar, the command
//! registry and the app runtime - lives in `nova_os` and has no Bevy UI in it.
//!
//! # Crate layout
//!
//! | Module | Concern |
//! | --- | --- |
//! | `terminal` | The monitor itself: casing, CRT, shell, input, sound. |
//! | `map` | The `map` app: a schematic 3D minimap of local space. |
//! | `ship` | The `ship` app: a schematic 3D viewer of the player ship. |

pub mod map;
pub mod ship;
pub mod terminal;

/// The pure terminal model this crate renders. Re-exported because the two are
/// one API to a consumer: asking what the shell is showing, or registering a
/// command on it, means naming `nova_os` types, and whoever already depends on
/// the renderer should not have to name the model crate a second time.
pub use nova_os;

/// Live-tree rig for the forwarded CRT pointer, shared by the CRT mapping and
/// the map/ship blip click tests.
#[cfg(test)]
mod pointer_rig;

/// Glob-import surface: `use nova_os_ui::prelude::*`.
pub mod prelude {
    pub use super::{map::prelude::*, ship::prelude::*, terminal::prelude::*, NovaOsUiPlugin};
}

use bevy::prelude::*;
use nova_hud::NovaHudSystems;

use crate::{map::NovaOsMapSystems, ship::NovaOsShipSystems, terminal::NovaOsSystems};

/// Wires the whole NOVA OS monitor: the terminal shell plus every app that runs
/// on it.
///
/// Added by the assembly crate (`nova_core`), not by the HUD - the monitor and
/// the flight HUD are peers, and the plugin that orders them belongs above both.
/// Render-gated: a headless run has no monitor.
pub struct NovaOsUiPlugin;

impl Plugin for NovaOsUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MonitorFrame);
        // The shell first: both apps register into the command and app
        // registries it creates.
        app.add_plugins(terminal::NovaOsPlugin);
        app.add_plugins(map::NovaOsMapPlugin);
        app.add_plugins(ship::NovaOsShipPlugin);
    }
}

/// The order the monitor's four sets run in, and nothing else.
///
/// It lives at the composition root rather than in the three plugins because
/// every edge here crosses between them: which app peeks the shared pending
/// invocation first, and whether an app's answer reaches the paint on the frame
/// it was typed. Split out from [`NovaOsUiPlugin`] so the contract can be
/// asserted without standing up the CRT material and the render stack behind it.
pub(crate) struct MonitorFrame;

impl Plugin for MonitorFrame {
    fn build(&self, app: &mut App) {
        // Input/Simulate/Paint sit inside the HUD's slot so the monitor keeps
        // its position relative to the rest of gameplay; Toggle stays outside it
        // (see `NovaOsSystems::Toggle`).
        app.configure_sets(
            Update,
            (
                NovaOsSystems::Input,
                NovaOsSystems::Simulate,
                NovaOsSystems::Paint,
            )
                .chain()
                .in_set(NovaHudSystems),
        );
        app.configure_sets(Update, NovaOsSystems::Toggle.before(NovaOsSystems::Input));

        // F53: both app sets were declared and never ordered, so whether a
        // `ship repair` or `map goto` answer reached the screen this frame or
        // the next was bevy's topological tie-break. Each app applies its
        // invocation after the terminal produced it and before the terminal
        // paints. `map` before `ship` because the two share ONE pending
        // invocation slot: a fixed order makes which handler peeks first a
        // decision instead of an accident.
        app.configure_sets(
            Update,
            NovaOsShipSystems
                .after(NovaOsSystems::Input)
                .before(NovaOsSystems::Paint),
        );
        app.configure_sets(
            Update,
            NovaOsMapSystems
                .after(NovaOsSystems::Input)
                .before(NovaOsSystems::Paint)
                .before(NovaOsShipSystems),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The monitor frame is a scheduling fact, not a topological accident: the
    /// terminal takes input, then each app applies the invocation it owns, then
    /// the terminal paints - so a `ship repair` result row lands on the frame it
    /// was typed. `map` before `ship` because the two share one pending slot.
    ///
    /// This is what F53 was: both app sets were declared and never ordered.
    #[test]
    fn the_monitor_frame_runs_input_then_map_then_ship_then_paint() {
        #[derive(Resource, Default)]
        struct Order(Vec<&'static str>);

        let mut app = App::new();
        app.init_resource::<Order>();
        app.add_plugins(MonitorFrame);
        app.add_systems(
            Update,
            (
                (|mut order: ResMut<Order>| order.0.push("input")).in_set(NovaOsSystems::Input),
                (|mut order: ResMut<Order>| order.0.push("map")).in_set(NovaOsMapSystems),
                (|mut order: ResMut<Order>| order.0.push("ship")).in_set(NovaOsShipSystems),
                (|mut order: ResMut<Order>| order.0.push("paint")).in_set(NovaOsSystems::Paint),
                (|mut order: ResMut<Order>| order.0.push("toggle")).in_set(NovaOsSystems::Toggle),
            ),
        );

        app.update();

        assert_eq!(
            app.world().resource::<Order>().0,
            ["toggle", "input", "map", "ship", "paint"]
        );
    }
}
