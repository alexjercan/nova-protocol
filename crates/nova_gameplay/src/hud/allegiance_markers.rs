//! Allegiance markers: a small filled triangle
//! floating above every NON-player ship, tinted by the ship's
//! [`Allegiance`] so a mixed brawl reads friend-vs-foe at a glance - green
//! ally, red threat, grey neutral. Born from the ch5 raid playtest, where
//! two friendly wingmen and four enemy fighters in one melee were hard to
//! tell apart.
//!
//! The marker is a [`screen_indicator`]-anchored HUD node (the same
//! world-anchor -> UI projection every other overlay uses), one per ship,
//! on the `beacon_chips` observer template. It hides off-screen
//! ([`ScreenIndicatorOffscreen::Hide`]): pointing at off-screen ships is the
//! edge indicators' job, this is a glance cue for what is on screen.
//!
//! The triangle itself is a pure CSS-border-triangle: a zero-CONTENT node
//! (`BoxSizing::ContentBox`, so width/height 0 is the content box and the
//! borders extend OUTSIDE it) whose top border is coloured and whose
//! left/right borders are transparent. Bevy's UI border shader draws each
//! side's colour only in its mitered wedge (`nearest_border_active` in
//! bevy_ui_render), so a coloured top border over a zero-size node renders
//! as a downward-pointing filled triangle - no art asset. `ContentBox` is
//! load-bearing: Bevy defaults nodes to `BoxSizing::BorderBox`, under which
//! a 0x0 node would collapse its borders to nothing.
//!
//! ## Spawn-order note (why colour/player-ness are NOT read at spawn)
//!
//! `Allegiance` and [`PlayerSpaceshipMarker`] are BOTH required components of
//! the controller markers ([`PlayerSpaceshipMarker`] requires
//! `Allegiance::Player`, `AISpaceshipMarker` requires `Allegiance::Enemy`),
//! and those markers are inserted by a DEFERRED command inside nova_scenario's
//! own `Add<SpaceshipRootMarker>` observer - so at the moment THIS module's
//! `Add<SpaceshipRootMarker>` observer runs, neither is present yet. Reading
//! them there would mis-colour every ship grey and mark the player. Instead:
//! spawn the marker for every ship on `Add<SpaceshipRootMarker>` (grey
//! default), despawn the player's the moment its [`PlayerSpaceshipMarker`]
//! lands, and colour from [`Allegiance`] via change detection - which fires
//! when the requirement default lands at spawn-settle AND on a runtime
//! `SetAllegiance` flip (neutral-until-provoked). `SpaceshipController::None`
//! ships never get a controller marker, so they keep the grey default unless
//! they carry an authored allegiance.
//!
//! Instrument tier: combat-relevant, on with the HUD and cleared at the
//! cinematic level. Deliberately NOT contextual: the
//! accepted ruleset does not cover always-on allegiance triangles either way,
//! so they are left as they are pending the owner's playtest call.

use bevy::prelude::*;

use super::{screen_indicator::prelude::*, HudTier};
use crate::prelude::*;

/// Glob-import surface: `use nova_gameplay::hud::allegiance_markers::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::{
        allegiance_color, AllegianceMarkerHudMarker, AllegianceMarkerHudPlugin,
        AllegianceMarkerTargetEntity, AllegianceMarkerTriangleMarker,
    };
}

/// Ally green: the player's side (the player's own AI wingmen included).
const ALLY_GREEN: Color = nova_ui::theme::semantic::ALLY;
/// Threat red: hostile ships.
const THREAT_RED: Color = nova_ui::theme::semantic::THREAT;
/// Neutral grey: bystanders and ships with no allegiance at all.
const NEUTRAL_GREY: Color = nova_ui::theme::semantic::NEUTRAL;

/// Half the triangle's base width (px); the coloured left/right borders.
const TRIANGLE_HALF_WIDTH_PX: f32 = 7.0;
/// The triangle's height (px); the coloured top border.
const TRIANGLE_HEIGHT_PX: f32 = 9.0;

/// The indicator node's fixed footprint: the triangle's border box
/// (`2 * half_width` wide, `height` tall), so the triangle child fills it
/// exactly and centres on the ship's projected point.
const MARKER_SIZE: Vec2 = Vec2::new(2.0 * TRIANGLE_HALF_WIDTH_PX, TRIANGLE_HEIGHT_PX);

/// Upward pixel offset from the ship's projected centre, so the triangle
/// floats above the hull and points down at it. Fixed (not apparent-size
/// scaled) per the task's "cheap, fixed-size node" constraint; tune here if
/// playtest wants it higher off the hull.
const MARKER_OFFSET: Vec2 = Vec2::new(0.0, -40.0);

