//! The build view's live skin: the surface the ship being built would wear,
//! re-derived whenever the structure under it moves.
//!
//! The skin is a PURE FUNCTION of structure (`nova_ship`'s `derive_skin`), which
//! is what makes showing it while a part is still in hand safe. The same
//! structure always gives the same plates, so a preview cannot flicker, and
//! putting a part down cannot make the rest of the hull reshuffle. There is
//! nothing to solve and nothing to settle: throw the plates away and derive
//! them again.
//!
//! The part UNDER THE POINTER counts as structure while its placement is legal.
//! That is the whole feature the owner asked for - a hull is dragged about
//! UNDER the skin and the skin reflows around it - and it is why a refused
//! ghost contributes nothing: a placement that cannot be built must not be clad
//! as though it had been.
//!
//! Plates are DISPLAY ONLY. They carry no `SectionMarker` and no `Collider`, so
//! the placement solver never counts one as a part, the pointer never hits one
//! (avian's picking backend raycasts colliders), and the Q pipette cannot arm
//! one. A plate the builder could select or place against would be a part on a
//! ship nobody built.

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

// Bevy's platform Instant, not std's - `std::time::Instant::now` panics
// on wasm32-unknown-unknown, which this crate ships to.
use bevy::{platform::time::Instant, prelude::*};
use nova_ship::prelude::*;

use crate::{
    config::PlacementPreview,
    node::{sections_of, EditContext, SectionNodes, ShipNode},
    ExampleStates,
};

/// Marks one preview plate, so a re-derive can find the whole of last frame's
/// skin without touching anything else in the scene.
#[derive(Component)]
pub(crate) struct EditorSkinPlate;

/// What the plates on screen were derived FROM, so a frame that changed nothing
/// costs one hash instead of a respawned skin.
///
/// The ship node is part of it: entering another ship must re-derive rather
/// than keep the plates of the one you left, and the same structure under a
/// different root still needs its skin laid again.
#[derive(Default)]
pub(crate) struct ShownSkin {
    /// Hash of the derived-from structure, or `None` when nothing is clad.
    signature: Option<u64>,
    /// The resolved style on the shown plates. Style changes must re-dress an
    /// otherwise unchanged structure.
    style: Option<String>,
    /// How many plates the last derivation laid, to catch a skin that went away
    /// with the ship it was on.
    plates: usize,
}

