//! The Ship pane's station catalogue: the parts gallery (`nova_ship::gallery`)
//! over the pane while the ship is docked at a station that refits.
//!
//! The footer's Catalogue button opens it. [`manage_ship_gallery`] then puts a
//! [`GalleryHost`] node over the view, the panel and the footer, with its own
//! camera on [`GALLERY_LAYER`] rendering the part tiles into an image the node
//! shows 1:1, and takes both away when the gallery closes or the pane goes:
//! a tab switch or the interface closing closes the gallery.
//!
//! [`update_ship_gallery_listing`] lists every part not hidden from the editor
//! palette, each with its recipe items. A part the station does not build, or
//! whose recipe the player's hold and the station's stock together cannot
//! carry ([`plan_recipe_payment`] finds a missing part), is listed muted with
//! that reason and cannot be placed. Credits do not mute: a part the ship
//! cannot pay for shows the credit refusal as its note; a payable one, its
//! price. A placeable part that can replace the selected section offers Swap
//! with its price as the focus card's second verb, or, when the swap is
//! refused, a second note line that says why.
//!
//! Place puts the part in hand ([`HeldPart`]); the placement ghost
//! (`super::ghost`) shows where a click adds it. Swap raises a
//! [`SectionSwapCommand`] for the selected section
//! ([`apply_ship_gallery_alternates`]).
//!
//! Touch this module when changing what the catalogue lists or how the Ship
//! pane hosts the gallery.

use bevy::{
    camera::{visibility::RenderLayers, ImageRenderTarget, RenderTarget},
    prelude::*,
    ui::ComputedNode,
};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::theme::{ActiveUiTheme, UiColor};

use super::{
    rebuild::{payment_refusal_text, StationSite},
    refit::{swap_refusal_text, SectionSwapCommand},
    scene::ShipRuntime,
    sections::{design_section_code, ShipViewSubject},
};

/// The render layer of the catalogue's part tiles. Above the interface's own
/// layers (20 to 22) and the menu's (23), below the hull cards' (26 on).
const GALLERY_LAYER: usize = 24;

/// The catalogue camera's order: before the interface's render-to-texture
/// pass (-20), and apart from the ship camera (-31) and the map's (-30).
const GALLERY_CAMERA_ORDER: isize = -32;

/// How far in front of the catalogue camera its tiles stand, in scene units.
const GALLERY_STAGE_DISTANCE: f32 = 8.0;

/// The key and rim lights' bearings and illuminance (lux) and the camera's
/// ambient brightness: the editor's part turntable rig, which the hull cards
/// use too, so a tile reads as the part does on the editor's stage.
const GALLERY_KEY: (Vec3, f32) = (Vec3::new(-6.0, 8.0, 6.0), 9_000.0);
const GALLERY_RIM: (Vec3, f32) = (Vec3::new(5.0, -2.0, -7.0), 2_500.0);
const GALLERY_AMBIENT: f32 = 220.0;

/// Marks the Ship pane's split. The catalogue covers the split's parent, the
/// pane body, so the view, the panel and the footer while it is open.
#[derive(Component)]
pub(crate) struct ShipSplitMarker;

/// Marks the footer's Catalogue button, shown while the station builds parts.
#[derive(Component)]
pub(crate) struct ShipCatalogueButton;

/// Marks the catalogue's own camera, which holds its lights. The camera is a
/// root entity: a despawn of the pane body takes the host node, not the
/// camera, so [`manage_ship_gallery`] finds an orphan camera by this marker.
#[derive(Component)]
pub(crate) struct ShipCatalogueCameraMarker;

/// The image the catalogue's camera renders the tiles into. Worn by the host
/// node, whose [`GalleryHost`] names the camera.
#[derive(Component)]
pub(crate) struct ShipGalleryHost {
    image: Handle<Image>,
}

