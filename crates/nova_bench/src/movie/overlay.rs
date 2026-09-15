//! The compositor: the rail drawn onto a recorded frame, here, in this
//! process. ffmpeg is the codec and nothing else - layout that lived in a
//! filter graph could not be read, tested, or changed without re-deriving it
//! from a shell command.
//!
//! The panel is the game's own terminal face on a dark plate in the top left,
//! which is where every recorded scenario leaves room: the HUD owns the
//! corners on the right, the ship the middle, the control hints the bottom.
//! Older rows fade instead of vanishing, so a reader can still see the two
//! beats that led to the current one.

use std::path::Path;

use ab_glyph::{point, Font, FontVec, PxScale, ScaleFont};
use image::RgbImage;

use super::rail::{wrap, At, Cue, Film, Lane};

/// The width the layout constants are written for; every size scales from it.
const REFERENCE_WIDTH: f32 = 1280.0;

/// Columns of body text after the `> label` prefix.
const TEXT_COLS: usize = 76;

/// Columns the `> label ` prefix occupies.
const PREFIX_COLS: usize = 8;

/// Rows of body text on the panel.
const BODY_LINES: usize = 6;

/// How much of its colour a row keeps, by age: newest first.
const FADE: [f32; 6] = [1.0, 0.72, 0.56, 0.44, 0.35, 0.28];

/// The plate the rows sit on.
const PLATE: [u8; 3] = [0x08, 0x0A, 0x0C];

/// The header strip's ink, and the tint of its plate.
const HEADER: [u8; 3] = [0x7C, 0xFF, 0x9B];

/// Point sizes at [`REFERENCE_WIDTH`]: the header, then the body.
const HEADER_SIZE: f32 = 14.0;
const BODY_SIZE: f32 = 16.0;

/// The sizes one frame's width implies. Everything the compositor draws is
/// derived here, so a 720p and a 4K recording lay out identically.
#[derive(Debug, Clone, Copy)]
struct Metrics {
    scale: f32,
    margin: f32,
    header_size: f32,
    body_size: f32,
    header_height: f32,
    line_height: f32,
    pad: f32,
    advance: f32,
    width: f32,
}

/// The compositor: the face, and the sizes for the frames it is drawing.
pub struct Overlay {
    font: FontVec,
    metrics: Metrics,
}

impl Overlay {
    /// Load the face and fix the layout for frames `width` pixels across.
    pub fn new(font: &Path, width: u32) -> Result<Self, String> {
        let bytes = std::fs::read(font)
            .map_err(|error| format!("could not read the font {}: {error}", font.display()))?;
        let font = FontVec::try_from_vec(bytes)
            .map_err(|error| format!("{} is not a usable font: {error}", font.display()))?;
        let scale = width as f32 / REFERENCE_WIDTH;
        let body_size = BODY_SIZE * scale;
        let advance = font
            .as_scaled(PxScale::from(body_size))
            .h_advance(font.glyph_id('M'));
        let pad = 9.0 * scale;
        let metrics = Metrics {
            scale,
            margin: 24.0 * scale,
            header_size: HEADER_SIZE * scale,
            body_size,
            header_height: 24.0 * scale,
            line_height: 22.0 * scale,
            pad,
            advance,
            width: pad * 2.0 + advance * (PREFIX_COLS + TEXT_COLS) as f32,
        };
        Ok(Self { font, metrics })
    }

    /// Draw the rail's state at `at` onto that frame.
    pub fn draw(&self, image: &mut RgbImage, film: &Film, at: At) {
        let m = self.metrics;
        let body_height = m.pad * 2.0 + m.line_height * BODY_LINES as f32;
        let top = 20.0 * m.scale;

        // A marked run says so in its own header, in the cheat lane's amber,
        // for every frame from the mark on. No clip of it reads as clean.
        let tint = if film.cheated(at.rail) {
            Lane::Cheat.color()
        } else {
            HEADER
        };
        fill(image, m.margin, top, m.width, m.header_height, tint, 0.14);
        let baseline = top + m.header_height - m.pad * 0.7;
        self.text(
            image,
            m.margin + m.pad,
            baseline,
            m.header_size,
            tint,
            1.0,
            &header_line(film, at),
        );

        let body_top = top + m.header_height;
        fill(image, m.margin, body_top, m.width, body_height, PLATE, 0.66);

        let view = film.view(at.rail, TEXT_COLS, BODY_LINES);
        let newest = view.len().saturating_sub(1);
        let mut y = body_top + m.pad + m.body_size;
        for (index, cue) in view.iter().enumerate() {
            let alpha = FADE[(newest - index).min(FADE.len() - 1)];
            y = self.row(image, y, cue, alpha);
        }
    }

    /// Draw one cue's wrapped lines from baseline `y`; answer the next
    /// baseline.
    fn row(&self, image: &mut RgbImage, mut y: f32, cue: &Cue, alpha: f32) -> f32 {
        let m = self.metrics;
        let color = cue.row.lane.color();
        let left = m.margin + m.pad;
        let text_left = left + m.advance * PREFIX_COLS as f32;
        for (index, line) in wrap(&cue.row.text, TEXT_COLS).into_iter().enumerate() {
            if index == 0 {
                let prompt = format!("> {}", cue.row.lane.label());
                self.text(image, left, y, m.body_size, color, alpha * 0.85, &prompt);
            }
            self.text(image, text_left, y, m.body_size, color, alpha, &line);
            y += m.line_height;
        }
        y
    }

