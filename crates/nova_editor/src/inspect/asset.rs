//! The rows of the item on the bench.
//!
//! A form is the config walked by reflection, the same walk a scenario's nodes
//! get. What reflection cannot reach is written here by hand, as text with a
//! rule: a section's mesh and its destroy sound are `#[reflect(ignore)]`, a
//! build recipe's parts are a map, and a campaign's member list is a shape the
//! walk draws read-only. A theme's palette is a map too, drawn as a grid of
//! swatches ([`RowValue::Palette`]).

use std::collections::BTreeMap;

use bevy::{prelude::*, reflect::PartialReflect};
use nova_modding::prelude::Content;
use nova_scenario::prelude::Names;
use nova_ship::prelude::{RenderMeshTransform, SectionCollider, SectionConfig};
use nova_ui::theme::UiThemeConfig;

use super::{
    fixed, name_row, pose_rows, rotation_degrees, rotation_from_degrees, section_config,
    section_config_mut, walk, AssetSort, FieldRoot, InspectorRow, Limit, PaletteEntry, PathStep,
    RowValue, TURN_STEP,
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
    /// A build recipe's parts, as `Item=2, Other=1`.
    Parts,
    /// A campaign's member scenarios, in play order, comma separated.
    Scenarios,
    /// Which shape a section's collider is.
    ColliderShape,
}

impl AssetTextType {
    /// The undo step an edit of this field records.
    pub(crate) fn edit_label(self) -> &'static str {
        match self {
            Self::Mesh => "Edit Mesh",
            Self::DestroySound => "Edit Destroy Sound",
            Self::Parts => "Edit Parts",
            Self::Scenarios => "Edit Scenarios",
            Self::ColliderShape => "Edit Collider Shape",
        }
    }
}

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

/// A build recipe's parts as one line.
fn parts_text(parts: &BTreeMap<nova_gameplay::prelude::ItemDesignId, u32>) -> String {
    parts
        .iter()
        .map(|(item, count)| format!("{}={count}", item.as_str()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `text` as `key=value` pairs, comma separated. An empty line holds none.
fn pairs(text: &str) -> Result<Vec<(&str, &str)>, String> {
    text.split(',')
        .map(str::trim)
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair
                .split_once('=')
                .ok_or_else(|| format!("'{pair}' needs a name=value"))?;
            Ok((key.trim(), value.trim()))
        })
        .collect()
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
        (Content::Section(config), AssetTextType::Parts) => parts_text(&config.base.build.parts),
        (Content::Section(config), AssetTextType::ColliderShape) => {
            shape_word(config.base.collider).to_string()
        }
        (Content::Campaign(campaign), AssetTextType::Scenarios) => campaign.scenarios.join(", "),
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
        (Content::Section(config), AssetTextType::Parts) => {
            let mut parts = BTreeMap::new();
            for (item, count) in pairs(text)? {
                let count: u32 = count
                    .parse()
                    .map_err(|_| format!("{count} is not a whole count"))?;
                parts.insert(item.into(), count);
            }
            config.base.build.parts = parts;
            Ok(())
        }
        (Content::Section(config), AssetTextType::ColliderShape) => {
            config.base.collider = reshaped(config.base.collider, text)?;
            Ok(())
        }
        (Content::Campaign(campaign), AssetTextType::Scenarios) => {
            campaign.scenarios = text
                .split(',')
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(str::to_string)
                .collect();
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
    walk(&config.base, FieldRoot::Asset, Vec::new(), &mut base);
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
            ..text_row(
                AssetTextType::Parts,
                "Parts",
                parts_text(&config.base.build.parts),
            )
        }
        .saying("What the section costs in items, as Item=2, Other=1."),
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

/// What a theme or a style keeps that no row reaches, as a row that says so.
fn kept_row(label: &str, text: String) -> InspectorRow {
    fixed(FieldRoot::Asset, label, text)
        .saying("Kept as written: edit it in the content file, and the save preserves it.")
}

/// The rows of a form kind.
fn form_rows(item: &Content, inherited: &BTreeMap<String, String>, rows: &mut Vec<InspectorRow>) {
    let Some(config) = content_reflect(item) else {
        return;
    };
    let mut walked = Vec::new();
    walk(config, FieldRoot::Asset, Vec::new(), &mut walked);
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
            let mut members = text_row(
                AssetTextType::Scenarios,
                "Scenarios",
                item_text(item, AssetTextType::Scenarios),
            )
            .saying("Member scenario ids in play order, comma separated.");
            members.names = None::<Names>;
            rows.push(members);
        }
        Content::Style(style) => {
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

/// The palette `theme` takes from the themes it inherits: each ancestor's
/// colours, the nearer over the farther. `lookup` finds a theme by id. A
/// missing or repeated ancestor ends the walk; the lint names both faults.
pub(crate) fn inherited_palette<'a>(
    theme: &UiThemeConfig,
    lookup: impl Fn(&str) -> Option<&'a UiThemeConfig>,
) -> BTreeMap<String, String> {
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
    let mut palette = BTreeMap::new();
    for ancestor in chain.iter().rev() {
        palette.extend(ancestor.palette.clone());
    }
    palette
}

/// A theme's palette as one grid row: its own colours, and the ones it takes
/// from `inherited`, the palette of the theme it inherits.
fn palette_row(theme: &UiThemeConfig, inherited: &BTreeMap<String, String>) -> InspectorRow {
    let mut entries: BTreeMap<&str, PaletteEntry> = inherited
        .iter()
        .map(|(name, hex)| {
            let entry = PaletteEntry {
                name: name.clone(),
                hex: hex.clone(),
                inherited: true,
            };
            (name.as_str(), entry)
        })
        .collect();
    for (name, hex) in &theme.palette {
        entries.insert(
            name.as_str(),
            PaletteEntry {
                name: name.clone(),
                hex: hex.clone(),
                inherited: false,
            },
        );
    }
    let hint = match &theme.inherit {
        Some(parent) => format!(
            "Named colours. Press one to pick it. A dim name is {parent}'s until it is picked here."
        ),
        None => "Named colours. Press one to pick it.".to_string(),
    };
    InspectorRow {
        value: RowValue::Palette(entries.into_values().collect()),
        ..fixed(FieldRoot::PaletteColour, "Palette", "")
    }
    .saying(hint)
}

/// The rows of the bench's root: the item itself. `inherited` is the palette
/// a UI theme takes from the themes it inherits, and empty for anything else.
pub(crate) fn asset_rows(
    item: &Content,
    inherited: &BTreeMap<String, String>,
) -> Vec<InspectorRow> {
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
