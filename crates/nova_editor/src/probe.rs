//! The editor's outward state, as data rather than as pixels.
//!
//! Everything the editor decides - which tool is armed, what a click would
//! build, what the gallery is showing - lives in `pub(crate)` resources shaped
//! for the systems that write them. [`EditorProbe`] is the one PUBLIC,
//! read-only snapshot of those decisions, so a driven run waits on the editor
//! having reacted instead of counting frames and hoping. Refreshed in
//! `PostUpdate` and never read back by the editor itself.

use bevy::prelude::*;
use nova_ship::prelude::GameSections;
use nova_ui::prelude::TextFieldFocused;

use crate::{
    config::{PlacementPreview, SectionChoice, SelectedNode},
    gallery::GalleryState,
    gizmo::GizmoRig,
    node::{
        context_nodes, inside_id, sections_of, EditContext, ObjectNodes, SectionNodes, ShipNodes,
    },
    ui::inspector::{Document, InspectorField},
    ExampleStates,
};

/// Which placement tool the editor is holding.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum EditorTool {
    /// Select / rebind: a click arms a keybind capture and places nothing.
    #[default]
    Select,
    /// Placing the section with this catalog id.
    Place(String),
    /// Deleting the section clicked.
    Delete,
}

/// What a click would build right now.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum EditorPlacement {
    /// Nothing armed, nothing under the pointer, or the gallery covering it.
    #[default]
    None,
    /// A legal mate: `prototype` would land on the section `target`.
    Solved {
        /// Catalog id of the armed prototype.
        prototype: String,
        /// The preview section it mates onto.
        target: Entity,
    },
    /// The solver refused this pose.
    Refused {
        /// Catalog id of the armed prototype.
        prototype: String,
        /// Why, in the same words the placement status line shows.
        reason: &'static str,
    },
}

/// One section of the ship being edited, as data.
///
/// The POSE is in the ship's own frame, which is the frame the solver works in
/// and the frame a saved file records. A driven run used to read this off the
/// scene, which stopped being possible when the thing carrying `SectionMarker`
/// became a render-only view with an identity transform of its own.
#[derive(Debug, Clone, PartialEq)]
pub struct EditorSection {
    /// The section's stable id, unique within its ship.
    pub id: String,
    /// The catalog prototype it was built from.
    pub prototype: String,
    /// Where it sits, in the ship's frame.
    pub position: Vec3,
    /// How it is turned, in the ship's frame.
    pub rotation: Quat,
}

/// The editor's outward state, refreshed once a frame.
///
/// Read-only from outside the crate: it is what a harness waits ON, and an
/// editor that also read it back would be waiting on itself.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct EditorProbe {
    /// Which placement tool is armed.
    pub tool: EditorTool,
    /// What a click would build right now.
    pub placement: EditorPlacement,
    /// Whether the parts gallery overlay is up.
    pub gallery_open: bool,
    /// Whether the gallery's filter field holds the caret. Typing reaches the
    /// filter only while it does.
    pub filter_focused: bool,
    /// The catalog id the gallery's selection resolves to through the active
    /// filter - what Enter would focus, and then place.
    pub selected: Option<String>,
    /// The sections of the ship being edited, in id order. Empty out in the
    /// scenario context, where there is no ship to report.
    ///
    /// SCOPED to the edit context on purpose: with more than one ship on the
    /// stage, a sweep of every section in the world is several ships at once,
    /// and the runtime's own structure derivation rejects that.
    pub ship: Vec<EditorSection>,
    /// The id of the node the editor is INSIDE, or `None` at the scenario node.
    ///
    /// The one fact that says which ship every other editor system is scoped
    /// to, and the only way a driven run can tell "entered ship_2" from "still
    /// looking at ship_1 from outside".
    pub inside: Option<String>,
    /// The node ids the current context CONTAINS: at the scenario node its
    /// ships in id order and then the world's objects in id order, inside a
    /// ship that ship's sections. The Scene tree draws more than this (the root
    /// row, collapsed sibling ships), but this is the list every editor system
    /// is scoped to.
    pub context_nodes: Vec<String>,
    /// The id of the node the Scene tree has marked, or `None`.
    pub selected_node: Option<String>,
    /// Whether Play would hand off right now. False inside a ship, where the
    /// button is disabled.
    pub can_play: bool,
    /// The ship nodes ON STAGE, in id order: every ship at the scenario node,
    /// only the entered one inside a ship. What a driven run reads to prove
    /// the focus isolation, instead of counting rendered meshes.
    pub visible_ships: Vec<String>,
    /// The context nodes' poses, where they have one: ships in world space at
    /// the scenario node, sections in ship-local space inside one. What a drag
    /// beat asserts against.
    pub node_positions: Vec<(String, Vec3)>,
    /// Whether an inspector field holds the caret.
    ///
    /// Typing reaches the document only while one does, and while one does the
    /// editor's own single-letter keys stand down. A run that means to type a
    /// value waits for this the same way it waits on the gallery's filter.
    pub inspector_focused: bool,
    /// The inspector's rows for the node it is on, label then value, in the
    /// order the panel draws them.
    ///
    /// Read off the DOCUMENT through the same walk the panel uses, not off the
    /// panel's text nodes: a run that typed a radius wants to know the config
    /// took it, and a readout that agreed with a stale document would say yes
    /// either way.
    pub inspector: Vec<(String, String)>,
    /// The id of the node the transform handles are ON, or `None` while they
    /// are off screen.
    ///
    /// The handles are hidden wherever the pointer belongs to something else -
    /// inside a ship, under an armed part, behind the gallery - so this is
    /// also the answer to "can I drag an axis right now".
    pub gizmo_node: Option<String>,
}

