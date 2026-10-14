//! The one double-click detector every screen shares.

use bevy::prelude::*;

/// How long after a click a second one on the same thing still reads as a
/// double, in [`Time<Real>`] seconds.
///
/// The window desktops have used for decades. A screen that acts on a double
/// click times both clicks on the real clock, so a paused or scaled game clock
/// never stretches the window.
pub const DOUBLE_CLICK_SECS: f32 = 0.5;

/// The thing the last click landed on, and when.
///
/// `K` is whatever names the clicked thing for its screen: an entity, or a row
/// key when the row's entity can be rebuilt between the two clicks. A click on
/// a DIFFERENT thing restarts the count: a fast click on one row and then
/// another is two selections, not a double.
#[derive(Resource, Clone, PartialEq, Debug)]
pub struct DoubleClick<K: Send + Sync + 'static> {
    last: Option<(K, f32)>,
}

impl<K: Send + Sync + 'static> Default for DoubleClick<K> {
    fn default() -> Self {
        Self { last: None }
    }
}

impl<K: PartialEq + Send + Sync + 'static> DoubleClick<K> {
    /// Record a click on `key` at `now` real seconds, and say whether it
    /// completes a double.
    ///
    /// A completed double is FORGOTTEN rather than kept: three fast clicks are
    /// one double and one single, not two doubles, so drumming on a row acts
    /// once.
    pub fn press(&mut self, key: K, now: f32) -> bool {
        let double = self
            .last
            .as_ref()
            .is_some_and(|(last, at)| *last == key && now - at <= DOUBLE_CLICK_SECS);
        self.last = (!double).then_some((key, now));
        double
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_click_on_the_same_key_inside_the_window_is_one_double() {
        let mut click = DoubleClick::<u8>::default();
        assert!(!click.press(1, 10.0));
        assert!(click.press(1, 10.0 + DOUBLE_CLICK_SECS));
        // The third fast click starts a new count.
        assert!(!click.press(1, 10.6));
        // Another key, or the same key too late, is a single.
        assert!(!click.press(2, 10.7));
        assert!(!click.press(2, 11.3));
    }
}
