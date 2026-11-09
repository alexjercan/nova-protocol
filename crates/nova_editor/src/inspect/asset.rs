//! The rows of the item on the bench.
//!
//! A form is the config walked by reflection, the same walk a scenario's nodes
//! get. What reflection cannot reach is written here by hand. A section's mesh
//! and its destroy sound are `#[reflect(ignore)]`, so they are text with a
//! rule. A build recipe's parts, a start role's stock and a campaign's member
//! list are list editors ([`RowValue::List`]): a row per entry, picked from
//! the ids the mod sees. A theme's palette is the fixed colour set, drawn as a
//! grid of swatches ([`RowValue::Palette`]).

use bevy::{prelude::*, reflect::PartialReflect};
use nova_gameplay::prelude::{ItemCategoryType, ItemDesignId, ShipInventoryStock};
use nova_modding::prelude::Content;
use nova_ship::prelude::{RenderMeshTransform, SectionCollider, SectionConfig};
use nova_ui::theme::{UiColorType, UiPalette, UiThemeConfig};

use super::{
    fixed, name_row, pose_rows, rotation_degrees, rotation_from_degrees, section_config,
    section_config_mut, walk, walk_edited, AssetSort, FieldRoot, InspectorRow, Limit, PaletteEntry,
    PathStep, RowValue, TURN_STEP,
};
use crate::{
    asset::{
        has_mesh, mesh_of, mesh_pose_mut, section_mass, set_mesh, AssetNode, AssetPartType,
        SocketNode,
    },
    glyph::kind_mark,
};

/// The smallest size a collider's rows take, in cells: a body with no volume has
/// no mass, and a section with no mass does not simulate.
const SMALLEST_CELLS: f32 = 0.05;

/// How far one pixel of a drag moves a size in cells.
const CELL_STEP: f32 = 0.05;

/// What the shape row offers when the file states no collider.
const UNIT_CUBE: &str = "Unit Cube";

/// A field of the item that reads as text and is written by a rule of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AssetTextType {
    /// A section's render mesh ref.
    Mesh,
    /// A section's destroy sound ref.
    DestroySound,
    /// Which shape a section's collider is.
    ColliderShape,
    /// The model of a style's fixture, by its place in the list. Its own
    /// rule because the field is kept out of reflection: an asset reference
    /// is not a value the walk can rebuild.
    FixtureModel(usize),
}

impl AssetTextType {
    /// The undo step an edit of this field records.
    pub(crate) fn edit_label(self) -> &'static str {
        match self {
            Self::Mesh => "Edit Mesh",
            Self::DestroySound => "Edit Destroy Sound",
            Self::ColliderShape => "Edit Collider Shape",
            Self::FixtureModel(_) => "Edit Fixture Model",
        }
    }
}

/// A list field of the item, edited a row at a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AssetListType {
    /// A section's build recipe: an item and a count a row.
    Parts,
    /// A start role's kit stock: an item and a count a row.
    Stock,
    /// A campaign's member scenarios, in play order.
    Scenarios,
}

impl AssetListType {
    /// The undo step an edit of this list records.
    pub(crate) fn edit_label(self) -> &'static str {
        match self {
            Self::Parts => "Edit Parts",
            Self::Stock => "Edit Stock",
            Self::Scenarios => "Edit Scenarios",
        }
    }

    /// Whether a row carries a count.
    pub(crate) fn counted(self) -> bool {
        matches!(self, Self::Parts | Self::Stock)
    }

    /// Whether `item` may stand on a row: a build recipe takes only items of
    /// the Parts category, which is what the recipe lint holds it to.
    pub(crate) fn takes(self, item: &Content) -> bool {
        match (self, item) {
            (Self::Parts, Content::Item(design)) => design.category == ItemCategoryType::Parts,
            _ => item.kind() == self.names_kind(),
        }
    }

    /// The content kind a row names.
    pub(crate) fn names_kind(self) -> &'static str {
        match self {
            Self::Parts | Self::Stock => "item",
            Self::Scenarios => "scenario",
        }
    }

    /// A row's content kind with its article, for a refusal.
    pub(crate) fn noun(self) -> &'static str {
        match self {
            Self::Parts => "a Parts item",
            Self::Stock => "an item",
            Self::Scenarios => "a scenario",
        }
    }

    /// What the add row says.
    pub(crate) fn add_word(self) -> &'static str {
        match self {
            Self::Parts => "Add part",
            Self::Stock => "Add item",
            Self::Scenarios => "Add scenario",
        }
    }
}

