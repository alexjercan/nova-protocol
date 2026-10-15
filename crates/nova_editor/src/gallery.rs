//! The editor's host for the parts gallery (`nova_ship::gallery`): a
//! full-window node over the stage, the Tab toggle, the camera parked on an
//! empty stage while the gallery is up, and the chrome hidden under it.
//!
//! The gallery itself - its state, layout, tiles, focus view and keyboard - is
//! shared with the Ship pane. Change this module when the editor's side of
//! the browse changes: where the stage stands, what Tab does, what hides.

use avian3d::prelude::{ColliderAabb, Sensor};
use bevy::{camera::visibility::RenderLayers, core_pipeline::Skybox, prelude::*};
use nova_events::units::prelude::*;
use nova_gameplay::prelude::subtree_collider_aabb;
use nova_ship::prelude::*;
use nova_ui::prelude::{owns_or_enters, InputMode};

use crate::{config::EditorCamera, node::EditContext, ui::layer, ExampleStates};

/// Opens and closes the gallery from the editor.
///
/// Tab, because that is already the game's "open the panel" key. It is free
/// here: the interface it opens in flight needs a ship to show, and the
/// editor's build mode has none (see `toggle_interface`).
pub(crate) const GALLERY_TOGGLE_KEY: KeyCode = KeyCode::Tab;

/// How far above everything the document holds the gallery stage stands.
///
/// Clear of the build area so a tile's collider can never be picked as part of
/// the ship, and so nothing the builder made wanders into frame behind the
/// tiles.
const STAGE_CLEARANCE: Meters = Meters(2_000.0);

/// Ray distance from the parked camera to the stage. The gallery fits its
/// tiles at the same distance, so the two must agree.
const STAGE_DISTANCE: f32 = 8.0;

/// The editor's free-fly camera's place while the gallery is up, restored on
/// close.
///
/// Stored on the camera rather than in a `Local` so it dies with the scene: a
/// second visit to the editor spawns a fresh camera and must not inherit the
/// previous visit's parked pose.
#[derive(Component)]
pub(crate) struct ParkedPose {
    /// Where the builder had flown to.
    transform: Transform,
    /// The scenario's skybox, taken off the camera for as long as the gallery
    /// is up. A cubemap draws behind everything, so one of its stars came
    /// through the gallery's transparent tiles as a stray lit pixel under the
    /// header. This stage is meant to be empty.
    skybox: Option<Skybox>,
}

/// The editor's own rail + drawer root, hidden while the gallery is up.
///
/// The gallery is a MODE, not a panel: its tile grid has to stay transparent
/// for the 3D previews to show through, so the chrome behind it cannot simply
/// be covered - it has to go away.
#[derive(Component)]
pub(crate) struct EditorChrome;

/// The full-window node the gallery lays itself out in, drawn by `camera`.
pub(crate) fn gallery_host(camera: Entity) -> impl Bundle {
    (
        DespawnOnExit(ExampleStates::Editor),
        Name::new("Parts Gallery Host"),
        GalleryHost {
            camera,
            stage: Vec3::Y * STAGE_CLEARANCE.to_engine(),
            layers: RenderLayers::default(),
            close_keys: "Esc/Tab",
        },
        GlobalZIndex(layer::GALLERY_Z),
        // Empty while the gallery is closed; it must not eat the stage's
        // pointer then.
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            ..default()
        },
    )
}

/// Open or close the gallery. Runs whether or not it is up.
pub(crate) fn toggle_gallery(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<GalleryState>) {
    if !keys.just_pressed(GALLERY_TOGGLE_KEY) {
        return;
    }
    if state.open {
        *state = GalleryState {
            open: false,
            ..state.clone()
        };
    } else {
        // A fresh browse starts unfiltered, exactly as the rail row opens it.
        *state = state.opened();
    }
}

/// Whether Tab has a gallery to talk to: parts are ship-context verbs (the
/// Parts button lives in the ship's action group, and a part armed at the
/// scenario node is put straight back down), so the toggle only OPENS inside
/// a ship - while staying answerable wherever it is already open, so Tab can
/// always close what Tab opened.
fn gallery_reachable(context: Res<EditContext>, state: Res<GalleryState>) -> bool {
    state.open || context.ship().is_some()
}

