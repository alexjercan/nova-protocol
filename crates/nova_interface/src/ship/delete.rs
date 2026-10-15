//! The Ship pane's Delete action: take a selected section off the player
//! ship for good while it is docked at a station that refits.
//!
//! One rule, [`plan_ship_delete`], answers both the panel and the act: the
//! dock must offer Refit, the section must not be the port the dock holds
//! by, [`remove_build_fault`] must find no new fault, and the parts the strip
//! rule gives back ([`plan_section_yield`] at the section's health share)
//! must fit in the hold. [`update_ship_delete_verdict`] runs it for the
//! selection every frame: the panel's Delete button names the yield while it
//! holds, and is disabled with the reason in its tooltip while it refuses.
//!
//! The `ship_delete` key and the enabled button raise a
//! [`SectionDeleteCommand`]; a refused selection raises none, and so shows no
//! note. [`apply_ship_delete_commands`] plans it again: the parts go in the
//! hold, one `Stripped` report names the player ship as the hull, the note
//! line names the parts, and the [`SectionRemoveRequest`] takes the section
//! off and re-clads the hull round the hole.
//!
//! Touch this module when changing what Delete gives back or refuses.

use std::collections::BTreeMap;

use bevy::{prelude::*, ui_widgets::Activate};
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use super::{
    rebuild::{ResolvedSite, StationSite},
    refit::build_fault_text,
    scene::ShipRuntime,
    sections::design_section_code,
};
use crate::pane::play_menu_select;

/// A request to delete one live section of the player ship, raised by the
/// `ship_delete` key or the panel's Delete button.
#[derive(Message, Clone, Copy, Debug)]
pub struct SectionDeleteCommand {
    /// The live section, a child of the player ship's root.
    pub target: Entity,
}

/// Why a delete is refused, in check order.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SectionDeleteRefusalType {
    /// The ship is not docked.
    NotDocked,
    /// The dock does not offer Refit.
    NoRefitService,
    /// The section is the port the dock holds by.
    DockedPort,
    /// The ship left would fail a post-build check it passed before.
    Build(RefitBuildFaultType),
    /// The parts weigh more than the hold has room for.
    Hold(HullStripPauseType),
}

/// What a planned delete takes off and gives back.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SectionDelete {
    /// The design section id.
    pub(crate) section: String,
    /// The parts the strip rule gives back.
    pub(crate) parts: BTreeMap<ItemDesignId, u32>,
}

impl ResolvedSite<'_> {
    /// Plan deleting live section `target` at health share `health_share`
    /// against the hold `own`; `None` when `target` is not a live section of
    /// this ship.
    pub(crate) fn plan_delete(
        &self,
        items: &GameItems,
        target: Entity,
        health_share: f32,
        own: &ShipInventory,
    ) -> Option<Result<SectionDelete, SectionDeleteRefusalType>> {
        let index = self.live.iter().position(|live| live.entity == target)?;
        let section = self.design.get(&self.live[index].id)?;
        Some(self.plan_delete_at(items, index, section, health_share, own))
    }

    fn plan_delete_at(
        &self,
        items: &GameItems,
        index: usize,
        section: &DesignSection,
        health_share: f32,
        own: &ShipInventory,
    ) -> Result<SectionDelete, SectionDeleteRefusalType> {
        match self.partner {
            None => return Err(SectionDeleteRefusalType::NotDocked),
            Some((_, false)) => return Err(SectionDeleteRefusalType::NoRefitService),
            Some((_, true)) => {}
        }
        if self.port == Some(self.live[index].entity) {
            return Err(SectionDeleteRefusalType::DockedPort);
        }
        if let Some(fault) = remove_build_fault(&self.placed(), index) {
            return Err(SectionDeleteRefusalType::Build(fault));
        }
        let target = StripTarget {
            recipe: &section.config.base.build,
            health_share,
        };
        let parts =
            plan_section_yield(items, target, own).map_err(SectionDeleteRefusalType::Hold)?;
        Ok(SectionDelete {
            section: section.id.clone(),
            parts,
        })
    }
}

