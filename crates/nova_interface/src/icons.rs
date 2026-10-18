//! The icon masks the interface panes draw: one silhouette per section
//! family, per kind of map body, per item category, per chart mark and per
//! job kind.
//!
//! Each mask is a white shape whose alpha is its coverage, drawn once at
//! startup and tinted by the theme where it is shown, so a theme change never
//! redraws a mask.
//!
//! Touch this module when changing what a section family, a map body, a
//! chart mark or a job kind looks like.

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use nova_gameplay::prelude::{ClusterType, ItemCategoryType, JobType, SectionClass};
use nova_ui::{theme::UiColor, widget::ThemedImageTint};

/// How the panes draw a section kind: one icon, tint and legend word per
/// family, so a weapon reads as a weapon whatever it fires.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SectionIconType {
    Weapon,
    Thruster,
    Controller,
    Hull,
    Docking,
    Mining,
    CargoIntake,
}

/// What a map contact is, as the map draws it: a ship, a rock, a planet or a
/// nav point.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BodyIconType {
    Ship,
    Asteroid,
    Planet,
    Objective,
}

/// What the Map's chart view, its panel, its log and its legend draw a find
/// as: a cell's contents by cluster type, a cell itself, the found marks and
/// the own ship.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ChartIconType {
    Asteroids,
    Rocks,
    Planet,
    Derelicts,
    /// A visited cell with no cluster body, in the log.
    Sector,
    Station,
    Wreck,
    Job,
    You,
}

/// What the Jobs pane draws a job kind as, so a board scans by shape.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum JobIconType {
    Bring,
    Recover,
    Escort,
    Answer,
    Neutralize,
}

/// Every icon mask the panes draw, in each family's `ALL` order.
#[derive(Resource)]
pub(crate) struct InterfaceIcons {
    pub(crate) sections: [Handle<Image>; SectionIconType::ALL.len()],
    pub(crate) bodies: [Handle<Image>; BodyIconType::ALL.len()],
    pub(crate) categories: [Handle<Image>; ITEM_CATEGORIES.len()],
    pub(crate) charts: [Handle<Image>; ChartIconType::ALL.len()],
    pub(crate) jobs: [Handle<Image>; JobIconType::ALL.len()],
}

impl InterfaceIcons {
    /// The mask of a section family.
    pub(crate) fn section(&self, icon: SectionIconType) -> Handle<Image> {
        self.sections[icon as usize].clone()
    }

    /// The mask of a map body.
    pub(crate) fn body(&self, icon: BodyIconType) -> Handle<Image> {
        self.bodies[icon as usize].clone()
    }

    /// The mask of an item category.
    pub(crate) fn category(&self, category: ItemCategoryType) -> Handle<Image> {
        self.categories[category as usize].clone()
    }

    /// The mask of a chart mark.
    pub(crate) fn chart(&self, icon: ChartIconType) -> Handle<Image> {
        self.charts[icon as usize].clone()
    }

    /// The mask of a job kind.
    pub(crate) fn job(&self, icon: JobIconType) -> Handle<Image> {
        self.jobs[icon as usize].clone()
    }

    /// Default handles for every mask, for tests that build pane markup
    /// without drawing images.
    #[cfg(test)]
    pub(crate) fn blank() -> Self {
        Self {
            sections: std::array::from_fn(|_| Handle::default()),
            bodies: std::array::from_fn(|_| Handle::default()),
            categories: std::array::from_fn(|_| Handle::default()),
            charts: std::array::from_fn(|_| Handle::default()),
            jobs: std::array::from_fn(|_| Handle::default()),
        }
    }
}

// Each mask's coverage takes `p`, a point in `[-1, 1]` squared with y down,
// and `px`, the width of one texel in those units.

/// Coverage of a shape whose signed distance at a texel is `d`.
fn solid(d: f32, px: f32) -> f32 {
    (0.5 - d / px).clamp(0.0, 1.0)
}

