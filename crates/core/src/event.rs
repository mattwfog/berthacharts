//! Reactivity events emitted by a [`crate::Chart`].
//!
//! Bindings subscribe to these and translate them into framework-idiomatic
//! signals (Leptos) or external-store snapshots (React). Core itself does
//! not know about framework primitives.

use crate::ids::{DatasetId, LayerId, MarkId, ScaleId, SelectionId, TransformId};

/// Events the chart emits. `#[non_exhaustive]` so new variants do not break
/// matching at call sites.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ChartEvent {
    /// A scale was inserted or replaced.
    ScaleChanged(ScaleId),
    /// A dataset was inserted or replaced.
    DatasetChanged(DatasetId),
    /// A transform was inserted or replaced.
    TransformChanged(TransformId),
    /// A layer was added, removed, or re-ordered.
    LayerChanged(LayerId),
    /// The viewport (canvas size / DPR / plot area) changed.
    ViewportChanged,
    /// The selection state changed.
    SelectionChanged(SelectionId),
    /// A specific mark was hovered or un-hovered.
    MarkHovered(Option<MarkId>),
    /// The chart has enqueued a frame; renderer should draw on the next tick.
    DirtyRequested,
    /// A frame finished rendering.
    Rendered,
}

/// Handle returned by [`crate::Chart::subscribe`]. Drop to unsubscribe.
#[must_use]
pub struct Subscription {
    /// Disposer — runs on drop.
    disposer: Option<Box<dyn FnOnce() + Send + Sync + 'static>>,
}

impl Subscription {
    /// Create a subscription from a disposer closure (internal constructor).
    pub(crate) fn new(dispose: impl FnOnce() + Send + Sync + 'static) -> Self {
        Self {
            disposer: Some(Box::new(dispose)),
        }
    }

    /// Eagerly unsubscribe.
    pub fn unsubscribe(mut self) {
        if let Some(d) = self.disposer.take() {
            d();
        }
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(d) = self.disposer.take() {
            d();
        }
    }
}

impl std::fmt::Debug for Subscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscription").finish_non_exhaustive()
    }
}