/// The listing for the docked refit station: each part with its price or
/// refusal, and Swap when it can replace the selected section. `None` while
/// the ship is not docked at a living station that refits.
fn station_listing(
    runtime: &ShipRuntime,
    items: &GameItems,
    sections: &GameSections,
    site: &StationSite,
    q_holds: &Query<(&ShipInventory, &ShipCredits)>,
) -> Option<GalleryListing> {
    if !site.gate(StationServiceType::Refit) {
        return None;
    }
    let resolved = site.resolve(StationServiceType::Refit)?;
    let (own, own_cr) = q_holds
        .get(resolved.player)
        .expect("the player ship carries a ShipInventory and ShipCredits");
    let partner = resolved
        .partner
        .and_then(|(partner, ..)| q_holds.get(partner).ok())
        .map(|(stock, credits)| (stock, credits.0));
    let (stock, stock_cr) = partner?;
    let (_, _, builds) = resolved.partner.expect("a partner with a hold is docked");
    // Station lint refuses an unknown id; a record that skipped lint fails
    // here rather than listing nothing for it.
    for id in builds {
        assert!(
            sections.get_section(id).is_some(),
            "station builds '{id}', which GameSections lacks; station lint refuses this"
        );
    }
    let selected = resolved.selected_section(runtime.selected);
    let mut entries = Vec::new();
    for offered in sections
        .iter()
        .filter(|offered| !offered.base.hide_in_editor)
    {
        let mut entry = offered_entry(items, offered, builds, stock, stock_cr, own, own_cr.0);
        // A muted part's swap could only refuse with the reason it already
        // shows, so it offers none.
        if let Some(section) = selected.filter(|section| {
            entry.placeable && section_swap_family(&section.config, offered).is_ok()
        }) {
            let code = design_section_code(resolved.design, &section.id)
                .expect("a live design section has a code");
            match resolved.plan_swap(items, section, offered, own, own_cr.0, partner) {
                Ok(payment) => {
                    entry.alternate = Some(format!("Swap {code}, {}", cr_text(payment.price_cr)));
                }
                Err(refusal) => {
                    entry.note.push('\n');
                    entry.note.push_str(&swap_refusal_text(
                        items,
                        &code,
                        &offered.base.name,
                        &refusal,
                    ));
                }
            }
        }
        entries.push(entry);
    }
    Some(GalleryListing(entries))
}

/// One catalogue entry for `offered` at a station that builds `builds` with
/// stock `stock`/`stock_cr`, against the ship's `own`/`own_cr`, without a
/// swap. Reasons in `SectionAddRefusalType` order: not built, then payment.
/// A part not built or short of a recipe item cannot be placed and has no
/// tile line; its note is the disabled Place's tooltip. A credit refusal
/// stays placeable with the credits it needs on its tile, and the placement
/// ghost refuses the click.
fn offered_entry(
    items: &GameItems,
    offered: &SectionConfig,
    builds: &[String],
    stock: &ShipInventory,
    stock_cr: u32,
    own: &ShipInventory,
    own_cr: u32,
) -> GalleryEntry {
    let name = &offered.base.name;
    let recipe = &offered.base.build;
    let (tile, note, recipe_text, placeable) = if !builds.contains(&offered.base.id) {
        // Never bought here, so the lines carry the counts and no source.
        let counts = recipe
            .parts
            .iter()
            .map(|(item, count)| format!("{count} {}", items.design(item).name))
            .collect();
        (
            None,
            "This station cannot craft this section".to_string(),
            counts,
            false,
        )
    } else {
        let recipe_text = recipe_lines(items, recipe, stock, own);
        match plan_recipe_payment(items, recipe, stock, stock_cr, own, own_cr) {
            Ok(payment) => (
                Some(cr_text(payment.price_cr)),
                cr_text(payment.price_cr),
                recipe_text,
                true,
            ),
            Err(refusal) => {
                let placeable = !matches!(refusal, RecipePaymentRefusalType::MissingParts { .. });
                (
                    placeable.then(|| tile_refusal_text(&refusal)),
                    payment_refusal_text(items, name, &refusal),
                    recipe_text,
                    placeable,
                )
            }
        }
    };
    GalleryEntry {
        id: offered.base.id.clone(),
        tile,
        note,
        recipe: recipe_text,
        alternate: None,
        placeable,
    }
}