/// Signed distance to a disc.
fn circle(p: Vec2, c: Vec2, r: f32) -> f32 {
    (p - c).length() - r
}

/// Signed distance to a ring of radius `r` and width `w` around `c`.
fn ring(p: Vec2, c: Vec2, r: f32, w: f32) -> f32 {
    ((p - c).length() - r).abs() - w * 0.5
}

/// Signed distance to a stroke from `a` to `b`, `w` wide.
fn segment(p: Vec2, a: Vec2, b: Vec2, w: f32) -> f32 {
    let t = ((p - a).dot(b - a) / (b - a).length_squared()).clamp(0.0, 1.0);
    (p - (a + (b - a) * t)).length() - w * 0.5
}

/// Signed distance to the outline of an axis-aligned box of half size `h`
/// around `c`, stroked `w` wide.
fn box_outline(p: Vec2, c: Vec2, h: Vec2, w: f32) -> f32 {
    let q = (p - c).abs() - h;
    let outside = q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0);
    outside.abs() - w * 0.5
}

/// Signed distance to a chevron pointing right, its tip at `tip`, `len`
/// long on each arm, stroked `w` wide.
fn chevron(p: Vec2, tip: Vec2, len: f32, w: f32) -> f32 {
    segment(p, tip + Vec2::new(-len, -len), tip, w).min(segment(
        p,
        tip + Vec2::new(-len, len),
        tip,
        w,
    ))
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

impl SectionIconType {
    pub(crate) const ALL: [Self; 7] = [
        Self::Weapon,
        Self::Thruster,
        Self::Controller,
        Self::Hull,
        Self::Docking,
        Self::Mining,
        Self::CargoIntake,
    ];

    /// The family a section kind belongs to.
    pub(crate) fn of(kind: SectionClass) -> Self {
        match kind {
            SectionClass::Turret | SectionClass::Torpedo | SectionClass::Railgun => Self::Weapon,
            SectionClass::Thruster => Self::Thruster,
            SectionClass::Controller => Self::Controller,
            SectionClass::Hull => Self::Hull,
            SectionClass::Docking => Self::Docking,
            SectionClass::Mining => Self::Mining,
            SectionClass::CargoIntake => Self::CargoIntake,
        }
    }

    /// The legend word.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Weapon => "Weapon",
            Self::Thruster => "Thruster",
            Self::Controller => "Controller",
            Self::Hull => "Hull",
            Self::Docking => "Docking",
            Self::Mining => "Mining",
            Self::CargoIntake => "Cargo Intake",
        }
    }

    /// The tint of the family's icon and 3D block. Hull keeps the ink colour,
    /// so the bulk of a ship reads as the hull and the rest stands out.
    pub(crate) fn color(self) -> UiColor {
        match self {
            Self::Weapon => UiColor::Danger,
            Self::Thruster => UiColor::Info,
            Self::Controller => UiColor::AccentHigh,
            Self::Hull => UiColor::Primary,
            Self::Docking => UiColor::Nominal,
            Self::Mining => UiColor::Accent,
            Self::CargoIntake => UiColor::AccentHigh,
        }
    }

    /// Alpha of the 3D block fill. Hull stays faint so the tinted parts inside
    /// the outline read first.
    pub(crate) fn block_alpha(self) -> f32 {
        match self {
            Self::Hull => 0.22,
            _ => 0.5,
        }
    }

    /// Each family has its own silhouette: a reticle for a weapon, a nozzle
    /// and plume for a thruster, a diamond core for the controller, a riveted
    /// plate for hull, a clamped collar for docking, a beam for mining, and an
    /// open cargo door for the intake.
    fn coverage(self, p: Vec2, px: f32) -> f32 {
        match self {
            Self::Weapon => {
                let ticks = [Vec2::X, Vec2::NEG_X, Vec2::Y, Vec2::NEG_Y]
                    .into_iter()
                    .map(|axis| segment(p, axis * 0.3, axis * 0.95, 0.14))
                    .fold(f32::INFINITY, f32::min);
                let reticle =
                    ring(p, Vec2::ZERO, 0.62, 0.14)
                        .min(ticks)
                        .min(circle(p, Vec2::ZERO, 0.11));
                solid(reticle, px)
            }
            Self::Thruster => {
                let bell = polygon(
                    p,
                    &[
                        Vec2::new(-0.22, -0.85),
                        Vec2::new(0.22, -0.85),
                        Vec2::new(0.62, 0.15),
                        Vec2::new(-0.62, 0.15),
                    ],
                );
                let plume = polygon(
                    p,
                    &[
                        Vec2::new(-0.42, 0.3),
                        Vec2::new(0.42, 0.3),
                        Vec2::new(0.0, 0.95),
                    ],
                );
                solid(bell, px).max(solid(plume, px) * 0.6)
            }
            Self::Controller => {
                let diamond = (p.x.abs() + p.y.abs() - 0.85) * std::f32::consts::FRAC_1_SQRT_2;
                solid(diamond.abs() - 0.07, px).max(solid(circle(p, Vec2::ZERO, 0.27), px))
            }
            Self::Hull => {
                let hexagon: Vec<Vec2> = (0..6)
                    .map(|corner| {
                        let angle = std::f32::consts::FRAC_PI_3 * corner as f32;
                        Vec2::new(angle.cos(), angle.sin()) * 0.88
                    })
                    .collect();
                let plate = polygon(p, &hexagon);
                let rivets = circle(p, Vec2::new(-0.38, 0.0), 0.11).min(circle(
                    p,
                    Vec2::new(0.38, 0.0),
                    0.11,
                ));
                (solid(plate, px) * 0.35)
                    .max(solid(plate.abs() - 0.07, px))
                    .max(solid(rivets, px))
            }
            Self::Docking => {
                let clamps = [45.0f32, 135.0, 225.0, 315.0]
                    .into_iter()
                    .map(|degrees| {
                        let axis = Vec2::from_angle(degrees.to_radians());
                        segment(p, axis * 0.66, axis * 0.95, 0.24)
                    })
                    .fold(f32::INFINITY, f32::min);
                solid(ring(p, Vec2::ZERO, 0.5, 0.16).min(clamps), px)
                    .max(solid(circle(p, Vec2::ZERO, 0.16), px) * 0.6)
            }
            Self::Mining => {
                let casing = polygon(
                    p,
                    &[
                        Vec2::new(-0.5, 0.8),
                        Vec2::new(0.5, 0.8),
                        Vec2::new(0.5, 0.35),
                        Vec2::new(-0.5, 0.35),
                    ],
                );
                let emitter = segment(p, Vec2::new(0.0, 0.28), Vec2::new(0.0, -0.25), 0.2);
                let beam = segment(p, Vec2::new(0.0, -0.32), Vec2::new(0.0, -0.9), 0.1);
                solid(casing, px)
                    .max(solid(emitter, px))
                    .max(solid(beam, px))
            }
            Self::CargoIntake => {
                let door = polygon(
                    p,
                    &[
                        Vec2::new(-0.88, -0.78),
                        Vec2::new(0.88, -0.78),
                        Vec2::new(0.7, 0.78),
                        Vec2::new(-0.7, 0.78),
                    ],
                );
                let mouth = polygon(
                    p,
                    &[
                        Vec2::new(-0.58, -0.43),
                        Vec2::new(0.58, -0.43),
                        Vec2::new(0.46, 0.48),
                        Vec2::new(-0.46, 0.48),
                    ],
                );
                let sill = segment(p, Vec2::new(-0.7, 0.76), Vec2::new(0.7, 0.76), 0.13);
                solid(door.max(-mouth), px).max(solid(sill, px))
            }
        }
    }
}

