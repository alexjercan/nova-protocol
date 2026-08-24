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

use crate::{
    config::{PlacementPreview, SectionChoice},
    gallery::GalleryState,
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

/// The editor's outward state, refreshed once a frame.
///
/// Read-only from outside the crate: it is what a harness waits ON, and an
/// editor that also read it back would be waiting on itself.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
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
    mut probe: ResMut<EditorProbe>,
) {
    let wanted = if *editor.get() == ExampleStates::Editor {
        snapshot(&choice, &preview, &gallery, sections.as_deref())
    } else {
        EditorProbe::default()
    };
    // Compared rather than written: an identical snapshot rewritten every frame
    // would make this resource's change detection say nothing.
    if *probe != wanted {
        *probe = wanted;
    }
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
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
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
    fn world(state: ExampleStates) -> World {
        let mut world = World::new();
        world.insert_resource(State::new(state));
        world.insert_resource(SectionChoice::None);
        world.init_resource::<PlacementPreview>();
        world.init_resource::<GalleryState>();
        world.init_resource::<EditorProbe>();
        world
    }

    fn sync(world: &mut World) -> EditorProbe {
        world
            .run_system_once(sync_editor_probe)
            .expect("the probe sync runs");
        world.resource::<EditorProbe>().clone()
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