/// One line per recipe item, in id order: `<count> <name>: <h> from the
/// hold, <b> bought at <ask> each, <s> short`, each source only when nonzero.
/// The split is [`plan_recipe_payment`]'s: the ship's hold first, then the
/// station's stock.
fn recipe_lines(
    items: &GameItems,
    recipe: &BuildRecipe,
    stock: &ShipInventory,
    own: &ShipInventory,
) -> Vec<String> {
    recipe
        .parts
        .iter()
        .map(|(item, &count)| {
            let design = items.design(item);
            let held = own.count(item).min(count);
            let bought = (count - held).min(stock.count(item));
            let short = count - held - bought;
            let mut sources = Vec::new();
            if held > 0 {
                sources.push(format!("{held} from the hold"));
            }
            if bought > 0 {
                sources.push(format!(
                    "{bought} bought at {} each",
                    cr_text(design.ask_cr)
                ));
            }
            if short > 0 {
                sources.push(format!("{short} short"));
            }
            format!("{count} {}: {}", design.name, sources.join(", "))
        })
        .collect()
}

/// Why a part cannot be paid, in the few words its gallery tile has room for.
/// The focus card gives the whole reason.
pub(crate) fn tile_refusal_text(refusal: &RecipePaymentRefusalType) -> String {
    match refusal {
        RecipePaymentRefusalType::MissingParts { .. } => "Short of parts".to_string(),
        RecipePaymentRefusalType::InsufficientCredits { need, .. } => {
            format!("Needs {}", cr_text(*need))
        }
        RecipePaymentRefusalType::CreditOverflow => "Price too high".to_string(),
    }
}

/// Show the Catalogue button while the ship is docked at a living station
/// that refits and the view shows the player ship, not a
/// [`ShipViewSubject`] (cached as `ShipRuntime::refits` for the footer), and
/// keep the open gallery's listing current: a price, a refusal, the stock or
/// the selection's swap offers change it.
pub(crate) fn update_ship_gallery_listing(
    mut runtime: ResMut<ShipRuntime>,
    subject: Res<ShipViewSubject>,
    items: Option<Res<GameItems>>,
    sections: Option<Res<GameSections>>,
    site: StationSite,
    q_holds: Query<(&ShipInventory, &ShipCredits)>,
    mut q_button: Query<&mut Node, With<ShipCatalogueButton>>,
    mut q_host: Query<&mut GalleryListing, With<ShipGalleryHost>>,
) {
    let refits = runtime.active && subject.0.is_none() && site.gate(StationServiceType::Refit);
    if runtime.refits != refits {
        runtime.refits = refits;
    }
    let display = if refits { Display::Flex } else { Display::None };
    for mut node in &mut q_button {
        if node.display != display {
            node.display = display;
        }
    }
    let Ok(mut shown) = q_host.single_mut() else {
        return;
    };
    let (Some(items), Some(sections)) = (items, sections) else {
        return;
    };
    let Some(listing) = station_listing(&runtime, &items, &sections, &site, &q_holds) else {
        return;
    };
    if *shown != listing {
        *shown = listing;
    }
}