impl BodyIconType {
    pub(crate) const ALL: [Self; 4] = [Self::Ship, Self::Asteroid, Self::Planet, Self::Objective];

    /// A chevron for a ship, a rough lump for an asteroid, a ringed disc for
    /// a planet and a diamond for a nav point.
    fn coverage(self, p: Vec2, px: f32) -> f32 {
        match self {
            Self::Ship => {
                let starboard = polygon(
                    p,
                    &[
                        Vec2::new(0.0, -0.9),
                        Vec2::new(0.7, 0.75),
                        Vec2::new(0.0, 0.4),
                    ],
                );
                let port = polygon(
                    p,
                    &[
                        Vec2::new(0.0, -0.9),
                        Vec2::new(0.0, 0.4),
                        Vec2::new(-0.7, 0.75),
                    ],
                );
                solid(starboard.min(port), px)
            }
            Self::Asteroid => {
                let lump = polygon(
                    p,
                    &[
                        Vec2::new(-0.2, -0.8),
                        Vec2::new(0.45, -0.7),
                        Vec2::new(0.82, -0.12),
                        Vec2::new(0.62, 0.6),
                        Vec2::new(-0.05, 0.82),
                        Vec2::new(-0.68, 0.45),
                        Vec2::new(-0.78, -0.3),
                    ],
                );
                let crater = circle(p, Vec2::new(0.2, 0.1), 0.18);
                solid(lump, px).min(1.0 - solid(crater, px) * 0.6)
            }
            Self::Planet => {
                let disc = circle(p, Vec2::ZERO, 0.5);
                let band = ring(p, Vec2::ZERO, 0.8, 0.1).max(p.y.abs() - 0.18);
                solid(disc, px).max(solid(band, px))
            }
            Self::Objective => {
                let diamond = (p.x.abs() + p.y.abs() - 0.8) * std::f32::consts::FRAC_1_SQRT_2;
                solid(diamond.abs() - 0.09, px).max(solid(circle(p, Vec2::ZERO, 0.16), px))
            }
        }
    }
}

