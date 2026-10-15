//! The Ship pane's station catalogue: the parts gallery (`nova_ship::gallery`)
//! over the pane while the ship is docked at a station that refits.
//!
//! The footer's Catalogue button opens it. [`manage_ship_gallery`] then puts a
//! [`GalleryHost`] node over the view and the panel, with its own camera on
//! [`GALLERY_LAYER`] rendering the part tiles into an image the node shows
//! 1:1, and takes both away when the gallery closes or the pane goes.
//!
//! [`update_ship_gallery_listing`] lists the parts the station can build: a
//! part not hidden from the editor palette whose recipe the player's hold and
//! the station's stock together carry ([`plan_recipe_payment`] finds no
//! missing part). Credits do not filter: a part the ship cannot pay for shows
//! the credit refusal as its note. A part that can replace the selected
//! section offers Swap as the focus card's second verb, and the note names
//! the swap price or why the swap is refused.
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
    rebuild::{payment_refusal_text, rebuild_price_text, StationSite},
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

/// Marks the pane's split, the node the catalogue covers while it is open.
#[derive(Component)]
pub(crate) struct ShipSplitMarker;

/// Marks the footer's Catalogue button, shown while the station builds parts.
#[derive(Component)]
pub(crate) struct ShipCatalogueButton;

/// The catalogue's own camera, which holds its lights, and the image it
/// renders the tiles into. Worn by the host node, which takes the camera
/// with it.
#[derive(Component)]
pub(crate) struct ShipGalleryHost {
    camera: Entity,
    image: Handle<Image>,
}

/// The listing for the docked refit station: each listed part with its price
/// or refusal, and Swap when it can replace the selected section. `None`
/// while the ship is not docked at a living station that refits.
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
        .and_then(|(partner, _)| q_holds.get(partner).ok())
        .map(|(stock, credits)| (stock, credits.0));
    let (stock, stock_cr) = partner?;
    let selected = resolved.selected_section(runtime.selected);
    let mut entries = Vec::new();
    for offered in sections
        .iter()
        .filter(|offered| !offered.base.hide_in_editor)
    {
        let recipe = &offered.base.build;
        if matches!(
            plan_recipe_payment(items, recipe, stock, stock_cr, own, u32::MAX),
            Err(RecipePaymentRefusalType::MissingParts { .. })
        ) {
            continue;
        }
        let name = &offered.base.name;
        let (tile, mut note) =
            match plan_recipe_payment(items, recipe, stock, stock_cr, own, own_cr.0) {
                Ok(payment) => (tile_price_text(&payment), rebuild_price_text(&payment)),
                Err(refusal) => (
                    tile_refusal_text(&refusal),
                    payment_refusal_text(items, name, &refusal),
                ),
            };
        let mut alternate = None;
        if let Some(section) =
            selected.filter(|section| section_swap_family(&section.config, offered).is_ok())
        {
            let code = design_section_code(resolved.design, &section.id)
                .expect("a live design section has a code");
            match resolved.plan_swap(items, section, offered, own, own_cr.0, partner) {
                Ok(payment) => {
                    note.push_str(&format!("\nSwap {code}: {}", rebuild_price_text(&payment)));
                    alternate = Some(format!("Swap {code}"));
                }
                Err(refusal) => {
                    note.push('\n');
                    note.push_str(&swap_refusal_text(items, &code, name, &refusal));
                }
            }
        }
        entries.push(GalleryEntry {
            id: offered.base.id.clone(),
            tile,
            note,
            alternate,
        });
    }
    Some(GalleryListing(entries))
}

/// A part's price on its gallery tile: the credits and how many parts the
/// station sells into it. The focus card names the parts from the hold.
fn tile_price_text(payment: &RecipePayment) -> String {
    let bought: u32 = payment.bought.values().sum();
    format!("{} cr, {bought} bought", payment.price_cr)
}

/// Why a part cannot be paid, in the few words its gallery tile has room for.
/// The focus card gives the whole reason.
fn tile_refusal_text(refusal: &RecipePaymentRefusalType) -> String {
    match refusal {
        RecipePaymentRefusalType::MissingParts { .. } => "Short of parts".to_string(),
        RecipePaymentRefusalType::InsufficientCredits { need, .. } => format!("Needs {need} cr"),
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
/// split, showing the image its camera renders the tiles into. A gallery this
/// pane left open when the pane went away, or the dock ended, is closed.
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
    q_split: Query<Entity, With<ShipSplitMarker>>,
    mut q_host: Query<(Entity, &ShipGalleryHost, &ComputedNode, &mut ImageNode)>,
    mut q_camera: Query<&mut Projection>,
) {
    // The gallery state is shared with the editor's host, so only a gallery
    // this pane hosts is closed here: the editor's browse is not the pane's.
    let refits = runtime.active && subject.0.is_none() && site.gate(StationServiceType::Refit);
    if !q_host.is_empty() && (!state.open || !refits) {
        if state.open {
            state.open = false;
            state.focused = false;
        }
        for (host, owned, _, _) in &q_host {
            commands.entity(owned.camera).despawn();
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
    if let Ok((_, owned, computed, mut node)) = q_host.single_mut() {
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
            q_camera.get_mut(owned.camera).ok(),
        );
        return;
    }
    let Ok(split) = q_split.single() else {
        return;
    };
    let image = images.add(new_render_target_image(UVec2::splat(64)));
    let layers = RenderLayers::layer(GALLERY_LAYER);
    let eye = Transform::default();
    let camera = commands
        .spawn((
            Name::new("ShipCatalogueCamera"),
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
            camera,
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
        // Above the view's badges and the panel, its siblings.
        ZIndex(10),
        ChildOf(split),
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
