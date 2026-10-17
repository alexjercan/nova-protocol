//! The chart's tile textures: one coverage mask per cluster type, the faint
//! grid of a visited cell with no cluster body and the outline of a cell the
//! player never entered.
//!
//! Each mask is white with its coverage as alpha, drawn once at startup, and
//! the chart tints it with the cluster type's colour when it builds, so a
//! theme change redraws no texture. A cell with several cluster types stacks
//! one tinted layer per type.
//!
//! Touch this module when changing what a cell's contents look like on the
//! chart.

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use nova_gameplay::prelude::ClusterType;

/// Every tile texture the chart draws.
#[derive(Resource)]
pub(crate) struct ChartTileTextures {
    /// The faint grid of a visited cell with no cluster body.
    pub(crate) grid: Handle<Image>,
    /// The outline of a cell the player never entered, and of the selected
    /// cell in the accent.
    pub(crate) outline: Handle<Image>,
    /// One texture per cluster type, in [`ClusterType::ALL`] order.
    clusters: [Handle<Image>; ClusterType::ALL.len()],
}

impl ChartTileTextures {
    /// The texture of a cluster type's layer.
    pub(crate) fn cluster(&self, cluster: ClusterType) -> Handle<Image> {
        let index = ClusterType::ALL
            .iter()
            .position(|each| *each == cluster)
            .expect("ClusterType::ALL lists every type");
        self.clusters[index].clone()
    }
}

/// Texture edge, in texels.
const TILE_TEXELS: u32 = 128;

/// Coverage of a shape whose signed distance at a texel is `d`.
fn soft(d: f32, px: f32) -> f32 {
    (0.5 - d / px).clamp(0.0, 1.0)
}

fn disc(p: Vec2, c: Vec2, r: f32) -> f32 {
    (p - c).length() - r
}

/// A fixed hash, so every texture draws the same scatter on every start.
fn hash(seed: u64, salt: u64) -> u64 {
    let mut x = seed ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// A seeded point in `[-0.9, 0.9]` squared.
fn scatter(seed: u64, i: u64) -> Vec2 {
    let h = hash(seed, i);
    let x = (h & 0xFFFF) as f32 / 65_535.0;
    let y = ((h >> 16) & 0xFFFF) as f32 / 65_535.0;
    Vec2::new(x * 1.8 - 0.9, y * 1.8 - 0.9)
}

/// A seeded size in `[min, max]`.
fn scatter_size(seed: u64, i: u64, min: f32, max: f32) -> f32 {
    let h = hash(seed, i ^ 0xABCD);
    min + ((h >> 32) & 0xFFFF) as f32 / 65_535.0 * (max - min)
}

/// A faint grid, four lines each way, and a border.
fn grid_coverage(p: Vec2) -> f32 {
    let cellf = |v: f32| ((v + 1.0) * 2.0).fract();
    let line = |f: f32| (f.min(1.0 - f) * 0.5 - 0.01).min(0.0).abs() / 0.01;
    let grid = line(cellf(p.x)).max(line(cellf(p.y))).clamp(0.0, 1.0);
    let edge = (p.x.abs().max(p.y.abs()) - 0.97).max(0.0) / 0.03;
    (grid * 0.45).max(edge.min(1.0) * 0.7)
}

/// An outline only.
fn outline_coverage(p: Vec2, px: f32) -> f32 {
    let edge = p.x.abs().max(p.y.abs());
    soft((edge - 0.97).abs() - 0.025, px)
}

/// A speckle of cratered lumps for an asteroid-rich cell, a sparse scatter of
/// rocks for a rock-only one, a shaded disc with a ring for a planet-heavy
/// one, and a few cracked hull slivers for a derelict field.
fn cluster_coverage(cluster: ClusterType, p: Vec2, px: f32) -> f32 {
    match cluster {
        ClusterType::AsteroidRich => (0..70u64)
            .map(|i| {
                let c = scatter(11, i);
                let r = scatter_size(11, i, 0.02, 0.065);
                let crater = disc(p, c + Vec2::splat(r * 0.3), r * 0.4);
                soft(disc(p, c, r), px) * (0.55 + 0.45 * soft(crater, px).max(0.0)).min(1.0)
            })
            .fold(0.0, f32::max),
        ClusterType::RockOnly => (0..16u64)
            .map(|i| {
                let c = scatter(23, i);
                let r = scatter_size(23, i, 0.015, 0.035);
                soft(disc(p, c, r), px)
            })
            .fold(0.0, f32::max),
        ClusterType::PlanetHeavy => {
            let c = Vec2::new(0.05, -0.05);
            let body = soft(disc(p, c, 0.34), px);
            let lit = ((p.x - c.x + p.y - c.y) * -1.6).clamp(0.0, 1.0);
            let shade = 0.3 + 0.7 * lit;
            let q = p - c;
            let ellipse = (q.x * q.x / 0.42 + q.y * q.y / 0.06).sqrt();
            let ring = soft((ellipse - 1.0).abs() - 0.07, px) * 0.75;
            // The ring passes behind the disc's upper half.
            let behind = q.y < 0.0 && q.length() < 0.34;
            if behind {
                body * shade
            } else {
                (body * shade).max(ring)
            }
        }
        ClusterType::DerelictField => (0..5u64)
            .map(|i| {
                let c = scatter(37, i) * 0.8;
                let angle = scatter_size(37, i, 0.0, std::f32::consts::PI);
                let dir = Vec2::from_angle(angle);
                let len = scatter_size(37, i + 50, 0.12, 0.22);
                let q = p - c;
                let along = q.dot(dir);
                let across = q.perp_dot(dir).abs();
                let sliver = (along.abs() - len).max(across - 0.035);
                let crack = (along - len * 0.2).abs() - 0.012;
                soft(sliver.max(-crack), px)
            })
            .fold(0.0, f32::max),
    }
}

/// Draw one texture from its coverage over `[-1, 1]` squared.
fn tile_image(images: &mut Assets<Image>, coverage: impl Fn(Vec2, f32) -> f32) -> Handle<Image> {
    let texel = 2.0 / TILE_TEXELS as f32;
    let mut data = Vec::with_capacity((TILE_TEXELS * TILE_TEXELS * 4) as usize);
    for y in 0..TILE_TEXELS {
        for x in 0..TILE_TEXELS {
            let at = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * texel - Vec2::ONE;
            let alpha = coverage(at, texel);
            data.extend_from_slice(&[255, 255, 255, (alpha * 255.0).round() as u8]);
        }
    }
    images.add(Image::new(
        Extent3d {
            width: TILE_TEXELS,
            height: TILE_TEXELS,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

/// Draw every tile texture once.
pub(crate) fn init_chart_tiles(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(ChartTileTextures {
        grid: tile_image(&mut images, |p, _| grid_coverage(p)),
        outline: tile_image(&mut images, outline_coverage),
        clusters: ClusterType::ALL
            .map(|cluster| tile_image(&mut images, |p, px| cluster_coverage(cluster, p, px))),
    });
}