/// The tooltip of the disabled Delete button for a refused delete of
/// section `code`, the part named `name`.
pub(crate) fn delete_refusal_text(
    code: &str,
    name: &str,
    refusal: &SectionDeleteRefusalType,
) -> String {
    match refusal {
        SectionDeleteRefusalType::NotDocked => "delete: dock at a station that refits".to_string(),
        SectionDeleteRefusalType::NoRefitService => "delete: this dock offers no refit".to_string(),
        SectionDeleteRefusalType::DockedPort => {
            format!("delete: the dock holds the ship by {code}")
        }
        SectionDeleteRefusalType::Build(RefitBuildFaultType::Disconnected) => {
            format!("delete: taking {code} off would split the ship")
        }
        SectionDeleteRefusalType::Build(RefitBuildFaultType::InvalidSockets) => {
            format!("delete: the sections round {code} would not mate without it")
        }
        SectionDeleteRefusalType::Build(fault) => build_fault_text("delete", name, code, fault),
        SectionDeleteRefusalType::Hold(HullStripPauseType::HoldFull { need_g, free_g }) => {
            format!(
                "delete: {code} gives {} of parts, the hold has {} free",
                kg_text(*need_g),
                kg_text(u64::from(*free_g))
            )
        }
    }
}

/// The health share a delete gives parts back at: current over max, or
/// whole for a section with no pool.
fn health_share(health: Option<&Health>) -> f32 {
    match health {
        Some(health) if health.max > 0.0 => (health.current / health.max).clamp(0.0, 1.0),
        _ => 1.0,
    }
}

/// The parts a delete gives back, as the Delete button and the note name
/// them: `+1 Frame spar, +3 Section panel`, or `nothing to salvage`.
pub(crate) fn delete_yield_text(items: &GameItems, parts: &BTreeMap<ItemDesignId, u32>) -> String {
    if parts.values().all(|count| *count == 0) {
        return "nothing to salvage".to_string();
    }
    parts
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(item, count)| format!("+{count} {}", items.design(item).name))
        .collect::<Vec<_>>()
        .join(", ")
}

/// What deleting live section `target` would do now: its design code and
/// the planned delete, or the reason line it is refused with. `None` when
/// `target` is not a live section of this ship. The one rule the panel's
/// Delete button and [`apply_ship_delete_commands`] both read.
pub(crate) fn plan_ship_delete(
    resolved: &ResolvedSite<'_>,
    items: &GameItems,
    target: Entity,
    health: Option<&Health>,
    own: &ShipInventory,
) -> Option<(String, Result<SectionDelete, String>)> {
    let planned = resolved.plan_delete(items, target, health_share(health), own)?;
    let live = resolved.live.iter().find(|live| live.entity == target)?;
    let code = design_section_code(resolved.design, &live.id)?;
    let planned = planned.map_err(|refusal| {
        let name = resolved
            .selected_section(Some(target))
            .map_or(code.clone(), |section| section.config.base.name.clone());
        delete_refusal_text(&code, &name, &refusal)
    });
    Some((code, planned))
}

/// Raise a [`SectionDeleteCommand`] for the selection when the panel's
/// Delete button is clicked while the delete holds. A refused delete
/// disables the button, so this is the backstop for a click in the same
/// frame the verdict turned.
pub(crate) fn on_ship_delete_button(
    _activate: On<Activate>,
    runtime: Res<ShipRuntime>,
    mut deletes: MessageWriter<SectionDeleteCommand>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    let Some(target) = runtime.selected else {
        return;
    };
    if !matches!(runtime.delete_verdict(), Some(Ok(_))) {
        return;
    }
    deletes.write(SectionDeleteCommand { target });
    play_menu_select(&mut commands, bank.as_deref());
}

/// Cache [`plan_ship_delete`] for the selection, for the panel's Delete
/// button, the `ship_delete` key and [`ShipRuntime::delete_verdict`]: the
/// yield text, or the reason. `None` while there is no selection, a part is
/// in hand or the pane is closed: the button hides and the key does nothing.
pub(crate) fn update_ship_delete_verdict(
    mut runtime: ResMut<ShipRuntime>,
    held: Res<HeldPart>,
    items: Option<Res<GameItems>>,
    site: StationSite,
    q_health: Query<&Health, With<SectionMarker>>,
    q_hold: Query<&ShipInventory, With<PlayerSpaceshipMarker>>,
) {
    let verdict = (|| {
        let target = runtime.selected?;
        if !runtime.active || held.0.is_some() {
            return None;
        }
        let resolved = site.resolve(StationServiceType::Refit)?;
        let items = items.as_deref()?;
        let own = q_hold.get(resolved.player).ok()?;
        let (_, planned) =
            plan_ship_delete(&resolved, items, target, q_health.get(target).ok(), own)?;
        Some(planned.map(|delete| delete_yield_text(items, &delete.parts)))
    })();
    if runtime.delete_verdict != verdict {
        runtime.delete_verdict = verdict;
    }
}