/// Stand the stage clear of everything the document holds.
///
/// Only while the gallery is CLOSED: the tiles are spawned on this and the
/// parked camera is placed from it, and a stage that moved mid-browse would
/// drag one out from under the other.
pub(crate) fn sync_gallery_stage(
    state: Res<GalleryState>,
    context: Res<EditContext>,
    q_children: Query<&Children>,
    q_bounds: Query<&ColliderAabb, Without<Sensor>>,
    mut host: Query<&mut GalleryHost>,
) {
    if state.open {
        return;
    }
    let Ok(mut host) = host.single_mut() else {
        return;
    };
    let top = context
        .scenario()
        .and_then(|scenario| subtree_collider_aabb(scenario, &q_children, &q_bounds))
        .map_or(0.0, |bounds| bounds.max.y);
    let wanted = Vec3::Y * (top + STAGE_CLEARANCE.to_engine());
    if host.stage != wanted {
        host.stage = wanted;
    }
}

/// Park the camera on the gallery stage while the gallery is open, and put it
/// back where the player left it on close.
///
/// The free-fly rig is REMOVED rather than ignored: it writes the camera
/// transform from its own input context, and gallery typing must not fly the
/// camera. The pose is re-applied every frame for the same reason the
/// scenario's cinematic camera does it - one stale input frame would drift the
/// stage.
pub(crate) fn park_camera_for_gallery(
    mut commands: Commands,
    state: Res<GalleryState>,
    host: Query<&GalleryHost>,
    camera: Option<
        Single<(Entity, &mut Transform, Option<&Skybox>, Option<&ParkedPose>), With<EditorCamera>>,
    >,
) {
    let Some(camera) = camera else {
        return;
    };
    let (entity, mut transform, skybox, parked) = camera.into_inner();

    if state.open {
        let Ok(host) = host.single() else {
            return;
        };
        if parked.is_none() {
            commands
                .entity(entity)
                .insert(ParkedPose {
                    transform: *transform,
                    skybox: skybox.cloned(),
                })
                .remove::<Skybox>();
        }
        *transform = Transform::from_translation(host.stage + Vec3::Z * STAGE_DISTANCE)
            .looking_at(host.stage, Vec3::Y);
    } else if let Some(parked) = parked {
        *transform = parked.transform;
        let sky = parked.skybox.clone();
        let mut camera = commands.entity(entity);
        camera.remove::<ParkedPose>();
        if let Some(sky) = sky {
            camera.insert(sky);
        }
    }
}

