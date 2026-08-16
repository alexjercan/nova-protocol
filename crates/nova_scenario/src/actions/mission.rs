//! Mission-surface actions: objectives, story lines, HUD readouts, and the
//! marker/keybind emphasis a beat points the player at.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_hud::prelude::*;

use crate::prelude::*;

/// A scenario action that adds an objective to the HUD.
///
/// The objective *data* (id + message) is `nova_gameplay`'s `Objective`, but
/// this scenario-action wrapper stays nova-local because it implements the (foreign)
/// `EventAction` trait - which the orphan rule forbids implementing on the foreign
/// `Objective` type directly.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObjectiveActionConfig {
    /// Opaque identifier, used to complete/remove the objective later.
    pub id: String,
    /// The text shown in the objectives HUD.
    pub message: String,
}

impl ObjectiveActionConfig {
    /// Construct from string slices.
    pub fn new(id: &str, message: &str) -> Self {
        Self {
            id: id.to_string(),
            message: message.to_string(),
        }
    }
}

impl EventAction<NovaEventWorld> for ObjectiveActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        world.push_objective(self.clone());
    }
}

/// One speaker-attributed story line for the HUD comms panel. Appends to the
/// event world's story log; the log is scenario-scoped (cleared at teardown
/// with the rest of the event world), so a line can never leak into the next
/// scenario or the menu. RON: `StoryMessage((speaker: "Alpha", text:
/// "Strip it clean."))`. Optionally add `dwell: Some(12.0)` for a longer hold
/// and `icon: Some("self://icons/alpha.png")` for a speaker image. Strict RON
/// uses `Some`; omit the field for the HUD fallback icon.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StoryMessageActionConfig {
    /// Who says it (the panel renders it as the line's prefix).
    pub speaker: String,
    /// The line itself.
    pub text: String,
    /// Optional on-screen hold override in seconds. Strict RON: `dwell:
    /// Some(12.0)`, never a bare number; omit the field for the default (8s).
    /// The panel clamps to [3, 30] at use; content_lint warns on an authored
    /// value outside that range.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub dwell: Option<f32>,
    /// Optional speaker icon image for the comms stack. Strict RON:
    /// `icon: Some("self://icons/voice.png")`, never a bare string. Omit or
    /// write `None` for the HUD fallback tile.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub icon: Option<AssetRef<Image>>,
}

impl EventAction<NovaEventWorld> for StoryMessageActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        world.push_story_message(self.clone());
    }
}

/// How a [`HudReadoutActionConfig`] renders its bound variable on the HUD. Maps
/// one-to-one onto nova_gameplay's `HudReadoutFormat` at sync time (the HUD
/// cannot depend on nova_scenario, so the enum is mirrored, the same split as
/// `StoryMessageActionConfig` -> `StoryLine`). The `Config` suffix is what keeps
/// the two halves distinguishable: nova_core globs both crates' preludes, so a
/// shared name would be an ambiguous glob re-export and force the sync to reach
/// for the render enum by its full path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HudReadoutFormatConfig {
    /// One decimal place, e.g. `12.3`.
    #[default]
    Number,
    /// No decimals (rounded), e.g. `12`.
    Integer,
    /// Minutes and seconds, `mm:ss.s` (e.g. `01:23.4`) - the time-trial clock.
    Time,
}

impl From<HudReadoutFormatConfig> for HudReadoutFormat {
    fn from(config: HudReadoutFormatConfig) -> Self {
        match config {
            HudReadoutFormatConfig::Number => HudReadoutFormat::Number,
            HudReadoutFormatConfig::Integer => HudReadoutFormat::Integer,
            HudReadoutFormatConfig::Time => HudReadoutFormat::Time,
        }
    }
}