/// One row of a list field: the id it names, and its count where the list
/// counts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ListEntry {
    /// The id the row names.
    pub(crate) id: String,
    /// How many, for a counted list.
    pub(crate) count: Option<u32>,
}

/// One edit of a list field, as the list editor says it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ListEditType {
    /// Add this id at the end, with a count of one where the list counts.
    Add(String),
    /// Take this row away.
    Remove(usize),
    /// Name this id on this row instead.
    Pick(usize, String),
    /// Give this row the count typed.
    Count(usize, String),
}

/// The largest count a row takes: what four digits hold.
pub(crate) const MOST_COUNT: u32 = 9999;

/// What a refused count says.
const COUNT_RULE: &str = "a count is a whole number from 1 to 9999";

/// What a press row does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AssetPressType {
    /// Open the path window for a `.glb`.
    ImportMesh,
    /// Add a socket to the section.
    AddSocket,
}

/// A render mesh transform as the rows read it: turned in degrees.
#[derive(Reflect, Clone, Copy, Debug, PartialEq)]
pub(crate) struct MeshPose {
    /// The offset from the section's origin, in cells.
    pub(crate) position: Vec3,
    /// Yaw, pitch and roll, in degrees.
    pub(crate) rotation: Vec3,
    /// The scale on each axis.
    pub(crate) scale: Vec3,
}

impl MeshPose {
    /// The pose `held` reads as, absent being the identity.
    pub(crate) fn of(held: Option<RenderMeshTransform>) -> Self {
        let held = held.unwrap_or_default();
        Self {
            position: held.position,
            rotation: rotation_degrees(&Transform::from_rotation(held.rotation)),
            scale: held.scale,
        }
    }

    /// The transform the rows now say.
    pub(crate) fn transform(self) -> RenderMeshTransform {
        RenderMeshTransform {
            position: self.position,
            rotation: rotation_from_degrees(self.rotation),
            scale: self.scale,
        }
    }
}

/// The part of `item` the walk runs over: a section's base, or the whole config
/// of a form kind. `None` for a kind with no form.
pub(crate) fn content_reflect(item: &Content) -> Option<&dyn PartialReflect> {
    Some(match item {
        Content::Section(config) => &config.base,
        Content::Style(config) => config,
        Content::StartRole(config) => config,
        Content::Item(config) => config,
        Content::PriceTable(config) => config,
        Content::TradeProfile(config) => config,
        Content::UiTheme(config) => config.as_ref(),
        Content::Campaign(config) => config,
        Content::Scenario(_) | Content::Ship(_) | Content::Station(_) | Content::Lesson(_) => {
            return None
        }
    })
}

/// [`content_reflect`], for writing.
pub(crate) fn content_reflect_mut(item: &mut Content) -> Option<&mut dyn PartialReflect> {
    Some(match item {
        Content::Section(config) => &mut config.base,
        Content::Style(config) => config,
        Content::StartRole(config) => config,
        Content::Item(config) => config,
        Content::PriceTable(config) => config,
        Content::TradeProfile(config) => config,
        Content::UiTheme(config) => config.as_mut(),
        Content::Campaign(config) => config,
        Content::Scenario(_) | Content::Ship(_) | Content::Station(_) | Content::Lesson(_) => {
            return None
        }
    })
}

/// What one field of `item` breaks against another, in the words the unit
/// slot shows, or `None` when its fields agree. The cross-field rule of
/// `item_faults`: a lot never bids above its ask, or buying and selling back
/// would gain credits.
pub(crate) fn form_fault(item: &Content) -> Option<String> {
    match item {
        Content::Item(design) if design.bid_cr > design.ask_cr => {
            Some(format!("bid over ask {}", design.ask_cr))
        }
        _ => None,
    }
}

/// A section's kind payload, for writing.
pub(crate) fn asset_kind_mut(item: &mut Content) -> Option<&mut dyn PartialReflect> {
    match item {
        Content::Section(config) => Some(section_config_mut(&mut config.kind)),
        _ => None,
    }
}