/// The allegiance tint for a ship's marker. `None` (a ship carrying no
/// [`Allegiance`] at all) reads as neutral, same as an explicit
/// `Allegiance::Neutral` - a bystander either way.
pub fn allegiance_color(allegiance: Option<&Allegiance>) -> Color {
    match allegiance {
        Some(Allegiance::Player) => ALLY_GREEN,
        Some(Allegiance::Enemy) => THREAT_RED,
        Some(Allegiance::Neutral) | None => NEUTRAL_GREY,
    }
}

/// Marker for one allegiance-marker layer (one per non-player ship).
#[derive(Component, Debug, Clone, Reflect)]
pub struct AllegianceMarkerHudMarker;

/// The ship entity a marker layer (or its triangle) tracks.
#[derive(Component, Debug, Clone, Copy, Deref, DerefMut, Reflect)]
pub struct AllegianceMarkerTargetEntity(pub Entity);

/// Marker for the triangle node itself (the recolour system's target). It
/// also carries [`AllegianceMarkerTargetEntity`] so recolour can map a
/// changed ship straight to its triangle without walking the layer tree.
#[derive(Component, Debug, Clone, Reflect)]
pub struct AllegianceMarkerTriangleMarker;

/// The down-pointing filled triangle node: a zero-content `ContentBox` node
/// whose top border is the allegiance colour and whose left/right borders are
/// transparent, so Bevy's mitered border shader draws it as a filled triangle
/// pointing down. See the module docs for why `ContentBox` is load-bearing.
fn allegiance_triangle_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        top: Val::Px(0.0),
        // Zero CONTENT box: the borders below become the whole visible shape.
        width: Val::Px(0.0),
        height: Val::Px(0.0),
        box_sizing: BoxSizing::ContentBox,
        border: UiRect {
            left: Val::Px(TRIANGLE_HALF_WIDTH_PX),
            right: Val::Px(TRIANGLE_HALF_WIDTH_PX),
            top: Val::Px(TRIANGLE_HEIGHT_PX),
            bottom: Val::Px(0.0),
        },
        ..default()
    }
}

/// The border colours for a triangle of allegiance `color`: only the top
/// border is painted (the visible triangle), the rest transparent.
fn allegiance_triangle_border(color: Color) -> BorderColor {
    BorderColor {
        top: color,
        left: Color::NONE,
        right: Color::NONE,
        bottom: Color::NONE,
    }
}

/// UI bundle for one ship's allegiance-marker layer: the screen-projected
/// indicator anchored to the ship, floated above the hull and hidden
/// off-screen, with the coloured triangle as its content.
fn allegiance_marker_hud(ship: Entity, color: Color) -> impl Bundle {
    (
        Name::new("AllegianceMarkerHUD"),
        AllegianceMarkerHudMarker,
        AllegianceMarkerTargetEntity(ship),
        HudTier::Instrument,
        screen_indicator_layer(),
        children![(
            Name::new("AllegianceMarkerUI"),
            screen_indicator(ScreenIndicatorConfig {
                anchor: Some(ScreenIndicatorAnchorKind::Entity(ship)),
                size: ScreenIndicatorSize::Fixed(MARKER_SIZE),
                offset: MARKER_OFFSET,
                offscreen: ScreenIndicatorOffscreen::Hide,
            }),
            children![(
                Name::new("AllegianceMarkerTriangle"),
                AllegianceMarkerTriangleMarker,
                AllegianceMarkerTargetEntity(ship),
                allegiance_triangle_node(),
                allegiance_triangle_border(color),
                Pickable::IGNORE,
            )],
        )],
    )
}

/// Draws an allegiance-coloured triangle above every non-player ship
/// (Instrument tier). Registers the marker types, adds the spawn/despawn and
/// player-skip observers, and runs `recolor_allegiance_markers` in Update
/// within [`super::NovaHudSystems`].
#[derive(Default)]
pub struct AllegianceMarkerHudPlugin;

impl Plugin for AllegianceMarkerHudPlugin {
    fn build(&self, app: &mut App) {
        debug!("AllegianceMarkerHudPlugin: build");

        app.register_type::<AllegianceMarkerHudMarker>();
        app.register_type::<AllegianceMarkerTargetEntity>();
        app.register_type::<AllegianceMarkerTriangleMarker>();

        app.add_observer(setup_allegiance_marker);
        app.add_observer(remove_allegiance_marker);
        app.add_systems(
            Update,
            (
                despawn_player_allegiance_markers,
                recolor_allegiance_markers,
            )
                .in_set(super::NovaHudSystems),
        );
    }
}

