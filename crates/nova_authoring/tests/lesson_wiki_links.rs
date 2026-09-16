//! Every lesson's `wiki_path` against the manual the site actually ships.
//!
//! The handbook is the CONDENSED companion: each lesson ends in a line that
//! sends the reader to the page with the whole story. Nothing in the game can
//! check that line - the manual is a website, the lesson is content, and a
//! `wiki_path` is just a string - so a page renamed on the site would leave
//! the handbook pointing at nothing, on the one screen a new player is sent to
//! first. The first authored set did exactly that: seventeen of the
//! twenty-four lessons named pages (`wiki/flight`, `wiki/combat`,
//! `wiki/shipbuilding`) that the site has never served.
//!
//! So this test resolves every authored path against `web/src/`: the page has
//! to be a file, and an anchor has to be a heading on that page, slugged the
//! way the site slugs its headings.

use std::path::PathBuf;

use nova_authoring::generation;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .expect("the workspace root is two levels above this crate")
        .to_path_buf()
}

/// A heading's anchor, the way the site builds one: lowercased, with
/// everything that is not a letter, a digit or a space dropped, and the spaces
/// joined by hyphens. `## RCS: fine docking thrusters` is
/// `rcs-fine-docking-thrusters`.
fn slug(heading: &str) -> String {
    let cleaned: String = heading
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_whitespace() || *c == '-')
        .collect();
    cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .to_lowercase()
}

/// The `#`-prefixed headings of one Markdown page, already slugged. Fenced
/// code is skipped so a `# comment` inside a block is not read as a heading.
fn anchors(markdown: &str) -> Vec<String> {
    let mut fenced = false;
    let mut anchors = Vec::new();
    for line in markdown.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        if let Some(heading) = line.strip_prefix('#') {
            let text = heading.trim_start_matches('#').trim();
            if !text.is_empty() {
                anchors.push(slug(text));
            }
        }
    }
    anchors
}

/// Every authored lesson links into a page the manual ships, at a heading that
/// page actually has.
#[test]
fn every_lesson_links_into_a_page_the_manual_ships() {
    let src = repo_root().join("web/src");

    for lesson in generation::build_lessons() {
        let (page, anchor) = match lesson.wiki_path.split_once('#') {
            Some((page, anchor)) => (page, Some(anchor)),
            None => (lesson.wiki_path.as_str(), None),
        };

        let markdown = src.join(format!("{page}.md"));
        assert!(
            markdown.is_file(),
            "'{}' links to '{}', and the manual has no {}.md - the site serves what is in \
             web/src, so this line dead-ends the reader",
            lesson.id,
            lesson.wiki_path,
            page,
        );

        let Some(anchor) = anchor else {
            continue;
        };
        let found = anchors(&std::fs::read_to_string(&markdown).expect("a readable page"));
        assert!(
            found.iter().any(|heading| heading == anchor),
            "'{}' links to '{}', and {}.md has no heading that anchors '{anchor}'. Its \
             headings anchor: {}",
            lesson.id,
            lesson.wiki_path,
            page,
            found.join(", "),
        );
    }
}
