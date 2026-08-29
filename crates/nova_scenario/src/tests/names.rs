//! The [`Names`] field attribute, read back the way a reader reads it.

use bevy::reflect::{TypeInfo, Typed};

use crate::prelude::*;

/// What a named field of `T` says it names.
fn names<T: Typed>(field: &str) -> Option<Names> {
    let TypeInfo::Struct(info) = T::type_info() else {
        return None;
    };
    info.field(field)?.get_attribute::<Names>().copied()
}

#[test]
fn a_config_string_says_what_it_names() {
    assert_eq!(
        names::<SetAllegianceActionConfig>("id"),
        Some(Names::Object)
    );
    assert_eq!(names::<EntityFilterConfig>("id"), Some(Names::Object));
    assert_eq!(names::<EntityFilterConfig>("other_id"), Some(Names::Object));
    assert_eq!(names::<TimerStartActionConfig>("key"), Some(Names::Timer));
    assert_eq!(
        names::<VariableSetActionConfig>("key"),
        Some(Names::Variable)
    );
    assert_eq!(
        names::<NextScenarioActionConfig>("scenario_id"),
        Some(Names::Scenario)
    );
    assert_eq!(
        names::<ObjectiveCompleteActionConfig>("id"),
        Some(Names::Objective)
    );
}

#[test]
fn a_spawn_declares_its_id_and_a_reference_expects_one() {
    assert_eq!(
        names::<BaseScenarioObjectConfig>("id"),
        Some(Names::NewObject)
    );
    assert_eq!(names::<ScenarioAreaConfig>("id"), Some(Names::NewObject));
    assert_eq!(
        names::<DespawnScenarioObjectActionConfig>("id"),
        Some(Names::Object)
    );
}

#[test]
fn a_string_that_names_nothing_in_the_world_carries_no_attribute() {
    // The message of a debug line and the text of a story line are prose, and
    // an offer to complete them against the scenario's ids would be wrong.
    assert_eq!(names::<DebugMessageActionConfig>("message"), None);
    assert_eq!(names::<StoryMessageActionConfig>("text"), None);
    assert_eq!(names::<StoryMessageActionConfig>("speaker"), None);
}
