//! Generate a hull nobody drew, into the ship being edited.
//!
//! The generator is `nova_wfc`, run over the SAME merged content the rest of
//! the editor reads: a mod that ships a section changes what Generate can roll
//! and a mod that ships a grammar changes how it rolls, with nothing here
//! knowing an id. What this module owns is the seed the builder types, the
//! SECTIONS they ticked, and the REFUSAL - a collapse that fails, or a hull the
//! game's own content lint would reject, writes one line to the status and
//! leaves the document as it was.
//!
//! Generate is a SHIP verb. A ship is the unit a hull is, so the block sits
//! inside one and what it rolls replaces what that ship holds - which is what
//! makes a seed a thing you can turn until you like the answer, rather than a
//! way to fill a scenario with drafts. Every part of the result is an ordinary
//! section node afterwards: selectable, movable, deletable, and bound to the
//! key its kind answers to, so a generated ship can be flown the moment it is
//! made the player's.

use bevy::{prelude::*, ui::InteractionDisabled, ui_widgets::Activate};
use nova_scenario::prelude::ShipHull;
use nova_ship::prelude::{
    GameGrammars, GameSections, GameStyles, GrammarGrid, GrammarPart, GrammarZone,
    SectionFootprint, SectionKind, ShipGrammarConfig, STANDARD_HULL_GRAMMAR_ID,
};
use nova_ui::{
    prelude::{TextFieldError, TextFieldValue},
    widget::Selected,
};
use nova_wfc::prelude::{hull_errors, TileSet};

#[cfg(test)]
mod tests;

use crate::{
    bundle::resume_ordinal,
    config::{EditorSays, PartChoice, SelectedNode},
    node::{
        insert_lifted_section, resume_ordinals, EditContext, NextChildOrdinal, NodeId, SectionNode,
        ShipNode,
    },
    placement::default_binds,
};

/// The seed the next Generate collapses, as the builder last left it.
///
/// A resource rather than only the field's text, because two controls write it:
/// the field the builder types into and the Reroll that hands them a fresh one.
/// It starts random, so the first press of Generate is a ship rather than
/// everyone's ship zero.
#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct HullSeed(pub(crate) u64);

impl Default for HullSeed {
    fn default() -> Self {
        Self(rand::random())
    }
}

/// The grammar the next Generate collapses: which hull LINE the ship is drawn
/// from, as the builder last picked it.
///
/// A resource beside [`HullSeed`], and for the same reason: it is an input to
/// the roll rather than a property of the ship. A ship keeps its sections and
/// its cladding; the line it was rolled off is a dial the builder can turn and
/// roll again.
///
/// Starts at [`STANDARD_HULL_GRAMMAR_ID`], so a builder who never opens the
/// list gets the hull the base game draws.
#[derive(Resource, Debug, Clone)]
pub(crate) struct HullGrammar(pub(crate) String);

impl Default for HullGrammar {
    fn default() -> Self {
        Self(STANDARD_HULL_GRAMMAR_ID.to_string())
    }
}

/// The rail block that generates a hull, shown only INSIDE a ship: a hull is
/// what one ship IS, so the block sits in that ship's own settings.
#[derive(Component)]
pub(crate) struct GenerateSettings;

/// The seed field of that block.
#[derive(Component)]
pub(crate) struct HullSeedField;

/// The button that collapses one, greyed while the seed does not parse.
#[derive(Component)]
pub(crate) struct GenerateButton;

/// The message under the seed field when what is typed is not a seed.
const SEED_REFUSAL: &str = "Enter an unsigned integer";