/// Show, update, or clear a named HUD readout bound to a scenario variable -
/// the DISPLAY half of the scenario-variable vocabulary. The timekeeping half
/// already exists: `scenario_elapsed` (and any authored variable) lives on the
/// event world; this action is what finally puts one on the HUD. Generic on
/// purpose (per the spike): any mod can surface any variable (a score, a
/// countdown, a lap time), not just a run clock.
///
/// A readout is identified by its `slot`. Firing the action with `visible:
/// true` shows or updates that slot; the HUD then tracks the bound variable's
/// CURRENT value every frame (read at sync time), so a single fire from the
/// start gate is enough for a live clock. `visible: false` clears just that
/// slot. Every readout also clears automatically at scenario teardown, exactly
/// like the comms panel, so one cannot leak into the next scenario or the menu.
///
/// The value freezes on pause and behind the outcome overlay because
/// `scenario_elapsed` freezes there - a time-trial's FINAL time simply holds,
/// frozen, on the HUD through the Victory banner.
///
/// RON: `HudReadout((slot: "timer", variable: "scenario_elapsed", format: Time,
/// label: Some("TIME")))`; clear with `HudReadout((slot: "timer", variable:
/// "scenario_elapsed", visible: false))`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HudReadoutActionConfig {
    /// The readout's stable id: shows/updates/clears this one slot, and lets a
    /// scenario run several readouts side by side.
    pub slot: String,
    /// The scenario variable whose value the readout shows (e.g.
    /// `"scenario_elapsed"`). Read live off the event world every frame.
    pub variable: String,
    /// How the value renders. Omit for the default ([`HudReadoutFormatConfig::Number`]).
    #[cfg_attr(feature = "serde", serde(default))]
    pub format: HudReadoutFormatConfig,
    /// Optional caption shown before the value (e.g. `"TIME"`).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub label: Option<String>,
    /// `true` (the default) shows/updates the slot; `false` clears it.
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub visible: bool,
}

/// Serde default for [`HudReadoutActionConfig::visible`]: a readout with the
/// field omitted is shown, not hidden.
#[cfg(feature = "serde")]
fn default_true() -> bool {
    true
}

impl HudReadoutActionConfig {
    /// Construct a shown readout (`visible: true`) with the default format and
    /// no label.
    pub fn new(slot: &str, variable: &str) -> Self {
        Self {
            slot: slot.to_string(),
            variable: variable.to_string(),
            format: HudReadoutFormatConfig::default(),
            label: None,
            visible: true,
        }
    }
}

impl EventAction<NovaEventWorld> for HudReadoutActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        world.set_hud_readout(self.clone());
    }
}

/// Action that completes (removes) the HUD objective with the given id.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObjectiveCompleteActionConfig {
    /// The id of the objective to complete.
    pub id: String,
}

impl EventAction<NovaEventWorld> for ObjectiveCompleteActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        world.remove_objective(&self.id);
    }
}

/// Attach the gold objective marker to the scenario object whose [`EntityId`]
/// matches `target_id`: inserts [`ObjectiveMarkerTarget`] with `label`, and the
/// HUD's objective-markers observer grows the chip. Scoped-only lookup, same
/// rule as DespawnScenarioObject. Attaching to an already-marked entity just
/// updates the label.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObjectiveMarkerAttachActionConfig {
    /// The `EntityId` of the scoped object the marker chip attaches to.
    pub target_id: String,
    /// The short name the marker chip shows next to the distance.
    pub label: String,
}

impl ObjectiveMarkerAttachActionConfig {
    /// Construct from string slices.
    pub fn new(target_id: &str, label: &str) -> Self {
        Self {
            target_id: target_id.to_string(),
            label: label.to_string(),
        }
    }
}