/// Re-derive the build view's skin when the structure under it changes.
///
/// Chained after `sync_placement_ghost` so the skin is derived from the SAME
/// solve the ghost on screen is showing - a frame late would put the skin
/// where the part used to be.
///
/// `Res`, never `ResMut`: writing the build state here would mark it changed
/// every frame and this would re-derive for ever.
#[expect(
    clippy::too_many_arguments,
    reason = "the skin is derived from the document, the ghost and the style catalog"
)]
pub(crate) fn sync_editor_skin(
    mut commands: Commands,
    context: Res<EditContext>,
    preview: Res<PlacementPreview>,
    sections: Res<GameSections>,
    styles: Res<GameStyles>,
    nodes: SectionNodes,
    q_ships: Query<&ShipNode>,
    q_plates: Query<Entity, With<EditorSkinPlate>>,
    mut shown: Local<ShownSkin>,
) {
    let Some(root) = context.ship() else {
        return;
    };
    let Ok(ship) = q_ships.get(root) else {
        return;
    };
    if !ship.skin {
        if shown.signature.is_some() {
            strip(&mut commands, &q_plates);
            *shown = ShownSkin::default();
        }
        return;
    }

    // Poses and sockets come out of the BUILD STATE, not off the preview
    // entities: it is the same data Play flattens into a ship, so the editor
    // cannot show a skin the flown ship will not wear. The EXIT comes from the
    // same place and for the same reason a placement's does: a preview section
    // carries its sockets and its collider as components and nothing that says
    // what kind of part it is.
    // `sections_of` hands them over in id order, which is why the sort this
    // used to do by POSITION is gone: the document has a stable key now, so the
    // signature below cannot report a change that is only a change of order.
    let mut placed: Vec<PlacedPart> = sections_of(root, &nodes)
        .into_iter()
        .filter_map(|(_, _, section, transform)| {
            let config = section.resolve(Some(&sections))?;
            Some(PlacedPart {
                position: transform.translation,
                rotation: transform.rotation,
                link_points: config.base.link_points.as_slice(),
                footprint: *SectionFootprint::from_collider(
                    config.base.collider.unwrap_or_default(),
                ),
                exit: exit_normal(&config.kind),
            })
        })
        .collect();
    if let Some(ghost) = ghost_section(&preview, &sections) {
        placed.push(ghost);
    }

    let signature = signature(root, &placed);
    let style = editor_style(ship, &styles);
    let style_id = style.map(|style| style.id.as_str());
    if shown.signature == Some(signature)
        && shown.style.as_deref() == style_id
        && q_plates.iter().count() == shown.plates
    {
        return;
    }

    let started = Instant::now();
    strip(&mut commands, &q_plates);
    // The style goes on the ship NODE, which is where `dress_skin_plate`'s
    // ancestor walk looks for it - the same walk that finds a flown ship's on
    // its own root two levels up.
    commands
        .entity(root)
        .insert(ShipStyle(style.map(|style| style.id.clone())));

    let (structure, phase, _) = read_structure(&placed);
    let plates = derive_skin(&structure);
    let mut laid: Vec<Entity> = Vec::with_capacity(plates.len());
    for plate in &plates {
        laid.push(
            commands
                .spawn((
                    // The ship node OUTLIVES the editor state, so a plate has to
                    // carry its own teardown: without this the skin would
                    // still be hanging on the ship when Play spawns the real one.
                    DespawnOnExit(ExampleStates::Editor),
                    Name::new("Editor Skin Plate"),
                    EditorSkinPlate,
                    // What `dress_skin_plate` (nova_ship) hangs the meshes off.
                    // The gameplay half of a plate - health, mass, collider -
                    // is deliberately absent: this one is a picture of a plate.
                    ShipSkinMarker(plate.shape),
                    Transform::from_translation(plate.cell.as_vec3() + phase)
                        .with_rotation(plate.rotation),
                    // A meshed child needs a parent that carries visibility, or
                    // bevy drops the mesh and says so.
                    Visibility::Inherited,
                    ChildOf(root),
                ))
                .id(),
        );
    }

    // Decoration, from the SAME scatter the flown ship runs - it is a pure
    // function of the structure, so the build view can show it without a
    // gameplay half. A preview greeble is a marker, a pose and a visibility and
    // nothing else: no collider for the pointer to hit, no health, and no
    // `EditorSkinPlate` of its own, since it goes when its plate does.
    let readings = read_plates(&structure, &plates);
    let mut taken: Vec<usize> = vec![0; style.map_or(0, |style| style.fixtures.len())];
    if let Some(style) = style {
        for placement in scatter_decor(&plates, &readings, style) {
            commands.spawn((
                DespawnOnExit(ExampleStates::Editor),
                Name::new("Editor Skin Decor"),
                ShipDecorMarker(style.fixtures[placement.fixture].model.clone()),
                decor_pose(&plates[placement.plate], placement.turns),
                Visibility::Inherited,
                ChildOf(laid[placement.plate]),
            ));
            taken[placement.fixture] += 1;
        }
    }

    *shown = ShownSkin {
        signature: Some(signature),
        style: style_id.map(str::to_string),
        plates: plates.len(),
    };
    // The same histogram and the same per-rule tally the spawner logs, because
    // this is where the owner BUILDS: a hand-built hull is nearly all studs,
    // ridges and spurs, and a rule tuned on a generated ship lands almost
    // nothing on it. Guessing at that from the screen is how a style ends up
    // tuned for a hull nobody builds.
    debug!(
        "sync_editor_skin: {} plate(s) and {} decoration(s) over {} section(s) in {:.2} ms; \
         relief {}; decoration {}",
        plates.len(),
        taken.iter().sum::<usize>(),
        placed.len(),
        started.elapsed().as_secs_f32() * 1000.0,
        relief_tally(&readings),
        style.map_or_else(
            || "none (no style)".to_string(),
            |style| decor_tally(style, &taken, &decor_reach(&plates, &readings, style)),
        ),
    );
}

/// The style the build view dresses its skin in.
///
/// The ship's own choice when it has made one. There is no picker yet, so a
/// ship that has not falls back to the FIRST authored style rather than to a
/// hard-coded id - which is what makes a mod that ships one look show up in the
/// editor without the editor knowing its name.
fn editor_style<'a>(ship: &ShipNode, styles: &'a GameStyles) -> Option<&'a ShipStyleConfig> {
    match &ship.style {
        Some(id) => styles.get_style(id),
        None => styles.first(),
    }
}

/// Take every plate off the ship.
fn strip(commands: &mut Commands, q_plates: &Query<Entity, With<EditorSkinPlate>>) {
    for plate in q_plates {
        // `try_despawn`: the ship a plate hangs on can go the same frame, and a
        // recursive despawn takes its plates with it.
        commands.entity(plate).try_despawn();
    }
}