/// Apply [`SectionDeleteCommand`]s by [`plan_ship_delete`]: put the parts in
/// the hold, queue one `Stripped` report with the player ship as the hull,
/// raise the [`SectionRemoveRequest`], clear the selection and name the parts
/// on the note line. A refused command changes nothing and shows nothing: the
/// panel already shows the reason. A command for a section no longer live is
/// dropped.
///
/// # Panics
///
/// On a player ship with no scenario id: every scenario ship wears one.
pub(crate) fn apply_ship_delete_commands(
    mut messages: MessageReader<SectionDeleteCommand>,
    mut runtime: ResMut<ShipRuntime>,
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StationSite,
    q_health: Query<&Health, With<SectionMarker>>,
    mut q_player: Query<
        (&EntityId, &mut ShipInventory, &mut ShipInventoryReports),
        With<PlayerSpaceshipMarker>,
    >,
) {
    if messages.is_empty() {
        return;
    }
    let (Some(resolved), Some(items)) = (site.resolve(StationServiceType::Refit), items) else {
        messages.clear();
        return;
    };
    // One delete a frame: the site was read before it, and the section it
    // took stays live until the commands run.
    let mut deleted = false;
    for command in messages.read() {
        if deleted {
            continue;
        }
        let Ok((hull, mut own, mut reports)) = q_player.get_mut(resolved.player) else {
            panic!(
                "delete: the player ship {:?} carries no scenario id or hold",
                resolved.player
            );
        };
        let Some((code, Ok(delete))) = plan_ship_delete(
            &resolved,
            &items,
            command.target,
            q_health.get(command.target).ok(),
            &own,
        ) else {
            continue;
        };
        for (item, &count) in &delete.parts {
            own.add(&items, item, count);
        }
        let text = delete_yield_text(&items, &delete.parts);
        let salvaged = delete.parts.values().any(|count| *count > 0);
        info!("delete: {code} ('{}') gave {text}", delete.section);
        reports.0.push(ShipInventoryReport::Stripped {
            hull: hull.0.clone(),
            section: code.clone(),
            parts: delete.parts,
        });
        commands.trigger(SectionRemoveRequest {
            entity: resolved.player,
            section: command.target,
        });
        if runtime.selected == Some(command.target) {
            runtime.selected = None;
        }
        let note = if salvaged {
            format!("deleted {code}: {text} to the hold")
        } else {
            format!("deleted {code}: {text}")
        };
        runtime.note = Some((note, 2.5));
        deleted = true;
    }
}

#[cfg(test)]
mod tests {
    use nova_gameplay::test_support::test_items;

    use super::*;

    /// The sections `SectionRemoveRequest` asked to take off.
    #[derive(Resource, Default)]
    struct Removed(Vec<Entity>);

    fn cube_points() -> Vec<LinkPoint> {
        [
            ("px", Vec3::X),
            ("nx", Vec3::NEG_X),
            ("py", Vec3::Y),
            ("ny", Vec3::NEG_Y),
            ("pz", Vec3::Z),
            ("nz", Vec3::NEG_Z),
        ]
        .into_iter()
        .map(|(id, normal)| LinkPoint {
            id: id.to_string(),
            position: normal * 0.5,
            normal,
            skin: LinkPointSkinType::Clad,
        })
        .collect()
    }

    /// The player ship: a hull with its docking port above, its computer
    /// below, its drive aft and a gun forward, each a one-cell cube.
    struct Ship {
        player: Entity,
        port: Entity,
        hull: Entity,
        computer: Entity,
        gun: Entity,
    }