impl EventAction<NovaEventWorld> for ObjectiveMarkerAttachActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.target_id.clone();
        let label = self.label.clone();
        debug!("ObjectiveMarkerAttach: '{}' <- '{}'", id, label);

        // Same shape as DespawnScenarioObject: the id lookup needs world
        // access, so the queued command resolves and inserts in one step -
        // which also means an attach ordered after a spawn in the same
        // handler sees the freshly spawned entity.
        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let mut query =
                    world.query_filtered::<(Entity, &EntityId), With<ScenarioScopedMarker>>();
                let matches: Vec<Entity> = query
                    .iter(world)
                    .filter(|(_, entity_id)| entity_id.0 == id)
                    .map(|(entity, _)| entity)
                    .collect();
                if matches.is_empty() {
                    warn!(
                        "ObjectiveMarkerAttach: no scoped entity with id '{}'; check the \
                         scenario for a typo or an attach before the spawn",
                        id
                    );
                }
                for entity in matches {
                    if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
                        entity_mut.insert(ObjectiveMarkerTarget::new(&label));
                    }
                }
            });
        });
    }
}

/// Detach the objective marker from the scenario object whose [`EntityId`]
/// matches `target_id` (no-op with a warning when nothing matches; a
/// marker whose entity despawned is already detached - the chip died with
/// it).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObjectiveMarkerDetachActionConfig {
    /// The `EntityId` of the scoped object to detach the marker chip from.
    pub target_id: String,
}

impl ObjectiveMarkerDetachActionConfig {
    /// Construct from a string slice.
    pub fn new(target_id: &str) -> Self {
        Self {
            target_id: target_id.to_string(),
        }
    }
}

impl EventAction<NovaEventWorld> for ObjectiveMarkerDetachActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.target_id.clone();
        debug!("ObjectiveMarkerDetach: '{}'", id);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let mut query =
                    world.query_filtered::<(Entity, &EntityId), With<ScenarioScopedMarker>>();
                let matches: Vec<Entity> = query
                    .iter(world)
                    .filter(|(_, entity_id)| entity_id.0 == id)
                    .map(|(entity, _)| entity)
                    .collect();
                if matches.is_empty() {
                    // Quieter than attach: detaching an entity that already
                    // despawned (crate picked up) is a legitimate script
                    // shape, not necessarily a typo.
                    debug!("ObjectiveMarkerDetach: no scoped entity with id '{}'", id);
                }
                for entity in matches {
                    if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
                        entity_mut.remove::<ObjectiveMarkerTarget>();
                    }
                }
            });
        });
    }
}

/// Emphasize one keybind-dock chip: pushes `verb` into nova_gameplay's
/// [`HintEmphasis`] resource, so the dock pulses that chip toward objective
/// gold until a `HintEmphasisClear` (or scenario teardown) drops it. Only
/// `DOCK_VERBS` names are valid; the resource refuses unknown verbs with a
/// warning.
///
/// The dock hides verbs the player cannot use, so emphasizing an unavailable
/// verb REVEALS its chip and pulses it in the dim band - that is how a tutorial
/// points at a key before it lights up. Emphasis is still a spotlight, never a
/// grant: it does not make the verb pressable.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HintEmphasisSetActionConfig {
    /// The keybind-dock chip to emphasize (one of `DOCK_VERBS`).
    pub verb: String,
}

impl HintEmphasisSetActionConfig {
    /// Construct from a string slice.
    pub fn new(verb: &str) -> Self {
        Self {
            verb: verb.to_string(),
        }
    }
}

impl EventAction<NovaEventWorld> for HintEmphasisSetActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let verb = self.verb.clone();
        debug!("HintEmphasisSet: '{}'", verb);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                // get_resource_mut, not resource_mut: headless rigs that
                // exercise scenario scripts without the HUD plugins have no
                // emphasis resource, and the action must not panic there.
                let Some(mut emphasis) = world.get_resource_mut::<HintEmphasis>() else {
                    warn!("HintEmphasisSet: no HintEmphasis resource (HUD not loaded)");
                    return;
                };
                emphasis.set(&verb);
            });
        });
    }
}

/// Drop the emphasis on one keybind-dock chip (see [`HintEmphasisSetActionConfig`]).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HintEmphasisClearActionConfig {
    /// The keybind-dock chip to clear (one of `DOCK_VERBS`).
    pub verb: String,
}

impl HintEmphasisClearActionConfig {
    /// Construct from a string slice.
    pub fn new(verb: &str) -> Self {
        Self {
            verb: verb.to_string(),
        }
    }
}