    /// Rasterise one line at a baseline, left aligned.
    fn text(
        &self,
        image: &mut RgbImage,
        x: f32,
        y: f32,
        size: f32,
        color: [u8; 3],
        alpha: f32,
        line: &str,
    ) {
        let scale = PxScale::from(size);
        let scaled = self.font.as_scaled(scale);
        let mut caret = x;
        for character in line.chars() {
            let id = self.font.glyph_id(character);
            let glyph = id.with_scale_and_position(scale, point(caret, y));
            caret += scaled.h_advance(id);
            let Some(outline) = self.font.outline_glyph(glyph) else {
                continue;
            };
            let bounds = outline.px_bounds();
            outline.draw(|dx, dy, coverage| {
                blend(
                    image,
                    bounds.min.x + dx as f32,
                    bounds.min.y + dy as f32,
                    color,
                    coverage * alpha,
                );
            });
        }
    }
}

/// The header strip's line: what ran, where the movie stands, whose turn it
/// is. A clip lifted out of the run still says which run it came from.
fn header_line(film: &Film, at: At) -> String {
    format!(
        "{}   {}   tick {}   turn {}{}",
        film.head.identity(),
        clock(at.clock),
        Film::tick(at.clock),
        film.turn(at.rail),
        if film.cheated(at.rail) {
            "   CHEATED"
        } else {
            ""
        }
    )
}

/// `mm:ss.hh` of a frame, at the recorder's one frame per tick.
fn clock(frame: u64) -> String {
    let hundredths = frame * 100 / u64::from(super::FRAMES_PER_SECOND);
    format!(
        "{:02}:{:02}.{:02}",
        hundredths / 6000,
        hundredths / 100 % 60,
        hundredths % 100
    )
}

/// Fill a rectangle with `color` at `alpha`.
fn fill(image: &mut RgbImage, x: f32, y: f32, width: f32, height: f32, color: [u8; 3], alpha: f32) {
    let left = x.max(0.0) as u32;
    let top = y.max(0.0) as u32;
    let right = ((x + width).max(0.0) as u32).min(image.width());
    let bottom = ((y + height).max(0.0) as u32).min(image.height());
    for py in top..bottom {
        for px in left..right {
            blend(image, px as f32, py as f32, color, alpha);
        }
    }
}

/// Mix `color` into one pixel. Out-of-frame coordinates are dropped, so a
/// panel wider than a small recording clips instead of panicking.
fn blend(image: &mut RgbImage, x: f32, y: f32, color: [u8; 3], alpha: f32) {
    if alpha <= 0.0 || x < 0.0 || y < 0.0 {
        return;
    }
    let (x, y) = (x as u32, y as u32);
    if x >= image.width() || y >= image.height() {
        return;
    }
    let alpha = alpha.min(1.0);
    let pixel = image.get_pixel_mut(x, y);
    for channel in 0..3 {
        let under = f32::from(pixel.0[channel]);
        let over = f32::from(color[channel]);
        pixel.0[channel] = under.mul_add(1.0 - alpha, over * alpha).round() as u8;
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::audit::BenchEvent;

    fn film() -> Film {
        super::super::rail::film(
            &[
                BenchEvent::RunStart {
                    scenario: "slingshot.content.ron".into(),
                    agent: "pi-gpt-5.6-sol-medium".into(),
                    goal: "Reach EXIT".into(),
                    seed: Some(7),
                    budget: json!({}),
                    run_dir: "/runs/1".into(),
                },
                BenchEvent::AgentRequest {
                    request: json!({ "act": { "gestures": [{ "press": "flight.main_drive" }], "ticks": 9 } }),
                },
            ],
            2000,
        )
    }

    #[test]
    fn the_header_names_the_run_the_clock_and_the_turn() {
        let film = film();
        assert_eq!(clock(0), "00:00.00");
        assert_eq!(clock(1500), "00:25.00");
        assert_eq!(clock(4932), "01:22.20");
        assert_eq!(
            header_line(&film, At::live(1500)),
            "slingshot   pi-gpt-5.6-sol-medium   seed 7   00:25.00   tick 1501   turn 1"
        );
        let held = At {
            rail: 4000,
            clock: 1500,
        };
        assert!(
            header_line(&film, held).contains("00:25.00   tick 1501"),
            "a held tail keeps the clock at the last frame the game drew"
        );

        let marked = Film {
            cheated_from: Some(1400),
            ..film
        };
        assert!(!header_line(&marked, At::live(1399)).contains("CHEATED"));
        assert!(header_line(&marked, At::live(1400)).ends_with("   CHEATED"));
    }

    #[test]
    fn a_panel_wider_than_the_frame_clips_instead_of_panicking() {
        let mut image = RgbImage::new(64, 32);
        fill(&mut image, 40.0, 20.0, 400.0, 400.0, [255, 255, 255], 1.0);
        assert_eq!(image.get_pixel(50, 25).0, [255, 255, 255]);
        blend(&mut image, 200.0, 5.0, [255, 0, 0], 1.0);
        blend(&mut image, -1.0, 5.0, [255, 0, 0], 1.0);
        assert_eq!(image.get_pixel(0, 5).0, [0, 0, 0]);
    }

    #[test]
    fn a_half_alpha_pixel_sits_halfway_between_the_frame_and_the_ink() {
        let mut image = RgbImage::new(4, 4);
        blend(&mut image, 1.0, 1.0, [200, 100, 0], 0.5);
        assert_eq!(image.get_pixel(1, 1).0, [100, 50, 0]);
    }
}
