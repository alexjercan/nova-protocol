//! Item highlight brackets (task 20260712-093831, spike
//! docs/spikes/20260712-140842-objective-conveyance-visuals.md): a hollow
//! bracket over every [`ItemHighlight`] entity, so interactable props
//! (salvage crates, future pickups) pop against the debris around them.
//! The bracket tracks the prop's on-screen size via the component's
//! AUTHORED visible radius (`ScreenIndicatorSize::WorldRadius`) - NOT
//! collider-derived `ApparentSize`, because a pickup's only collider is
//! its sensor sphere and the bracket would balloon to the trigger volume
//! (review R1.1) - and hides off-screen: pointing at off-screen items is
//! the objective marker's job, and the viewport edges stay reserved for
//! threats and the active objective.
//!
//! The bracket's alpha breathes in step with the crate's emissive pulse
//! (same period, nova_scenario's salvage module), so mesh and HUD read as
//! one system.
//!
//! Chrome tier: a learning aid, not a flight instrument.

use bevy::prelude::*;

use super::{screen_indicator::prelude::*, HudTier};
use crate::prelude::*;

pub mod prelude {
    pub use super::{
        ItemHighlightBracketMarker, ItemHighlightHudMarker, ItemHighlightTargetEntity,
        ItemHighlightsHudPlugin, ITEM_HIGHLIGHT_PULSE_PERIOD_SECS,
    };
}

/// Minimum (and fallback) on-screen size of a highlight bracket (px).
const BRACKET_MIN_PX: f32 = 28.0;

/// Bracket border thickness (px).
const BRACKET_BORDER_PX: f32 = 1.5;

/// Bracket tint: the crate's own orange, so the bracket reads as the prop
/// asserting itself rather than a new HUD voice.
const BRACKET_COLOR: Color = Color::srgba(1.0, 0.75, 0.15, 0.8);

/// Shared pulse period (seconds) for the bracket alpha AND the crate's
/// emissive sine (nova_scenario's salvage module cites this constant):
/// one clock, one visual system.
pub const ITEM_HIGHLIGHT_PULSE_PERIOD_SECS: f32 = 1.6;

/// Alpha band the bracket breath sweeps (relative to BRACKET_COLOR's own
/// alpha): visible motion, never vanishing.
const BREATH_ALPHA_MIN: f32 = 0.55;
const BREATH_ALPHA_MAX: f32 = 1.0;

/// Marker for one item highlight layer (one per highlighted entity).
#[derive(Component, Debug, Clone, Reflect)]
pub struct ItemHighlightHudMarker;

/// The highlighted entity this bracket overlays.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct ItemHighlightTargetEntity(pub Entity);

/// Marker for the bracket border node (the breath system's target).
#[derive(Component, Debug, Clone, Reflect)]
pub struct ItemHighlightBracketMarker;

/// UI bundle for one highlighted entity's bracket layer.
fn item_highlight_hud(target: Entity, world_radius: f32) -> impl Bundle {
    (
        Name::new("ItemHighlightHUD"),
        ItemHighlightHudMarker,
        ItemHighlightTargetEntity(target),
        HudTier::Chrome,
        screen_indicator_layer(),
        children![(
            Name::new("ItemHighlightUI"),
            screen_indicator(ScreenIndicatorConfig {
                anchor: Some(ScreenIndicatorAnchorKind::Entity(target)),
                size: ScreenIndicatorSize::WorldRadius {
                    radius: world_radius,
                    min_px: BRACKET_MIN_PX,
                },
                offset: Vec2::ZERO,
                offscreen: ScreenIndicatorOffscreen::Hide,
            }),
            children![(
                Name::new("ItemHighlightBracket"),
                ItemHighlightBracketMarker,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    border: UiRect::all(Val::Px(BRACKET_BORDER_PX)),
                    ..default()
                },
                BorderColor::all(BRACKET_COLOR),
                Pickable::IGNORE,
            )],
        )],
    )
}

#[derive(Default)]
pub struct ItemHighlightsHudPlugin;

impl Plugin for ItemHighlightsHudPlugin {
    fn build(&self, app: &mut App) {
        debug!("ItemHighlightsHudPlugin: build");

        app.register_type::<ItemHighlight>();

        app.add_observer(setup_item_highlight);
        app.add_observer(remove_item_highlight);
        app.add_systems(
            Update,
            breathe_item_highlights.in_set(super::NovaHudSystems),
        );
    }
}