/// Take the typed seed, and say so when it is not one.
///
/// The FIELD is what the builder edits, so it is what the check reads: parsing
/// on the press instead would leave a bad seed looking accepted until the press
/// that refused it. Generate greys with the same finding, because a button that
/// cannot work should not look like it can.
pub(crate) fn read_seed_field(
    mut commands: Commands,
    fields: Query<(Entity, &TextFieldValue), (With<HullSeedField>, Changed<TextFieldValue>)>,
    buttons: Query<Entity, With<GenerateButton>>,
    mut seed: ResMut<HullSeed>,
) {
    let Some((entity, value)) = fields.iter().next() else {
        return;
    };
    let parsed = value.0.trim().parse::<u64>();
    match parsed {
        Ok(typed) => {
            seed.0 = typed;
            commands.entity(entity).remove::<TextFieldError>();
        }
        Err(_) => {
            commands
                .entity(entity)
                .insert(TextFieldError(SEED_REFUSAL.to_string()));
        }
    }
    for button in &buttons {
        if parsed.is_ok() {
            commands.entity(button).remove::<InteractionDisabled>();
        } else {
            commands.entity(button).insert(InteractionDisabled);
        }
    }
}

/// Hand the builder a fresh seed, in the field they can still edit.
pub(crate) fn reroll_seed(
    _activate: On<Activate>,
    mut seed: ResMut<HullSeed>,
    mut fields: Query<&mut TextFieldValue, With<HullSeedField>>,
) {
    seed.0 = rand::random();
    for mut value in &mut fields {
        value.0 = seed.0.to_string();
    }
}

/// Collapse one hull and lay it into the ship being edited.
///
/// The order is the whole point: collapse, LINT, and only then touch the
/// document. A hull the game would refuse never becomes nodes, so the ship
/// cannot be left holding one that fails to save - and every refusal below
/// leaves what was on the stage exactly as it was.
///
/// REPLACES the ship's sections. A seed is a dial rather than a purchase: the
/// builder rolls it until they like the hull, and a Generate that added would
/// stack ten drafts inside one ship on the way there.
pub(crate) fn generate_ship(
    _activate: On<Activate>,
    mut commands: Commands,
    seed: Res<HullSeed>,
    line: Res<HullGrammar>,
    sections: Option<Res<GameSections>>,
    grammars: Option<Res<GameGrammars>>,
    styles: Option<Res<GameStyles>>,
    drawable: Query<(&PartChoice, Has<Selected>)>,
    held: Query<&Children>,
    q_sections: Query<(), With<SectionNode>>,
    mut ordinals: Query<&mut NextChildOrdinal>,
    q_ships: Query<&ShipNode>,
    context: Res<EditContext>,
    mut selected: ResMut<SelectedNode>,
    mut says: EditorSays,
) {
    let Some(ship) = context.ship() else {
        says.refuse("go inside a ship to generate a hull into it");
        return;
    };
    let (Some(sections), Some(grammars)) = (sections.as_deref(), grammars.as_deref()) else {
        says.refuse("the content has not finished loading");
        return;
    };

    let drawn: Vec<Drawn> = drawable
        .iter()
        .filter(|(_, ticked)| *ticked)
        .map(|(choice, _)| Drawn {
            prototype: choice.prototype.clone(),
            zone: choice.zone,
        })
        .collect();
    // The look the ship already wears, carried INTO the collapse. A seed is a
    // dial: rolling it asks for a different hull under the same ship, not for
    // the ship's cladding back at the factory setting.
    let (clad, wears) = q_ships
        .get(ship)
        .map_or((true, None), |node| (node.skin, node.style.clone()));
    let hull = match drawn_grammar(sections, grammars, &line.0, &drawn).and_then(|grammar| {
        collapse(
            sections,
            &grammar,
            styles.as_deref(),
            seed.0,
            clad,
            wears.as_deref(),
        )
    }) {
        Ok(hull) => hull,
        Err(refusal) => {
            says.refuse(refusal);
            return;
        }
    };

    // Only now, with a hull in hand that the lint accepts.
    for child in held
        .iter_descendants(ship)
        .filter(|node| q_sections.contains(*node))
    {
        commands.entity(child).despawn();
    }

    let parts = hull.sections.len();
    let ordinal = resume_ordinal(hull.sections.iter().map(|section| section.id.as_str()));
    for section in hull.sections {
        let bare = SectionNode {
            source: section.source,
            modifications: section.modifications,
            binds: vec![],
        };
        // The keys the flight HUD names, on the parts that answer to them. A
        // generated ship is one the builder can set to Player and fly, and an
        // unbound battery is a ship with no trigger.
        let binds = bare
            .resolve(Some(sections))
            .map_or_else(Vec::new, |config| default_binds(&config.kind));
        insert_lifted_section(
            &mut commands,
            Some(sections),
            ship,
            NodeId(section.id),
            SectionNode { binds, ..bare },
            Transform::from_translation(section.position).with_rotation(section.rotation),
        );
    }
    resume_ordinals(&mut ordinals, ship, ordinal);
    selected.0 = Some(ship);
    says.note(format!("seed {} built a hull of {parts} parts", seed.0));
}