impl ChartIconType {
    pub(crate) const ALL: [Self; 9] = [
        Self::Asteroids,
        Self::Rocks,
        Self::Planet,
        Self::Derelicts,
        Self::Sector,
        Self::Station,
        Self::Wreck,
        Self::Job,
        Self::You,
    ];

    /// What the chart's legend lists, in its order: the contents, the marks
    /// and the own ship. A bare cell needs no legend entry.
    pub(crate) const LEGEND: [Self; 8] = [
        Self::Asteroids,
        Self::Rocks,
        Self::Planet,
        Self::Derelicts,
        Self::Station,
        Self::Wreck,
        Self::Job,
        Self::You,
    ];

    /// The icon of a cluster type: what the player finds there.
    pub(crate) const fn of(cluster: ClusterType) -> Self {
        match cluster {
            ClusterType::AsteroidRich => Self::Asteroids,
            ClusterType::RockOnly => Self::Rocks,
            ClusterType::PlanetHeavy => Self::Planet,
            ClusterType::DerelictField => Self::Derelicts,
        }
    }

    /// The legend word.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Asteroids => "Asteroids",
            Self::Rocks => "Rocks",
            Self::Planet => "Planet",
            Self::Derelicts => "Derelicts",
            Self::Sector => "Sector",
            Self::Station => "Station",
            Self::Wreck => "Wreck",
            Self::Job => "Job",
            Self::You => "You",
        }
    }

    /// The tint of the icon, and of the tile layer or the mark it stands
    /// for: a living station's colour for the station icon, the Map's own
    /// ship colour for you.
    pub(crate) const fn color(self) -> UiColor {
        match self {
            Self::Asteroids => UiColor::Accent,
            Self::Rocks => UiColor::Secondary,
            Self::Planet => UiColor::Info,
            Self::Derelicts => UiColor::Body,
            Self::Sector => UiColor::Label,
            Self::Station => UiColor::Nominal,
            Self::Wreck => UiColor::AccentHigh,
            Self::Job => UiColor::Accent,
            Self::You => UiColor::Primary,
        }
    }

    /// Three lumps for asteroids, a scatter of small rocks, the map's ringed
    /// planet, two hull halves for derelicts, a framed dot for a bare cell,
    /// a cube in projection for a station, a cracked outline for a wreck, the
    /// map's nav diamond for a job, and the own-ship dot in a faint ring.
    fn coverage(self, p: Vec2, px: f32) -> f32 {
        match self {
            Self::Asteroids => {
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
            Self::Rocks => [
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
            Self::Planet => BodyIconType::Planet.coverage(p, px),
            Self::Derelicts => {
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
            Self::Sector => solid(box_outline(p, Vec2::ZERO, Vec2::splat(0.72), 0.12), px)
                .max(solid(circle(p, Vec2::ZERO, 0.14), px)),
            Self::Station => {
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
            Self::Wreck => {
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
            Self::Job => BodyIconType::Objective.coverage(p, px),
            Self::You => solid(circle(p, Vec2::ZERO, 0.32), px)
                .max(solid(ring(p, Vec2::ZERO, 0.7, 0.08), px) * 0.6),
        }
    }
}

impl JobIconType {
    pub(crate) const ALL: [Self; 5] = [
        Self::Bring,
        Self::Recover,
        Self::Escort,
        Self::Answer,
        Self::Neutralize,
    ];

    /// The icon of a job kind.
    pub(crate) const fn of(kind: &JobType) -> Self {
        match kind {
            JobType::Procure { .. } => Self::Bring,
            JobType::Recover { .. } => Self::Recover,
            JobType::Escort { .. } => Self::Escort,
            JobType::Distress { .. } => Self::Answer,
            JobType::Neutralize { .. } => Self::Neutralize,
        }
    }

    /// The tint of every job icon: the accent, as the chart's job mark.
    pub(crate) const fn color(self) -> UiColor {
        UiColor::Accent
    }

    /// A strapped crate for a procurement, a hook over a cracked hull for a
    /// recovery, two chevrons for an escort, a ringed exclamation mark for a
    /// distress call and a reticle for a neutralize.
    fn coverage(self, p: Vec2, px: f32) -> f32 {
        match self {
            Self::Bring => {
                let shell = box_outline(p, Vec2::new(0.0, 0.1), Vec2::new(0.72, 0.62), 0.14);
                let lid = segment(p, Vec2::new(-0.72, -0.22), Vec2::new(0.72, -0.22), 0.1);
                let strap = segment(p, Vec2::new(0.0, -0.22), Vec2::new(0.0, 0.72), 0.1);
                solid(shell.min(lid).min(strap), px)
            }
            Self::Recover => {
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
            Self::Escort => {
                let lead = chevron(p, Vec2::new(0.1, 0.0), 0.55, 0.16);
                let wing = chevron(p, Vec2::new(0.75, 0.0), 0.55, 0.16);
                solid(lead.min(wing), px)
            }
            Self::Answer => {
                let beacon = ring(p, Vec2::ZERO, 0.78, 0.13);
                let bar = segment(p, Vec2::new(0.0, -0.45), Vec2::new(0.0, 0.1), 0.17);
                let dot = circle(p, Vec2::new(0.0, 0.42), 0.11);
                solid(beacon.min(bar).min(dot), px)
            }
            Self::Neutralize => {
                let ticks = [Vec2::X, Vec2::NEG_X, Vec2::Y, Vec2::NEG_Y]
                    .into_iter()
                    .map(|axis| segment(p, axis * 0.35, axis * 0.95, 0.13))
                    .fold(f32::INFINITY, f32::min);
                let reticle =
                    ring(p, Vec2::ZERO, 0.6, 0.13)
                        .min(ticks)
                        .min(circle(p, Vec2::ZERO, 0.1));
                solid(reticle, px)
            }
        }
    }
}

/// Every item category, in declaration order, which is the order its mask is
/// stored in: [`InterfaceIcons::category`] indexes by `category as usize`.
pub(crate) const ITEM_CATEGORIES: [ItemCategoryType; 5] = [
    ItemCategoryType::Raw,
    ItemCategoryType::Repair,
    ItemCategoryType::Ammo,
    ItemCategoryType::Food,
    ItemCategoryType::Parts,
];

// A category listed out of declaration order would draw another category's
// mask, so the build fails instead.
const _: () = {
    let mut index = 0;
    while index < ITEM_CATEGORIES.len() {
        assert!(
            ITEM_CATEGORIES[index] as usize == index,
            "ITEM_CATEGORIES must list ItemCategoryType in declaration order"
        );
        index += 1;
    }
};

/// A pile of lumps for raw goods, a wrench for repair stock, three rounds for
/// ammo, a tin for food and a gear for parts.
fn category_coverage(category: ItemCategoryType, p: Vec2, px: f32) -> f32 {
    match category {
        ItemCategoryType::Raw => {
            let lumps = circle(p, Vec2::new(-0.4, 0.4), 0.36)
                .min(circle(p, Vec2::new(0.4, 0.42), 0.34))
                .min(circle(p, Vec2::new(0.0, -0.2), 0.4));
            solid(lumps, px)
        }
        ItemCategoryType::Repair => {
            let head = ring(p, Vec2::new(-0.35, -0.35), 0.3, 0.2);
            // The jaw: the head's ring opens toward the top left.
            let jaw = circle(p, Vec2::new(-0.62, -0.62), 0.22);
            let handle = segment(p, Vec2::new(-0.15, -0.15), Vec2::new(0.7, 0.7), 0.24);
            solid(head.max(-jaw).min(handle), px)
        }
        ItemCategoryType::Ammo => [-0.5f32, 0.0, 0.5]
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
        ItemCategoryType::Food => {
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
        ItemCategoryType::Parts => {
            let teeth = (0..8)
                .map(|tooth| {
                    let axis = Vec2::from_angle(std::f32::consts::FRAC_PI_4 * tooth as f32);
                    segment(p, axis * 0.5, axis * 0.88, 0.22)
                })
                .fold(f32::INFINITY, f32::min);
            let wheel = ring(p, Vec2::ZERO, 0.42, 0.26);
            solid(wheel.min(teeth), px)
        }
    }
}

/// Icon mask edge, in texels.
const ICON_TEXELS: u32 = 64;

/// Draw one mask from its coverage.
fn icon_mask(images: &mut Assets<Image>, coverage: impl Fn(Vec2, f32) -> f32) -> Handle<Image> {
    let texel = 2.0 / ICON_TEXELS as f32;
    let mut data = Vec::with_capacity((ICON_TEXELS * ICON_TEXELS * 4) as usize);
    for y in 0..ICON_TEXELS {
        for x in 0..ICON_TEXELS {
            let at = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * texel - Vec2::ONE;
            let alpha = coverage(at, texel);
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

/// Draw every icon mask once.
pub(crate) fn init_interface_icons(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(InterfaceIcons {
        sections: SectionIconType::ALL
            .map(|icon| icon_mask(&mut images, |p, px| icon.coverage(p, px))),
        bodies: BodyIconType::ALL.map(|body| icon_mask(&mut images, |p, px| body.coverage(p, px))),
        categories: ITEM_CATEGORIES
            .map(|category| icon_mask(&mut images, |p, px| category_coverage(category, p, px))),
        charts: ChartIconType::ALL.map(|icon| icon_mask(&mut images, |p, px| icon.coverage(p, px))),
        jobs: JobIconType::ALL.map(|icon| icon_mask(&mut images, |p, px| icon.coverage(p, px))),
    });
}

/// An icon mask tinted by the theme, `size` px square.
pub(crate) fn icon_node(image: Handle<Image>, tint: UiColor, size: f32) -> impl Bundle {
    (
        ImageNode::new(image),
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