/// Refresh [`EditorProbe`] from the build state.
///
/// Outside the editor scene the snapshot is the default, so nothing can read a
/// build that is no longer on screen.
pub(crate) fn sync_editor_probe(
    editor: Res<State<ExampleStates>>,
    choice: Res<SectionChoice>,
    preview: Res<PlacementPreview>,
    gallery: Res<GalleryState>,
    sections: Option<Res<GameSections>>,
    context: Res<EditContext>,
    selected: Res<SelectedNode>,
    nodes: SectionNodes,
    q_ships: ShipNodes,
    q_objects: ObjectNodes,
    q_visibility: Query<&Visibility>,
    poses: Query<&Transform>,
    document: Document,
    caret: Query<(), (With<TextFieldFocused>, With<InspectorField>)>,
    rig: Query<&Visibility, With<GizmoRig>>,
    mut probe: ResMut<EditorProbe>,
) {
    let wanted = if *editor.get() == ExampleStates::Editor {
        let mut snapshot = snapshot(&choice, &preview, &gallery, sections.as_deref());
        let listed = context_nodes(&context, &q_ships, &q_objects, &nodes);
        snapshot.ship = edited_ship(&context, &nodes);
        snapshot.inside = inside_id(&context, &q_ships).map(|id| id.0.clone());
        snapshot.selected_node = selected.0.and_then(|node| {
            listed
                .iter()
                .find(|listed| listed.entity == node)
                .map(|listed| listed.id.0.clone())
        });
        // The same rule `continue_to_simulation` enforces, reported rather than
        // re-derived from the button's paint - a driven run asserts what Play
        // WOULD do, not what it looks like.
        snapshot.can_play = context.scenario().is_some() && context.ship().is_none();
        // What `sync_ship_focus` decided, read off the same component it
        // writes. A ship with no `Visibility` (a bare test fixture) counts as
        // on stage, which is what it would render as.
        let mut on_stage: Vec<String> = q_ships
            .iter()
            .filter(|(entity, ..)| {
                q_visibility
                    .get(*entity)
                    .map(|visibility| *visibility != Visibility::Hidden)
                    .unwrap_or(true)
            })
            .map(|(_, _, id, _)| id.0.clone())
            .collect();
        on_stage.sort_unstable();
        snapshot.visible_ships = on_stage;
        snapshot.node_positions = listed
            .iter()
            .filter_map(|node| {
                let pose = poses.get(node.entity).ok()?;
                Some((node.id.0.clone(), pose.translation))
            })
            .collect();
        snapshot.context_nodes = listed.into_iter().map(|node| node.id.0.clone()).collect();
        // The handles ride the selection, so the node they are on is the
        // selected one - reported only while they are actually up.
        snapshot.gizmo_node = rig
            .iter()
            .any(|visibility| *visibility != Visibility::Hidden)
            .then(|| snapshot.selected_node.clone())
            .flatten();
        snapshot.inspector_focused = !caret.is_empty();
        snapshot.inspector = document
            .inspection()
            .map(|(_, rows)| {
                rows.into_iter()
                    .map(|row| (row.label, row.value.reading()))
                    .collect()
            })
            .unwrap_or_default();
        snapshot
    } else {
        EditorProbe::default()
    };
    // Compared rather than written: an identical snapshot rewritten every frame
    // would make this resource's change detection say nothing.
    if *probe != wanted {
        *probe = wanted;
    }
}