/// Every ship grows a marker the moment it spawns, grey by default. The
/// colour is fixed up by `recolor_allegiance_markers` the instant the ship's
/// `Allegiance` lands (see the module docs: it is not present yet here). The
/// player's marker is removed by `despawn_player_allegiance_markers` once its
/// `PlayerSpaceshipMarker` lands.
fn setup_allegiance_marker(add: On<Add, SpaceshipRootMarker>, mut commands: Commands) {
    let ship = add.entity;
    debug!("setup_allegiance_marker: ship {:?}", ship);
    commands.spawn(allegiance_marker_hud(ship, NEUTRAL_GREY));
}

/// The marker layer dies with its ship (death despawns the ship, scenario
/// unload - any removal path).
fn remove_allegiance_marker(
    remove: On<Remove, SpaceshipRootMarker>,
    mut commands: Commands,
    q_layers: Query<(Entity, &AllegianceMarkerTargetEntity), With<AllegianceMarkerHudMarker>>,
) {
    let ship = remove.entity;
    for (layer, target) in &q_layers {
        if **target == ship {
            trace!("remove_allegiance_marker: despawning layer {:?}", layer);
            commands.entity(layer).despawn();
        }
    }
}

/// The player's own ship shows no marker (you know where you are, and the
/// screen centre stays clean). This is a SYSTEM, not an `Add` observer,
/// deliberately: `PlayerSpaceshipMarker` and `SpaceshipRootMarker` can land
/// in one spawn (the marker `#[require]`s the root), and an `Add` observer
/// would run before `setup_allegiance_marker`'s deferred spawn command has
/// flushed - it would find no layer to despawn and the player would keep its
/// marker. Running on `Added<PlayerSpaceshipMarker>` in Update instead defers
/// past that flush, so the just-spawned layer is always visible to despawn.
/// The indicator starts hidden and this runs the same frame, so nothing
/// flashes at screen centre.
fn despawn_player_allegiance_markers(
    q_new_players: Query<Entity, Added<PlayerSpaceshipMarker>>,
    q_layers: Query<(Entity, &AllegianceMarkerTargetEntity), With<AllegianceMarkerHudMarker>>,
    mut commands: Commands,
) {
    for ship in &q_new_players {
        for (layer, target) in &q_layers {
            if **target == ship {
                debug!(
                    "despawn_player_allegiance_markers: despawning the player's marker {:?}",
                    layer
                );
                commands.entity(layer).despawn();
            }
        }
    }
}

