//! The handbook's demonstrations, end to end over the REAL authored lessons.
//!
//! A sprite sheet is just pixels: nothing in the file says where its cells are,
//! so the game cuts one by the grid the LESSON authors. That makes the
//! generator that draws the placeholder art and the builder that authors the
//! lesson two halves of one contract, and nothing else in the pipeline would
//! notice them drifting apart - a still drawn where a loop is authored would
//! ship as a twelfth of one frame, silently, on the one screen a new player is
//! sent to first.
//!
//! So this test reads `scripts/gen-lesson-media.py`'s own table and holds it
//! against the catalog: same lessons, same titles, same form, and a file on
//! disk for every one of them.

use std::{collections::BTreeMap, path::PathBuf};

use nova_authoring::generation;
use nova_training::prelude::LessonMedia;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .expect("the workspace root is two levels above this crate")
        .to_path_buf()
}

/// The generator's `LESSONS` table as (id, title, is_loop).
///
/// Parsed from the script's source rather than run: the test asserts on what
/// the table SAYS, and running python from a Rust test would make a cargo test
/// depend on an interpreter.
fn generator_table() -> Vec<(String, String, bool)> {
    let script = std::fs::read_to_string(repo_root().join("scripts/gen-lesson-media.py"))
        .expect("scripts/gen-lesson-media.py");
    let (_, rest) = script
        .split_once("\nLESSONS = [")
        .expect("the generator declares a LESSONS table");
    let (table, _) = rest.split_once("\n]").expect("the table is closed");

    table
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|line| {
            let fields: Vec<String> = line
                .trim_start_matches('(')
                .trim_end_matches(',')
                .trim_end_matches(')')
                .split("\", \"")
                .map(|field| field.trim().trim_matches('"').to_string())
                .collect();
            assert_eq!(fields.len(), 3, "malformed generator row: {line}");
            let kind = fields[2].as_str();
            assert!(
                kind == "still" || kind == "loop",
                "a generator row is either a still or a loop, not {kind:?}"
            );
            (fields[0].clone(), fields[1].clone(), kind == "loop")
        })
        .collect()
}

/// Every authored lesson is drawn by the generator, in the same form, under
/// the same title - and no row of the generator's table is art for a lesson
/// that no longer exists.
#[test]
fn the_media_generator_draws_exactly_the_authored_lessons() {
    let authored: Vec<(String, String, bool)> = generation::build_lessons()
        .into_iter()
        .map(|lesson| (lesson.id, lesson.title, lesson.media.is_loop()))
        .collect();
    let generated = generator_table();

    let authored_ids: Vec<&str> = authored.iter().map(|(id, ..)| id.as_str()).collect();
    let generated_ids: Vec<&str> = generated.iter().map(|(id, ..)| id.as_str()).collect();
    assert_eq!(
        authored_ids, generated_ids,
        "scripts/gen-lesson-media.py draws a different set of lessons than the catalog authors"
    );

    let generated: BTreeMap<&str, (&str, bool)> = generated
        .iter()
        .map(|(id, title, is_loop)| (id.as_str(), (title.as_str(), *is_loop)))
        .collect();
    for (id, title, is_loop) in &authored {
        let (drawn_title, drawn_loop) = generated[id.as_str()];
        assert_eq!(
            drawn_title, title,
            "the placeholder art for '{id}' is titled differently than the lesson"
        );
        assert_eq!(
            drawn_loop,
            *is_loop,
            "'{id}' is authored as a {} and drawn as a {}; the game cuts the sheet by the \
             AUTHORED grid, so a still drawn here would be shown as one cell of it",
            if *is_loop { "loop" } else { "still" },
            if drawn_loop { "loop" } else { "still" }
        );
    }
}

/// Media is REQUIRED so the details layout never reflows between lessons: the
/// path has to be the base bundle's own, declared in its manifest, and there
/// has to be a file at it.
#[test]
fn every_lesson_ships_a_declared_demonstration_that_exists() {
    let root = repo_root();
    let manifest = std::fs::read_to_string(root.join("assets/base/base.bundle.ron"))
        .expect("assets/base/base.bundle.ron");

    for lesson in generation::build_lessons() {
        let media = match &lesson.media {
            LessonMedia::Image { image, .. } => image,
            LessonMedia::Loop { sheet, .. } => sheet,
        };
        let path = media
            .path()
            .expect("an authored lesson carries a media PATH, not a handle");
        let rel = path
            .strip_prefix("self://")
            .unwrap_or_else(|| panic!("'{}' authors '{path}', not a `self://` ref", lesson.id));
        assert!(
            manifest.contains(&format!("\"{rel}\"")),
            "'{}' names '{rel}', which base.bundle.ron does not declare as a resource",
            lesson.id
        );
        assert!(
            root.join("assets/base").join(rel).is_file(),
            "'{}' names '{rel}', and there is no file there - run scripts/gen-lesson-media.py",
            lesson.id
        );
    }
}

/// A demonstration has to DECODE, and a sheet has to divide by the grid the
/// lesson cuts it on.
///
/// The screen cuts a sheet by the AUTHORED `columns`/`rows` and nothing in the
/// file argues back: a sheet whose width is not a multiple of its columns is
/// drawn as twelve frames sliced through their own edges, on the one screen a
/// new player is sent to first. That is a data error, not a code error, so it
/// is caught here rather than at runtime.
///
/// Decoding is the second half. A demonstration is WebP (see `media_path` in
/// `base_content/lessons.rs`), and `image` is the crate `bevy_image` decodes
/// with - a pure-Rust decoder with no platform half - so a file this test
/// reads is a file the game reads, native or wasm.
#[test]
fn every_demonstration_decodes_and_a_loop_divides_by_its_grid() {
    let root = repo_root();

    for lesson in generation::build_lessons() {
        let media = match &lesson.media {
            LessonMedia::Image { image, .. } => image,
            LessonMedia::Loop { sheet, .. } => sheet,
        };
        let rel = media
            .path()
            .and_then(|path| path.strip_prefix("self://"))
            .expect("an authored lesson carries a `self://` media path")
            .to_string();
        let file = root.join("assets/base").join(&rel);
        let decoded = image::open(&file).unwrap_or_else(|why| {
            panic!(
                "'{}' names '{rel}', which will not decode: {why}",
                lesson.id
            )
        });
        let (width, height) = (decoded.width(), decoded.height());

        if let LessonMedia::Loop { columns, rows, .. } = &lesson.media {
            assert!(
                width % columns == 0 && height % rows == 0,
                "'{}' authors a {columns}x{rows} grid over a {width}x{height} sheet, which does \
                 not divide: the cells would be cut through their own edges",
                lesson.id
            );
        }
    }
}