/// What an unauthored part joins the draw at.
///
/// The grammar prices every part it names, tuned by looking at hulls. A
/// section it does not name has no such number, and this is the plain one the
/// block's own note puts on screen beside the list - stated rather than
/// hidden, because a builder ticking a part the shipped ship never draws is
/// running an experiment and has to know the terms of it.
pub(crate) const UNAUTHORED_WEIGHT: f32 = 1.0;

/// The grammar this roll runs: the LINE the builder picked, drawing exactly the
/// sections that are ticked, around the biggest drive among them.
///
/// The vacuum taper stays the grammar's own - it is what a standard hull IS,
/// and it is not a part. What the list decides is the draw, the ship's MAIN
/// ENGINE, and through that engine the size of the ship.
pub(crate) struct Drawn {
    /// The catalog section id the builder ticked.
    pub(crate) prototype: String,
    /// Where they zoned it, if they did.
    pub(crate) zone: Option<GrammarZone>,
}

fn drawn_grammar(
    sections: &GameSections,
    grammars: &GameGrammars,
    line: &str,
    drawn: &[Drawn],
) -> Result<ShipGrammarConfig, String> {
    let base = grammars
        .get_grammar(line)
        .ok_or_else(|| format!("no ship grammar '{line}' in the merged content"))?;
    if drawn.is_empty() {
        return Err("tick at least one section for the collapse to draw".to_string());
    }
    let mut grammar = base.clone();
    grammar.parts = drawn
        .iter()
        .map(|ticked| {
            let mut part = base
                .parts
                .iter()
                .find(|part| part.prototype == ticked.prototype)
                .cloned()
                .unwrap_or_else(|| GrammarPart {
                    prototype: ticked.prototype.clone(),
                    weight: UNAUTHORED_WEIGHT,
                    aim: None,
                    zone: None,
                });
            // The builder's zone beats the grammar's, and clearing one on the
            // row clears it here: the row IS the setting.
            part.zone = ticked.zone;
            part
        })
        .collect();

    // The seeded roles are the grammar's own, and `nova_wfc` gives every role
    // it names tiles whether or not the draw prices it - so a builder who
    // unticks the hull cube still gets the spine the grammar says a ship has.
    if let Some(main) = main_engine(sections, drawn) {
        grammar.keel.stern_drive = main;
    }
    grammar.keel.bow_gun = bow_gun(sections, drawn);
    grammar.grid = holding(sections, &grammar);
    Ok(grammar)
}

/// The biggest drive the builder ticked, which becomes the ship's main engine.
///
/// A drive several cells across only works at the TRANSOM - it wants nine or
/// twenty-five exhaust lanes clear at once - so the collapse seeds it there
/// rather than rolling for it. Ticking one is therefore a decision about what
/// KIND of ship this is, and the biggest is the one the ship is built around.
/// The rest of the ticked drives still roll onto whatever transom is left.
pub(crate) fn main_engine(sections: &GameSections, drawn: &[Drawn]) -> Option<String> {
    biggest(sections, drawn, |kind| {
        matches!(kind, SectionKind::Thruster(_))
    })
}