/// Show the editor chrome only while the gallery is closed.
pub(crate) fn sync_editor_chrome(
    state: Res<GalleryState>,
    mut chrome: Query<&mut Visibility, With<EditorChrome>>,
) {
    let wanted = if state.open {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut visibility in &mut chrome {
        // Compared before writing: an unconditional write dirties the whole UI
        // tree's visibility every frame.
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
}

/// Wire the gallery and its editor host into the editor plugin.
pub(crate) fn register(app: &mut App) {
    if !app.is_plugin_added::<PartsGalleryPlugin>() {
        app.add_plugins(PartsGalleryPlugin);
    }

    // A stale gallery or a part in hand must not survive a scene change.
    app.add_systems(
        OnEnter(ExampleStates::Editor),
        |mut state: ResMut<GalleryState>, mut held: ResMut<HeldPart>| {
            *state = GalleryState::default();
            *held = HeldPart::default();
        },
    );

    app.add_systems(
        Update,
        (
            // Ahead of the gallery: Tab is the only gallery key that acts
            // while the gallery is CLOSED, and the frame it opens on is a
            // frame the browse keys must already see as open. Browse's own
            // gesture, so it answers under Browse and under Normal and goes
            // quiet under a mode above it: a rebind waiting for a key gets
            // the key.
            toggle_gallery
                .run_if(gallery_reachable)
                .run_if(owns_or_enters(InputMode::Browse))
                .before(PartsGallerySystems),
            // Before the tiles are spawned on the stage and placed through
            // the camera.
            (sync_gallery_stage, park_camera_for_gallery)
                .chain()
                .before(PartsGallerySystems),
            sync_editor_chrome.after(PartsGallerySystems),
        )
            .run_if(in_state(ExampleStates::Editor)),
    );
}

#[cfg(test)]
mod tests {
    use avian3d::prelude::{Collider, SimpleCollider};
    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::node::{found_empty_document, EditorNode, NodeId};

    /// A document holding one body whose top stands at `top_y` engine units,
    /// and the editor's gallery host.
    fn document_with_a_body_at(top_y: f32) -> App {
        let mut app = App::new();
        app.init_resource::<EditContext>();
        app.init_resource::<GalleryState>();
        app.world_mut().spawn(gallery_host(Entity::PLACEHOLDER));
        app.world_mut()
            .run_system_once(|mut commands: Commands, mut context: ResMut<EditContext>| {
                found_empty_document(&mut commands, &mut context);
            })
            .expect("the document is founded");
        let scenario = app
            .world()
            .resource::<EditContext>()
            .scenario()
            .expect("a document");
        let body = app
            .world_mut()
            .spawn((
                EditorNode,
                NodeId("body".to_string()),
                Transform::default(),
                ChildOf(scenario),
            ))
            .id();
        app.world_mut().spawn((
            ChildOf(body),
            Collider::sphere(1.0).aabb(Vec3::Y * (top_y - 1.0), Quat::IDENTITY),
        ));
        app
    }

    fn stage_of(app: &mut App) -> Vec3 {
        app.world_mut()
            .run_system_once(sync_gallery_stage)
            .expect("the stage is placed");
        app.world_mut()
            .query::<&GalleryHost>()
            .single(app.world())
            .expect("one host")
            .stage
    }

    /// The fixed 2 km this replaced assumed a bounded build area. An object
    /// authored near y = 20 km drew straight through the tiles.
    #[test]
    fn the_stage_stands_clear_of_whatever_the_document_holds() {
        let tall = 20_000.0;
        let mut app = document_with_a_body_at(tall);
        assert_eq!(
            stage_of(&mut app),
            Vec3::Y * (tall + STAGE_CLEARANCE.to_engine()),
            "the stage clears the top of the document, not a fixed altitude"
        );
    }

    /// The tiles are spawned on the stage and the parked camera is placed from
    /// it, so a stage that moved mid-browse would drag one out from under the
    /// other.
    #[test]
    fn the_stage_holds_still_while_the_gallery_is_open() {
        let mut app = document_with_a_body_at(0.0);
        let parked = stage_of(&mut app);

        app.world_mut().resource_mut::<GalleryState>().open = true;
        let body = app
            .world_mut()
            .query_filtered::<Entity, With<NodeId>>()
            .iter(app.world())
            .next()
            .expect("the body is there");
        app.world_mut()
            .entity_mut(body)
            .insert(Transform::from_xyz(0.0, 5_000.0, 0.0));

        assert_eq!(
            stage_of(&mut app),
            parked,
            "the stage the tiles stand on does not move mid-browse"
        );
    }

    /// The stage is empty by contract: the scenario's skybox comes off the
    /// camera while the gallery is parked on it, and goes back on at the close.
    /// A cubemap is not clipped by anything, so leaving it on put a star in
    /// among the tiles.
    #[test]
    fn parking_the_camera_takes_the_skybox_off_and_gives_it_back() {
        let mut app = App::new();
        app.insert_resource(GalleryState {
            open: true,
            ..default()
        });
        app.add_systems(Update, park_camera_for_gallery);
        let flown_to = Vec3::new(1.0, 2.0, 3.0);
        let camera = app
            .world_mut()
            .spawn((
                EditorCamera,
                Transform::from_translation(flown_to),
                Skybox {
                    brightness: 42.0,
                    ..default()
                },
            ))
            .id();
        app.world_mut().spawn(gallery_host(camera));

        app.update();
        assert!(
            app.world().get::<Skybox>(camera).is_none(),
            "the parked camera must not draw the world's sky"
        );

        app.world_mut().resource_mut::<GalleryState>().open = false;
        app.update();
        let sky = app
            .world()
            .get::<Skybox>(camera)
            .expect("closing the gallery gives the sky back");
        assert_eq!(sky.brightness, 42.0, "and gives back the SAME sky");
        assert_eq!(
            app.world()
                .get::<Transform>(camera)
                .map(|pose| pose.translation),
            Some(flown_to),
            "along with the pose the builder had flown to"
        );
    }

    /// Tab opens the gallery from the build view and closes it again. It is the
    /// game's "open the panel" key, and it is free in the editor because the
    /// interface it opens in flight needs a ship to show.
    #[test]
    fn tab_opens_and_closes_the_gallery() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<GalleryState>();
        app.add_systems(Update, toggle_gallery);
        let tap = |app: &mut App| {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Tab);
            app.update();
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::Tab);
            keys.clear();
        };

        tap(&mut app);
        assert!(app.world().resource::<GalleryState>().open);

        tap(&mut app);
        assert!(!app.world().resource::<GalleryState>().open);
    }
}
