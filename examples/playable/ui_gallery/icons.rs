//! The gallery's icon masks: one white silhouette per job kind, chart
//! content, item category, role and control, drawn once at startup as a
//! coverage mask and tinted by the theme where it is shown. The same recipe
//! as the game's `nova_interface::icons`, re-created here because that module
//! is crate-private.

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use nova_ui::{theme::UiColor, widget::ThemedImageTint};

/// Every icon the gallery draws.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum IconType {
    // Job kinds.
    Bring,
    Recover,
    Escort,
    Answer,
    Neutralize,
    // Chart contents and marks.
    Asteroids,
    Rocks,
    Planet,
    Derelicts,
    Station,
    Wreck,
    Job,
    You,
    // Item categories, the game's own silhouettes.
    Raw,
    Repair,
    Ammo,
    Food,
    Parts,
    // Roles and controls.
    Pirate,
    Milestone,
    Randomize,
}

impl IconType {
    pub const ALL: [IconType; 21] = [
        IconType::Bring,
        IconType::Recover,
        IconType::Escort,
        IconType::Answer,
        IconType::Neutralize,
        IconType::Asteroids,
        IconType::Rocks,
        IconType::Planet,
        IconType::Derelicts,
        IconType::Station,
        IconType::Wreck,
        IconType::Job,
        IconType::You,
        IconType::Raw,
        IconType::Repair,
        IconType::Ammo,
        IconType::Food,
        IconType::Parts,
        IconType::Pirate,
        IconType::Milestone,
        IconType::Randomize,
    ];

    /// The icon sheet's families, in its order.
    pub const FAMILIES: [(&'static str, &'static [IconType]); 5] = [
        (
            "Job kinds",
            &[
                IconType::Bring,
                IconType::Recover,
                IconType::Escort,
                IconType::Answer,
                IconType::Neutralize,
            ],
        ),
        (
            "Chart",
            &[
                IconType::Asteroids,
                IconType::Rocks,
                IconType::Planet,
                IconType::Derelicts,
                IconType::Station,
                IconType::Wreck,
                IconType::Job,
                IconType::You,
            ],
        ),
        (
            "Item categories",
            &[
                IconType::Raw,
                IconType::Repair,
                IconType::Ammo,
                IconType::Food,
                IconType::Parts,
            ],
        ),
        ("Roles", &[IconType::Pirate]),
        ("Controls", &[IconType::Milestone, IconType::Randomize]),
    ];

    pub fn label(self) -> &'static str {
        match self {
            IconType::Bring => "Bring",
            IconType::Recover => "Recover",
            IconType::Escort => "Escort",
            IconType::Answer => "Answer",
            IconType::Neutralize => "Neutralize",
            IconType::Asteroids => "Asteroids",
            IconType::Rocks => "Rocks",
            IconType::Planet => "Planet",
            IconType::Derelicts => "Derelicts",
            IconType::Station => "Station",
            IconType::Wreck => "Wreck",
            IconType::Job => "Job",
            IconType::You => "You",
            IconType::Raw => "Raw",
            IconType::Repair => "Repair",
            IconType::Ammo => "Ammo",
            IconType::Food => "Food",
            IconType::Parts => "Parts",
            IconType::Pirate => "Pirate",
            IconType::Milestone => "Milestone",
            IconType::Randomize => "Randomize",
        }
    }

    /// The tint an icon reads in where no caller picks one.
    pub fn color(self) -> UiColor {
        match self {
            IconType::Bring
            | IconType::Recover
            | IconType::Escort
            | IconType::Answer
            | IconType::Neutralize
            | IconType::Job => UiColor::Accent,
            IconType::Asteroids => UiColor::Accent,
            IconType::Rocks => UiColor::Secondary,
            IconType::Planet => UiColor::Info,
            IconType::Derelicts => UiColor::Body,
            IconType::Station => UiColor::Nominal,
            IconType::Wreck => UiColor::AccentHigh,
            IconType::You => UiColor::Primary,
            IconType::Raw => UiColor::Nominal,
            IconType::Repair => UiColor::Info,
            IconType::Ammo => UiColor::Danger,
            IconType::Food => UiColor::Accent,
            IconType::Parts => UiColor::AccentHigh,
            IconType::Pirate => UiColor::Danger,
            IconType::Milestone => UiColor::Nominal,
            IconType::Randomize => UiColor::Primary,
        }
    }
}

