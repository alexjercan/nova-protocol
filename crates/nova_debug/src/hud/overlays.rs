//! The debug overlays, one switch each.
//!
//! [`DebugOverlays`] is the one record of which dev overlay is drawn. Each
//! field owns one layer and nothing else, so turning one on never raises a
//! second. [`apply_debug_overlays`] writes a change to the layer's own switch:
//! a gizmo group's `enabled`, the wireframe pass, avian's diagnostics panel,
//! or `nova_hud`'s ammo number. The draw systems of the Nova gizmo groups also
//! run only while their flag is on, so a hidden overlay costs no queries.
//!
//! Every overlay boots off, and a switch stays as set after the Nova Debug
//! window closes. A missing gizmo group is a wiring fault and panics in
//! [`GizmoConfigStore::config_mut`] on the first apply.

use avian3d::prelude::{PhysicsDiagnosticsUiSettings, PhysicsGizmos};
use bevy::{pbr::wireframe::WireframeConfig, prelude::*};
use bevy_inspector_egui::egui;
#[cfg(feature = "debug")]
use nova_hud::prelude::AmmoReadoutDebug;

/// Which dev overlays are drawn. All off by default.
#[derive(Resource, Reflect, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[reflect(Resource)]
pub struct DebugOverlays {
    /// Avian's collider and contact gizmos and its diagnostics panel.
    pub physics: bool,
    /// The global wireframe pass over every mesh.
    pub wireframe: bool,
    /// Gravity wells and each body's link to its dominant well.
    pub gravity: bool,
    /// Turret barrels, rounds, drives and torpedo spawners.
    pub sections: bool,
    /// The exact `rounds/capacity` under each ammo gauge. Only a `debug`
    /// build has the number; elsewhere the flag draws nothing.
    pub ammo_number: bool,
}

/// The gizmo group the gravity overlay draws in.
#[derive(Clone, Default, Reflect, GizmoConfigGroup)]
pub struct GravityGizmos;

/// The gizmo group the ship-section overlay draws in.
#[derive(Clone, Default, Reflect, GizmoConfigGroup)]
pub struct SectionGizmos;

/// Run condition: the overlay `pick` selects is on.
pub(crate) fn overlay_on(
    pick: fn(&DebugOverlays) -> bool,
) -> impl Fn(Res<DebugOverlays>) -> bool + Clone {
    move |overlays: Res<DebugOverlays>| pick(&overlays)
}

/// Write each flag of [`DebugOverlays`] to the switch of its own layer.
///
/// Runs when the record changes, and on its first frame, so the boot state
/// (all off) reaches avian's physics gizmos, which start on.
pub fn apply_debug_overlays(
    overlays: Res<DebugOverlays>,
    mut store: ResMut<GizmoConfigStore>,
    mut wireframe: ResMut<WireframeConfig>,
    mut physics_ui: ResMut<PhysicsDiagnosticsUiSettings>,
    #[cfg(feature = "debug")] mut ammo_number: ResMut<AmmoReadoutDebug>,
) {
    store.config_mut::<PhysicsGizmos>().0.enabled = overlays.physics;
    physics_ui.enabled = overlays.physics;
    wireframe.global = overlays.wireframe;
    store.config_mut::<GravityGizmos>().0.enabled = overlays.gravity;
    store.config_mut::<SectionGizmos>().0.enabled = overlays.sections;
    #[cfg(feature = "debug")]
    ammo_number.set_if_neq(AmmoReadoutDebug(overlays.ammo_number));
}

/// One checkbox per overlay. A tick writes the record only when it changed,
/// so an idle window does not re-apply every frame.
pub(crate) fn overlays_ui(world: &mut World, ui: &mut egui::Ui) {
    let Some(current) = world.get_resource::<DebugOverlays>().copied() else {
        ui.label("this app has no debug overlays");
        return;
    };
    let mut wanted = current;
    ui.checkbox(
        &mut wanted.physics,
        "Physics: colliders, contacts, diagnostics",
    );
    ui.checkbox(&mut wanted.wireframe, "Wireframe: every mesh");
    ui.checkbox(&mut wanted.gravity, "Gravity: wells and dominant links");
    ui.checkbox(&mut wanted.sections, "Sections: barrels, rounds, drives");
    ui.checkbox(&mut wanted.ammo_number, "Ammo number: rounds/capacity");
    if wanted != current {
        *world.resource_mut::<DebugOverlays>() = wanted;
    }
}