impl EventAction<NovaEventWorld> for HintEmphasisClearActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let verb = self.verb.clone();
        debug!("HintEmphasisClear: '{}'", verb);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let Some(mut emphasis) = world.get_resource_mut::<HintEmphasis>() else {
                    return;
                };
                emphasis.clear(&verb);
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The authored RON shape parses and round-trips - the exact syntax the
    /// authoring guide documents: `StoryMessage((speaker:..., text:...))`, with
    /// `dwell` OMITTED defaulting to None and the documented strict-RON `dwell:
    /// Some(12.0)` parsing.
    #[cfg(feature = "serde")]
    #[test]
    fn story_message_ron_round_trips() {
        let authored = r#"StoryMessage((speaker: "Alpha", text: "Quota's quota."))"#;
        let parsed: EventActionConfig = ron::from_str(authored).expect("authored RON parses");
        let EventActionConfig::StoryMessage(config) = &parsed else {
            panic!("parsed the StoryMessage variant");
        };
        assert_eq!(config.speaker, "Alpha");
        assert_eq!(config.text, "Quota's quota.");
        assert_eq!(config.dwell, None, "omitted dwell defaults to None");

        let with_dwell = r#"StoryMessage((speaker: "Alpha", text: "Slowly.", dwell: Some(12.0)))"#;
        let parsed_dwell: EventActionConfig =
            ron::from_str(with_dwell).expect("the documented dwell syntax parses");
        let EventActionConfig::StoryMessage(config_dwell) = &parsed_dwell else {
            panic!("parsed the StoryMessage variant");
        };
        assert_eq!(config_dwell.dwell, Some(12.0));

        let ron = ron::to_string(&parsed).expect("serializes");
        let back: EventActionConfig = ron::from_str(&ron).expect("round-trips");
        let EventActionConfig::StoryMessage(again) = back else {
            panic!("round-tripped the StoryMessage variant");
        };
        assert_eq!(&again, config);
    }

    /// StoryMessage icons are optional authorable image refs: omitted stays
    /// `None` for back-compat, while strict RON `Some("self://...")` /
    /// `Some("dep://...")` round-trip as AssetRef paths.
    #[cfg(feature = "serde")]
    #[test]
    fn story_message_icon_ron_round_trips() {
        let legacy = r#"StoryMessage((speaker: "Alpha", text: "Quota's quota."))"#;
        let parsed: EventActionConfig = ron::from_str(legacy).expect("legacy RON parses");
        let EventActionConfig::StoryMessage(config) = &parsed else {
            panic!("parsed the StoryMessage variant");
        };
        assert_eq!(config.icon, None, "omitted icon defaults to None");

        let with_self = r#"StoryMessage((speaker: "Alpha", text: "Face.", icon: Some("self://icons/alpha.png")))"#;
        let parsed_self: EventActionConfig =
            ron::from_str(with_self).expect("self icon syntax parses");
        let EventActionConfig::StoryMessage(config_self) = &parsed_self else {
            panic!("parsed the StoryMessage variant");
        };
        assert_eq!(
            config_self.icon.as_ref().and_then(|icon| icon.path()),
            Some("self://icons/alpha.png")
        );

        let with_dep = r#"StoryMessage((speaker: "Relay", text: "Shared.", icon: Some("dep://base/icons/comms.png")))"#;
        let parsed_dep: EventActionConfig =
            ron::from_str(with_dep).expect("dep icon syntax parses");
        let EventActionConfig::StoryMessage(config_dep) = &parsed_dep else {
            panic!("parsed the StoryMessage variant");
        };
        assert_eq!(
            config_dep.icon.as_ref().and_then(|icon| icon.path()),
            Some("dep://base/icons/comms.png")
        );

        let ron = ron::to_string(&parsed_self).expect("serializes");
        assert!(
            ron.contains("icon:Some(\"self://icons/alpha.png\")"),
            "ron: {ron}"
        );
        let back: EventActionConfig = ron::from_str(&ron).expect("round-trips");
        let EventActionConfig::StoryMessage(again) = back else {
            panic!("round-tripped the StoryMessage variant");
        };
        assert_eq!(again, *config_self);
    }

    /// The authored `HudReadout` RON shapes parse and round-trip: the shown
    /// form with a format + label, the omitted `format`/`label`/`visible`
    /// defaults (Number / None / true), and the clear form (`visible: false`).
    #[cfg(feature = "serde")]
    #[test]
    fn hud_readout_ron_round_trips() {
        let shown = r#"HudReadout((slot: "timer", variable: "scenario_elapsed", format: Time, label: Some("TIME")))"#;
        let parsed: EventActionConfig = ron::from_str(shown).expect("shown RON parses");
        let EventActionConfig::HudReadout(config) = &parsed else {
            panic!("parsed the HudReadout variant");
        };
        assert_eq!(config.slot, "timer");
        assert_eq!(config.variable, "scenario_elapsed");
        assert_eq!(config.format, HudReadoutFormatConfig::Time);
        assert_eq!(config.label.as_deref(), Some("TIME"));
        assert!(config.visible, "visible defaults to true when omitted");

        let minimal = r#"HudReadout((slot: "score", variable: "score"))"#;
        let parsed_min: EventActionConfig = ron::from_str(minimal).expect("minimal RON parses");
        let EventActionConfig::HudReadout(config_min) = &parsed_min else {
            panic!("parsed the HudReadout variant");
        };
        assert_eq!(
            config_min.format,
            HudReadoutFormatConfig::Number,
            "omitted format defaults to Number"
        );
        assert_eq!(config_min.label, None);
        assert!(config_min.visible);

        let cleared =
            r#"HudReadout((slot: "timer", variable: "scenario_elapsed", visible: false))"#;
        let parsed_clear: EventActionConfig = ron::from_str(cleared).expect("clear RON parses");
        let EventActionConfig::HudReadout(config_clear) = &parsed_clear else {
            panic!("parsed the HudReadout variant");
        };
        assert!(
            !config_clear.visible,
            "the clear form parses visible: false"
        );

        let ron = ron::to_string(&parsed).expect("serializes");
        let back: EventActionConfig = ron::from_str(&ron).expect("round-trips");
        let EventActionConfig::HudReadout(again) = back else {
            panic!("round-tripped the HudReadout variant");
        };
        assert_eq!(&again, config);
    }

    /// The `HudReadout` action's EFFECT through the production drain: the
    /// action upserts a readout on the event world, and the sync mirrors it -
    /// with the bound variable's CURRENT value - into the HUD's `HudReadouts`
    /// resource. A `visible: false` fire clears the slot.
    #[test]
    fn hud_readout_action_syncs_and_clears_through_the_drain() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<HudReadouts>();

        // Show a Time readout bound to a variable, and set that variable.
        {
            let mut world = app.world_mut().resource_mut::<NovaEventWorld>();
            world.insert_variable(
                "scenario_elapsed".to_string(),
                VariableLiteral::Number(83.4),
            );
            let show = EventActionConfig::HudReadout(HudReadoutActionConfig {
                slot: "timer".to_string(),
                variable: "scenario_elapsed".to_string(),
                format: HudReadoutFormatConfig::Time,
                label: Some("TIME".to_string()),
                visible: true,
            });
            show.action(&mut world, &GameEventInfo { data: None });
        }
        NovaEventWorld::state_to_world_system(app.world_mut());

        let readouts = app.world().resource::<HudReadouts>();
        assert_eq!(readouts.0.len(), 1, "the shown readout synced");
        assert_eq!(readouts.0[0].slot, "timer");
        assert_eq!(readouts.0[0].value, 83.4, "the live variable value synced");

        // Clear it.
        {
            let mut world = app.world_mut().resource_mut::<NovaEventWorld>();
            let clear = EventActionConfig::HudReadout(HudReadoutActionConfig {
                slot: "timer".to_string(),
                variable: "scenario_elapsed".to_string(),
                format: HudReadoutFormatConfig::Time,
                label: None,
                visible: false,
            });
            clear.action(&mut world, &GameEventInfo { data: None });
        }
        NovaEventWorld::state_to_world_system(app.world_mut());
        assert!(
            app.world().resource::<HudReadouts>().0.is_empty(),
            "the clear fire dropped the slot"
        );
    }

    /// The marker attach/detach pair drives the [`ObjectiveMarkerTarget`]
    /// component on exactly the scoped object with the id - unscoped
    /// entities with colliding ids (ship sections) are never marked, and a
    /// re-attach updates the label in place.
    #[test]
    fn objective_marker_attach_and_detach_drive_the_component() {
        use nova_events::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        let beacon = world
            .spawn((ScenarioScopedMarker, EntityId::new("beacon_1".to_string())))
            .id();
        let section = world.spawn(EntityId::new("beacon_1".to_string())).id();

        let attach = ObjectiveMarkerAttachActionConfig::new("beacon_1", "BEACON 1");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        attach.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        assert_eq!(
            world
                .get::<ObjectiveMarkerTarget>(beacon)
                .map(|marker| marker.label.as_str()),
            Some("BEACON 1"),
            "the scoped object is marked"
        );
        assert!(
            world.get::<ObjectiveMarkerTarget>(section).is_none(),
            "an unscoped entity with the same id (a ship section) is never marked"
        );

        // Re-attach updates the label in place (no detach needed between).
        let relabel = ObjectiveMarkerAttachActionConfig::new("beacon_1", "NEXT");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        relabel.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);
        assert_eq!(
            world
                .get::<ObjectiveMarkerTarget>(beacon)
                .map(|marker| marker.label.as_str()),
            Some("NEXT")
        );

        let detach = ObjectiveMarkerDetachActionConfig::new("beacon_1");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        detach.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);
        assert!(
            world.get::<ObjectiveMarkerTarget>(beacon).is_none(),
            "detach removes the marker"
        );
    }

    /// Attach/detach against a missing id must warn and complete, not
    /// crash - the detach-after-despawn shape is legitimate script data
    /// (crate picked up before its detach action runs).
    #[test]
    fn objective_marker_actions_with_missing_id_are_harmless() {
        use nova_events::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();
        let bystander = world
            .spawn((ScenarioScopedMarker, EntityId::new("beacon_1".to_string())))
            .id();

        for action in [
            EventActionConfig::ObjectiveMarkerAttach(ObjectiveMarkerAttachActionConfig::new(
                "no_such_id",
                "GHOST",
            )),
            EventActionConfig::ObjectiveMarkerDetach(ObjectiveMarkerDetachActionConfig::new(
                "no_such_id",
            )),
        ] {
            let mut event_world = world.resource_mut::<NovaEventWorld>();
            action.action(&mut event_world, &GameEventInfo::default());
            NovaEventWorld::state_to_world_system(&mut world);
        }

        assert!(world.get_entity(bystander).is_ok());
        assert!(world.get::<ObjectiveMarkerTarget>(bystander).is_none());
    }

    /// The emphasis pair mutates nova_gameplay's HintEmphasis resource
    /// through the queued-command drain; without the resource (headless
    /// scenario rigs) both are warn-and-continue no-ops.
    #[test]
    fn hint_emphasis_actions_drive_the_resource() {
        use nova_events::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        // Without the resource: harmless.
        let set = HintEmphasisSetActionConfig::new("GOTO");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        set.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        // With it: set lands, clear drops.
        world.init_resource::<HintEmphasis>();
        let set = HintEmphasisSetActionConfig::new("GOTO");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        set.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);
        assert!(world.resource::<HintEmphasis>().contains("GOTO"));

        let clear = HintEmphasisClearActionConfig::new("GOTO");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        clear.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);
        assert!(!world.resource::<HintEmphasis>().contains("GOTO"));
    }
}