/// Every highlighted entity grows its bracket the moment the tag lands,
/// sized to the tag's authored visible radius.
fn setup_item_highlight(
    add: On<Add, ItemHighlight>,
    q_highlight: Query<&ItemHighlight>,
    mut commands: Commands,
) {
    let target = add.entity;
    let Ok(highlight) = q_highlight.get(target) else {
        return;
    };
    debug!("setup_item_highlight: target {:?}", target);
    commands.spawn(item_highlight_hud(target, highlight.world_radius));
}

/// The bracket layer dies with its tag (pickup despawn, scenario unload -
/// any removal path).
fn remove_item_highlight(
    remove: On<Remove, ItemHighlight>,
    mut commands: Commands,
    q_highlights: Query<(Entity, &ItemHighlightTargetEntity), With<ItemHighlightHudMarker>>,
) {
    let target = remove.entity;
    for (layer, layer_target) in &q_highlights {
        if **layer_target == target {
            trace!("remove_item_highlight: despawning layer {:?}", layer);
            commands.entity(layer).despawn();
        }
    }
}

/// The breath wave at `elapsed` seconds: the alpha factor the bracket
/// border carries this frame. Shares its period with the crate emissive
/// pulse so mesh and bracket move together.
fn breath_alpha(elapsed_secs: f32) -> f32 {
    let t = elapsed_secs * std::f32::consts::TAU / ITEM_HIGHLIGHT_PULSE_PERIOD_SECS;
    let wave = 0.5 + 0.5 * t.sin();
    BREATH_ALPHA_MIN + (BREATH_ALPHA_MAX - BREATH_ALPHA_MIN) * wave
}

/// Breathe every bracket's border alpha with the shared wave.
fn breathe_item_highlights(
    time: Res<Time>,
    mut q_brackets: Query<&mut BorderColor, With<ItemHighlightBracketMarker>>,
) {
    let alpha = breath_alpha(time.elapsed_secs());
    let breathed = BRACKET_COLOR.with_alpha(BRACKET_COLOR.alpha() * alpha);
    for mut border in &mut q_brackets {
        *border = BorderColor::all(breathed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world_with_observers() -> World {
        let mut world = World::new();
        world.add_observer(setup_item_highlight);
        world.add_observer(remove_item_highlight);
        world
    }

    fn highlight_targets(world: &mut World) -> Vec<Entity> {
        world
            .query_filtered::<&ItemHighlightTargetEntity, With<ItemHighlightHudMarker>>()
            .iter(world)
            .map(|target| **target)
            .collect()
    }

    /// The bracket's indicator carries the tag's AUTHORED radius as a
    /// WorldRadius mode - never collider-derived ApparentSize, which would
    /// size to the pickup sensor (review R1.1).
    #[test]
    fn brackets_size_to_the_authored_radius() {
        let mut world = world_with_observers();
        world.spawn(ItemHighlight::new(1.3));
        world.flush();

        let sizes: Vec<ScreenIndicatorSize> = world
            .query::<&ScreenIndicatorSize>()
            .iter(&world)
            .copied()
            .collect();
        assert_eq!(
            sizes,
            vec![ScreenIndicatorSize::WorldRadius {
                radius: 1.3,
                min_px: BRACKET_MIN_PX,
            }],
            "the bracket projects the authored visible radius"
        );
    }

    /// One bracket per highlighted entity; despawning the prop (the pickup
    /// path) removes exactly its bracket.
    #[test]
    fn brackets_follow_the_highlight_lifecycle() {
        let mut world = world_with_observers();
        let a = world.spawn(ItemHighlight::new(1.3)).id();
        let b = world.spawn(ItemHighlight::new(1.3)).id();
        world.flush();

        let targets = highlight_targets(&mut world);
        assert_eq!(targets.len(), 2);
        assert!(targets.contains(&a) && targets.contains(&b));

        world.entity_mut(a).despawn();
        world.flush();

        assert_eq!(
            highlight_targets(&mut world),
            vec![b],
            "the picked-up prop's bracket dies with it, the sibling survives"
        );
    }

    /// The breath sweeps its band - a flat wave would be dead code posing
    /// as a pulse.
    #[test]
    fn breath_alpha_sweeps_its_band() {
        let mut lowest = f32::MAX;
        let mut highest = f32::MIN;
        for i in 0..100 {
            let alpha = breath_alpha(i as f32 * ITEM_HIGHLIGHT_PULSE_PERIOD_SECS / 100.0);
            assert!((BREATH_ALPHA_MIN..=BREATH_ALPHA_MAX).contains(&alpha));
            lowest = lowest.min(alpha);
            highest = highest.max(alpha);
        }
        assert!(
            highest - lowest > 0.8 * (BREATH_ALPHA_MAX - BREATH_ALPHA_MIN),
            "one period sweeps (nearly) the whole band, got [{lowest}, {highest}]"
        );
    }
}