/// Register the record and its apply system.
pub(crate) fn build(app: &mut App) {
    app.init_resource::<DebugOverlays>();
    app.register_type::<DebugOverlays>();
    #[cfg(feature = "debug")]
    app.init_resource::<AmmoReadoutDebug>();
    app.add_systems(
        Update,
        apply_debug_overlays.run_if(resource_changed::<DebugOverlays>),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An app with every layer switch the apply system writes, each one on,
    /// so a flag that fails to reach its switch reads as on.
    fn rig() -> App {
        let mut app = App::new();
        let mut store = GizmoConfigStore::default();
        store.insert(GizmoConfig::default(), PhysicsGizmos::default());
        store.insert(GizmoConfig::default(), GravityGizmos);
        store.insert(GizmoConfig::default(), SectionGizmos);
        app.insert_resource(store);
        app.insert_resource(WireframeConfig {
            global: true,
            ..default()
        });
        app.insert_resource(PhysicsDiagnosticsUiSettings {
            enabled: true,
            ..default()
        });
        #[cfg(feature = "debug")]
        app.insert_resource(AmmoReadoutDebug(true));
        build(&mut app);
        app
    }

    /// What each layer's switch reads, in [`DebugOverlays`] field order.
    fn layers(app: &App) -> [bool; 5] {
        let world = app.world();
        let store = world.resource::<GizmoConfigStore>();
        let physics = store.config::<PhysicsGizmos>().0.enabled;
        assert_eq!(
            physics,
            world.resource::<PhysicsDiagnosticsUiSettings>().enabled,
            "the physics gizmos and the physics panel are one overlay"
        );
        #[cfg(feature = "debug")]
        let ammo_number = world.resource::<AmmoReadoutDebug>().0;
        #[cfg(not(feature = "debug"))]
        let ammo_number = false;
        [
            physics,
            world.resource::<WireframeConfig>().global,
            store.config::<GravityGizmos>().0.enabled,
            store.config::<SectionGizmos>().0.enabled,
            ammo_number,
        ]
    }

    /// Every overlay boots off, and each flag switches its own layer and no
    /// other.
    #[test]
    fn each_overlay_flag_switches_only_its_own_layer() {
        let mut app = rig();
        app.update();
        assert_eq!(layers(&app), [false; 5], "every overlay boots off");

        let flags: [fn(&mut DebugOverlays) -> &mut bool; 5] = [
            |o| &mut o.physics,
            |o| &mut o.wireframe,
            |o| &mut o.gravity,
            |o| &mut o.sections,
            |o| &mut o.ammo_number,
        ];
        for (at, flag) in flags.into_iter().enumerate() {
            let mut overlays = DebugOverlays::default();
            *flag(&mut overlays) = true;
            app.insert_resource(overlays);
            app.update();
            let mut wanted = [false; 5];
            // The ammo number only exists in a `debug` build.
            wanted[at] = cfg!(feature = "debug") || at != 4;
            assert_eq!(layers(&app), wanted, "flag {at} alone");
        }
    }

    /// `hide_dev_overlays` clears the ammo number with the rest. It used to
    /// clear the three nova_debug flags only, so a capture taken after F11
    /// kept the number in shot.
    #[cfg(feature = "debug")]
    #[test]
    fn hide_dev_overlays_clears_the_ammo_number() {
        let mut app = rig();
        app.insert_resource(DebugOverlays {
            ammo_number: true,
            gravity: true,
            ..default()
        });
        app.update();
        assert!(app.world().resource::<AmmoReadoutDebug>().0);

        crate::harness::hide_dev_overlays(app.world_mut());
        app.update();
        assert_eq!(layers(&app), [false; 5]);
    }
}