/// Put the gallery over the pane while it is open and the ship is docked at
/// a station that refits, and take it away otherwise: the host node over the
/// pane body, showing the image its camera renders the tiles into. A gallery
/// this pane left open when the pane went away, or the dock ended, is closed,
/// and its camera goes with it when the host went first.
#[expect(
    clippy::too_many_arguments,
    reason = "one system owning the host node, its camera and its image"
)]
pub(crate) fn manage_ship_gallery(
    mut commands: Commands,
    runtime: Res<ShipRuntime>,
    subject: Res<ShipViewSubject>,
    mut state: ResMut<GalleryState>,
    theme: Res<ActiveUiTheme>,
    site: StationSite,
    images: Option<ResMut<Assets<Image>>>,
    q_split: Query<&ChildOf, With<ShipSplitMarker>>,
    q_cam: Query<Entity, With<ShipCatalogueCameraMarker>>,
    mut q_host: Query<(
        Entity,
        &ShipGalleryHost,
        &GalleryHost,
        &ComputedNode,
        &mut ImageNode,
    )>,
    mut q_camera: Query<&mut Projection>,
) {
    // The gallery state is shared with the editor's host, so only a gallery
    // this pane's camera draws is closed here: the editor's browse is not the
    // pane's. A tab switch or the interface closing despawns the host with
    // the pane body and leaves the camera, so the camera alone closes it.
    let refits = runtime.active && subject.0.is_none() && site.gate(StationServiceType::Refit);
    if !q_cam.is_empty() && (q_host.is_empty() || !state.open || !refits) {
        if state.open {
            state.open = false;
            state.focused = false;
        }
        for camera in &q_cam {
            commands.entity(camera).despawn();
        }
        for (host, ..) in &q_host {
            commands.entity(host).despawn();
        }
        return;
    }
    if !state.open || !refits {
        return;
    }
    let Some(mut images) = images else {
        return;
    };
    if let Ok((_, owned, gallery, computed, mut node)) = q_host.single_mut() {
        // A node not laid out yet measures zero; the image keeps its size.
        let desired = computed.size().round().as_uvec2();
        if desired.cmpeq(UVec2::ZERO).any() {
            return;
        }
        if node.image != owned.image {
            node.image = owned.image.clone();
        }
        resize_render_target(
            &mut images,
            &owned.image,
            desired,
            q_camera.get_mut(gallery.camera).ok(),
        );
        return;
    }
    let Ok(split) = q_split.single() else {
        return;
    };
    let body = split.parent();
    let image = images.add(new_render_target_image(UVec2::splat(64)));
    let layers = RenderLayers::layer(GALLERY_LAYER);
    let eye = Transform::default();
    let camera = commands
        .spawn((
            Name::new("ShipCatalogueCamera"),
            ShipCatalogueCameraMarker,
            Camera3d::default(),
            Camera {
                order: GALLERY_CAMERA_ORDER,
                clear_color: ClearColorConfig::Custom(theme.color(UiColor::Surface)),
                ..default()
            },
            RenderTarget::Image(ImageRenderTarget {
                handle: image.clone(),
                scale_factor: 1.0,
            }),
            AmbientLight {
                brightness: GALLERY_AMBIENT,
                ..default()
            },
            eye,
            layers.clone(),
        ))
        .with_children(|camera| {
            for (name, (bearing, illuminance)) in [
                ("ShipCatalogueKeyLight", GALLERY_KEY),
                ("ShipCatalogueRimLight", GALLERY_RIM),
            ] {
                camera.spawn((
                    Name::new(name),
                    DirectionalLight {
                        illuminance,
                        shadow_maps_enabled: false,
                        ..default()
                    },
                    Transform::from_translation(bearing).looking_at(Vec3::ZERO, Vec3::Y),
                    layers.clone(),
                ));
            }
        })
        .id();
    commands.spawn((
        Name::new("ShipCatalogueGallery"),
        ShipGalleryHost {
            image: image.clone(),
        },
        GalleryHost {
            camera,
            stage: eye.translation + eye.forward() * GALLERY_STAGE_DISTANCE,
            layers,
            close_keys: "Esc",
        },
        GalleryListing::default(),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            ..default()
        },
        ImageNode::new(image),
        // Above the split and the footer, its siblings.
        ZIndex(10),
        ChildOf(body),
    ));
}

/// Raise a [`SectionSwapCommand`] for the selected section when the
/// gallery's Swap verb is pressed for a part.
pub(crate) fn apply_ship_gallery_alternates(
    mut alternates: MessageReader<GalleryAlternate>,
    runtime: Res<ShipRuntime>,
    site: StationSite,
    mut swaps: MessageWriter<SectionSwapCommand>,
) {
    for alternate in alternates.read() {
        let Some(section) = site
            .resolve(StationServiceType::Refit)
            .and_then(|resolved| resolved.selected_section(runtime.selected).cloned())
        else {
            continue;
        };
        swaps.write(SectionSwapCommand {
            section: section.id,
            prototype: alternate.id.clone(),
        });
    }
}

#[cfg(test)]
mod tests {
    use nova_gameplay::test_support::test_items;

    use super::*;