// Each mask's coverage takes `p`, a point in `[-1, 1]` squared with y down,
// and `px`, the width of one texel in those units.

/// Coverage of a shape whose signed distance at a texel is `d`.
fn solid(d: f32, px: f32) -> f32 {
    (0.5 - d / px).clamp(0.0, 1.0)
}

fn circle(p: Vec2, c: Vec2, r: f32) -> f32 {
    (p - c).length() - r
}

fn ring(p: Vec2, c: Vec2, r: f32, w: f32) -> f32 {
    ((p - c).length() - r).abs() - w * 0.5
}

fn segment(p: Vec2, a: Vec2, b: Vec2, w: f32) -> f32 {
    let t = ((p - a).dot(b - a) / (b - a).length_squared()).clamp(0.0, 1.0);
    (p - (a + (b - a) * t)).length() - w * 0.5
}

/// Signed distance to a convex polygon, wound clockwise on screen.
fn polygon(p: Vec2, points: &[Vec2]) -> f32 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| {
            let edge = *b - *a;
            let normal = Vec2::new(edge.y, -edge.x).normalize();
            (p - *a).dot(normal)
        })
        .fold(f32::NEG_INFINITY, f32::max)
}

/// An axis-aligned box outline of half size `h`, stroke `w`.
fn box_outline(p: Vec2, c: Vec2, h: Vec2, w: f32) -> f32 {
    let q = (p - c).abs() - h;
    let outside = q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0);
    outside.abs() - w * 0.5
}

/// A chevron pointing right, its tip at `tip`, `len` long, stroke `w`.
fn chevron(p: Vec2, tip: Vec2, len: f32, w: f32) -> f32 {
    segment(p, tip + Vec2::new(-len, -len), tip, w).min(segment(
        p,
        tip + Vec2::new(-len, len),
        tip,
        w,
    ))
}