/// The sections of the ship in the edit context, in id order.
fn edited_ship(context: &EditContext, nodes: &SectionNodes) -> Vec<EditorSection> {
    let Some(ship) = context.ship() else {
        return Vec::new();
    };
    sections_of(ship, nodes)
        .into_iter()
        .map(|(_, id, section, transform)| EditorSection {
            id: id.0.clone(),
            prototype: section.prototype().to_string(),
            position: transform.translation,
            rotation: transform.rotation,
        })
        .collect()
}

/// The snapshot for one frame of the live editor.
fn snapshot(
    choice: &SectionChoice,
    preview: &PlacementPreview,
    gallery: &GalleryState,
    sections: Option<&GameSections>,
) -> EditorProbe {
    let tool = match choice {
        SectionChoice::None => EditorTool::Select,
        SectionChoice::Section(id) => EditorTool::Place(id.clone()),
        SectionChoice::Delete => EditorTool::Delete,
    };
    EditorProbe {
        // A placement is always FOR the tool in hand. The solver only ever
        // produces one for the armed prototype, so the two can disagree in
        // exactly one way: something changed the tool later in the same `Update`
        // than the solve - Escape putting the part down, the gallery arming a
        // different one on its way out. Publishing a solve for a part nobody is
        // holding would let `editor_placement_solved()` advance on a build that
        // cannot happen (review a4a6 R1).
        placement: match (&tool, preview.placement.as_ref()) {
            (EditorTool::Place(armed), Some(placement)) if *armed == placement.prototype => {
                match placement.solve.refusal {
                    None => EditorPlacement::Solved {
                        prototype: placement.prototype.clone(),
                        target: placement.target_section,
                    },
                    Some(refusal) => EditorPlacement::Refused {
                        prototype: placement.prototype.clone(),
                        reason: refusal.message(),
                    },
                }
            }
            _ => EditorPlacement::None,
        },
        tool,
        gallery_open: gallery.open,
        filter_focused: gallery.open && gallery.filter_focused,
        selected: gallery
            .open
            .then(|| sections.and_then(|sections| gallery.selected_id(sections)))
            .flatten(),
        // Filled in by the caller, which has the document; this half of the
        // snapshot is a pure function of the tool and the gallery.
        ship: Vec::new(),
        inside: None,
        context_nodes: Vec::new(),
        selected_node: None,
        can_play: false,
        visible_ships: Vec::new(),
        node_positions: Vec::new(),
        inspector_focused: false,
        inspector: Vec::new(),
        gizmo_node: None,
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use nova_scenario::prelude::SectionSource;
    use nova_ship::prelude::{BaseSectionConfig, HullSectionConfig, SectionConfig, SectionKind};

    use super::*;
    use crate::{config::Placement, snap};

    fn catalog(ids: &[&str]) -> GameSections {
        GameSections(
            ids.iter()
                .map(|id| SectionConfig {
                    base: BaseSectionConfig {
                        id: (*id).to_string(),
                        name: (*id).to_string(),
                        ..default()
                    },
                    kind: SectionKind::Hull(HullSectionConfig::default()),
                })
                .collect(),
        )
    }

    fn solved(prototype: &str, target: Entity, refusal: Option<snap::Refusal>) -> PlacementPreview {
        PlacementPreview {
            placement: Some(Placement {
                prototype: prototype.to_string(),
                target_section: target,
                solve: snap::Placement {
                    transform: Transform::default(),
                    source: 0,
                    target: 0,
                    refusal,
                },
            }),
        }
    }

    /// A world in the editor state, with the resources the snapshot reads.
    ///
    /// The edit context is empty: these cases are about the TOOL and the
    /// gallery, so there is no ship to report and `ship` stays empty. The
    /// document's own reporting is covered in `crate::node`.
    fn world(state: ExampleStates) -> World {
        let mut world = World::new();
        world.insert_resource(State::new(state));
        world.insert_resource(SectionChoice::None);
        world.init_resource::<PlacementPreview>();
        world.init_resource::<GalleryState>();
        world.init_resource::<EditContext>();
        world.init_resource::<SelectedNode>();
        world.init_resource::<EditorProbe>();
        world
    }

    fn sync(world: &mut World) -> EditorProbe {
        world
            .run_system_once(sync_editor_probe)
            .expect("the probe sync runs");
        world.resource::<EditorProbe>().clone()
    }

    /// The ship in the edit context is reported as DATA, in id order.
    ///
    /// This is what a driven run reads instead of the scene: what carries
    /// `SectionMarker` in the editor is a render-only view whose own transform
    /// is identity, so the pose is only answerable from the document. Reported
    /// in ID order rather than query order, because a run that picks one
    /// section out of this list must pick the same one every time.
    #[test]
    fn the_probe_reports_the_edited_ship_in_id_order() {
        use crate::node::{NodeId, SectionNode};

        let mut world = world(ExampleStates::Editor);
        assert!(
            sync(&mut world).ship.is_empty(),
            "no ship entered, nothing to report"
        );

        let ship = world.spawn(crate::node::ShipNode::default()).id();
        // Spawned nose-first so query order and id order disagree.
        for (id, z) in [("hull_2", 1.0), ("hull_1", 0.0)] {
            world.spawn((
                SectionNode {
                    source: SectionSource::Inline(SectionConfig {
                        base: BaseSectionConfig {
                            id: "hull".to_string(),
                            name: "hull".to_string(),
                            ..default()
                        },
                        kind: SectionKind::Hull(HullSectionConfig::default()),
                    }),
                    modifications: vec![],
                    binds: vec![],
                },
                NodeId(id.to_string()),
                Transform::from_xyz(0.0, 0.0, z),
                ChildOf(ship),
            ));
        }
        world.resource_mut::<EditContext>().path = vec![Entity::PLACEHOLDER, ship];

        let reported = sync(&mut world).ship;
        assert_eq!(
            reported.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            ["hull_1", "hull_2"],
            "reported in id order, not in the order they were spawned"
        );
        assert_eq!(reported[0].prototype, "hull");
        assert_eq!(reported[1].position, Vec3::new(0.0, 0.0, 1.0));
    }

    /// The marked node's inspector rows travel with the snapshot.
    ///
    /// A driven run types a number into the panel and then has to ask whether
    /// the DOCUMENT took it. Reading the panel's own text back would answer
    /// yes for a repaint that never reached the config, so the rows are walked
    /// off the document here, the same way the panel walks them.
    #[test]
    fn the_probe_reports_the_rows_the_inspector_is_showing() {
        use nova_scenario::prelude::{AsteroidConfig, ScenarioObjectKind};

        use crate::node::{EditorNode, NodeId, ObjectNode, ScenarioNode};

        let mut world = world(ExampleStates::Editor);
        let scenario = world
            .spawn((EditorNode, ScenarioNode, NodeId("scenario".to_string())))
            .id();
        world.resource_mut::<EditContext>().path = vec![scenario];
        assert!(
            sync(&mut world).inspector.is_empty(),
            "the document root holds nodes, not fields of its own"
        );

        let rock = world
            .spawn((
                EditorNode,
                ObjectNode {
                    name: "rock".to_string(),
                    kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                        radius: 7.0,
                        texture: default(),
                        impact_sound: None,
                        destroy_sound: None,
                        mass: None,
                        invulnerable: false,
                        seed: None,
                        lock_signature: None,
                    }),
                },
                NodeId("asteroid_1".to_string()),
                Transform::from_xyz(0.0, 0.0, 0.0),
                ChildOf(scenario),
            ))
            .id();
        world.resource_mut::<SelectedNode>().0 = Some(rock);

        let rows = sync(&mut world).inspector;
        assert_eq!(
            rows.iter()
                .find(|(label, _)| label == "Radius")
                .map(|(_, value)| value.as_str()),
            Some("7"),
            "the rock's own config is what the panel is showing: {rows:?}"
        );
    }

    /// The context, as data. A driven run has to be able to tell "entered
    /// ship_2" from "looking at ship_2 from outside", and to know whether Play
    /// would hand off before it presses it.
    #[test]
    fn the_probe_reports_the_context_its_contents_and_the_play_gate() {
        use crate::node::{NodeId, ScenarioNode, SectionNode, ShipDriver, ShipNode};

        let mut world = world(ExampleStates::Editor);
        let scenario = world
            .spawn((ScenarioNode, NodeId("scenario".to_string())))
            .id();
        world.resource_mut::<EditContext>().path = vec![scenario];
        // Spawned out of order, so id order and spawn order disagree.
        for (id, driver) in [("ship_2", ShipDriver::Ai), ("ship_1", ShipDriver::Player)] {
            world.spawn((
                ShipNode {
                    driver,
                    ..default()
                },
                NodeId(id.to_string()),
                ChildOf(scenario),
            ));
        }

        let outside = sync(&mut world);
        assert_eq!(outside.inside, None, "the document opens at the scenario");
        assert_eq!(outside.context_nodes, ["ship_1", "ship_2"]);
        assert!(outside.can_play, "Play is the scenario node's gesture");
        assert!(outside.ship.is_empty(), "no ship entered, none reported");

        let first = world
            .query_filtered::<Entity, With<ShipNode>>()
            .iter(&world)
            .find(|entity| world.get::<NodeId>(*entity) == Some(&NodeId("ship_1".to_string())))
            .expect("ship_1");
        world.resource_mut::<EditContext>().enter(first);
        // A section so the list inside the ship is not empty either way.
        world.spawn((
            SectionNode {
                source: SectionSource::Inline(SectionConfig {
                    base: BaseSectionConfig {
                        id: "hull".to_string(),
                        name: "hull".to_string(),
                        ..default()
                    },
                    kind: SectionKind::Hull(HullSectionConfig::default()),
                }),
                modifications: vec![],
                binds: vec![],
            },
            NodeId("hull_1".to_string()),
            Transform::default(),
            ChildOf(first),
        ));

        let inside = sync(&mut world);
        assert_eq!(inside.inside, Some("ship_1".to_string()));
        assert_eq!(
            inside.context_nodes,
            ["hull_1"],
            "inside a ship the list is that ship's sections"
        );
        assert!(
            !inside.can_play,
            "Play compiles the document, which is not what a ship context asked for"
        );
    }

    /// The stage as data: which ships render, so a driven run proves the focus
    /// isolation without counting meshes - and the context nodes' poses, which
    /// is what a drag beat asserts against.
    #[test]
    fn the_probe_reports_the_stage_and_the_node_positions() {
        use crate::node::{NodeId, ScenarioNode, ShipNode};

        let mut world = world(ExampleStates::Editor);
        let scenario = world
            .spawn((ScenarioNode, NodeId("scenario".to_string())))
            .id();
        world.resource_mut::<EditContext>().path = vec![scenario];
        let _first = world
            .spawn((
                ShipNode::default(),
                NodeId("ship_1".to_string()),
                Transform::default(),
                Visibility::Visible,
                ChildOf(scenario),
            ))
            .id();
        let second = world
            .spawn((
                ShipNode::default(),
                NodeId("ship_2".to_string()),
                Transform::from_xyz(24.0, 0.0, 0.0),
                Visibility::Visible,
                ChildOf(scenario),
            ))
            .id();

        let outside = sync(&mut world);
        assert_eq!(outside.visible_ships, ["ship_1", "ship_2"]);
        assert_eq!(
            outside.node_positions,
            [
                ("ship_1".to_string(), Vec3::ZERO),
                ("ship_2".to_string(), Vec3::new(24.0, 0.0, 0.0)),
            ],
            "ships report their stage positions at the scenario node"
        );

        // What sync_ship_focus does on entering the first ship.
        *world.get_mut::<Visibility>(second).unwrap() = Visibility::Hidden;
        assert_eq!(
            sync(&mut world).visible_ships,
            ["ship_1"],
            "a hidden sibling is not on the stage"
        );
    }

    /// A selection is reported by ID, and only while its node is one the
    /// current context lists.
    #[test]
    fn the_probe_reports_the_selected_node_by_id() {
        use crate::node::{NodeId, ScenarioNode, ShipNode};

        let mut world = world(ExampleStates::Editor);
        let scenario = world
            .spawn((ScenarioNode, NodeId("scenario".to_string())))
            .id();
        world.resource_mut::<EditContext>().path = vec![scenario];
        let ship = world
            .spawn((
                ShipNode::default(),
                NodeId("ship_1".to_string()),
                ChildOf(scenario),
            ))
            .id();

        world.resource_mut::<SelectedNode>().0 = Some(ship);
        assert_eq!(sync(&mut world).selected_node, Some("ship_1".to_string()));

        // Inside the ship, the ship itself is not a row - so there is nothing
        // to report even though the resource still names it.
        world.resource_mut::<EditContext>().enter(ship);
        assert_eq!(sync(&mut world).selected_node, None);
    }

    /// The armed tool and the solved mate are the two facts a placement beat
    /// waits on, and both are readable without touching a solver internal.
    #[test]
    fn the_probe_reports_the_armed_tool_and_the_solved_placement() {
        let mut world = world(ExampleStates::Editor);
        assert_eq!(sync(&mut world), EditorProbe::default());

        let target = world.spawn_empty().id();
        world.insert_resource(SectionChoice::Section("hull".to_string()));
        world.insert_resource(solved("hull", target, None));

        assert_eq!(
            sync(&mut world),
            EditorProbe {
                tool: EditorTool::Place("hull".to_string()),
                placement: EditorPlacement::Solved {
                    prototype: "hull".to_string(),
                    target,
                },
                ..default()
            }
        );

        // A refusal is the SAME line the builder is shown, so a beat can assert
        // on the words rather than scraping the status node for them.
        world.insert_resource(solved("hull", target, Some(snap::Refusal::Occupied)));
        assert_eq!(
            sync(&mut world).placement,
            EditorPlacement::Refused {
                prototype: "hull".to_string(),
                reason: "socket occupied",
            }
        );

        // The other two tools are readable as themselves rather than as "not
        // placing".
        world.insert_resource(SectionChoice::Delete);
        assert_eq!(sync(&mut world).tool, EditorTool::Delete);
    }

    /// What the gallery is showing is reported while it is up and gone once it
    /// is down - the two facts an arming beat waits on.
    #[test]
    fn the_gallery_reports_its_caret_and_its_selection() {
        let mut world = world(ExampleStates::Editor);
        world.insert_resource(catalog(&["hull_a", "hull_b"]));
        world.insert_resource(GalleryState {
            open: true,
            filter_focused: true,
            selected: 1,
            ..default()
        });

        let probe = sync(&mut world);
        assert!(probe.gallery_open && probe.filter_focused);
        assert_eq!(
            probe.selected.as_deref(),
            Some("hull_b"),
            "the selection resolves through the active filter"
        );

        world.insert_resource(GalleryState::default());
        let probe = sync(&mut world);
        assert!(!probe.gallery_open && !probe.filter_focused);
        assert_eq!(probe.selected, None);
    }

    /// A placement is always FOR the tool in hand.
    ///
    /// The solver only ever produces one for the armed prototype, so the two can
    /// disagree in exactly one way: something changed the tool later in the same
    /// `Update` than the solve - Escape putting the part down, or the gallery
    /// arming a different one on its way out. Publishing that would let a beat
    /// advance on a build that cannot happen (review a4a6 R1).
    #[test]
    fn a_placement_for_a_part_nobody_is_holding_is_not_published() {
        let mut world = world(ExampleStates::Editor);
        let target = world.spawn_empty().id();
        world.insert_resource(solved("hull_a", target, None));

        // Escape put the part down after the solve.
        world.insert_resource(SectionChoice::None);
        assert_eq!(sync(&mut world).placement, EditorPlacement::None);

        // The delete tool is not a placing tool either.
        world.insert_resource(SectionChoice::Delete);
        assert_eq!(sync(&mut world).placement, EditorPlacement::None);

        // A DIFFERENT part in hand: the solve belongs to the one just put down.
        world.insert_resource(SectionChoice::Section("hull_b".to_string()));
        assert_eq!(sync(&mut world).placement, EditorPlacement::None);

        // Delivery guard: the same solve with its own part in hand publishes.
        world.insert_resource(SectionChoice::Section("hull_a".to_string()));
        assert!(matches!(
            sync(&mut world).placement,
            EditorPlacement::Solved { .. }
        ));
    }

    /// Stands in for `update_placement_preview`, which needs a ship, a camera
    /// and a pointer before it can say anything.
    ///
    /// What it shares with the real system is the only thing the schedule test
    /// below turns on: it writes a FRESH solve for the armed part, and it is
    /// registered with the same gate and the same order against the gallery's
    /// keyboard.
    fn stage_a_solve(choice: Res<SectionChoice>, mut preview: ResMut<PlacementPreview>) {
        let SectionChoice::Section(id) = &*choice else {
            return;
        };
        preview.placement = Some(Placement {
            prototype: id.clone(),
            target_section: Entity::PLACEHOLDER,
            solve: snap::Placement {
                transform: Transform::default(),
                source: 0,
                target: 0,
                refusal: None,
            },
        });
    }

    /// The editor's real schedule shape around the gallery: an ungated clear,
    /// then a solve gated on the gallery being closed, both before the gallery's
    /// keyboard, and the snapshot in `PostUpdate`.
    fn scheduled_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.insert_state(ExampleStates::Editor);
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_message::<bevy::input::keyboard::KeyboardInput>();
        app.insert_resource(catalog(&["hull"]));
        app.insert_resource(SectionChoice::None);
        app.init_resource::<PlacementPreview>();
        app.init_resource::<GalleryState>();
        app.init_resource::<EditContext>();
        app.init_resource::<SelectedNode>();
        app.init_resource::<EditorProbe>();
        app.add_systems(
            Update,
            (
                crate::placement::clear_placement_preview,
                stage_a_solve.run_if(not(crate::gallery::gallery_open)),
            )
                .chain()
                .before(crate::gallery::gallery_keyboard)
                .run_if(in_state(ExampleStates::Editor)),
        );
        app.add_systems(Update, crate::gallery::gallery_keyboard);
        app.add_systems(PostUpdate, sync_editor_probe);
        app
    }

    /// Tap `key` for exactly one frame.
    fn tap(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(key);
        keys.clear();
    }

    /// The frame a keystroke closes the gallery must publish NO placement.
    ///
    /// The solver is gated on the gallery being closed and ordered before the
    /// gallery's keyboard, so on that frame it does not run at all - while the
    /// keyboard arms a part and takes the overlay down after it. Left alone, the
    /// snapshot then republishes the build view's answer from before the gallery
    /// went up: a different pointer position, a different camera, possibly a
    /// different part (review a4a6 R1).
    ///
    /// Driven through the real `gallery_keyboard` in the real order, not by
    /// hand-setting the state.
    #[test]
    fn a_gallery_close_publishes_no_placement_from_before_it_opened() {
        let mut app = scheduled_app();

        // Delivery guard: with the gallery down and a part in hand, this rig
        // DOES publish a placement - so the assertion below is not vacuous.
        app.world_mut()
            .insert_resource(SectionChoice::Section("hull".to_string()));
        app.update();
        assert!(
            matches!(
                app.world().resource::<EditorProbe>().placement,
                EditorPlacement::Solved { .. }
            ),
            "the rig publishes a solve when the solver has run"
        );

        // Up goes the gallery, over the ship and over that solve.
        app.world_mut().insert_resource(GalleryState {
            open: true,
            focused: true,
            ..default()
        });
        app.update();
        assert_eq!(
            app.world().resource::<EditorProbe>().placement,
            EditorPlacement::None,
            "nothing is being placed while the overlay covers the build area"
        );
        assert!(
            app.world()
                .resource::<PlacementPreview>()
                .placement
                .is_none(),
            "and the preview itself is cleared, not merely hidden by the snapshot"
        );

        // Enter takes the part and closes the gallery, both inside this frame's
        // Update and both AFTER the solver's gate was read.
        tap(&mut app, KeyCode::Enter);

        let probe = app.world().resource::<EditorProbe>();
        assert_eq!(
            probe.tool,
            EditorTool::Place("hull".to_string()),
            "the gallery armed the part on its way out"
        );
        assert!(!probe.gallery_open, "and the overlay is down");
        assert_eq!(
            probe.placement,
            EditorPlacement::None,
            "but no solve ran this frame, so there is no answer to publish"
        );

        // The very next frame solves again, so the walk resumes rather than
        // being stuck on `None`.
        app.update();
        assert!(matches!(
            app.world().resource::<EditorProbe>().placement,
            EditorPlacement::Solved { .. }
        ));
    }

    /// Off the editor scene there is no build to report, so a run that has
    /// flown away cannot read the ship it left behind.
    #[test]
    fn leaving_the_editor_clears_the_probe() {
        let mut world = world(ExampleStates::Scenario);
        let target = world.spawn_empty().id();
        world.insert_resource(SectionChoice::Section("hull".to_string()));
        world.insert_resource(solved("hull", target, None));
        world.insert_resource(GalleryState {
            open: true,
            ..default()
        });

        assert_eq!(sync(&mut world), EditorProbe::default());
    }
}