    /// A PDC whose recipe is a turret body, a barrel and two section panels,
    /// plus 100 cr of work.
    fn pdc() -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: "pdc".to_string(),
                name: "PDC".to_string(),
                build: BuildRecipe {
                    credits: 100,
                    parts: [
                        ("TurretBody".into(), 1),
                        ("TurretBarrel".into(), 1),
                        ("SectionPanel".into(), 2),
                    ]
                    .into(),
                    strip_yield: 0.5,
                },
                ..default()
            },
            kind: SectionKind::Turret(TurretSectionConfig::default()),
        }
    }

    fn hold(items: &GameItems, stacks: &[(&str, u32)]) -> ShipInventory {
        ShipInventory::new(
            items,
            10_000_000,
            stacks
                .iter()
                .map(|(item, count)| (ItemDesignId::from(*item), *count)),
        )
    }

    fn builds_pdc() -> Vec<String> {
        vec!["pdc".to_string()]
    }

    /// A part the station does not build is listed muted with no tile line,
    /// whatever both holds carry, and its note is the concise reason. Nothing
    /// is ever bought for it, so its recipe lines are counts with no source.
    #[test]
    fn a_part_the_station_does_not_build_is_listed_muted_with_its_reason() {
        let items = test_items();
        let stock = hold(
            &items,
            &[("TurretBody", 1), ("TurretBarrel", 1), ("SectionPanel", 2)],
        );
        let entry = offered_entry(
            &items,
            &pdc(),
            &["hull".to_string()],
            &stock,
            0,
            &stock,
            10_000,
        );

        assert!(!entry.placeable);
        assert_eq!(entry.tile, None);
        assert_eq!(entry.note, "This station cannot craft this section");
        assert_eq!(
            entry.recipe,
            ["2 Section panel", "1 Turret barrel", "1 Turret body"]
        );
    }

    /// A built part neither hold covers is muted with no tile line, its note
    /// names the first short item and by how many, and its lines say which
    /// item is short.
    #[test]
    fn a_part_short_of_a_recipe_item_names_the_item_and_the_shortfall() {
        let items = test_items();
        let own = hold(&items, &[("SectionPanel", 1)]);
        let stock = hold(&items, &[("TurretBody", 1), ("TurretBarrel", 1)]);
        let entry = offered_entry(&items, &pdc(), &builds_pdc(), &stock, 0, &own, 10_000);

        assert!(!entry.placeable);
        assert_eq!(entry.tile, None);
        assert_eq!(
            entry.note,
            "PDC needs 1 more Section panel than both holds carry"
        );
        assert_eq!(entry.recipe[0], "2 Section panel: 1 from the hold, 1 short");
    }

    /// A payable part's tile is its price alone, its recipe lists each item
    /// with its source, and the lines split the parts as the payment does.
    #[test]
    fn a_payable_part_lists_each_recipe_item_with_its_source() {
        let items = test_items();
        let own = hold(&items, &[("SectionPanel", 2), ("TurretBarrel", 1)]);
        let stock = hold(&items, &[("TurretBody", 3)]);
        let entry = offered_entry(&items, &pdc(), &builds_pdc(), &stock, 0, &own, 10_000);

        assert!(entry.placeable);
        assert_eq!(
            entry.recipe,
            [
                "2 Section panel: 2 from the hold",
                "1 Turret barrel: 1 from the hold",
                "1 Turret body: 1 bought at 200 cr each",
            ]
        );
        let payment = plan_recipe_payment(&items, &pdc().base.build, &stock, 0, &own, 10_000)
            .expect("the holds carry the recipe and the ship the price");
        assert_eq!(
            payment.from_hold,
            [("SectionPanel".into(), 2), ("TurretBarrel".into(), 1)].into()
        );
        assert_eq!(payment.bought, [("TurretBody".into(), 1)].into());
        assert_eq!(entry.tile.as_deref(), Some("300 cr"));
        assert_eq!(entry.note, "300 cr");
        assert_eq!(payment.price_cr, 300);
    }

    /// Credits do not mute: a part the ship cannot pay for stays placeable
    /// and says the price, and the placement ghost refuses the click.
    #[test]
    fn a_credit_short_part_stays_placeable() {
        let items = test_items();
        let own = hold(&items, &[("SectionPanel", 2), ("TurretBarrel", 1)]);
        let stock = hold(&items, &[("TurretBody", 1)]);
        let entry = offered_entry(&items, &pdc(), &builds_pdc(), &stock, 0, &own, 50);

        assert!(entry.placeable);
        assert_eq!(entry.tile.as_deref(), Some("Needs 300 cr"));
        assert_eq!(entry.note, "PDC costs 300 cr, the ship holds 50 cr");
        assert_eq!(entry.recipe.len(), 3);
    }
}