/// Recolour a ship's triangle whenever its `Allegiance` changes. `Changed`
/// fires when the requirement-default allegiance lands at spawn-settle (so
/// the grey spawn default becomes the real colour) AND on a runtime
/// `SetAllegiance` flip (a neutral-until-provoked hauler turning red). A
/// ship that never carries an `Allegiance` keeps the grey spawn default.
fn recolor_allegiance_markers(
    q_ships: Query<(Entity, &Allegiance), (Changed<Allegiance>, With<SpaceshipRootMarker>)>,
    mut q_triangles: Query<
        (&AllegianceMarkerTargetEntity, &mut BorderColor),
        With<AllegianceMarkerTriangleMarker>,
    >,
) {
    for (ship, allegiance) in &q_ships {
        let color = allegiance_color(Some(allegiance));
        // One triangle per ship, so stop as soon as it is found.
        for (target, mut border) in &mut q_triangles {
            if **target == ship {
                if border.top != color {
                    border.top = color;
                }
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allegiance_color_maps_every_side() {
        assert_eq!(allegiance_color(Some(&Allegiance::Player)), ALLY_GREEN);
        assert_eq!(allegiance_color(Some(&Allegiance::Enemy)), THREAT_RED);
        assert_eq!(allegiance_color(Some(&Allegiance::Neutral)), NEUTRAL_GREY);
        // A ship with no Allegiance at all reads neutral, like a bystander.
        assert_eq!(allegiance_color(None), NEUTRAL_GREY);
    }

    /// The triangle is a zero-CONTENT `ContentBox` node with a coloured top
    /// border and transparent sides - the CSS-border-triangle shape. If Bevy
    /// defaulted this to `BorderBox` the 0x0 node would collapse its borders,
    /// so pin the box sizing and the border geometry explicitly.
    #[test]
    fn triangle_node_is_a_content_box_border_triangle() {
        let node = allegiance_triangle_node();
        assert_eq!(node.box_sizing, BoxSizing::ContentBox);
        assert_eq!(node.width, Val::Px(0.0));
        assert_eq!(node.height, Val::Px(0.0));
        assert_eq!(node.border.top, Val::Px(TRIANGLE_HEIGHT_PX));
        assert_eq!(node.border.left, Val::Px(TRIANGLE_HALF_WIDTH_PX));
        assert_eq!(node.border.right, Val::Px(TRIANGLE_HALF_WIDTH_PX));
        assert_eq!(node.border.bottom, Val::Px(0.0));

        // Only the top border is painted; the sides stay transparent so the
        // mitered shader leaves a single downward triangle.
        let border = allegiance_triangle_border(THREAT_RED);
        assert_eq!(border.top, THREAT_RED);
        assert_eq!(border.left, Color::NONE);
        assert_eq!(border.right, Color::NONE);
        assert_eq!(border.bottom, Color::NONE);
    }

    // -- App-driven, production-faithful behaviour --

    fn marker_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AllegianceMarkerHudPlugin);
        app
    }

    /// The ship entities that currently carry a marker layer.
    fn marker_targets(app: &mut App) -> Vec<Entity> {
        app.world_mut()
            .query_filtered::<&AllegianceMarkerTargetEntity, With<AllegianceMarkerHudMarker>>()
            .iter(app.world())
            .map(|target| **target)
            .collect()
    }

    /// The top-border (visible) colour of a ship's triangle, if it has one.
    fn triangle_color(app: &mut App, ship: Entity) -> Option<Color> {
        let colors: Vec<(Entity, Color)> = app
            .world_mut()
            .query_filtered::<(&AllegianceMarkerTargetEntity, &BorderColor), With<AllegianceMarkerTriangleMarker>>()
            .iter(app.world())
            .map(|(target, border)| (**target, border.top))
            .collect();
        colors
            .into_iter()
            .find(|(target, _)| *target == ship)
            .map(|(_, color)| color)
    }

    /// The whole feature through the real observers and systems: the player
    /// (spawned via its real marker, whose `#[require]` reproduces the
    /// root-then-player spawn ordering) ends with NO marker; enemy / friendly
    /// / neutral / no-allegiance ships each get exactly one, correctly
    /// coloured; a runtime allegiance flip recolours; and a despawn takes the
    /// marker with it.
    #[test]
    fn markers_track_allegiance_and_skip_the_player() {
        let mut app = marker_app();

        let player = app.world_mut().spawn(PlayerSpaceshipMarker).id();
        let enemy = app
            .world_mut()
            .spawn((SpaceshipRootMarker, Allegiance::Enemy))
            .id();
        // An AI wingman on the player's side: Allegiance::Player but NOT the
        // player's own ship, so it gets a green marker.
        let friendly = app
            .world_mut()
            .spawn((SpaceshipRootMarker, Allegiance::Player))
            .id();
        let neutral = app
            .world_mut()
            .spawn((SpaceshipRootMarker, Allegiance::Neutral))
            .id();
        // A ship carrying no Allegiance at all (a controller-None bystander).
        let bystander = app.world_mut().spawn(SpaceshipRootMarker).id();
        // An AI hostile spawned via its real marker: `Allegiance::Enemy`
        // arrives through the marker's `#[require]` default, mirroring how
        // production spawns enemies - it must recolour red once that default
        // lands, not stay grey.
        let ai_enemy = app.world_mut().spawn(AISpaceshipMarker).id();

        // Settle: the spawn observer's deferred layers flush, then the
        // despawn-player and recolour systems run.
        app.update();
        app.update();

        // One marker per NON-player ship; the player has none.
        let targets = marker_targets(&mut app);
        assert!(
            !targets.contains(&player),
            "the player's own ship shows no marker"
        );
        for ship in [enemy, friendly, neutral, bystander, ai_enemy] {
            assert_eq!(
                targets.iter().filter(|target| **target == ship).count(),
                1,
                "exactly one marker for ship {ship:?}"
            );
        }

        // Correct colour per allegiance, including the AI hostile whose
        // Enemy allegiance came from the marker requirement.
        assert_eq!(triangle_color(&mut app, ai_enemy), Some(THREAT_RED));
        assert_eq!(triangle_color(&mut app, enemy), Some(THREAT_RED));
        assert_eq!(triangle_color(&mut app, friendly), Some(ALLY_GREEN));
        assert_eq!(triangle_color(&mut app, neutral), Some(NEUTRAL_GREY));
        assert_eq!(
            triangle_color(&mut app, bystander),
            Some(NEUTRAL_GREY),
            "a ship with no allegiance stays grey"
        );

        // A runtime SetAllegiance-style flip recolours the triangle: the
        // neutral hauler is provoked and turns hostile.
        app.world_mut()
            .entity_mut(neutral)
            .insert(Allegiance::Enemy);
        app.update();
        assert_eq!(
            triangle_color(&mut app, neutral),
            Some(THREAT_RED),
            "a provoked ship recolours to threat red"
        );

        // A despawned ship takes its marker with it (the death / unload path).
        app.world_mut().entity_mut(enemy).despawn();
        app.update();
        assert!(
            !marker_targets(&mut app).contains(&enemy),
            "a despawned ship's marker is gone"
        );
    }
}