/// Whether `row` sits under the field `name` at the top of its path.
fn is_under(row: &InspectorRow, name: &str) -> bool {
    matches!(row.path.first(), Some(PathStep::Field(held)) if held == name)
}

/// A row typed as text and written by `kind`'s rule.
fn text_row(kind: AssetTextType, label: &str, text: String) -> InspectorRow {
    InspectorRow {
        root: FieldRoot::AssetText(kind),
        value: RowValue::Text(text),
        ..fixed(FieldRoot::AssetText(kind), label, "")
    }
}

/// The row a button stands in.
fn press_row(press: AssetPressType, label: &str, word: &str) -> InspectorRow {
    InspectorRow {
        value: RowValue::Press(word.to_string()),
        ..fixed(FieldRoot::AssetPress(press), label, "")
    }
}

/// The row the item's id is typed in. A rename, so it runs as a verb.
fn id_row(item: &Content) -> InspectorRow {
    InspectorRow {
        root: FieldRoot::AssetId,
        ..name_row(item.id().to_string())
    }
    .labelled("Id")
    .saying("What other content names this by. A rename updates the mod's own references.")
}

/// A count map as list rows, in id order.
fn counted_entries<'a>(stacks: impl Iterator<Item = (&'a ItemDesignId, u32)>) -> Vec<ListEntry> {
    stacks
        .map(|(id, count)| ListEntry {
            id: id.as_str().to_string(),
            count: Some(count),
        })
        .collect()
}