fn coverage(icon: IconType, p: Vec2, px: f32) -> f32 {
    match icon {
        // A crate: a box outline with a strap across and a lid line.
        IconType::Bring => {
            let shell = box_outline(p, Vec2::new(0.0, 0.1), Vec2::new(0.72, 0.62), 0.14);
            let lid = segment(p, Vec2::new(-0.72, -0.22), Vec2::new(0.72, -0.22), 0.1);
            let strap = segment(p, Vec2::new(0.0, -0.22), Vec2::new(0.0, 0.72), 0.1);
            solid(shell.min(lid).min(strap), px)
        }
        // A hook over a wreck: a hooked line descending onto a broken hull.
        IconType::Recover => {
            let line = segment(p, Vec2::new(0.1, -0.95), Vec2::new(0.1, -0.2), 0.12);
            let hook = ring(p, Vec2::new(-0.12, 0.0), 0.24, 0.12).max(p.y - 0.12);
            let hull = polygon(
                p,
                &[
                    Vec2::new(-0.85, 0.5),
                    Vec2::new(0.4, 0.5),
                    Vec2::new(0.7, 0.85),
                    Vec2::new(-0.6, 0.85),
                ],
            );
            let crack = segment(p, Vec2::new(-0.1, 0.45), Vec2::new(0.15, 0.9), 0.1);
            solid(line.min(hook), px).max(solid(hull.max(-crack), px) * 0.8)
        }
        // Two chevrons, the escort beside the hauler.
        IconType::Escort => {
            let a = chevron(p, Vec2::new(0.1, 0.0), 0.55, 0.16);
            let b = chevron(p, Vec2::new(0.75, 0.0), 0.55, 0.16);
            solid(a.min(b), px)
        }
        // A beacon: a ring with an exclamation mark inside.
        IconType::Answer => {
            let beacon = ring(p, Vec2::ZERO, 0.78, 0.13);
            let bar = segment(p, Vec2::new(0.0, -0.45), Vec2::new(0.0, 0.1), 0.17);
            let dot = circle(p, Vec2::new(0.0, 0.42), 0.11);
            solid(beacon.min(bar).min(dot), px)
        }
        // A crosshair: a ring, four ticks and a centre dot.
        IconType::Neutralize => {
            let ticks = [Vec2::X, Vec2::NEG_X, Vec2::Y, Vec2::NEG_Y]
                .into_iter()
                .map(|axis| segment(p, axis * 0.35, axis * 0.95, 0.13))
                .fold(f32::INFINITY, f32::min);
            let reticle = ring(p, Vec2::ZERO, 0.6, 0.13)
                .min(ticks)
                .min(circle(p, Vec2::ZERO, 0.1));
            solid(reticle, px)
        }
        // Three lumps.
        IconType::Asteroids => {
            let lumps = polygon(
                p,
                &[
                    Vec2::new(-0.95, 0.1),
                    Vec2::new(-0.55, -0.35),
                    Vec2::new(-0.1, -0.1),
                    Vec2::new(-0.25, 0.45),
                    Vec2::new(-0.8, 0.55),
                ],
            )
            .min(polygon(
                p,
                &[
                    Vec2::new(0.1, -0.9),
                    Vec2::new(0.6, -0.8),
                    Vec2::new(0.75, -0.3),
                    Vec2::new(0.3, -0.05),
                    Vec2::new(0.0, -0.4),
                ],
            ))
            .min(polygon(
                p,
                &[
                    Vec2::new(0.2, 0.2),
                    Vec2::new(0.85, 0.15),
                    Vec2::new(0.9, 0.7),
                    Vec2::new(0.45, 0.9),
                    Vec2::new(0.1, 0.6),
                ],
            ));
            solid(lumps, px)
        }
        // A scatter of small rocks.
        IconType::Rocks => [
            (Vec2::new(-0.6, -0.5), 0.2),
            (Vec2::new(0.4, -0.65), 0.14),
            (Vec2::new(0.65, 0.1), 0.18),
            (Vec2::new(-0.1, 0.05), 0.12),
            (Vec2::new(-0.65, 0.55), 0.15),
            (Vec2::new(0.25, 0.65), 0.21),
        ]
        .into_iter()
        .map(|(c, r)| solid(circle(p, c, r), px))
        .fold(0.0, f32::max),
        // A ringed disc, the game's planet body.
        IconType::Planet => {
            let disc = circle(p, Vec2::ZERO, 0.5);
            let band = ring(p, Vec2::ZERO, 0.8, 0.1).max(p.y.abs() - 0.18);
            solid(disc, px).max(solid(band, px))
        }
        // A broken hull: two hull halves with a crack between.
        IconType::Derelicts => {
            let bow = polygon(
                p,
                &[
                    Vec2::new(-0.9, -0.1),
                    Vec2::new(-0.3, -0.55),
                    Vec2::new(-0.05, -0.2),
                    Vec2::new(-0.2, 0.35),
                    Vec2::new(-0.75, 0.4),
                ],
            );
            let stern = polygon(
                p,
                &[
                    Vec2::new(0.15, -0.35),
                    Vec2::new(0.85, -0.5),
                    Vec2::new(0.9, 0.3),
                    Vec2::new(0.35, 0.55),
                    Vec2::new(0.1, 0.1),
                ],
            );
            solid(bow.min(stern), px)
        }
        // The game's station silhouette: a cube outline in projection.
        IconType::Station => {
            let front = box_outline(p, Vec2::new(-0.15, 0.15), Vec2::new(0.55, 0.55), 0.12);
            let back = box_outline(p, Vec2::new(0.2, -0.2), Vec2::new(0.55, 0.55), 0.08);
            let edges = [
                (Vec2::new(-0.7, -0.4), Vec2::new(-0.35, -0.75)),
                (Vec2::new(0.4, -0.4), Vec2::new(0.75, -0.75)),
                (Vec2::new(0.4, 0.7), Vec2::new(0.75, 0.35)),
            ]
            .into_iter()
            .map(|(a, b)| segment(p, a, b, 0.08))
            .fold(f32::INFINITY, f32::min);
            solid(front, px).max(solid(back.min(edges), px) * 0.6)
        }
        // A cracked outline: a hull outline split by a jagged crack.
        IconType::Wreck => {
            let shell = polygon(
                p,
                &[
                    Vec2::new(-0.9, 0.0),
                    Vec2::new(-0.4, -0.7),
                    Vec2::new(0.6, -0.6),
                    Vec2::new(0.9, 0.1),
                    Vec2::new(0.4, 0.75),
                    Vec2::new(-0.5, 0.65),
                ],
            );
            let crack = segment(p, Vec2::new(-0.1, -0.75), Vec2::new(0.15, -0.15), 0.16)
                .min(segment(
                    p,
                    Vec2::new(0.15, -0.15),
                    Vec2::new(-0.15, 0.25),
                    0.16,
                ))
                .min(segment(
                    p,
                    Vec2::new(-0.15, 0.25),
                    Vec2::new(0.1, 0.8),
                    0.16,
                ));
            solid((shell.abs() - 0.07).max(-crack), px)
        }
        // A diamond, the game's nav point.
        IconType::Job => {
            let diamond = (p.x.abs() + p.y.abs() - 0.8) * std::f32::consts::FRAC_1_SQRT_2;
            solid(diamond.abs() - 0.09, px).max(solid(circle(p, Vec2::ZERO, 0.16), px))
        }
        // The hub dot: the map's own-ship sphere with a faint ring.
        IconType::You => solid(circle(p, Vec2::ZERO, 0.32), px)
            .max(solid(ring(p, Vec2::ZERO, 0.7, 0.08), px) * 0.6),
        IconType::Raw => {
            let lumps = circle(p, Vec2::new(-0.4, 0.4), 0.36)
                .min(circle(p, Vec2::new(0.4, 0.42), 0.34))
                .min(circle(p, Vec2::new(0.0, -0.2), 0.4));
            solid(lumps, px)
        }
        IconType::Repair => {
            let head = ring(p, Vec2::new(-0.35, -0.35), 0.3, 0.2);
            let jaw = circle(p, Vec2::new(-0.62, -0.62), 0.22);
            let handle = segment(p, Vec2::new(-0.15, -0.15), Vec2::new(0.7, 0.7), 0.24);
            solid(head.max(-jaw).min(handle), px)
        }
        IconType::Ammo => [-0.5f32, 0.0, 0.5]
            .into_iter()
            .map(|x| {
                let case = polygon(
                    p,
                    &[
                        Vec2::new(x - 0.16, -0.2),
                        Vec2::new(x + 0.16, -0.2),
                        Vec2::new(x + 0.16, 0.85),
                        Vec2::new(x - 0.16, 0.85),
                    ],
                );
                let tip = circle(p, Vec2::new(x, -0.3), 0.16).max(p.y - (-0.2));
                let nose = polygon(
                    p,
                    &[
                        Vec2::new(x, -0.85),
                        Vec2::new(x + 0.16, -0.3),
                        Vec2::new(x - 0.16, -0.3),
                    ],
                );
                solid(case.min(tip).min(nose), px)
            })
            .fold(0.0, f32::max),
        IconType::Food => {
            let tin = polygon(
                p,
                &[
                    Vec2::new(-0.6, -0.55),
                    Vec2::new(0.6, -0.55),
                    Vec2::new(0.6, 0.75),
                    Vec2::new(-0.6, 0.75),
                ],
            );
            let lid = segment(p, Vec2::new(-0.72, -0.72), Vec2::new(0.72, -0.72), 0.14);
            let band = segment(p, Vec2::new(-0.6, 0.1), Vec2::new(0.6, 0.1), 0.1);
            (solid(tin, px) * 0.55)
                .max(solid(tin.abs() - 0.06, px))
                .max(solid(lid, px))
                .max(solid(band, px))
        }
        IconType::Parts => {
            let teeth = (0..8)
                .map(|tooth| {
                    let axis = Vec2::from_angle(std::f32::consts::FRAC_PI_4 * tooth as f32);
                    segment(p, axis * 0.5, axis * 0.88, 0.22)
                })
                .fold(f32::INFINITY, f32::min);
            let wheel = ring(p, Vec2::ZERO, 0.42, 0.26);
            solid(wheel.min(teeth), px)
        }
        // A blade: a slanted edge with a crossguard.
        IconType::Pirate => {
            let blade = polygon(
                p,
                &[
                    Vec2::new(0.75, -0.85),
                    Vec2::new(0.9, -0.6),
                    Vec2::new(-0.1, 0.35),
                    Vec2::new(-0.3, 0.15),
                ],
            );
            let guard = segment(p, Vec2::new(-0.55, 0.0), Vec2::new(-0.05, 0.5), 0.14);
            let grip = segment(p, Vec2::new(-0.3, 0.25), Vec2::new(-0.8, 0.8), 0.18);
            solid(blade.min(guard).min(grip), px)
        }
        // A flag on a pole: the milestone.
        IconType::Milestone => {
            let pole = segment(p, Vec2::new(-0.6, -0.9), Vec2::new(-0.6, 0.9), 0.13);
            let flag = polygon(
                p,
                &[
                    Vec2::new(-0.5, -0.85),
                    Vec2::new(0.8, -0.5),
                    Vec2::new(-0.5, -0.1),
                ],
            );
            solid(pole.min(flag), px)
        }
        // Two arrows chasing each other around a ring.
        IconType::Randomize => {
            let arc = ring(p, Vec2::ZERO, 0.62, 0.14);
            let gaps = (p.y.abs() - 0.22).max(0.3 - p.x.abs());
            let heads = polygon(
                p,
                &[
                    Vec2::new(0.42, -0.62),
                    Vec2::new(0.92, -0.3),
                    Vec2::new(0.4, 0.02),
                ],
            )
            .min(polygon(
                p,
                &[
                    Vec2::new(-0.42, 0.62),
                    Vec2::new(-0.92, 0.3),
                    Vec2::new(-0.4, -0.02),
                ],
            ));
            solid(arc.max(gaps).min(heads), px)
        }
    }
}

