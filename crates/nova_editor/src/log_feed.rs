//! What the log says at WARN and above, kept for the Log tab.
//!
//! The Log tab is the only place a builder reads the editor's own trouble, and
//! a `warn!` otherwise reaches only the terminal the game was started from.

use std::{
    collections::VecDeque,
    fmt::Debug,
    sync::{Arc, Mutex},
};

use bevy::{
    log::{
        tracing::{
            field::{Field, Visit},
            Event, Level, Subscriber,
        },
        tracing_subscriber::layer::{Context, Layer},
        BoxedLayer,
    },
    prelude::*,
};

/// How many lines wait for the editor. Outside it nobody drains them, so the
/// oldest go.
const FEED_LINES: usize = 200;

/// The longest line kept, in characters. A panic-sized message is not a row.
const LINE_CHARS: usize = 300;

/// One line of the log at WARN or above.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FedLine {
    /// Whether it was logged at ERROR rather than WARN.
    pub(crate) error: bool,
    /// Its message's first line.
    pub(crate) text: String,
}

/// The lines the log layer has seen and the Log tab has not.
#[derive(Resource, Clone, Default)]
pub struct EditorLogFeed(Arc<Mutex<VecDeque<FedLine>>>);

impl EditorLogFeed {
    /// Take every waiting line, oldest first.
    pub(crate) fn drain(&self) -> Vec<FedLine> {
        self.0
            .lock()
            .map(|mut lines| lines.drain(..).collect())
            .unwrap_or_default()
    }

    fn push(&self, line: FedLine) {
        if let Ok(mut lines) = self.0.lock() {
            if lines.len() == FEED_LINES {
                lines.pop_front();
            }
            lines.push_back(line);
        }
    }
}

/// The `LogPlugin::custom_layer` that fills [`EditorLogFeed`].
///
/// Returns the layer and inserts its feed, so the Log tab reads what this
/// app's log says. An app whose log was installed elsewhere has no feed, and
/// its Log tab holds status lines only.
pub fn editor_log_layer(app: &mut App) -> Option<BoxedLayer> {
    let feed = EditorLogFeed::default();
    app.insert_resource(feed.clone());
    Some(Box::new(FeedLayer(feed)))
}

struct FeedLayer(EditorLogFeed);

impl<S: Subscriber> Layer<S> for FeedLayer {
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        let level = *event.metadata().level();
        if level > Level::WARN {
            return;
        }
        let mut message = MessageField::default();
        event.record(&mut message);
        let first = message.0.lines().next().unwrap_or_default();
        self.0.push(FedLine {
            error: level == Level::ERROR,
            text: format!("{}: {first}", event.metadata().target())
                .chars()
                .take(LINE_CHARS)
                .collect(),
        });
    }
}

/// The event's `message` field.
#[derive(Default)]
struct MessageField(String);

impl Visit for MessageField {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            self.0 = format!("{value:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::log::{
        tracing::{self, subscriber::with_default},
        tracing_subscriber::{layer::SubscriberExt, Registry},
    };

    use super::*;

    #[test]
    fn warnings_and_errors_reach_the_feed_and_info_does_not() {
        let feed = EditorLogFeed::default();
        let subscriber = Registry::default().with(FeedLayer(feed.clone()));

        with_default(subscriber, || {
            tracing::info!("just talking");
            tracing::warn!(target: "nova_editor", "a warning\nwith a tail");
            tracing::error!(target: "bevy_ecs", "an error");
        });

        assert_eq!(
            feed.drain(),
            vec![
                FedLine {
                    error: false,
                    text: "nova_editor: a warning".to_string(),
                },
                FedLine {
                    error: true,
                    text: "bevy_ecs: an error".to_string(),
                },
            ]
        );
    }
}