/// The rows `kind` reads as on `item`.
pub(crate) fn list_entries(item: &Content, kind: AssetListType) -> Vec<ListEntry> {
    match (item, kind) {
        (Content::Section(config), AssetListType::Parts) => counted_entries(
            config
                .base
                .build
                .parts
                .iter()
                .map(|(id, count)| (id, *count)),
        ),
        (Content::StartRole(role), AssetListType::Stock) => {
            counted_entries(role.kit.stock.stacks())
        }
        (Content::Campaign(campaign), AssetListType::Scenarios) => campaign
            .scenarios
            .iter()
            .map(|id| ListEntry {
                id: id.clone(),
                count: None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// `entries` after `edit`, or why the edit cannot be made. `known` is every
/// id a row may name.
pub(crate) fn edited_entries(
    entries: &[ListEntry],
    kind: AssetListType,
    edit: &ListEditType,
    known: &[String],
) -> Result<Vec<ListEntry>, String> {
    let mut next = entries.to_vec();
    let fits = |id: &str, except: Option<usize>| {
        if !known.iter().any(|held| held == id) {
            return Err(format!("{id} is not {} this mod sees", kind.noun()));
        }
        let taken = entries
            .iter()
            .enumerate()
            .any(|(index, entry)| Some(index) != except && entry.id == id);
        if taken {
            return Err(format!("{id} is already listed"));
        }
        Ok(())
    };
    let gone = || "that row is gone - pick the list again".to_string();
    match edit {
        ListEditType::Add(id) => {
            fits(id, None)?;
            next.push(ListEntry {
                id: id.clone(),
                count: kind.counted().then_some(1),
            });
        }
        ListEditType::Remove(index) => {
            if *index >= next.len() {
                return Err(gone());
            }
            next.remove(*index);
        }
        ListEditType::Pick(index, id) => {
            fits(id, Some(*index))?;
            next.get_mut(*index).ok_or_else(gone)?.id.clone_from(id);
        }
        ListEditType::Count(index, text) => {
            let count = text
                .trim()
                .parse::<u32>()
                .ok()
                .filter(|count| (1..=MOST_COUNT).contains(count))
                .ok_or_else(|| COUNT_RULE.to_string())?;
            let entry = next.get_mut(*index).ok_or_else(gone)?;
            if entry.count.is_none() {
                return Err("this list holds no counts".to_string());
            }
            entry.count = Some(count);
        }
    }
    Ok(next)
}

/// Write `entries` into `kind`'s field of `item`.
pub(crate) fn set_list_entries(
    item: &mut Content,
    kind: AssetListType,
    entries: &[ListEntry],
) -> Result<(), String> {
    let counted = || {
        entries
            .iter()
            .map(|entry| {
                (
                    ItemDesignId::from(entry.id.as_str()),
                    entry.count.unwrap_or(1),
                )
            })
            .collect::<Vec<_>>()
    };
    match (item, kind) {
        (Content::Section(config), AssetListType::Parts) => {
            config.base.build.parts = counted().into_iter().collect();
        }
        (Content::StartRole(role), AssetListType::Stock) => {
            role.kit.stock = ShipInventoryStock::new(counted());
        }
        (Content::Campaign(campaign), AssetListType::Scenarios) => {
            campaign.scenarios = entries.iter().map(|entry| entry.id.clone()).collect();
        }
        _ => return Err("not a list of this item".to_string()),
    }
    Ok(())
}

/// The row a list field stands in.
fn list_row(kind: AssetListType, label: &str, item: &Content) -> InspectorRow {
    InspectorRow {
        value: RowValue::List(list_entries(item, kind)),
        ..fixed(FieldRoot::AssetList(kind), label, "")
    }
}

/// The word the shape row reads for `collider`.
fn shape_word(collider: Option<SectionCollider>) -> &'static str {
    match collider {
        None => UNIT_CUBE,
        Some(SectionCollider::Cuboid { .. }) => "Cuboid",
        Some(SectionCollider::Sphere { .. }) => "Sphere",
        Some(SectionCollider::Capsule { .. }) => "Capsule",
        Some(SectionCollider::Cylinder { .. }) => "Cylinder",
    }
}

/// `held` as the shape `word` names, about as big as it was.
fn reshaped(held: Option<SectionCollider>, word: &str) -> Result<Option<SectionCollider>, String> {
    let size = held.unwrap_or_default().aabb_half_extents() * 2.0;
    let radius = size.x.min(size.z) * 0.5;
    Ok(match word {
        UNIT_CUBE => None,
        "Cuboid" => Some(SectionCollider::Cuboid { size }),
        "Sphere" => Some(SectionCollider::Sphere {
            radius: size.max_element() * 0.5,
        }),
        "Capsule" => Some(SectionCollider::Capsule {
            radius,
            length: (size.y - radius * 2.0).max(SMALLEST_CELLS),
        }),
        "Cylinder" => Some(SectionCollider::Cylinder {
            radius,
            height: size.y,
        }),
        other => return Err(format!("{other} is not a collider shape")),
    })
}

/// What `kind`'s field reads as on `item`.
pub(crate) fn item_text(item: &Content, kind: AssetTextType) -> String {
    match (item, kind) {
        (Content::Section(config), AssetTextType::Mesh) => mesh_of(&config.kind),
        (Content::Section(config), AssetTextType::DestroySound) => config
            .base
            .destroy_sound
            .as_ref()
            .and_then(|sound| sound.path())
            .map(str::to_string)
            .unwrap_or_default(),
        (Content::Section(config), AssetTextType::ColliderShape) => {
            shape_word(config.base.collider).to_string()
        }
        (Content::Style(style), AssetTextType::FixtureModel(index)) => style
            .fixtures
            .get(index)
            .and_then(|fixture| fixture.model.path())
            .map(str::to_string)
            .unwrap_or_default(),
        _ => String::new(),
    }
}

/// Write `text` into `kind`'s field of `item`, or say why it cannot go there.
pub(crate) fn set_item_text(
    item: &mut Content,
    kind: AssetTextType,
    text: &str,
) -> Result<(), String> {
    match (item, kind) {
        (Content::Section(config), AssetTextType::Mesh) => set_mesh(&mut config.kind, text),
        (Content::Section(config), AssetTextType::DestroySound) => {
            let text = text.trim();
            config.base.destroy_sound = (!text.is_empty()).then(|| text.to_string().into());
            Ok(())
        }
        (Content::Section(config), AssetTextType::ColliderShape) => {
            config.base.collider = reshaped(config.base.collider, text)?;
            Ok(())
        }
        (Content::Style(style), AssetTextType::FixtureModel(index)) => {
            let fixture = style
                .fixtures
                .get_mut(index)
                .ok_or_else(|| "that fixture is gone".to_string())?;
            let text = text.trim();
            if text.is_empty() {
                return Err("a fixture needs a model".to_string());
            }
            fixture.model = text.to_string().into();
            Ok(())
        }
        _ => Err("not a field of this item".to_string()),
    }
}

/// What a section's body weighs, as a row.
fn mass_row(config: &SectionConfig) -> InspectorRow {
    fixed(
        FieldRoot::Asset,
        "Mass",
        format!("{:.1} kg", section_mass(config)),
    )
    .saying("From the collider's volume: a section's density is one.")
}

/// The rows of a section's own fields. Its collider, mesh and sockets have rows
/// of their own under their tree nodes.
fn section_rows(config: &SectionConfig, rows: &mut Vec<InspectorRow>) {
    rows.push(
        fixed(FieldRoot::Asset, "Kind", kind_mark(Some(&config.kind)).1)
            .saying("Section kinds are code. Add > Sections asks for the kind of a new section."),
    );
    let mut base = Vec::new();
    walk_edited(&config.base, FieldRoot::Asset, Vec::new(), &mut base);
    base.retain(|row| {
        !is_under(row, "id")
            && !is_under(row, "collider")
            && !is_under(row, "link_points")
            && row.path.get(1) != Some(&PathStep::Field("parts".to_string()))
    });
    rows.extend(base);
    // Beside the rest of its group: a group heading repeats after any other row.
    let build = "Build".to_string();
    let after = rows
        .iter()
        .rposition(|row| row.group.first() == Some(&build))
        .map_or(rows.len(), |last| last + 1);
    rows.insert(
        after,
        InspectorRow {
            group: vec![build],
            ..list_row(
                AssetListType::Parts,
                "Parts",
                &Content::Section(Box::new(config.clone())),
            )
        }
        .saying("What the section costs in items: an item and a count a row."),
    );
    let mut sound = text_row(
        AssetTextType::DestroySound,
        "Destroy Sound",
        item_text(
            &Content::Section(Box::new(config.clone())),
            AssetTextType::DestroySound,
        ),
    );
    sound.asset = Some(AssetSort::Audio);
    rows.push(sound);
    rows.push(mass_row(config));
    let mut kind = Vec::new();
    walk(
        section_config(&config.kind),
        FieldRoot::AssetKind,
        Vec::new(),
        &mut kind,
    );
    kind.retain(|row| !is_under(row, "render_mesh_transform"));
    rows.extend(kind);
    rows.push(press_row(
        AssetPressType::AddSocket,
        "Sockets",
        "Add Socket",
    ));
}

/// The path step of a kit's stock.
fn stock_step() -> PathStep {
    PathStep::Field("stock".to_string())
}

/// What a theme or a style keeps that no row reaches, as a row that says so.
fn kept_row(label: &str, text: String) -> InspectorRow {
    fixed(FieldRoot::Asset, label, text)
        .saying("Kept as written: edit it in the content file, and the save preserves it.")
}

/// The rows of a form kind.
fn form_rows(item: &Content, inherited: &UiPalette, rows: &mut Vec<InspectorRow>) {
    let Some(config) = content_reflect(item) else {
        return;
    };
    let mut walked = Vec::new();
    walk_edited(config, FieldRoot::Asset, Vec::new(), &mut walked);
    walked.retain(|row| {
        !is_under(row, "id") && !is_under(row, "palette_map") && !is_under(row, "scenarios")
    });
    match item {
        Content::UiTheme(theme) => {
            walked.retain(|row| !is_under(row, "palette"));
            rows.extend(walked);
            rows.push(palette_row(theme, inherited));
            let roles = if theme.roles == Default::default() {
                "none"
            } else {
                "kept as written"
            };
            rows.push(kept_row("Roles", roles.to_string()));
        }
        Content::Campaign(_) => {
            rows.extend(walked);
            rows.push(
                list_row(AssetListType::Scenarios, "Scenarios", item)
                    .saying("The member scenarios, in play order. A new one plays last."),
            );
        }
        Content::StartRole(_) => {
            let kit = "Kit".to_string();
            walked.retain(|row| !(is_under(row, "kit") && row.path.get(1) == Some(&stock_step())));
            let after = walked
                .iter()
                .rposition(|row| row.group.first() == Some(&kit))
                .map_or(walked.len(), |last| last + 1);
            walked.insert(
                after,
                InspectorRow {
                    group: vec![kit],
                    ..list_row(AssetListType::Stock, "Stock", item)
                }
                .saying("What the role starts with in its hold: an item and a count a row."),
            );
            rows.extend(walked);
        }
        Content::Style(style) => {
            // Each fixture's model under its own entry: the field is kept out
            // of reflection, so the walk has no row for it.
            for index in (0..style.fixtures.len()).rev() {
                let entry = [
                    PathStep::Field("fixtures".to_string()),
                    PathStep::Item(index),
                ];
                let Some(last) = walked.iter().rposition(|row| row.path.starts_with(&entry)) else {
                    continue;
                };
                let kind = AssetTextType::FixtureModel(index);
                let mut model = text_row(kind, "Model", item_text(item, kind))
                    .saying("The fixture's model file, a .glb the mod ships.");
                model.group = vec![format!("Fixtures {}", index + 1)];
                model.asset = Some(AssetSort::Model);
                walked.insert(last + 1, model);
            }
            rows.extend(walked);
            rows.push(kept_row(
                "Greeble Rules",
                format!(
                    "{} greebles, {} link types",
                    style.greeble_rules.greebles.len(),
                    style.greeble_rules.link_types.len()
                ),
            ));
        }
        _ => rows.extend(walked),
    }
}

/// The palette `theme` takes from the themes it inherits: each colour from
/// the nearest ancestor that sets it. `lookup` finds a theme by id. A missing
/// or repeated ancestor ends the walk; the lint names both faults.
pub(crate) fn inherited_palette<'a>(
    theme: &UiThemeConfig,
    lookup: impl Fn(&str) -> Option<&'a UiThemeConfig>,
) -> UiPalette {
    let mut chain: Vec<&UiThemeConfig> = Vec::new();
    let mut next = theme.inherit.as_deref();
    while let Some(id) = next {
        let Some(parent) = lookup(id) else {
            break;
        };
        if parent.id == theme.id || chain.iter().any(|seen| seen.id == parent.id) {
            break;
        }
        chain.push(parent);
        next = parent.inherit.as_deref();
    }
    let mut palette = UiPalette::default();
    for &color in UiColorType::ALL {
        *palette.slot_mut(color) = chain
            .iter()
            .find_map(|ancestor| ancestor.palette.get(color))
            .map(str::to_string);
    }
    palette
}

/// A theme's palette as one grid row: one swatch per [`UiColorType`], the
/// same rows for every theme. A colour the theme does not set shows the one
/// it takes from `inherited`, the palette of the theme it inherits.
fn palette_row(theme: &UiThemeConfig, inherited: &UiPalette) -> InspectorRow {
    let entries = UiColorType::ALL
        .iter()
        .map(|&color| {
            let (hex, inherited) = match (theme.palette.get(color), inherited.get(color)) {
                (Some(own), _) => (own, false),
                (None, Some(parent)) => (parent, true),
                // A root theme with a gap: the swatch stays empty, and the
                // lint names the colour.
                (None, None) => ("", false),
            };
            PaletteEntry {
                name: color.name().to_string(),
                hex: hex.to_string(),
                inherited,
            }
        })
        .collect();
    let hint = match &theme.inherit {
        Some(parent) => format!(
            "The colours every theme sets. Press one to pick it. A dim name is {parent}'s until it is picked here."
        ),
        None => "The colours every theme sets. Press one to pick it.".to_string(),
    };
    InspectorRow {
        value: RowValue::Palette(entries),
        ..fixed(FieldRoot::PaletteColour, "Palette", "")
    }
    .saying(hint)
}

/// The rows of the bench's root: the item itself. `inherited` is the palette
/// a UI theme takes from the themes it inherits, and empty for anything else.
pub(crate) fn asset_rows(item: &Content, inherited: &UiPalette) -> Vec<InspectorRow> {
    let mut rows = match item {
        Content::PriceTable(_) => vec![fixed(FieldRoot::Asset, "Id", item.id())
            .saying("A mod states one price table, and it replaces the base table whole.")],
        _ => vec![id_row(item)],
    };
    match item {
        Content::Section(config) => section_rows(config, &mut rows),
        other => form_rows(other, inherited, &mut rows),
    }
    rows
}

/// The rows of a collider part. They write to `root`, which holds the config.
fn collider_rows(config: &SectionConfig, root: Entity) -> Vec<InspectorRow> {
    let options: Vec<String> = [UNIT_CUBE, "Cuboid", "Sphere", "Capsule", "Cylinder"]
        .map(str::to_string)
        .to_vec();
    let word = shape_word(config.base.collider);
    let mut rows = vec![InspectorRow {
        value: RowValue::Choice {
            chosen: options.iter().position(|held| held == word).unwrap_or(0),
            hints: vec![String::new(); options.len()],
            options,
        },
        ..fixed(
            FieldRoot::AssetText(AssetTextType::ColliderShape),
            "Shape",
            "",
        )
    }
    .saying("The body this section collides with. Sizes are in 10 m build cells.")];
    let mut base = Vec::new();
    walk(&config.base, FieldRoot::Asset, Vec::new(), &mut base);
    rows.extend(
        base.into_iter()
            .filter(|row| is_under(row, "collider"))
            .filter(|row| !matches!(row.value, RowValue::Fixed(_)))
            .map(|row| InspectorRow {
                group: Vec::new(),
                unit: "cells",
                nudge: CELL_STEP,
                limit: Limit::AtLeast(SMALLEST_CELLS),
                ..row
            }),
    );
    rows.push(mass_row(config));
    for row in &mut rows {
        row.owner = Some(root);
    }
    rows
}

/// The rows of a mesh part. `fault` is why the mesh does not load, if it does
/// not. They write to `root`, which holds the config.
fn mesh_rows(config: &mut SectionConfig, root: Entity, fault: Option<String>) -> Vec<InspectorRow> {
    if !has_mesh(&config.kind) {
        let rows = vec![fixed(FieldRoot::Asset, "Mesh", "per joint")
            .saying("A turret holds a mesh on each joint: see its joint tree in the root panel.")];
        return rows;
    }
    let mut mesh =
        text_row(AssetTextType::Mesh, "Mesh", mesh_of(&config.kind)).saying(fault.unwrap_or_else(
            || "The model this section draws. Empty draws the kind's built-in mesh.".to_string(),
        ));
    mesh.asset = Some(AssetSort::Model);
    let mut rows = vec![
        mesh,
        press_row(AssetPressType::ImportMesh, "Import", "Import .glb..."),
    ];
    let held = mesh_pose_mut(&mut config.kind).and_then(|pose| *pose);
    let pose = MeshPose::of(held);
    let axes = [
        ("Position", "position", "cells", pose.position, CELL_STEP),
        ("Rotation", "rotation", "deg", pose.rotation, TURN_STEP),
        ("Scale", "scale", "", pose.scale, CELL_STEP),
    ];
    for (label, field, unit, value, nudge) in axes {
        let mut row = super::axes_row(FieldRoot::MeshPose, label, unit, nudge, Limit::Free, value);
        row.path = vec![PathStep::Field(field.to_string())];
        rows.push(row);
    }
    for row in &mut rows {
        row.owner = Some(root);
    }
    rows
}

/// The rows of a part of the section under `root`.
pub(crate) fn part_rows(
    part: AssetPartType,
    node: &AssetNode,
    root: Entity,
    fault: Option<String>,
) -> Vec<InspectorRow> {
    let Content::Section(config) = &node.0 else {
        return Vec::new();
    };
    match part {
        AssetPartType::Collider => collider_rows(config, root),
        AssetPartType::Mesh => mesh_rows(&mut config.as_ref().clone(), root, fault),
    }
}

/// The rows of one socket.
pub(crate) fn socket_rows(id: &str, socket: &SocketNode, pose: &Transform) -> Vec<InspectorRow> {
    let mut rows = vec![InspectorRow {
        root: FieldRoot::SocketId,
        ..name_row(id.to_string())
    }
    .labelled("Id")
    .saying("What a mate names this socket by, unique on the section.")];
    walk(socket, FieldRoot::Socket, Vec::new(), &mut rows);
    rows.retain(|row| !is_under(row, "order") && !is_under(row, "authored"));
    let normal = socket.normal(pose.rotation);
    rows.push(
        fixed(
            FieldRoot::Socket,
            "Normal",
            format!("{:.3}, {:.3}, {:.3}", normal.x, normal.y, normal.z),
        )
        .saying("The way the socket faces, from its rotation."),
    );
    rows.extend(pose_rows(pose));
    rows
}
