//! The build view's live cladding: the skin the ship being built would wear,
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
//! UNDER the skin and the cladding reflows around it - and it is why a refused
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

use bevy::prelude::*;
use nova_scenario::prelude::SectionSource;
use nova_ship::prelude::*;

use crate::config::{PlacementPreview, PlayerSpaceshipConfig, SpaceshipPreviewMarker};

/// Marks one preview plate, so a re-derive can find the whole of last frame's
/// skin without touching anything else in the scene.
#[derive(Component)]
pub(crate) struct EditorSkinPlate;

/// What the plates on screen were derived FROM, so a frame that changed nothing
/// costs one hash instead of a respawned skin.
///
/// The root entity is part of it: New Ship despawns the preview ship and its
/// plates with it, and the same structure rebuilt under a new root still needs
/// its skin laid again.
#[derive(Default)]
pub(crate) struct ShownSkin {
    /// Hash of the derived-from structure, or `None` when nothing is clad.
    signature: Option<u64>,
    /// How many plates the last derivation laid, to catch a skin that went away
    /// with the ship it was on.
    plates: usize,
}

/// Re-derive the build view's cladding when the structure under it changes.
///
/// Chained after `sync_placement_ghost` so the skin is derived from the SAME
/// solve the ghost on screen is showing - a frame late would put the cladding
/// where the part used to be.
///
/// `Res`, never `ResMut`: writing the build state here would mark it changed
/// every frame and this would re-derive for ever.
pub(crate) fn sync_editor_skin(
    mut commands: Commands,
    player_config: Res<PlayerSpaceshipConfig>,
    preview: Res<PlacementPreview>,
    sections: Res<GameSections>,
    styles: Res<GameStyles>,
    root: Option<Single<Entity, With<SpaceshipPreviewMarker>>>,
    q_plates: Query<Entity, With<EditorSkinPlate>>,
    mut shown: Local<ShownSkin>,
) {
    let Some(root) = root.map(|root| *root) else {
        return;
    };
    if !player_config.skin {
        if shown.signature.is_some() {
            strip(&mut commands, &q_plates);
            *shown = ShownSkin::default();
        }
        return;
    }

    // Poses and sockets come out of the BUILD STATE, not off the preview
    // entities: it is the same data Play flattens into a ship, so the editor
    // cannot show a skin the flown ship will not wear.
    let mut placed: Vec<(Transform, &[LinkPoint])> = player_config
        .sections
        .values()
        .filter_map(|section| {
            let SectionSource::Inline(config) = &section.source else {
                return None;
            };
            Some((
                Transform::from_translation(section.position).with_rotation(section.rotation),
                config.base.link_points.as_slice(),
            ))
        })
        .collect();
    // Sorted because the build state is keyed by ENTITY: the same ship read
    // twice can hand its sections over in two orders, and the signature below
    // would then report a change that is only a change of order.
    placed.sort_unstable_by(|(a, _), (b, _)| {
        (a.translation.x, a.translation.y, a.translation.z)
            .partial_cmp(&(b.translation.x, b.translation.y, b.translation.z))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if let Some(ghost) = ghost_section(&preview, &sections) {
        placed.push(ghost);
    }

    let signature = signature(root, &placed);
    if shown.signature == Some(signature) && q_plates.iter().count() == shown.plates {
        return;
    }

    let started = std::time::Instant::now();
    strip(&mut commands, &q_plates);
    // The style goes on the preview ROOT, which is where `dress_skin_plate`'s
    // ancestor walk looks for it - the same walk that finds a flown ship's on
    // its own root two levels up.
    let style = editor_style(&player_config, &styles);
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
    let mut decorated = 0;
    if let Some(style) = style {
        let readings = read_plates(&structure, &plates);
        for placement in scatter_decor(&plates, &readings, style) {
            commands.spawn((
                Name::new("Editor Skin Decor"),
                ShipDecorMarker(style.fixtures[placement.fixture].model.clone()),
                decor_pose(&plates[placement.plate], placement.turns),
                Visibility::Inherited,
                ChildOf(laid[placement.plate]),
            ));
            decorated += 1;
        }
    }

    *shown = ShownSkin {
        signature: Some(signature),
        plates: plates.len(),
    };
    debug!(
        "sync_editor_skin: {} plate(s) and {decorated} decoration(s) over {} section(s) \
         in {:.2} ms",
        plates.len(),
        placed.len(),
        started.elapsed().as_secs_f32() * 1000.0,
    );
}

/// The style the build view dresses its cladding in.
///
/// The ship's own choice when it has made one. There is no picker yet, so a
/// ship that has not falls back to the FIRST authored style rather than to a
/// hard-coded id - which is what makes a mod that ships one look show up in the
/// editor without the editor knowing its name.
fn editor_style<'a>(
    player_config: &PlayerSpaceshipConfig,
    styles: &'a GameStyles,
) -> Option<&'a ShipStyleConfig> {
    match &player_config.style {
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
/// click will build nothing, and cladding it anyway would draw a ship that
/// cannot exist.
fn ghost_section<'a>(
    preview: &PlacementPreview,
    sections: &'a GameSections,
) -> Option<(Transform, &'a [LinkPoint])> {
    let placement = preview.placement.as_ref()?;
    if placement.solve.refusal.is_some() {
        return None;
    }
    let part = sections.get_section(&placement.prototype)?;
    Some((placement.solve.transform, part.base.link_points.as_slice()))
}

/// What the skin on screen was derived from, in one number.
///
/// Cheap enough to take every frame, which is what lets the expensive half run
/// only when the answer would differ. Dragging the pointer across one face of a
/// hull does not move the ghost at all - placement mates sockets, so the ghost
/// travels in whole cells - and those frames stop here.
fn signature(root: Entity, placed: &[(Transform, &[LinkPoint])]) -> u64 {
    let mut hasher = DefaultHasher::new();
    root.hash(&mut hasher);
    for (pose, link_points) in placed {
        pose.translation.x.to_bits().hash(&mut hasher);
        pose.translation.y.to_bits().hash(&mut hasher);
        pose.translation.z.to_bits().hash(&mut hasher);
        pose.rotation.x.to_bits().hash(&mut hasher);
        pose.rotation.y.to_bits().hash(&mut hasher);
        pose.rotation.z.to_bits().hash(&mut hasher);
        pose.rotation.w.to_bits().hash(&mut hasher);
        // The sockets, not the part: two parts with the same footprint and the
        // same sockets are the same thing to the skin.
        for point in *link_points {
            point.normal.x.to_bits().hash(&mut hasher);
            point.normal.y.to_bits().hash(&mut hasher);
            point.normal.z.to_bits().hash(&mut hasher);
        }
    }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use avian3d::prelude::Collider;
    use nova_gameplay::{markers::prelude::SectionMarker, prelude::AssetRef};
    use nova_scenario::prelude::SpaceshipSectionConfig;

    use super::*;
    use crate::{
        config::Placement,
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

    /// The editor as this system sees it: a build state, a preview root, and
    /// nothing else it reads.
    fn app(skin: bool) -> App {
        styled_app(skin, GameStyles::default())
    }

    /// The same, with a style catalog the build view can dress the skin in.
    fn styled_app(skin: bool, styles: GameStyles) -> App {
        let mut app = App::new();
        app.insert_resource(GameSections(vec![hull("hull")]));
        app.insert_resource(PlayerSpaceshipConfig { skin, ..default() });
        app.insert_resource(styles);
        app.init_resource::<PlacementPreview>();
        app.world_mut().spawn((
            SpaceshipPreviewMarker,
            Transform::default(),
            Visibility::Visible,
        ));
        app.add_systems(Update, sync_editor_skin);
        app
    }

    /// Put a section into the build state at `position`, keyed by a fresh
    /// entity as the editor keys it.
    fn build(app: &mut App, position: Vec3) {
        let entity = app.world_mut().spawn_empty().id();
        let section = hull("hull");
        app.world_mut()
            .resource_mut::<PlayerSpaceshipConfig>()
            .sections
            .insert(
                entity,
                SpaceshipSectionConfig {
                    id: entity.to_string(),
                    position,
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Inline(section),
                    modifications: vec![],
                },
            );
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
    /// cladding it would draw a ship that cannot exist.
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
                scatter: default(),
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

        app.world_mut().resource_mut::<PlayerSpaceshipConfig>().skin = false;
        app.update();
        assert_eq!(plates(&mut app), 0, "unclad means no plates at all");

        app.world_mut().resource_mut::<PlayerSpaceshipConfig>().skin = true;
        app.update();
        assert_eq!(plates(&mut app), 6, "and back on again");
    }
}