/// The spinal gun the builder ticked, which becomes the ship's bow gun.
///
/// The bow's answer to [`main_engine`], and the same bargain. A gun the whole
/// SHIP aims fires down its own axis, so it only works standing in the keel
/// column at the nose, where the lane in front of it leaves the grid. Ticking
/// one is a decision about what kind of ship this is; the collapse seeds it
/// there rather than hoping a roll finds the one place it fits.
///
/// A gun too wide to stand in the keel column is left to `nova_wfc` to refuse
/// by name, which is a better line than anything this could say.
pub(crate) fn bow_gun(sections: &GameSections, drawn: &[Drawn]) -> Option<String> {
    biggest(sections, drawn, |kind| {
        matches!(kind, SectionKind::Railgun(_))
    })
}

/// The ticked section of this kind that fills the most cells.
fn biggest(
    sections: &GameSections,
    drawn: &[Drawn],
    wanted: impl Fn(&SectionKind) -> bool,
) -> Option<String> {
    drawn
        .iter()
        .filter_map(|ticked| {
            let config = sections.get_section(&ticked.prototype)?;
            if !wanted(&config.kind) {
                return None;
            }
            let span = SectionFootprint::from_collider(config.base.collider.unwrap_or_default()).0;
            Some((span.x * span.y * span.z, ticked.prototype.clone()))
        })
        .max_by_key(|(cells, _)| *cells)
        .map(|(_, prototype)| prototype)
}

/// The grammar's grid, grown to hold both the roles it seeds.
///
/// Never shrunk: the authored size is the shape of the ship, and this only
/// says that a hull carrying a capital drive is a capital hull. The drive
/// stands one column off the seam with its deck plate in front of it; the bow
/// gun stands in the keel column at the other end, and the two of them have to
/// leave a keel between. Both are the bounds `nova_wfc` refuses a too-small
/// grid with.
fn holding(sections: &GameSections, grammar: &ShipGrammarConfig) -> GrammarGrid {
    let grid = grammar.grid;
    let cells = |prototype: &str| {
        sections
            .get_section(prototype)
            .map_or(UVec3::ONE, |config| {
                SectionFootprint::from_collider(config.base.collider.unwrap_or_default()).0
            })
    };
    let drive = cells(&grammar.keel.stern_drive);
    let bore = grammar
        .keel
        .bow_gun
        .as_deref()
        .map_or(0, |gun| cells(gun).z);
    GrammarGrid {
        half_width: grid.half_width.max(drive.x + 1),
        height: grid.height.max(drive.y),
        length: grid.length.max(drive.z + 1).max(bore + drive.z + 2),
    }
}

/// One hull, collapsed and then run through the game's own content gate.
///
/// `Err` is a line fit for the status: what could not be read, what could not
/// collapse, or the first thing the lint refused.
///
/// `clad` and `wears` are the EDITED SHIP's, not this function's to pick: a
/// builder who turned the skin off in Ship Settings, or chose the third style,
/// still has that after a reroll.
fn collapse(
    sections: &GameSections,
    grammar: &ShipGrammarConfig,
    styles: Option<&GameStyles>,
    seed: u64,
    clad: bool,
    wears: Option<&str>,
) -> Result<ShipHull, String> {
    let tiles = TileSet::build(sections, grammar)?;
    // Resolved the way the build view resolves it (`skin::editor_style`): the
    // ship's own style, or the first the content merge loaded where the ship
    // has not chosen - which is what `ShipNode::style: None` MEANS, so a hull
    // built here wears what the stage was already showing.
    let style = clad
        .then(|| match wears {
            Some(id) => styles
                .and_then(|styles| styles.get_style(id))
                .map(|style| style.id.as_str()),
            None => styles.and_then(|styles| nova_wfc::prelude::style_at(styles, 0)),
        })
        .flatten();
    let hull = tiles.hull(seed, clad, style)?;
    let errors = hull_errors(&hull, sections);
    match errors.first() {
        Some(first) => Err(format!(
            "the collapse built a hull the game refuses: {first}"
        )),
        None => Ok(hull),
    }
}