/// Icon mask edge, in texels.
const ICON_TEXELS: u32 = 64;

/// Draw one mask from its coverage.
fn icon_mask(images: &mut Assets<Image>, icon: IconType) -> Handle<Image> {
    let texel = 2.0 / ICON_TEXELS as f32;
    let mut data = Vec::with_capacity((ICON_TEXELS * ICON_TEXELS * 4) as usize);
    for y in 0..ICON_TEXELS {
        for x in 0..ICON_TEXELS {
            let at = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * texel - Vec2::ONE;
            let alpha = coverage(icon, at, texel);
            data.extend_from_slice(&[255, 255, 255, (alpha * 255.0).round() as u8]);
        }
    }
    images.add(Image::new(
        Extent3d {
            width: ICON_TEXELS,
            height: ICON_TEXELS,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

/// Every icon mask, in [`IconType::ALL`] order.
#[derive(Resource)]
pub struct GalleryIcons(Vec<Handle<Image>>);

impl GalleryIcons {
    pub fn get(&self, icon: IconType) -> Handle<Image> {
        let index = IconType::ALL
            .iter()
            .position(|each| *each == icon)
            .expect("IconType::ALL lists every icon");
        self.0[index].clone()
    }
}

/// Draw every icon mask once.
pub fn init_gallery_icons(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(GalleryIcons(
        IconType::ALL
            .iter()
            .map(|icon| icon_mask(&mut images, *icon))
            .collect(),
    ));
}

/// An icon mask tinted by the theme, `size` px square.
pub fn icon_node(icons: &GalleryIcons, icon: IconType, tint: UiColor, size: f32) -> impl Bundle {
    (
        ImageNode::new(icons.get(icon)),
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            flex_shrink: 0.0,
            ..default()
        },
        ThemedImageTint::new(tint),
        Pickable::IGNORE,
    )
}

/// An icon in its own tint.
pub fn icon(icons: &GalleryIcons, icon: IconType, size: f32) -> impl Bundle {
    icon_node(icons, icon, icon.color(), size)
}