    /// The delete app with the player ship holding `capacity_g`, docked port
    /// to port with a living station offering `services`.
    fn docked_app(capacity_g: u32, services: Vec<StationServiceType>) -> (App, Ship) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let items = test_items();
        app.insert_resource(items.clone());
        app.insert_resource(ShipRuntime {
            active: true,
            ..default()
        });
        app.init_resource::<HeldPart>();
        app.init_resource::<Removed>();
        app.add_message::<SectionDeleteCommand>();
        app.add_observer(
            |request: On<SectionRemoveRequest>, mut removed: ResMut<Removed>| {
                removed.0.push(request.section);
            },
        );
        app.add_systems(
            Update,
            (apply_ship_delete_commands, update_ship_delete_verdict).chain(),
        );
        let recipe = |panels: u32, spars: u32| {
            let mut parts = BTreeMap::from([("SectionPanel".into(), panels)]);
            if spars > 0 {
                parts.insert("FrameSpar".into(), spars);
            }
            BuildRecipe {
                credits: 10,
                parts,
                strip_yield: 0.5,
            }
        };
        let design =
            |id: &str, kind: SectionKind, position: Vec3, build: BuildRecipe| DesignSection {
                id: id.to_string(),
                position,
                rotation: Quat::IDENTITY,
                config: SectionConfig {
                    base: BaseSectionConfig {
                        id: id.to_string(),
                        name: id.to_string(),
                        collider: Some(SectionCollider::Cuboid { size: Vec3::ONE }),
                        link_points: cube_points(),
                        build,
                        ..default()
                    },
                    kind,
                },
            };
        let sections = vec![
            design(
                "hull",
                SectionKind::Hull(HullSectionConfig::default()),
                Vec3::ZERO,
                recipe(2, 0),
            ),
            design(
                "port",
                SectionKind::Docking(DockingSectionConfig::default()),
                Vec3::Y,
                recipe(2, 0),
            ),
            design(
                "computer",
                SectionKind::Controller(ControllerSectionConfig {
                    steering_lag: 0.2,
                    max_torque: 1.0,
                    render_mesh: None,
                    render_mesh_transform: None,
                }),
                Vec3::NEG_Y,
                recipe(2, 0),
            ),
            design(
                "drive",
                SectionKind::Thruster(ThrusterSectionConfig {
                    magnitude: 1000.0,
                    ..default()
                }),
                Vec3::NEG_X,
                recipe(2, 0),
            ),
            design(
                "gun",
                SectionKind::Turret(TurretSectionConfig::default()),
                Vec3::X,
                recipe(6, 2),
            ),
        ];
        let world = app.world_mut();
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                EntityId("player".to_string()),
                ShipInventory::new(&items, capacity_g, []),
                ShipInventoryReports::default(),
                ShipDesignSections(sections.clone()),
            ))
            .id();
        let live: Vec<Entity> = sections
            .iter()
            .map(|section| {
                world
                    .spawn((
                        SectionMarker,
                        EntityId(section.id.clone()),
                        Transform::from_translation(section.position),
                        SectionLinkPoints(cube_points()),
                        Health::new(100.0),
                        ChildOf(player),
                    ))
                    .id()
            })
            .collect();
        let station = world
            .spawn((
                SpaceshipRootMarker,
                StationMarker(StationDesignId::from("waystation")),
                StationServices(services),
            ))
            .id();
        let station_port = world.spawn(ChildOf(station)).id();
        let connection = world
            .spawn(DockingConnection {
                first_ship: player,
                first_section: live[1],
                second_ship: station,
                second_section: station_port,
                helm: DockedHelmType::Neutral,
                measurement_fault: false,
            })
            .id();
        world.entity_mut(player).insert(DockedShip {
            connection,
            helm: Quat::IDENTITY,
            drives: false,
        });
        app.update();
        (
            app,
            Ship {
                player,
                hull: live[0],
                port: live[1],
                computer: live[2],
                gun: live[4],
            },
        )
    }

    fn delete(app: &mut App, target: Entity) -> Option<String> {
        app.world_mut().resource_mut::<ShipRuntime>().selected = Some(target);
        app.world_mut()
            .write_message(SectionDeleteCommand { target });
        app.update();
        app.world()
            .resource::<ShipRuntime>()
            .note()
            .map(str::to_string)
    }

    /// The cached verdict for `target` once it is selected.
    fn verdict(app: &mut App, target: Entity) -> Option<Result<String, String>> {
        app.world_mut().resource_mut::<ShipRuntime>().selected = Some(target);
        app.update();
        app.world()
            .resource::<ShipRuntime>()
            .delete_verdict()
            .map(|verdict| verdict.map(str::to_string).map_err(str::to_string))
    }

    fn reports(app: &App, player: Entity) -> &[ShipInventoryReport] {
        &app.world()
            .get::<ShipInventoryReports>(player)
            .expect("the player keeps reports")
            .0
    }

    /// A part in hand takes Delete away: no verdict, so the button hides and
    /// the key and the button do nothing. An empty hand gives it back.
    #[test]
    fn a_part_in_hand_takes_delete_away() {
        let (mut app, ship) = docked_app(1_000_000, vec![StationServiceType::Refit]);
        app.insert_resource(HeldPart(Some("pdc".to_string())));
        assert_eq!(verdict(&mut app, ship.gun), None);
        app.insert_resource(HeldPart(None));
        assert!(matches!(verdict(&mut app, ship.gun), Some(Ok(_))));
    }

    /// A docked delete of a leaf gun names its strip yield before the press,
    /// puts it in the hold, names it on the note line, reports it as stripped
    /// off the player's own hull, asks for its removal and clears the
    /// selection. A part that gives nothing back says so.
    #[test]
    fn a_deleted_section_puts_its_strip_yield_in_the_hold_and_reports_it_stripped() {
        let (mut app, ship) = docked_app(1_000_000, vec![StationServiceType::Refit]);
        app.world_mut().resource_mut::<ShipRuntime>().selected = Some(ship.gun);
        app.update();
        assert_eq!(
            app.world().resource::<ShipRuntime>().delete_verdict(),
            Some(Ok("+1 Frame spar, +3 Section panel"))
        );
        assert_eq!(
            delete_yield_text(&test_items(), &BTreeMap::new()),
            "nothing to salvage"
        );

        let note = delete(&mut app, ship.gun);
        assert_eq!(
            note.as_deref(),
            Some("deleted PDC-1: +1 Frame spar, +3 Section panel to the hold")
        );
        let hold = app.world().get::<ShipInventory>(ship.player).unwrap();
        assert_eq!(hold.count(&"SectionPanel".into()), 3);
        assert_eq!(hold.count(&"FrameSpar".into()), 1);
        let stripped: Vec<(&str, &str)> = reports(&app, ship.player)
            .iter()
            .filter_map(|report| match report {
                ShipInventoryReport::Stripped { hull, section, .. } => {
                    Some((hull.as_str(), section.as_str()))
                }
                _ => None,
            })
            .collect();
        assert_eq!(stripped, [("player", "PDC-1")]);
        assert_eq!(app.world().resource::<Removed>().0, [ship.gun]);
        assert_eq!(app.world().resource::<ShipRuntime>().selected(), None);
    }

    /// A delete that would split the ship, take its last computer, let go
    /// of the dock, overfill the hold or happen away from a refit is refused
    /// with its reason before any press, and a command raised for it anyway
    /// changes nothing and shows no note.
    #[test]
    fn a_delete_is_refused_with_its_reason_and_changes_nothing() {
        let (mut app, ship) = docked_app(1_000_000, vec![StationServiceType::Refit]);
        assert_eq!(
            verdict(&mut app, ship.hull),
            Some(Err(
                "delete: taking HULL-1 off would split the ship".to_string()
            ))
        );
        assert!(matches!(
            verdict(&mut app, ship.computer),
            Some(Err(line)) if line.starts_with("delete: the ship would carry 0 flight computers")
        ));
        assert_eq!(
            verdict(&mut app, ship.port),
            Some(Err("delete: the dock holds the ship by DOCK-1".to_string()))
        );
        assert_eq!(delete(&mut app, ship.port), None, "a refusal shows no note");
        assert_eq!(
            app.world().resource::<ShipRuntime>().selected(),
            Some(ship.port),
            "a refusal leaves its section selected"
        );

        // 30 kg of panels and 40 kg of spar, with room for 50.
        let (mut small, small_ship) = docked_app(50_000, vec![StationServiceType::Refit]);
        assert_eq!(
            verdict(&mut small, small_ship.gun),
            Some(Err(
                "delete: PDC-1 gives 70 kg of parts, the hold has 50 kg free".to_string()
            ))
        );
        assert_eq!(delete(&mut small, small_ship.gun), None);

        let (mut storage, storage_ship) = docked_app(1_000_000, vec![StationServiceType::Storage]);
        assert_eq!(
            verdict(&mut storage, storage_ship.gun),
            Some(Err("delete: this dock offers no refit".to_string()))
        );
        assert_eq!(delete(&mut storage, storage_ship.gun), None);

        for (app, player) in [
            (&app, ship.player),
            (&small, small_ship.player),
            (&storage, storage_ship.player),
        ] {
            assert!(reports(app, player).is_empty());
            assert!(app.world().resource::<Removed>().0.is_empty());
            assert!(
                app.world()
                    .get::<ShipInventory>(player)
                    .unwrap()
                    .count(&"SectionPanel".into())
                    == 0
            );
        }
    }
}