/// The part under the pointer, as one more piece of structure - or `None` when
/// nothing is armed or the placement is refused.
///
/// A REFUSED ghost is left out on purpose. The bounds box already says the
/// click will build nothing, and skinning it anyway would draw a ship that
/// cannot exist.
fn ghost_section<'a>(
    preview: &PlacementPreview,
    sections: &'a GameSections,
) -> Option<PlacedPart<'a>> {
    let placement = preview.placement.as_ref()?;
    if placement.solve.refusal.is_some() {
        return None;
    }
    let part = sections.get_section(&placement.prototype)?;
    Some(PlacedPart {
        position: placement.solve.transform.translation,
        rotation: placement.solve.transform.rotation,
        link_points: part.base.link_points.as_slice(),
        footprint: *SectionFootprint::from_collider(part.base.collider.unwrap_or_default()),
        exit: exit_normal(&part.kind),
    })
}

/// What the skin on screen was derived from, in one number.
///
/// Cheap enough to take every frame, which is what lets the expensive half run
/// only when the answer would differ. Dragging the pointer across one face of a
/// hull does not move the ghost at all - placement mates sockets, so the ghost
/// travels in whole cells - and those frames stop here.
fn signature(root: Entity, placed: &[PlacedPart]) -> u64 {
    let mut hasher = DefaultHasher::new();
    root.hash(&mut hasher);
    for part in placed {
        part.position.x.to_bits().hash(&mut hasher);
        part.position.y.to_bits().hash(&mut hasher);
        part.position.z.to_bits().hash(&mut hasher);
        part.rotation.x.to_bits().hash(&mut hasher);
        part.rotation.y.to_bits().hash(&mut hasher);
        part.rotation.z.to_bits().hash(&mut hasher);
        part.rotation.w.to_bits().hash(&mut hasher);
        part.footprint.x.hash(&mut hasher);
        part.footprint.y.hash(&mut hasher);
        part.footprint.z.hash(&mut hasher);
        // The sockets and the exit, not the part: two parts with the same
        // footprint, the same sockets and the same muzzle are the same thing to
        // the skin.
        for point in part.link_points {
            point.normal.x.to_bits().hash(&mut hasher);
            point.normal.y.to_bits().hash(&mut hasher);
            point.normal.z.to_bits().hash(&mut hasher);
        }
        if let Some(exit) = part.exit {
            exit.x.to_bits().hash(&mut hasher);
            exit.y.to_bits().hash(&mut hasher);
            exit.z.to_bits().hash(&mut hasher);
        }
    }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use avian3d::prelude::Collider;
    use nova_gameplay::{markers::prelude::SectionMarker, prelude::AssetRef};
    use nova_scenario::prelude::SectionSource;

    use super::*;
    use crate::{
        config::Placement,
        node::{NextChildOrdinal, NodeId, SectionNode},
        snap::{self, Refusal},
    };

    /// A unit-cube hull: six sockets, one per face, which is what the skin
    /// reads a cell's presented faces off.
    fn hull(id: &str) -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                name: id.to_string(),
                link_points: unit_cube_link_points(),
                ..default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        }
    }

    /// The editor as this system sees it: a document with one ship node
    /// entered, and nothing else it reads.
    fn app(skin: bool) -> App {
        styled_app(skin, GameStyles::default())
    }

    /// The same, with a style catalog the build view can dress the skin in.
    fn styled_app(skin: bool, styles: GameStyles) -> App {
        let mut app = App::new();
        app.insert_resource(GameSections(vec![hull("hull")]));
        app.insert_resource(styles);
        app.init_resource::<PlacementPreview>();
        let ship = app
            .world_mut()
            .spawn((
                ShipNode { skin, ..default() },
                NextChildOrdinal::default(),
                Transform::default(),
                Visibility::Visible,
            ))
            .id();
        app.insert_resource(EditContext {
            path: vec![Entity::PLACEHOLDER, ship],
        });
        app.add_systems(Update, sync_editor_skin);
        app
    }

    /// The ship node the tests build on.
    fn edited(app: &App) -> Entity {
        app.world()
            .resource::<EditContext>()
            .ship()
            .expect("the test app enters its ship")
    }

    /// Put a section node on the ship at `position`.
    fn build(app: &mut App, position: Vec3) {
        let ship = edited(app);
        let index = app
            .world_mut()
            .query_filtered::<(), With<SectionNode>>()
            .iter(app.world())
            .count();
        app.world_mut().spawn((
            SectionNode {
                source: SectionSource::Inline(hull("hull")),
                modifications: vec![],
                binds: vec![],
            },
            NodeId(format!("hull_{index}")),
            Transform::from_translation(position),
            Visibility::Visible,
            ChildOf(ship),
        ));
    }

    /// The part under the pointer, at `position`, with `refusal`.
    fn aim(app: &mut App, position: Vec3, refusal: Option<Refusal>) {
        app.world_mut().resource_mut::<PlacementPreview>().placement = Some(Placement {
            prototype: "hull".to_string(),
            target_section: Entity::PLACEHOLDER,
            solve: snap::Placement {
                transform: Transform::from_translation(position),
                source: 0,
                target: 0,
                refusal,
            },
        });
    }

    fn plates(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<EditorSkinPlate>>()
            .iter(app.world())
            .count()
    }

    /// One cube of hull shows six faces to vacuum, so it wears six plates -
    /// and the plates arrive without anyone asking for them a second time.
    #[test]
    fn a_built_ship_is_clad_from_its_structure_alone() {
        let mut app = app(true);
        build(&mut app, Vec3::ZERO);
        app.update();

        assert_eq!(plates(&mut app), 6, "a lone cube is clad on all six faces");
    }

    /// The claim the whole feature rests on: the skin follows the BUILD. A
    /// second cube changes the surface, and the plates on screen change with
    /// it without a rebuild being asked for.
    #[test]
    fn placing_a_section_reflows_the_skin() {
        let mut app = app(true);
        build(&mut app, Vec3::ZERO);
        app.update();
        let alone = plates(&mut app);

        build(&mut app, Vec3::X);
        app.update();
        let pair = plates(&mut app);

        assert_ne!(
            pair, alone,
            "a second cube must change the surface the skin covers"
        );
        // Two cubes present ten free faces between them, and the two cells
        // between them are structure now.
        assert_eq!(pair, 10);
    }

    /// A frame that changed nothing must not respawn the skin: the plates are
    /// the SAME entities, which is what says the preview cannot flicker while
    /// the pointer moves across a face.
    #[test]
    fn an_unchanged_build_keeps_the_plates_it_had() {
        let mut app = app(true);
        build(&mut app, Vec3::ZERO);
        app.update();
        let before: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<EditorSkinPlate>>()
            .iter(app.world())
            .collect();

        app.update();
        app.update();
        let after: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<EditorSkinPlate>>()
            .iter(app.world())
            .collect();

        assert_eq!(before, after, "an idle frame must leave the skin alone");
    }

    /// A plate must be invisible to the editor's machinery: no `SectionMarker`
    /// for the validator or the pipette to find, no `Collider` for the pointer
    /// to hit. A plate a builder can select or place against is a part on a
    /// ship nobody built.
    #[test]
    fn a_plate_is_not_a_section() {
        let mut app = app(true);
        build(&mut app, Vec3::ZERO);
        app.update();

        assert_eq!(
            app.world_mut()
                .query_filtered::<(), With<SectionMarker>>()
                .iter(app.world())
                .count(),
            0,
            "no plate reads as a section"
        );
        assert_eq!(
            app.world_mut()
                .query_filtered::<(), With<Collider>>()
                .iter(app.world())
                .count(),
            0,
            "no plate stands in the pointer's way"
        );
    }

    /// The owner's ask: a part dragged about UNDER the skin reflows it before
    /// it is committed. The ghost counts as structure while its placement
    /// holds.
    #[test]
    fn the_part_under_the_pointer_is_clad_before_it_is_placed() {
        let mut app = app(true);
        build(&mut app, Vec3::ZERO);
        app.update();
        let alone = plates(&mut app);

        aim(&mut app, Vec3::X, None);
        app.update();
        let with_ghost = plates(&mut app);

        assert_ne!(
            with_ghost, alone,
            "the part under the pointer must reflow the skin around it"
        );
        // The same answer the committed pair gives: what the ghost shows is
        // what the click builds.
        assert_eq!(with_ghost, 10);
    }

    /// A REFUSED ghost is not structure. The click will build nothing, so
    /// skinning it would draw a ship that cannot exist.
    #[test]
    fn a_refused_ghost_is_not_clad() {
        let mut app = app(true);
        build(&mut app, Vec3::ZERO);
        app.update();
        let alone = plates(&mut app);

        aim(&mut app, Vec3::X, Some(Refusal::Occupied));
        app.update();

        assert_eq!(
            plates(&mut app),
            alone,
            "a placement that cannot be built must not be clad"
        );
    }

    /// The build view wears a style: its plates carry the decoration the flown
    /// ship will, bolted to the plates so a reflow takes them with it.
    ///
    /// The determinism is what makes this safe, and it is checked here as well
    /// as in `nova_ship`: an idle frame must leave the same greebles on the same
    /// entities, or a hull dragged about would twinkle.
    #[test]
    fn the_build_view_wears_its_ship_style() {
        let mut app = styled_app(true, GameStyles(vec![test_style()]));
        build(&mut app, Vec3::ZERO);
        app.update();

        assert_eq!(plates(&mut app), 6);
        let before: Vec<Entity> = decor(&mut app);
        assert_eq!(before.len(), 6, "an unfiltered rule dresses every plate");

        // Display only: a greeble the pointer could hit would be a part on a
        // ship nobody built, exactly as a plate would be.
        assert_eq!(
            app.world_mut()
                .query_filtered::<(), (With<Collider>, With<ShipDecorMarker>)>()
                .iter(app.world())
                .count(),
            0,
            "a preview greeble stands in the pointer's way",
        );

        app.update();
        app.update();
        assert_eq!(
            before,
            decor(&mut app),
            "an idle frame respawned the greebles"
        );

        // A reflow takes the old decoration with the old plates.
        build(&mut app, Vec3::X);
        app.update();
        assert_eq!(
            decor(&mut app).len(),
            plates(&mut app),
            "the decoration did not reflow with the skin",
        );
    }

    /// Choosing another look must re-dress a skin even when its structure did
    /// not change. Otherwise the row highlight changes but the old greebles
    /// remain until the builder toggles the skin off and on.
    #[test]
    fn choosing_a_style_redresses_the_skin_immediately() {
        let first = test_style();
        let mut second = test_style();
        second.id = "second".to_string();
        second.name = "Second".to_string();
        let mut app = styled_app(true, GameStyles(vec![first, second]));
        build(&mut app, Vec3::ZERO);
        app.update();
        let before = decor(&mut app);

        let ship = edited(&app);
        app.world_mut()
            .get_mut::<ShipNode>(ship)
            .expect("the ship node")
            .style = Some("second".to_string());
        app.update();

        assert_ne!(
            before,
            decor(&mut app),
            "a style change must replace the old decoration"
        );
        let style = app
            .world_mut()
            .query_filtered::<&ShipStyle, With<ShipNode>>()
            .single(app.world())
            .expect("one preview root");
        assert_eq!(style.0.as_deref(), Some("second"));
    }

    /// No style authored anywhere is a bare skin, not a panic and not a
    /// half-dressed one.
    #[test]
    fn a_build_with_no_style_authored_is_clad_and_bare() {
        let mut app = app(true);
        build(&mut app, Vec3::ZERO);
        app.update();

        assert_eq!(plates(&mut app), 6);
        assert!(decor(&mut app).is_empty());
    }

    /// Every preview greeble on screen.
    fn decor(app: &mut App) -> Vec<Entity> {
        app.world_mut()
            .query_filtered::<Entity, With<ShipDecorMarker>>()
            .iter(app.world())
            .collect()
    }

    /// A style whose one fixture takes every plate.
    fn test_style() -> ShipStyleConfig {
        ShipStyleConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            surfaces: Vec::new(),
            fixtures: vec![StyleFixtureConfig {
                id: "block".to_string(),
                model: AssetRef::from(
                    "self://gltf/greebles/placeholder_block.glb#Scene0".to_string(),
                ),
                health: 10.0,
                density: 0.1,
                collider: Vec3::new(0.2, 0.1, 0.2),
                // `Whole` (the default seat) refuses a crease, and the lone
                // cell this suite builds is six cone plates - the seat gate
                // would land nothing on it. `Any` keeps the fixture what these
                // tests need: a rule that takes every plate.
                scatter: ScatterRule {
                    seat: ScatterSeat::Any,
                    ..default()
                },
            }],
        }
    }

    /// The toggle takes the skin off, and the plates go with it.
    #[test]
    fn the_toggle_takes_the_skin_off() {
        let mut app = app(true);
        build(&mut app, Vec3::ZERO);
        app.update();
        assert_eq!(plates(&mut app), 6);

        let ship = edited(&app);
        app.world_mut()
            .get_mut::<ShipNode>(ship)
            .expect("the ship node")
            .skin = false;
        app.update();
        assert_eq!(plates(&mut app), 0, "unclad means no plates at all");

        app.world_mut()
            .get_mut::<ShipNode>(ship)
            .expect("the ship node")
            .skin = true;
        app.update();
        assert_eq!(plates(&mut app), 6, "and back on again");
    }
}
